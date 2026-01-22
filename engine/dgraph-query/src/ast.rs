//! AST: Abstract Syntax Tree for DQL
//!
//! Complete structures matching Dgraph's GraphQuery and SubGraph.
//! Simple, explicit structures. No magic.

#![allow(missing_docs)] // Many enum variants, doc comments are noise

use dgraph_common::Uid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Root Operations
// ============================================================================

/// Root operation
#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    /// Query operation
    Query(Query),
    /// Mutation operation
    Mutation(Mutation),
}

// ============================================================================
// Query Structures (matches dql.GraphQuery)
// ============================================================================

/// Query block
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Query {
    /// Query name/alias (e.g., "me" in "me(func: ...)")
    pub alias: Option<String>,
    /// Root function (eq, has, uid, etc.)
    pub func: Option<Function>,
    /// Starting UIDs (direct uid specification)
    pub uids: Vec<Uid>,
    /// Fields to fetch (children in the query tree)
    pub children: Vec<Query>,
    /// Predicate name (for child queries, e.g., "name", "friends")
    pub attr: Option<String>,
    /// Filter expression (@filter)
    pub filter: Option<Filter>,
    /// Facets filter (@facets filter)
    pub facets_filter: Option<Filter>,
    /// First N results
    pub first: Option<u32>,
    /// Skip N results
    pub offset: Option<u32>,
    /// After UID (pagination)
    pub after: Option<u64>,
    /// Order by specifications
    pub order: Option<Order>,
    /// Language preferences (name@en, name@*)
    pub langs: Vec<String>,

    // === Variables ===
    /// Variable name defined by this query (e.g., "x as ...")
    pub var: Option<String>,
    /// Variables this query needs
    pub needs_var: Vec<VarContext>,
    /// Facet variables (e.g., "@facets(L1 as weight)")
    pub facet_var: HashMap<String, String>,

    // === Directives ===
    /// @normalize directive
    pub normalize: bool,
    /// @recurse directive
    pub recurse: bool,
    /// @recurse arguments
    pub recurse_args: RecurseArgs,
    /// @cascade directive (list of predicates, "__all__" for all)
    pub cascade: Vec<String>,
    /// @ignorereflex directive
    pub ignore_reflex: bool,
    /// @groupby directive
    pub is_groupby: bool,
    /// @groupby attributes
    pub groupby_attrs: Vec<GroupByAttr>,
    /// Is count query (count(predicate))
    pub is_count: bool,
    /// expand() argument
    pub expand: Option<String>,

    // === Shortest Path ===
    /// Shortest path arguments
    pub shortest_path: Option<ShortestPathArgs>,

    // === Facets ===
    /// Requested facets
    pub facets: Option<FacetParams>,
    /// Facet ordering
    pub facets_order: Vec<FacetOrder>,

    // === Internal ===
    /// Is this an internal node (no actual query)?
    pub is_internal: bool,
    /// Is this an empty block (for variables)?
    pub is_empty: bool,
    /// Allowed predicates (for ACL)
    pub allowed_preds: Vec<String>,
}

/// Variable context
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VarContext {
    /// Variable name
    pub name: String,
    /// Variable type
    pub typ: VarType,
}

/// Variable type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VarType {
    #[default]
    Any = 0,
    Uid = 1,
    Value = 2,
    List = 3,
}

/// Recurse arguments
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecurseArgs {
    /// Maximum depth
    pub depth: Option<u64>,
    /// Allow loops in traversal
    pub allow_loop: bool,
}

/// GroupBy attribute
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupByAttr {
    /// Predicate to group by
    pub attr: String,
    /// Alias for output
    pub alias: Option<String>,
    /// Language preferences
    pub langs: Vec<String>,
}

/// Shortest path arguments
#[derive(Clone, Debug, PartialEq)]
pub struct ShortestPathArgs {
    /// From node (uid or variable)
    pub from: Function,
    /// To node (uid or variable)
    pub to: Function,
    /// Number of paths (k-shortest)
    pub num_paths: Option<u32>,
    /// Maximum weight
    pub max_weight: Option<f64>,
    /// Minimum weight
    pub min_weight: Option<f64>,
}

/// Facet parameters
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FacetParams {
    /// Request all facets
    pub all_keys: bool,
    /// Specific facet parameters
    pub params: Vec<FacetParam>,
}

/// Single facet parameter
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FacetParam {
    /// Facet key
    pub key: String,
    /// Alias for output
    pub alias: Option<String>,
}

/// Facet ordering
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FacetOrder {
    /// Facet key to order by
    pub key: String,
    /// Descending order
    pub desc: bool,
}

// ============================================================================
// Functions
// ============================================================================

