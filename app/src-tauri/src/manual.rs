//! Clarity's manual: Markdown topics in docs/manual, one file per topic,
//! named by its id (`code.queries.md`). A topic's first line is its title
//! (`# Queries`), the sentence under it its summary, and the rest the page.
//! People read the files as they are; the agent gets the list of titles and
//! summaries, and reads a page when it needs one. docs/manual/README.md lists
//! the topics in the same order for people.

/// Every topic, in the order of the manual. A topic file that is not listed
/// here fails the tests, and so does a listed one that the index leaves out.
const TOPICS: [(&str, &str); 33] = [
    ("start.overview", include_str!("../../../docs/manual/start.overview.md")),
    ("start.first-project", include_str!("../../../docs/manual/start.first-project.md")),
    ("project.file", include_str!("../../../docs/manual/project.file.md")),
    ("project.run-build", include_str!("../../../docs/manual/project.run-build.md")),
    ("setup.tools", include_str!("../../../docs/manual/setup.tools.md")),
    ("design.windows", include_str!("../../../docs/manual/design.windows.md")),
    ("design.parts", include_str!("../../../docs/manual/design.parts.md")),
    ("design.frame", include_str!("../../../docs/manual/design.frame.md")),
    ("design.custom-parts", include_str!("../../../docs/manual/design.custom-parts.md")),
    ("design.sound", include_str!("../../../docs/manual/design.sound.md")),
    ("design.text", include_str!("../../../docs/manual/design.text.md")),
    ("code.language", include_str!("../../../docs/manual/code.language.md")),
    ("code.handlers", include_str!("../../../docs/manual/code.handlers.md")),
    ("code.functions", include_str!("../../../docs/manual/code.functions.md")),
    ("code.queries", include_str!("../../../docs/manual/code.queries.md")),
    ("code.data-events", include_str!("../../../docs/manual/code.data-events.md")),
    ("code.libraries", include_str!("../../../docs/manual/code.libraries.md")),
    ("code.engines", include_str!("../../../docs/manual/code.engines.md")),
    ("data.entities", include_str!("../../../docs/manual/data.entities.md")),
    ("data.migrations", include_str!("../../../docs/manual/data.migrations.md")),
    ("data.sources", include_str!("../../../docs/manual/data.sources.md")),
    ("data.pictures-files", include_str!("../../../docs/manual/data.pictures-files.md")),
    ("languages", include_str!("../../../docs/manual/languages.md")),
    ("themes", include_str!("../../../docs/manual/themes.md")),
    ("stacks.rust", include_str!("../../../docs/manual/stacks.rust.md")),
    ("stacks.csharp", include_str!("../../../docs/manual/stacks.csharp.md")),
    ("stacks.go", include_str!("../../../docs/manual/stacks.go.md")),
    ("stacks.c", include_str!("../../../docs/manual/stacks.c.md")),
    ("stacks.cpp", include_str!("../../../docs/manual/stacks.cpp.md")),
    ("ai.agent", include_str!("../../../docs/manual/ai.agent.md")),
    ("ai.connections", include_str!("../../../docs/manual/ai.connections.md")),
    ("vault", include_str!("../../../docs/manual/vault.md")),
    ("app.keys", include_str!("../../../docs/manual/app.keys.md")),
];

/// A page of the manual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topic {
    pub id: &'static str,
    /// The first heading, without its `#`.
    pub title: &'static str,
    /// The sentence under the title.
    pub summary: &'static str,
    /// The whole page, as written.
    pub text: &'static str,
}

fn read(id: &'static str, text: &'static str) -> Topic {
    let mut lines = text.lines();
    let title = lines.next().unwrap_or_default().trim_start_matches('#').trim();
    let summary = lines.map(str::trim).find(|l| !l.is_empty()).unwrap_or_default();
    Topic { id, title, summary, text }
}

/// Every topic, in the order of the manual.
pub fn topics() -> Vec<Topic> {
    TOPICS.iter().map(|(id, text)| read(id, text)).collect()
}

/// The topic `id`.
pub fn topic(id: &str) -> Option<Topic> {
    TOPICS.iter().find(|(name, _)| *name == id.trim()).map(|(id, text)| read(id, text))
}

/// A topic as the manual's window lists it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct ManualEntry {
    pub id: String,
    pub title: String,
    pub summary: String,
}

/// Every topic, as the manual's window lists it.
pub fn entries() -> Vec<ManualEntry> {
    topics().into_iter().map(|t| ManualEntry { id: t.id.into(), title: t.title.into(), summary: t.summary.into() }).collect()
}

/// The language reference, which the manual's pages link to.
const REFERENCE: &str = include_str!("../../../docs/LANGUAGE.md");

/// A page for the manual's window: a topic, or the language reference (`reference`).
pub fn page(id: &str) -> Result<String, String> {
    if id.trim() == "reference" {
        return Ok(REFERENCE.to_string());
    }
    topic(id).map(|t| t.text.to_string()).ok_or_else(|| format!("the manual has no page {}", id.trim()))
}

/// The topics, for the manual's window.
#[tauri::command]
pub fn manual_topics() -> Vec<ManualEntry> {
    entries()
}

/// A page of the manual, for its window.
#[tauri::command]
pub fn manual_page(id: String) -> Result<String, String> {
    page(&id)
}

