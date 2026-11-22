//! `clarity data migrate`: a project's SQLite database created and brought up
//! to date as its program does when it starts, without opening a window
//! (a gap Writer met: seeding data needed the program started under Xvfb).

use std::process::Command;

#[test]
fn data_migrate_creates_the_database_then_finds_it_up_to_date() {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-projects");
    let root = std::env::temp_dir().join(format!("clarity-migrate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("todo")).unwrap();
    std::fs::copy(source.join("todo.clarity"), root.join("todo.clarity")).unwrap();
    for entry in walk(&source.join("todo")) {
        let relative = entry.strip_prefix(source.join("todo")).unwrap();
        if relative.starts_with("data") || relative.starts_with("out") {
            continue;
        }
        let to = root.join("todo").join(relative);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(&entry, to).unwrap();
    }
    let clarity = env!("CARGO_BIN_EXE_clarity");
    // The YAML file source `ideas` is external: it never gets steps.
    let pending = Command::new(clarity).args(["migrate", "--check"]).arg(root.join("todo.clarity")).output().unwrap();
    let listed = String::from_utf8_lossy(&pending.stderr);
    assert!(!listed.contains("data source ideas"), "{listed}");
    let first = Command::new(clarity).args(["data", "migrate"]).arg(root.join("todo.clarity")).output().unwrap();
    let said = String::from_utf8_lossy(&first.stderr);
    assert!(first.status.success() && said.contains("Migrated main by"), "{said}");
    assert!(root.join("todo/data/todo.db").is_file());
    let again = Command::new(clarity).args(["data", "migrate"]).arg(root.join("todo.clarity")).output().unwrap();
    assert!(String::from_utf8_lossy(&again.stderr).contains("is up to date"));
    // A data source the project does not own is said.
    let unknown = Command::new(clarity).args(["data", "migrate", "--source", "nope"]).arg(root.join("todo.clarity")).output().unwrap();
    assert!(!unknown.status.success() && String::from_utf8_lossy(&unknown.stderr).contains("owns no database of data source nope"));
    // A database ahead of the project's steps (a newer version of the app's) is refused, as clarity-migrate refuses it.
    let ahead = Command::new("sqlite3").arg(root.join("todo/data/todo.db")).arg("INSERT INTO clarity_schema (version) VALUES (999)").status().unwrap();
    assert!(ahead.success());
    let refused = Command::new(clarity).args(["data", "migrate"]).arg(root.join("todo.clarity")).output().unwrap();
    let said = String::from_utf8_lossy(&refused.stderr);
    assert!(!refused.status.success() && said.contains("belongs to a newer version of the app"), "{said}");
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
