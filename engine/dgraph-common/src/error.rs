//! Error types - explicit, no hidden variants

use thiserror::Error;

/// Result type for dgraph operations
pub type Result<T> = std::result::Result<T, Error>;

/// All possible errors - exhaustive, no catch-all
#[derive(Error, Debug)]
pub enum Error {
    /// Invalid UID (zero or reserved)
    #[error("invalid uid: {0}")]
    InvalidUid(u64),

    /// Key too large
    #[error("key size {size} exceeds max {max}")]
    KeyTooLarge { size: usize, max: usize },

    /// Value too large
    #[error("value size {size} exceeds max {max}")]
    ValueTooLarge { size: usize, max: usize },

    /// Key not found
    #[error("key not found")]
    NotFound,

    /// Corrupt data detected
    #[error("corruption: {0}")]
    Corruption(String),

    /// Transaction conflict
    #[error("transaction conflict at ts {0}")]
    TxnConflict(u64),

    /// Transaction too old
    #[error("transaction ts {txn_ts} older than watermark {watermark}")]
    TxnTooOld { txn_ts: u64, watermark: u64 },

    /// Storage error
    #[error("storage: {0}")]
    Storage(String),

    /// Raft error
    #[error("raft: {0}")]
    Raft(String),

    /// Schema error
    #[error("schema: {0}")]
    Schema(String),

    /// Query parse error
    #[error("parse error at position {pos}: {msg}")]
    Parse { pos: usize, msg: String },

    /// IO error
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Returns true if this error is retriable
    #[inline]
    #[must_use]
    pub const fn is_retriable(&self) -> bool {
        matches!(self, Self::TxnConflict(_))
    }

    /// Returns true if this is a corruption error
    #[inline]
    #[must_use]
    pub const fn is_corruption(&self) -> bool {
        matches!(self, Self::Corruption(_))
    }
}
