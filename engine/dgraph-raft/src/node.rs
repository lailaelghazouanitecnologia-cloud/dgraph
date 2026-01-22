//! Raft node

use crate::{Log, NodeRole, RaftState};
use bytes::Bytes;
use std::sync::Arc;

/// Node identifier
pub type NodeId = u64;

/// Node configuration
#[derive(Clone, Debug)]
pub struct NodeConfig {
    /// This node's ID
    pub id: NodeId,
    /// Peer node IDs
    pub peers: Vec<NodeId>,
    /// Election timeout in milliseconds
    pub election_timeout_ms: u64,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            id: 1,
            peers: Vec::new(),
            election_timeout_ms: 150,
            heartbeat_interval_ms: 50,
        }
    }
}

/// Raft node
pub struct Node {
    config: NodeConfig,
    state: Arc<RaftState>,
    log: Arc<Log>,
}

impl Node {
    /// Create new node
    #[must_use]
    pub fn new(config: NodeConfig) -> Self {
        Self {
            config,
            state: Arc::new(RaftState::new()),
            log: Arc::new(Log::new()),
        }
    }

    /// Get node ID
    #[must_use]
    pub fn id(&self) -> NodeId {
        self.config.id
    }

    /// Get current role
    #[must_use]
    pub fn role(&self) -> NodeRole {
        self.state.role()
    }

    /// Get current term
    #[must_use]
    pub fn term(&self) -> u64 {
        self.state.term()
    }

    /// Check if this node is leader
    #[must_use]
    pub fn is_leader(&self) -> bool {
        self.state.role() == NodeRole::Leader
    }

    /// Propose a new entry (leader only)
    ///
    /// Returns entry index if successful
    pub fn propose(&self, data: Bytes) -> Option<u64> {
        if !self.is_leader() {
            return None;
        }
        let term = self.state.term();
        Some(self.log.append(term, data))
    }

    /// Get log reference
    #[must_use]
    pub fn log(&self) -> &Log {
        &self.log
    }

    /// Get state reference
    #[must_use]
    pub fn state(&self) -> &RaftState {
        &self.state
    }

    /// Get commit index
    #[must_use]
    pub fn commit_index(&self) -> u64 {
        self.state.commit_index()
    }

    /// Handle election timeout
    pub fn tick_election(&self) {
        match self.state.role() {
            NodeRole::Follower | NodeRole::Candidate => {
                // Start election
                let _new_term = self.state.start_election();
                self.state.vote_for(self.config.id);
                // Would send RequestVote to peers
            }
            NodeRole::Leader => {
                // Leaders don't have election timeout
            }
        }
    }

    /// Handle heartbeat (leader only)
    pub fn tick_heartbeat(&self) {
        if self.is_leader() {
            // Would send AppendEntries to peers
        }
    }

    /// Receive vote response
    pub fn receive_vote(&self, from: NodeId, term: u64, granted: bool) {
        if term > self.state.term() {
            self.state.become_follower(term, 0);
            return;
        }

        if granted && self.state.role() == NodeRole::Candidate {
            // Count votes, become leader if majority
            // Simplified: become leader immediately
            self.state.become_leader();
        }
    }
}
