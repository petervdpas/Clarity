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
    // Windows keeps settings under APPDATA, the others under XDG_CONFIG_HOME.
    let was = std::env::var_os("APPDATA");
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &home);
        std::env::set_var("APPDATA", &home);
    }
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
    // SAFETY: as above.
    unsafe {
        match was {
            Some(was) => std::env::set_var("APPDATA", was),
            None => std::env::remove_var("APPDATA"),
        }
    }
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

/// On Windows a secret is kept as every stack keeps it
/// (conformance/secret-store.json): target, user and UTF-8 blob.
#[test]
fn the_credential_manager_shape_is_every_stack_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../conformance/secret-store.json")).unwrap();
    for case in cases["windows"].as_array().unwrap() {
        let text = |name: &str| case[name].as_str().unwrap().to_string();
        assert_eq!(secrets::windows_target(&text("program"), &text("name")), text("target"));
        let blob = secrets::windows_blob(&text("value"));
        let hex: String = blob.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, text("blob"));
        assert_eq!(secrets::from_windows_blob(&blob).as_deref(), Some(text("value").as_str()));
        assert_eq!(secrets::WINDOWS_PERSIST, text("persist"));
    }
}

/// On Windows a kept secret is a generic credential for this machine, its
/// user the setting's name and its blob the UTF-8 bytes, as every stack keeps
/// it (conformance/secret-store.json); not enterprise persistence, which
/// roams with the person's account.
#[cfg(windows)]
#[test]
fn the_credential_manager_keeps_it_for_this_machine() {
    use windows_sys::Win32::Security::Credentials::{CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredFree, CredReadW};
    if !store_works() {
        return;
    }
    secrets::set(PROGRAM, "kept", "Password=pä€").unwrap();
    let target: Vec<u16> = secrets::windows_target(PROGRAM, "kept").encode_utf16().chain([0]).collect();
    let mut found: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: a NUL-terminated target; the credential Windows gives is read, then freed.
    let (persist, user, blob) = unsafe {
        assert_ne!(CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut found), 0, "the credential is kept");
        let c = &*found;
        let user_len = (0..).take_while(|&i| *c.UserName.add(i) != 0).count();
        let user = String::from_utf16_lossy(std::slice::from_raw_parts(c.UserName, user_len));
        let blob = std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize).to_vec();
        let persist = c.Persist;
        CredFree(found as *const _);
        (persist, user, blob)
    };
    secrets::set(PROGRAM, "kept", "").unwrap();
    assert_eq!(persist, CRED_PERSIST_LOCAL_MACHINE);
    assert_eq!(user, "kept");
    assert_eq!(blob, "Password=pä€".as_bytes());
    assert_eq!(secrets::get(PROGRAM, "kept"), None, "forgotten");
}
