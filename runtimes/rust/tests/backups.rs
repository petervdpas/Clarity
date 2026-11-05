//! A backup before migration steps change a database (docs/BACKUP-PLAN.md),
//! named and pruned as every stack does it (conformance/backups.json).

use clarity_runtime::backup;
use clarity_runtime::Db;

fn cases() -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/backups.json")).unwrap()).unwrap()
}

#[test]
fn backups_are_named_and_pruned_as_every_stack_does() {
    let c = cases();
    for s in c["stamps"].as_array().unwrap() {
        assert_eq!(s["stamp"], backup::stamp(s["seconds"].as_i64().unwrap()));
    }
    for n in c["names"].as_array().unwrap() {
        assert_eq!(n["file"], backup::file_name(n["database"].as_str().unwrap(), n["seconds"].as_i64().unwrap(), n["step"].as_u64().unwrap() as usize));
    }
    assert_eq!(c["kept"].as_u64().unwrap() as usize, backup::KEPT_AUTOMATIC);
    let dir = std::env::temp_dir().join(format!("clarity-runtime-prune-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("backups")).unwrap();
    let prune = &c["prune"];
    for f in prune["files"].as_array().unwrap() {
        std::fs::write(dir.join("backups").join(f.as_str().unwrap()), "").unwrap();
    }
    backup::prune(&dir.join(format!("{}.db", prune["database"].as_str().unwrap())));
    let mut left: Vec<String> = std::fs::read_dir(dir.join("backups")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    left.sort();
    let wanted: Vec<String> = prune["left"].as_array().unwrap().iter().map(|f| f.as_str().unwrap().to_string()).collect();
    assert_eq!(wanted, left);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_database_with_a_schema_is_backed_up_before_new_steps_and_a_new_one_is_not() {
    let dir = std::env::temp_dir().join(format!("clarity-runtime-backup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("shop.db");
    let first: &[&str] = &["CREATE TABLE item (id INTEGER PRIMARY KEY, name TEXT NOT NULL DEFAULT '')"];
    let second: &[&str] = &["ALTER TABLE item ADD COLUMN price INTEGER NOT NULL DEFAULT 0"];
    {
        let mut db = Db::open(&path).unwrap();
        db.migrate(&[first]).unwrap();
        db.batch("", "INSERT INTO item (name) VALUES ('kept')").unwrap();
    }
    assert!(!dir.join("backups").exists(), "a new database needs no backup");
    {
        let mut db = Db::open(&path).unwrap();
        assert_eq!(1, db.migrate(&[first, second]).unwrap());
    }
    let backups: Vec<_> = std::fs::read_dir(dir.join("backups")).unwrap().flatten().map(|e| e.path()).collect();
    assert_eq!(1, backups.len());
    let name = backups[0].file_name().unwrap().to_string_lossy().to_string();
    assert!(name.starts_with("shop-") && name.ends_with("-before-migration-0002.db"), "{name}");
    // The backup holds the data and the schema as they were before step 2.
    let mut copy = Db::open(&backups[0]).unwrap();
    assert_eq!(1, copy.schema_version().unwrap());
    assert!(copy.batch("", "SELECT price FROM item").is_err(), "step 2 is not in the backup");
    // Nothing to apply: no backup.
    drop(copy);
    Db::open(&path).unwrap().migrate(&[first, second]).unwrap();
    assert_eq!(1, std::fs::read_dir(dir.join("backups")).unwrap().count());
    let _ = std::fs::remove_dir_all(&dir);
}
