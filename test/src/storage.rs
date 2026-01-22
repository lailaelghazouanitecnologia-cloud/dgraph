//! Tests for dgraph-storage

use bytes::Bytes;
use dgraph_common::{Key, Timestamp, Uid};
use dgraph_storage::{Facet, Op, Posting, PostingList, PostingType, Store, StoreConfig, ValType};
use std::sync::Arc;

// ============================================================================
// Posting Tests
// ============================================================================

#[test]
fn posting_reference() {
    let uid = Uid::new(42).unwrap();
    let ts = Timestamp::new(100);
    let posting = Posting::reference(uid, ts);

    assert_eq!(posting.uid, 42);
    assert_eq!(posting.posting_type, PostingType::Ref);
    assert_eq!(posting.val_type, ValType::Uid);
    assert_eq!(posting.commit_ts, ts);
    assert!(posting.value.is_empty());
}

#[test]
fn posting_value() {
    let ts = Timestamp::new(100);
    let data = Bytes::from("hello");
    let posting = Posting::value(data.clone(), ts);

    assert_eq!(posting.uid, 0);
    assert_eq!(posting.posting_type, PostingType::Value);
    assert_eq!(posting.commit_ts, ts);
    assert_eq!(posting.value, data);
}

#[test]
fn posting_typed_values() {
    let ts = Timestamp::new(100);

    // String posting
    let p = Posting::string("hello", ts);
    assert_eq!(p.val_type, ValType::String);
    assert_eq!(p.value_as_str(), Some("hello"));

    // Int posting
    let p = Posting::int(42, ts);
    assert_eq!(p.val_type, ValType::Int);
    assert_eq!(p.value_as_int(), Some(42));

    // Float posting
    let p = Posting::float(3.14, ts);
    assert_eq!(p.val_type, ValType::Float);
    assert!((p.value_as_float().unwrap() - 3.14).abs() < 0.001);

    // Bool posting
    let p = Posting::bool(true, ts);
    assert_eq!(p.val_type, ValType::Bool);
    assert_eq!(p.value_as_bool(), Some(true));
}

#[test]
fn posting_with_lang() {
    let ts = Timestamp::new(100);
    let posting = Posting::value_lang(Bytes::from("Hola"), "es", ts);

    assert_eq!(posting.posting_type, PostingType::ValueLang);
    assert_eq!(posting.lang(), Some("es"));
}

#[test]
fn posting_with_facet() {
    let uid = Uid::new(42).unwrap();
    let ts = Timestamp::new(100);
    let posting = Posting::reference(uid, ts)
        .with_facet("weight".to_string(), Bytes::from("0.5"));

    assert_eq!(posting.facets.len(), 1);
    assert_eq!(posting.facets[0].key, "weight");
}

#[test]
fn posting_with_typed_facet() {
    let uid = Uid::new(42).unwrap();
    let ts = Timestamp::new(100);

    let facet = Facet::float("weight", 0.5);
    let posting = Posting::reference(uid, ts).with_typed_facet(facet);

    assert_eq!(posting.facets.len(), 1);
    assert_eq!(posting.facet("weight").unwrap().as_float(), Some(0.5));
}

#[test]
fn posting_visibility() {
    let uid = Uid::new(42).unwrap();
    let posting = Posting::reference(uid, Timestamp::new(100));

    // Visible at commit_ts and after
    assert!(posting.visible_at(Timestamp::new(100)));
    assert!(posting.visible_at(Timestamp::new(150)));

    // Not visible before commit_ts
    assert!(!posting.visible_at(Timestamp::new(50)));
    assert!(!posting.visible_at(Timestamp::new(99)));
}

#[test]
fn posting_is_ref_is_value() {
    let uid = Uid::new(42).unwrap();
    let ts = Timestamp::new(100);

    let ref_posting = Posting::reference(uid, ts);
    assert!(ref_posting.is_ref());
    assert!(!ref_posting.is_value());

    let val_posting = Posting::string("test", ts);
    assert!(!val_posting.is_ref());
    assert!(val_posting.is_value());
}

// ============================================================================
// Posting List Tests
// ============================================================================

#[test]
fn posting_list_empty() {
    let pl = PostingList::new();
    assert!(pl.is_empty());
    assert_eq!(pl.len(), 0);
}

#[test]
fn posting_list_add_single() {
    let mut pl = PostingList::new();
    let uid = Uid::new(42).unwrap();
    let posting = Posting::reference(uid, Timestamp::new(100));

    pl.add(posting);

    assert!(!pl.is_empty());
    assert_eq!(pl.len(), 1);
}

