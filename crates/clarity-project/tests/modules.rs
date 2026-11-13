//! A new module made by hand in the IDE: its file beside the others, with
//! its `module` line, refused for a name that is not one or already taken.

use clarity_project::new_module_file;

#[test]
fn a_new_module_is_a_file_with_its_module_line() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/clarity-new-module-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("shop.clarity"), "module shop\n\nentity Customer\n    name: text\n").unwrap();
    std::fs::write(dir.join("odd.clarity"), "module stock\n").unwrap();
    let (path, text) = new_module_file(&dir, "billing").unwrap();
    assert_eq!(path, dir.join("billing.clarity"));
    assert_eq!(text, "module billing\n");
    assert!(new_module_file(&dir, "shop").unwrap_err().contains("already"));
    assert!(new_module_file(&dir, "stock").unwrap_err().contains("already a module stock"));
    assert!(new_module_file(&dir, "two words").unwrap_err().contains("not a valid module name"));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The IDE's type list is the tools' own: the catalog hands both lists over.
#[test]
fn the_catalog_has_the_field_and_setting_types() {
    let catalog = clarity_project::catalog();
    assert_eq!(catalog.field_types, clarity_spec::dictionary::SIMPLE_TYPES);
    assert_eq!(catalog.named_types, clarity_spec::dictionary::NAMED_TYPES);
    assert_eq!(catalog.setting_types, clarity_spec::dictionary::SETTING_TYPES);
}
