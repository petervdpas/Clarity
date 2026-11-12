//! Clarity.Auth: the sign-in strategies of the programs Clarity generates
//! (docs/APP-CAPABILITIES-PLAN.md step 4), alike on every stack. A runtime
//! finds the person by what `find` says, then asks `check`; it never knows
//! one strategy from another.

use serde_json::{Value, json};

fn call(op: &str, args: Value) -> Result<Value, clarity_auth::Problem> {
    clarity_auth::call(op, &args.to_string()).map(|t| serde_json::from_str(&t).unwrap())
}

#[test]
fn the_catalog_names_each_strategy_its_fields_inputs_and_targets() {
    let catalog = call("catalog", json!({})).unwrap();
    let names: Vec<&str> = catalog.as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["password", "account", "token", "recovery", "github", "microsoft", "openid"]);
    assert_eq!(catalog[4]["provider"], json!(true));
    assert_eq!(catalog[0]["provider"], json!(false));
    let password = &catalog[0];
    assert_eq!(password["inputs"], json!(["name", "password"]));
    assert_eq!(password["fields"], json!([{"role": "match", "kinds": ["text", "email"]}, {"role": "secret", "kinds": ["text"], "hash": "argon2id"}]));
    assert_eq!(catalog[1]["targets"], json!(["desktop", "cli"]));
    assert_eq!(catalog[2]["targets"], json!(["desktop", "web", "cli"]));
    assert_eq!(catalog[2]["fields"][0]["hash"], "sha256");
    assert_eq!(catalog[2]["firstAccount"], false);
}

#[test]
fn a_password_is_kept_as_a_hash_and_checked_against_it() {
    let stored = call("hash", json!({"method": "argon2id", "text": "correct horse"})).unwrap();
    let stored = stored.as_str().unwrap().to_string();
    assert!(stored.starts_with("$argon2id$") && !stored.contains("correct horse"), "{stored}");
    let find = call("find", json!({"strategy": "password", "inputs": {"name": "ann@example.com", "password": "correct horse"}})).unwrap();
    assert_eq!(find, json!({"role": "match", "equals": "ann@example.com"}));
    let right = call("check", json!({"strategy": "password", "inputs": {"name": "ann@example.com", "password": "correct horse"}, "stored": {"secret": stored}})).unwrap();
    let wrong = call("check", json!({"strategy": "password", "inputs": {"name": "ann@example.com", "password": "wrong"}, "stored": {"secret": stored}})).unwrap();
    assert_eq!((right, wrong), (json!(true), json!(false)));
    // A record whose hash was never set signs nobody in.
    assert_eq!(call("check", json!({"strategy": "password", "inputs": {"name": "a", "password": ""}, "stored": {"secret": ""}})).unwrap(), json!(false));
}

#[test]
fn a_token_is_found_by_its_hash_so_no_record_is_tried_one_by_one() {
    let token = call("newToken", json!({})).unwrap();
    let token = token.as_str().unwrap();
    assert!(token.len() >= 40, "{token}");
    let stored = call("hash", json!({"method": "sha256", "text": token})).unwrap();
    let find = call("find", json!({"strategy": "token", "inputs": {"token": token}})).unwrap();
    assert_eq!(find, json!({"role": "secret", "equals": stored}));
    assert_eq!(call("check", json!({"strategy": "token", "inputs": {"token": token}, "stored": {"secret": stored}})).unwrap(), json!(true));
    assert_eq!(call("check", json!({"strategy": "token", "inputs": {"token": "other"}, "stored": {"secret": stored}})).unwrap(), json!(false));
}

#[test]
fn the_account_strategy_matches_the_computer_s_own_user() {
    let find = call("find", json!({"strategy": "account", "inputs": {}})).unwrap();
    assert_eq!(find["role"], "match");
    assert!(!find["equals"].as_str().unwrap().is_empty());
    assert_eq!(call("check", json!({"strategy": "account", "inputs": {}, "stored": {}})).unwrap(), json!(true));
}

