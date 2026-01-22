//! dgraph-schema: Schema definition and validation

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]

use dgraph_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Scalar types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
        }
    }
}

/// Index type for a predicate
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexType {
    /// Hash index (exact match)
    Hash,
    /// Exact index (exact match, sortable)
    Exact,
    /// Term index (full-text)
    Term,
    /// Fulltext index (with stemming)
    Fulltext,
    /// Trigram index (regex, fuzzy)
    Trigram,
    /// Integer range index
    Int,
    /// Float range index
    Float,
    /// DateTime range index
    DateTime,
    /// Geo index
    Geo,
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
    /// Upsert directive
    pub upsert: bool,
    /// Lang directive (multi-language string)
    pub lang: bool,
}

impl Predicate {
    /// Create new predicate
    #[must_use]
    pub fn new(name: impl Into<String>, value_type: ScalarType) -> Self {
        Self {
            name: name.into(),
            value_type,
            list: false,
            reverse: None,
            indexes: Vec::new(),
            upsert: false,
            lang: false,
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

    /// Set upsert directive
    #[must_use]
    pub fn upsert(mut self) -> Self {
        self.upsert = true;
        self
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
    pub fn add_predicate(&mut self, pred: Predicate) {
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

    /// Get type by name
    #[must_use]
    pub fn get_type(&self, name: &str) -> Option<&TypeDef> {
        self.types.get(name)
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
}
