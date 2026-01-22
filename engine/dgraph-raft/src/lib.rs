//! dgraph-raft: Raft consensus engine
//!
//! Implements Raft consensus for distributed coordination.
//! Based on the Raft paper by Ongaro and Ousterhout.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod state;
mod log;
mod message;
mod node;

pub use state::{RaftState, NodeRole};
pub use log::{Entry, Log};
pub use message::{
    AppendEntries, AppendEntriesResponse, InstallSnapshot, InstallSnapshotResponse, Message,
    Ready, RequestVote, RequestVoteResponse, Snapshot,
};
pub use node::{Node, NodeConfig, NodeId};
