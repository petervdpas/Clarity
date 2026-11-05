//! The splash screen: a small borderless window in the middle of the screen
//! from the moment Clarity starts until its own window shows, as C++
//! Builder greets its users. It shows Clarity's icon (the installers'
//! drawing, icons/icon.svg), its name and version, and what is loading. Its
//! page (public/splash.html) needs none of the IDE's code, so it shows at
//! once; it stays at least a moment, so it never only flashes.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Manager};

/// Its window's label.
const LABEL: &str = "splash";

/// The shortest time it shows, so it is read rather than seen flash by.
const AT_LEAST: Duration = Duration::from_millis(1200);

/// The icon, as the installers draw it.
const LOGO: &str = include_str!("../icons/icon.svg");

/// When the splash opened, while it is open.
#[derive(Default)]
pub struct SplashState {
    opened: Mutex<Option<Instant>>,
}

/// What the splash page shows before anything is loaded.
#[derive(Serialize)]
struct Shown<'a> {
    title: &'a str,
    /// The version, as the title says it after the name: `0.9.0`, `0.9.0-dev`, `(development)`.
    version: &'a str,
    logo: &'a str,
    status: &'a str,
}

/// The script that hands the page what it shows, before the page runs.
fn handed(title: &str, version: &str) -> String {
    let shown = serde_json::to_string(&Shown { title, version, logo: LOGO, status: "Starting" }).unwrap_or_else(|_| "{}".into());
    format!("window.__clarity_splash = {shown};")
}

/// Opens the splash screen; `title` is Clarity's name and version, as its window's title says them, and `version` the version alone.
pub fn open(app: &AppHandle, title: &str, version: &str) {
    let built = tauri::WebviewWindowBuilder::new(app, LABEL, tauri::WebviewUrl::App("splash.html".into()))
        .title(title)
        .inner_size(560.0, 240.0)
        .resizable(false)
        .decorations(false)
        .center()
        .skip_taskbar(true)
        .always_on_top(true)
        .focused(false)
        .devtools(false)
        .background_color(tauri::window::Color(0x15, 0x18, 0x1e, 0xff))
        .initialization_script(handed(title, version))
        .build();
    if built.is_ok()
        && let Ok(mut opened) = app.state::<SplashState>().opened.lock()
    {
        *opened = Some(Instant::now());
    }
}

/// Says what is loading now, on the splash screen while it is open.
pub fn status(app: &AppHandle, text: &str) {
    // The page needs none of Tauri's script: it is handed the text as a call of its own.
    if let Some(window) = app.get_webview_window(LABEL) {
        let text = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into());
        let _ = window.eval(format!("window.__clarity_splash_status && window.__clarity_splash_status({text})"));
    }
}

/// Closes the splash screen once it has shown a moment; Clarity's own window shows.
pub fn done(app: &AppHandle) {
    let Some(opened) = app.state::<SplashState>().opened.lock().ok().and_then(|mut o| o.take()) else { return };
    let app = app.clone();
    std::thread::spawn(move || {
        let shown = opened.elapsed();
        if shown < AT_LEAST {
            std::thread::sleep(AT_LEAST - shown);
        }
        if let Some(window) = app.get_webview_window(LABEL) {
            let _ = window.destroy();
        }
    });
}

/// Clarity's window has drawn itself and shows: the splash screen goes.
#[tauri::command]
pub fn splash_done(app: AppHandle) {
    done(&app);
}

/// The IDE says what it loads, on the splash screen while it is open.
#[tauri::command]
pub fn splash_status(app: AppHandle, text: String) {
    status(&app, &text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_gets_the_title_the_icon_and_a_first_status() {
        let script = handed("Clarity IDE 0.9.0", "0.9.0");
        let json = script.trim_start_matches("window.__clarity_splash = ").trim_end_matches(';');
        let shown: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(shown["title"], "Clarity IDE 0.9.0");
        assert_eq!(shown["version"], "0.9.0", "the name stands above it");
        assert!(shown["logo"].as_str().unwrap().starts_with("<svg"), "the installers' icon");
        assert_eq!(shown["status"], "Starting");
    }
}
