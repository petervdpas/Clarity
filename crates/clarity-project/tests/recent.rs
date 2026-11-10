//! The file browser's memory (docs/PROJECT-BROWSER-PLAN.md): the 10
//! projects opened last, newest first, and the folder each use was left in
//! ("10 recent, remember last folder"). Kept in `recent.json` in Clarity's
//! own folder, changed key by key.

use std::path::PathBuf;

use clarity_project::recent::{KEPT, last_folder_in, recent_in, remember_folder_in, remember_project_in};

fn file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-recent-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("recent.json")
}

#[test]
fn the_newest_project_comes_first_once_and_ten_are_kept() {
    let file = file("projects");
    let dir = file.parent().unwrap().to_path_buf();
    let project = |n: usize| {
        let p = dir.join(format!("p{n}.clarity"));
        std::fs::write(&p, "project P\n").unwrap();
        p
    };
    for n in 0..12 {
        remember_project_in(&file, &project(n)).unwrap();
    }
    remember_project_in(&file, &project(5)).unwrap();
    let recent = recent_in(&file);
    assert_eq!(KEPT, 10);
    assert_eq!(recent.len(), 10);
    assert_eq!(recent[0], project(5), "opened again: first, and once");
    assert_eq!(recent.iter().filter(|p| **p == project(5)).count(), 1);
    assert_eq!(recent[1], project(11));
    // A project that is gone is left out of the list, and stays in the file.
    std::fs::remove_file(project(11)).unwrap();
    assert!(!recent_in(&file).contains(&project(11)));
    assert!(std::fs::read_to_string(&file).unwrap().contains("p11.clarity"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn each_use_remembers_its_folder_and_other_keys_stay() {
    let file = file("folders");
    let dir = file.parent().unwrap().to_path_buf();
    std::fs::write(&file, r#"{"mine": 1}"#).unwrap();
    assert_eq!(last_folder_in(&file, "open-project"), None);
    remember_folder_in(&file, "open-project", &dir).unwrap();
    remember_folder_in(&file, "folder", &std::env::temp_dir()).unwrap();
    assert_eq!(last_folder_in(&file, "open-project").as_deref(), Some(dir.as_path()));
    assert_eq!(last_folder_in(&file, "folder").as_deref(), Some(std::env::temp_dir().as_path()));
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"mine\""), "{text}");
    // A folder that is gone is no place to start.
    remember_folder_in(&file, "file", &dir.join("gone")).unwrap();
    assert_eq!(last_folder_in(&file, "file"), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_broken_file_is_left_alone() {
    let file = file("broken");
    std::fs::write(&file, "not json").unwrap();
    assert!(recent_in(&file).is_empty());
    assert!(remember_folder_in(&file, "folder", &std::env::temp_dir()).is_err());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "not json");
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}