/// Which page the manual's window shows, and the heading on it, if any.
#[derive(Debug, Clone, serde::Serialize)]
struct Shown {
    topic: String,
    anchor: Option<String>,
}

/// A topic id or a heading's anchor as it may go into the window's address:
/// letters, digits, dots and hyphens.
fn plain_id(id: &str) -> String {
    id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-').collect()
}

/// Opens the manual's window on `topic`, at the heading `anchor` when one is
/// given, or brings it forward and turns it there. It is a window of its own
/// beside Clarity's, so a page stays in view while you work; it closes with
/// Clarity.
#[tauri::command]
pub async fn open_manual(app: tauri::AppHandle, topic: String, anchor: Option<String>) -> Result<(), String> {
    use tauri::{Emitter, Manager};
    let shown = Shown { topic: plain_id(&topic), anchor: anchor.as_deref().map(plain_id).filter(|a| !a.is_empty()) };
    if let Some(window) = app.get_webview_window("manual") {
        window.unminimize().ok();
        window.show().map_err(|err| err.to_string())?;
        window.set_focus().map_err(|err| err.to_string())?;
        return window.emit("manual-show", shown).map_err(|err| err.to_string());
    }
    let at = shown.anchor.as_ref().map(|a| format!("&anchor={a}")).unwrap_or_default();
    let url = format!("index.html?window=manual&topic={}{at}", shown.topic);
    tauri::WebviewWindowBuilder::new(&app, "manual", tauri::WebviewUrl::App(url.into()))
        .title("Clarity manual")
        .inner_size(860.0, 720.0)
        .min_inner_size(480.0, 360.0)
        .devtools(false)
        .build()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

/// The manual as a list for the agent: one line per topic with its id,
/// title and summary.
pub fn index() -> String {
    topics().iter().map(|t| format!("- {}: {}. {}\n", t.id, t.title, t.summary)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn folder() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual")
    }

    #[test]
    fn every_topic_file_is_in_the_manual_and_the_index() {
        let mut files: Vec<String> = std::fs::read_dir(folder())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".md") && n != "README.md")
            .map(|n| n.trim_end_matches(".md").to_string())
            .collect();
        files.sort();
        let mut listed: Vec<String> = TOPICS.iter().map(|(id, _)| id.to_string()).collect();
        listed.sort();
        assert_eq!(files, listed, "docs/manual and the list of topics differ");

        // The index for people names every topic, in the same order.
        let readme = std::fs::read_to_string(folder().join("README.md")).unwrap();
        // Its list entries, `- [Title](id.md): summary`, not the links of its introduction.
        let linked: Vec<&str> = readme
            .lines()
            .filter_map(|l| l.strip_prefix("- ["))
            .filter_map(|l| l.split("](").nth(1))
            .filter_map(|l| l.split(".md)").next())
            .collect();
        let order: Vec<&str> = TOPICS.iter().map(|(id, _)| *id).collect();
        assert_eq!(linked, order, "docs/manual/README.md must link every topic, in the manual's order");
    }

    #[test]
    fn every_topic_has_a_title_and_a_summary() {
        for t in topics() {
            assert!(t.text.starts_with("# "), "{} starts with its title", t.id);
            assert!(!t.title.is_empty() && !t.summary.is_empty(), "{} has a title and a summary", t.id);
            assert!(!t.summary.starts_with('#') && t.summary.ends_with('.'), "{}: the summary is one sentence under the title: {}", t.id, t.summary);
        }
    }

    #[test]
    fn every_link_leads_somewhere() {
        for t in topics() {
            for (at, _) in t.text.match_indices("](") {
                let target = t.text[at + 2..].split(')').next().unwrap_or_default();
                if target.starts_with("http") || target.starts_with('#') {
                    continue;
                }
                let file = target.split('#').next().unwrap_or_default();
                assert!(folder().join(file).exists(), "{}: the link {target} leads nowhere", t.id);
            }
        }
    }

    /// Ids reach the window's address as letters, digits, dots and hyphens.
    #[test]
    fn ids_stay_plain() {
        assert_eq!(plain_id("setup.tools"), "setup.tools");
        assert_eq!(plain_id("c-and-c-on-windows&x=<y>"), "c-and-c-on-windowsxy");
    }

    #[test]
    fn the_manual_is_written_in_the_house_style() {
        for t in topics() {
            for bad in ['\u{2014}', '\u{2013}', '\u{2026}', '\u{201C}', '\u{201D}', '\u{2018}', '\u{2019}'] {
                assert!(!t.text.contains(bad), "{} holds {bad:?}", t.id);
            }
        }
    }

    #[test]
    fn the_window_lists_the_topics_and_shows_a_page() {
        let list = entries();
        assert_eq!(list.len(), TOPICS.len());
        assert_eq!(list[0].id, "start.overview");
        assert!(!list[0].title.is_empty() && !list[0].summary.is_empty());
        assert!(page("code.queries").unwrap().starts_with("# "));
        // The language reference is a page of the window too.
        assert!(page("reference").unwrap().starts_with("# The Clarity language"));
        assert!(page("teleport").unwrap_err().contains("teleport"));
    }

    #[test]
    fn a_topic_is_found_by_its_id() {
        assert_eq!(topic("code.queries").unwrap().id, "code.queries");
        assert!(topic("nothing").is_none());
        assert!(index().lines().count() == TOPICS.len() && index().contains("- start.overview: "));
    }
}
