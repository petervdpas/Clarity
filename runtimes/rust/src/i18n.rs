//! Texts by key in the backend (DESIGN 1.5, I1c): a text code makes, such
//! as what a function returns, takes the value of its key in the language
//! the screen picked, or the default language's when that language leaves
//! it empty (conformance/translations.json, `code`). The values come from
//! the model the application is made with; the screen names the language
//! with `set_language`.

use std::collections::BTreeMap;
use std::sync::RwLock;

/// Each language's values, by key.
type Values = BTreeMap<String, BTreeMap<String, String>>;

struct Languages {
    values: Values,
    /// The language the project is written in.
    default: String,
    /// The language the screen picked; none for the default.
    picked: Option<String>,
}

static LANGUAGES: RwLock<Languages> = RwLock::new(Languages { values: BTreeMap::new(), default: String::new(), picked: None });

/// Takes every language's values from the model (`project.translations`),
/// and the default language (`project.locale.language`).
pub fn load(model: &str) {
    let model: serde_json::Value = serde_json::from_str(model).unwrap_or_default();
    let values: Values = serde_json::from_value(model["project"]["translations"].clone()).unwrap_or_default();
    let default = model["project"]["locale"]["language"].as_str().unwrap_or("en").to_string();
    if let Ok(mut l) = LANGUAGES.write() {
        l.values = values;
        l.default = default;
    }
}

/// The language texts show in, as the screen picked it; none for the default.
pub fn set_language(language: Option<&str>) {
    if let Ok(mut l) = LANGUAGES.write() {
        l.picked = language.map(str::to_string);
    }
}

/// The language the screen picked; none for the default.
pub fn language() -> Option<String> {
    LANGUAGES.read().ok().and_then(|l| l.picked.clone())
}

/// The value of `key` in the language picked, or the default language's
/// when that one leaves it empty; none when no language has the key.
pub fn value(key: &str) -> Option<String> {
    let l = LANGUAGES.read().ok()?;
    let of = |language: &str| l.values.get(language).and_then(|v| v.get(key)).filter(|v| !v.is_empty()).cloned();
    l.picked.as_deref().and_then(of).or_else(|| of(&l.default))
}

/// A text an engine gives: the value of the key it names, or the text
/// itself when it names none.
pub fn text(key_or_text: &str) -> String {
    value(key_or_text).unwrap_or_else(|| key_or_text.to_string())
}