#[test]
fn posting_list_maintains_order() {
    let mut pl = PostingList::new();

    // Add in non-sorted order
    pl.add(Posting::reference(Uid::new(30).unwrap(), Timestamp::new(100)));
    pl.add(Posting::reference(Uid::new(10).unwrap(), Timestamp::new(100)));
    pl.add(Posting::reference(Uid::new(20).unwrap(), Timestamp::new(100)));

    // Should be sorted by uid
    let uids: Vec<u64> = pl
        .iter_at(Timestamp::new(100))
        .map(|p| p.uid)
        .collect();

    assert_eq!(uids, vec![10, 20, 30]);
}

#[test]
fn posting_list_mvcc_visibility() {
    let mut pl = PostingList::new();
    let uid = Uid::new(1).unwrap();

    // Add multiple versions
    pl.add(Posting::reference(uid, Timestamp::new(100)));
    pl.add(Posting::reference(uid, Timestamp::new(200)));
    pl.add(Posting::reference(uid, Timestamp::new(300)));

    // At ts=50: nothing visible
    assert_eq!(pl.count_at(Timestamp::new(50)), 0);

    // At ts=150: only version at 100 visible
    assert_eq!(pl.count_at(Timestamp::new(150)), 1);
    let p = pl.get(uid, Timestamp::new(150)).unwrap();
    assert_eq!(p.commit_ts, Timestamp::new(100));

    // At ts=250: version at 200 is newest visible
    let p = pl.get(uid, Timestamp::new(250)).unwrap();
    assert_eq!(p.commit_ts, Timestamp::new(200));

    // At ts=300: version at 300 visible
    let p = pl.get(uid, Timestamp::new(300)).unwrap();
    assert_eq!(p.commit_ts, Timestamp::new(300));
}

#[test]
fn posting_list_timestamps() {
    let mut pl = PostingList::new();

    pl.add(Posting::reference(Uid::new(1).unwrap(), Timestamp::new(100)));
    pl.add(Posting::reference(Uid::new(2).unwrap(), Timestamp::new(200)));
    pl.add(Posting::reference(Uid::new(3).unwrap(), Timestamp::new(150)));

    assert_eq!(pl.min_ts(), Timestamp::new(100));
    assert_eq!(pl.max_ts(), Timestamp::new(200));
}

#[test]
fn posting_list_uids_at() {
    let mut pl = PostingList::new();

    pl.add(Posting::reference(Uid::new(10).unwrap(), Timestamp::new(100)));
    pl.add(Posting::reference(Uid::new(20).unwrap(), Timestamp::new(100)));
    pl.add(Posting::value(Bytes::from("text"), Timestamp::new(100))); // value, not ref

    let uids: Vec<Uid> = pl.uids_at(Timestamp::new(100)).collect();
    assert_eq!(uids.len(), 2);
    assert!(uids.contains(&Uid::new(10).unwrap()));
    assert!(uids.contains(&Uid::new(20).unwrap()));
}

#[test]
fn posting_list_gc() {
    let mut pl = PostingList::new();
    let uid = Uid::new(1).unwrap();

    // Add multiple versions
    pl.add(Posting::reference(uid, Timestamp::new(100)));
    pl.add(Posting::reference(uid, Timestamp::new(200)));
    pl.add(Posting::reference(uid, Timestamp::new(300)));

    assert_eq!(pl.len(), 3);

    // GC below watermark 250
    let removed = pl.gc(Timestamp::new(250));
    assert!(removed > 0);
}

#[test]
fn posting_list_merge() {
    let mut pl1 = PostingList::new();
    pl1.add(Posting::reference(Uid::new(1).unwrap(), Timestamp::new(100)));

    let mut pl2 = PostingList::new();
    pl2.add(Posting::reference(Uid::new(2).unwrap(), Timestamp::new(100)));

    pl1.merge(pl2);
    assert_eq!(pl1.count_at(Timestamp::new(100)), 2);
}

// ============================================================================
// Store Tests
// ============================================================================

#[test]
fn store_empty() {
    let store = Store::new(StoreConfig::default());
    assert!(store.is_empty());
    assert_eq!(store.len(), 0);
}

