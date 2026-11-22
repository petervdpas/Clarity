//! A backup before migration steps change a database (docs/ledger/backups.md),
//! named and pruned as every stack does it (conformance/backups.json).

use clarity_runtime::backup;

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

