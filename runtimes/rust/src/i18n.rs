//! Translation in the backend (DESIGN 1.5, I1c): text code makes, such as
//! what a function returns, is translated in the language the screen picked
//! before its slots are filled (conformance/translations.json, `code`). The
//! translations come from the model the application is made with; the
//! screen names the language with `set_language`.

use std::collections::BTreeMap;
use std::sync::RwLock;

/// The translations per language, each text as written with its translation.
type Translations = BTreeMap<String, BTreeMap<String, String>>;

static TRANSLATIONS: RwLock<Translations> = RwLock::new(BTreeMap::new());
static LANGUAGE: RwLock<Option<String>> = RwLock::new(None);

/// Takes the translations from the model (`project.translations`).
pub fn load(model: &str) {
    let model: serde_json::Value = serde_json::from_str(model).unwrap_or_default();
    let all: Translations = serde_json::from_value(model["project"]["translations"].clone()).unwrap_or_default();
    if let Ok(mut t) = TRANSLATIONS.write() {
        *t = all;
    }
}

/// The language texts show in, as the screen picked it; none for the texts as written.
pub fn set_language(language: Option<&str>) {
    if let Ok(mut l) = LANGUAGE.write() {
        *l = language.map(str::to_string);
    }
}

/// The language the screen picked; none for the texts as written.
pub fn language() -> Option<String> {
    LANGUAGE.read().ok().and_then(|l| l.clone())
}

/// `text` in the language picked, or as written when it has no translation there.
pub fn translate(text: &str) -> String {
    let language = LANGUAGE.read().ok().and_then(|l| l.clone());
    language
        .and_then(|l| TRANSLATIONS.read().ok().and_then(|t| t.get(&l).and_then(|of| of.get(text).cloned())))
        .unwrap_or_else(|| text.to_string())
}
