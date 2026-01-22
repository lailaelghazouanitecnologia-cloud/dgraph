//! dgraph-proto: Protocol buffer definitions
//!
//! Wire format for Dgraph. Minimal, explicit.

#![deny(missing_docs, clippy::all, clippy::pedantic)]

use prost::Message;

/// Request wrapper
#[derive(Clone, PartialEq, Message)]
pub struct Request {
    /// Query string
    #[prost(string, tag = "1")]
    pub query: String,
    /// Variables
    #[prost(map = "string, string", tag = "2")]
    pub vars: std::collections::HashMap<String, String>,
    /// Start timestamp
    #[prost(uint64, tag = "3")]
    pub start_ts: u64,
    /// Read only transaction
    #[prost(bool, tag = "4")]
    pub read_only: bool,
    /// Best effort (stale reads ok)
    #[prost(bool, tag = "5")]
    pub best_effort: bool,
}

/// Response wrapper
#[derive(Clone, PartialEq, Message)]
pub struct Response {
    /// JSON response data
    #[prost(bytes = "vec", tag = "1")]
    pub json: Vec<u8>,
    /// Transaction context
    #[prost(message, optional, tag = "2")]
    pub txn: Option<TxnContext>,
    /// Latency info
    #[prost(message, optional, tag = "3")]
    pub latency: Option<Latency>,
}

/// Transaction context
#[derive(Clone, PartialEq, Message)]
pub struct TxnContext {
    /// Start timestamp
    #[prost(uint64, tag = "1")]
    pub start_ts: u64,
    /// Commit timestamp
    #[prost(uint64, tag = "2")]
    pub commit_ts: u64,
    /// Aborted flag
    #[prost(bool, tag = "3")]
    pub aborted: bool,
    /// Keys modified
    #[prost(bytes = "vec", repeated, tag = "4")]
    pub keys: Vec<Vec<u8>>,
    /// Predicates modified
    #[prost(string, repeated, tag = "5")]
    pub preds: Vec<String>,
}

/// Latency information
#[derive(Clone, PartialEq, Message)]
pub struct Latency {
    /// Parsing latency in nanoseconds
    #[prost(uint64, tag = "1")]
    pub parsing_ns: u64,
    /// Processing latency in nanoseconds
    #[prost(uint64, tag = "2")]
    pub processing_ns: u64,
    /// Encoding latency in nanoseconds
    #[prost(uint64, tag = "3")]
    pub encoding_ns: u64,
    /// Total latency in nanoseconds
    #[prost(uint64, tag = "4")]
    pub total_ns: u64,
}

/// Mutation
#[derive(Clone, PartialEq, Message)]
pub struct Mutation {
    /// Set triples (JSON)
    #[prost(bytes = "vec", tag = "1")]
    pub set_json: Vec<u8>,
    /// Delete triples (JSON)
    #[prost(bytes = "vec", tag = "2")]
    pub delete_json: Vec<u8>,
    /// Set triples (NQuads)
    #[prost(bytes = "vec", tag = "3")]
    pub set_nquads: Vec<u8>,
    /// Delete triples (NQuads)
    #[prost(bytes = "vec", tag = "4")]
    pub del_nquads: Vec<u8>,
    /// Condition
    #[prost(string, tag = "5")]
    pub cond: String,
    /// Commit now
    #[prost(bool, tag = "6")]
    pub commit_now: bool,
}

/// Operation (alter schema, etc.)
#[derive(Clone, PartialEq, Message)]
pub struct ProtoOperation {
    /// Schema string
    #[prost(string, tag = "1")]
    pub schema: String,
    /// Drop all data
    #[prost(bool, tag = "2")]
    pub drop_all: bool,
    /// Drop attribute
    #[prost(string, tag = "3")]
    pub drop_attr: String,
    /// Drop operation
    #[prost(enumeration = "DropOp", tag = "4")]
    pub drop_op: i32,
}

/// Drop operation type
#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum DropOp {
    /// No drop
    None = 0,
    /// Drop all
    All = 1,
    /// Drop data only
    Data = 2,
    /// Drop type
    Type = 3,
}

/// Version info
#[derive(Clone, PartialEq, Message)]
pub struct Version {
    /// Version tag
    #[prost(string, tag = "1")]
    pub tag: String,
}
