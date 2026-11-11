//! Sessions: a token naming a signed-in record until it expires or is
//! closed. Kept by the token's hash, so the program's memory does not hold
//! the tokens browsers carry.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::Problem;
use crate::hash::{hash, new_token};

struct Session {
    record: i64,
    until: u64,
}

fn sessions() -> &'static Mutex<HashMap<String, Session>> {
    static SESSIONS: OnceLock<Mutex<HashMap<String, Session>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn key(token: &str) -> String {
    hash("sha256", token).expect("sha256 always hashes")
}

/// A new session for the record, for `seconds` from `now`; its token.
pub fn session_open(record: i64, seconds: u64, now: u64) -> Result<String, Problem> {
    let token = new_token()?;
    let mut all = sessions().lock().unwrap_or_else(|e| e.into_inner());
    all.retain(|_, s| now < s.until);
    all.insert(key(&token), Session { record, until: now.saturating_add(seconds) });
    Ok(token)
}

/// The record the token names, or 0 when it names none (unknown, closed or expired).
pub fn session_find(token: &str, now: u64) -> i64 {
    let all = sessions().lock().unwrap_or_else(|e| e.into_inner());
    match all.get(&key(token)) {
        Some(s) if now < s.until => s.record,
        _ => 0,
    }
}

pub fn session_close(token: &str) {
    sessions().lock().unwrap_or_else(|e| e.into_inner()).remove(&key(token));
}

/// Signs the record out everywhere (its password changed, it was removed).
pub fn session_close_all(record: i64) {
    sessions().lock().unwrap_or_else(|e| e.into_inner()).retain(|_, s| s.record != record);
}

/// Signs the record out everywhere but in the session of `token` (its password changed there).
pub fn session_close_others(record: i64, token: &str) {
    let kept = key(token);
    sessions().lock().unwrap_or_else(|e| e.into_inner()).retain(|k, s| s.record != record || *k == kept);
}
