//! Index: Index key management and mutation logic
//!
//! Handles creation and lookup of index entries.

use crate::tokenizer::{Token, Tokenizer, TokenizerId};
use bytes::{BufMut, Bytes, BytesMut};
use dgraph_common::{Key, KeyKind, Uid};

/// Index entry - represents a single index posting
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexEntry {
    /// The predicate being indexed
    pub predicate: String,
    /// The encoded token
    pub token: Token,
    /// The UID that has this value
    pub uid: Uid,
}

impl IndexEntry {
    /// Create a new index entry
    #[must_use]
    pub fn new(predicate: impl Into<String>, token: Token, uid: Uid) -> Self {
        Self {
            predicate: predicate.into(),
            token,
            uid,
        }
    }

    /// Get the index key for storage
    ///
    /// # Errors
    /// Returns error if key would exceed max size
    pub fn key(&self) -> dgraph_common::Result<Key> {
        Key::index(&self.predicate, &self.token.encode())
    }
}

/// Index operation type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexOp {
    /// Add index entry
    Add,
    /// Remove index entry
    Remove,
}

/// Index mutation - a change to an index
#[derive(Clone, Debug)]
pub struct IndexMutation {
    /// The operation
    pub op: IndexOp,
    /// The index entry
    pub entry: IndexEntry,
}

impl IndexMutation {
    /// Create an add mutation
    #[must_use]
    pub fn add(predicate: impl Into<String>, token: Token, uid: Uid) -> Self {
        Self {
            op: IndexOp::Add,
            entry: IndexEntry::new(predicate, token, uid),
        }
    }

    /// Create a remove mutation
    #[must_use]
    pub fn remove(predicate: impl Into<String>, token: Token, uid: Uid) -> Self {
        Self {
            op: IndexOp::Remove,
            entry: IndexEntry::new(predicate, token, uid),
        }
    }
}

/// Generate index mutations for a value change
pub fn generate_index_mutations<T: Tokenizer>(
    tokenizer: &T,
    predicate: &str,
    uid: Uid,
    old_value: Option<&str>,
    new_value: Option<&str>,
) -> Vec<IndexMutation> {
    let mut mutations = Vec::new();

    // Remove old index entries
    if let Some(old) = old_value {
        for token in tokenizer.tokenize_string(old) {
            mutations.push(IndexMutation::remove(predicate, token, uid));
        }
    }

    // Add new index entries
    if let Some(new) = new_value {
        for token in tokenizer.tokenize_string(new) {
            mutations.push(IndexMutation::add(predicate, token, uid));
        }
    }

    mutations
}

/// Generate index mutations for integer value change
pub fn generate_int_index_mutations<T: Tokenizer>(
    tokenizer: &T,
    predicate: &str,
    uid: Uid,
    old_value: Option<i64>,
    new_value: Option<i64>,
) -> Vec<IndexMutation> {
    let mut mutations = Vec::new();

    if let Some(old) = old_value {
        for token in tokenizer.tokenize_int(old) {
            mutations.push(IndexMutation::remove(predicate, token, uid));
        }
    }

    if let Some(new) = new_value {
        for token in tokenizer.tokenize_int(new) {
            mutations.push(IndexMutation::add(predicate, token, uid));
        }
    }

    mutations
}

/// Generate index mutations for float value change
pub fn generate_float_index_mutations<T: Tokenizer>(
    tokenizer: &T,
    predicate: &str,
    uid: Uid,
    old_value: Option<f64>,
    new_value: Option<f64>,
) -> Vec<IndexMutation> {
    let mut mutations = Vec::new();

    if let Some(old) = old_value {
        for token in tokenizer.tokenize_float(old) {
            mutations.push(IndexMutation::remove(predicate, token, uid));
        }
    }

    if let Some(new) = new_value {
        for token in tokenizer.tokenize_float(new) {
            mutations.push(IndexMutation::add(predicate, token, uid));
        }
    }

    mutations
}

/// Count key - for predicates with count index
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountKey {
    /// Predicate name
    pub predicate: String,
    /// The count value
    pub count: u32,
    /// Whether this is a reverse count
    pub reverse: bool,
}

impl CountKey {
    /// Create a new count key
    #[must_use]
    pub fn new(predicate: impl Into<String>, count: u32, reverse: bool) -> Self {
        Self {
            predicate: predicate.into(),
            count,
            reverse,
        }
    }

    /// Encode to bytes
    #[must_use]
    pub fn encode(&self) -> Bytes {
        let pred_bytes = self.predicate.as_bytes();
        let mut buf = BytesMut::with_capacity(1 + 2 + pred_bytes.len() + 1 + 4);

        // Kind byte for count
        buf.put_u8(KeyKind::Count as u8);

        // Predicate length-prefixed
        #[allow(clippy::cast_possible_truncation)]
        buf.put_u16(pred_bytes.len() as u16);
        buf.put_slice(pred_bytes);

        // Reverse flag
        buf.put_u8(if self.reverse { 1 } else { 0 });

        // Count
        buf.put_u32(self.count);

        buf.freeze()
    }
}

