//! DevTools' swap core (a Clarity run from its repository only): the
//! pieces of Clarity an app uses, the core ones always on and the others
//! added by the cDdev; for each, the release or the cDdev's own build from
//! the repository; a signal when that build is behind its source, naming
//! what to run; and the folders builds then use, the native libraries put
//! together from the release with the swapped ones in their place.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use clarity_gen::tools::devtools::swap::{self, Swaps};

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
fn the_core_pieces_are_always_on_and_the_others_are_added() {
    let core: Vec<&str> = swap::PIECES.iter().filter(|p| p.core).map(|p| p.key).collect();
    assert_eq!(core, ["clarityUi", "clarityUiBrowser", "rustCrates", "nugetPackages", "goModule"]);
    let added: Vec<&str> = swap::PIECES.iter().filter(|p| !p.core).map(|p| p.key).collect();
    assert_eq!(added, ["clarityAudio", "clarityVideo", "clarityImage", "clarityFormats", "clarityAuth", "clarityAppMcp"]);
    let mut swaps = Swaps::default();
    assert_eq!(swaps.tabs(), core, "a fresh panel shows the core tabs");
    swaps.add("clarityAudio").unwrap();
    assert!(swaps.tabs().contains(&"clarityAudio"));
    assert!(swaps.add("rustCrates").is_err(), "a core tab is always there");
    assert!(swaps.add("nothing").is_err());
    swaps.swap("clarityAudio", true).unwrap();
    swaps.close("clarityAudio").unwrap();
    assert!(!swaps.tabs().contains(&"clarityAudio") && !swaps.mine("clarityAudio"), "closed, its piece is the release again");
    assert!(swaps.close("clarityUi").is_err(), "a core tab cannot be closed");
    assert!(swaps.swap("clarityVideo", true).is_err(), "a piece is swapped in its open tab only");
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
