//! A sign-in the desktop program remembers between runs (`remember N days`):
//! an entry for the system's key store, `record.until.signature`, signed
//! with the person's stored secret. Editing it to name someone else breaks
//! the signature, and a changed secret ends it.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::{Problem, ProblemKind};

fn signature(record: i64, until: u64, stored: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"clarity remembered sign-in\0");
    hasher.update(stored.as_bytes());
    hasher.update(format!("\0{record}\0{until}").as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

/// The entry for `record`, for `seconds` from `now`, signed with its stored secret.
pub fn remember_make(record: i64, seconds: u64, stored: &str, now: u64) -> Result<String, Problem> {
    if stored.is_empty() {
        return Err(Problem::new(ProblemKind::BadInput, "a sign-in is remembered only for a strategy that keeps a secret"));
    }
    let until = now.saturating_add(seconds);
    Ok(format!("{record}.{until}.{}", signature(record, until, stored)))
}

fn parts(entry: &str) -> Option<(i64, u64, &str)> {
    let mut it = entry.splitn(3, '.');
    let record = it.next()?.parse().ok()?;
    let until = it.next()?.parse().ok()?;
    Some((record, until, it.next()?))
}

/// The record an entry names, or 0; whether it holds is `remember_check`'s.
pub fn remember_record(entry: &str) -> i64 {
    parts(entry).map(|(record, _, _)| record).unwrap_or(0)
}

/// The record the entry signs in, or 0 when it expired, was edited or the secret changed.
pub fn remember_check(entry: &str, stored: &str, now: u64) -> i64 {
    let Some((record, until, given)) = parts(entry) else { return 0 };
    if stored.is_empty() || now >= until {
        return 0;
    }
    if bool::from(signature(record, until, stored).as_bytes().ct_eq(given.as_bytes())) { record } else { 0 }
}
