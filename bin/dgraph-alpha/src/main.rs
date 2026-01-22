//! dgraph-alpha: Data node binary
//!
//! Handles queries and mutations.

use dgraph_common::Timestamp;
use dgraph_query::{Executor, Parser};
use dgraph_storage::{Store, StoreConfig};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("dgraph-alpha starting");

    // Create store
    let store = Arc::new(Store::new(StoreConfig::default()));
    info!("storage initialized");

    // Create executor
    let executor = Executor::new(Arc::clone(&store));
    info!("query executor ready");

    // Example query
    let query = r#"
    {
        me(func: uid(0x1)) {
            name
            age
        }
    }
    "#;

    let mut parser = Parser::new(query);
    match parser.parse() {
        Ok(op) => {
            info!(?op, "parsed query");

            if let dgraph_query::Operation::Query(q) = op {
                let read_ts = store.current_ts();
                match executor.execute(&q, read_ts) {
                    Ok(result) => {
                        info!(%result, "query result");
                    }
                    Err(e) => {
                        tracing::error!(%e, "query execution failed");
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!(%e, "parse error");
        }
    }

    info!("dgraph-alpha ready");

    // In real implementation: start HTTP/gRPC server here
    // For now, just demonstrate the system works
}
