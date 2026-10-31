//! The web screen every generated program serves is a built copy
//! (crates/clarity-gen/ui, from app/ with `npm run build:runtime`). The build
//! records the source files it was made from and a hash of them
//! (ui/runtime.sources.json); a screen change that was not built again fails
//! here, instead of every web stack quietly running the old screen.

use std::path::Path;

/// FNV-1a, 64 bits, over each file's path and bytes, as the build hashes them (app/vite.runtime.config.ts).
fn fnv(files: &[(String, Vec<u8>)]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (path, bytes) in files {
        for b in path.as_bytes().iter().chain(b"\n").chain(bytes.iter()).chain(b"\n") {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

#[test]
fn the_web_screen_is_built_from_the_sources_as_they_are() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let app = crate_dir.join("../../app");
    let record = std::fs::read_to_string(crate_dir.join("ui/runtime.sources.json")).expect("ui/runtime.sources.json: run `npm run build:runtime` in app/");
    let record: serde_json::Value = serde_json::from_str(&record).unwrap();
    let paths: Vec<String> = record["files"].as_array().expect("files").iter().map(|f| f.as_str().unwrap().to_string()).collect();
    assert!(paths.iter().any(|p| p == "src/lib/ui/CanvasView.svelte"), "the record lists the screen's sources");
    let files: Vec<(String, Vec<u8>)> = paths.iter().map(|p| (p.clone(), std::fs::read(app.join(p)).unwrap_or_default())).collect();
    assert_eq!(
        fnv(&files),
        record["fnv1a64"].as_str().unwrap_or_default(),
        "the web screen changed since crates/clarity-gen/ui was built: run `npm run build:runtime` in app/"
    );
}
