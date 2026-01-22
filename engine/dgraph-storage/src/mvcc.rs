//! MVCC: Multi-Version Concurrency Control
//!
//! Handles transaction isolation and conflict detection.

use dgraph_common::{Error, Key, Result, Timestamp};
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::Posting;

/// Transaction context for MVCC
#[derive(Debug)]
pub struct TxnContext {
    /// Transaction start timestamp
    pub start_ts: Timestamp,
    /// Keys read during transaction
    reads: RwLock<Vec<Key>>,
    /// Keys written during transaction
    writes: RwLock<BTreeMap<Key, Vec<Posting>>>,
    /// Whether transaction has been committed or aborted
    done: RwLock<bool>,
}

impl TxnContext {
    /// Create new transaction context
    #[must_use]
    pub fn new(start_ts: Timestamp) -> Self {
        Self {
            start_ts,
            reads: RwLock::new(Vec::new()),
            writes: RwLock::new(BTreeMap::new()),
            done: RwLock::new(false),
        }
    }

    /// Record a read
    pub fn record_read(&self, key: Key) {
        debug_assert!(!*self.done.read());
        self.reads.write().push(key);
    }

    /// Record a write
    pub fn record_write(&self, key: Key, posting: Posting) {
        debug_assert!(!*self.done.read());
        self.writes.write().entry(key).or_default().push(posting);
    }

    /// Get all writes
    #[must_use]
    pub fn get_writes(&self) -> BTreeMap<Key, Vec<Posting>> {
        self.writes.read().clone()
    }

    /// Get all read keys
    #[must_use]
    pub fn get_reads(&self) -> Vec<Key> {
        self.reads.read().clone()
    }

    /// Mark transaction as done
    pub fn mark_done(&self) {
        *self.done.write() = true;
    }

    /// Check if transaction is done
    #[must_use]
    pub fn is_done(&self) -> bool {
        *self.done.read()
    }
}

/// MVCC layer for managing concurrent transactions
pub struct MvccLayer {
    /// Committed transactions: commit_ts -> (start_ts, written keys)
    committed: RwLock<BTreeMap<Timestamp, (Timestamp, Vec<Key>)>>,
    /// Read watermark: transactions older than this can be garbage collected
    read_watermark: RwLock<Timestamp>,
}

impl MvccLayer {
    /// Create new MVCC layer
    #[must_use]
    pub fn new() -> Self {
        Self {
            committed: RwLock::new(BTreeMap::new()),
            read_watermark: RwLock::new(Timestamp::ZERO),
        }
    }

    /// Check for conflicts before committing
    ///
    /// # Errors
    /// Returns `TxnConflict` if another transaction committed conflicting writes
    pub fn check_conflicts(&self, ctx: &TxnContext) -> Result<()> {
        let reads = ctx.get_reads();
        if reads.is_empty() {
            return Ok(());
        }

        let committed = self.committed.read();

        // Check if any committed transaction (after our start) wrote to keys we read
        for (commit_ts, (_, written_keys)) in committed.range(ctx.start_ts.next()..) {
            for read_key in &reads {
                if written_keys.contains(read_key) {
                    return Err(Error::TxnConflict(commit_ts.get()));
                }
            }
        }

        Ok(())
    }

    /// Record a committed transaction
    pub fn record_commit(&self, start_ts: Timestamp, commit_ts: Timestamp, keys: Vec<Key>) {
        self.committed.write().insert(commit_ts, (start_ts, keys));
    }

    /// Update read watermark (allows garbage collection of old versions)
    pub fn advance_watermark(&self, ts: Timestamp) {
        let mut watermark = self.read_watermark.write();
        if ts > *watermark {
            *watermark = ts;
        }
    }

    /// Get current read watermark
    #[must_use]
    pub fn read_watermark(&self) -> Timestamp {
        *self.read_watermark.read()
    }

    /// Garbage collect old commit records
    pub fn gc(&self) {
        let watermark = self.read_watermark();
        let mut committed = self.committed.write();

        // Remove all commits before watermark
        let to_remove: Vec<_> = committed
            .range(..watermark)
            .map(|(ts, _)| *ts)
            .collect();

        for ts in to_remove {
            committed.remove(&ts);
        }
    }
}

impl Default for MvccLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to a transaction
pub struct Txn {
    ctx: Arc<TxnContext>,
    mvcc: Arc<MvccLayer>,
}

impl Txn {
    /// Create new transaction
    #[must_use]
    pub fn new(start_ts: Timestamp, mvcc: Arc<MvccLayer>) -> Self {
        Self {
            ctx: Arc::new(TxnContext::new(start_ts)),
            mvcc,
        }
    }

    /// Get start timestamp
    #[must_use]
    pub fn start_ts(&self) -> Timestamp {
        self.ctx.start_ts
    }

    /// Record a read
    pub fn read(&self, key: Key) {
        self.ctx.record_read(key);
    }

    /// Record a write
    pub fn write(&self, key: Key, posting: Posting) {
        self.ctx.record_write(key, posting);
    }

    /// Commit transaction
    ///
    /// # Errors
    /// Returns error if there's a conflict
    pub fn commit(self, commit_ts: Timestamp) -> Result<()> {
        // Check conflicts
        self.mvcc.check_conflicts(&self.ctx)?;

        // Record commit
        let writes = self.ctx.get_writes();
        let keys: Vec<_> = writes.keys().cloned().collect();

        if !keys.is_empty() {
            self.mvcc.record_commit(self.ctx.start_ts, commit_ts, keys);
        }

        self.ctx.mark_done();
        Ok(())
    }

    /// Abort transaction
    pub fn abort(self) {
        self.ctx.mark_done();
    }

    /// Get pending writes for this transaction
    #[must_use]
    pub fn pending_writes(&self) -> BTreeMap<Key, Vec<Posting>> {
        self.ctx.get_writes()
    }
}