/// Root function
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    /// Function name
    pub name: FuncName,
    /// Predicate (for comparison functions)
    pub attr: Option<String>,
    /// Language for the predicate
    pub lang: Option<String>,
    /// Arguments
    pub args: Vec<Value>,
    /// UIDs (for uid function)
    pub uids: Vec<u64>,
    /// Variables this function needs
    pub needs_var: Vec<VarContext>,
    /// Is this count(predicate)?
    pub is_count: bool,
    /// Is this val(variable)?
    pub is_value_var: bool,
    /// Is this len(variable)?
    pub is_len_var: bool,
}

impl Function {
    /// Create a simple function with name and args
    #[must_use]
    pub fn new(name: FuncName, args: Vec<Value>) -> Self {
        Self {
            name,
            attr: None,
            lang: None,
            args,
            uids: Vec::new(),
            needs_var: Vec::new(),
            is_count: false,
            is_value_var: false,
            is_len_var: false,
        }
    }

    /// Create a uid function
    #[must_use]
    pub fn uid(uids: Vec<u64>) -> Self {
        Self {
            name: FuncName::Uid,
            attr: None,
            lang: None,
            args: Vec::new(),
            uids,
            needs_var: Vec::new(),
            is_count: false,
            is_value_var: false,
            is_len_var: false,
        }
    }

    /// Create a has function
    #[must_use]
    pub fn has(predicate: impl Into<String>) -> Self {
        Self {
            name: FuncName::Has,
            attr: Some(predicate.into()),
            lang: None,
            args: Vec::new(),
            uids: Vec::new(),
            needs_var: Vec::new(),
            is_count: false,
            is_value_var: false,
            is_len_var: false,
        }
    }

    /// Create an eq function
    #[must_use]
    pub fn eq(predicate: impl Into<String>, value: Value) -> Self {
        Self {
            name: FuncName::Eq,
            attr: Some(predicate.into()),
            lang: None,
            args: vec![value],
            uids: Vec::new(),
            needs_var: Vec::new(),
            is_count: false,
            is_value_var: false,
            is_len_var: false,
        }
    }
}

/// Built-in function names
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FuncName {
    // Root/filter functions
    Uid,         // uid(0x1, 0x2, ...)
    UidIn,       // uid_in(predicate, uid)
    Eq,          // eq(predicate, value)
    Has,         // has(predicate)
    Ge,          // ge(predicate, value)
    Le,          // le(predicate, value)
    Gt,          // gt(predicate, value)
    Lt,          // lt(predicate, value)
    Between,     // between(predicate, val1, val2)
    AllOfTerms,  // allofterms(predicate, "word1 word2")
    AnyOfTerms,  // anyofterms(predicate, "word1 word2")
    AllOfText,   // alloftext(predicate, "phrase")
    AnyOfText,   // anyoftext(predicate, "phrase")
    Regexp,      // regexp(predicate, /pattern/)
    Match,       // match(predicate, "text", distance)
    Near,        // near(predicate, geo, distance)
    Within,      // within(predicate, geo)
    Contains,    // contains(predicate, geo)
    Intersects,  // intersects(predicate, geo)
    Type,        // type(TypeName)
    SimilarTo,   // similar_to(predicate, k, vector)

    // Value functions
    Val,   // val(variable)
    Len,   // len(variable)
    Count, // count(predicate)
}

impl FuncName {
    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "uid" => Some(Self::Uid),
            "uid_in" => Some(Self::UidIn),
            "eq" => Some(Self::Eq),
            "has" => Some(Self::Has),
            "ge" => Some(Self::Ge),
            "le" => Some(Self::Le),
            "gt" => Some(Self::Gt),
            "lt" => Some(Self::Lt),
            "between" => Some(Self::Between),
            "allofterms" => Some(Self::AllOfTerms),
            "anyofterms" => Some(Self::AnyOfTerms),
            "alloftext" => Some(Self::AllOfText),
            "anyoftext" => Some(Self::AnyOfText),
            "regexp" => Some(Self::Regexp),
            "match" => Some(Self::Match),
            "near" => Some(Self::Near),
            "within" => Some(Self::Within),
            "contains" => Some(Self::Contains),
            "intersects" => Some(Self::Intersects),
            "type" => Some(Self::Type),
            "similar_to" => Some(Self::SimilarTo),
            "val" => Some(Self::Val),
            "len" => Some(Self::Len),
            "count" => Some(Self::Count),
            _ => None,
        }
    }

    /// Get the string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Uid => "uid",
            Self::UidIn => "uid_in",
            Self::Eq => "eq",
            Self::Has => "has",
            Self::Ge => "ge",
            Self::Le => "le",
            Self::Gt => "gt",
            Self::Lt => "lt",
            Self::Between => "between",
            Self::AllOfTerms => "allofterms",
            Self::AnyOfTerms => "anyofterms",
            Self::AllOfText => "alloftext",
            Self::AnyOfText => "anyoftext",
            Self::Regexp => "regexp",
            Self::Match => "match",
            Self::Near => "near",
            Self::Within => "within",
            Self::Contains => "contains",
            Self::Intersects => "intersects",
            Self::Type => "type",
            Self::SimilarTo => "similar_to",
            Self::Val => "val",
            Self::Len => "len",
            Self::Count => "count",
        }
    }

    /// Is this a comparison function?
    #[must_use]
    pub const fn is_comparison(&self) -> bool {
        matches!(
            self,
            Self::Eq | Self::Ge | Self::Le | Self::Gt | Self::Lt | Self::Between
        )
    }

    /// Is this a term function?
    #[must_use]
    pub const fn is_term(&self) -> bool {
        matches!(
            self,
            Self::AllOfTerms | Self::AnyOfTerms | Self::AllOfText | Self::AnyOfText
        )
    }

    /// Is this a geo function?
    #[must_use]
    pub const fn is_geo(&self) -> bool {
        matches!(
            self,
            Self::Near | Self::Within | Self::Contains | Self::Intersects
        )
    }
}

