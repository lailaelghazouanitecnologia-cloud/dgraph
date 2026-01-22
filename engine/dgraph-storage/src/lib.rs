//! dgraph-storage: Storage engine
//!
//! Posting lists with MVCC. No bullshit.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod posting;
mod mvcc;
mod store;

pub use posting::{Posting, PostingList, PostingKind};
pub use mvcc::{MvccLayer, TxnContext};
pub use store::{Store, StoreConfig};
