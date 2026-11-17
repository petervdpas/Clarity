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
    let out = Command::new("cmake").arg("-S").arg(source).arg("-B").arg(build).env("CLARITY_UI", folder).output().expect("cmake runs");
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn the_folder_named_now_wins_over_the_one_cmake_kept() {
    if Command::new("cmake").arg("--version").output().is_err() {
        return;
    }
    for runtime in ["c", "cpp"] {
        let dir = temp(runtime);
        // The runtime's macro that picks the folder, on its own: the rest of the file finds libraries (curl, SQLite) a bare machine may lack.
        let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes").join(runtime).join("clarity.cmake")).unwrap();
        let start = text.find("macro(clarity_ui_folder)").expect("the macro");
        let end = start + text[start..].find("endmacro()").expect("its end") + "endmacro()".len();
        let source = dir.join("project");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("CMakeLists.txt"), format!("cmake_minimum_required(VERSION 3.16)\nproject(swapped NONE)\n{}\nclarity_ui_folder()\nmessage(STATUS \"uses ${{CLARITY_UI}}\")\n", &text[start..end])).unwrap();
        let release = clarity_ui_folder(&dir.join("release"));
        let own = clarity_ui_folder(&dir.join("own"));
        let build = dir.join("build");
        // As CMake prints a PATH: with `/` between folders, on Windows too.
        let (release_shown, own_shown) = (clarity_project::slashed(&release), clarity_project::slashed(&own));
        assert!(configured(&source, &build, &release).contains(&format!("uses {release_shown}")), "{runtime}: the release first");
        assert!(configured(&source, &build, &own).contains(&format!("uses {own_shown}")), "{runtime}: swapped, the next build uses the cDdev's own");
        assert!(configured(&source, &build, &release).contains(&format!("uses {release_shown}")), "{runtime}: and the release again");
    }
}

