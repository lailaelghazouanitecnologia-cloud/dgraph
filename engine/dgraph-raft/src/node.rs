//! Raft node
//!
//! Implements the core Raft consensus algorithm.

use crate::log::Entry;
use crate::message::{
    AppendEntries, AppendEntriesResponse, InstallSnapshot, InstallSnapshotResponse, Message,
    Ready, RequestVote, RequestVoteResponse,
};
use crate::{Log, NodeRole, RaftState};
use bytes::Bytes;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
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
    /// Maximum entries per append
    pub max_entries_per_append: usize,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            id: 1,
            peers: Vec::new(),
            election_timeout_ms: 150,
            heartbeat_interval_ms: 50,
            max_entries_per_append: 100,
        }
    }
}

/// Per-peer replication state (leader only)
#[derive(Clone, Debug)]
struct PeerProgress {
    /// Index of next entry to send
    next_index: u64,
    /// Highest index known to be replicated
    match_index: u64,
    /// Whether this peer is active
    active: bool,
}

impl PeerProgress {
    fn new(next_index: u64) -> Self {
        Self {
            next_index,
            match_index: 0,
            active: true,
        }
    }
}

/// Raft node
pub struct Node {
    config: NodeConfig,
    state: Arc<RaftState>,
    log: Arc<Log>,
    /// Votes received in current election (candidate only)
    votes_received: RwLock<HashSet<NodeId>>,
    /// Peer progress (leader only)
    peer_progress: RwLock<HashMap<NodeId, PeerProgress>>,
    /// Pending messages to send
    pending_messages: RwLock<Vec<(NodeId, Message)>>,
}

impl Node {
    /// Create new node
    #[must_use]
    pub fn new(config: NodeConfig) -> Self {
        Self {
            config,
            state: Arc::new(RaftState::new()),
            log: Arc::new(Log::new()),
            votes_received: RwLock::new(HashSet::new()),
            peer_progress: RwLock::new(HashMap::new()),
            pending_messages: RwLock::new(Vec::new()),
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

    /// Get leader ID
    #[must_use]
    pub fn leader_id(&self) -> Option<NodeId> {
        let id = self.state.leader_id();
        if id == 0 {
            None
        } else {
            Some(id)
        }
    }

    /// Propose a new entry (leader only)
    ///
    /// Returns entry index if successful
    pub fn propose(&self, data: Bytes) -> Option<u64> {
        if !self.is_leader() {
            return None;
        }
        let term = self.state.term();
        let index = self.log.append(term, data);

        // Schedule replication to peers
        self.broadcast_append_entries();

        Some(index)
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
                self.start_election();
            }
            NodeRole::Leader => {
                // Leaders don't have election timeout
            }
        }
    }

    /// Handle heartbeat (leader only)
    pub fn tick_heartbeat(&self) {
        if self.is_leader() {
            self.broadcast_append_entries();
        }
    }

    /// Start an election
    fn start_election(&self) {
        let new_term = self.state.start_election();
        self.state.vote_for(self.config.id);

        // Clear votes and add self-vote
        {
            let mut votes = self.votes_received.write();
            votes.clear();
            votes.insert(self.config.id);
        }

        // If single node, become leader immediately
        if self.config.peers.is_empty() {
            self.become_leader();
            return;
        }

        // Send RequestVote to all peers
        let last_log_index = self.log.last_index();
        let last_log_term = self.log.last_term();

        let request = RequestVote::new(new_term, self.config.id, last_log_index, last_log_term);

        let mut messages = self.pending_messages.write();
        for &peer in &self.config.peers {
            messages.push((peer, Message::RequestVote(request.clone())));
        }
    }

    /// Become leader
    fn become_leader(&self) {
        self.state.become_leader();

        // Initialize peer progress
        {
            let next_index = self.log.last_index() + 1;
            let mut progress = self.peer_progress.write();
            progress.clear();
            for &peer in &self.config.peers {
                progress.insert(peer, PeerProgress::new(next_index));
            }
        } // Drop write lock before calling broadcast

        // Send initial empty append entries (heartbeat)
        self.broadcast_append_entries();
    }

