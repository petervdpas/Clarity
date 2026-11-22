//! DevTools' swap core (a Clarity run from its repository only): the
//! pieces of Clarity an app uses, the core ones always on and the others
//! added by the cDdev; for each, the release or the cDdev's own build from
//! the repository; a signal when that build is behind its source, naming
//! what to run; and the folders builds then use, the native libraries put
//! together from the release with the swapped ones in their place.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use clarity_devtools::swap::{self, Swaps};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-swaps-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn put(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn age(path: &Path, seconds: u64) {
    let at = SystemTime::now() - Duration::from_secs(seconds);
    std::fs::File::options().write(true).open(path).unwrap().set_modified(at).unwrap();
}

/// A repository with Clarity.UI's and Clarity.Audio's libraries built, and
/// their sources older than the builds.
fn repository(root: &Path) -> PathBuf {
    let repo = root.join("repo");
    for (src, header) in [("rust-ui", "rust-ui-ffi/include/clarity_ui.h"), ("rust-audio", "rust-audio-ffi/include/clarity_audio.h")] {
        let file = repo.join("runtimes").join(src).join("src/lib.rs");
        put(&file, "// source");
        age(&file, 600);
        put(&repo.join("runtimes").join(header), "// header");
    }
    // Built after the sources and well before now: a file's time has a
    // coarse clock, so a change made now must be clearly newer.
    for lib in ["libclarity_ui.so", "libclarity_audio.so"] {
        let built = repo.join("target/release").join(lib);
        put(&built, "mine");
        age(&built, 300);
    }
    repo
}

/// The release's folder of the native libraries, every one of them.
fn release(root: &Path) -> PathBuf {
    let folder = root.join("release");
    for name in ["ui", "audio", "video"] {
        put(&folder.join(format!("include/clarity_{name}.h")), "release header");
        put(&folder.join(format!("lib/libclarity_{name}.so")), "release");
    }
    folder
}

#[test]
fn a_fresh_panel_shows_no_piece_and_pieces_are_added() {
    // DevTools shows Disk, Release planning and the pieces marked with a signal; any other piece's tab is added.
    let core: Vec<&str> = swap::PIECES.iter().filter(|p| p.core).map(|p| p.key).collect();
    assert_eq!(core, ["clarityUi", "clarityUiBrowser", "rustCrates", "nugetPackages", "goModule"]);
    let mut swaps = Swaps::default();
    assert!(swaps.tabs().is_empty(), "a fresh panel has no piece's tab of its own");
    swaps.add("clarityAudio").unwrap();
    swaps.add("clarityUi").unwrap();
    assert_eq!(swaps.tabs(), ["clarityUi", "clarityAudio"], "in the pieces' order");
    assert!(swaps.add("nothing").is_err());
    swaps.swap("clarityAudio", true).unwrap();
    swaps.close("clarityAudio").unwrap();
    assert!(!swaps.tabs().contains(&"clarityAudio") && !swaps.mine("clarityAudio"), "closed, its piece is the release again");
    assert!(swaps.add("clarityAudio").is_ok() && swaps.tabs().contains(&"clarityAudio"), "and comes back with + Add a piece");
    assert!(swaps.swap("clarityVideo", true).is_err(), "a piece is swapped in its open tab only");
}

