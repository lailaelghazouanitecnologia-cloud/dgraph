//! Key: Storage key encoding
//!
//! Keys are prefixed by type byte, then encoded deterministically.
//! Format: [kind:1][payload:variable]

use crate::{consts, Error, Result, Uid};
use bytes::{BufMut, Bytes, BytesMut};

/// Key type discriminator - first byte of every key
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum KeyKind {
    /// Data key: predicate + subject uid
    Data = 0x00,
    /// Reverse index: predicate + object uid
    Reverse = 0x01,
    /// Index key: predicate + index token
    Index = 0x02,
    /// Count index: predicate + count
    Count = 0x03,
    /// Schema key: predicate
    Schema = 0x04,
    /// Type key: type name
    Type = 0x05,
}

impl TryFrom<u8> for KeyKind {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x00 => Ok(Self::Data),
            0x01 => Ok(Self::Reverse),
            0x02 => Ok(Self::Index),
            0x03 => Ok(Self::Count),
            0x04 => Ok(Self::Schema),
            0x05 => Ok(Self::Type),
            _ => Err(Error::Corruption(format!("invalid key kind: {value:#x}"))),
        }
    }
}

/// Storage key - immutable, validated on construction
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    data: Bytes,
}

impl Key {
    /// Create a data key: predicate + subject UID
    ///
    /// # Errors
    /// Returns error if key would exceed max size
    pub fn data(predicate: &str, subject: Uid) -> Result<Self> {
        Self::build(KeyKind::Data, predicate, Some(subject), None)
    }

    /// Create a reverse key: predicate + object UID
    ///
    /// # Errors
    /// Returns error if key would exceed max size
    pub fn reverse(predicate: &str, object: Uid) -> Result<Self> {
        Self::build(KeyKind::Reverse, predicate, Some(object), None)
    }

    /// Create an index key: predicate + token
    ///
    /// # Errors
    /// Returns error if key would exceed max size
    pub fn index(predicate: &str, token: &[u8]) -> Result<Self> {
        Self::build(KeyKind::Index, predicate, None, Some(token))
    }

    /// Create a schema key
    ///
    /// # Errors
    /// Returns error if key would exceed max size
    pub fn schema(predicate: &str) -> Result<Self> {
        Self::build(KeyKind::Schema, predicate, None, None)
    }

    fn build(
        kind: KeyKind,
        predicate: &str,
        uid: Option<Uid>,
        token: Option<&[u8]>,
    ) -> Result<Self> {
        let pred_bytes = predicate.as_bytes();
        let token_len = token.map_or(0, <[u8]>::len);
        let uid_len = if uid.is_some() { 8 } else { 0 };

        // 1 (kind) + 2 (pred len) + pred + uid + token
        let total = 1 + 2 + pred_bytes.len() + uid_len + token_len;

        if total > consts::MAX_KEY_SIZE {
            return Err(Error::KeyTooLarge {
                size: total,
                max: consts::MAX_KEY_SIZE,
            });
        }

        let mut buf = BytesMut::with_capacity(total);

        // Kind byte
        buf.put_u8(kind as u8);

        // Predicate: length-prefixed
        #[allow(clippy::cast_possible_truncation)]
        buf.put_u16(pred_bytes.len() as u16);
        buf.put_slice(pred_bytes);

        // UID if present
        if let Some(u) = uid {
            buf.put_u64(u.get());
        }

        // Token if present
        if let Some(t) = token {
            buf.put_slice(t);
        }

        Ok(Self { data: buf.freeze() })
    }

    /// Get the key kind
    ///
    /// # Panics
    /// Panics if key is empty (should never happen with valid key)
    #[must_use]
    pub fn kind(&self) -> KeyKind {
        debug_assert!(!self.data.is_empty());
        KeyKind::try_from(self.data[0]).expect("valid key kind")
    }

    /// Get raw bytes
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Parse predicate from key
    #[must_use]
    pub fn predicate(&self) -> Option<&str> {
        if self.data.len() < 3 {
            return None;
        }
        let len = u16::from_be_bytes([self.data[1], self.data[2]]) as usize;
        if self.data.len() < 3 + len {
            return None;
        }
        std::str::from_utf8(&self.data[3..3 + len]).ok()
    }

    /// Parse subject UID from data key
    #[must_use]
    pub fn subject(&self) -> Option<Uid> {
        if self.kind() != KeyKind::Data {
            return None;
        }
        self.extract_uid()
    }

    fn extract_uid(&self) -> Option<Uid> {
        if self.data.len() < 3 {
            return None;
        }
        let pred_len = u16::from_be_bytes([self.data[1], self.data[2]]) as usize;
        let uid_start = 3 + pred_len;
        if self.data.len() < uid_start + 8 {
            return None;
        }
        let uid_bytes: [u8; 8] = self.data[uid_start..uid_start + 8].try_into().ok()?;
        Uid::from_be_bytes(uid_bytes).ok()
    }
}

impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}
