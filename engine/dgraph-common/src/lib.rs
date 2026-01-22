//! dgraph-common: Core utilities and types
//!
//! Tiger-style: explicit, correct, no bullshit.

#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    clippy::all,
    clippy::pedantic,
    clippy::nursery
)]
#![allow(clippy::module_name_repetitions)]

mod error;
mod uid;
mod key;
mod timestamp;

pub use error::{Error, Result};
pub use uid::Uid;
pub use key::{Key, KeyKind};
pub use timestamp::Timestamp;

/// Panic in debug, return error in release.
#[macro_export]
macro_rules! assert_or_err {
    ($cond:expr, $err:expr) => {
        if !$cond {
            debug_assert!(false, "assertion failed: {}", stringify!($cond));
            return Err($err);
        }
    };
}

/// Constants
pub mod consts {
    /// Maximum key size in bytes
    pub const MAX_KEY_SIZE: usize = 4096;
    /// Maximum value size in bytes
    pub const MAX_VALUE_SIZE: usize = 64 * 1024 * 1024; // 64MB
    /// Default page size
    pub const PAGE_SIZE: usize = 4096;
}
