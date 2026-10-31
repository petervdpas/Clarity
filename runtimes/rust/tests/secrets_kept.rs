//! Keeping the settings writes a secret only when it changed: another
//! setting changing never needs the protected store, nor fails where it
//! cannot be reached (DESIGN 5.27). Its own file: it sets where settings go.
#![cfg(feature = "secrets")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[test]
fn a_secret_goes_to_the_store_only_when_it_changed() {
    let folder = std::env::temp_dir().join(format!("clarity-secrets-kept-{}", std::process::id()));
    let variable = if cfg!(target_os = "windows") {
        "APPDATA"
    } else if cfg!(target_os = "macos") {
        "HOME"
    } else {
        "XDG_CONFIG_HOME"
    };
    // SAFETY: this test alone runs in this process, before anything reads it.
    unsafe { std::env::set_var(variable, &folder) };
    let kept = Arc::new(Mutex::new(HashMap::from([(("app".to_string(), "token".to_string()), "abc".to_string())])));
    let sets = Arc::new(Mutex::new(0));
    let (read, written, count) = (kept.clone(), kept.clone(), sets.clone());
    clarity_runtime::secrets::use_store(
        move |program, name| read.lock().unwrap().get(&(program.to_string(), name.to_string())).cloned(),
        move |program, name, value| {
            *count.lock().unwrap() += 1;
            written.lock().unwrap().insert((program.to_string(), name.to_string()), value.to_string());
            Ok(())
        },
    );
    let mut values = clarity_runtime::settings::read_with("app", &["token"]);
    values.insert("showToolbar".into(), false.into());
    clarity_runtime::settings::keep_with("app", &values, &["token"]).unwrap();
    assert_eq!(*sets.lock().unwrap(), 0, "an unchanged secret was written");
    values.insert("token".into(), "xyz".into());
    clarity_runtime::settings::keep_with("app", &values, &["token"]).unwrap();
    assert_eq!(*sets.lock().unwrap(), 1);
    assert_eq!(kept.lock().unwrap()[&("app".to_string(), "token".to_string())], "xyz");
    // A secret never kept is empty: keeping it empty writes nothing.
    values.insert("other".into(), "".into());
    clarity_runtime::settings::keep_with("app", &values, &["token", "other"]).unwrap();
    assert_eq!(*sets.lock().unwrap(), 1, "an empty secret never kept was written");
    let _ = std::fs::remove_dir_all(&folder);
}
