//! AST: Abstract Syntax Tree for DQL
//!
//! Simple, explicit structures. No magic.

use dgraph_common::Uid;
use serde::{Deserialize, Serialize};

/// Root operation
#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    /// Query operation
    Query(Query),
    /// Mutation operation
    Mutation(Mutation),
}

/// Query block
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Query {
    /// Query name/alias
    pub alias: Option<String>,
    /// Root function (eq, has, uid, etc.)
    pub func: Option<Function>,
    /// Starting UIDs
    pub uids: Vec<Uid>,
    /// Fields to fetch
    pub children: Vec<Query>,
    /// Predicate name (for child queries)
    pub attr: Option<String>,
    /// Filter expression
    pub filter: Option<Filter>,
    /// First N results
    pub first: Option<u32>,
    /// Skip N results
    pub offset: Option<u32>,
    /// Order by
    pub order: Option<Order>,
}

/// Root function
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    /// Function name
    pub name: FuncName,
    /// Arguments
    pub args: Vec<Value>,
}

/// Built-in function names
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FuncName {
    /// uid(0x1, 0x2, ...)
    Uid,
    /// eq(predicate, value)
    Eq,
    /// has(predicate)
    Has,
    /// ge(predicate, value)
    Ge,
    /// le(predicate, value)
    Le,
    /// gt(predicate, value)
    Gt,
    /// lt(predicate, value)
    Lt,
    /// allofterms(predicate, "word1 word2")
    AllOfTerms,
    /// anyofterms(predicate, "word1 word2")
    AnyOfTerms,
}

impl FuncName {
    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "uid" => Some(Self::Uid),
            "eq" => Some(Self::Eq),
            "has" => Some(Self::Has),
            "ge" => Some(Self::Ge),
            "le" => Some(Self::Le),
            "gt" => Some(Self::Gt),
            "lt" => Some(Self::Lt),
            "allofterms" => Some(Self::AllOfTerms),
            "anyofterms" => Some(Self::AnyOfTerms),
            _ => None,
        }
    }
}

/// Filter expression
#[derive(Clone, Debug, PartialEq)]
pub enum Filter {
    /// AND of filters
    And(Vec<Filter>),
    /// OR of filters
    Or(Vec<Filter>),
    /// NOT filter
    Not(Box<Filter>),
    /// Function call
    Func(Function),
}

/// Order specification
#[derive(Clone, Debug, PartialEq)]
pub struct Order {
    /// Predicate to order by
    pub attr: String,
    /// Descending order
    pub desc: bool,
}

/// Value in query
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// UID reference
    Uid(u64),
    /// String value
    String(String),
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Bool(bool),
    /// Variable reference
    Var(String),
}

impl Value {
    /// Try to get as string
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as i64
    #[must_use]
    pub const fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as UID
    #[must_use]
    pub const fn as_uid(&self) -> Option<u64> {
        match self {
            Self::Uid(u) => Some(*u),
            _ => None,
        }
    }
}

/// Mutation block
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mutation {
    /// Set operations
    pub set: Vec<Triple>,
    /// Delete operations
    pub delete: Vec<Triple>,
}

/// RDF-style triple
#[derive(Clone, Debug, PartialEq)]
pub struct Triple {
    /// Subject (UID or blank node)
    pub subject: Subject,
    /// Predicate name
    pub predicate: String,
    /// Object (UID or value)
    pub object: Object,
    /// Facets
    pub facets: Vec<(String, Value)>,
}

/// Subject of a triple
#[derive(Clone, Debug, PartialEq)]
pub enum Subject {
    /// Existing UID
    Uid(Uid),
    /// Blank node (to be assigned UID)
    Blank(String),
}

/// Object of a triple
#[derive(Clone, Debug, PartialEq)]
pub enum Object {
    /// Reference to another node
    Uid(Uid),
    /// Blank node reference
    Blank(String),
    /// Literal value
    Value(Value),
}