#[test]
fn a_swapped_piece_keeps_its_tab() {
    // Kept swaps from before, a core piece swapped without being added: its tab shows, so it can be swapped back.
    let swaps: Swaps = serde_json::from_str(r#"{"mine":["clarityUiBrowser"]}"#).unwrap();
    assert!(swaps.tabs().contains(&"clarityUiBrowser"));
    assert!(swaps.shown(|_| false).iter().any(|(p, auto)| p.key == "clarityUiBrowser" && !*auto));
}

#[test]
fn a_build_behind_its_source_is_signalled_with_what_to_run() {
    let root = scratch("signal");
    let repo = repository(&root);
    let ui = swap::piece("clarityUi").unwrap();
    assert_eq!(swap::behind(&repo, ui, "linux"), None, "the build is newer than its source");
    put(&repo.join("runtimes/rust-ui/src/lib.rs"), "// changed");
    let said = swap::behind(&repo, ui, "linux").expect("behind its source");
    assert!(said.contains("cargo build --release -p clarity-ui-ffi"), "{said}");
    let video = swap::piece("clarityVideo").unwrap();
    assert!(swap::behind(&repo, video, "linux").is_some_and(|s| s.contains("not built") && s.contains("clarity-video-ffi")));
    // A runtime is used from its source as it is: never behind.
    assert_eq!(swap::behind(&repo, swap::piece("rustCrates").unwrap(), "linux"), None);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn builds_use_the_release_until_a_piece_is_swapped() {
    let root = scratch("choices");
    let repo = repository(&root);
    let release = release(&root);
    let mut swaps = Swaps::default();
    let chosen = swap::choices(&repo, &swaps, Some(&release), "linux").unwrap();
    assert!(chosen.is_empty(), "nothing swapped: every build uses the release");
    swaps.add("clarityAudio").unwrap();
    swaps.swap("clarityAudio", true).unwrap();
    swaps.add("rustCrates").unwrap();
    swaps.swap("rustCrates", true).unwrap();
    let chosen = swap::choices(&repo, &swaps, Some(&release), "linux").unwrap();
    assert_eq!(chosen.get("rustCrates"), Some(&repo.join("runtimes/rust")));
    // The native libraries: the release's, with the cDdev's Clarity.Audio in its place.
    let put_together = chosen.get("clarityUi").expect("a folder put together");
    let read = |p: &str| std::fs::read_to_string(put_together.join(p)).unwrap();
    assert_eq!(read("lib/libclarity_ui.so"), "release");
    assert_eq!(read("lib/libclarity_audio.so"), "mine");
    assert_eq!(read("include/clarity_audio.h"), "// header");
    assert_eq!(read("lib/libclarity_video.so"), "release");
    // A native piece swapped while the release is not on this machine says so.
    assert!(swap::choices(&repo, &swaps, None, "linux").unwrap_err().contains("release"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_piece_whose_own_build_is_behind_shows_its_tab_by_itself() {
    let swaps = Swaps::default();
    let shown = swaps.shown(|p| p.key == "clarityAudio");
    let keys: Vec<(&str, bool)> = shown.iter().map(|(p, auto)| (p.key, *auto)).collect();
    assert!(keys.contains(&("clarityAudio", true)), "behind: shown by itself {keys:?}");
    assert!(!keys.iter().any(|(k, _)| *k == "clarityVideo"), "not behind and not added: not shown");
    assert!(!keys.iter().any(|(k, _)| *k == "clarityUi"), "a core piece neither behind nor added: not shown either");
    let order: Vec<&str> = shown.iter().map(|(p, _)| p.key).collect();
    let pieces: Vec<&str> = swap::PIECES.iter().map(|p| p.key).filter(|k| order.contains(k)).collect();
    assert_eq!(order, pieces, "in the pieces' order");
}

#[test]
fn a_tab_shown_by_itself_comes_back_while_its_build_is_behind() {
    // Closing it hides it while Clarity runs (the IDE keeps that); the swaps keep only what was added and swapped.
    let mut swaps = Swaps::default();
    assert!(swaps.swap("clarityAudio", true).is_err(), "shown by itself, it is added before it is swapped (the IDE does)");
    swaps.add("clarityAudio").unwrap();
    swaps.swap("clarityAudio", true).unwrap();
    swaps.close("clarityAudio").unwrap();
    assert!(swaps.shown(|p| p.key == "clarityAudio").iter().any(|(p, auto)| p.key == "clarityAudio" && *auto), "behind: shown by itself again");
    assert_eq!(serde_json::to_string(&swaps).unwrap(), "{}", "nothing of a closed tab is kept");
    let old: Swaps = serde_json::from_str(r#"{"closed":["clarityUiBrowser"]}"#).unwrap();
    assert!(old.shown(|p| p.key == "clarityUiBrowser").iter().any(|(p, _)| p.key == "clarityUiBrowser"), "a closed list kept before is no longer read");
}
