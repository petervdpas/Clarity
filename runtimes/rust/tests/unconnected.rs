//! Why the databases did not open (DESIGN 5.27), in Clarity's own words that
//! the project translates, as every runtime says it (conformance/unconnected.json).

use clarity_runtime::ui::Text;
use clarity_runtime::{Db, Dialect, Error, unconnected_reason};
use serde_json::Value;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../conformance/unconnected.json")).unwrap()
}

fn text(value: &Value) -> Text {
    serde_json::from_value(value.clone()).unwrap()
}

#[test]
fn a_secret_connection_not_kept_says_so() {
    let cases = cases();
    let location = cases["notKept"]["location"].as_str().unwrap();
    let err = Db::connect(&[("main", Dialect::Sqlite, location)]).err().unwrap();
    assert_eq!(unconnected_reason(&err), text(&cases["notKept"]["reason"]));
    // Its message stays as it was, for logs.
    assert!(err.to_string().contains("the secret setting neverKept is not kept yet"), "{err}");
}

#[test]
fn any_other_failure_says_the_runtimes_message() {
    let cases = cases();
    let err = Error::Storage(cases["failed"]["storage"].as_str().unwrap().into());
    assert_eq!(unconnected_reason(&err), text(&cases["failed"]["reason"]));
}
