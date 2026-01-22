//! dgraph-storage: Storage engine
//!
//! Posting lists with MVCC. No bullshit.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod posting;
mod mvcc;
mod store;
mod tokenizer;
mod index;

pub use posting::{Facet, Op, Posting, PostingKind, PostingList, PostingType, ValType};
pub use mvcc::{MvccLayer, Txn, TxnContext};
pub use store::{Store, StoreConfig};
pub use tokenizer::{
    BoolTokenizer, ExactTokenizer, FloatTokenizer, FulltextTokenizer, HashTokenizer,
    IntTokenizer, TermTokenizer, Token, Tokenizer, TokenizerId, TokenizerRegistry,
    TrigramTokenizer,
};
pub use index::{
    generate_float_index_mutations, generate_index_mutations, generate_int_index_mutations,
    CountKey, IndexEntry, IndexLookup, IndexMutation, IndexOp, IndexSpec, ReverseKey,
};