    /// Broadcast append entries to all peers
    fn broadcast_append_entries(&self) {
        if !self.is_leader() {
            return;
        }

        let progress = self.peer_progress.read();
        let mut messages = self.pending_messages.write();

        for &peer in &self.config.peers {
            if let Some(peer_progress) = progress.get(&peer) {
                let prev_log_index = peer_progress.next_index.saturating_sub(1);
                let prev_log_term = if prev_log_index == 0 {
                    0
                } else {
                    self.log.get(prev_log_index).map_or(0, |e| e.term)
                };

                let entries = self.log.entries_from(peer_progress.next_index);
                let entries: Vec<Entry> = entries
                    .into_iter()
                    .take(self.config.max_entries_per_append)
                    .collect();

                let append = AppendEntries::with_entries(
                    self.state.term(),
                    self.config.id,
                    prev_log_index,
                    prev_log_term,
                    entries,
                    self.state.commit_index(),
                );

                messages.push((peer, Message::AppendEntries(append)));
            }
        }
    }

    /// Step processes incoming messages
    pub fn step(&self, from: NodeId, msg: Message) {
        match msg {
            Message::RequestVote(req) => self.handle_request_vote(from, req),
            Message::RequestVoteResponse(resp) => self.handle_request_vote_response(resp),
            Message::AppendEntries(req) => self.handle_append_entries(from, req),
            Message::AppendEntriesResponse(resp) => self.handle_append_entries_response(resp),
            Message::InstallSnapshot(req) => self.handle_install_snapshot(from, req),
            Message::InstallSnapshotResponse(resp) => self.handle_install_snapshot_response(resp),
        }
    }

    /// Handle RequestVote RPC
    fn handle_request_vote(&self, from: NodeId, req: RequestVote) {
        let current_term = self.state.term();

        // If candidate's term is behind, reject
        if req.term < current_term {
            self.send_vote_response(from, current_term, false);
            return;
        }

        // If candidate's term is ahead, become follower
        if req.term > current_term {
            self.state.become_follower(req.term, 0);
        }

        // Check if we can vote for this candidate
        let voted_for = self.state.voted_for();
        let can_vote = voted_for == 0 || voted_for == req.candidate_id;

        // Check if candidate's log is at least as up-to-date as ours
        let last_log_index = self.log.last_index();
        let last_log_term = self.log.last_term();
        let log_ok = req.last_log_term > last_log_term
            || (req.last_log_term == last_log_term && req.last_log_index >= last_log_index);

        if can_vote && log_ok {
            self.state.vote_for(req.candidate_id);
            self.send_vote_response(from, self.state.term(), true);
        } else {
            self.send_vote_response(from, self.state.term(), false);
        }
    }

    /// Send vote response
    fn send_vote_response(&self, to: NodeId, term: u64, granted: bool) {
        let response = if granted {
            RequestVoteResponse::grant(term, self.config.id)
        } else {
            RequestVoteResponse::deny(term, self.config.id)
        };
        self.pending_messages
            .write()
            .push((to, Message::RequestVoteResponse(response)));
    }

    /// Handle RequestVote response
    fn handle_request_vote_response(&self, resp: RequestVoteResponse) {
        // If response is from a higher term, become follower
        if resp.term > self.state.term() {
            self.state.become_follower(resp.term, 0);
            return;
        }

        // Only process if we're still a candidate
        if self.state.role() != NodeRole::Candidate {
            return;
        }

        if resp.vote_granted {
            let mut votes = self.votes_received.write();
            votes.insert(resp.from);

            // Check if we have majority
            let total_nodes = 1 + self.config.peers.len();
            let majority = total_nodes / 2 + 1;
            if votes.len() >= majority {
                drop(votes);
                self.become_leader();
            }
        }
    }

