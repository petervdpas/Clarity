//! DevTools' swap core, for the cDdev (a Clarity run from its repository):
//! the pieces of Clarity an app uses, each the release or the cDdev's own
//! build. The core pieces always have their tab; the others the cDdev adds
//! when they know that piece has changes. Each tab's Build makes the
//! cDdev's own build (`build`, run by jobs.rs); a build behind its source is
//! signalled.
//! Builds then use the folders `choices` gives: the native libraries put
//! together from the release with the swapped ones in their place (they are
//! one folder, as the release's archive is), the browser files and the
//! runtimes from the repository. An app developer (installed Clarity) always
//! uses the release: nothing here is read there.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::jobs::Runnable;
pub use clarity_gen_core::tools::pieces::*;

/// A native library's file in `lib/` on system `os`: `libclarity_audio.so`.
pub fn library_file(stem: &str, os: &str) -> String {
    clarity_gen_core::clarity_ui::library_file(os).replace("clarity_ui", stem)
}

/// The file the cDdev's build of `piece` made, whose time says how new it
/// is; none for a runtime, used from its source as it is.
pub fn built_file(repo: &Path, piece: &Piece, os: &str) -> Option<PathBuf> {
    match piece.kind {
        Kind::Native { stem, .. } => Some(repo.join("target").join("release").join(library_file(stem, os))),
        Kind::Browser => Some(repo.join("target").join("clarity-ui-browser").join("clarity-ui.wasm")),
        Kind::Runtime { .. } => None,
    }
}

/// When the cDdev's build of `piece` was made; none when it is not built or has no build.
pub fn built_at(repo: &Path, piece: &Piece, os: &str) -> Option<SystemTime> {
    built_file(repo, piece, os).and_then(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok())
}

/// The key of a piece's build among DevTools' builds (`piece:clarityUi`),
/// with `debug` of its debug build (`piece:clarityUi:debug`): the IDE runs
/// it under this key and build_state reads its record by it.
pub fn job_key(piece: &str, debug: bool) -> String {
    if debug { format!("piece:{piece}:debug") } else { format!("piece:{piece}") }
}

/// The build a tab's Build button runs for `piece`, in the repository:
/// at most two jobs of cargo at once, as the machine can bear.
pub fn build(repo: &Path, piece: &Piece) -> Runnable {
    match piece.kind {
        Kind::Native { package, .. } => Runnable::new("cargo", &["build", "--release", "-p", package], repo).with_env("CARGO_BUILD_JOBS", "2"),
        // Thin LTO for the cDdev's own browser files (Cargo.toml's browser-own); releases build `browser`.
        // One job: linking the browser files takes most of the memory a 7.5 GB machine has beside the IDE.
        Kind::Browser => Runnable::new(&clarity_gen_core::tools::bash(), &["runtimes/rust-ui-browser/build.sh"], repo).with_env("CARGO_BUILD_JOBS", "1").with_env("CLARITY_BROWSER_PROFILE", "browser-own"),
        Kind::Runtime { folder: "runtimes/rust" } => Runnable::new("cargo", &["build", "-p", "clarity-runtime"], repo).with_env("CARGO_BUILD_JOBS", "2"),
        // The NuGet packages come from the Release build, into each project's bin/Release.
        Kind::Runtime { folder: "runtimes/csharp" } => Runnable::new("dotnet", &["build", "-c", "Release", "Clarity.slnx"], &repo.join("runtimes/csharp")).with_env("MSBUILDDISABLENODEREUSE", "1"),
        Kind::Runtime { folder } => Runnable::new("go", &["build", "./..."], &repo.join(folder)),
    }
}

/// After a build of `piece` passed (`debug`: its Build debug), its library as
/// current as its sources: Cargo leaves the file as it was when nothing in it
/// changed (a change only to a test-only dependency), and the C# packages'
/// check (runtimes/csharp/ClarityNative.targets) compares the library's time
/// with its sources'. Nothing for a piece without a library.
pub fn mark_built(repo: &Path, piece: &Piece, debug: bool, os: &str) {
    let Kind::Native { stem, .. } = piece.kind else { return };
    let file = repo.join("target").join(if debug { "debug" } else { "release" }).join(library_file(stem, os));
    if let Ok(opened) = std::fs::File::options().write(true).open(&file) {
        let _ = opened.set_modified(SystemTime::now());
    }
}

