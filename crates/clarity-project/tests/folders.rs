//! Clarity's folders, set once for every project on this computer: where
//! new projects go, and where the apps are written. Kept in `folders.json`
//! in Clarity's own folder, beside `environment.json`, so the app and the
//! `clarity` command read the same choice.

use std::path::{Path, PathBuf};

use clarity_project::{out_dir_in, parse, Folders};

const SHOP: &str = "project Shop\n    stack rust web desktop\n";

fn home(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("folders-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn nothing_set_keeps_the_app_beside_the_project() {
    let folders = Folders::load(&home("none").join("folders.json"));
    assert_eq!(folders, Folders::default());
    let file = Path::new("/work/shop.clarity");
    assert_eq!(out_dir_in(file, &parse(SHOP).unwrap(), &folders), Path::new("/work/shop/out"));
}

#[test]
fn the_output_folder_holds_a_folder_per_project() {
    let folders = Folders { projects: None, output: Some(PathBuf::from("/builds")) };
    let file = Path::new("/work/shop.clarity");
    assert_eq!(out_dir_in(file, &parse(SHOP).unwrap(), &folders), Path::new("/builds/shop"));
    // A project that names its own app folder keeps it.
    let mut own = parse(SHOP).unwrap();
    own.out = "../apps/shop".into();
    assert_eq!(out_dir_in(file, &own, &folders), Path::new("/work/shop/../apps/shop"));
    own.out = "/elsewhere/shop".into();
    assert_eq!(out_dir_in(file, &own, &folders), Path::new("/elsewhere/shop"));
}

#[test]
fn the_folders_are_saved_and_read_back() {
    let dir = home("saved");
    let file = dir.join("folders.json");
    let folders = Folders { projects: Some(dir.join("projects")), output: Some(dir.join("apps")) };
    folders.save(&file).unwrap();
    assert_eq!(Folders::load(&file), folders);
    // Only one of them set.
    let one = Folders { projects: None, output: Some(dir.join("apps")) };
    one.save(&file).unwrap();
    assert_eq!(Folders::load(&file), one);
}

#[test]
fn a_folder_is_a_full_path() {
    assert!(Folders::problem("projects").unwrap().contains("projects is not a full path"));
    assert_eq!(Folders::problem(""), Some("choose a folder".to_string()));
    let full = std::env::temp_dir();
    assert_eq!(Folders::problem(&full.display().to_string()), None);
    // An existing file is not a folder.
    let dir = home("file");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("taken"), "").unwrap();
    let taken = dir.join("taken").display().to_string();
    assert_eq!(Folders::problem(&taken), Some(format!("{taken} is a file, not a folder")));
}

#[test]
fn a_new_project_name_is_a_file_in_the_projects_folder() {
    let folders = Folders { projects: Some(PathBuf::from("/work")), output: None };
    assert_eq!(folders.new_project_file("shop").unwrap(), Path::new("/work/shop.clarity"));
    assert_eq!(folders.new_project_file(" my shop.clarity ").unwrap(), Path::new("/work/my shop.clarity"));
    assert!(folders.new_project_file("").unwrap_err().contains("name"));
    assert!(folders.new_project_file("a/b").unwrap_err().contains("name"));
    assert!(Folders::default().new_project_file("shop").unwrap_err().contains("projects folder"));
}