#[test]
fn failed_tries_slow_down_per_key_within_the_window() {
    let key = json!({"key": "account:bob", "tries": 2, "window": 60, "now": 1000});
    assert_eq!(call("triesAllowed", key.clone()).unwrap(), json!(true));
    call("triesFailed", json!({"key": "account:bob", "now": 1000})).unwrap();
    call("triesFailed", json!({"key": "account:bob", "now": 1010})).unwrap();
    assert_eq!(call("triesAllowed", json!({"key": "account:bob", "tries": 2, "window": 60, "now": 1020})).unwrap(), json!(false));
    assert_eq!(call("triesAllowed", json!({"key": "account:other", "tries": 2, "window": 60, "now": 1020})).unwrap(), json!(true));
    // The first failure falls out of the window.
    assert_eq!(call("triesAllowed", json!({"key": "account:bob", "tries": 2, "window": 60, "now": 1061})).unwrap(), json!(true));
    call("triesCleared", json!({"key": "account:bob"})).unwrap();
    assert_eq!(call("triesAllowed", json!({"key": "account:bob", "tries": 2, "window": 60, "now": 1020})).unwrap(), json!(true));
}

#[test]
fn a_session_names_its_record_until_it_expires_or_is_closed() {
    let token = call("sessionOpen", json!({"record": 7, "seconds": 3600, "now": 5000})).unwrap();
    let token = token.as_str().unwrap().to_string();
    assert_eq!(call("sessionFind", json!({"token": token, "now": 5100})).unwrap(), json!(7));
    assert_eq!(call("sessionFind", json!({"token": token, "now": 8601})).unwrap(), json!(0));
    let other = call("sessionOpen", json!({"record": 7, "seconds": 3600, "now": 5000})).unwrap();
    let third = call("sessionOpen", json!({"record": 8, "seconds": 3600, "now": 5000})).unwrap();
    call("sessionClose", json!({"token": other})).unwrap();
    assert_eq!(call("sessionFind", json!({"token": other, "now": 5001})).unwrap(), json!(0));
    call("sessionCloseAll", json!({"record": 8})).unwrap();
    assert_eq!(call("sessionFind", json!({"token": third, "now": 5001})).unwrap(), json!(0));
    assert_eq!(call("sessionFind", json!({"token": "made up", "now": 5001})).unwrap(), json!(0));
}

#[test]
fn what_it_does_not_know_it_says() {
    let unknown = call("find", json!({"strategy": "carrier pigeon", "inputs": {}})).unwrap_err();
    assert_eq!((unknown.kind, unknown.text.as_str()), (clarity_auth::ProblemKind::UnknownStrategy, "there is no sign-in strategy 'carrier pigeon'; there are password, account, token, recovery, github, microsoft, openid"));
    let missing = call("find", json!({"strategy": "password", "inputs": {"name": "ann"}})).unwrap_err();
    assert_eq!((missing.kind, missing.text.as_str()), (clarity_auth::ProblemKind::BadInput, "the password strategy needs the input 'password'"));
    assert_eq!(call("shout", json!({})).unwrap_err().kind, clarity_auth::ProblemKind::Failed);
    assert_eq!(call("hash", json!({"method": "md5", "text": "x"})).unwrap_err().kind, clarity_auth::ProblemKind::BadInput);
}

#[test]
fn the_first_account_is_made_of_the_inputs_its_fields_by_role() {
    let made = call("firstAccount", json!({"strategy": "password", "inputs": {"name": "ann", "password": "correct horse"}})).unwrap();
    assert_eq!(made["match"], "ann");
    let secret = made["secret"].as_str().unwrap();
    assert!(secret.starts_with("$argon2id$"), "{secret}");
    let checked = call("check", json!({"strategy": "password", "inputs": {"name": "ann", "password": "correct horse"}, "stored": {"secret": secret}})).unwrap();
    assert_eq!(checked, json!(true));
    let account = call("firstAccount", json!({"strategy": "account", "inputs": {}})).unwrap();
    assert!(!account["match"].as_str().unwrap().is_empty() && account.get("secret").is_none(), "{account}");
    // A strategy that cannot make the first account says so.
    assert_eq!(call("firstAccount", json!({"strategy": "token", "inputs": {"token": "x"}})).unwrap_err().kind, clarity_auth::ProblemKind::BadInput);
}

