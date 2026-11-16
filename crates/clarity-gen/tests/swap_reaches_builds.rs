//! A cDdev's swap of Clarity.UI counts from the next build on every stack:
//! Clarity's build names the folder (`CLARITY_UI`) each time, and a C or
//! C++ build folder configured before with another folder (the release)
//! takes the one named now, not the one CMake kept.

use std::path::{Path, PathBuf};
use std::process::Command;

fn temp(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("swap-builds").join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A folder as Clarity.UI's: its header and its library.
fn clarity_ui_folder(at: &Path) -> PathBuf {
    std::fs::create_dir_all(at.join("include")).unwrap();
    std::fs::create_dir_all(at.join("lib")).unwrap();
    std::fs::write(at.join("include/clarity_ui.h"), "").unwrap();
    std::fs::write(at.join("lib/libclarity_ui.so"), "").unwrap();
    at.to_path_buf()
}

fn configured(source: &Path, build: &Path, folder: &Path) -> String {
    // Only configured: the runtime's own targets have no sources in this bare project, so generating fails after.
    let out = Command::new("cmake").arg("-S").arg(source).arg("-B").arg(build).env("CLARITY_UI", folder).output().expect("cmake runs");
    let said = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(said.contains("uses "), "{said}{}", String::from_utf8_lossy(&out.stderr));
    said
}

#[test]
fn the_folder_named_now_wins_over_the_one_cmake_kept() {
    if cfg!(not(target_os = "linux")) || Command::new("cmake").arg("--version").output().is_err() {
        return;
    }
    for runtime in ["c", "cpp"] {
        let dir = temp(runtime);
        let cmake = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes").join(runtime).join("clarity.cmake").canonicalize().unwrap();
        let source = dir.join("project");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("CMakeLists.txt"), format!("cmake_minimum_required(VERSION 3.16)\nproject(swapped C)\ninclude({})\nclarity_ui_folder()\nmessage(STATUS \"uses ${{CLARITY_UI}}\")\n", cmake.display())).unwrap();
        let release = clarity_ui_folder(&dir.join("release"));
        let own = clarity_ui_folder(&dir.join("own"));
        let build = dir.join("build");
        assert!(configured(&source, &build, &release).contains(&format!("uses {}", release.display())), "{runtime}: the release first");
        assert!(configured(&source, &build, &own).contains(&format!("uses {}", own.display())), "{runtime}: swapped, the next build uses the cDdev's own");
        assert!(configured(&source, &build, &release).contains(&format!("uses {}", release.display())), "{runtime}: and the release again");
    }
}

