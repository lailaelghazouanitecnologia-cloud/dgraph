//! dgraph-zero: Coordination node binary
//!
//! Manages cluster membership, UID assignment, and Raft consensus.

use dgraph_raft::{Node, NodeConfig, NodeRole};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("dgraph-zero starting");

    // Create Raft node
    let config = NodeConfig {
        id: 1,
        peers: vec![2, 3],
        election_timeout_ms: 150,
        heartbeat_interval_ms: 50,
    };

    let node = Node::new(config);
    info!(id = node.id(), "raft node created");

    // Simulate becoming leader (in real impl, this happens via election)
    node.state().become_leader();
    assert_eq!(node.role(), NodeRole::Leader);
    info!("became leader");

    // Example: propose an entry
    let data = b"membership update".to_vec();
    if let Some(index) = node.propose(data.into()) {
        info!(index, "proposed entry");
    }

    info!(
        term = node.term(),
        commit_index = node.commit_index(),
        "dgraph-zero ready"
    );

    // In real implementation: start gRPC server here
}
