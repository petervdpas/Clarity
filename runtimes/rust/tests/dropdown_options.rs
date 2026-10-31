//! The dropdown's options (U1c) against the cases every runtime shares
//! (conformance/dropdown-options.json): what `find_options` answers.

use clarity_runtime::dropdown_options;
use clarity_runtime::ui::RefOption;
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../conformance/dropdown-options.json");

#[test]
fn a_typed_dropdown_offers_the_values_its_field_has() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    for case in all["suggestions"].as_array().unwrap() {
        let values: Vec<String> = serde_json::from_value(case["values"].clone()).unwrap();
        let gives = dropdown_options::suggestions(&values, case["typed"].as_str().unwrap());
        assert_eq!(json!(gives), case["gives"], "{case}");
    }
}

#[test]
fn a_search_dropdown_offers_the_records_with_every_typed_word() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    let options: Vec<RefOption> = serde_json::from_value(all["search"]["options"].clone()).unwrap();
    let show = vec!["name".to_string(), "city".to_string()];
    for case in all["search"]["cases"].as_array().unwrap() {
        let found = dropdown_options::searched(options.clone(), &show, case["typed"].as_str().unwrap());
        let ids: Vec<i64> = found.iter().map(|o| o.id).collect();
        assert_eq!(json!(ids), case["gives"], "{case}");
    }
}

#[test]
fn a_search_looks_only_in_the_fields_it_shows() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    let options: Vec<RefOption> = serde_json::from_value(all["search"]["options"].clone()).unwrap();
    let found = dropdown_options::searched(options, &["name".to_string()], "utrecht");
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn find_answers_the_list_of_its_kind() {
    let values = || Ok(vec!["Gent".to_string(), "Utrecht".to_string()]);
    let options = || Ok(vec![RefOption { id: 4, text: "Jansen".into(), ..RefOption::default() }]);
    let typed = dropdown_options::find("typed", &[], "ut", values, options).unwrap();
    assert_eq!(typed, json!(["Utrecht"]));
    let search = dropdown_options::find("search", &[], "jan", values, options).unwrap();
    assert_eq!(search, json!([{ "id": 4, "text": "Jansen" }]));
    assert!(dropdown_options::find("other", &[], "", values, options).is_err());
}
