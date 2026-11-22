//! Clarity's defaults are the spec's (conformance/defaults.json); the copies
//! Clarity.Auth carries say the same.

#[test]
fn the_defaults_are_the_spec_s() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/defaults.json");
    let file: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(file["signin"]["tries"], clarity_auth::TRIES);
    assert_eq!(file["signin"]["triesMinutes"].as_u64().unwrap() * 60, clarity_auth::TRIES_SECONDS);
    assert_eq!(file["httpSeconds"], clarity_auth::HTTP_SECONDS);
}
