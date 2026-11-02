//! Clarity.UI's own words, in the person's language: the value of a key in
//! the translations the program loaded (`clarity_runtime::i18n`), else the
//! English it is written with. A key is always written out whole.

/// The value of `key` in the language shown, else `english`.
pub fn translate(key: &str, english: &str) -> String {
    clarity_runtime::i18n::value(key).unwrap_or_else(|| english.to_string())
}

/// `english` with `{name}` filled with `value`: a word of Clarity.UI's with a value in it.
pub fn fill(text: &str, name: &str, value: &str) -> String {
    let mut slot = String::with_capacity(name.len() + 2);
    slot.push('{');
    slot.push_str(name);
    slot.push('}');
    text.replace(&slot, value)
}

/// A text a program sent as it shows: text as it is, or a text with its key
/// and slots (`{"text", "values", "key"}`) shown in the language shown;
/// nothing for anything else.
pub fn shown(sent: &serde_json::Value) -> String {
    match sent {
        serde_json::Value::String(s) => s.clone(),
        other => serde_json::from_value::<clarity_runtime::ui::Text>(other.clone()).map(|t| t.show()).unwrap_or_default(),
    }
}
