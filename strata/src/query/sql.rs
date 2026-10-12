//! SQL front end: parses Strata's dialect with `sqlparser` and binds it to a
//! table schema, producing a [`Select`].
//!
//! Supported: `SELECT <cols | * | score() | COUNT(*)> FROM <table>
//! [WHERE <pred>] [GROUP BY <keyword>] [ORDER BY <score> [ASC|DESC]]
//! [LIMIT n [OFFSET m]]`, where predicates combine `MATCH(field, q)`,
//! comparisons, `BETWEEN`, `IN` with AND/OR/NOT, and scores are `bm25`,
//! `similarity`, `distance`, `rrf`, weighted sums of those, or a column.
//! Parameters (`:name`) are bound from [`Params`].

use std::collections::HashMap;
use std::sync::Arc;

use sqlparser::ast::{self as sql, BinaryOperator, Expr, FunctionArg, FunctionArgExpr, FunctionArguments};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::error::{Error, Result};
use crate::query::logical::{OrderBy, Opts, Pred, Projection, Score, Select, Tiebreak};
use crate::schema::{FieldType, Schema};
use crate::value::Value;

/// Values for `:name` placeholders.
#[derive(Clone, Debug, Default)]
pub struct Params(pub HashMap<String, Value>);

impl Params {
    pub fn new() -> Self {
        Params::default()
    }
    pub fn set(mut self, name: &str, v: Value) -> Self {
        self.0.insert(name.trim_start_matches([':', '$', '@']).to_string(), v);
        self
    }
    fn get(&self, placeholder: &str) -> Result<&Value> {
        let name = placeholder.trim_start_matches([':', '$', '@', '?']);
        self.0.get(name).ok_or_else(|| err(format!("unbound parameter {placeholder}")))
    }
}

fn err(m: impl Into<String>) -> Error {
    Error::Query(m.into())
}

/// Parses and binds one SELECT statement.
pub fn bind(sql_text: &str, schema: &Schema, params: &Params) -> Result<Select> {
    let stmts = Parser::parse_sql(&GenericDialect {}, sql_text).map_err(|e| err(e.to_string()))?;
    let [stmt] = stmts.as_slice() else { return Err(err("expected exactly one statement")) };
    let sql::Statement::Query(q) = stmt else { return Err(err("only SELECT is supported")) };
    Binder { schema, params }.query(q)
}

struct Binder<'a> {
    schema: &'a Schema,
    params: &'a Params,
}

