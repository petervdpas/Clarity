//! Failed tries per key (an account, an address), counted within a window,
//! for the whole program.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn failures() -> &'static Mutex<HashMap<String, Vec<u64>>> {
    static FAILURES: OnceLock<Mutex<HashMap<String, Vec<u64>>>> = OnceLock::new();
    FAILURES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Whether another try is allowed: fewer than `tries` failures in the last `window` seconds.
pub fn tries_allowed(key: &str, tries: u32, window: u64, now: u64) -> bool {
    let mut all = failures().lock().unwrap_or_else(|e| e.into_inner());
    let Some(times) = all.get_mut(key) else { return true };
    times.retain(|t| now < t.saturating_add(window));
    (times.len() as u64) < tries as u64
}

pub fn tries_failed(key: &str, now: u64) {
    failures().lock().unwrap_or_else(|e| e.into_inner()).entry(key.to_string()).or_default().push(now);
}

/// A success forgets the key's failures.
pub fn tries_cleared(key: &str) {
    failures().lock().unwrap_or_else(|e| e.into_inner()).remove(key);
}
