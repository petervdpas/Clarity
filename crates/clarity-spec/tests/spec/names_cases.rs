//! How Clarity writes a name, pinned by conformance/names.json, which the IDE runs too.

fn cases() -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/names.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn names_are_written_as_the_cases_say() {
    let cases = cases();
    for c in cases["isCamelCase"].as_array().unwrap() {
        assert_eq!(clarity_spec::names::is_camel_case(c["name"].as_str().unwrap()), c["is"].as_bool().unwrap(), "{c}");
    }
    for c in cases["camelCase"].as_array().unwrap() {
        assert_eq!(clarity_spec::names::camel_case(c["words"].as_str().unwrap()), c["name"].as_str().unwrap(), "{c}");
    }
    for c in cases["hyphenated"].as_array().unwrap() {
        assert_eq!(clarity_spec::code::hyphenated(c["name"].as_str().unwrap()), c["line"].as_str().unwrap(), "{c}");
    }
}
