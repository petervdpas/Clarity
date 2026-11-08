//! What code reads, sets and tells of a text box or memo where the person
//! types (Writer gap 21): the caret, the selected text, and text put in at
//! the caret. Clarity.UI keeps them in the part's values, read as a custom
//! part's are, so every stack reads and sets them alike.

use crate::code::check::TypeInfo;

/// What code reads of where the person types: the caret, as how many
/// characters stand before it (0 at the start), and the selected text.
/// And the address of the link into the application last clicked in its preview (`clarity:fragment/412`), for `on link`.
pub const READS: [(&str, TypeInfo); 3] = [("caret", TypeInfo::Integer), ("selectedText", TypeInfo::Text), ("link", TypeInfo::Text)];

/// What code reads of `name`, or None.
pub fn read(name: &str) -> Option<TypeInfo> {
    READS.iter().find(|(n, _)| *n == name).map(|(_, ty)| ty.clone())
}

/// Whether code sets it too: the caret, which moves there and selects nothing.
pub fn settable(name: &str) -> bool {
    name == "caret"
}

/// What it says before the person typed: 0 or empty.
pub fn read_fallback(name: &str) -> &'static str {
    match read(name) {
        Some(TypeInfo::Integer) => "0",
        _ => "\"\"",
    }
}

/// What code tells it: `manuscriptBox.insert(text)` puts the text where the
/// selection is (at the caret with none), as one step Ctrl+Z takes back.
pub const HEARS: [(&str, &[(&str, TypeInfo)]); 1] = [("insert", &[("text", TypeInfo::Text)])];

/// The values a hear takes, or None when it hears no such thing.
pub fn hear(name: &str) -> Option<&'static [(&'static str, TypeInfo)]> {
    HEARS.iter().find(|(n, _)| *n == name).map(|(_, params)| *params)
}