/// The debug build a tab's Build debug button runs for `piece`: a native
/// library's debug profile (target/debug), which C# programs built in Debug
/// and the runtimes' tests take; none for a runtime or the browser files.
pub fn build_debug(repo: &Path, piece: &Piece) -> Option<Runnable> {
    match piece.kind {
        Kind::Native { package, .. } => Some(Runnable::new("cargo", &["build", "-p", package], repo).with_env("CARGO_BUILD_JOBS", "2")),
        _ => None,
    }
}

/// The signal: the cDdev's build of `piece` is not there, or is behind its
/// source, with the Build that makes it again. None when it is as new as
/// its source, or the piece is used from its source.
pub fn behind(repo: &Path, piece: &Piece, os: &str) -> Option<String> {
    built_file(repo, piece, os)?;
    let command = build(repo, piece).shown();
    let Some(built) = built_at(repo, piece, os) else {
        return Some(format!("{} is not built: press Build ({command})", piece.name));
    };
    let newest = piece.sources.iter().filter_map(|s| clarity_gen_core::clarity_ui::newest_in(&repo.join(s))).max()?;
    (newest > built).then(|| format!("{} is behind its source: press Build ({command})", piece.name))
}

/// Copies every file of `from` into `to`, folders and all.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// The folder of native libraries builds use with `swapped` native pieces:
/// the release's files, with the cDdev's build of each swapped piece (its
/// library and its header) in its place, in `<repo>/target/clarity-swap`.
fn put_together(repo: &Path, release: &Path, swapped: &[&Piece], os: &str) -> Result<PathBuf, String> {
    let folder = repo.join("target").join("clarity-swap");
    let failed = |e: std::io::Error| format!("cannot put the libraries together in {}: {e}", folder.display());
    let _ = std::fs::remove_dir_all(&folder);
    copy_tree(release, &folder).map_err(failed)?;
    for piece in swapped {
        let Kind::Native { stem, header, .. } = piece.kind else { continue };
        let mut files = vec![(repo.join("target").join("release").join(library_file(stem, os)), folder.join("lib").join(library_file(stem, os)))];
        if os == "windows" {
            let import = format!("{stem}.dll.lib");
            files.push((repo.join("target").join("release").join(&import), folder.join("lib").join(&import)));
        }
        files.push((repo.join(header).join(format!("{stem}.h")), folder.join("include").join(format!("{stem}.h"))));
        for (from, to) in files {
            if !from.is_file() {
                return Err(format!("{} is not built: press its Build first ({})", piece.name, build(repo, piece).shown()));
            }
            std::fs::create_dir_all(to.parent().unwrap_or(&folder)).map_err(failed)?;
            std::fs::copy(&from, &to).map_err(failed)?;
        }
    }
    Ok(folder)
}

/// The folders builds use in place of the release, by library choice
/// (tools::environment::Choosable): none for a piece left at the release.
/// `release` is the release's folder of native libraries, needed once a
/// native piece is swapped.
pub fn choices(repo: &Path, swaps: &Swaps, release: Option<&Path>, os: &str) -> Result<BTreeMap<String, PathBuf>, String> {
    let mut out = BTreeMap::new();
    let swapped: Vec<&Piece> = swaps.tabs().into_iter().filter(|k| swaps.mine(k)).filter_map(piece).collect();
    let natives: Vec<&Piece> = swapped.iter().copied().filter(|p| matches!(p.kind, Kind::Native { .. })).collect();
    if !natives.is_empty() {
        let release = release.ok_or("the release's native libraries are not on this machine yet: build an app once to fetch them, then swap")?;
        out.insert("clarityUi".to_string(), put_together(repo, release, &natives, os)?);
    }
    for piece in swapped {
        match piece.kind {
            Kind::Browser => {
                out.insert(piece.key.to_string(), repo.join("target").join("clarity-ui-browser"));
            }
            Kind::Runtime { folder } => {
                out.insert(piece.key.to_string(), repo.join(folder));
            }
            Kind::Native { .. } => {}
        }
    }
    Ok(out)
}
