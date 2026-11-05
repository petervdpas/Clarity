//! Backups of a project's SQLite databases (docs/BACKUP-PLAN.md): a copy
//! beside the database, named for when and why, the automatic ones pruned
//! to the newest ten, and a restore that first backs up what it replaces.

use std::path::{Path, PathBuf};

use clarity_db::backup::{self, KEPT_AUTOMATIC};
use rusqlite::Connection;

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-backup-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    dir
}

/// A database with one table holding `rows` rows.
fn database(dir: &Path, rows: i64) -> PathBuf {
    let path = dir.join("data/shop.db");
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE IF NOT EXISTS item (id INTEGER PRIMARY KEY, name TEXT); DELETE FROM item;").unwrap();
    for i in 0..rows {
        db.execute("INSERT INTO item (name) VALUES (?1)", [format!("item {i}")]).unwrap();
    }
    path
}

fn rows(path: &Path) -> i64 {
    Connection::open(path).unwrap().query_row("SELECT count(*) FROM item", [], |r| r.get(0)).unwrap()
}

#[test]
fn a_backup_is_a_copy_beside_the_database_named_for_when_and_why() {
    let dir = folder("take");
    let db = database(&dir, 3);
    let taken = backup::take(&db, "manual").unwrap();
    assert_eq!(taken.path.parent().unwrap(), dir.join("data/backups"));
    assert!(taken.file.starts_with("shop-") && taken.file.ends_with("-manual.db"), "{}", taken.file);
    assert_eq!(taken.why, "manual");
    assert_eq!(taken.when.len(), "20261105T213012Z".len(), "{}", taken.when);
    assert_eq!(3, rows(&taken.path), "the copy holds the data");
    // Taken again in the same second, it gets a name of its own.
    let again = backup::take(&db, "manual").unwrap();
    assert_ne!(taken.path, again.path);
    assert_eq!(2, backup::list(&db).len());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn there_is_nothing_to_back_up_before_the_database_exists() {
    let dir = folder("missing");
    let error = backup::take(&dir.join("data/none.db"), "manual").unwrap_err();
    assert!(error.contains("no database"), "{error}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_automatic_backups_are_pruned_to_the_newest_and_the_manual_ones_stay() {
    let dir = folder("prune");
    let db = database(&dir, 1);
    backup::take(&db, "manual").unwrap();
    for step in 1..=KEPT_AUTOMATIC + 3 {
        backup::take(&db, &format!("before-migration-{step:04}")).unwrap();
    }
    let all = backup::list(&db);
    assert_eq!(1, all.iter().filter(|b| b.why == "manual").count());
    assert_eq!(KEPT_AUTOMATIC, all.iter().filter(|b| b.why != "manual").count());
    // The newest stay: the first three steps' went.
    assert!(!all.iter().any(|b| b.why == "before-migration-0001"));
    assert!(all.iter().any(|b| b.why == format!("before-migration-{:04}", KEPT_AUTOMATIC + 3)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_restore_backs_up_what_it_replaces_first() {
    let dir = folder("restore");
    let db = database(&dir, 5);
    let five = backup::take(&db, "manual").unwrap();
    database(&dir, 2);
    assert_eq!(2, rows(&db));
    let before = backup::restore(&db, &five.path).unwrap();
    assert_eq!(5, rows(&db), "the backup's data is back");
    assert_eq!(before.why, "before-restore");
    assert_eq!(2, rows(&before.path), "what it replaced is kept");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_restore_takes_only_a_backup_of_that_database() {
    let dir = folder("foreign");
    let db = database(&dir, 1);
    let stray = dir.join("other.db");
    std::fs::copy(&db, &stray).unwrap();
    let error = backup::restore(&db, &stray).unwrap_err();
    assert!(error.contains("not a backup of"), "{error}");
    let not_sqlite = dir.join("data/backups/shop-20261105T000000Z-manual.db");
    std::fs::create_dir_all(not_sqlite.parent().unwrap()).unwrap();
    std::fs::write(&not_sqlite, "no database").unwrap();
    let error = backup::restore(&db, &not_sqlite).unwrap_err();
    assert!(error.contains("not a database"), "{error}");
    assert_eq!(1, rows(&db), "nothing changed");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_restore_waits_while_the_database_is_in_use() {
    let dir = folder("busy");
    let db = database(&dir, 1);
    let one = backup::take(&db, "manual").unwrap();
    let holder = Connection::open(&db).unwrap();
    holder.execute_batch("BEGIN IMMEDIATE; INSERT INTO item (name) VALUES ('x');").unwrap();
    let error = backup::restore(&db, &one.path).unwrap_err();
    assert!(error.contains("in use"), "{error}");
    holder.execute_batch("ROLLBACK").unwrap();
    drop(holder);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_backup_is_deleted_only_from_its_own_folder() {
    let dir = folder("delete");
    let db = database(&dir, 1);
    let one = backup::take(&db, "manual").unwrap();
    backup::delete(&db, &one.path).unwrap();
    assert!(backup::list(&db).is_empty());
    assert!(backup::delete(&db, &db).is_err(), "the database itself is no backup");
    assert!(db.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_moment_is_named_in_utc() {
    let at = |secs: u64| backup::stamp(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs));
    assert_eq!("19700101T000000Z", at(0));
    assert_eq!("20000229T000000Z", at(951_782_400), "a leap day");
    assert_eq!("20261105T205905Z", at(1_793_912_345));
    assert_eq!("21000228T235959Z", at(4_107_542_399), "2100 is no leap year");
}

/// The names, stamps and pruning every stack shares (conformance/backups.json).
#[test]
fn backups_follow_the_conformance_cases() {
    let cases: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/backups.json")).unwrap()).unwrap();
    for c in cases["stamps"].as_array().unwrap() {
        let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(c["seconds"].as_u64().unwrap());
        assert_eq!(c["stamp"].as_str().unwrap(), backup::stamp(at));
    }
    assert_eq!(KEPT_AUTOMATIC as u64, cases["kept"].as_u64().unwrap());
    let prune = &cases["prune"];
    let dir = folder("conformance");
    let db = dir.join(format!("data/{}.db", prune["database"].as_str().unwrap()));
    std::fs::write(&db, "").unwrap();
    std::fs::create_dir_all(backup::folder(&db)).unwrap();
    for f in prune["files"].as_array().unwrap() {
        std::fs::write(backup::folder(&db).join(f.as_str().unwrap()), "").unwrap();
    }
    backup::prune_automatic(&db);
    let mut left: Vec<String> = std::fs::read_dir(backup::folder(&db)).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    left.sort();
    let wanted: Vec<String> = prune["left"].as_array().unwrap().iter().map(|f| f.as_str().unwrap().to_string()).collect();
    assert_eq!(wanted, left);
    let _ = std::fs::remove_dir_all(&dir);
}
