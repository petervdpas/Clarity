// Keys set by the person (DESIGN 5.28), as every renderer sets them
// (conformance/shortcut-keys.json), and where the settings file keeps them.

use std::collections::BTreeMap;

use clarity_runtime::keys::{Command, KeyRules, PersonKeys};
use serde_json::{Map, Value, json};

const CASES: &str = include_str!("../../../conformance/shortcut-keys.json");

fn cases() -> Value {
    serde_json::from_str(CASES).unwrap()
}

fn strings(value: &Value) -> Vec<String> {
    value.as_array().unwrap().iter().map(|k| k.as_str().unwrap().to_string()).collect()
}

fn commands() -> Vec<Command> {
    cases()["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| Command { id: c["id"].as_str().unwrap().to_string(), layers: strings(&c["layers"]), keys: strings(&c["keys"]) })
        .collect()
}

fn choices(value: &Value) -> BTreeMap<String, Vec<String>> {
    value.as_object().unwrap().iter().map(|(id, keys)| (id.clone(), strings(keys))).collect()
}

#[test]
fn assign_swaps_in_a_shared_place_and_keeps_only_what_differs() {
    let commands = commands();
    for case in cases()["assign"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let (id, key) = (case["id"].as_str().unwrap(), case["key"].as_str().unwrap());
        let (next, swapped) = KeyRules::assign(&commands, &choices(&case["choices"]), id, key);
        assert_eq!(next, choices(&case["expect"]["choices"]), "{name}");
        assert_eq!(swapped.as_deref(), case["expect"]["swapped"].as_str(), "{name}");
        assert_eq!(KeyRules::also_has(&commands, &choices(&case["choices"]), id, key), strings(&case["expect"]["also"]), "{name}");
    }
}

#[test]
fn reset_puts_the_design_back() {
    let commands = commands();
    for case in cases()["reset"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let next = KeyRules::reset(&commands, &choices(&case["choices"]), case["id"].as_str().unwrap());
        assert_eq!(next, choices(&case["expect"]), "{name}");
    }
}

#[test]
fn refused_says_why_a_key_cannot_be_a_commands() {
    for case in cases()["refused"].as_array().unwrap() {
        let key = case[0].as_str().unwrap();
        assert_eq!(KeyRules::refused(key), case[1].as_str(), "{key}");
    }
}

#[test]
fn a_key_with_plus_is_split_after_its_modifiers() {
    assert_eq!(KeyRules::parts("Ctrl++"), (vec!["Ctrl".to_string()], "+".to_string()));
    assert_eq!(KeyRules::parts("Ctrl+Shift+N"), (vec!["Ctrl".to_string(), "Shift".to_string()], "N".to_string()));
    assert_eq!(KeyRules::parts("F2"), (Vec::new(), "F2".to_string()));
}

#[test]
fn a_commands_keys_are_the_persons_or_the_designs() {
    let settings: Map<String, Value> = serde_json::from_value(json!({ "theme": "dark", "keys": { "Card.saveButton": ["Ctrl+K"], "quitItem": [] } })).unwrap();
    let person = PersonKeys::read(&settings);
    assert_eq!(person.of("Card.saveButton", Some("Ctrl+S")), vec!["Ctrl+K"]);
    assert_eq!(person.of("quitItem", Some("Ctrl+Q")), Vec::<String>::new());
    assert_eq!(person.of("newItem", Some("Ctrl+N")), vec!["Ctrl+N"]);
    assert_eq!(person.of("keysItem", None), Vec::<String>::new());
    assert_eq!(person.of("keysItem", Some("")), Vec::<String>::new());
}

#[test]
fn what_the_file_kept_is_read_as_lists_of_keys_only() {
    let settings: Map<String, Value> = serde_json::from_value(json!({ "keys": { "a": ["Ctrl+A"], "b": "Ctrl+B", "c": [1, "Ctrl+C"] } })).unwrap();
    let person = PersonKeys::read(&settings);
    assert_eq!(person.choices.keys().collect::<Vec<_>>(), vec!["a"]);
    assert!(PersonKeys::read(&Map::new()).choices.is_empty());
}

#[test]
fn the_persons_keys_are_kept_beside_the_settings() {
    let mut settings: Map<String, Value> = serde_json::from_value(json!({ "theme": "dark", "keys": { "old": ["F1"] } })).unwrap();
    let person = PersonKeys { choices: BTreeMap::from([("Card.printButton".to_string(), vec!["Ctrl+P".to_string()])]) };
    person.keep(&mut settings);
    assert_eq!(Value::Object(settings), json!({ "theme": "dark", "keys": { "Card.printButton": ["Ctrl+P"] } }));
}
