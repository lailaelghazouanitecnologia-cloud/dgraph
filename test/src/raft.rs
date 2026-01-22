//! Tests for dgraph-raft

use bytes::Bytes;
use dgraph_raft::{Entry, Log, Node, NodeConfig, NodeRole, RaftState};

// ============================================================================
// RaftState Tests
// ============================================================================

#[test]
fn state_initial() {
    let state = RaftState::new();

    assert_eq!(state.term(), 0);
    assert_eq!(state.voted_for(), 0);
    assert_eq!(state.role(), NodeRole::Follower);
    assert_eq!(state.leader_id(), 0);
    assert_eq!(state.commit_index(), 0);
    assert_eq!(state.last_applied(), 0);
}

#[test]
fn state_set_term() {
    let state = RaftState::new();

    state.set_term(5);
    assert_eq!(state.term(), 5);

    state.set_term(10);
    assert_eq!(state.term(), 10);
}

#[test]
fn state_start_election() {
    let state = RaftState::new();
    state.set_term(5);

    let new_term = state.start_election();

    assert_eq!(new_term, 6);
    assert_eq!(state.term(), 6);
    assert_eq!(state.role(), NodeRole::Candidate);
    assert_eq!(state.voted_for(), 0); // cleared
}

#[test]
fn state_vote() {
    let state = RaftState::new();

    state.vote_for(42);
    assert_eq!(state.voted_for(), 42);
}

#[test]
fn state_become_follower() {
    let state = RaftState::new();
    state.start_election(); // become candidate

    state.become_follower(10, 99);

    assert_eq!(state.term(), 10);
    assert_eq!(state.role(), NodeRole::Follower);
    assert_eq!(state.leader_id(), 99);
    assert_eq!(state.voted_for(), 0);
}

#[test]
fn state_become_leader() {
    let state = RaftState::new();

    state.become_leader();
    assert_eq!(state.role(), NodeRole::Leader);
}

#[test]
fn state_commit_and_applied() {
    let state = RaftState::new();

    state.set_commit_index(10);
    assert_eq!(state.commit_index(), 10);

    state.set_last_applied(5);
    assert_eq!(state.last_applied(), 5);

    state.set_last_applied(10);
    assert_eq!(state.last_applied(), 10);
}

// ============================================================================
// Log Tests
// ============================================================================

#[test]
fn log_empty() {
    let log = Log::new();

    assert!(log.is_empty());
    assert_eq!(log.len(), 0);
    assert_eq!(log.last_index(), 0);
    assert_eq!(log.last_term(), 0);
}

#[test]
fn log_append() {
    let log = Log::new();

    let idx1 = log.append(1, Bytes::from("first"));
    assert_eq!(idx1, 1);
    assert_eq!(log.len(), 1);

    let idx2 = log.append(1, Bytes::from("second"));
    assert_eq!(idx2, 2);
    assert_eq!(log.len(), 2);
}

#[test]
fn log_get() {
    let log = Log::new();

    log.append(1, Bytes::from("entry1"));
    log.append(2, Bytes::from("entry2"));
    log.append(2, Bytes::from("entry3"));

    let e1 = log.get(1).unwrap();
    assert_eq!(e1.index, 1);
    assert_eq!(e1.term, 1);
    assert_eq!(e1.data, Bytes::from("entry1"));

    let e2 = log.get(2).unwrap();
    assert_eq!(e2.index, 2);
    assert_eq!(e2.term, 2);

    let e3 = log.get(3).unwrap();
    assert_eq!(e3.index, 3);
    assert_eq!(e3.term, 2);

    // Non-existent
    assert!(log.get(0).is_none());
    assert!(log.get(4).is_none());
}

#[test]
fn log_last_index_term() {
    let log = Log::new();

    log.append(1, Bytes::from("a"));
    assert_eq!(log.last_index(), 1);
    assert_eq!(log.last_term(), 1);

    log.append(2, Bytes::from("b"));
    assert_eq!(log.last_index(), 2);
    assert_eq!(log.last_term(), 2);

    log.append(2, Bytes::from("c"));
    assert_eq!(log.last_index(), 3);
    assert_eq!(log.last_term(), 2);
}

