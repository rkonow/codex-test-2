//! Hybrid retrieval with priors, the injection gate, snippets and the
//! text Engram injects.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::capture::past_reference;
use crate::record::{Kind, Record};
use crate::store::{Filter, Hit, Store};
use crate::Result;

/// Ranking parameters (see DESIGN.md §5).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankParams {
    /// Candidates from each of BM25 and vector search.
    pub n: usize,
    /// Weight of normalised BM25 against normalised cosine.
    pub w_bm25: f64,
    /// Importance prior: score × (1 + λ_ι ι).
    pub lambda_imp: f64,
    /// Recency prior weight λ_t and scale τ (in records).
    pub lambda_rec: f64,
    pub tau: f64,
    /// Use BM25 / vectors at all.
    pub use_bm25: bool,
    pub use_vectors: bool,
}

impl Default for RankParams {
    fn default() -> Self {
        RankParams { n: 50, w_bm25: 0.5, lambda_imp: 0.2, lambda_rec: 0.05, tau: 5000.0, use_bm25: true, use_vectors: true }
    }
}

/// Absolute evidence about the best candidates, for the gate.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Evidence {
    /// Highest cosine similarity among vector candidates.
    pub max_cos: f64,
    /// Highest IDF-weighted query-term coverage among the top 5.
    pub max_cov: f64,
    /// The top candidate is in both the BM25 and the vector top 10.
    pub agree: bool,
    /// The prompt refers back to earlier work.
    pub cue: bool,
    /// How far the best vector candidate stands out: cos₁ − cos₁₀.
    #[serde(default)]
    pub cos_gap: f64,
    /// How far the best BM25 candidate stands out: 1 − bm25₁₀ / bm25₁.
    #[serde(default)]
    pub bm_gap: f64,
}

/// The injection gate, a linear score over the evidence:
/// G = a·max_cos + b·max_cov + c·cue + d·agree + e·cos_gap + f·bm_gap.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Gate {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
    /// Full injection at G ≥ hi; a brief one (two records) at lo ≤ G < hi.
    pub hi: f64,
    pub lo: f64,
}

impl Default for Gate {
    fn default() -> Self {
        // Logistic fit on LongMemEval-S with the static embedder
        // (scripts/fit_gate.py): hi holds precision 0.9, lo precision 0.7.
        Gate { a: 4.936, b: 3.920, c: 0.317, d: 1.099, e: 1.968, f: 1.042, hi: 7.658, lo: 5.329 }
    }
}

