//! Posting: Core data structure for graph edges
//!
//! A posting represents a single edge or value.
//! A posting list is a sorted collection of postings for one (predicate, subject) pair.
//!
//! This implementation matches the Dgraph pb.proto Posting message.

use bytes::Bytes;
use dgraph_common::{Timestamp, Uid};

/// Value type (matches pb.proto Posting.ValType)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ValType {
    /// Default/unset value type
    #[default]
    Default = 0,
    /// Binary data
    Binary = 1,
    /// 64-bit integer
    Int = 2,
    /// 64-bit float
    Float = 3,
    /// Boolean
    Bool = 4,
    /// DateTime
    DateTime = 5,
    /// Geospatial data
    Geo = 6,
    /// UID reference
    Uid = 7,
    /// Password (hashed)
    Password = 8,
    /// String
    String = 9,
    /// JSON object
    Object = 10,
    /// Big float (arbitrary precision)
    BigFloat = 11,
    /// Float vector (for similarity search)
    VFloat = 12,
}

impl ValType {
    /// Parse from u8
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Default),
            1 => Some(Self::Binary),
            2 => Some(Self::Int),
            3 => Some(Self::Float),
            4 => Some(Self::Bool),
            5 => Some(Self::DateTime),
            6 => Some(Self::Geo),
            7 => Some(Self::Uid),
            8 => Some(Self::Password),
            9 => Some(Self::String),
            10 => Some(Self::Object),
            11 => Some(Self::BigFloat),
            12 => Some(Self::VFloat),
            _ => None,
        }
    }

    /// Check if this is a numeric type
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::Float | Self::BigFloat)
    }

    /// Check if this is a string-like type
    #[must_use]
    pub const fn is_string_like(&self) -> bool {
        matches!(self, Self::String | Self::Password | Self::Default)
    }
}

/// Posting type (matches pb.proto Posting.PostingType)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PostingType {
    /// UID reference (edge to another node)
    #[default]
    Ref = 0,
    /// Simple value
    Value = 1,
    /// Value with language tag
    ValueLang = 2,
}

impl PostingType {
    /// Parse from u8
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Ref),
            1 => Some(Self::Value),
            2 => Some(Self::ValueLang),
            _ => None,
        }
    }
}

/// Operation type for mutations
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Op {
    /// Set operation (add/update)
    #[default]
    Set = 0,
    /// Delete operation
    Del = 1,
    /// Override operation
    Ovr = 2,
}

/// Facet: key-value pair attached to an edge
#[derive(Clone, Debug, PartialEq)]
pub struct Facet {
    /// Facet key
    pub key: String,
    /// Facet value (serialized)
    pub value: Bytes,
    /// Value type
    pub val_type: ValType,
    /// Tokens (for indexed facets)
    pub tokens: Vec<String>,
    /// Alias for query output
    pub alias: String,
}

impl Facet {
    /// Create a new string facet
    #[must_use]
    pub fn string(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: Bytes::from(value.into()),
            val_type: ValType::String,
            tokens: Vec::new(),
            alias: String::new(),
        }
    }

    /// Create a new int facet
    #[must_use]
    pub fn int(key: impl Into<String>, value: i64) -> Self {
        Self {
            key: key.into(),
            value: Bytes::from(value.to_le_bytes().to_vec()),
            val_type: ValType::Int,
            tokens: Vec::new(),
            alias: String::new(),
        }
    }

    /// Create a new float facet
    #[must_use]
    pub fn float(key: impl Into<String>, value: f64) -> Self {
        Self {
            key: key.into(),
            value: Bytes::from(value.to_le_bytes().to_vec()),
            val_type: ValType::Float,
            tokens: Vec::new(),
            alias: String::new(),
        }
    }

    /// Create a new bool facet
    #[must_use]
    pub fn bool(key: impl Into<String>, value: bool) -> Self {
        Self {
            key: key.into(),
            value: Bytes::from(vec![value as u8]),
            val_type: ValType::Bool,
            tokens: Vec::new(),
            alias: String::new(),
        }
    }

    /// Get value as string (if string type)
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        if self.val_type == ValType::String {
            std::str::from_utf8(&self.value).ok()
        } else {
            None
        }
    }

    /// Get value as i64 (if int type)
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        if self.val_type == ValType::Int && self.value.len() == 8 {
            Some(i64::from_le_bytes(self.value[..8].try_into().ok()?))
        } else {
            None
        }
    }

    /// Get value as f64 (if float type)
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        if self.val_type == ValType::Float && self.value.len() == 8 {
            Some(f64::from_le_bytes(self.value[..8].try_into().ok()?))
        } else {
            None
        }
    }

    /// Get value as bool (if bool type)
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        if self.val_type == ValType::Bool && !self.value.is_empty() {
            Some(self.value[0] != 0)
        } else {
            None
        }
    }
}

