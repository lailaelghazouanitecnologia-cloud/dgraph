//! Tests for dgraph-common

use dgraph_common::{consts, Error, Key, KeyKind, Timestamp, TimestampOracle, Uid};

// ============================================================================
// UID Tests
// ============================================================================

#[test]
fn uid_zero_is_invalid() {
    let result = Uid::new(0);
    assert!(matches!(result, Err(Error::InvalidUid(0))));
}

#[test]
fn uid_one_is_valid() {
    let uid = Uid::new(1).unwrap();
    assert_eq!(uid.get(), 1);
}

#[test]
fn uid_max_is_valid() {
    let uid = Uid::new(u64::MAX).unwrap();
    assert_eq!(uid.get(), u64::MAX);
}

#[test]
fn uid_reserved_range() {
    // 1 to 0x8000 are reserved
    assert!(Uid::new(1).unwrap().is_reserved());
    assert!(Uid::new(0x8000).unwrap().is_reserved());
    assert!(!Uid::new(0x8001).unwrap().is_reserved());
    assert!(!Uid::new(0x10000).unwrap().is_reserved());
}

#[test]
fn uid_roundtrip_bytes() {
    let values = [1u64, 42, 0xDEAD_BEEF, u64::MAX];
    for v in values {
        let uid = Uid::new(v).unwrap();
        let bytes = uid.to_be_bytes();
        let parsed = Uid::from_be_bytes(bytes).unwrap();
        assert_eq!(uid, parsed, "roundtrip failed for {v:#x}");
    }
}

#[test]
fn uid_ordering() {
    let a = Uid::new(10).unwrap();
    let b = Uid::new(20).unwrap();
    assert!(a < b);
    assert!(b > a);
    assert_eq!(a, Uid::new(10).unwrap());
}

// ============================================================================
// Timestamp Tests
// ============================================================================

#[test]
fn timestamp_zero_is_invalid() {
    let ts = Timestamp::ZERO;
    assert!(!ts.is_valid());
    assert_eq!(ts.get(), 0);
}

#[test]
fn timestamp_nonzero_is_valid() {
    let ts = Timestamp::new(1);
    assert!(ts.is_valid());
}

#[test]
fn timestamp_ordering() {
    let t1 = Timestamp::new(10);
    let t2 = Timestamp::new(20);
    assert!(t1 < t2);
    assert!(Timestamp::ZERO < t1);
    assert!(t2 < Timestamp::MAX);
}

#[test]
fn timestamp_next() {
    let t1 = Timestamp::new(10);
    let t2 = t1.next();
    assert_eq!(t2.get(), 11);
}

#[test]
fn timestamp_next_saturates() {
    let t = Timestamp::MAX;
    let next = t.next();
    assert_eq!(next, Timestamp::MAX); // saturating_add
}

// ============================================================================
// Timestamp Oracle Tests
// ============================================================================

#[test]
fn oracle_monotonic() {
    let oracle = TimestampOracle::new(Timestamp::new(100));

    let t1 = oracle.next();
    let t2 = oracle.next();
    let t3 = oracle.next();

    assert_eq!(t1.get(), 101);
    assert_eq!(t2.get(), 102);
    assert_eq!(t3.get(), 103);
    assert!(t1 < t2);
    assert!(t2 < t3);
}

#[test]
fn oracle_advance() {
    let oracle = TimestampOracle::new(Timestamp::new(10));

    oracle.advance_to(Timestamp::new(100));
    assert_eq!(oracle.current().get(), 100);

    // Advancing to lower value is no-op
    oracle.advance_to(Timestamp::new(50));
    assert_eq!(oracle.current().get(), 100);

    // Can still get next
    let next = oracle.next();
    assert_eq!(next.get(), 101);
}

#[test]
fn oracle_concurrent_safety() {
    use std::sync::Arc;
    use std::thread;

    let oracle = Arc::new(TimestampOracle::new(Timestamp::ZERO));
    let mut handles = vec![];

    for _ in 0..10 {
        let o = Arc::clone(&oracle);
        handles.push(thread::spawn(move || {
            let mut timestamps = vec![];
            for _ in 0..100 {
                timestamps.push(o.next());
            }
            timestamps
        }));
    }

    let mut all: Vec<Timestamp> = handles
        .into_iter()
        .flat_map(|h| h.join().unwrap())
        .collect();

    // All timestamps should be unique
    all.sort();
    for i in 1..all.len() {
        assert!(all[i - 1] < all[i], "duplicate timestamp detected");
    }

    // Should have exactly 1000 unique timestamps
    assert_eq!(all.len(), 1000);
}

// ============================================================================
// Key Tests
// ============================================================================

#[test]
fn key_data_roundtrip() {
    let uid = Uid::new(12345).unwrap();
    let key = Key::data("name", uid).unwrap();

    assert_eq!(key.kind(), KeyKind::Data);
    assert_eq!(key.predicate(), Some("name"));
    assert_eq!(key.subject(), Some(uid));
}

#[test]
fn key_schema() {
    let key = Key::schema("age").unwrap();
    assert_eq!(key.kind(), KeyKind::Schema);
    assert_eq!(key.predicate(), Some("age"));
}

#[test]
fn key_index() {
    let key = Key::index("name", b"alice").unwrap();
    assert_eq!(key.kind(), KeyKind::Index);
    assert_eq!(key.predicate(), Some("name"));
}

#[test]
fn key_reverse() {
    let uid = Uid::new(999).unwrap();
    let key = Key::reverse("friend", uid).unwrap();
    assert_eq!(key.kind(), KeyKind::Reverse);
    assert_eq!(key.predicate(), Some("friend"));
}

#[test]
fn key_too_large_fails() {
    let huge_pred = "x".repeat(consts::MAX_KEY_SIZE);
    let result = Key::schema(&huge_pred);
    assert!(matches!(result, Err(Error::KeyTooLarge { .. })));
}

#[test]
fn key_deterministic() {
    // Same inputs should produce identical bytes
    let uid = Uid::new(42).unwrap();
    let k1 = Key::data("test", uid).unwrap();
    let k2 = Key::data("test", uid).unwrap();
    assert_eq!(k1.as_bytes(), k2.as_bytes());
}

#[test]
fn key_different_predicates_different_bytes() {
    let uid = Uid::new(42).unwrap();
    let k1 = Key::data("name", uid).unwrap();
    let k2 = Key::data("age", uid).unwrap();
    assert_ne!(k1.as_bytes(), k2.as_bytes());
}

#[test]
fn key_different_uids_different_bytes() {
    let k1 = Key::data("name", Uid::new(1).unwrap()).unwrap();
    let k2 = Key::data("name", Uid::new(2).unwrap()).unwrap();
    assert_ne!(k1.as_bytes(), k2.as_bytes());
}

#[test]
fn key_kind_byte_values() {
    // Verify key kind bytes are stable
    assert_eq!(KeyKind::Data as u8, 0x00);
    assert_eq!(KeyKind::Reverse as u8, 0x01);
    assert_eq!(KeyKind::Index as u8, 0x02);
    assert_eq!(KeyKind::Count as u8, 0x03);
    assert_eq!(KeyKind::Schema as u8, 0x04);
    assert_eq!(KeyKind::Type as u8, 0x05);
}
