//! What a window says when code fails at a value or a record: Clarity's own
//! words by a literal key (translated where the project has them), while the
//! error's message stays the English of the shared cases.

use clarity_runtime::program::said;
use clarity_runtime::Error;

#[test]
fn an_item_outside_its_list_is_said_by_its_key() {
    let failed = clarity_runtime::at(&[1_i64], 10).unwrap_err();
    assert_eq!(failed.to_string(), "there is no item 10 in a list of 1");
    let shown = said(&failed);
    assert_eq!(shown.key.as_deref(), Some("itemOutsideList"));
    assert_eq!(shown.show(), "There is no item 10 in a list of 1");
}

#[test]
fn a_record_not_there_or_in_use_is_said_by_its_key() {
    let missing = said(&Error::NotFound { table: "Customer", id: Some(3) });
    assert_eq!(missing.key.as_deref(), Some("recordNotThere"));
    assert_eq!(missing.show(), "There is no Customer with id 3");
    assert_eq!(said(&Error::InUse { table: "Customer", id: 3 }).key.as_deref(), Some("recordStillUsed"));
    // The message stays English, for logs and the shared cases.
    assert_eq!(Error::NotFound { table: "Customer", id: Some(3) }.to_string(), "there is no Customer with id 3");
}

#[test]
fn a_window_says_the_failure_in_clarity_s_own_words() {
    let db = std::sync::Mutex::new(clarity_runtime::Db::memory().unwrap());
    let failed = clarity_runtime::program::guarded(&db, |_| clarity_runtime::at(&[1_i64], 10).copied());
    assert_eq!(failed.unwrap_err().key.as_deref(), Some("itemOutsideList"));
}