/// Single posting (edge or value)
///
/// Matches pb.proto Posting message structure
#[derive(Clone, Debug)]
pub struct Posting {
    /// Target UID (for edges) or 0 (for values)
    pub uid: u64,
    /// Value bytes
    pub value: Bytes,
    /// Type of the value
    pub val_type: ValType,
    /// Type of posting (ref, value, value with language)
    pub posting_type: PostingType,
    /// Language tag (only for ValueLang postings)
    pub lang_tag: Bytes,
    /// Facets attached to this posting
    pub facets: Vec<Facet>,
    /// Operation type (used during mutations)
    pub op: Op,
    /// Start timestamp (for MVCC - transaction start)
    pub start_ts: Timestamp,
    /// Commit timestamp (for MVCC - when committed)
    pub commit_ts: Timestamp,
}

impl Posting {
    /// Create a reference posting (edge to another node)
    #[must_use]
    pub fn reference(uid: Uid, commit_ts: Timestamp) -> Self {
        Self {
            uid: uid.get(),
            value: Bytes::new(),
            val_type: ValType::Uid,
            posting_type: PostingType::Ref,
            lang_tag: Bytes::new(),
            facets: Vec::new(),
            op: Op::Set,
            start_ts: Timestamp::ZERO,
            commit_ts,
        }
    }

    /// Create a value posting with typed value
    #[must_use]
    pub fn typed_value(data: Bytes, val_type: ValType, commit_ts: Timestamp) -> Self {
        Self {
            uid: 0,
            value: data,
            val_type,
            posting_type: PostingType::Value,
            lang_tag: Bytes::new(),
            facets: Vec::new(),
            op: Op::Set,
            start_ts: Timestamp::ZERO,
            commit_ts,
        }
    }

    /// Create a string value posting
    #[must_use]
    pub fn string(s: impl Into<String>, commit_ts: Timestamp) -> Self {
        Self::typed_value(Bytes::from(s.into()), ValType::String, commit_ts)
    }

    /// Create an int value posting
    #[must_use]
    pub fn int(v: i64, commit_ts: Timestamp) -> Self {
        Self::typed_value(Bytes::from(v.to_le_bytes().to_vec()), ValType::Int, commit_ts)
    }

    /// Create a float value posting
    #[must_use]
    pub fn float(v: f64, commit_ts: Timestamp) -> Self {
        Self::typed_value(Bytes::from(v.to_le_bytes().to_vec()), ValType::Float, commit_ts)
    }

    /// Create a bool value posting
    #[must_use]
    pub fn bool(v: bool, commit_ts: Timestamp) -> Self {
        Self::typed_value(Bytes::from(vec![v as u8]), ValType::Bool, commit_ts)
    }

    /// Create a value posting with language tag
    #[must_use]
    pub fn value_lang(data: Bytes, lang: impl Into<String>, commit_ts: Timestamp) -> Self {
        Self {
            uid: 0,
            value: data,
            val_type: ValType::String,
            posting_type: PostingType::ValueLang,
            lang_tag: Bytes::from(lang.into()),
            facets: Vec::new(),
            op: Op::Set,
            start_ts: Timestamp::ZERO,
            commit_ts,
        }
    }

    /// Create a value posting (backward compatibility)
    #[must_use]
    pub fn value(data: Bytes, commit_ts: Timestamp) -> Self {
        Self::typed_value(data, ValType::Default, commit_ts)
    }

    /// Add a facet to this posting
    #[must_use]
    pub fn with_facet(mut self, key: String, value: Bytes) -> Self {
        self.facets.push(Facet {
            key,
            value,
            val_type: ValType::Default,
            tokens: Vec::new(),
            alias: String::new(),
        });
        self
    }

    /// Add a typed facet
    #[must_use]
    pub fn with_typed_facet(mut self, facet: Facet) -> Self {
        self.facets.push(facet);
        self
    }

    /// Set the operation type
    #[must_use]
    pub const fn with_op(mut self, op: Op) -> Self {
        self.op = op;
        self
    }

    /// Set start timestamp
    #[must_use]
    pub const fn with_start_ts(mut self, ts: Timestamp) -> Self {
        self.start_ts = ts;
        self
    }

    /// Check if this posting is visible at the given read timestamp
    #[inline]
    #[must_use]
    pub fn visible_at(&self, read_ts: Timestamp) -> bool {
        self.commit_ts <= read_ts
    }

