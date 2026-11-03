//! Clarity.UI for Rust splits as the C# version does (C# SplitTests): what
//! only a desktop program has (the system's windows through winit, OpenGL
//! through glutin, the clipboard through arboard, sound through rodio) is
//! the `desktop` feature's, so a program built without it, and Clarity's
//! runtime, carry nothing of it. The keys window and the unconnected flow
//! are the core's, since a page shows them too.

use std::path::Path;

const NATIVE_ONLY: [&str; 5] = ["winit", "glutin", "glutin-winit", "arboard", "rodio"];

fn manifest(folder: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(folder).join("Cargo.toml")).unwrap()
}

/// The line of a manifest that names a crate as a dependency.
fn dependency<'a>(manifest: &'a str, name: &str) -> Option<&'a str> {
    manifest.lines().find(|l| l.trim_start().starts_with(&format!("{name} =")))
}

#[test]
fn the_core_references_nothing_native() {
    let ui = manifest(".");
    // Each native crate is the desktop feature's alone.
    for name in NATIVE_ONLY {
        let line = dependency(&ui, name).unwrap_or_else(|| panic!("{name} is a dependency"));
        assert!(line.contains("optional = true"), "{name} is optional: {line}");
    }
    let feature = ui.lines().find(|l| l.starts_with("desktop =")).expect("a desktop feature");
    for name in NATIVE_ONLY {
        assert!(feature.contains(&format!("\"dep:{name}\"")), "the desktop feature brings {name}");
    }
    // The runtime has none of them at all.
    let runtime = manifest("../rust");
    for name in NATIVE_ONLY {
        assert_eq!(None, dependency(&runtime, name), "the runtime names {name}");
    }
    // The desktop host's module is the feature's (and never a browser build's).
    let lib = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")).unwrap();
    let at = lib.find("\npub mod desktop;").expect("a desktop module");
    let cfg = lib[..at].lines().next_back().unwrap_or_default();
    assert!(cfg.starts_with("#[cfg(") && cfg.contains("feature = \"desktop\""), "{cfg}");
    assert!(cfg.contains("not(target_os = \"emscripten\")"), "{cfg}");
}

#[test]
fn the_desktop_shell_is_the_desktop_feature_and_the_keys_window_the_core_s() {
    let ui = manifest(".");
    assert!(ui.lines().any(|l| l.starts_with("default = [\"desktop\"]")));
    // Shown in a page too: the core's.
    let _keys: fn() -> String = clarity_ui::dialogs::keys::title;
    let _unconnected = std::any::type_name::<clarity_ui::window::WindowStack>();
    let readme = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md")).unwrap();
    assert!(readme.contains("feature `desktop`"));
}