/// Reverse key - for uid-to-uid edges with reverse index
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReverseKey {
    /// Predicate name
    pub predicate: String,
    /// The object UID
    pub object_uid: Uid,
}

impl ReverseKey {
    /// Create a new reverse key
    #[must_use]
    pub fn new(predicate: impl Into<String>, object_uid: Uid) -> Self {
        Self {
            predicate: predicate.into(),
            object_uid,
        }
    }

    /// Get the storage key
    ///
    /// # Errors
    /// Returns error if key would exceed max size
    pub fn key(&self) -> dgraph_common::Result<Key> {
        Key::reverse(&self.predicate, self.object_uid)
    }
}

/// Index specification for a predicate
#[derive(Clone, Debug, Default)]
pub struct IndexSpec {
    /// Tokenizer names to use for this predicate
    pub tokenizers: Vec<String>,
    /// Whether to maintain a count index
    pub count: bool,
    /// Whether to maintain a reverse index (for UID predicates)
    pub reverse: bool,
    /// Whether this predicate is upsert-able
    pub upsert: bool,
}

impl IndexSpec {
    /// Create a new index spec
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tokenizer
    #[must_use]
    pub fn with_tokenizer(mut self, name: impl Into<String>) -> Self {
        self.tokenizers.push(name.into());
        self
    }

    /// Enable count index
    #[must_use]
    pub fn with_count(mut self) -> Self {
        self.count = true;
        self
    }

    /// Enable reverse index
    #[must_use]
    pub fn with_reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Enable upsert
    #[must_use]
    pub fn with_upsert(mut self) -> Self {
        self.upsert = true;
        self
    }

    /// Check if any indexing is enabled
    #[must_use]
    pub fn is_indexed(&self) -> bool {
        !self.tokenizers.is_empty() || self.count || self.reverse
    }
}

/// Index lookup helper - find UIDs matching a token
#[derive(Clone, Debug)]
pub struct IndexLookup {
    /// Predicate to search
    pub predicate: String,
    /// Token to match
    pub token: Token,
}

impl IndexLookup {
    /// Create a new lookup
    #[must_use]
    pub fn new(predicate: impl Into<String>, token: Token) -> Self {
        Self {
            predicate: predicate.into(),
            token,
        }
    }

    /// Get the key to lookup
    ///
    /// # Errors
    /// Returns error if key would exceed max size
    pub fn key(&self) -> dgraph_common::Result<Key> {
        Key::index(&self.predicate, &self.token.encode())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::{ExactTokenizer, TermTokenizer};

    #[test]
    fn index_entry_key() {
        let token = Token::from_str(TokenizerId::Exact, "test");
        let uid = Uid::new(123).unwrap();
        let entry = IndexEntry::new("name", token, uid);
        let key = entry.key().unwrap();
        assert_eq!(key.kind(), KeyKind::Index);
    }

    #[test]
    fn generate_string_mutations() {
        let tok = TermTokenizer;
        let uid = Uid::new(1).unwrap();
        let mutations = generate_index_mutations(&tok, "name", uid, None, Some("hello world"));
        assert_eq!(mutations.len(), 2);
        assert!(mutations.iter().all(|m| m.op == IndexOp::Add));
    }

    #[test]
    fn generate_update_mutations() {
        let tok = ExactTokenizer;
        let uid = Uid::new(1).unwrap();
        let mutations =
            generate_index_mutations(&tok, "name", uid, Some("old"), Some("new"));
        assert_eq!(mutations.len(), 2);
        assert!(mutations.iter().any(|m| m.op == IndexOp::Remove));
        assert!(mutations.iter().any(|m| m.op == IndexOp::Add));
    }

    #[test]
    fn count_key_encode() {
        let ck = CountKey::new("friends", 10, false);
        let encoded = ck.encode();
        assert_eq!(encoded[0], KeyKind::Count as u8);
    }

    #[test]
    fn reverse_key() {
        let uid = Uid::new(123).unwrap();
        let rk = ReverseKey::new("follows", uid);
        let key = rk.key().unwrap();
        assert_eq!(key.kind(), KeyKind::Reverse);
    }

    #[test]
    fn index_spec_builder() {
        let spec = IndexSpec::new()
            .with_tokenizer("exact")
            .with_tokenizer("term")
            .with_count()
            .with_reverse();

        assert!(spec.is_indexed());
        assert_eq!(spec.tokenizers.len(), 2);
        assert!(spec.count);
        assert!(spec.reverse);
    }
}
