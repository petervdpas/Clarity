//! `clarity data migrate`: a project's SQLite database created and brought up
//! to date as its program does when it starts, without opening a window
//! (a gap Writer met: seeding data needed the program started under Xvfb).

use std::process::Command;

#[test]
fn data_migrate_creates_the_database_then_finds_it_up_to_date() {
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let root = std::env::temp_dir().join(format!("clarity-migrate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("todo")).unwrap();
    std::fs::copy(examples.join("todo.clarity"), root.join("todo.clarity")).unwrap();
    for entry in walk(&examples.join("todo")) {
        let relative = entry.strip_prefix(examples.join("todo")).unwrap();
        if relative.starts_with("data") || relative.starts_with("out") {
            continue;
        }
        let to = root.join("todo").join(relative);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(&entry, to).unwrap();
    }
    let clarity = env!("CARGO_BIN_EXE_clarity");
    let first = Command::new(clarity).args(["data", "migrate"]).arg(root.join("todo.clarity")).output().unwrap();
    let said = String::from_utf8_lossy(&first.stderr);
    assert!(first.status.success() && said.contains("Migrated main by"), "{said}");
    assert!(root.join("todo/data/todo.db").is_file());
    let again = Command::new(clarity).args(["data", "migrate"]).arg(root.join("todo.clarity")).output().unwrap();
    assert!(String::from_utf8_lossy(&again.stderr).contains("is up to date"));
    let _ = std::fs::remove_dir_all(&root);
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else {
            found.push(path);
        }
    }
    found
}
