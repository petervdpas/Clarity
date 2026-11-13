//! Clarity's manual: Markdown topics in manual, one file per topic,
//! named by its id (`code.queries.md`). A topic's first line is its title
//! (`# Queries`), the sentence under it its summary, and the rest the page.
//! People read the files as they are; the agent gets the list of titles and
//! summaries, and reads a page when it needs one. manual/README.md lists
//! the topics in the same order for people.

pub use clarity_tools::manual::{page, topics};

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
        // Hidden on a dark ground until the page has drawn itself (main.ts), so it never flashes white.
        .visible(false)
        .background_color(tauri::window::Color(0x15, 0x18, 0x1e, 0xff))
        .build()
        .map(|window| {
            // Closing it only hides it, so the next F1 shows it at once.
            let hidden = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = hidden.hide();
                }
            });
            show_later(window);
        })
        .map_err(|err| err.to_string())
}

/// Shows `window` after a while if its page has not shown it by then, so
/// a page that fails to load still leaves a window to see.
pub fn show_later(window: tauri::WebviewWindow) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(4));
        let _ = window.show();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ids reach the window's address as letters, digits, dots and hyphens.
    #[test]
    fn ids_stay_plain() {
        assert_eq!(plain_id("setup.tools"), "setup.tools");
        assert_eq!(plain_id("c-and-c-on-windows&x=<y>"), "c-and-c-on-windowsxy");
    }

    #[test]
    fn the_window_lists_the_topics_and_shows_a_page() {
        let list = entries();
        assert_eq!(list.len(), topics().len());
        assert_eq!(list[0].id, "start.overview");
        assert!(!list[0].title.is_empty() && !list[0].summary.is_empty());
        assert!(page("code.queries").unwrap().starts_with("# "));
        assert!(page("teleport").unwrap_err().contains("teleport"));
    }
}
