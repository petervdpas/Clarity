//! A text by its key, filled in the backend in the language the screen
//! picked, as every runtime does (conformance/translations.json, `code`).

use clarity_runtime::i18n;
use clarity_runtime::ui::{Slot, Text};
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/translations.json");

#[test]
fn code_texts_are_translated_as_every_runtime_does() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    let code = &all["code"];
    i18n::load(&code["model"].to_string());
    let cases = code["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        i18n::set_language(case["language"].as_str());
        let mut text = Text::key(case["key"].as_str().unwrap());
        for (path, value) in case["values"].as_object().unwrap() {
            text = text.with(path, Slot::text(value.as_str().unwrap()));
        }
        assert_eq!(text.show(), case["gives"].as_str().unwrap(), "{}", case["what"]);
    }
    i18n::set_language(None);
}
