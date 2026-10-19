// Tauri's build, and the release a development build stands on: the newest
// `v*` tag reachable from the checkout, so a build from the repository says
// "Clarity 0.3.0-dev" instead of the placeholder version in tauri.conf.json
// (the release pipeline stamps a release's own version from its tag).
use std::process::Command;

fn main() {
    let tag = Command::new("git")
        .args(["describe", "--tags", "--match", "v*", "--abbrev=0"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default();
    println!("cargo:rustc-env=CLARITY_DEV_RELEASE={tag}");
    // Built again when a commit or a tag is made.
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/tags");
    tauri_build::build()
}
