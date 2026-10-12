//! Forward index: columns with zone maps, keyword and range postings, doc
//! store. (To be implemented; see DESIGN.md §5.2.)

use crate::error::Result;
use crate::schema::{Field, Schema};
use crate::segment::build::BuildOptions;
use crate::segment::{FieldIndexBuilder, NoopBuilder, SegmentWriter};
use crate::value::Document;

/// Builder for an indexed KEYWORD, INT or FLOAT field.
pub fn field_builder(_field: &Field, _opts: &BuildOptions) -> Result<Box<dyn FieldIndexBuilder>> {
    Ok(Box::new(NoopBuilder))
}

/// Writes the doc store (stored, non-vector fields of every document).
pub fn build_doc_store(_schema: &Schema, _docs: &[Document], _opts: &BuildOptions, _out: &mut SegmentWriter) -> Result<()> {
    Ok(())
}
