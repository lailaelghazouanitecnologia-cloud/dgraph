//! dgraph-proto: Protocol buffer definitions
//!
//! Wire format for Dgraph. Minimal, explicit.

#![deny(missing_docs, clippy::all, clippy::pedantic)]

use bytes::Bytes;
use prost::Message;

/// Request wrapper
#[derive(Clone, Debug, Message)]
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
#[derive(Clone, Debug, Message)]
pub struct Response {
    /// JSON response data
    #[prost(bytes, tag = "1")]
    pub json: Bytes,
    /// Transaction context
    #[prost(message, optional, tag = "2")]
    pub txn: Option<TxnContext>,
    /// Latency info
    #[prost(message, optional, tag = "3")]
    pub latency: Option<Latency>,
}

/// Transaction context
#[derive(Clone, Debug, Message)]
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
    #[prost(bytes, repeated, tag = "4")]
    pub keys: Vec<Bytes>,
    /// Predicates modified
    #[prost(string, repeated, tag = "5")]
    pub preds: Vec<String>,
}

/// Latency information
#[derive(Clone, Debug, Default, Message)]
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
#[derive(Clone, Debug, Message)]
pub struct Mutation {
    /// Set triples (JSON)
    #[prost(bytes, tag = "1")]
    pub set_json: Bytes,
    /// Delete triples (JSON)
    #[prost(bytes, tag = "2")]
    pub delete_json: Bytes,
    /// Set triples (NQuads)
    #[prost(bytes, tag = "3")]
    pub set_nquads: Bytes,
    /// Delete triples (NQuads)
    #[prost(bytes, tag = "4")]
    pub del_nquads: Bytes,
    /// Condition
    #[prost(string, tag = "5")]
    pub cond: String,
    /// Commit now
    #[prost(bool, tag = "6")]
    pub commit_now: bool,
}

/// Operation (alter schema, etc.)
#[derive(Clone, Debug, Message)]
pub struct Operation {
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
#[derive(Clone, Debug, Message)]
pub struct Version {
    /// Version tag
    #[prost(string, tag = "1")]
    pub tag: String,
}
