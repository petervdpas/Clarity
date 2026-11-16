//! DevTools' Disk tab: the folders Clarity's builds and checks fill in its
//! repository, their sizes, and removing one.

use std::path::PathBuf;

use clarity_gen::tools::devtools::disk;

fn temp(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("disk-tests").join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn file(path: PathBuf, bytes: usize) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, vec![0u8; bytes]).unwrap();
}

#[test]
fn every_place_is_a_build_folder_inside_the_repository() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let places = disk::places(&repo);
    let mut keys: Vec<&str> = places.iter().map(|p| p.key).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), places.len(), "each place has its own key");
    for place in &places {
        assert!(!place.cost.is_empty(), "{} says what removing it costs", place.key);
        for path in &place.paths {
            assert!(path.starts_with(&repo) && path != &repo, "{}: {} is inside the repository", place.key, path.display());
            let rel = path.strip_prefix(&repo).unwrap().to_string_lossy().replace('\\', "/");
            assert!(rel.starts_with("target/") || rel.ends_with("/out") || rel.ends_with("/bin") || rel.ends_with("/obj"), "{}: {rel} is a build folder git ignores", place.key);
        }
    }
    assert!(places.iter().any(|p| p.key == "debug") && places.iter().any(|p| p.key == "freshTarget"));
}

#[test]
fn a_place_is_as_big_as_its_files_and_removing_it_takes_only_its_own() {
    let repo = temp("repo");
    file(repo.join("target/tmp/a/one.bin"), 1000);
    file(repo.join("target/tmp/two.bin"), 24);
    file(repo.join("target/debug/kept.bin"), 7);
    file(repo.join("examples/shop/out/app.bin"), 10);
    file(repo.join("examples/todo/out/app.bin"), 5);
    file(repo.join("examples/todo/todo.clarity"), 3);
    let tmp = disk::places(&repo).into_iter().find(|p| p.key == "testScratch").unwrap();
    assert_eq!(disk::size(&tmp), 1024);
    let outs = disk::places(&repo).into_iter().find(|p| p.key == "exampleOuts").unwrap();
    assert_eq!(outs.paths.len(), 2, "every example's out/");
    assert_eq!(disk::size(&outs), 15);
    assert_eq!(disk::remove(&repo, "testScratch").unwrap(), 1024, "says what it freed");
    assert!(!repo.join("target/tmp").exists());
    assert!(repo.join("target/debug/kept.bin").exists(), "another place stays");
    disk::remove(&repo, "exampleOuts").unwrap();
    assert!(!repo.join("examples/shop/out").exists() && repo.join("examples/todo/todo.clarity").exists(), "a project's own files stay");
    assert!(disk::remove(&repo, "nothing").is_err());
}

#[test]
fn the_free_space_and_sizes_read_as_people_read_them() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(disk::free_space(&repo).is_some_and(|f| f > 0), "the disk the repository is on");
    assert_eq!(disk::bytes_text(512), "512 bytes");
    assert_eq!(disk::bytes_text(448 * 1024 * 1024), "448 MB");
    assert_eq!(disk::bytes_text(9 * 1024 * 1024 * 1024 + 400 * 1024 * 1024), "9.4 GB");
}
