//! One sign-in attempt, decided in one place for every stack
//! (`signin_begin`, `signin_decide`): the runtime only looks the person up
//! and stores what it is told. Begin: the strategy is the project's and
//! works on the target, the inputs find the person, and tries are allowed.
//! Decide: the person found is checked, what changes is stored (a recovery
//! code used up), the first account is made where none is, and a refusal
//! counts as a failed try.

use clarity_auth::attempt::{Begun, Block, BlockStrategy, Decided, signin_begin, signin_decide};
use serde_json::{Map, Value, json};

fn block() -> Block {
    Block {
        strategies: vec![
            BlockStrategy { name: "password".into(), columns: vec!["email".into(), "password_hash".into()], options: Map::new() },
            BlockStrategy { name: "recovery".into(), columns: vec!["email".into(), "codes".into()], options: Map::new() },
            BlockStrategy { name: "account".into(), columns: vec!["login".into()], options: Map::new() },
        ],
        tries: 2,
        tries_seconds: 900,
    }
}

fn inputs(pairs: &[(&str, &str)]) -> Map<String, Value> {
    pairs.iter().map(|(k, v)| (k.to_string(), json!(v))).collect()
}

fn lookup(begun: Begun) -> (String, Option<String>, String, Vec<String>) {
    match begun {
        Begun::Lookup { column, secret, equals, keys } => (column, secret, equals, keys),
        Begun::Refused(key) => panic!("refused: {key}"),
    }
}

#[test]
fn begin_says_where_to_look_the_person_up() {
    let typed = inputs(&[("name", "ann@example.com"), ("password", "right")]);
    let (column, secret, equals, keys) = lookup(signin_begin(&block(), "password", &typed, "desktop", Some("10.0.0.1"), 0));
    assert_eq!((column.as_str(), secret.as_deref(), equals.as_str()), ("email", Some("password_hash"), "ann@example.com"));
    assert_eq!(keys, vec!["account:ann@example.com", "address:10.0.0.1"]);
}

#[test]
fn begin_refuses_a_strategy_the_project_does_not_name_or_that_does_not_work_here() {
    assert_eq!(signin_begin(&block(), "token", &inputs(&[("token", "t")]), "desktop", None, 0), Begun::Refused("signinRefused"));
    // The computer's account works on the desktop and the command line, not served.
    assert_eq!(signin_begin(&block(), "account", &Map::new(), "web", None, 0), Begun::Refused("signinRefused"));
    // Missing inputs refuse as a wrong password does.
    assert_eq!(signin_begin(&block(), "password", &inputs(&[("name", "x")]), "desktop", None, 0), Begun::Refused("signinRefused"));
}

#[test]
fn decide_signs_in_the_person_whose_secret_fits() {
    let hash = clarity_auth::hash("argon2id", "right").unwrap();
    let typed = inputs(&[("name", "cy@example.com"), ("password", "right")]);
    let (_, _, _, keys) = lookup(signin_begin(&block(), "password", &typed, "desktop", None, 0));
    assert_eq!(signin_decide(&block(), "password", &typed, &keys, Some(&hash), false, 0), Decided::SignedIn { store: Map::new() });
    let wrong = inputs(&[("name", "cy@example.com"), ("password", "wrong")]);
    assert_eq!(signin_decide(&block(), "password", &wrong, &keys, Some(&hash), false, 0), Decided::Refused("signinRefused"));
}

#[test]
fn failed_tries_lock_the_account_until_a_sign_in_works() {
    let hash = clarity_auth::hash("argon2id", "right").unwrap();
    let wrong = inputs(&[("name", "dee@example.com"), ("password", "wrong")]);
    for _ in 0..2 {
        let (_, _, _, keys) = lookup(signin_begin(&block(), "password", &wrong, "desktop", None, 100));
        assert_eq!(signin_decide(&block(), "password", &wrong, &keys, Some(&hash), false, 100), Decided::Refused("signinRefused"));
    }
    assert_eq!(signin_begin(&block(), "password", &wrong, "desktop", None, 100), Begun::Refused("signinTooManyTries"));
    // The window passes: tries again.
    assert!(matches!(signin_begin(&block(), "password", &wrong, "desktop", None, 100 + 901), Begun::Lookup { .. }));
}

#[test]
fn a_recovery_code_is_used_up_and_stored_by_its_column() {
    let codes = clarity_auth::hash("sha256-lines", "one\ntwo").unwrap();
    let typed = inputs(&[("name", "eve@example.com"), ("code", "one")]);
    let (column, secret, _, keys) = lookup(signin_begin(&block(), "recovery", &typed, "desktop", None, 0));
    assert_eq!((column.as_str(), secret.as_deref()), ("email", Some("codes")));
    let Decided::SignedIn { store } = signin_decide(&block(), "recovery", &typed, &keys, Some(&codes), false, 0) else { panic!("signed in") };
    let left = store.get("codes").and_then(Value::as_str).expect("the codes column");
    assert_ne!(left, codes, "the code used is gone");
}

#[test]
fn the_first_account_is_made_when_there_is_nobody_yet() {
    let typed = inputs(&[("name", "fay@example.com"), ("password", "right")]);
    let (_, _, _, keys) = lookup(signin_begin(&block(), "password", &typed, "desktop", None, 0));
    let Decided::Make { columns } = signin_decide(&block(), "password", &typed, &keys, None, true, 0) else { panic!("made") };
    assert_eq!(columns.get("email"), Some(&json!("fay@example.com")));
    assert!(columns.get("password_hash").and_then(Value::as_str).is_some_and(|h| h.starts_with("$argon2id$")));
    // With people already, nobody unknown signs in.
    assert_eq!(signin_decide(&block(), "password", &typed, &keys, None, false, 0), Decided::Refused("signinRefused"));
}

#[test]
fn the_calls_take_and_give_json_for_the_other_stacks() {
    let block = json!({"strategies": [{"name": "password", "columns": ["email", "password_hash"]}], "tries": 5, "triesSeconds": 900});
    let begun: Value = serde_json::from_str(&clarity_auth::call("signinBegin", &json!({"block": block, "strategy": "password", "inputs": {"name": "gus@example.com", "password": "right"}, "target": "desktop"}).to_string()).unwrap()).unwrap();
    assert_eq!(begun["column"], "email");
    assert_eq!(begun["secret"], "password_hash");
    assert_eq!(begun["equals"], "gus@example.com");
    let decided: Value = serde_json::from_str(&clarity_auth::call("signinDecide", &json!({"block": block, "strategy": "password", "inputs": {"name": "gus@example.com", "password": "right"}, "keys": begun["keys"], "found": null, "noPeople": true}).to_string()).unwrap()).unwrap();
    assert_eq!(decided["make"]["email"], "gus@example.com");
    let refused: Value = serde_json::from_str(&clarity_auth::call("signinBegin", &json!({"block": block, "strategy": "token", "inputs": {}, "target": "desktop"}).to_string()).unwrap()).unwrap();
    assert_eq!(refused["refused"], "signinRefused");
}
