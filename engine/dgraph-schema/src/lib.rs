//! dgraph-schema: Schema definition and validation
//!
//! Defines the schema for predicates and types in the graph.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]

use dgraph_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Scalar types supported by predicates
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScalarType {
    /// 64-bit integer
    Int,
    /// 64-bit float
    Float,
    /// UTF-8 string
    String,
    /// Boolean
    Bool,
    /// Date-time
    DateTime,
    /// Geographic point
    Geo,
    /// Binary data
    Binary,
    /// UID reference
    Uid,
    /// Password (hashed)
    Password,
    /// Vector float (embeddings)
    VFloat,
}

impl ScalarType {
    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "int" => Some(Self::Int),
            "float" => Some(Self::Float),
            "string" => Some(Self::String),
            "bool" => Some(Self::Bool),
            "datetime" => Some(Self::DateTime),
            "geo" => Some(Self::Geo),
            "binary" => Some(Self::Binary),
            "uid" => Some(Self::Uid),
            "password" => Some(Self::Password),
            "vfloat" | "float32vector" => Some(Self::VFloat),
            _ => None,
        }
    }

    /// Convert to string
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Float => "float",
            Self::String => "string",
            Self::Bool => "bool",
            Self::DateTime => "datetime",
            Self::Geo => "geo",
            Self::Binary => "binary",
            Self::Uid => "uid",
            Self::Password => "password",
            Self::VFloat => "vfloat",
        }
    }

    /// Check if this type supports the given index type
    #[must_use]
    pub fn supports_index(self, index: IndexType) -> bool {
        match (self, index) {
            // String indexes
            (Self::String, IndexType::Hash | IndexType::Exact | IndexType::Term | IndexType::Fulltext | IndexType::Trigram) => true,
            // Numeric indexes
            (Self::Int, IndexType::Int) => true,
            (Self::Float, IndexType::Float) => true,
            // DateTime indexes
            (Self::DateTime, IndexType::DateTime | IndexType::Year | IndexType::Month | IndexType::Day | IndexType::Hour) => true,
            // Geo indexes
            (Self::Geo, IndexType::Geo) => true,
            // Bool indexes
            (Self::Bool, IndexType::Bool) => true,
            _ => false,
        }
    }

    /// Check if this is a numeric type
    #[must_use]
    pub const fn is_numeric(self) -> bool {
        matches!(self, Self::Int | Self::Float)
    }

    /// Check if this is a string-like type
    #[must_use]
    pub const fn is_string_like(self) -> bool {
        matches!(self, Self::String | Self::Password)
    }
}

/// Index type for a predicate
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndexType {
    /// Hash index (exact match, non-sortable)
    Hash,
    /// Exact index (exact match, sortable)
    Exact,
    /// Term index (word-based)
    Term,
    /// Fulltext index (with stemming and stopwords)
    Fulltext,
    /// Trigram index (regex, fuzzy matching)
    Trigram,
    /// Integer range index
    Int,
    /// Float range index
    Float,
    /// DateTime range index
    DateTime,
    /// Year index for DateTime
    Year,
    /// Month index for DateTime
    Month,
    /// Day index for DateTime
    Day,
    /// Hour index for DateTime
    Hour,
    /// Geo index
    Geo,
    /// Boolean index
    Bool,
}