impl Binder<'_> {
    fn query(&self, q: &sql::Query) -> Result<Select> {
        let sql::SetExpr::Select(s) = q.body.as_ref() else { return Err(err("only simple SELECT is supported")) };
        if s.from.len() != 1 || !s.from[0].joins.is_empty() {
            return Err(err("exactly one table, no joins"));
        }
        if let sql::TableFactor::Table { name, .. } = &s.from[0].relation {
            let t = name.to_string();
            if !t.eq_ignore_ascii_case(&self.schema.table) {
                return Err(err(format!("no such table: {t}")));
            }
        }
        let filter = match &s.selection {
            Some(e) => self.pred(e)?.simplify(),
            None => Pred::True,
        };
        let group_by = match &s.group_by {
            sql::GroupByExpr::Expressions(es, _) if es.is_empty() => None,
            sql::GroupByExpr::Expressions(es, _) if es.len() == 1 => {
                let f = self.field_of(&es[0])?;
                if self.schema.fields[f].ty != FieldType::Keyword {
                    return Err(err("GROUP BY needs a KEYWORD field"));
                }
                Some(f)
            }
            _ => return Err(err("GROUP BY supports one keyword field")),
        };
        let mut projection = Vec::new();
        for item in &s.projection {
            let (e, alias) = match item {
                sql::SelectItem::UnnamedExpr(e) => (e, None),
                sql::SelectItem::ExprWithAlias { expr, alias } => (expr, Some(alias.value.clone())),
                sql::SelectItem::Wildcard(_) => {
                    for (i, f) in self.schema.fields.iter().enumerate() {
                        if f.stored && !matches!(f.ty, FieldType::Vector { .. }) {
                            projection.push((f.name.clone(), Projection::Field(i)));
                        }
                    }
                    continue;
                }
                _ => return Err(err("unsupported select item")),
            };
            let p = self.projection(e)?;
            if let (Some(g), Projection::Field(f)) = (group_by, &p) {
                if *f != g {
                    return Err(err("with GROUP BY, select the group field and COUNT(*)"));
                }
            }
            projection.push((alias.unwrap_or_else(|| e.to_string()), p));
        }
        let (order, tiebreak) = match &q.order_by {
            None => (None, None),
            Some(ob) => match &ob.kind {
                sql::OrderByKind::Expressions(es) if (1..=2).contains(&es.len()) => {
                    let desc = es[0].options.asc == Some(false);
                    let order = self.order(&es[0].expr, desc, &projection, group_by)?;
                    let tiebreak = match es.get(1) {
                        None => None,
                        Some(e) => Some(self.tiebreak(&e.expr, e.options.asc == Some(false), &projection, group_by)?),
                    };
                    (Some(order), tiebreak)
                }
                _ => return Err(err("ORDER BY supports a key and an optional tie-break")),
            },
        };
        let (limit, offset) = match &q.limit_clause {
            None => (None, 0),
            Some(sql::LimitClause::LimitOffset { limit, offset, .. }) => (
                limit.as_ref().map(|e| self.usize_lit(e)).transpose()?,
                offset.as_ref().map(|o| self.usize_lit(&o.value)).transpose()?.unwrap_or(0),
            ),
            Some(sql::LimitClause::OffsetCommaLimit { offset, limit }) => {
                (Some(self.usize_lit(limit)?), self.usize_lit(offset)?)
            }
        };
        Ok(Select { projection, filter, order, group_by, tiebreak, limit, offset })
    }

    /// The second ORDER BY key: the primary key, or the group value of a facet.
    fn tiebreak(&self, e: &Expr, desc: bool, proj: &[(String, Projection)], group: Option<usize>) -> Result<Tiebreak> {
        let field = match literal_usize(e) {
            Some(n) => match proj.get(n.wrapping_sub(1)) {
                Some((_, Projection::Field(f))) => *f,
                _ => return Err(err(format!("ORDER BY ..., {n}: not a column"))),
            },
            None => self.field_of(e)?,
        };
        if Some(field) == group {
            Ok(Tiebreak::GroupValue { desc })
        } else if Some(field) == self.schema.primary_key {
            Ok(Tiebreak::Key { desc })
        } else {
            Err(err("the second ORDER BY key must be the primary key or the group field"))
        }
    }

    fn projection(&self, e: &Expr) -> Result<Projection> {
        match e {
            Expr::Identifier(_) | Expr::CompoundIdentifier(_) => Ok(Projection::Field(self.field_of(e)?)),
            Expr::Function(f) => match fname(f).as_str() {
                "score" => Ok(Projection::Score),
                "count" => Ok(Projection::Count),
                n => Err(err(format!("unsupported function in SELECT: {n}"))),
            },
            _ => Err(err(format!("unsupported select expression: {e}"))),
        }
    }

    fn order(&self, e: &Expr, desc: bool, proj: &[(String, Projection)], group: Option<usize>) -> Result<OrderBy> {
        // Positional `ORDER BY 2`, `COUNT(*)` and the group field address facet output.
        if let Some(n) = literal_usize(e) {
            let (_, p) = proj.get(n.wrapping_sub(1)).ok_or_else(|| err(format!("ORDER BY {n}: no such column")))?;
            return match p {
                Projection::Count => Ok(OrderBy::Count { desc }),
                Projection::Field(f) if Some(*f) == group => Ok(OrderBy::GroupValue { desc }),
                Projection::Field(f) => Ok(OrderBy::Score { score: Score::Column { field: *f }, desc }),
                Projection::Score => Err(err("ORDER BY score(): order by the score expression itself")),
            };
        }
        if let Expr::Function(f) = e {
            if fname(f) == "count" {
                return Ok(OrderBy::Count { desc });
            }
        }
        if group.is_some() {
            let f = self.field_of(e)?;
            return if Some(f) == group { Ok(OrderBy::GroupValue { desc }) } else { Err(err("facet queries order by count or value")) };
        }
        // `ORDER BY distance(..) ASC` ranks like `similarity DESC`.
        if let Expr::Function(f) = e {
            if fname(f) == "distance" {
                return Ok(OrderBy::Score { score: self.score(e)?, desc: !desc });
            }
        }
        Ok(OrderBy::Score { score: self.score(e)?, desc })
    }

    fn score(&self, e: &Expr) -> Result<Score> {
        match e {
            Expr::Nested(inner) => self.score(inner),
            Expr::Identifier(_) | Expr::CompoundIdentifier(_) => {
                let f = self.field_of(e)?;
                match self.schema.fields[f].ty {
                    FieldType::Int | FieldType::Float => Ok(Score::Column { field: f }),
                    _ => Err(err("ORDER BY a column needs a numeric field")),
                }
            }
            Expr::BinaryOp { left, op: BinaryOperator::Plus, right } => {
                Ok(Score::Weighted([self.weighted(left, 1.0)?, self.weighted(right, 1.0)?].concat()))
            }
            Expr::BinaryOp { left, op: BinaryOperator::Minus, right } => {
                Ok(Score::Weighted([self.weighted(left, 1.0)?, self.weighted(right, -1.0)?].concat()))
            }
            Expr::BinaryOp { op: BinaryOperator::Multiply, .. } => Ok(Score::Weighted(self.weighted(e, 1.0)?)),
            Expr::Function(f) => {
                let (args, opts) = self.args(f)?;
                match fname(f).as_str() {
                    "bm25" => {
                        let [field, text] = args.as_slice() else { return Err(err("bm25(field, text)")) };
                        let field = self.typed_field(field, |t| matches!(t, FieldType::Text { .. }), "TEXT")?;
                        let text = self.literal(text)?.as_str().ok_or_else(|| err("bm25 text must be a string"))?.to_string();
                        Ok(Score::Bm25 { field, text, opts })
                    }
                    "similarity" | "distance" => {
                        let [field, v] = args.as_slice() else { return Err(err("similarity(field, vector)")) };
                        let field = self.typed_field(field, |t| matches!(t, FieldType::Vector { .. }), "VECTOR")?;
                        let FieldType::Vector { dim, .. } = self.schema.fields[field].ty else { unreachable!() };
                        let vector = self.vector(v)?;
                        if vector.len() != dim {
                            return Err(err(format!("vector has {} dimensions, field has {dim}", vector.len())));
                        }
                        Ok(Score::Similarity { field, vector, opts })
                    }
                    "rrf" => {
                        if args.len() < 2 {
                            return Err(err("rrf needs at least two scores"));
                        }
                        let inputs = args.iter().map(|a| self.score(a)).collect::<Result<Vec<_>>>()?;
                        let num = |k: &str, d: f64| opts.get(k).and_then(|v| v.as_f64()).unwrap_or(d);
                        Ok(Score::Rrf { inputs, k: num("k", 60.0), depth: num("depth", 100.0) as usize })
                    }
                    n => Err(err(format!("unknown score function {n}"))),
                }
            }
            _ => Err(err(format!("unsupported ORDER BY expression: {e}"))),
        }
    }

    /// Terms of a weighted sum, each multiplied by `w`.
    fn weighted(&self, e: &Expr, w: f64) -> Result<Vec<(f64, Score)>> {
        match e {
            Expr::Nested(inner) => self.weighted(inner, w),
            Expr::BinaryOp { left, op: BinaryOperator::Plus, right } => {
                Ok([self.weighted(left, w)?, self.weighted(right, w)?].concat())
            }
            Expr::BinaryOp { left, op: BinaryOperator::Minus, right } => {
                Ok([self.weighted(left, w)?, self.weighted(right, -w)?].concat())
            }
            Expr::BinaryOp { left, op: BinaryOperator::Multiply, right } => {
                if let Some(c) = self.number(left) {
                    self.weighted(right, w * c)
                } else if let Some(c) = self.number(right) {
                    self.weighted(left, w * c)
                } else {
                    Err(err("weights must be numeric constants"))
                }
            }
            _ => Ok(vec![(w, self.score(e)?)]),
        }
    }

    fn pred(&self, e: &Expr) -> Result<Pred> {
        Ok(match e {
            Expr::Nested(inner) => self.pred(inner)?,
            Expr::BinaryOp { left, op: BinaryOperator::And, right } => Pred::And(vec![self.pred(left)?, self.pred(right)?]),
            Expr::BinaryOp { left, op: BinaryOperator::Or, right } => Pred::Or(vec![self.pred(left)?, self.pred(right)?]),
            Expr::UnaryOp { op: sql::UnaryOperator::Not, expr } => Pred::Not(Box::new(self.pred(expr)?)),
            Expr::BinaryOp { left, op, right } => self.comparison(left, op, right)?,
            Expr::Between { expr, negated, low, high } => {
                let f = self.numeric_field(expr)?;
                let (lo, hi) = (self.num_lit(low)?, self.num_lit(high)?);
                let p = Pred::Range { field: f, lo, hi };
                if *negated { Pred::Not(Box::new(p)) } else { p }
            }
            Expr::InList { expr, list, negated } => {
                let f = self.field_of(expr)?;
                let values = list.iter().map(|v| self.field_value(f, v)).collect::<Result<Vec<_>>>()?;
                let p = Pred::In { field: f, values };
                if *negated { Pred::Not(Box::new(p)) } else { p }
            }
            Expr::Function(func) if fname(func) == "match" => {
                let (args, _) = self.args(func)?;
                let [field, q] = args.as_slice() else { return Err(err("MATCH(field, query)")) };
                let field = self.typed_field(field, |t| matches!(t, FieldType::Text { .. }), "TEXT")?;
                if !self.schema.fields[field].indexed {
                    return Err(err(format!("{} is not indexed", self.schema.fields[field].name)));
                }
                let query = self.literal(q)?.as_str().ok_or_else(|| err("MATCH query must be a string"))?.to_string();
                Pred::Match { field, query }
            }
            Expr::Value(v) if matches!(v.value, sql::Value::Boolean(_)) => {
                if matches!(v.value, sql::Value::Boolean(true)) { Pred::True } else { Pred::False }
            }
            _ => return Err(err(format!("unsupported predicate: {e}"))),
        })
    }

    fn comparison(&self, left: &Expr, op: &BinaryOperator, right: &Expr) -> Result<Pred> {
        use BinaryOperator::*;
        // Normalise to `field op literal`.
        let (field_e, lit_e, op) = if self.field_of(left).is_ok() {
            (left, right, op.clone())
        } else {
            let flipped = match op {
                Lt => Gt,
                LtEq => GtEq,
                Gt => Lt,
                GtEq => LtEq,
                o => o.clone(),
            };
            (right, left, flipped)
        };
        let f = self.field_of(field_e)?;
        let ty = &self.schema.fields[f].ty;
        if matches!(ty, FieldType::Keyword) {
            let v = self.field_value(f, lit_e)?;
            return match op {
                Eq => Ok(Pred::In { field: f, values: vec![v] }),
                NotEq => Ok(Pred::Not(Box::new(Pred::In { field: f, values: vec![v] }))),
                _ => Err(err("keyword fields support =, != and IN")),
            };
        }
        if !matches!(ty, FieldType::Int | FieldType::Float) {
            return Err(err(format!("cannot compare field {}", self.schema.fields[f].name)));
        }
        let x = self.num_lit(lit_e)?;
        let int = matches!(ty, FieldType::Int);
        let above = |x: f64| if int { x.floor() + 1.0 } else { next_up(x) };
        let below = |x: f64| if int { x.ceil() - 1.0 } else { next_down(x) };
        let (inf, ninf) = (f64::INFINITY, f64::NEG_INFINITY);
        let range = |lo, hi| Pred::Range { field: f, lo, hi };
        Ok(match op {
            Eq => range(x, x),
            NotEq => Pred::Not(Box::new(range(x, x))),
            Gt => range(above(x), inf),
            GtEq => range(if int { x.ceil() } else { x }, inf),
            Lt => range(ninf, below(x)),
            LtEq => range(ninf, if int { x.floor() } else { x }),
            o => return Err(err(format!("unsupported operator {o}"))),
        })
    }

    /// Positional and named (`name => value`) arguments of a function call.
    fn args<'e>(&self, f: &'e sql::Function) -> Result<(Vec<&'e Expr>, Opts)> {
        let FunctionArguments::List(list) = &f.args else {
            return Ok((Vec::new(), Opts::new()));
        };
        let (mut pos, mut opts) = (Vec::new(), Opts::new());
        for a in &list.args {
            match a {
                FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => pos.push(e),
                FunctionArg::Unnamed(FunctionArgExpr::Wildcard) => {}
                FunctionArg::Named { name, arg: FunctionArgExpr::Expr(e), .. } => {
                    opts.insert(name.value.to_ascii_lowercase(), self.literal(e)?);
                }
                _ => return Err(err(format!("unsupported argument {a}"))),
            }
        }
        Ok((pos, opts))
    }

    fn field_of(&self, e: &Expr) -> Result<usize> {
        let name = match e {
            Expr::Identifier(id) => &id.value,
            Expr::CompoundIdentifier(ids) => &ids.last().unwrap().value,
            _ => return Err(err(format!("expected a column, got {e}"))),
        };
        self.schema.field_index(name).ok_or_else(|| err(format!("no such column: {name}")))
    }

    fn typed_field(&self, e: &Expr, ok: impl Fn(&FieldType) -> bool, what: &str) -> Result<usize> {
        let f = self.field_of(e)?;
        if ok(&self.schema.fields[f].ty) { Ok(f) } else { Err(err(format!("{} is not a {what} field", self.schema.fields[f].name))) }
    }

    fn numeric_field(&self, e: &Expr) -> Result<usize> {
        self.typed_field(e, |t| matches!(t, FieldType::Int | FieldType::Float), "numeric")
    }

    fn literal(&self, e: &Expr) -> Result<Value> {
        match e {
            Expr::Value(v) => match &v.value {
                sql::Value::Number(s, _) => {
                    Ok(s.parse::<i64>().map(Value::Int).or_else(|_| s.parse::<f64>().map(Value::Float)).map_err(|_| err("bad number"))?)
                }
                sql::Value::SingleQuotedString(s) | sql::Value::DoubleQuotedString(s) => Ok(Value::Str(s.clone())),
                sql::Value::Boolean(b) => Ok(Value::Int(*b as i64)),
                sql::Value::Null => Ok(Value::Null),
                sql::Value::Placeholder(p) => Ok(self.params.get(p)?.clone()),
                other => Err(err(format!("unsupported literal {other}"))),
            },
            Expr::UnaryOp { op: sql::UnaryOperator::Minus, expr } => match self.literal(expr)? {
                Value::Int(x) => Ok(Value::Int(-x)),
                Value::Float(x) => Ok(Value::Float(-x)),
                _ => Err(err("cannot negate a non-number")),
            },
            Expr::Identifier(id) if !id.value.is_empty() => Ok(Value::Str(id.value.clone())),
            _ => Err(err(format!("expected a literal or parameter, got {e}"))),
        }
    }

    fn number(&self, e: &Expr) -> Option<f64> {
        if self.field_of(e).is_ok() {
            return None;
        }
        self.literal(e).ok()?.as_f64()
    }

    fn num_lit(&self, e: &Expr) -> Result<f64> {
        self.number(e).ok_or_else(|| err(format!("expected a number, got {e}")))
    }

    fn usize_lit(&self, e: &Expr) -> Result<usize> {
        let x = self.num_lit(e)?;
        if x < 0.0 || x.fract() != 0.0 {
            return Err(err("LIMIT/OFFSET must be non-negative integers"));
        }
        Ok(x as usize)
    }

    /// A literal coerced to field `f`'s type.
    fn field_value(&self, f: usize, e: &Expr) -> Result<Value> {
        let v = self.literal(e)?;
        match (&self.schema.fields[f].ty, v) {
            (FieldType::Keyword, Value::Str(s)) => Ok(Value::Str(s)),
            (FieldType::Keyword, Value::Int(x)) => Ok(Value::Str(x.to_string())),
            (FieldType::Int, Value::Int(x)) => Ok(Value::Int(x)),
            (FieldType::Float, v @ (Value::Int(_) | Value::Float(_))) => Ok(Value::Float(v.as_f64().unwrap())),
            (t, v) => Err(err(format!("value {v:?} does not fit a {t:?} field"))),
        }
    }

    fn vector(&self, e: &Expr) -> Result<Arc<[f32]>> {
        match self.literal(e)? {
            Value::Vector(v) => Ok(v),
            Value::Str(s) => {
                let v: Vec<f32> = serde_json::from_str(&s).map_err(|_| err("vector literal must be a JSON array"))?;
                Ok(v.into())
            }
            v => Err(err(format!("expected a vector, got {v:?}"))),
        }
    }
}

