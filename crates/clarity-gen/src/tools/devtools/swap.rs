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

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::jobs::Runnable;

/// What a piece is, and so where its own build is and how builds use it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A native library in the folder of Clarity.UI's: its file stem
    /// (`clarity_audio`), the crate that builds it and its header's folder.
    Native { stem: &'static str, package: &'static str, header: &'static str },
    /// Clarity.UI's browser files: the design, the preview and web programs.
    Browser,
    /// A runtime used from its source folder as it is (`runtimes/rust`).
    Runtime { folder: &'static str },
}

/// A piece of Clarity an app uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    /// The key it is kept under; a native or runtime piece's is also its
    /// library choice's (tools::environment::Choosable).
    pub key: &'static str,
    pub name: &'static str,
    /// What uses it, as people read it.
    pub uses: &'static str,
    /// Its tab is always there.
    pub core: bool,
    pub kind: Kind,
    /// The folders its own build is made from, in the repository.
    pub sources: &'static [&'static str],
}

const fn native(key: &'static str, name: &'static str, uses: &'static str, core: bool, stem: &'static str, package: &'static str, header: &'static str, sources: &'static [&'static str]) -> Piece {
    Piece { key, name, uses, core, kind: Kind::Native { stem, package, header }, sources }
}

/// Every piece, the core ones first.
pub const PIECES: [Piece; 11] = [
    native("clarityUi", "Clarity.UI's library", "Go, C, C++ and Rust desktop programs", true, "clarity_ui", "clarity-ui-ffi", "runtimes/rust-ui-ffi/include", &["runtimes/rust-ui/src", "runtimes/rust-ui-ffi/src", "runtimes/rust/src"]),
    Piece { key: "clarityUiBrowser", name: "Clarity.UI's browser files", uses: "the design, the preview and web programs", core: true, kind: Kind::Browser, sources: &crate::clarity_ui::BROWSER_SOURCES },
    Piece { key: "rustCrates", name: "Rust crates", uses: "Rust programs (clarity-runtime and the crates it takes)", core: true, kind: Kind::Runtime { folder: "runtimes/rust" }, sources: &[] },
    Piece { key: "nugetPackages", name: "NuGet packages", uses: "C# programs (Clarity.Runtime, Clarity.Ui and the rest)", core: true, kind: Kind::Runtime { folder: "runtimes/csharp" }, sources: &[] },
    Piece { key: "goModule", name: "Go module", uses: "Go programs (github.com/petervdpas/Clarity/runtimes/go)", core: true, kind: Kind::Runtime { folder: "runtimes/go" }, sources: &[] },
    native("clarityAudio", "Clarity.Audio's library", "programs that play songs", false, "clarity_audio", "clarity-audio-ffi", "runtimes/rust-audio-ffi/include", &["runtimes/rust-audio/src", "runtimes/rust-audio-ffi/src"]),
    native("clarityVideo", "Clarity.Video's library", "programs that play videos", false, "clarity_video", "clarity-video-ffi", "runtimes/rust-video-ffi/include", &["runtimes/rust-video/src", "runtimes/rust-video-ffi/src"]),
    native("clarityImage", "Clarity.Image's library", "programs with picture functions", false, "clarity_image", "clarity-image-ffi", "runtimes/rust-image-ffi/include", &["runtimes/rust-image/src", "runtimes/rust-image-ffi/src"]),
    native("clarityFormats", "Clarity.Formats' library", "programs reading data formats", false, "clarity_formats", "clarity-formats-ffi", "runtimes/rust-formats-ffi/include", &["runtimes/rust-formats/src", "runtimes/rust-formats-ffi/src"]),
    native("clarityAuth", "Clarity.Auth's library", "programs that sign in", false, "clarity_auth", "clarity-auth-ffi", "runtimes/rust-auth-ffi/include", &["runtimes/rust-auth/src", "runtimes/rust-auth-ffi/src"]),
    native("clarityAppMcp", "Clarity.AppMcp's library", "programs offering commands to AI agents", false, "clarity_app_mcp", "clarity-app-mcp-ffi", "runtimes/rust-app-mcp-ffi/include", &["runtimes/rust-app-mcp/src", "runtimes/rust-app-mcp-ffi/src"]),
];

