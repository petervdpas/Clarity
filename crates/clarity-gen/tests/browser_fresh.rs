//! A Clarity run from its repository draws the design and preview with the
//! browser files the repository built; when Clarity.UI's source changed since,
//! it says so, as those files do not show the change (a stale build once left
//! every window undrawn).

use std::path::Path;
use std::time::{Duration, SystemTime};

fn touch(path: &Path, at: SystemTime) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "x").unwrap();
    std::fs::File::options().write(true).open(path).unwrap().set_modified(at).unwrap();
}

#[test]
fn browser_files_older_than_clarity_ui_s_source_are_named_stale() {
    let repo = std::env::temp_dir().join(format!("clarity-browser-fresh-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    let then = SystemTime::now() - Duration::from_secs(3600);
    let now = SystemTime::now();
    touch(&repo.join("runtimes/rust-ui/src/lib.rs"), then);
    touch(&repo.join("target/clarity-ui-browser/clarity-ui.wasm"), now);
    assert_eq!(clarity_gen::clarity_ui::stale_browser(&repo), None);
    touch(&repo.join("runtimes/rust-ui/src/parts/tabs.rs"), now + Duration::from_secs(60));
    assert!(clarity_gen::clarity_ui::stale_browser(&repo).is_some_and(|m| m.contains("build.sh")));
    // Not built at all: nothing to call stale.
    std::fs::remove_file(repo.join("target/clarity-ui-browser/clarity-ui.wasm")).unwrap();
    assert_eq!(clarity_gen::clarity_ui::stale_browser(&repo), None);
    let _ = std::fs::remove_dir_all(&repo);
}
