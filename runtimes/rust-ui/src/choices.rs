//! What the person chose to see the program in: its language and its theme
//! (DESIGN 1.5), picked with the `language` and `theme` targets of a menu
//! or the settings, and kept for the next start in a small file of the
//! program's own (`choices.json`), as the web screen keeps them in the
//! browser's storage. The languages offered are the one the texts are
//! written in, then those the project translates; the themes are the
//! project's, else Clarity's own. Whoever shows something of them (the
//! window stack, the screen's look) reads `changes` to know they moved.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::app_files::AppFiles;
use crate::theme::ThemeLook;

/// A language or theme offered: its name (`nl`, `Dark`), its label as shown, and whether it is the one shown now.
#[derive(Debug, Clone, PartialEq)]
pub struct Offer {
    pub name: String,
    pub label: String,
    pub current: bool,
}

/// A theme the program offers: its name, its label as written (`@darkTheme`) and the name its files go by (`dark`).
#[derive(Debug, Clone, PartialEq)]
struct ThemeOffer {
    name: String,
    label: String,
    file: String,
}

#[derive(Default)]
struct Chosen {
    written: String,
    translated: Vec<String>,
    language: Option<String>,
    themes: Vec<ThemeOffer>,
    theme: Option<String>,
    folder: Option<PathBuf>,
    kept: BTreeMap<String, String>,
    changes: u64,
}

static CHOSEN: Mutex<Option<Chosen>> = Mutex::new(None);

fn with<R>(act: impl FnOnce(&mut Chosen) -> R) -> R {
    let mut guard = CHOSEN.lock().unwrap_or_else(|p| p.into_inner());
    act(guard.get_or_insert_with(Chosen::default))
}

const LANGUAGE_KEY: &str = "language";
const THEME_KEY: &str = "theme";

/// Starts from the program's model: the languages and themes it offers, and
/// what the person chose in an earlier run (kept in `folder`; none keeps
/// choices for this run only).
pub fn start(files: &AppFiles, folder: Option<PathBuf>) {
    let model: Value = serde_json::from_str(files.model).unwrap_or_default();
    let project = &model["project"];
    let written = project["locale"]["language"].as_str().unwrap_or("en").to_string();
    let mut translated: Vec<String> = project["translations"].as_object().map(|t| t.keys().filter(|k| **k != written).cloned().collect()).unwrap_or_default();
    translated.sort();
    let themes = theme_offers(&model, files);
    let kept = folder.as_ref().map(|f| read_kept(&f.join("choices.json"))).unwrap_or_default();
    with(|c| {
        c.written = written;
        c.translated = translated;
        c.themes = themes;
        c.folder = folder;
        c.kept = kept;
        c.language = None;
        c.theme = None;
        // A language kept from an earlier run shows while the project still has it.
        if let Some(stored) = c.kept.get(LANGUAGE_KEY).cloned() {
            if c.translated.contains(&stored) {
                c.language = Some(stored);
            }
        }
        let stored = c.kept.get(THEME_KEY).cloned();
        c.theme = stored.filter(|s| c.themes.iter().any(|t| t.name == *s)).or_else(|| c.themes.first().map(|t| t.name.clone()));
        c.changes += 1;
    });
    clarity_runtime::i18n::set_language(language_picked().as_deref());
}

