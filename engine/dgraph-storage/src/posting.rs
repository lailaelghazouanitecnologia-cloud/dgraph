//! Posting: Core data structure for graph edges
//!
//! A posting represents a single edge or value.
//! A posting list is a sorted collection of postings for one (predicate, subject) pair.

use bytes::Bytes;
use dgraph_common::{Timestamp, Uid};

/// Type of posting
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PostingKind {
    /// Reference to another node (edge)
    Ref = 0,
    /// Scalar value
    Value = 1,
}

/// Single posting (edge or value)
#[derive(Clone, Debug)]
pub struct Posting {
    /// Target UID (for edges) or 0 (for values)
    pub uid: u64,
    /// Value bytes (empty for pure edges)
    pub value: Bytes,
    /// Posting kind
    pub kind: PostingKind,
    /// Commit timestamp
    pub commit_ts: Timestamp,
    /// Start timestamp (for MVCC)
    pub start_ts: Timestamp,
    /// Facets (key-value pairs attached to edge)
    pub facets: Vec<(String, Bytes)>,
}

impl Posting {
    /// Create a reference posting (edge to another node)
    #[must_use]
    pub fn reference(uid: Uid, commit_ts: Timestamp) -> Self {
        Self {
            uid: uid.get(),
            value: Bytes::new(),
            kind: PostingKind::Ref,
            commit_ts,
            start_ts: Timestamp::ZERO,
            facets: Vec::new(),
        }
    }

    /// Create a value posting
    #[must_use]
    pub fn value(data: Bytes, commit_ts: Timestamp) -> Self {
        Self {
            uid: 0,
            value: data,
            kind: PostingKind::Value,
            commit_ts,
            start_ts: Timestamp::ZERO,
            facets: Vec::new(),
        }
    }

    /// Add a facet to this posting
    pub fn with_facet(mut self, key: String, value: Bytes) -> Self {
        self.facets.push((key, value));
        self
    }

    /// Check if this posting is visible at the given read timestamp
    #[inline]
    #[must_use]
    pub fn visible_at(&self, read_ts: Timestamp) -> bool {
        self.commit_ts <= read_ts
    }
}

/// Posting list: sorted collection of postings
#[derive(Clone, Debug, Default)]
pub struct PostingList {
    /// Sorted by (uid, commit_ts desc)
    postings: Vec<Posting>,
    /// Minimum commit timestamp in list
    min_ts: Timestamp,
    /// Maximum commit timestamp in list
    max_ts: Timestamp,
}

impl PostingList {
    /// Create empty posting list
    #[must_use]
    pub const fn new() -> Self {
        Self {
            postings: Vec::new(),
            min_ts: Timestamp::MAX,
            max_ts: Timestamp::ZERO,
        }
    }

    /// Number of postings (including all versions)
    #[must_use]
    pub fn len(&self) -> usize {
        self.postings.len()
    }

    /// Check if empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.postings.is_empty()
    }

    /// Add a posting, maintaining sort order
    pub fn add(&mut self, posting: Posting) {
        // Update timestamps
        if posting.commit_ts < self.min_ts {
            self.min_ts = posting.commit_ts;
        }
        if posting.commit_ts > self.max_ts {
            self.max_ts = posting.commit_ts;
        }

        // Binary search for insert position
        let pos = self.postings.binary_search_by(|p| {
            match p.uid.cmp(&posting.uid) {
                std::cmp::Ordering::Equal => {
                    // Same uid: sort by commit_ts descending (newer first)
                    posting.commit_ts.cmp(&p.commit_ts)
                }
                other => other,
            }
        });

        let idx = match pos {
            Ok(i) | Err(i) => i,
        };
        self.postings.insert(idx, posting);
    }

    /// Iterate postings visible at given timestamp
    pub fn iter_at(&self, read_ts: Timestamp) -> impl Iterator<Item = &Posting> {
        let mut seen_uids = ahash::AHashSet::new();
        self.postings.iter().filter(move |p| {
            if !p.visible_at(read_ts) {
                return false;
            }
            // Only return first (newest visible) version per uid
            if p.uid == 0 {
                // Values: always include if visible
                true
            } else {
                // Refs: deduplicate by uid
                seen_uids.insert(p.uid)
            }
        })
    }

    /// Count distinct UIDs visible at timestamp
    #[must_use]
    pub fn count_at(&self, read_ts: Timestamp) -> usize {
        self.iter_at(read_ts).count()
    }

    /// Get posting for specific UID at timestamp
    #[must_use]
    pub fn get(&self, uid: Uid, read_ts: Timestamp) -> Option<&Posting> {
        self.postings.iter().find(|p| {
            p.uid == uid.get() && p.visible_at(read_ts)
        })
    }

    /// Get all UIDs as iterator
    pub fn uids_at(&self, read_ts: Timestamp) -> impl Iterator<Item = Uid> + '_ {
        self.iter_at(read_ts)
            .filter(|p| p.kind == PostingKind::Ref)
            .filter_map(|p| Uid::new(p.uid).ok())
    }

    /// Minimum timestamp in this list
    #[must_use]
    pub const fn min_ts(&self) -> Timestamp {
        self.min_ts
    }

    /// Maximum timestamp in this list
    #[must_use]
    pub const fn max_ts(&self) -> Timestamp {
        self.max_ts
    }
}
