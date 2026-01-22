//! UID: Unique identifier for graph nodes
//!
//! UIDs are non-zero u64 values. Zero is invalid.
//! Range 1..=0x8000 is reserved for internal use.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Reserved UID range upper bound (exclusive)
const RESERVED_MAX: u64 = 0x8001;

/// Unique identifier for a node in the graph
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Uid(u64);

impl Uid {
    /// Create a new UID, validating it's not zero
    ///
    /// # Errors
    /// Returns error if value is zero
    #[inline]
    pub const fn new(value: u64) -> Result<Self> {
        if value == 0 {
            return Err(Error::InvalidUid(0));
        }
        Ok(Self(value))
    }

    /// Create UID without validation - caller must ensure non-zero
    ///
    /// # Safety
    /// Value must be non-zero
    #[inline]
    #[must_use]
    pub const fn new_unchecked(value: u64) -> Self {
        debug_assert!(value != 0, "UID cannot be zero");
        Self(value)
    }

    /// Get the raw u64 value
    #[inline]
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Check if this UID is in the reserved range
    #[inline]
    #[must_use]
    pub const fn is_reserved(self) -> bool {
        self.0 < RESERVED_MAX
    }

    /// Minimum valid user UID
    #[must_use]
    pub const fn min_user() -> Self {
        Self(RESERVED_MAX)
    }

    /// Convert to big-endian bytes
    #[inline]
    #[must_use]
    pub const fn to_be_bytes(self) -> [u8; 8] {
        self.0.to_be_bytes()
    }

    /// Parse from big-endian bytes
    ///
    /// # Errors
    /// Returns error if parsed value is zero
    #[inline]
    pub const fn from_be_bytes(bytes: [u8; 8]) -> Result<Self> {
        Self::new(u64::from_be_bytes(bytes))
    }
}

impl fmt::Debug for Uid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Uid({:#x})", self.0)
    }
}

impl fmt::Display for Uid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl TryFrom<u64> for Uid {
    type Error = Error;

    #[inline]
    fn try_from(value: u64) -> Result<Self> {
        Self::new(value)
    }
}

impl From<Uid> for u64 {
    #[inline]
    fn from(uid: Uid) -> Self {
        uid.0
    }
}