impl Gate {
    /// The gate fitted for an embedder spec (cosine scales differ).
    pub fn for_embedder(spec: &str) -> Gate {
        // BGE-small vectors (in the daemon, or from a benchmark cache).
        if spec.starts_with("bert") || spec.contains("bge") {
            Gate::bert()
        } else {
            Gate::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Decision {
    /// Strong evidence: the full budget.
    Inject,
    /// Weak evidence: the top two records in a small budget.
    Brief,
    Skip,
}

impl Gate {
    pub fn bert() -> Gate {
        Gate::default()
    }

    pub fn score(&self, e: &Evidence) -> f64 {
        self.a * e.max_cos + self.b * e.max_cov + self.c * e.cue as u8 as f64 + self.d * e.agree as u8 as f64 + self.e * e.cos_gap + self.f * e.bm_gap
    }
    pub fn decide(&self, e: &Evidence) -> Decision {
        let g = self.score(e);
        if g >= self.hi {
            Decision::Inject
        } else if g >= self.lo {
            Decision::Brief
        } else {
            Decision::Skip
        }
    }
}

fn analyzed(text: &str) -> HashSet<String> {
    strata::text::analyze(text).into_iter().collect()
}

/// Hybrid search: BM25 and vector candidates, min-max fused, with priors.
pub fn search(store: &Store, query: &str, filter: &Filter, p: &RankParams) -> Result<(Vec<Hit>, Evidence)> {
    let terms = strata::text::analyze_query(query);
    let bm = if p.use_bm25 && !terms.is_empty() { store.bm25_top(query, p.n, filter)? } else { Vec::new() };
    let vq = if p.use_vectors { Some(store.embedder.embed_query(query)) } else { None };
    let dv = match &vq {
        Some(v) => store.vector_top(v, p.n, filter)?,
        None => Vec::new(),
    };
    Ok(fuse(store, query, &terms, bm, dv, p))
}

/// Fuses BM25 and vector candidate lists (exposed for benchmarks that run
/// the two searches themselves).
pub fn fuse(store: &Store, query: &str, terms: &[String], bm: Vec<(Record, f64)>, dv: Vec<(Record, f64)>, p: &RankParams) -> (Vec<Hit>, Evidence) {
    let top_ids = |l: &[(Record, f64)]| l.iter().take(10).map(|(r, _)| r.id.clone()).collect::<HashSet<_>>();
    let (bm_top, dv_top) = (top_ids(&bm), top_ids(&dv));
    let max_cos = dv.iter().map(|x| x.1).fold(0.0, f64::max);
    let nth = |l: &[(Record, f64)], i: usize| l.get(i.min(l.len().saturating_sub(1))).map_or(0.0, |x| x.1);
    let cos_gap = if dv.is_empty() { 0.0 } else { nth(&dv, 0) - nth(&dv, 9) };
    let bm_gap = if bm.is_empty() || nth(&bm, 0) <= 0.0 { 0.0 } else { 1.0 - nth(&bm, 9) / nth(&bm, 0) };
    let range = |l: &[(Record, f64)]| {
        let lo = l.iter().map(|x| x.1).fold(f64::INFINITY, f64::min);
        let hi = l.iter().map(|x| x.1).fold(f64::NEG_INFINITY, f64::max);
        (lo, hi)
    };
    let (blo, bhi) = range(&bm);
    let (vlo, vhi) = range(&dv);
    let norm = |x: f64, lo: f64, hi: f64| if hi > lo { (x - lo) / (hi - lo) } else { 1.0 };
    let mut by: HashMap<String, Hit> = HashMap::new();
    for (r, s) in bm {
        by.insert(r.id.clone(), Hit { record: r, bm25: Some(s), cos: None, score: 0.0 });
    }
    for (r, s) in dv {
        by.entry(r.id.clone()).and_modify(|h| h.cos = Some(s)).or_insert(Hit { record: r, bm25: None, cos: Some(s), score: 0.0 });
    }
    let max_seq = by.values().map(|h| h.record.seq).max().unwrap_or(0).max(store.last_seq());
    let (wb, wv) = match (bhi.is_finite(), vhi.is_finite()) {
        (true, true) => (p.w_bm25, 1.0 - p.w_bm25),
        (true, false) => (1.0, 0.0),
        _ => (0.0, 1.0),
    };
    let mut hits: Vec<Hit> = by
        .into_values()
        .map(|mut h| {
            let b = h.bm25.map_or(0.0, |x| norm(x, blo, bhi));
            let c = h.cos.map_or(0.0, |x| norm(x, vlo, vhi));
            let rel = wb * b + wv * c;
            let age = (max_seq - h.record.seq).max(0) as f64;
            h.score = rel * (1.0 + p.lambda_imp * h.record.importance) + p.lambda_rec * (-age / p.tau).exp();
            h
        })
        .collect();
    hits.sort_by(|a, b| b.score.total_cmp(&a.score).then(b.record.seq.cmp(&a.record.seq)));
    let mut ev = Evidence { max_cos, cue: past_reference(query), cos_gap, bm_gap, ..Default::default() };
    if let Some(top) = hits.first() {
        ev.agree = bm_top.contains(&top.record.id) && dv_top.contains(&top.record.id);
    }
    if !terms.is_empty() {
        let idf = store.idf(terms).unwrap_or_else(|_| vec![1.0; terms.len()]);
        let total: f64 = idf.iter().sum();
        for h in hits.iter().take(5) {
            let have = analyzed(&format!("{}\n{}", h.record.text, h.record.ctx));
            let got: f64 = terms.iter().zip(&idf).filter(|(t, _)| have.contains(*t)).map(|(_, w)| w).sum();
            ev.max_cov = ev.max_cov.max(if total > 0.0 { got / total } else { 0.0 });
        }
    }
    (hits, ev)
}

/// The part of `text` (at most `max` bytes) with the most query terms.
pub fn snippet(text: &str, terms: &HashSet<String>, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut hits = Vec::new();
    let mut pos = 0;
    for w in text.split(|c: char| !c.is_alphanumeric()) {
        if !w.is_empty() {
            if let Some(t) = strata::text::analyzer::term_of(&w.to_lowercase()) {
                if terms.contains(t.as_ref()) {
                    hits.push(pos);
                }
            }
        }
        pos += w.len() + 1;
    }
    let mut best = (0usize, 0usize);
    for &h in &hits {
        let start = h.saturating_sub(max / 5);
        let n = hits.iter().filter(|&&x| x >= start && x < start + max).count();
        if n > best.1 {
            best = (start, n);
        }
    }
    let floor = |mut i: usize| {
        i = i.min(text.len());
        while !text.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let (s, e) = (floor(best.0), floor(best.0 + max));
    format!("{}{}{}", if s > 0 { "…" } else { "" }, text[s..e].trim(), if e < text.len() { "…" } else { "" })
}

/// How retrieved records become injected text.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackParams {
    pub budget_chars: usize,
    pub snippet_chars: usize,
    /// Records taken from one session turn at most.
    pub per_turn: usize,
    /// Records at most.
    pub max_records: usize,
}

impl Default for PackParams {
    fn default() -> Self {
        PackParams { budget_chars: 4000, snippet_chars: 600, per_turn: 2, max_records: 8 }
    }
}

impl PackParams {
    /// The weak-evidence tier: two records, about 300 tokens.
    pub fn brief() -> PackParams {
        PackParams { budget_chars: 1200, snippet_chars: 500, per_turn: 1, max_records: 2 }
    }
}

/// Selects hits within the budget; returns them oldest first with snippets.
pub fn pack(hits: &[Hit], query: &str, skip: &HashSet<String>, p: &PackParams) -> Vec<(Record, String)> {
    let terms: HashSet<String> = strata::text::analyze_query(query).into_iter().collect();
    let mut per: HashMap<(String, i64), usize> = HashMap::new();
    let mut used = 0usize;
    let mut out: Vec<(Record, String)> = Vec::new();
    for h in hits {
        if out.len() >= p.max_records {
            break;
        }
        if skip.contains(&h.record.id) {
            continue;
        }
        let k = (h.record.session.clone(), h.record.turn);
        if *per.get(&k).unwrap_or(&0) >= p.per_turn {
            continue;
        }
        let snip = snippet(&h.record.text, &terms, p.snippet_chars);
        if used + snip.len() + 80 > p.budget_chars && !out.is_empty() {
            continue;
        }
        used += snip.len() + 80;
        *per.entry(k).or_default() += 1;
        out.push((h.record.clone(), snip));
    }
    out.sort_by_key(|(r, _)| r.seq);
    out
}

/// `2026-10-11 07:43 UTC` for a Unix time.
pub fn fmt_time(ts: i64) -> String {
    let days = ts.div_euclid(86_400);
    let secs = ts.rem_euclid(86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + (m <= 2) as i64;
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02} UTC", secs / 3600, (secs % 3600) / 60)
}

/// `3 h ago`, `2 days ago`.
pub fn fmt_ago(ts: i64, now: i64) -> String {
    let d = (now - ts).max(0);
    match d {
        0..=119 => "just now".into(),
        120..=7199 => format!("{} min ago", d / 60),
        7200..=172_799 => format!("{} h ago", d / 3600),
        _ => format!("{} days ago", d / 86_400),
    }
}

fn short(session: &str) -> String {
    session.chars().take(8).collect()
}

/// One injected record: a provenance line and its snippet.
pub fn fmt_record(r: &Record, snip: &str, now: i64) -> String {
    let mut head = format!("[{}] {} · session {} turn {} · {}", r.id, fmt_time(r.ts), short(&r.session), r.turn, r.kind.as_str());
    if r.tag != "-" && !r.tag.is_empty() {
        head += &format!(" ({})", r.tag);
    }
    head += &format!(" · {}", fmt_ago(r.ts, now));
    format!("{head}\n{}", snip.trim())
}

/// The recall block for a prompt (`weak`: a low-confidence match).
pub fn fmt_recall_as(items: &[(Record, String)], now: i64, weak: bool) -> String {
    let mut s = if weak {
        format!(
            "Engram found {} possibly related record{} from earlier work in this project (a weak match, outside the current context window). They may be irrelevant or outdated.\n",
            items.len(),
            if items.len() == 1 { "" } else { "s" }
        )
    } else {
        format!(
            "Engram recalled {} record{} from earlier work in this project, outside the current context window (oldest first). They may be outdated; newer records and the current files take precedence.\n",
            items.len(),
            if items.len() == 1 { "" } else { "s" }
        )
    };
    for (r, snip) in items {
        s += "\n";
        s += &fmt_record(r, snip, now);
        s += "\n";
    }
    s += "\nThe memory_timeline tool shows what surrounded a record id; memory_search finds more.";
    s
}

pub fn fmt_recall(items: &[(Record, String)], now: i64) -> String {
    fmt_recall_as(items, now, false)
}

/// The `SessionStart` briefing: preferences, decisions, open tasks and where
/// the last session left off.
pub fn briefing(store: &Store, current_session: &str, max_chars: usize, now: i64) -> Result<String> {
    let total = store.len();
    if total == 0 {
        return Ok(String::new());
    }
    let none = Filter::default();
    let mut s = String::new();
    let sessions = store.query("SELECT session, COUNT(*) FROM memory GROUP BY session ORDER BY 2 DESC LIMIT 1000")?.rows.len();
    let last = store.latest(&[], 1, &Filter { exclude_window: Some((current_session.to_string(), 0)), ..Default::default() })?;
    s += &format!("Engram memory for this project: {total} records from {sessions} session{}", if sessions == 1 { "" } else { "s" });
    if let Some(r) = last.first() {
        s += &format!(", last activity {}", fmt_ago(r.ts, now));
    }
    s += ". The memory_search, memory_timeline and memory_sql tools query it; memory_remember stores decisions and facts.\n";
    let clip = |t: &str, n: usize| -> String {
        let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
        if t.chars().count() > n {
            format!("{}…", t.chars().take(n).collect::<String>())
        } else {
            t
        }
    };
    let section = |title: &str, recs: Vec<Record>, n: usize, s: &mut String| {
        let recs: Vec<Record> = recs.into_iter().filter(|r| r.tag != "superseded").collect();
        if recs.is_empty() {
            return;
        }
        *s += &format!("\n{title}\n");
        for r in recs {
            *s += &format!("- {} ({}, {})\n", clip(&r.text, n), r.id, fmt_ago(r.ts, now));
        }
    };
    section("Preferences and rules stated earlier:", store.latest(&[Kind::Preference], 8, &none)?, 220, &mut s);
    section("Recorded decisions and facts:", store.latest(&[Kind::Decision, Kind::Fact, Kind::Note], 6, &none)?, 220, &mut s);
    let todos = store.latest(&[Kind::Todo], 2, &none)?;
    section("Open tasks at the last checkpoint:", todos, 500, &mut s);
    let prev = Filter { exclude_window: Some((current_session.to_string(), 0)), ..Default::default() };
    let mut eps = store.latest(&[Kind::Episode], 3, &prev)?;
    eps.reverse();
    section("Where the most recent earlier session left off:", eps, 300, &mut s);
    if s.len() > max_chars {
        let mut cut = max_chars;
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        s.truncate(cut);
        s += "…";
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_formats() {
        assert_eq!(fmt_time(0), "1970-01-01 00:00 UTC");
        assert_eq!(fmt_time(1_791_788_086), "2026-10-12 06:54 UTC");
        assert_eq!(fmt_ago(100, 100 + 3 * 3600), "3 h ago");
    }

    #[test]
    fn snippet_finds_terms() {
        let text = format!("{} the planner probes local selectivity {}", "x ".repeat(500), "y ".repeat(500));
        let terms: HashSet<String> = strata::text::analyze_query("planner selectivity").into_iter().collect();
        let s = snippet(&text, &terms, 200);
        assert!(s.contains("planner probes local selectivity"), "{s}");
        assert!(s.len() <= 210);
    }
}
