//! Clarity spec language: lexer, parser, checker and the neutral model.

pub mod check;
pub mod diagnostic;
pub mod edit;
pub mod frame;
pub mod lexer;
pub mod model;
pub mod parser;
pub mod sample;
pub mod span;
pub mod templates;
pub mod widgets;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

pub use diagnostic::{Diagnostic, Severity};
pub use model::Project;
pub use span::Span;

pub const SPEC_EXTENSION: &str = "clarity";

/// A short, stable hash of a source text (64-bit FNV-1a, as hex).
pub fn content_version(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Parses and checks a set of in-memory sources given as (path, text).
pub fn load_sources<'a>(sources: impl IntoIterator<Item = (&'a str, &'a str)>) -> Project {
    let mut files = Vec::new();
    let mut diagnostics = Vec::new();
    for (path, text) in sources {
        let (file, file_diagnostics) = parser::parse(text, path);
        files.push(file);
        diagnostics.extend(file_diagnostics);
    }
    diagnostics.extend(check::check(&files));
    Project { files, diagnostics }
}

/// Loads every `.clarity` file in `dir` (not recursive), sorted by name.
pub fn load_dir(dir: &Path) -> std::io::Result<Project> {
    let paths = spec_files(dir)?;
    let mut sources = Vec::new();
    for path in &paths {
        sources.push((path.display().to_string(), std::fs::read_to_string(path)?));
    }
    Ok(load_sources(
        sources.iter().map(|(p, t)| (p.as_str(), t.as_str())),
    ))
}

pub fn spec_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == SPEC_EXTENSION))
        .collect();
    paths.sort();
    Ok(paths)
}

/// Like `load_dir`, but with `text` in place of the file at `path`. Used to
/// check an edit against the whole project before it is written.
pub fn load_dir_with(dir: &Path, path: &Path, text: &str) -> std::io::Result<Project> {
    let paths = spec_files(dir)?;
    let mut sources = Vec::new();
    for p in &paths {
        let content = if p == path {
            text.to_string()
        } else {
            std::fs::read_to_string(p)?
        };
        sources.push((p.display().to_string(), content));
    }
    Ok(load_sources(
        sources.iter().map(|(p, t)| (p.as_str(), t.as_str())),
    ))
}

/// The first error in `after` that `before` does not have. Positions shift
/// with edits, so errors are compared by file and message.
pub fn new_error<'a>(before: &Project, after: &'a Project) -> Option<&'a Diagnostic> {
    let known: Vec<(&str, &str)> = before
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| (d.path.as_str(), d.message.as_str()))
        .collect();
    after.diagnostics.iter().find(|d| {
        d.severity == Severity::Error && !known.contains(&(d.path.as_str(), d.message.as_str()))
    })
}

/// Everything a preview needs: the checked model, sample rows per entity,
/// the display text of each sample row and the widget catalog.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Preview {
    pub project: Project,
    pub samples: BTreeMap<String, Vec<sample::Row>>,
    pub displays: BTreeMap<String, Vec<String>>,
    pub catalog: Vec<widgets::WidgetInfo>,
    /// The frame regions the Parts panel offers.
    pub frame: Vec<frame::RegionInfo>,
    /// Click targets that are not windows (`quit`, `close`).
    pub commands: Vec<String>,
}

impl Preview {
    pub fn new(project: Project) -> Self {
        let samples = sample::sample_data(&project);
        let displays = sample::display_values(&project);
        Self {
            project,
            samples,
            displays,
            catalog: widgets::catalog(),
            frame: frame::catalog(),
            commands: frame::BUILT_IN_COMMANDS
                .iter()
                .map(|c| c.to_string())
                .collect(),
        }
    }
}
