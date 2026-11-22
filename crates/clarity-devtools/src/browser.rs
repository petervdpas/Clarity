//! Whether the browser files the IDE's design and preview draw with are
//! there and as new as Clarity.UI's source (the DevTools signal for them).

use std::path::Path;

use clarity_gen_core::clarity_ui::{browser_complete, repository_browser_folder};

/// Why the browser files Clarity's repository built are behind its source,
/// with what to run: the DevTools signal for the browser files
/// (swap::behind). None while they are as new as the source, or are
/// not built, which `repository_browser` says.
pub fn stale_browser(repo: &Path) -> Option<String> {
    let piece = crate::swap::piece("clarityUiBrowser")?;
    crate::swap::built_at(repo, piece, std::env::consts::OS)?;
    crate::swap::behind(repo, piece, std::env::consts::OS)
}

/// How the browser files the IDE's design and preview draw with stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BrowserFiles {
    /// All there, and as new as the source.
    Fine,
    /// Not there, or not all of them: nothing draws.
    Missing,
    /// The repository's build, older than Clarity.UI's source: what changed since does not show.
    Stale,
}

/// How the browser files in `folder` (the one the preview serves from, none
/// when none was found) stand, for a Clarity run from its repository `repo`
/// or an installed one (`repo` none, which has no source to be behind).
pub fn browser_files_in(folder: Option<&Path>, repo: Option<&Path>) -> BrowserFiles {
    let Some(folder) = folder.filter(|f| browser_complete(f)) else { return BrowserFiles::Missing };
    match repo {
        Some(repo) if folder == repository_browser_folder(repo) && stale_browser(repo).is_some() => BrowserFiles::Stale,
        _ => BrowserFiles::Fine,
    }
}

/// How the browser files the preview serves stand on this machine, found
/// again at each call (a folder deleted or built since counts).
pub fn browser_files(repo: Option<&Path>) -> BrowserFiles {
    browser_files_in(clarity_gen_core::web_browser::local_folder(repo).ok().as_deref(), repo)
}