#[test]
fn log_entries_from() {
    let log = Log::new();

    log.append(1, Bytes::from("a"));
    log.append(1, Bytes::from("b"));
    log.append(2, Bytes::from("c"));
    log.append(2, Bytes::from("d"));

    let entries = log.entries_from(2);
    assert_eq!(entries.len(), 3); // entries 2, 3, 4

    let entries = log.entries_from(4);
    assert_eq!(entries.len(), 1); // entry 4 only

    let entries = log.entries_from(5);
    assert_eq!(entries.len(), 0); // none
}

#[test]
fn log_truncate() {
    let log = Log::new();

    log.append(1, Bytes::from("a"));
    log.append(1, Bytes::from("b"));
    log.append(1, Bytes::from("c"));
    log.append(1, Bytes::from("d"));

    // Truncate from index 3 (exclusive): keep 1, 2
    log.truncate_from(3);

    assert_eq!(log.len(), 2);
    assert_eq!(log.last_index(), 2);
    assert!(log.get(1).is_some());
    assert!(log.get(2).is_some());
    assert!(log.get(3).is_none());
}

// ============================================================================
// Node Tests
// ============================================================================

#[test]
fn node_creation() {
    let config = NodeConfig {
        id: 1,
        peers: vec![2, 3],
        election_timeout_ms: 150,
        heartbeat_interval_ms: 50,
        ..Default::default()
    };

    let node = Node::new(config);

    assert_eq!(node.id(), 1);
    assert_eq!(node.role(), NodeRole::Follower);
    assert_eq!(node.term(), 0);
    assert!(!node.is_leader());
}

#[test]
fn node_become_leader() {
    let node = Node::new(NodeConfig::default());

    // Manually become leader
    node.state().become_leader();

    assert!(node.is_leader());
    assert_eq!(node.role(), NodeRole::Leader);
}

#[test]
fn node_propose_as_leader() {
    let node = Node::new(NodeConfig::default());
    node.state().become_leader();

    let idx = node.propose(Bytes::from("command1"));
    assert_eq!(idx, Some(1));

    let idx = node.propose(Bytes::from("command2"));
    assert_eq!(idx, Some(2));

    // Check log
    assert_eq!(node.log().len(), 2);
}

#[test]
fn node_propose_as_follower_fails() {
    let node = Node::new(NodeConfig::default());
    // Node starts as follower

    let idx = node.propose(Bytes::from("command"));
    assert_eq!(idx, None); // Followers can't propose
}

#[test]
fn node_tick_election() {
    let node = Node::new(NodeConfig {
        id: 1,
        peers: vec![2, 3],
        ..Default::default()
    });

    assert_eq!(node.role(), NodeRole::Follower);
    assert_eq!(node.term(), 0);

    // Trigger election timeout
    node.tick_election();

    assert_eq!(node.role(), NodeRole::Candidate);
    assert_eq!(node.term(), 1);
    assert_eq!(node.state().voted_for(), 1); // Voted for self
}

#[test]
fn node_receive_vote_becomes_leader() {
    let node = Node::new(NodeConfig {
        id: 1,
        peers: vec![2, 3],
        ..Default::default()
    });

    // Start election
    node.tick_election();
    assert_eq!(node.role(), NodeRole::Candidate);

    // Receive vote from peer (simplified: becomes leader on any grant)
    node.receive_vote(2, 1, true);

    assert_eq!(node.role(), NodeRole::Leader);
}

#[test]
fn node_step_down_on_higher_term() {
    let node = Node::new(NodeConfig {
        id: 1,
        peers: vec![2, 3],
        ..Default::default()
    });

    node.state().become_leader();
    assert!(node.is_leader());

    // Receive vote response with higher term
    node.receive_vote(2, 5, false);

    assert_eq!(node.role(), NodeRole::Follower);
    assert_eq!(node.term(), 5);
}

// ============================================================================
// Entry Tests
// ============================================================================

#[test]
fn entry_creation() {
    let entry = Entry::new(1, 5, Bytes::from("data"));

    assert_eq!(entry.index, 1);
    assert_eq!(entry.term, 5);
    assert_eq!(entry.data, Bytes::from("data"));
}