// ============================================================================
// Filters
// ============================================================================

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

impl Filter {
    /// Create AND filter
    #[must_use]
    pub fn and(filters: Vec<Filter>) -> Self {
        Self::And(filters)
    }

    /// Create OR filter
    #[must_use]
    pub fn or(filters: Vec<Filter>) -> Self {
        Self::Or(filters)
    }

    /// Create NOT filter
    #[must_use]
    pub fn not(filter: Filter) -> Self {
        Self::Not(Box::new(filter))
    }

    /// Create function filter
    #[must_use]
    pub fn func(f: Function) -> Self {
        Self::Func(f)
    }
}

// ============================================================================
// Order
// ============================================================================

/// Order specification
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    /// Predicate to order by
    pub attr: String,
    /// Descending order
    pub desc: bool,
    /// Language preferences
    pub langs: Vec<String>,
}

impl Order {
    /// Create ascending order
    #[must_use]
    pub fn asc(attr: impl Into<String>) -> Self {
        Self {
            attr: attr.into(),
            desc: false,
            langs: Vec::new(),
        }
    }

    /// Create descending order
    #[must_use]
    pub fn desc(attr: impl Into<String>) -> Self {
        Self {
            attr: attr.into(),
            desc: true,
            langs: Vec::new(),
        }
    }
}

// ============================================================================
// Values
// ============================================================================

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
    /// Variable reference ($var)
    Var(String),
    /// Value variable reference (val(var))
    ValVar(String),
    /// Regex pattern
    Regex(String),
    /// Geo coordinates [lat, long] or GeoJSON
    Geo(String),
    /// DateTime value
    DateTime(String),
    /// List of values
    List(Vec<Value>),
    /// Vector (for similarity search)
    Vector(Vec<f64>),
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

    /// Try to get as f64
    #[must_use]
    pub const fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(i) => Some(*i as f64),
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

    /// Try to get as bool
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Check if this is a variable reference
    #[must_use]
    pub const fn is_var(&self) -> bool {
        matches!(self, Self::Var(_) | Self::ValVar(_))
    }
}

// ============================================================================
// Mutations
// ============================================================================

/// Mutation block
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mutation {
    /// Set operations
    pub set: Vec<Triple>,
    /// Delete operations
    pub delete: Vec<Triple>,
    /// Condition for upsert (@if)
    pub cond: Option<String>,
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
    /// Language tag
    pub lang: Option<String>,
}

/// Subject of a triple
#[derive(Clone, Debug, PartialEq)]
pub enum Subject {
    /// Existing UID
    Uid(Uid),
    /// Blank node (to be assigned UID)
    Blank(String),
    /// Variable reference
    Var(String),
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
    /// Variable reference
    Var(String),
}

// ============================================================================
// Math Expressions (for aggregation)
// ============================================================================

/// Math expression tree
#[derive(Clone, Debug, PartialEq)]
pub enum MathExpr {
    /// Constant value
    Const(f64),
    /// Variable reference
    Var(String),
    /// Binary operation
    BinOp {
        op: MathOp,
        left: Box<MathExpr>,
        right: Box<MathExpr>,
    },
    /// Unary operation
    UnaryOp { op: MathOp, expr: Box<MathExpr> },
    /// Function call
    Func { name: String, args: Vec<MathExpr> },
}

/// Math operation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Neg,
    Floor,
    Ceil,
    Ln,
    Exp,
    Sqrt,
    Min,
    Max,
    LogBase,
    Cond,
    Since,
}
