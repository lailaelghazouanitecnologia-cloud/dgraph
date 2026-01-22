//! dgraph-raft: Raft consensus engine
//!
//! Simplified Raft for distributed consensus.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod state;
mod log;
mod node;

pub use state::{RaftState, NodeRole};
pub use log::{Entry, Log};
pub use node::{Node, NodeConfig, NodeId};
