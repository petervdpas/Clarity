//! How many migration steps a database applied, read without changing it:
//! whether the agent's latest step may still be folded (docs/MCP-PLAN.md).

use clarity_db::applied_steps;

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-db-applied-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn no_file_and_no_table_have_applied_nothing() {
    let dir = folder("none");
    assert_eq!(applied_steps(&dir.join("app.db")), Ok(0));
    let path = dir.join("bare.db");
    rusqlite::Connection::open(&path).unwrap().execute_batch("CREATE TABLE customer (id INTEGER PRIMARY KEY);").unwrap();
    assert_eq!(applied_steps(&path), Ok(0));
}

#[test]
fn the_highest_version_is_the_steps_applied() {
    let path = folder("some").join("app.db");
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch("CREATE TABLE clarity_schema (version INTEGER NOT NULL); INSERT INTO clarity_schema VALUES (1); INSERT INTO clarity_schema VALUES (3);")
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    assert_eq!(applied_steps(&path), Ok(3));
    assert_eq!(std::fs::read(&path).unwrap(), before, "read only");
}
