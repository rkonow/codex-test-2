//! Single-file storage: meta slots, extents, commit protocol, WAL, snapshots,
//! delete sets, merges. (To be implemented; see DESIGN.md §3.)

use crate::error::Result;
use crate::schema::Schema;
use crate::segment::SegmentWriter;
use crate::value::Document;

/// Writes the primary-key index section of a segment.
pub fn build_pk_index(_schema: &Schema, _docs: &[Document], _out: &mut SegmentWriter) -> Result<()> {
    Ok(())
}
