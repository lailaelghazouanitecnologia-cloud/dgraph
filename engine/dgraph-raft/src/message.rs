//! Raft protocol messages
//!
//! Implements the core Raft RPC messages.

use bytes::Bytes;
use crate::log::Entry;

/// Raft message types
#[derive(Clone, Debug)]
pub enum Message {
    /// Request vote from peers during election
    RequestVote(RequestVote),
    /// Response to vote request
    RequestVoteResponse(RequestVoteResponse),
    /// Append entries (heartbeat or replication)
    AppendEntries(AppendEntries),
    /// Response to append entries
    AppendEntriesResponse(AppendEntriesResponse),
    /// Install snapshot
    InstallSnapshot(InstallSnapshot),
    /// Response to install snapshot
    InstallSnapshotResponse(InstallSnapshotResponse),
}

/// RequestVote RPC
#[derive(Clone, Debug)]
pub struct RequestVote {
    /// Candidate's term
    pub term: u64,
    /// Candidate requesting vote
    pub candidate_id: u64,
    /// Index of candidate's last log entry
    pub last_log_index: u64,
    /// Term of candidate's last log entry
    pub last_log_term: u64,
}

impl RequestVote {
    /// Create new vote request
    #[must_use]
    pub fn new(term: u64, candidate_id: u64, last_log_index: u64, last_log_term: u64) -> Self {
        Self {
            term,
            candidate_id,
            last_log_index,
            last_log_term,
        }
    }
}

/// RequestVote response
#[derive(Clone, Debug)]
pub struct RequestVoteResponse {
    /// Responder's term (for candidate to update itself)
    pub term: u64,
    /// True means candidate received vote
    pub vote_granted: bool,
    /// Node that sent this response
    pub from: u64,
}

impl RequestVoteResponse {
    /// Create grant response
    #[must_use]
    pub fn grant(term: u64, from: u64) -> Self {
        Self {
            term,
            vote_granted: true,
            from,
        }
    }

    /// Create deny response
    #[must_use]
    pub fn deny(term: u64, from: u64) -> Self {
        Self {
            term,
            vote_granted: false,
            from,
        }
    }
}

/// AppendEntries RPC (heartbeat + log replication)
#[derive(Clone, Debug)]
pub struct AppendEntries {
    /// Leader's term
    pub term: u64,
    /// Leader's ID (for redirects)
    pub leader_id: u64,
    /// Index of log entry immediately preceding new ones
    pub prev_log_index: u64,
    /// Term of prev_log_index entry
    pub prev_log_term: u64,
    /// Log entries to store (empty for heartbeat)
    pub entries: Vec<Entry>,
    /// Leader's commit index
    pub leader_commit: u64,
}

impl AppendEntries {
    /// Create heartbeat (empty append entries)
    #[must_use]
    pub fn heartbeat(term: u64, leader_id: u64, prev_log_index: u64, prev_log_term: u64, leader_commit: u64) -> Self {
        Self {
            term,
            leader_id,
            prev_log_index,
            prev_log_term,
            entries: Vec::new(),
            leader_commit,
        }
    }

    /// Create append entries with log entries
    #[must_use]
    pub fn with_entries(term: u64, leader_id: u64, prev_log_index: u64, prev_log_term: u64, entries: Vec<Entry>, leader_commit: u64) -> Self {
        Self {
            term,
            leader_id,
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit,
        }
    }

    /// Check if this is a heartbeat (no entries)
    #[must_use]
    pub fn is_heartbeat(&self) -> bool {
        self.entries.is_empty()
    }
}

/// AppendEntries response
#[derive(Clone, Debug)]
pub struct AppendEntriesResponse {
    /// Responder's term
    pub term: u64,
    /// True if follower contained entry matching prev_log_index and prev_log_term
    pub success: bool,
    /// Node that sent this response
    pub from: u64,
    /// Hint for next index (optimization)
    pub match_index: u64,
}

impl AppendEntriesResponse {
    /// Create success response
    #[must_use]
    pub fn success(term: u64, from: u64, match_index: u64) -> Self {
        Self {
            term,
            success: true,
            from,
            match_index,
        }
    }

    /// Create failure response
    #[must_use]
    pub fn failure(term: u64, from: u64) -> Self {
        Self {
            term,
            success: false,
            from,
            match_index: 0,
        }
    }
}

/// InstallSnapshot RPC
#[derive(Clone, Debug)]
pub struct InstallSnapshot {
    /// Leader's term
    pub term: u64,
    /// Leader's ID
    pub leader_id: u64,
    /// Index of last entry in snapshot
    pub last_included_index: u64,
    /// Term of last entry in snapshot
    pub last_included_term: u64,
    /// Byte offset of this chunk
    pub offset: u64,
    /// Snapshot chunk data
    pub data: Bytes,
    /// True if this is the last chunk
    pub done: bool,
}

impl InstallSnapshot {
    /// Create a new snapshot message
    #[must_use]
    pub fn new(term: u64, leader_id: u64, last_included_index: u64, last_included_term: u64, offset: u64, data: Bytes, done: bool) -> Self {
        Self {
            term,
            leader_id,
            last_included_index,
            last_included_term,
            offset,
            data,
            done,
        }
    }
}

/// InstallSnapshot response
#[derive(Clone, Debug)]
pub struct InstallSnapshotResponse {
    /// Responder's term
    pub term: u64,
    /// Node that sent this response
    pub from: u64,
}

impl InstallSnapshotResponse {
    /// Create response
    #[must_use]
    pub fn new(term: u64, from: u64) -> Self {
        Self { term, from }
    }
}

/// Ready struct - contains messages to send after processing
#[derive(Clone, Debug, Default)]
pub struct Ready {
    /// Messages to send to other nodes
    pub messages: Vec<(u64, Message)>,
    /// Entries to persist
    pub entries_to_persist: Vec<Entry>,
    /// Entries to apply to state machine
    pub entries_to_apply: Vec<Entry>,
    /// Snapshot to apply (if any)
    pub snapshot: Option<Snapshot>,
    /// Whether leader state changed
    pub leader_changed: bool,
}

impl Ready {
    /// Create new empty ready
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there's work to do
    #[must_use]
    pub fn has_work(&self) -> bool {
        !self.messages.is_empty() || !self.entries_to_persist.is_empty() || !self.entries_to_apply.is_empty() || self.snapshot.is_some()
    }
}

/// Snapshot metadata and data
#[derive(Clone, Debug)]
pub struct Snapshot {
    /// Index of last entry in snapshot
    pub last_included_index: u64,
    /// Term of last entry in snapshot
    pub last_included_term: u64,
    /// Snapshot data
    pub data: Bytes,
}

impl Snapshot {
    /// Create a new snapshot
    #[must_use]
    pub fn new(last_included_index: u64, last_included_term: u64, data: Bytes) -> Self {
        Self {
            last_included_index,
            last_included_term,
            data,
        }
    }
}
