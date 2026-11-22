//! The dropdown's options (U1c) against the cases every runtime shares
//! (conformance/dropdown-options.json), as Clarity.UI offers them with these.

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
fn a_program_offers_every_record_with_the_fields_shown() {
    let options = vec![RefOption { id: 4, text: "Jansen".into(), fields: [("city".to_string(), json!("Gent")), ("age".to_string(), json!(40))].into_iter().collect() }];
    let offered = dropdown_options::offered(options, &["city".to_string()]).unwrap();
    assert_eq!(offered, json!([{ "id": 4, "text": "Jansen", "fields": { "city": "Gent" } }]));
}
