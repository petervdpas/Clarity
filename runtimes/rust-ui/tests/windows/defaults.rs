//! Clarity's defaults are the spec's (conformance/defaults.json); the copies
//! Clarity.UI carries say the same.

#[test]
fn a_toast_shows_as_long_as_the_spec_says_when_it_says_nothing() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/defaults.json");
    let file: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(file["toastLifespan"]["default"], clarity_ui::window::messages::DEFAULT_TOAST_LIFESPAN_MS);
}