#[test]
fn store_transaction_basic() {
    let store = Store::new(StoreConfig::default());

    // Start transaction
    let txn = store.begin();
    let start_ts = txn.start_ts();
    assert!(start_ts.is_valid());

    // Record a write
    let key = Key::data("name", Uid::new(1).unwrap()).unwrap();
    let posting = Posting::value(Bytes::from("Alice"), start_ts);
    txn.write(key.clone(), posting);

    // Commit
    let commit_ts = store.next_ts();
    store.apply(txn, commit_ts).unwrap();

    // Verify
    assert_eq!(store.len(), 1);
}

#[test]
fn store_read_at_timestamp() {
    let store = Arc::new(Store::new(StoreConfig::default()));
    let uid = Uid::new(1).unwrap();

    // Write at t=1
    {
        let txn = store.begin();
        let key = Key::data("name", uid).unwrap();
        let posting = Posting::value(Bytes::from("Alice"), txn.start_ts());
        txn.write(key, posting);
        let commit_ts = store.next_ts();
        store.apply(txn, commit_ts).unwrap();
    }

    // Write at t=3 (update)
    {
        let txn = store.begin();
        let key = Key::data("name", uid).unwrap();
        let posting = Posting::value(Bytes::from("Bob"), txn.start_ts());
        txn.write(key, posting);
        let commit_ts = store.next_ts();
        store.apply(txn, commit_ts).unwrap();
    }

    let key = Key::data("name", uid).unwrap();

    // Read at different timestamps
    // Note: exact timestamps depend on oracle state
    let current = store.current_ts();
    let pl = store.get(&key, current).unwrap();
    assert!(!pl.is_empty());
}

#[test]
fn store_multiple_keys() {
    let store = Store::new(StoreConfig::default());

    let txn = store.begin();

    // Write multiple keys
    for i in 1..=10 {
        let key = Key::data("name", Uid::new(i).unwrap()).unwrap();
        let posting = Posting::value(Bytes::from(format!("User{i}")), txn.start_ts());
        txn.write(key, posting);
    }

    let commit_ts = store.next_ts();
    store.apply(txn, commit_ts).unwrap();

    assert_eq!(store.len(), 10);
}

#[test]
fn store_gc() {
    let store = Store::new(StoreConfig::default());

    // Do some transactions
    for i in 1..=5 {
        let txn = store.begin();
        let key = Key::data("counter", Uid::new(1).unwrap()).unwrap();
        let posting = Posting::value(Bytes::from(format!("{i}")), txn.start_ts());
        txn.write(key, posting);
        let commit_ts = store.next_ts();
        store.apply(txn, commit_ts).unwrap();
    }

    // Advance watermark and GC
    store.advance_watermark(store.current_ts());
    store.gc();

    // Store should still work
    let txn = store.begin();
    assert!(txn.start_ts().is_valid());
}

// ============================================================================
// ValType Tests
// ============================================================================

#[test]
fn val_type_from_u8() {
    assert_eq!(ValType::from_u8(0), Some(ValType::Default));
    assert_eq!(ValType::from_u8(2), Some(ValType::Int));
    assert_eq!(ValType::from_u8(3), Some(ValType::Float));
    assert_eq!(ValType::from_u8(9), Some(ValType::String));
    assert_eq!(ValType::from_u8(255), None);
}

#[test]
fn val_type_is_numeric() {
    assert!(ValType::Int.is_numeric());
    assert!(ValType::Float.is_numeric());
    assert!(ValType::BigFloat.is_numeric());
    assert!(!ValType::String.is_numeric());
    assert!(!ValType::Bool.is_numeric());
}

#[test]
fn val_type_is_string_like() {
    assert!(ValType::String.is_string_like());
    assert!(ValType::Password.is_string_like());
    assert!(ValType::Default.is_string_like());
    assert!(!ValType::Int.is_string_like());
}

// ============================================================================
// Facet Tests
// ============================================================================

#[test]
fn facet_string() {
    let f = Facet::string("key", "value");
    assert_eq!(f.key, "key");
    assert_eq!(f.as_str(), Some("value"));
    assert_eq!(f.val_type, ValType::String);
}

#[test]
fn facet_int() {
    let f = Facet::int("count", 42);
    assert_eq!(f.as_int(), Some(42));
    assert_eq!(f.val_type, ValType::Int);
}

#[test]
fn facet_float() {
    let f = Facet::float("weight", 3.14);
    assert!((f.as_float().unwrap() - 3.14).abs() < 0.001);
    assert_eq!(f.val_type, ValType::Float);
}

#[test]
fn facet_bool() {
    let f = Facet::bool("active", true);
    assert_eq!(f.as_bool(), Some(true));
    assert_eq!(f.val_type, ValType::Bool);
}
