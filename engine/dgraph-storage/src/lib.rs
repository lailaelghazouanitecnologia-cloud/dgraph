//! dgraph-storage: Storage engine
//!
//! Posting lists with MVCC. No bullshit.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod posting;
mod mvcc;
mod store;

pub use posting::{Facet, Op, Posting, PostingKind, PostingList, PostingType, ValType};
pub use mvcc::{MvccLayer, Txn, TxnContext};
pub use store::{Store, StoreConfig};
