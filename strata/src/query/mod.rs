//! SQL front end, planner and executor (DESIGN.md §6–7).
//!
//! * [`sql`]: parses the dialect and binds it to a schema.
//! * [`logical`]: the bound query: projection, predicate tree, score.

pub mod fusion;
pub mod logical;
pub mod plan;
pub mod sql;

pub use logical::{OrderBy, Pred, Projection, Score, Select, Tiebreak};
pub use sql::{bind, Params};
