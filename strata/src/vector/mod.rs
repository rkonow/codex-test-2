//! Vector storage and ANN indexes: SQ8 + float32 forward vectors, flat, IVF,
//! HNSW, filtered search. (To be implemented; see DESIGN.md §5.3.)

use crate::error::Result;
use crate::schema::Field;
use crate::segment::build::BuildOptions;
use crate::segment::{FieldIndexBuilder, NoopBuilder};

/// Builder for a VECTOR field.
pub fn field_builder(_field: &Field, _opts: &BuildOptions) -> Result<Box<dyn FieldIndexBuilder>> {
    Ok(Box::new(NoopBuilder))
}