/// The themes the model offers, each with the files the program carries for it; none when it carries none of its own.
fn theme_offers(model: &Value, files: &AppFiles) -> Vec<ThemeOffer> {
    let names: Vec<String> = model["project"]["themes"].as_array().map(|a| a.iter().filter_map(|n| n.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let mut declared: BTreeMap<String, (String, String)> = BTreeMap::new();
    for file in model["project"]["files"].as_array().into_iter().flatten() {
        for theme in file["themes"].as_array().into_iter().flatten() {
            let name = theme["name"]["name"].as_str().unwrap_or_default().to_string();
            let label = theme["label"].as_str().unwrap_or_default().to_string();
            let class = theme["class"].as_str().unwrap_or_default();
            declared.insert(name, (label, class.strip_prefix("theme-").unwrap_or(class).to_string()));
        }
    }
    names
        .into_iter()
        .map(|name| {
            let (label, file) = declared.get(&name).cloned().unwrap_or_else(|| (name.clone(), kebab(&name)));
            ThemeOffer { label: if label.is_empty() { name.clone() } else { label }, name, file }
        })
        .filter(|t| files.themes.iter().any(|(n, _)| n.starts_with(&format!("{}.", t.file))))
        .collect()
}

/// A theme's name as its files go by: `NightOwl` is `night-owl`.
fn kebab(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 && !out.ends_with('-') {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out
}

fn read_kept(file: &std::path::Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// Keeps one choice: read, changed and written back, so other keys stay as they were.
fn keep(c: &mut Chosen, key: &str, value: &str) {
    c.kept.insert(key.to_string(), value.to_string());
    let Some(folder) = c.folder.clone() else { return };
    let file = folder.join("choices.json");
    let mut all = read_kept(&file);
    all.insert(key.to_string(), value.to_string());
    if std::fs::create_dir_all(&folder).is_ok() {
        if let Ok(text) = serde_json::to_string_pretty(&all) {
            // A choice that cannot be kept still shows in this run.
            let _ = std::fs::write(&file, text);
        }
    }
}

/// How often the language or theme changed: what shows them reads them again when it moved.
pub fn changes() -> u64 {
    with(|c| c.changes)
}

/// The language picked over the one the texts are written in; none for the written one.
fn language_picked() -> Option<String> {
    with(|c| c.language.clone())
}

/// The language shown now (`en`, `nl`).
pub fn language() -> String {
    with(|c| c.language.clone().unwrap_or_else(|| if c.written.is_empty() { "en".to_string() } else { c.written.clone() }))
}

/// The languages to switch between, the one the texts are written in first, each named in itself.
pub fn languages() -> Vec<Offer> {
    let now = language();
    with(|c| {
        let written = if c.written.is_empty() { "en".to_string() } else { c.written.clone() };
        std::iter::once(written).chain(c.translated.iter().cloned()).map(|tag| Offer { label: language_name(&tag), current: tag == now, name: tag }).collect()
    })
}

/// Shows the texts in `language` and keeps it for the next start; one the program does not offer is left.
pub fn choose_language(language: &str) {
    let changed = with(|c| {
        let written = if c.written.is_empty() { "en" } else { c.written.as_str() };
        let picked = if language == written {
            None
        } else if c.translated.iter().any(|t| t == language) {
            Some(language.to_string())
        } else {
            return false;
        };
        if picked == c.language {
            return false;
        }
        c.language = picked;
        keep(c, LANGUAGE_KEY, language);
        c.changes += 1;
        true
    });
    if changed {
        clarity_runtime::i18n::set_language(language_picked().as_deref());
    }
}

/// The themes the program offers, the one it starts with first, each labelled in the language shown.
pub fn themes() -> Vec<Offer> {
    let offers = with(|c| c.themes.iter().map(|t| (t.name.clone(), t.label.clone(), c.theme.as_deref() == Some(t.name.as_str()))).collect::<Vec<_>>());
    offers.into_iter().map(|(name, label, current)| Offer { label: shown_label(&label), name, current }).collect()
}

/// A label as shown: a key (`@darkTheme`) by its value in the language shown, else as written.
fn shown_label(label: &str) -> String {
    match label.strip_prefix('@') {
        Some(key) => clarity_runtime::i18n::value(key).unwrap_or_else(|| key.to_string()),
        None => clarity_runtime::i18n::text(label),
    }
}

/// The name of the theme shown; none when the program offers none of its own.
pub fn theme() -> Option<String> {
    with(|c| c.theme.clone())
}

/// Shows the theme named `name` and keeps it for the next start; one the program does not offer is left.
pub fn choose_theme(name: &str) {
    with(|c| {
        if !c.themes.iter().any(|t| t.name == name) || c.theme.as_deref() == Some(name) {
            return;
        }
        c.theme = Some(name.to_string());
        keep(c, THEME_KEY, name);
        c.changes += 1;
    });
}

/// The look of the theme shown, for the system's light or dark: the
/// theme's file for that mode over Clarity's own; Clarity's own when the
/// program carries none, or its file cannot be read.
pub fn look(files: &AppFiles, dark: bool) -> Arc<ThemeLook> {
    let file = with(|c| c.theme.as_ref().and_then(|name| c.themes.iter().find(|t| t.name == *name)).map(|t| t.file.clone()));
    let Some(file) = file else { return files.look(dark) };
    let key = format!("{file}.{}", if dark { "dark" } else { "light" });
    match files.themes.iter().find(|(n, _)| *n == key) {
        Some((_, resolved)) => {
            let look = ThemeLook::over(resolved, ThemeLook::clarity(false));
            if look.dark() { ThemeLook::over(resolved, ThemeLook::clarity(true)) } else { look }
        }
        None => files.look(dark),
    }
}

/// A language named in itself, as people look for it: `nl` is Nederlands; one not known here is its tag.
pub fn language_name(tag: &str) -> String {
    const NAMES: [(&str, &str); 32] = [
        ("ar", "\u{627}\u{644}\u{639}\u{631}\u{628}\u{64a}\u{629}"),
        ("cs", "\u{10c}e\u{161}tina"),
        ("da", "Dansk"),
        ("de", "Deutsch"),
        ("el", "\u{395}\u{3bb}\u{3bb}\u{3b7}\u{3bd}\u{3b9}\u{3ba}\u{3ac}"),
        ("en", "English"),
        ("es", "Espa\u{f1}ol"),
        ("fi", "Suomi"),
        ("fr", "Fran\u{e7}ais"),
        ("fy", "Frysk"),
        ("he", "\u{5e2}\u{5d1}\u{5e8}\u{5d9}\u{5ea}"),
        ("hi", "\u{939}\u{93f}\u{928}\u{94d}\u{926}\u{940}"),
        ("hu", "Magyar"),
        ("id", "Indonesia"),
        ("it", "Italiano"),
        ("ja", "\u{65e5}\u{672c}\u{8a9e}"),
        ("ko", "\u{d55c}\u{ad6d}\u{c5b4}"),
        ("nb", "Norsk bokm\u{e5}l"),
        ("nl", "Nederlands"),
        ("no", "Norsk"),
        ("pl", "Polski"),
        ("pt", "Portugu\u{ea}s"),
        ("ro", "Rom\u{e2}n\u{103}"),
        ("ru", "\u{420}\u{443}\u{441}\u{441}\u{43a}\u{438}\u{439}"),
        ("sk", "Sloven\u{10d}ina"),
        ("sv", "Svenska"),
        ("th", "\u{e44}\u{e17}\u{e22}"),
        ("tr", "T\u{fc}rk\u{e7}e"),
        ("uk", "\u{423}\u{43a}\u{440}\u{430}\u{457}\u{43d}\u{441}\u{44c}\u{43a}\u{430}"),
        ("vi", "Ti\u{1ebf}ng Vi\u{1ec7}t"),
        ("zh", "\u{4e2d}\u{6587}"),
        ("ca", "Catal\u{e0}"),
    ];
    let primary = tag.split(['-', '_']).next().unwrap_or(tag).to_ascii_lowercase();
    NAMES.iter().find(|(t, _)| *t == primary).map(|(_, n)| n.to_string()).unwrap_or_else(|| tag.to_string())
}
