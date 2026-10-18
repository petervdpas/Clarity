//! The files of a file store (X2), against the cases every runtime shares
//! (conformance/files.json): listed, read and written as text, and a file as
//! a record's picture.

use std::path::{Path, PathBuf};

use clarity_runtime::{Db, Error, Picture, sql};
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/files.json");

fn cases() -> Value {
    serde_json::from_str(CASES).expect("files.json is JSON")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-files-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The tree of the cases, with their times.
fn plant(root: &Path) {
    for file in cases()["tree"].as_array().unwrap() {
        let path = root.join(file["path"].as_str().unwrap());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, file["text"].as_str().unwrap()).unwrap();
        let at = clarity_runtime::NaiveDateTime::parse_from_str(file["modified"].as_str().unwrap(), "%Y-%m-%dT%H:%M:%S").unwrap().and_utc();
        std::fs::File::options().write(true).open(&path).unwrap().set_modified(at.into()).unwrap();
    }
}

fn store(name: &str) -> (Db, PathBuf) {
    let root = scratch(name);
    plant(&root);
    let mut db = Db::memory().unwrap();
    db.add_store("photos", &root.display().to_string()).unwrap();
    (db, root)
}

#[test]
fn a_store_lists_its_files_as_every_runtime_does() {
    let (db, root) = store("list");
    let listed: Vec<Value> = db
        .files("photos")
        .unwrap()
        .into_iter()
        .map(|f| {
            serde_json::json!({
                "path": f.path,
                "name": f.name,
                "extension": f.extension,
                "size": f.size,
                "modified": f.modified.map(|m| m.format("%Y-%m-%dT%H:%M:%S").to_string()),
            })
        })
        .collect();
    assert_eq!(Value::Array(listed), cases()["listing"]);
    assert!(db.files("films").is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn text_is_read_and_written_inside_the_store() {
    let (db, root) = store("text");
    let notes = db.files("photos").unwrap().into_iter().find(|f| f.name == "notes.txt").unwrap();
    assert_eq!(db.read_text(&notes).unwrap(), "Hello\nworld\n");
    db.write_text("photos", "exports/titles.txt", "Blue Train\n").unwrap();
    assert_eq!(std::fs::read_to_string(root.join("exports/titles.txt")).unwrap(), "Blue Train\n");
    for path in cases()["refused"].as_array().unwrap() {
        let path = path.as_str().unwrap();
        assert!(matches!(db.write_text("photos", path, "x"), Err(Error::Invalid(_))), "{path}");
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_file_becomes_a_picture_linked_in_its_own_store() {
    let root = scratch("picture");
    std::fs::create_dir_all(root.join("2024")).unwrap();
    let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    std::fs::write(root.join("2024/cover.png"), png).unwrap();
    std::fs::write(root.join("notes.txt"), "text").unwrap();
    let mut db = Db::memory().unwrap();
    db.add_store("photos", &root.display().to_string()).unwrap();
    let files = db.files("photos").unwrap();
    let cover = files.iter().find(|f| f.name == "cover.png").unwrap();
    // In the field's own store: the file itself, by its path.
    assert_eq!(db.picture_from(cover, Some("photos")).unwrap(), Picture::InStore("2024/cover.png".into()));
    assert_eq!(sql::picture(&Some(Picture::InStore("2024/cover.png".into()))), clarity_runtime::SqlValue::Text("2024/cover.png".into()));
    // Elsewhere: a copy of its bytes.
    assert_eq!(db.picture_from(cover, None).unwrap(), Picture::New(png.to_vec()));
    let notes = files.iter().find(|f| f.name == "notes.txt").unwrap();
    assert!(matches!(db.picture_from(notes, None), Err(Error::Invalid(_))));
    // Before its record is saved, the screen shows it from the store.
    assert_eq!(db.store_file("photos", "2024/cover.png").unwrap(), png.to_vec());
    assert!(matches!(db.store_file("photos", "../x.png"), Err(Error::Invalid(_))));
    // On the wire, a linked file is its path.
    assert_eq!(serde_json::to_value(Picture::InStore("2024/cover.png".into())).unwrap(), serde_json::json!({ "file": "2024/cover.png" }));
    std::fs::remove_dir_all(root).unwrap();
}
