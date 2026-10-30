// The runtime fails safe: a handler that panics, as on a list index out of
// range, becomes an error the screen shows, and the database stays usable
// for the next handler.

use std::sync::Mutex;

use clarity_runtime::{Db, Error, run_guarded};

#[test]
fn a_handler_that_panics_is_an_error_and_the_next_one_runs() {
    let db = Mutex::new(Db::memory().unwrap());
    std::panic::set_hook(Box::new(|_| {}));
    let failed = run_guarded(&db, |_| -> clarity_runtime::Result<i64> {
        let none: Vec<i64> = Vec::new();
        Ok(none[3])
    });
    let _ = std::panic::take_hook();
    let error = failed.unwrap_err();
    assert!(error.as_str().unwrap().contains("out of bounds"), "{error}");
    assert_eq!(run_guarded(&db, |_| Ok(7)).unwrap(), 7);
    // Any other error is its text, as the screen shows it.
    let refused = run_guarded(&db, |_| -> clarity_runtime::Result<()> { Err(Error::Invalid("not a number".into())) });
    assert_eq!(refused.unwrap_err(), serde_json::Value::from("invalid value: not a number"));
}
