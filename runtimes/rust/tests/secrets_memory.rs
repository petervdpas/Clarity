//! Secrets kept in memory for one run (a program driven without a window,
//! apart from the person's keyring): kept and read back, gone with the run.

#![cfg(feature = "secrets")]

use clarity_runtime::secrets;

#[test]
fn secrets_in_memory_are_kept_and_read_back() {
    secrets::use_memory();
    assert_eq!(secrets::get("shop", "token"), None);
    secrets::set("shop", "token", "s3cret").unwrap();
    assert_eq!(secrets::get("shop", "token").as_deref(), Some("s3cret"));
}