    /// Check if this is a reference (edge) posting
    #[inline]
    #[must_use]
    pub const fn is_ref(&self) -> bool {
        matches!(self.posting_type, PostingType::Ref)
    }

    /// Check if this is a value posting
    #[inline]
    #[must_use]
    pub const fn is_value(&self) -> bool {
        matches!(self.posting_type, PostingType::Value | PostingType::ValueLang)
    }

    /// Get language tag as string (if present)
    #[must_use]
    pub fn lang(&self) -> Option<&str> {
        if self.lang_tag.is_empty() {
            None
        } else {
            std::str::from_utf8(&self.lang_tag).ok()
        }
    }

    /// Get value as string (if string type)
    #[must_use]
    pub fn value_as_str(&self) -> Option<&str> {
        if self.val_type.is_string_like() {
            std::str::from_utf8(&self.value).ok()
        } else {
            None
        }
    }

    /// Get value as i64 (if int type)
    #[must_use]
    pub fn value_as_int(&self) -> Option<i64> {
        if self.val_type == ValType::Int && self.value.len() >= 8 {
            Some(i64::from_le_bytes(self.value[..8].try_into().ok()?))
        } else {
            None
        }
    }

    /// Get value as f64 (if float type)
    #[must_use]
    pub fn value_as_float(&self) -> Option<f64> {
        if self.val_type == ValType::Float && self.value.len() >= 8 {
            Some(f64::from_le_bytes(self.value[..8].try_into().ok()?))
        } else {
            None
        }
    }

    /// Get value as bool (if bool type)
    #[must_use]
    pub fn value_as_bool(&self) -> Option<bool> {
        if self.val_type == ValType::Bool && !self.value.is_empty() {
            Some(self.value[0] != 0)
        } else {
            None
        }
    }

    /// Get facet by key
    #[must_use]
    pub fn facet(&self, key: &str) -> Option<&Facet> {
        self.facets.iter().find(|f| f.key == key)
    }
}

// Backward compatibility: keep the old PostingKind name
pub use PostingType as PostingKind;

/// Posting list: sorted collection of postings
#[derive(Clone, Debug, Default)]
pub struct PostingList {
    /// Sorted by (uid, commit_ts desc)
    postings: Vec<Posting>,
    /// Minimum commit timestamp in list
    min_ts: Timestamp,
    /// Maximum commit timestamp in list
    max_ts: Timestamp,
    /// Commit timestamp for the list itself (for split lists)
    pub commit_ts: Timestamp,
    /// Split UIDs (for large lists split across multiple keys)
    pub splits: Vec<u64>,
}

impl PostingList {
    /// Create empty posting list
    #[must_use]
    pub const fn new() -> Self {
        Self {
            postings: Vec::new(),
            min_ts: Timestamp::MAX,
            max_ts: Timestamp::ZERO,
            commit_ts: Timestamp::ZERO,
            splits: Vec::new(),
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

    /// Iterate all postings (including all versions)
    pub fn iter(&self) -> impl Iterator<Item = &Posting> {
        self.postings.iter()
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

    /// Iterate values visible at timestamp with language preference
    pub fn values_at_lang<'a>(
        &'a self,
        read_ts: Timestamp,
        langs: &'a [String],
    ) -> impl Iterator<Item = &'a Posting> {
        self.iter_at(read_ts).filter(move |p| {
            if !p.is_value() {
                return false;
            }
            if langs.is_empty() {
                return p.lang_tag.is_empty();
            }
            // Check if posting's language matches any preferred language
            if let Some(lang) = p.lang() {
                langs.iter().any(|l| l == lang || l == "*")
            } else {
                langs.iter().any(|l| l.is_empty() || l == ".")
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
            .filter(|p| p.is_ref())
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

    /// Remove postings older than the given watermark
    /// Returns number of removed postings
    pub fn gc(&mut self, watermark: Timestamp) -> usize {
        let before = self.postings.len();

        // Keep track of which UIDs we've seen (to keep latest version)
        let mut seen_uids = ahash::AHashSet::new();

        self.postings.retain(|p| {
            // Always keep if above watermark
            if p.commit_ts > watermark {
                return true;
            }
            // Below watermark: keep only latest version per UID
            if p.uid == 0 {
                // Value postings: keep all below watermark for now
                true
            } else {
                // Ref postings: keep only first (newest) version
                seen_uids.insert(p.uid)
            }
        });

        before - self.postings.len()
    }

    /// Merge another posting list into this one
    pub fn merge(&mut self, other: PostingList) {
        for posting in other.postings {
            self.add(posting);
        }
    }
}
