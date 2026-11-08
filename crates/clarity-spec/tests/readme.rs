//! A project's README.md, in its folder beside the modules: the design goal
//! and a guide for whoever works on it next, read with the project so the
//! IDE shows and edits it and the agent reads it first.

#[test]
fn the_readme_is_read_with_the_project() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("readme_project");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("shop.clarity"), "module shop\n").unwrap();
    std::fs::write(dir.join(clarity_spec::README_FILE), "# Shop\n\nSell books.\n").unwrap();
    let project = clarity_spec::load_dir(&dir).unwrap();
    assert_eq!(project.readme.as_deref(), Some("# Shop\n\nSell books.\n"));
    std::fs::remove_file(dir.join("README.md")).unwrap();
    assert_eq!(clarity_spec::load_dir(&dir).unwrap().readme, None);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_project_without_its_folder_has_no_readme() {
    assert_eq!(clarity_spec::load_sources([("shop.clarity", "module shop\n")]).readme, None);
}
