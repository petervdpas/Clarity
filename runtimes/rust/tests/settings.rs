//! Where an application keeps its settings, as every runtime finds it
//! (conformance/settings.json, `file`), keeping them, and the folder a file
//! store keeps its files in when a folder setting names it (`folders`).

use clarity_runtime::settings::{file_in, store_folder_in};
use serde_json::{Map, Value, json};

#[test]
fn the_settings_file_is_where_every_runtime_puts_it() {
    let cases: Value = serde_json::from_str(include_str!("../../../conformance/settings.json")).unwrap();
    for c in cases["file"].as_array().unwrap() {
        let env: Map<String, Value> = c["env"].as_object().unwrap().clone();
        let found = file_in(c["os"].as_str().unwrap(), |name| env.get(name).and_then(Value::as_str).map(str::to_string), c["program"].as_str().unwrap());
        assert_eq!(found.as_deref(), c["file"].as_str(), "{c}");
    }
}

#[test]
fn settings_kept_are_read_back() {
    let folder = std::env::temp_dir().join(format!("clarity-settings-{}", std::process::id()));
    // The variable this system keeps its settings under, so the test writes
    // in its own folder and not in the person's: APPDATA on Windows, HOME on
    // macOS, XDG_CONFIG_HOME elsewhere.
    let variable = if cfg!(target_os = "windows") {
        "APPDATA"
    } else if cfg!(target_os = "macos") {
        "HOME"
    } else {
        "XDG_CONFIG_HOME"
    };
    // SAFETY: this test alone sets the variable, before it reads it.
    unsafe { std::env::set_var(variable, &folder) };
    let program = "clarity-settings-test";
    assert!(clarity_runtime::settings::read(program).is_empty(), "nothing kept yet");
    let values = json!({ "pageSize": 20, "language": "nl" }).as_object().unwrap().clone();
    clarity_runtime::settings::keep(program, &values).unwrap();
    assert_eq!(clarity_runtime::settings::read(program), values);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_folder_setting_is_where_every_runtime_puts_a_store() {
    let cases: Value = serde_json::from_str(include_str!("../../../conformance/settings.json")).unwrap();
    for c in cases["folders"].as_array().unwrap() {
        let found = store_folder_in(c["os"].as_str().unwrap(), c["settingsFolder"].as_str().unwrap(), c["store"].as_str().unwrap(), &c["kept"], c["default"].as_str().unwrap());
        assert_eq!(found, c["folder"].as_str().unwrap(), "{c}");
    }
}
