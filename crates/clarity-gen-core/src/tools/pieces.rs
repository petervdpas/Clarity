//! The pieces of Clarity an app uses (Clarity.UI, its browser files, the
//! runtimes' packages, the shared libraries), and which build of each the
//! programs Clarity builds use: the release, or the cDdev's own build
//! (`Swaps`), as DevTools sets it and the generators read it.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};


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
pub const PIECES: [Piece; 12] = [
    native("clarityUi", "Clarity.UI's library", "Go, C, C++ and Rust desktop programs", true, "clarity_ui", "clarity-ui-ffi", "runtimes/rust-ui-ffi/include", &["runtimes/rust-ui/src", "runtimes/rust-ui-ffi/src", "runtimes/rust/src"]),
    Piece { key: "clarityUiBrowser", name: "Clarity.UI's browser files", uses: "the design, the preview and web programs", core: true, kind: Kind::Browser, sources: &crate::clarity_ui::BROWSER_SOURCES },
    Piece { key: "rustCrates", name: "Rust crates", uses: "Rust programs (clarity-runtime and the crates it takes)", core: true, kind: Kind::Runtime { folder: "runtimes/rust" }, sources: &[] },
    Piece { key: "nugetPackages", name: "NuGet packages", uses: "C# programs (Clarity.Runtime, Clarity.Ui and the rest)", core: true, kind: Kind::Runtime { folder: "runtimes/csharp" }, sources: &[] },
    Piece { key: "goModule", name: "Go module", uses: "Go programs (github.com/petervdpas/Clarity/runtimes/go)", core: true, kind: Kind::Runtime { folder: "runtimes/go" }, sources: &[] },
    native("clarityAudio", "Clarity.Audio's library", "programs that play songs", false, "clarity_audio", "clarity-audio-ffi", "runtimes/rust-audio-ffi/include", &["runtimes/rust-audio/src", "runtimes/rust-audio-ffi/src"]),
    native("clarityVideo", "Clarity.Video's library", "programs that play videos", false, "clarity_video", "clarity-video-ffi", "runtimes/rust-video-ffi/include", &["runtimes/rust-video/src", "runtimes/rust-video-ffi/src"]),
    native("clarityImage", "Clarity.Image's library", "programs with picture functions", false, "clarity_image", "clarity-image-ffi", "runtimes/rust-image-ffi/include", &["runtimes/rust-image/src", "runtimes/rust-image-ffi/src"]),
    native("clarityPdf", "Clarity.Pdf's library", "programs that make PDFs", false, "clarity_pdf", "clarity-pdf-ffi", "runtimes/rust-pdf-ffi/include", &["runtimes/rust-pdf/src", "runtimes/rust-pdf-ffi/src"]),
    native("clarityFormats", "Clarity.Formats' library", "programs reading data formats", false, "clarity_formats", "clarity-formats-ffi", "runtimes/rust-formats-ffi/include", &["runtimes/rust-data/src", "runtimes/rust-formats/src", "runtimes/rust-formats-ffi/src"]),
    native("clarityAuth", "Clarity.Auth's library", "programs that sign in", false, "clarity_auth", "clarity-auth-ffi", "runtimes/rust-auth-ffi/include", &["runtimes/rust-auth/src", "runtimes/rust-auth-ffi/src"]),
    native("clarityAppMcp", "Clarity.AppMcp's library", "programs offering commands to AI agents", false, "clarity_app_mcp", "clarity-app-mcp-ffi", "runtimes/rust-app-mcp-ffi/include", &["runtimes/rust-app-mcp/src", "runtimes/rust-app-mcp-ffi/src"]),
];

/// The piece kept under `key`.
pub fn piece(key: &str) -> Option<&'static Piece> {
    PIECES.iter().find(|p| p.key == key)
}

/// The cDdev's swaps: the pieces added as tabs, and those that use the
/// cDdev's own build. Kept in environment.json. A tab shown by itself
/// (marked with a signal) and closed stays away only while Clarity runs;
/// the IDE keeps that.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Swaps {
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    added: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    mine: BTreeSet<String>,
}

impl Swaps {
    /// The tabs kept open, in the pieces' order: those added, and those swapped to the cDdev's own build.
    pub fn tabs(&self) -> Vec<&'static str> {
        PIECES.iter().filter(|p| self.open(p)).map(|p| p.key).collect()
    }

    fn open(&self, p: &Piece) -> bool {
        self.added.contains(p.key) || self.mine.contains(p.key)
    }

    /// The tabs that show, in the pieces' order: those kept open, and those
    /// marked with a signal (`behind`: the own build is behind its source),
    /// which show by themselves while it is; each with whether it shows only
    /// for that.
    pub fn shown(&self, behind: impl Fn(&Piece) -> bool) -> Vec<(&'static Piece, bool)> {
        PIECES
            .iter()
            .filter_map(|p| {
                if self.open(p) {
                    Some((p, false))
                } else {
                    behind(p).then_some((p, true))
                }
            })
            .collect()
    }

    /// Whether `key` uses the cDdev's own build.
    pub fn mine(&self, key: &str) -> bool {
        self.mine.contains(key)
    }

    /// Adds a piece's tab.
    pub fn add(&mut self, key: &str) -> Result<(), String> {
        let p = piece(key).ok_or_else(|| format!("there is no piece {key}"))?;
        self.added.insert(p.key.to_string());
        Ok(())
    }

    /// Closes a piece's tab: its piece is the release again.
    pub fn close(&mut self, key: &str) -> Result<(), String> {
        let p = piece(key).ok_or_else(|| format!("there is no piece {key}"))?;
        self.added.remove(p.key);
        self.mine.remove(p.key);
        Ok(())
    }

    /// Swaps a piece of an open tab to the cDdev's own build (`mine`) or back to the release.
    pub fn swap(&mut self, key: &str, mine: bool) -> Result<(), String> {
        let piece = piece(key).ok_or_else(|| format!("there is no piece {key}"))?;
        if !self.open(piece) {
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

