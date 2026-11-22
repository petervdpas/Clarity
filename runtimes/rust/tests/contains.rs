//! `contains` outside a query, as every stack does it (conformance/contains.json).

use serde_json::Value;

#[test]
fn text_contains_a_part_case_ignored_as_every_stack_does() {
    let file: Value = serde_json::from_str(include_str!("../../../conformance/contains.json")).unwrap();
    for case in file["cases"].as_array().unwrap() {
        let (text, part) = (case["text"].as_str().unwrap(), case["part"].as_str().unwrap());
        assert_eq!(clarity_runtime::contains(text, part), case["contains"].as_bool().unwrap(), "{text:?} contains {part:?}");
    }
}
