//! Store: Key-value storage with posting lists

use dgraph_common::{Error, Key, Result, Timestamp, TimestampOracle};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;

use crate::{MvccLayer, Posting, PostingList, Txn};

/// Store configuration
#[derive(Clone, Debug)]
pub struct StoreConfig {
    /// Initial capacity for key map
    pub initial_capacity: usize,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            initial_capacity: 1024,
        }
    }
}

/// Main storage engine
pub struct Store {
    /// Key -> PostingList map
    data: DashMap<Vec<u8>, RwLock<PostingList>, ahash::RandomState>,
    /// MVCC layer
    mvcc: Arc<MvccLayer>,
    /// Timestamp oracle
    oracle: TimestampOracle,
}

impl Store {
    /// Create new store with config
    #[must_use]
    pub fn new(config: StoreConfig) -> Self {
        Self {
            data: DashMap::with_capacity_and_hasher(
                config.initial_capacity,
                ahash::RandomState::new(),
            ),
            mvcc: Arc::new(MvccLayer::new()),
            oracle: TimestampOracle::default(),
        }
    }

    /// Start a new read-write transaction
    #[must_use]
    pub fn begin(&self) -> Txn {
        let start_ts = self.oracle.next();
        Txn::new(start_ts, Arc::clone(&self.mvcc))
    }

    /// Get a posting list for a key at given timestamp
    #[must_use]
    pub fn get(&self, key: &Key, read_ts: Timestamp) -> Option<PostingList> {
        self.data
            .get(key.as_bytes())
            .map(|entry| entry.read().clone())
    }

    /// Apply committed writes from a transaction
    ///
    /// # Errors
    /// Returns error if commit fails
    pub fn apply(&self, txn: Txn, commit_ts: Timestamp) -> Result<()> {
        let writes = txn.pending_writes();

        // Apply writes to storage
        for (key, postings) in &writes {
            let entry = self.data.entry(key.as_bytes().to_vec()).or_insert_with(|| {
                RwLock::new(PostingList::new())
            });

            let mut list = entry.write();
            for mut posting in postings.clone() {
                posting.commit_ts = commit_ts;
                list.add(posting);
            }
        }

        // Commit transaction (records in MVCC layer)
        txn.commit(commit_ts)?;

        Ok(())
    }

    /// Get current timestamp
    #[must_use]
    pub fn current_ts(&self) -> Timestamp {
        self.oracle.current()
    }

    /// Get next timestamp
    pub fn next_ts(&self) -> Timestamp {
        self.oracle.next()
    }

    /// Number of keys in store
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if store is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Iterate all keys (for debugging)
    pub fn keys(&self) -> Vec<Vec<u8>> {
        self.data.iter().map(|e| e.key().clone()).collect()
    }

    /// Advance read watermark
    pub fn advance_watermark(&self, ts: Timestamp) {
        self.mvcc.advance_watermark(ts);
    }

    /// Run garbage collection
    pub fn gc(&self) {
        self.mvcc.gc();
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new(StoreConfig::default())
    }
}
