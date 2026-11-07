//! CLARITY_DATA moves every SQLite database of a program into a folder of its
//! own, under its own file name: a run in the IDE or by hand (a live check on
//! a copy) chooses its data without editing the generated program. The same
//! as the C# runtime's `DataFolder`.

use std::path::Path;

use clarity_runtime::data_folder;
use clarity_runtime::{Db, Dialect};

fn env(folder: Option<&str>) -> impl Fn(&str) -> Option<String> {
    let folder = folder.map(String::from);
    move |name| if name == data_folder::VARIABLE { folder.clone() } else { None }
}

#[test]
fn clarity_data_moves_an_sqlite_file_into_its_folder() {
    let folder = std::env::temp_dir().join("copy");
    let at = "/home/someone/noughts/data/noughts.db";
    assert_eq!(data_folder::sqlite(at, env(Some(&folder.display().to_string()))), folder.join("noughts.db").display().to_string());
    // Unset or empty, the file is where the project put it.
    assert_eq!(data_folder::sqlite(at, env(None)), at);
    assert_eq!(data_folder::sqlite(at, env(Some(""))), at);
    // A location from the environment or a secret setting is as it says.
    assert_eq!(data_folder::sqlite("env:SHOP_DB", env(Some("/copy"))), "env:SHOP_DB");
    assert_eq!(data_folder::sqlite("setting:shop/db", env(Some("/copy"))), "setting:shop/db");
    // SQLite's database in memory has no file to move.
    assert_eq!(data_folder::sqlite(":memory:", env(Some("/copy"))), ":memory:");
}

/// The process's variable, read when a database opens. The only test in this
/// file that sets it, so no other test sees it.
#[test]
fn a_database_opens_in_the_folder_clarity_data_names() {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("clarity-data");
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    // SAFETY: no other thread of this test binary reads or writes the environment.
    unsafe { std::env::set_var(data_folder::VARIABLE, &folder) };
    let elsewhere = folder.join("elsewhere").join("shop.db");
    let opened = Db::connect(&[("main", Dialect::Sqlite, elsewhere.to_str().unwrap())]);
    unsafe { std::env::remove_var(data_folder::VARIABLE) };
    drop(opened.unwrap());
    assert!(folder.join("shop.db").exists());
    assert!(!elsewhere.exists());
    std::fs::remove_dir_all(&folder).unwrap();
}

/// Every runtime places an SQLite database alike (conformance/data-folder.json).
#[test]
fn a_database_opens_where_every_runtime_opens_it() {
    let cases: serde_json::Value = serde_json::from_str(include_str!("../../../conformance/data-folder.json")).unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let env = case["env"].as_object().unwrap().clone();
        let got = clarity_runtime::data_folder::sqlite_in(case["os"].as_str().unwrap(), case["location"].as_str().unwrap(), |name| env.get(name).and_then(|v| v.as_str()).map(str::to_string));
        assert_eq!(case["file"].as_str().unwrap(), got.replace('\\', "/"), "{case}");
    }
}