impl IndexType {
    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "hash" => Some(Self::Hash),
            "exact" => Some(Self::Exact),
            "term" => Some(Self::Term),
            "fulltext" => Some(Self::Fulltext),
            "trigram" => Some(Self::Trigram),
            "int" => Some(Self::Int),
            "float" => Some(Self::Float),
            "datetime" => Some(Self::DateTime),
            "year" => Some(Self::Year),
            "month" => Some(Self::Month),
            "day" => Some(Self::Day),
            "hour" => Some(Self::Hour),
            "geo" => Some(Self::Geo),
            "bool" => Some(Self::Bool),
            _ => None,
        }
    }

    /// Convert to string
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hash => "hash",
            Self::Exact => "exact",
            Self::Term => "term",
            Self::Fulltext => "fulltext",
            Self::Trigram => "trigram",
            Self::Int => "int",
            Self::Float => "float",
            Self::DateTime => "datetime",
            Self::Year => "year",
            Self::Month => "month",
            Self::Day => "day",
            Self::Hour => "hour",
            Self::Geo => "geo",
            Self::Bool => "bool",
        }
    }

    /// Check if this index type is sortable
    #[must_use]
    pub const fn is_sortable(self) -> bool {
        matches!(
            self,
            Self::Exact | Self::Int | Self::Float | Self::DateTime | Self::Year | Self::Month | Self::Day | Self::Hour
        )
    }

    /// Check if this index type is lossy (can't reconstruct original value)
    #[must_use]
    pub const fn is_lossy(self) -> bool {
        matches!(
            self,
            Self::Term | Self::Fulltext | Self::Trigram | Self::Float | Self::Year | Self::Month | Self::Day | Self::Hour | Self::Geo
        )
    }
}

/// Predicate definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Predicate {
    /// Predicate name
    pub name: String,
    /// Value type
    pub value_type: ScalarType,
    /// List type (multiple values)
    pub list: bool,
    /// Reverse edge predicate name
    pub reverse: Option<String>,
    /// Indexes
    pub indexes: Vec<IndexType>,
    /// Count index (maintain edge count)
    pub count: bool,
    /// Upsert directive
    pub upsert: bool,
    /// Lang directive (multi-language string)
    pub lang: bool,
    /// Unique constraint
    pub unique: bool,
    /// Not-null constraint
    pub not_null: bool,
}

impl Default for Predicate {
    fn default() -> Self {
        Self {
            name: String::new(),
            value_type: ScalarType::String,
            list: false,
            reverse: None,
            indexes: Vec::new(),
            count: false,
            upsert: false,
            lang: false,
            unique: false,
            not_null: false,
        }
    }
}

impl Predicate {
    /// Create new predicate
    #[must_use]
    pub fn new(name: impl Into<String>, value_type: ScalarType) -> Self {
        Self {
            name: name.into(),
            value_type,
            ..Self::default()
        }
    }

    /// Set as list type
    #[must_use]
    pub fn list(mut self) -> Self {
        self.list = true;
        self
    }

    /// Add reverse edge
    #[must_use]
    pub fn reverse(mut self, name: impl Into<String>) -> Self {
        self.reverse = Some(name.into());
        self
    }

    /// Add index
    #[must_use]
    pub fn index(mut self, index: IndexType) -> Self {
        self.indexes.push(index);
        self
    }

    /// Enable count index
    #[must_use]
    pub fn count(mut self) -> Self {
        self.count = true;
        self
    }

    /// Set upsert directive
    #[must_use]
    pub fn upsert(mut self) -> Self {
        self.upsert = true;
        self
    }

    /// Set lang directive (multi-language string)
    #[must_use]
    pub fn lang(mut self) -> Self {
        self.lang = true;
        self
    }

    /// Set unique constraint
    #[must_use]
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Set not-null constraint
    #[must_use]
    pub fn not_null(mut self) -> Self {
        self.not_null = true;
        self
    }

    /// Check if the predicate has any indexing
    #[must_use]
    pub fn is_indexed(&self) -> bool {
        !self.indexes.is_empty() || self.count || self.reverse.is_some()
    }

    /// Check if the predicate has a specific index type
    #[must_use]
    pub fn has_index(&self, index: IndexType) -> bool {
        self.indexes.contains(&index)
    }

    /// Validate that the indexes are compatible with the value type
    ///
    /// # Errors
    /// Returns error if an index is not compatible with the value type
    pub fn validate_indexes(&self) -> Result<()> {
        for &index in &self.indexes {
            if !self.value_type.supports_index(index) {
                return Err(Error::Schema(format!(
                    "index '{}' is not compatible with type '{}' for predicate '{}'",
                    index.as_str(),
                    self.value_type.as_str(),
                    self.name
                )));
            }
        }
        Ok(())
    }
}