#[test]
fn a_recovery_code_signs_in_once_and_is_used_up() {
    let codes = call("newRecoveryCodes", json!({"count": 3})).unwrap();
    let codes: Vec<String> = codes.as_str().unwrap().lines().map(str::to_string).collect();
    assert_eq!(codes.len(), 3);
    assert!(codes.iter().all(|c| c.len() >= 10), "{codes:?}");
    let stored = call("hash", json!({"method": "sha256-lines", "text": codes.join("\n")})).unwrap();
    let stored = stored.as_str().unwrap().to_string();
    assert!(!codes.iter().any(|c| stored.contains(c.as_str())), "{stored}");
    let catalog = call("catalog", json!({})).unwrap();
    assert_eq!(catalog[3]["inputs"], json!(["name", "code"]));
    assert_eq!(catalog[3]["fields"][1]["hash"], "sha256-lines");
    let inputs = json!({"name": "ann", "code": codes[1]});
    assert_eq!(call("find", json!({"strategy": "recovery", "inputs": inputs})).unwrap(), json!({"role": "match", "equals": "ann"}));
    assert_eq!(call("check", json!({"strategy": "recovery", "inputs": inputs, "stored": {"secret": stored}})).unwrap(), json!(true));
    // After it signs in, the code is gone from what is stored.
    let after = call("afterSignin", json!({"strategy": "recovery", "inputs": inputs, "stored": {"secret": stored}})).unwrap();
    let left = after["secret"].as_str().unwrap().to_string();
    assert_eq!(call("check", json!({"strategy": "recovery", "inputs": inputs, "stored": {"secret": left}})).unwrap(), json!(false));
    let other = json!({"name": "ann", "code": codes[0]});
    assert_eq!(call("check", json!({"strategy": "recovery", "inputs": other, "stored": {"secret": left}})).unwrap(), json!(true));
    // A strategy that keeps what it stores as it is gives nothing to write.
    assert_eq!(call("afterSignin", json!({"strategy": "password", "inputs": {"name": "ann", "password": "x"}, "stored": {"secret": "s"}})).unwrap(), json!({}));
}

#[test]
fn a_secret_shorter_than_the_minimum_does_not_fit() {
    assert_eq!(call("secretFits", json!({"text": "short", "minimum": 12})).unwrap(), json!(false));
    assert_eq!(call("secretFits", json!({"text": "twelve chars", "minimum": 12})).unwrap(), json!(true));
    // Characters are counted, not bytes.
    assert_eq!(call("secretFits", json!({"text": "ééééé", "minimum": 5})).unwrap(), json!(true));
}

#[test]
fn a_changed_password_closes_the_person_s_other_sessions() {
    let kept = call("sessionOpen", json!({"record": 21, "seconds": 3600, "now": 5000})).unwrap();
    let other = call("sessionOpen", json!({"record": 21, "seconds": 3600, "now": 5000})).unwrap();
    let someone = call("sessionOpen", json!({"record": 22, "seconds": 3600, "now": 5000})).unwrap();
    call("sessionCloseOthers", json!({"record": 21, "token": kept})).unwrap();
    assert_eq!(call("sessionFind", json!({"token": kept, "now": 5001})).unwrap(), json!(21));
    assert_eq!(call("sessionFind", json!({"token": other, "now": 5001})).unwrap(), json!(0));
    assert_eq!(call("sessionFind", json!({"token": someone, "now": 5001})).unwrap(), json!(22));
}

#[test]
fn a_remembered_sign_in_holds_until_it_expires_or_the_secret_changes() {
    let stored = call("hash", json!({"method": "argon2id", "text": "correct horse"})).unwrap();
    let entry = call("rememberMake", json!({"record": 7, "seconds": 86400, "stored": stored, "now": 1000})).unwrap();
    let entry = entry.as_str().unwrap().to_string();
    assert!(!entry.contains(stored.as_str().unwrap()), "{entry}");
    // The entry names its record, so the runtime knows whose secret to read.
    assert_eq!(call("rememberRecord", json!({"entry": entry})).unwrap(), json!(7));
    assert_eq!(call("rememberCheck", json!({"entry": entry, "stored": stored, "now": 2000})).unwrap(), json!(7));
    assert_eq!(call("rememberCheck", json!({"entry": entry, "stored": stored, "now": 87401})).unwrap(), json!(0));
    let changed = call("hash", json!({"method": "argon2id", "text": "new horse"})).unwrap();
    assert_eq!(call("rememberCheck", json!({"entry": entry, "stored": changed, "now": 2000})).unwrap(), json!(0));
    // An entry edited to name someone else does not hold.
    let forged = entry.replacen("7", "8", 1);
    assert_eq!(call("rememberCheck", json!({"entry": forged, "stored": stored, "now": 2000})).unwrap(), json!(0));
    assert_eq!(call("rememberCheck", json!({"entry": "rubbish", "stored": stored, "now": 2000})).unwrap(), json!(0));
    assert_eq!(call("rememberRecord", json!({"entry": "rubbish"})).unwrap(), json!(0));
    // Nothing is remembered without a secret to sign it with.
    assert_eq!(call("rememberMake", json!({"record": 7, "seconds": 60, "stored": "", "now": 1})).unwrap_err().kind, clarity_auth::ProblemKind::BadInput);
}
