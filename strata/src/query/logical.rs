//! The bound form of a query: what to return, which rows qualify, and how to
//! rank them. The planner turns this into per-segment physical operators.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::value::Value;

/// A row predicate over schema fields (indices into `Schema::fields`).
#[derive(Clone, Debug, PartialEq)]
pub enum Pred {
    True,
    False,
    /// `MATCH(field, 'boolean query')` on a TEXT field (grammar: text module).
    Match { field: usize, query: String },
    /// Numeric range with inclusive bounds (`±inf` for open ends). Strict
    /// comparisons are normalised to inclusive ones at bind time.
    Range { field: usize, lo: f64, hi: f64 },
    /// Keyword (or numeric) equality with any of `values`.
    In { field: usize, values: Vec<Value> },
    And(Vec<Pred>),
    Or(Vec<Pred>),
    Not(Box<Pred>),
}

impl Pred {
    /// Flattens nested AND/OR and folds constants.
    pub fn simplify(self) -> Pred {
        match self {
            Pred::And(ps) => {
                let mut out = Vec::new();
                for p in ps.into_iter().map(Pred::simplify) {
                    match p {
                        Pred::True => {}
                        Pred::False => return Pred::False,
                        Pred::And(inner) => out.extend(inner),
                        p => out.push(p),
                    }
                }
                match out.len() {
                    0 => Pred::True,
                    1 => out.pop().unwrap(),
                    _ => Pred::And(out),
                }
            }
            Pred::Or(ps) => {
                let mut out = Vec::new();
                for p in ps.into_iter().map(Pred::simplify) {
                    match p {
                        Pred::False => {}
                        Pred::True => return Pred::True,
                        Pred::Or(inner) => out.extend(inner),
                        p => out.push(p),
                    }
                }
                match out.len() {
                    0 => Pred::False,
                    1 => out.pop().unwrap(),
                    _ => Pred::Or(out),
                }
            }
            Pred::Not(p) => match p.simplify() {
                Pred::True => Pred::False,
                Pred::False => Pred::True,
                Pred::Not(q) => *q,
                p => Pred::Not(Box::new(p)),
            },
            p => p,
        }
    }
}

/// Named options of a search function, e.g. `nprobe => 32`.
pub type Opts = BTreeMap<String, Value>;

/// A ranking function. Larger is better for every variant.
#[derive(Clone, Debug, PartialEq)]
pub enum Score {
    /// Disjunctive BM25 of the analysed terms of `text` in a TEXT field.
    Bm25 { field: usize, text: String, opts: Opts },
    /// Dot product with `vector` (cosine for unit vectors). `distance(...)`
    /// binds to this with the sort direction flipped.
    Similarity { field: usize, vector: Arc<[f32]>, opts: Opts },
    /// Reciprocal rank fusion: `Σ 1 / (k + rank_i)` over the inputs' rankings
    /// of their top `depth` rows.
    Rrf { inputs: Vec<Score>, k: f64, depth: usize },
    /// A weighted sum of scores (`0.3 * bm25(..) + 0.7 * similarity(..)`).
    Weighted(Vec<(f64, Score)>),
    /// A numeric column's value.
    Column { field: usize },
}

impl Score {
    /// Every leaf (BM25, similarity or column) of the expression.
    pub fn leaves(&self) -> Vec<&Score> {
        match self {
            Score::Rrf { inputs, .. } => inputs.iter().flat_map(|s| s.leaves()).collect(),
            Score::Weighted(ts) => ts.iter().flat_map(|(_, s)| s.leaves()).collect(),
            s => vec![s],
        }
    }

    pub fn opts(&self) -> Option<&Opts> {
        match self {
            Score::Bm25 { opts, .. } | Score::Similarity { opts, .. } => Some(opts),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum OrderBy {
    /// Rank by a score, best first (`desc = true`) or worst first.
    Score { score: Score, desc: bool },
    /// Facet queries: order groups by their count.
    Count { desc: bool },
    /// Facet queries: order groups by their value.
    GroupValue { desc: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Projection {
    Field(usize),
    /// The ORDER BY score (`score()`), or the fused score for hybrids.
    Score,
    /// `COUNT(*)`: of the whole result, or per group.
    Count,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Select {
    /// Output column names and what they hold.
    pub projection: Vec<(String, Projection)>,
    pub filter: Pred,
    pub order: Option<OrderBy>,
    /// A KEYWORD field to group by (facets), with `COUNT(*)` per group.
    pub group_by: Option<usize>,
    pub limit: Option<usize>,
    pub offset: usize,
}