    /// Handle AppendEntries RPC
    fn handle_append_entries(&self, from: NodeId, req: AppendEntries) {
        let current_term = self.state.term();

        // Reply false if term < currentTerm
        if req.term < current_term {
            self.send_append_response(from, current_term, false, 0);
            return;
        }

        // If term >= currentTerm, become follower
        if req.term >= current_term {
            self.state.become_follower(req.term, req.leader_id);
        }

        // Reply false if log doesn't contain an entry at prevLogIndex
        // whose term matches prevLogTerm
        if req.prev_log_index > 0 {
            match self.log.get(req.prev_log_index) {
                Some(entry) if entry.term == req.prev_log_term => {}
                _ => {
                    self.send_append_response(from, self.state.term(), false, 0);
                    return;
                }
            }
        }

        // Append any new entries not already in the log
        let mut new_entries_start = req.prev_log_index + 1;
        for entry in &req.entries {
            if let Some(existing) = self.log.get(entry.index) {
                if existing.term != entry.term {
                    // Conflict: truncate and append
                    self.log.truncate_from(entry.index);
                    break;
                }
                new_entries_start = entry.index + 1;
            } else {
                break;
            }
        }

        // Append remaining entries
        for entry in &req.entries {
            if entry.index >= new_entries_start {
                self.log.append(entry.term, entry.data.clone());
            }
        }

        // Update commit index
        if req.leader_commit > self.state.commit_index() {
            let new_commit = std::cmp::min(req.leader_commit, self.log.last_index());
            self.state.set_commit_index(new_commit);
        }

        let match_index = if req.entries.is_empty() {
            req.prev_log_index
        } else {
            req.entries.last().map_or(req.prev_log_index, |e| e.index)
        };

        self.send_append_response(from, self.state.term(), true, match_index);
    }

    /// Send append entries response
    fn send_append_response(&self, to: NodeId, term: u64, success: bool, match_index: u64) {
        let response = if success {
            AppendEntriesResponse::success(term, self.config.id, match_index)
        } else {
            AppendEntriesResponse::failure(term, self.config.id)
        };
        self.pending_messages
            .write()
            .push((to, Message::AppendEntriesResponse(response)));
    }

    /// Handle AppendEntries response
    fn handle_append_entries_response(&self, resp: AppendEntriesResponse) {
        // If response is from a higher term, become follower
        if resp.term > self.state.term() {
            self.state.become_follower(resp.term, 0);
            return;
        }

        // Only process if we're still leader
        if !self.is_leader() {
            return;
        }

        let mut progress = self.peer_progress.write();
        if let Some(peer) = progress.get_mut(&resp.from) {
            if resp.success {
                // Update match_index and next_index
                peer.match_index = resp.match_index;
                peer.next_index = resp.match_index + 1;
                peer.active = true;

                // Check if we can advance commit index
                drop(progress);
                self.maybe_advance_commit();
            } else {
                // Decrement next_index and retry
                peer.next_index = peer.next_index.saturating_sub(1).max(1);
            }
        }
    }

    /// Try to advance commit index based on match indexes
    fn maybe_advance_commit(&self) {
        let progress = self.peer_progress.read();
        let current_commit = self.state.commit_index();

        // Find the highest index that a majority has replicated
        for n in (current_commit + 1)..=self.log.last_index() {
            if let Some(entry) = self.log.get(n) {
                // Only commit entries from current term
                if entry.term != self.state.term() {
                    continue;
                }

                // Count how many nodes have this entry
                let mut count = 1; // Self
                for peer_progress in progress.values() {
                    if peer_progress.match_index >= n {
                        count += 1;
                    }
                }

                // Check if majority
                let total = 1 + self.config.peers.len();
                if count > total / 2 {
                    self.state.set_commit_index(n);
                }
            }
        }
    }

    /// Handle InstallSnapshot RPC
    fn handle_install_snapshot(&self, from: NodeId, req: InstallSnapshot) {
        let current_term = self.state.term();

        if req.term < current_term {
            self.send_snapshot_response(from, current_term);
            return;
        }

        if req.term > current_term {
            self.state.become_follower(req.term, req.leader_id);
        }

        // For simplicity, we don't handle chunked snapshots here
        // In a full implementation, you'd buffer chunks and apply when done

        self.send_snapshot_response(from, self.state.term());
    }

    /// Send snapshot response
    fn send_snapshot_response(&self, to: NodeId, term: u64) {
        let response = InstallSnapshotResponse::new(term, self.config.id);
        self.pending_messages
            .write()
            .push((to, Message::InstallSnapshotResponse(response)));
    }

