//! The installed command-line tool finds the folders its package carries beside
//! the IDE: Clarity.UI's library and its browser files.

use std::path::{Path, PathBuf};

use clarity_gen::clarity_ui::installed_resources;

#[test]
fn on_linux_the_package_carries_them_in_its_lib_folder() {
    assert_eq!(installed_resources(Path::new("/usr/bin"), "linux"), PathBuf::from("/usr/bin/../lib/Clarity IDE"));
}

#[test]
fn on_windows_they_are_beside_the_program() {
    assert_eq!(installed_resources(Path::new(r"C:\Program Files\Clarity IDE"), "windows"), PathBuf::from(r"C:\Program Files\Clarity IDE"));
}

#[test]
fn on_macos_they_are_in_the_bundle_s_resources() {
    assert_eq!(installed_resources(Path::new("/Applications/Clarity IDE.app/Contents/MacOS"), "macos"), PathBuf::from("/Applications/Clarity IDE.app/Contents/MacOS/../Resources"));
}

#[test]
fn the_product_name_is_the_ide_s() {
    let conf = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src-tauri/tauri.conf.json")).unwrap();
    let conf: serde_json::Value = serde_json::from_str(&conf).unwrap();
    assert_eq!(conf["productName"], clarity_gen::clarity_ui::PRODUCT_NAME);
}
