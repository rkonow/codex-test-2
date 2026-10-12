//! Tables and field types.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// How a vector field is indexed (every vector field also keeps float32
/// vectors and SQ8 codes in the forward index, so `Flat` adds nothing).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum VectorIndexKind {
    Flat,
    /// Inverted file: `nlist` k-means clusters, members stored as posting
    /// lists with their codes.
    Ivf { nlist: usize },
    /// Hierarchical navigable small world graph.
    Hnsw { m: usize, ef_construction: usize },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    /// Analysed full text. `positions` enables phrase queries.
    Text { positions: bool },
    /// An exact string value: equality, `IN`, facets.
    Keyword,
    Int,
    Float,
    /// A fixed-dimension float vector; similarity is the dot product.
    Vector { dim: usize, index: VectorIndexKind },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
    /// Kept in the doc store and returned by `SELECT`. (Vector fields are
    /// always retrievable from the vector sections.)
    pub stored: bool,
    /// Indexed for search (text: postings; keyword: postings + column;
    /// numbers: column + range postings; vectors: the ANN index).
    pub indexed: bool,
}

impl Field {
    pub fn new(name: &str, ty: FieldType) -> Self {
        Field { name: name.to_string(), ty, stored: true, indexed: true }
    }
    pub fn stored_only(mut self) -> Self {
        self.indexed = false;
        self
    }
    pub fn not_stored(mut self) -> Self {
        self.stored = false;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Schema {
    pub table: String,
    pub fields: Vec<Field>,
    /// Index into `fields` of the primary key (a Keyword or Int field).
    pub primary_key: Option<usize>,
}

impl Schema {
    pub fn new(table: &str, fields: Vec<Field>, primary_key: Option<&str>) -> Result<Self> {
        let s = Schema { table: table.to_string(), fields, primary_key: None };
        let mut s = s;
        if let Some(pk) = primary_key {
            let i = s.field_index(pk).ok_or_else(|| Error::Schema(format!("no primary key field {pk}")))?;
            if !matches!(s.fields[i].ty, FieldType::Keyword | FieldType::Int) {
                return Err(Error::Schema("primary key must be KEYWORD or INT".into()));
            }
            s.primary_key = Some(i);
        }
        let mut names: Vec<&str> = s.fields.iter().map(|f| f.name.as_str()).collect();
        names.sort();
        if names.windows(2).any(|w| w[0] == w[1]) {
            return Err(Error::Schema("duplicate field name".into()));
        }
        Ok(s)
    }

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name))
    }

    pub fn field(&self, name: &str) -> Option<&Field> {
        self.field_index(name).map(|i| &self.fields[i])
    }
}
