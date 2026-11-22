//! Clarity's defaults are the spec's (conformance/defaults.json); the copies
//! this runtime carries say the same.

fn defaults() -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/defaults.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn the_defaults_are_the_spec_s() {
    let file = defaults();
    assert_eq!(file["undoSteps"], clarity_runtime::UNDO_STEPS);
    assert_eq!(file["keptBackups"], clarity_runtime::backup::KEPT_AUTOMATIC);
    assert_eq!(file["ownersPerRead"], clarity_runtime::OWNERS_PER_READ);
    assert_eq!(file["editBusKeeps"], clarity_runtime::edits::EDIT_BUS_KEEPS);
    assert_eq!(file["quietMinutes"].as_u64().unwrap() * 60, clarity_runtime::serve::QUIET.as_secs());
}

#[cfg(feature = "http")]
#[test]
fn an_http_call_may_take_what_the_spec_says() {
    assert_eq!(defaults()["httpSeconds"].as_u64().unwrap(), clarity_runtime::http::TIMEOUT.as_secs());
}

#[cfg(feature = "auth")]
#[test]
fn the_sign_in_defaults_are_the_spec_s() {
    let file = defaults();
    let source = clarity_runtime::signin::SigninSource::new("Person", "Signin");
    assert_eq!(file["signin"]["sessionMinutes"].as_u64().unwrap() * 60, source.session_seconds as u64);
    assert_eq!(file["signin"]["tries"].as_u64().unwrap(), source.tries as u64);
    assert_eq!(file["signin"]["triesMinutes"].as_u64().unwrap() * 60, source.tries_seconds as u64);
}
