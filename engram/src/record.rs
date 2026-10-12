//! Memory records and the Strata table that holds them.

use serde::{Deserialize, Serialize};
use strata::{Field, FieldType, Schema, Value, VectorIndexKind};

use crate::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Prompt,
    Answer,
    ToolCall,
    ToolResult,
    Error,
    Episode,
    Summary,
    Decision,
    Preference,
    Fact,
    Todo,
    Note,
}

impl Kind {
    pub const ALL: [Kind; 12] = [
        Kind::Prompt,
        Kind::Answer,
        Kind::ToolCall,
        Kind::ToolResult,
        Kind::Error,
        Kind::Episode,
        Kind::Summary,
        Kind::Decision,
        Kind::Preference,
        Kind::Fact,
        Kind::Todo,
        Kind::Note,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Prompt => "prompt",
            Kind::Answer => "answer",
            Kind::ToolCall => "tool_call",
            Kind::ToolResult => "tool_result",
            Kind::Error => "error",
            Kind::Episode => "episode",
            Kind::Summary => "summary",
            Kind::Decision => "decision",
            Kind::Preference => "preference",
            Kind::Fact => "fact",
            Kind::Todo => "todo",
            Kind::Note => "note",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.as_str() == s)
    }

    /// Salience prior ι in [0, 1].
    pub fn prior(self) -> f64 {
        match self {
            Kind::Preference => 1.0,
            Kind::Decision => 0.9,
            Kind::Summary | Kind::Fact | Kind::Todo | Kind::Note => 0.8,
            Kind::Prompt => 0.7,
            Kind::Error | Kind::Episode => 0.6,
            Kind::Answer => 0.5,
            Kind::ToolCall | Kind::ToolResult => 0.3,
        }
    }

    /// Durable knowledge (shown in briefings, pinned).
    pub fn is_knowledge(self) -> bool {
        matches!(self, Kind::Preference | Kind::Decision | Kind::Fact | Kind::Todo | Kind::Note)
    }
}

/// A record before the daemon assigns its sequence number.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Draft {
    pub kind: Kind,
    pub tag: String,
    pub files: Vec<String>,
    pub text: String,
    /// Context in the record's lexical key (BM25) but not in its vector nor
    /// shown as its content: the turn's prompt for tool records, the section
    /// for summaries.
    pub ctx: String,
    /// Overrides the kind's prior.
    pub importance: Option<f64>,
}

impl Draft {
    pub fn new(kind: Kind, text: impl Into<String>) -> Draft {
        Draft { kind, tag: "-".into(), files: Vec::new(), text: text.into(), ctx: String::new(), importance: None }
    }
    pub fn tag(mut self, t: impl Into<String>) -> Draft {
        let t = t.into();
        self.tag = if t.is_empty() { "-".into() } else { t };
        self
    }
    pub fn files(mut self, f: Vec<String>) -> Draft {
        self.files = f;
        self
    }
    pub fn ctx(mut self, c: impl Into<String>) -> Draft {
        self.ctx = c.into();
        self
    }
    pub fn importance(mut self, x: f64) -> Draft {
        self.importance = Some(x);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub session: String,
    pub kind: Kind,
    pub tag: String,
    pub turn: i64,
    pub seq: i64,
    pub ts: i64,
    pub importance: f64,
    pub files: String,
    pub text: String,
    pub ctx: String,
}

impl Record {
    pub fn from_draft(d: Draft, session: &str, turn: i64, seq: i64, ts: i64) -> Record {
        Record {
            id: record_id(session, seq),
            session: session.to_string(),
            kind: d.kind,
            tag: d.tag,
            turn,
            seq,
            ts,
            importance: d.importance.unwrap_or_else(|| d.kind.prior()).clamp(0.0, 1.0),
            files: d.files.join(" "),
            text: d.text,
            ctx: d.ctx,
        }
    }

    /// The Strata row, in [`schema`] field order.
    pub fn to_doc(&self, emb: Vec<f32>) -> Vec<Value> {
        vec![
            Value::Str(self.id.clone()),
            Value::Str(self.session.clone()),
            Value::Str(self.kind.as_str().into()),
            Value::Str(self.tag.clone()),
            Value::Int(self.turn),
            Value::Int(self.seq),
            Value::Int(self.ts),
            Value::Float((self.importance * 100.0).round() / 100.0),
            Value::Str(self.files.clone()),
            Value::Str(self.text.clone()),
            Value::Str(self.ctx.clone()),
            Value::Str(String::new()),
            Value::Vector(emb.into()),
        ]
    }

    /// What the embedding is computed from: the text alone. The context
    /// joins the lexical key only; in a vector it pulls a response towards
    /// its request, which then crowds out the request itself
    /// (results/structure: MRR 0.60 → 0.66 on LongMemEval-S).
    pub fn embed_text(&self) -> String {
        self.text.clone()
    }
}

/// `<first 8 characters of the session>:<seq>`.
pub fn record_id(session: &str, seq: i64) -> String {
    let s: String = session.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect();
    format!("{}:{seq}", if s.is_empty() { "x".to_string() } else { s })
}

/// Columns every query projects, in this order (see [`crate::store::Hit`]).
pub const COLUMNS: &str = "id, session, kind, tag, turn, seq, ts, importance, files, text, ctx";

pub fn schema(dim: usize, index: VectorIndexKind) -> Result<Schema> {
    Ok(Schema::new(
        "memory",
        vec![
            Field::new("id", FieldType::Keyword),
            Field::new("session", FieldType::Keyword),
            Field::new("kind", FieldType::Keyword),
            Field::new("tag", FieldType::Keyword),
            Field::new("turn", FieldType::Int),
            Field::new("seq", FieldType::Int),
            Field::new("ts", FieldType::Int),
            Field::new("importance", FieldType::Float),
            Field::new("files", FieldType::Text { positions: false }),
            Field::new("text", FieldType::Text { positions: true }).stored_only(),
            Field::new("ctx", FieldType::Text { positions: false }).stored_only(),
            // What MATCH and bm25 search: the text and its context.
            Field::new("body", FieldType::Text { positions: true }).not_stored().generated(&["text", "ctx"]),
            Field::new("emb", FieldType::Vector { dim, index }).not_stored(),
        ],
        Some("id"),
    )?)
}
