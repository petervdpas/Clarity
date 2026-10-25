//! Secret settings in the system's protected store (DESIGN 5.27): kept,
//! read back and forgotten, apart from settings.json. Needs a store, such as
//! the Secret Service of a desktop session; without one it says so and passes.
#![cfg(feature = "secrets")]

use clarity_runtime::{secrets, settings};
use serde_json::{Map, Value, json};

const PROGRAM: &str = "clarity-runtime-test";

fn store_works() -> bool {
    let works = secrets::set(PROGRAM, "probe", "1").is_ok() && secrets::get(PROGRAM, "probe").as_deref() == Some("1");
    let _ = secrets::set(PROGRAM, "probe", "");
    if !works {
        eprintln!("no protected store here; the secret tests check nothing");
    }
    works
}

#[test]
fn a_secret_is_kept_read_and_forgotten() {
    if !store_works() {
        return;
    }
    secrets::set(PROGRAM, "token", "s3cr3t").unwrap();
    assert_eq!(secrets::get(PROGRAM, "token").as_deref(), Some("s3cr3t"));
    secrets::set(PROGRAM, "token", "").unwrap();
    assert_eq!(secrets::get(PROGRAM, "token"), None);
    // Forgetting what was never kept is no error.
    secrets::set(PROGRAM, "token", "").unwrap();
}

#[test]
fn settings_keep_their_secrets_out_of_the_file() {
    if !store_works() {
        return;
    }
    let home = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("secret-settings");
    let _ = std::fs::remove_dir_all(&home);
    // SAFETY: this test alone reads the settings folder of this process.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", &home) };
    let program = "clarity-runtime-test-settings";
    let mut values = Map::new();
    values.insert("pageSize".into(), json!(20));
    values.insert("apiToken".into(), json!("t0k3n"));
    settings::keep_with(program, &values, &["apiToken"]).unwrap();
    let file = std::fs::read_to_string(home.join(program).join("settings.json")).unwrap();
    assert!(file.contains("pageSize") && !file.contains("t0k3n") && !file.contains("apiToken"), "{file}");
    let read = settings::read_with(program, &["apiToken"]);
    assert_eq!(read.get("apiToken"), Some(&Value::from("t0k3n")));
    assert_eq!(read.get("pageSize"), Some(&json!(20)));
    secrets::set(program, "apiToken", "").unwrap();
}

/// A database connects with a secret setting (DESIGN 5.27): its connection,
/// password and all, kept in the protected store, not in the program.
#[test]
fn a_database_connects_with_a_secret_setting() {
    if !store_works() {
        return;
    }
    let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("secret-connection.db");
    secrets::set(PROGRAM, "shopDb", file.to_str().unwrap()).unwrap();
    let db = clarity_runtime::Db::connect(&[("main", clarity_runtime::Dialect::Sqlite, &format!("setting:{PROGRAM}/shopDb"))]);
    secrets::set(PROGRAM, "shopDb", "").unwrap();
    assert!(db.is_ok(), "{:?}", db.err());
    let missing = clarity_runtime::Db::connect(&[("main", clarity_runtime::Dialect::Sqlite, &format!("setting:{PROGRAM}/shopDb"))]).err().unwrap();
    assert!(missing.to_string().contains("the secret setting shopDb is not kept yet"), "{missing}");
}
