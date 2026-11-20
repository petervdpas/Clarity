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

/// Where Tauri puts a staged file (`staged`, relative to app/src-tauri) by the
/// installer's resources map: a glob copies each file it matches flat into its
/// target folder, by file name; a folder keeps its tree below its target; a
/// plain path goes to its target as it is.
fn installed_as(resources: &serde_json::Map<String, serde_json::Value>, staged: &str) -> Option<String> {
    resources.iter().find_map(|(source, target)| {
        let target = target.as_str()?;
        let name = staged.rsplit('/').next()?;
        match source.split_once('*') {
            Some((folder, _)) if source.contains("**") => staged.starts_with(folder).then(|| format!("{target}{name}")),
            Some((folder, _)) => (staged.starts_with(folder) && !staged[folder.len()..].contains('/')).then(|| format!("{target}{name}")),
            None if source.ends_with('/') => staged.strip_prefix(source.as_str()).map(|below| format!("{target}{below}")),
            None => (source == staged).then(|| target.to_string()),
        }
    })
}

/// Go, C and C++ desktop builds take Clarity.UI's library from the installed
/// clarity-ui folder only when it has include/ and lib/; else they fetch it,
/// which fails without the internet.
#[test]
fn the_installer_keeps_the_library_s_include_and_lib_folders() {
    let conf = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src-tauri/tauri.clarity-ui.conf.json")).unwrap();
    let conf: serde_json::Value = serde_json::from_str(&conf).unwrap();
    let resources = conf["bundle"]["resources"].as_object().unwrap();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("installed-clarity-ui");
    // What .github/scripts/stage-clarity-ui.sh stages on each system.
    for (os, library) in [("linux", "libclarity_ui.so"), ("windows", "clarity_ui.dll"), ("macos", "libclarity_ui.dylib")] {
        let _ = std::fs::remove_dir_all(&root);
        for staged in ["clarity-ui/include/clarity_ui.h".to_string(), format!("clarity-ui/lib/{library}"), "clarity-ui/LICENSE".to_string()] {
            let installed = installed_as(resources, &staged).unwrap_or_else(|| panic!("the installer does not carry {staged}"));
            let path = root.join(installed);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"").unwrap();
        }
        assert!(clarity_gen::clarity_ui::complete(&root.join("clarity-ui"), os), "on {os} the installed clarity-ui folder lacks include/ or lib/, so builds fetch the library");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The runtimes a build takes with no internet: one archive per stack, in
/// the installed runtimes folder (docs/ledger/offline.md).
#[test]
fn the_installer_carries_each_stack_s_runtimes_archive() {
    let conf = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src-tauri/tauri.clarity-ui.conf.json")).unwrap();
    let conf: serde_json::Value = serde_json::from_str(&conf).unwrap();
    let resources = conf["bundle"]["resources"].as_object().unwrap();
    let folder = clarity_gen::carried::FOLDER;
    for backend in [clarity_project::Backend::Rust, clarity_project::Backend::Go] {
        let below = clarity_gen::carried::archive_name(backend).unwrap();
        let staged = format!("{folder}/{below}");
        assert_eq!(installed_as(resources, &staged).as_deref(), Some(staged.as_str()), "the installer moves {staged}");
    }
}

/// The browser files come in two sets, the one without reports in the
/// folder and the one with them in `reports/` (web_browser::REPORTS_SET): the
/// installer keeps both, each file at its own path, so neither set is lost and
/// no path is added twice (a duplicate fails the rpm, deb and msi bundles).
#[test]
fn the_installer_keeps_both_sets_of_browser_files_apart() {
    let conf = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src-tauri/tauri.clarity-ui.conf.json")).unwrap();
    let conf: serde_json::Value = serde_json::from_str(&conf).unwrap();
    let resources = conf["bundle"]["resources"].as_object().unwrap();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("installed-clarity-ui-browser");
    let _ = std::fs::remove_dir_all(&root);
    // What runtimes/rust-ui-browser/build.sh and .github/scripts/stage-clarity-ui.sh stage.
    let mut staged: Vec<String> = vec!["clarity-ui-browser/LICENSE".to_string()];
    for set in ["clarity-ui-browser".to_string(), format!("clarity-ui-browser/{}", clarity_gen::web_browser::REPORTS_SET)] {
        staged.extend(clarity_gen::clarity_ui::BROWSER_FILES.iter().map(|name| format!("{set}/{name}")));
    }
    let mut installed = std::collections::BTreeSet::new();
    for file in &staged {
        let to = installed_as(resources, file).unwrap_or_else(|| panic!("the installer does not carry {file}"));
        assert!(installed.insert(to.clone()), "the installer puts {file} at {to}, where another file goes too");
        let path = root.join(&to);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"").unwrap();
    }
    let folder = root.join("clarity-ui-browser");
    assert!(clarity_gen::clarity_ui::browser_complete(&folder), "the installed set without reports is not complete");
    assert_eq!(clarity_gen::web_browser::widest_set(&folder), folder.join(clarity_gen::web_browser::REPORTS_SET), "the installed set with reports is not complete");
    let _ = std::fs::remove_dir_all(&root);
}
