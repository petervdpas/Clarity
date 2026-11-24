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
    for (at, n) in c["next"].as_array().unwrap().iter().enumerate() {
        let dir = std::env::temp_dir().join(format!("clarity-runtime-next-{}-{at}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for f in n["files"].as_array().unwrap() {
            std::fs::write(dir.join(f.as_str().unwrap()), "").unwrap();
        }
        assert_eq!(n["file"], backup::next_file(&dir, n["database"].as_str().unwrap(), n["seconds"].as_i64().unwrap(), n["why"].as_str().unwrap()), "case {at}");
        let _ = std::fs::remove_dir_all(&dir);
    }
    let dir = std::env::temp_dir().join(format!("clarity-runtime-prune-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("backups")).unwrap();
    let prune = &c["prune"];
    for f in prune["files"].as_array().unwrap() {
        std::fs::write(dir.join("backups").join(f.as_str().unwrap()), "").unwrap();
    }
    backup::prune(&dir.join("backups"), prune["database"].as_str().unwrap());
    let mut left: Vec<String> = std::fs::read_dir(dir.join("backups")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    left.sort();
    let wanted: Vec<String> = prune["left"].as_array().unwrap().iter().map(|f| f.as_str().unwrap().to_string()).collect();
    assert_eq!(wanted, left);
    let _ = std::fs::remove_dir_all(&dir);
}


/// A backup's file read back: its moment, its count within that second and
/// its reason; a file that is not one of this database's backups is none.
/// The one reading of backup names, for clarity-migrate and for Clarity's
/// Backups (clarity_db).
#[test]
fn a_backup_s_file_reads_back_as_when_count_and_why() {
    let read = |file: &str| backup::parse("shop", file);
    assert_eq!(read("shop-20261105T213012Z-manual.db"), Some(backup::Named { when: "20261105T213012Z".into(), count: 1, why: "manual".into() }));
    assert_eq!(read("shop-20261105T213012Z-before-migration-0006_2.db"), Some(backup::Named { when: "20261105T213012Z".into(), count: 2, why: "before-migration-0006".into() }));
    for not in ["other-20261105T213012Z-manual.db", "shop-2026110XT213012Z-manual.db", "shop-20261105T213012Z-Manual.db", "shop-20261105T213012Z-manual.txt", "shop-20261105T213012Z-manual_x.db"] {
        assert_eq!(read(not), None, "{not}");
    }
}


/// Backups of one second, each for another reason (the steps of a quick
/// migration), keep the order they were taken in: pruning removes the first
/// taken, never the newest, whatever order the folder lists them in.
#[test]
fn backups_of_one_second_are_pruned_oldest_first_whatever_their_reasons() {
    let dir = std::env::temp_dir().join(format!("clarity-runtime-one-second-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let taken: Vec<String> = (1..=backup::KEPT_AUTOMATIC + 3)
        .map(|step| {
            let file = backup::next_file(&dir, "shop", 1_793_912_345, &format!("before-migration-{step:04}"));
            std::fs::write(dir.join(&file), "").unwrap();
            backup::prune(&dir, "shop");
            assert!(dir.join(&file).exists(), "{file} was pruned as it was taken");
            file
        })
        .collect();
    let mut left: Vec<String> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    left.sort();
    let mut wanted = taken[3..].to_vec();
    wanted.sort();
    assert_eq!(wanted, left);
    let _ = std::fs::remove_dir_all(&dir);
}