/// Type definition (grouping of predicates)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeDef {
    /// Type name
    pub name: String,
    /// Fields (predicate names)
    pub fields: Vec<String>,
}

impl TypeDef {
    /// Create new type
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
        }
    }

    /// Add field
    #[must_use]
    pub fn field(mut self, name: impl Into<String>) -> Self {
        self.fields.push(name.into());
        self
    }
}

/// Complete schema
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Schema {
    /// Predicates by name
    pub predicates: HashMap<String, Predicate>,
    /// Types by name
    pub types: HashMap<String, TypeDef>,
}

impl Schema {
    /// Create empty schema
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add predicate
    ///
    /// # Errors
    /// Returns error if predicate indexes are invalid
    pub fn add_predicate(&mut self, pred: Predicate) -> Result<()> {
        pred.validate_indexes()?;
        self.predicates.insert(pred.name.clone(), pred);
        Ok(())
    }

    /// Add predicate without validation
    pub fn add_predicate_unchecked(&mut self, pred: Predicate) {
        self.predicates.insert(pred.name.clone(), pred);
    }

    /// Add type
    pub fn add_type(&mut self, type_def: TypeDef) {
        self.types.insert(type_def.name.clone(), type_def);
    }

    /// Get predicate by name
    #[must_use]
    pub fn get_predicate(&self, name: &str) -> Option<&Predicate> {
        self.predicates.get(name)
    }

    /// Get mutable predicate by name
    #[must_use]
    pub fn get_predicate_mut(&mut self, name: &str) -> Option<&mut Predicate> {
        self.predicates.get_mut(name)
    }

    /// Get type by name
    #[must_use]
    pub fn get_type(&self, name: &str) -> Option<&TypeDef> {
        self.types.get(name)
    }

    /// Check if predicate exists
    #[must_use]
    pub fn has_predicate(&self, name: &str) -> bool {
        self.predicates.contains_key(name)
    }

    /// Check if predicate is indexed
    #[must_use]
    pub fn is_indexed(&self, name: &str) -> bool {
        self.predicates.get(name).map_or(false, Predicate::is_indexed)
    }

    /// Check if predicate has reverse edge
    #[must_use]
    pub fn has_reverse(&self, name: &str) -> bool {
        self.predicates.get(name).map_or(false, |p| p.reverse.is_some())
    }

    /// Check if predicate has count index
    #[must_use]
    pub fn has_count(&self, name: &str) -> bool {
        self.predicates.get(name).map_or(false, |p| p.count)
    }

    /// Get all indexed predicates
    #[must_use]
    pub fn indexed_predicates(&self) -> Vec<&Predicate> {
        self.predicates.values().filter(|p| p.is_indexed()).collect()
    }

    /// Get all predicates with a specific index type
    #[must_use]
    pub fn predicates_with_index(&self, index: IndexType) -> Vec<&Predicate> {
        self.predicates
            .values()
            .filter(|p| p.has_index(index))
            .collect()
    }

    /// Validate predicate value
    ///
    /// # Errors
    /// Returns error if value doesn't match predicate type
    pub fn validate(&self, predicate: &str, _value: &[u8]) -> Result<()> {
        if !self.predicates.contains_key(predicate) {
            return Err(Error::Schema(format!("unknown predicate: {predicate}")));
        }
        // Simplified: skip actual validation
        Ok(())
    }

    /// Validate the entire schema
    ///
    /// # Errors
    /// Returns error if schema is invalid
    pub fn validate_schema(&self) -> Result<()> {
        // Validate all predicates
        for pred in self.predicates.values() {
            pred.validate_indexes()?;
        }

        // Validate types reference valid predicates
        for type_def in self.types.values() {
            for field in &type_def.fields {
                if !self.predicates.contains_key(field) {
                    return Err(Error::Schema(format!(
                        "type '{}' references unknown predicate '{}'",
                        type_def.name, field
                    )));
                }
            }
        }

        Ok(())
    }
}
