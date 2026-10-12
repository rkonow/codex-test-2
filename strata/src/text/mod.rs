//! Full-text index: analyzer, term dictionary, scored postings, BM25.
//! (To be implemented; see DESIGN.md §5.1.)

use crate::error::Result;
use crate::schema::Field;
use crate::segment::build::BuildOptions;
use crate::segment::{FieldIndexBuilder, NoopBuilder};

/// Builder for an indexed TEXT field.
pub fn field_builder(_field: &Field, _opts: &BuildOptions) -> Result<Box<dyn FieldIndexBuilder>> {
    Ok(Box::new(NoopBuilder))
}
