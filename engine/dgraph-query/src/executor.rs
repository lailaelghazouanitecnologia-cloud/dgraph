//! Executor: Query execution engine
//!
//! Executes parsed queries against storage.

use crate::ast::{FuncName, Query, Value};
use dgraph_common::{Key, Result, Timestamp, Uid};
use dgraph_storage::{PostingType, Store};
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;

/// Query executor
pub struct Executor {
    store: Arc<Store>,
}

impl Executor {
    /// Create new executor
    #[must_use]
    pub fn new(store: Arc<Store>) -> Self {
        Self { store }
    }

    /// Execute a query
    ///
    /// # Errors
    /// Returns error on execution failure
    pub fn execute(&self, query: &Query, read_ts: Timestamp) -> Result<JsonValue> {
        // Get starting UIDs from root function
        let uids = self.resolve_root(query)?;

        if uids.is_empty() {
            return Ok(json!([]));
        }

        // Execute query for each UID
        let results: Vec<JsonValue> = uids
            .iter()
            .filter_map(|uid| self.execute_for_uid(query, *uid, read_ts).ok())
            .collect();

        // Apply first/offset
        let results = self.apply_pagination(results, query);

        Ok(JsonValue::Array(results))
    }

    /// Resolve root function to get starting UIDs
    fn resolve_root(&self, query: &Query) -> Result<Vec<Uid>> {
        // Direct UIDs
        if !query.uids.is_empty() {
            return Ok(query.uids.clone());
        }

        // Root function
        if let Some(ref func) = query.func {
            match func.name {
                FuncName::Uid => {
                    let uids: Vec<Uid> = func
                        .args
                        .iter()
                        .filter_map(|v| match v {
                            Value::Uid(u) => Uid::new(*u).ok(),
                            Value::Int(i) if *i > 0 => Uid::new(*i as u64).ok(),
                            _ => None,
                        })
                        .collect();
                    return Ok(uids);
                }
                FuncName::Has => {
                    // Would need index scan - return empty for now
                    return Ok(Vec::new());
                }
                FuncName::Eq => {
                    // Would need index lookup - return empty for now
                    return Ok(Vec::new());
                }
                _ => {}
            }
        }

        Ok(Vec::new())
    }

    /// Execute query for a single UID
    fn execute_for_uid(
        &self,
        query: &Query,
        uid: Uid,
        read_ts: Timestamp,
    ) -> Result<JsonValue> {
        let mut result = serde_json::Map::new();

        // Add uid field
        result.insert("uid".to_string(), json!(format!("{:#x}", uid.get())));

        // Fetch each child predicate
        for child in &query.children {
            if let Some(ref attr) = child.attr {
                let key = Key::data(attr, uid)?;

                if let Some(posting_list) = self.store.get(&key, read_ts) {
                    // Check filter
                    if !self.check_filter(child, &posting_list, read_ts) {
                        continue;
                    }

                    // Get values or expand edges
                    let values: Vec<JsonValue> = posting_list
                        .iter_at(read_ts)
                        .filter_map(|p| {
                            match p.posting_type {
                                PostingType::Value | PostingType::ValueLang => {
                                    // Parse value as JSON
                                    serde_json::from_slice(&p.value).ok()
                                }
                                PostingType::Ref => {
                                    // Recursively expand if there are children
                                    if child.children.is_empty() {
                                        Uid::new(p.uid).ok().map(|u| {
                                            json!({"uid": format!("{:#x}", u.get())})
                                        })
                                    } else {
                                        Uid::new(p.uid)
                                            .ok()
                                            .and_then(|u| {
                                                self.execute_for_uid(child, u, read_ts).ok()
                                            })
                                    }
                                }
                            }
                        })
                        .collect();

                    // Single value or array
                    if values.len() == 1 && posting_list.iter_at(read_ts).any(|p| p.posting_type == PostingType::Value || p.posting_type == PostingType::ValueLang) {
                        result.insert(attr.clone(), values.into_iter().next().unwrap());
                    } else if !values.is_empty() {
                        result.insert(attr.clone(), JsonValue::Array(values));
                    }
                }
            }
        }

        Ok(JsonValue::Object(result))
    }

    /// Check if posting list passes filter
    fn check_filter(
        &self,
        query: &Query,
        _posting_list: &dgraph_storage::PostingList,
        _read_ts: Timestamp,
    ) -> bool {
        // Simplified: always pass
        // Full implementation would evaluate filter expression
        query.filter.is_none()
    }

    /// Apply first/offset pagination
    fn apply_pagination(&self, mut results: Vec<JsonValue>, query: &Query) -> Vec<JsonValue> {
        if let Some(offset) = query.offset {
            if (offset as usize) < results.len() {
                results = results.split_off(offset as usize);
            } else {
                results.clear();
            }
        }

        if let Some(first) = query.first {
            results.truncate(first as usize);
        }

        results
    }
}