fn fname(f: &sql::Function) -> String {
    f.name.to_string().to_ascii_lowercase()
}

fn literal_usize(e: &Expr) -> Option<usize> {
    match e {
        Expr::Value(v) => match &v.value {
            sql::Value::Number(s, _) => s.parse().ok(),
            _ => None,
        },
        _ => None,
    }
}

fn next_up(x: f64) -> f64 {
    if x.is_nan() || x == f64::INFINITY {
        return x;
    }
    if x == 0.0 {
        return f64::from_bits(1);
    }
    let b = x.to_bits();
    f64::from_bits(if x > 0.0 { b + 1 } else { b - 1 })
}

fn next_down(x: f64) -> f64 {
    -next_up(-x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::beir_schema;
    use crate::schema::VectorIndexKind;

    fn schema() -> Schema {
        beir_schema(4, VectorIndexKind::Flat)
    }

    fn q() -> Params {
        Params::new().set("q", Value::Vector(vec![0.5f32; 4].into())).set("t", Value::Str("mrna vaccine".into()))
    }

    #[test]
    fn binds_filtered_bm25() {
        let s = bind(
            "SELECT id, title, score() FROM docs WHERE MATCH(body, 'vaccine -animal') AND year BETWEEN 2019 AND 2021 \
             AND category IN ('c01', 'c02') AND rating > 4 ORDER BY bm25(body, :t) DESC LIMIT 10",
            &schema(),
            &q(),
        )
        .unwrap();
        assert_eq!(s.limit, Some(10));
        assert_eq!(s.projection.len(), 3);
        let Pred::And(ps) = &s.filter else { panic!("{:?}", s.filter) };
        assert_eq!(ps.len(), 4);
        assert!(matches!(&ps[0], Pred::Match { field: 3, query } if query == "vaccine -animal"));
        assert_eq!(ps[1], Pred::Range { field: 4, lo: 2019.0, hi: 2021.0 });
        assert!(matches!(&ps[2], Pred::In { field: 5, values } if values.len() == 2));
        assert!(matches!(ps[3], Pred::Range { field: 6, lo, hi } if lo > 4.0 && lo < 4.0001 && hi.is_infinite()));
        assert!(matches!(&s.order, Some(OrderBy::Score { score: Score::Bm25 { field: 3, text, .. }, desc: true }) if text == "mrna vaccine"));
    }

    #[test]
    fn binds_vector_and_hybrid_scores() {
        let s = bind("SELECT id FROM docs WHERE year > 2019 ORDER BY similarity(emb, :q, nprobe => 32) DESC LIMIT 5", &schema(), &q())
            .unwrap();
        assert_eq!(s.filter, Pred::Range { field: 4, lo: 2020.0, hi: f64::INFINITY });
        let Some(OrderBy::Score { score: Score::Similarity { opts, .. }, desc: true }) = &s.order else { panic!() };
        assert_eq!(opts.get("nprobe"), Some(&Value::Int(32)));

        let s = bind("SELECT id FROM docs ORDER BY distance(emb, :q) LIMIT 5", &schema(), &q()).unwrap();
        assert!(matches!(s.order, Some(OrderBy::Score { score: Score::Similarity { .. }, desc: true })));

        let s = bind("SELECT id FROM docs ORDER BY rrf(bm25(body, :t), similarity(emb, :q), k => 20) DESC LIMIT 5", &schema(), &q())
            .unwrap();
        let Some(OrderBy::Score { score: Score::Rrf { inputs, k, depth }, .. }) = &s.order else { panic!() };
        assert_eq!((inputs.len(), *k, *depth), (2, 20.0, 100));

        let s = bind("SELECT id FROM docs ORDER BY 0.3 * bm25(body, :t) + 0.7 * similarity(emb, :q) DESC LIMIT 5", &schema(), &q())
            .unwrap();
        let Some(OrderBy::Score { score: Score::Weighted(ts), .. }) = &s.order else { panic!() };
        assert_eq!(ts.iter().map(|(w, _)| *w).collect::<Vec<_>>(), vec![0.3, 0.7]);
    }

    #[test]
    fn binds_facets_and_rejects_errors() {
        let s = bind("SELECT category, COUNT(*) FROM docs WHERE MATCH(body, 'x') GROUP BY category ORDER BY 2 DESC LIMIT 5", &schema(), &q())
            .unwrap();
        assert_eq!(s.group_by, Some(5));
        assert_eq!(s.order, Some(OrderBy::Count { desc: true }));
        assert_eq!(s.projection[1].1, Projection::Count);
        let s = bind("SELECT category, COUNT(*) FROM docs GROUP BY category ORDER BY 2 DESC, 1 LIMIT 5", &schema(), &q()).unwrap();
        assert_eq!(s.tiebreak, Some(Tiebreak::GroupValue { desc: false }));
        let s = bind("SELECT id FROM docs WHERE rating BETWEEN 2.0 AND 2.0 ORDER BY rating DESC, id LIMIT 10", &schema(), &q()).unwrap();
        assert_eq!(s.tiebreak, Some(Tiebreak::Key { desc: false }));
        assert!(matches!(s.order, Some(OrderBy::Score { score: Score::Column { field: 6 }, desc: true })));

        for bad in [
            "SELECT id FROM other",
            "SELECT id FROM docs WHERE MATCH(year, 'x')",
            "SELECT id FROM docs WHERE category > 'c01'",
            "SELECT id FROM docs ORDER BY similarity(emb, :missing)",
            "SELECT id FROM docs ORDER BY bm25(body, :t) * similarity(emb, :q)",
            "SELECT id FROM docs ORDER BY year, rating",
            "DELETE FROM docs",
        ] {
            assert!(bind(bad, &schema(), &q()).is_err(), "{bad}");
        }
    }

    #[test]
    fn star_and_strict_float_bounds() {
        let s = bind("SELECT * FROM docs WHERE rating < 2.5 OR NOT category = 'c00' LIMIT 3 OFFSET 1", &schema(), &q()).unwrap();
        let names: Vec<&str> = s.projection.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["id", "title", "text", "year", "category", "rating", "topic"]);
        assert_eq!((s.limit, s.offset), (Some(3), 1));
        let Pred::Or(ps) = &s.filter else { panic!() };
        assert!(matches!(ps[0], Pred::Range { lo, hi, .. } if lo.is_infinite() && hi < 2.5 && hi > 2.4999));
        assert!(matches!(&ps[1], Pred::Not(_)));
    }
}
