//! Raft log: append-only replicated log

use bytes::Bytes;
use parking_lot::RwLock;

/// Log entry
#[derive(Clone, Debug)]
pub struct Entry {
    /// Entry index (1-based)
    pub index: u64,
    /// Term when entry was created
    pub term: u64,
    /// Entry data
    pub data: Bytes,
}

impl Entry {
    /// Create new entry
    #[must_use]
    pub fn new(index: u64, term: u64, data: Bytes) -> Self {
        Self { index, term, data }
    }
}

/// Raft log
pub struct Log {
    entries: RwLock<Vec<Entry>>,
    /// First index in log (for compaction)
    first_index: u64,
}

impl Log {
    /// Create new empty log
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            first_index: 1,
        }
    }

    /// Append entry to log
    pub fn append(&self, term: u64, data: Bytes) -> u64 {
        let mut entries = self.entries.write();
        let index = self.first_index + entries.len() as u64;
        entries.push(Entry::new(index, term, data));
        index
    }

    /// Get entry at index
    #[must_use]
    pub fn get(&self, index: u64) -> Option<Entry> {
        let entries = self.entries.read();
        if index < self.first_index {
            return None;
        }
        let offset = (index - self.first_index) as usize;
        entries.get(offset).cloned()
    }

    /// Get last index
    #[must_use]
    pub fn last_index(&self) -> u64 {
        let entries = self.entries.read();
        if entries.is_empty() {
            0
        } else {
            self.first_index + entries.len() as u64 - 1
        }
    }

    /// Get last term
    #[must_use]
    pub fn last_term(&self) -> u64 {
        let entries = self.entries.read();
        entries.last().map_or(0, |e| e.term)
    }

    /// Get entries from index
    #[must_use]
    pub fn entries_from(&self, from_index: u64) -> Vec<Entry> {
        let entries = self.entries.read();
        if from_index < self.first_index {
            return entries.clone();
        }
        let offset = (from_index - self.first_index) as usize;
        entries[offset..].to_vec()
    }

    /// Truncate log from index (exclusive)
    pub fn truncate_from(&self, from_index: u64) {
        let mut entries = self.entries.write();
        if from_index < self.first_index {
            entries.clear();
            return;
        }
        let offset = (from_index - self.first_index) as usize;
        entries.truncate(offset);
    }

    /// Number of entries
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Check if log is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }
}

impl Default for Log {
    fn default() -> Self {
        Self::new()
    }
}
