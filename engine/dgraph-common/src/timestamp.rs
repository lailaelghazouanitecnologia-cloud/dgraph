//! Timestamp: MVCC transaction timestamps
//!
//! Timestamps are monotonically increasing u64 values.
//! Zero means "no timestamp" / invalid.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// MVCC timestamp - non-zero for valid transactions
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[repr(transparent)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Zero timestamp - represents "no timestamp"
    pub const ZERO: Self = Self(0);

    /// Maximum timestamp
    pub const MAX: Self = Self(u64::MAX);

    /// Create from raw value
    #[inline]
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get raw value
    #[inline]
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Check if this is a valid (non-zero) timestamp
    #[inline]
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }

    /// Increment by one
    #[inline]
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ts({})", self.0)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for Timestamp {
    #[inline]
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Timestamp> for u64 {
    #[inline]
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

/// Atomic timestamp oracle - monotonically increasing
#[derive(Debug)]
pub struct TimestampOracle {
    current: AtomicU64,
}

impl TimestampOracle {
    /// Create new oracle starting at given timestamp
    #[must_use]
    pub const fn new(start: Timestamp) -> Self {
        Self {
            current: AtomicU64::new(start.0),
        }
    }

    /// Get next timestamp (atomic increment)
    #[inline]
    pub fn next(&self) -> Timestamp {
        Timestamp(self.current.fetch_add(1, Ordering::SeqCst) + 1)
    }

    /// Get current timestamp without incrementing
    #[inline]
    #[must_use]
    pub fn current(&self) -> Timestamp {
        Timestamp(self.current.load(Ordering::SeqCst))
    }

    /// Advance to at least the given timestamp
    pub fn advance_to(&self, ts: Timestamp) {
        let mut current = self.current.load(Ordering::SeqCst);
        while current < ts.0 {
            match self.current.compare_exchange_weak(
                current,
                ts.0,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }
    }
}

impl Default for TimestampOracle {
    fn default() -> Self {
        Self::new(Timestamp::ZERO)
    }
}