/// The piece kept under `key`.
pub fn piece(key: &str) -> Option<&'static Piece> {
    PIECES.iter().find(|p| p.key == key)
}

/// The cDdev's swaps: the pieces added as tabs, and those that use the
/// cDdev's own build. Kept in environment.json.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Swaps {
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    added: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    mine: BTreeSet<String>,
}

impl Swaps {
    /// The tabs in order: the core pieces, then those added.
    pub fn tabs(&self) -> Vec<&'static str> {
        PIECES.iter().filter(|p| p.core || self.added.contains(p.key)).map(|p| p.key).collect()
    }

    /// Whether `key` uses the cDdev's own build.
    pub fn mine(&self, key: &str) -> bool {
        self.mine.contains(key)
    }

    /// Adds the tab of a piece that is not core.
    pub fn add(&mut self, key: &str) -> Result<(), String> {
        match piece(key) {
            None => Err(format!("there is no piece {key}")),
            Some(p) if p.core => Err(format!("{} always has its tab", p.name)),
            Some(p) => {
                self.added.insert(p.key.to_string());
                Ok(())
            }
        }
    }

    /// Closes an added tab: its piece is the release again.
    pub fn close(&mut self, key: &str) -> Result<(), String> {
        match piece(key) {
            None => Err(format!("there is no piece {key}")),
            Some(p) if p.core => Err(format!("{} always has its tab", p.name)),
            Some(p) => {
                self.added.remove(p.key);
                self.mine.remove(p.key);
                Ok(())
            }
        }
    }

    /// Swaps a piece of an open tab to the cDdev's own build (`mine`) or back to the release.
    pub fn swap(&mut self, key: &str, mine: bool) -> Result<(), String> {
        let piece = piece(key).ok_or_else(|| format!("there is no piece {key}"))?;
        if !self.tabs().contains(&piece.key) {
            return Err(format!("{} has no tab: add it first", piece.name));
        }
        if mine {
            self.mine.insert(piece.key.to_string());
        } else {
            self.mine.remove(piece.key);
        }
        Ok(())
    }
}

/// A native library's file in `lib/` on system `os`: `libclarity_audio.so`.
fn library_file(stem: &str, os: &str) -> String {
    crate::clarity_ui::library_file(os).replace("clarity_ui", stem)
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

/// The build a tab's Build button runs for `piece`, in the repository:
/// at most two jobs of cargo at once, as the machine can bear.
pub fn build(repo: &Path, piece: &Piece) -> Runnable {
    match piece.kind {
        Kind::Native { package, .. } => Runnable::new("cargo", &["build", "--release", "-p", package], repo).with_env("CARGO_BUILD_JOBS", "2"),
        Kind::Browser => Runnable::new("bash", &["runtimes/rust-ui-browser/build.sh"], repo).with_env("CARGO_BUILD_JOBS", "2"),
        Kind::Runtime { folder: "runtimes/rust" } => Runnable::new("cargo", &["build", "-p", "clarity-runtime"], repo).with_env("CARGO_BUILD_JOBS", "2"),
        // The NuGet packages come from the Release build, into each project's bin/Release.
        Kind::Runtime { folder: "runtimes/csharp" } => Runnable::new("dotnet", &["build", "-c", "Release", "Clarity.slnx"], &repo.join("runtimes/csharp")).with_env("MSBUILDDISABLENODEREUSE", "1"),
        Kind::Runtime { folder } => Runnable::new("go", &["build", "./..."], &repo.join(folder)),
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
    let newest = piece.sources.iter().filter_map(|s| crate::clarity_ui::newest_in(&repo.join(s))).max()?;
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