    /// Handle InstallSnapshot response
    fn handle_install_snapshot_response(&self, resp: InstallSnapshotResponse) {
        if resp.term > self.state.term() {
            self.state.become_follower(resp.term, 0);
        }
    }

    /// Get ready with pending work
    pub fn ready(&self) -> Ready {
        let mut ready = Ready::new();

        // Collect pending messages
        {
            let mut messages = self.pending_messages.write();
            ready.messages = std::mem::take(&mut *messages);
        }

        // Collect entries to apply
        let commit_index = self.state.commit_index();
        let last_applied = self.state.last_applied();
        if commit_index > last_applied {
            for i in (last_applied + 1)..=commit_index {
                if let Some(entry) = self.log.get(i) {
                    ready.entries_to_apply.push(entry);
                }
            }
        }

        ready
    }

    /// Advance - called after Ready is processed
    pub fn advance(&self, ready: &Ready) {
        // Update last_applied
        if let Some(last) = ready.entries_to_apply.last() {
            self.state.set_last_applied(last.index);
        }
    }

    /// Get the quorum size
    #[must_use]
    pub fn quorum(&self) -> usize {
        (1 + self.config.peers.len()) / 2 + 1
    }

    /// Receive vote response (legacy method for compatibility)
    pub fn receive_vote(&self, from: NodeId, term: u64, granted: bool) {
        let response = if granted {
            RequestVoteResponse::grant(term, from)
        } else {
            RequestVoteResponse::deny(term, from)
        };
        self.handle_request_vote_response(response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_node_becomes_leader() {
        let config = NodeConfig {
            id: 1,
            peers: Vec::new(),
            ..Default::default()
        };
        let node = Node::new(config);

        // Trigger election
        node.tick_election();

        assert!(node.is_leader());
        assert_eq!(node.role(), NodeRole::Leader);
    }

    #[test]
    fn propose_as_leader() {
        let config = NodeConfig {
            id: 1,
            peers: Vec::new(),
            ..Default::default()
        };
        let node = Node::new(config);
        node.tick_election();

        let index = node.propose(Bytes::from("test")).unwrap();
        assert_eq!(index, 1);
        assert_eq!(node.log().len(), 1);
    }

    #[test]
    fn propose_as_follower_fails() {
        let config = NodeConfig::default();
        let node = Node::new(config);

        assert!(node.propose(Bytes::from("test")).is_none());
    }

    #[test]
    fn vote_request_granted() {
        let config = NodeConfig {
            id: 1,
            peers: vec![2],
            ..Default::default()
        };
        let node = Node::new(config);

        // Receive vote request from node 2 for term 1
        let request = RequestVote::new(1, 2, 0, 0);
        node.step(2, Message::RequestVote(request));

        // Should have granted vote
        assert_eq!(node.state.voted_for(), 2);
    }

    #[test]
    fn vote_request_denied_lower_term() {
        let config = NodeConfig {
            id: 1,
            peers: vec![2],
            ..Default::default()
        };
        let node = Node::new(config);

        // Set node to term 5
        node.state.set_term(5);

        // Receive vote request for term 1 (lower)
        let request = RequestVote::new(1, 2, 0, 0);
        node.step(2, Message::RequestVote(request));

        // Should not have granted vote
        assert_eq!(node.state.voted_for(), 0);
    }

    #[test]
    fn quorum_calculation() {
        // Single node
        let config1 = NodeConfig {
            id: 1,
            peers: Vec::new(),
            ..Default::default()
        };
        assert_eq!(Node::new(config1).quorum(), 1);

        // 3 nodes (1 + 2 peers)
        let config3 = NodeConfig {
            id: 1,
            peers: vec![2, 3],
            ..Default::default()
        };
        assert_eq!(Node::new(config3).quorum(), 2);

        // 5 nodes (1 + 4 peers)
        let config5 = NodeConfig {
            id: 1,
            peers: vec![2, 3, 4, 5],
            ..Default::default()
        };
        assert_eq!(Node::new(config5).quorum(), 3);
    }
}
