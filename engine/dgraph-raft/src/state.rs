//! Raft state machine

use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Node role in cluster
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeRole {
    /// Follower - receives log entries from leader
    Follower,
    /// Candidate - requesting votes
    Candidate,
    /// Leader - handles all client requests
    Leader,
}

/// Persistent Raft state
pub struct RaftState {
    /// Current term
    current_term: AtomicU64,
    /// Voted for in current term (0 = none)
    voted_for: AtomicU64,
    /// Current role
    role: RwLock<NodeRole>,
    /// Leader ID (0 = unknown)
    leader_id: AtomicU64,
    /// Commit index
    commit_index: AtomicU64,
    /// Last applied index
    last_applied: AtomicU64,
}

impl RaftState {
    /// Create new initial state
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_term: AtomicU64::new(0),
            voted_for: AtomicU64::new(0),
            role: RwLock::new(NodeRole::Follower),
            leader_id: AtomicU64::new(0),
            commit_index: AtomicU64::new(0),
            last_applied: AtomicU64::new(0),
        }
    }

    /// Get current term
    #[must_use]
    pub fn term(&self) -> u64 {
        self.current_term.load(Ordering::SeqCst)
    }

    /// Set current term
    pub fn set_term(&self, term: u64) {
        self.current_term.store(term, Ordering::SeqCst);
    }

    /// Increment term and become candidate
    pub fn start_election(&self) -> u64 {
        let new_term = self.current_term.fetch_add(1, Ordering::SeqCst) + 1;
        *self.role.write() = NodeRole::Candidate;
        self.voted_for.store(0, Ordering::SeqCst);
        new_term
    }

    /// Get voted for
    #[must_use]
    pub fn voted_for(&self) -> u64 {
        self.voted_for.load(Ordering::SeqCst)
    }

    /// Cast vote
    pub fn vote_for(&self, node_id: u64) {
        self.voted_for.store(node_id, Ordering::SeqCst);
    }

    /// Get current role
    #[must_use]
    pub fn role(&self) -> NodeRole {
        *self.role.read()
    }

    /// Become follower
    pub fn become_follower(&self, term: u64, leader_id: u64) {
        self.current_term.store(term, Ordering::SeqCst);
        *self.role.write() = NodeRole::Follower;
        self.leader_id.store(leader_id, Ordering::SeqCst);
        self.voted_for.store(0, Ordering::SeqCst);
    }

    /// Become leader
    pub fn become_leader(&self) {
        *self.role.write() = NodeRole::Leader;
    }

    /// Get leader ID
    #[must_use]
    pub fn leader_id(&self) -> u64 {
        self.leader_id.load(Ordering::SeqCst)
    }

    /// Get commit index
    #[must_use]
    pub fn commit_index(&self) -> u64 {
        self.commit_index.load(Ordering::SeqCst)
    }

    /// Set commit index
    pub fn set_commit_index(&self, index: u64) {
        self.commit_index.store(index, Ordering::SeqCst);
    }

    /// Get last applied
    #[must_use]
    pub fn last_applied(&self) -> u64 {
        self.last_applied.load(Ordering::SeqCst)
    }

    /// Set last applied
    pub fn set_last_applied(&self, index: u64) {
        self.last_applied.store(index, Ordering::SeqCst);
    }
}

impl Default for RaftState {
    fn default() -> Self {
        Self::new()
    }
}
