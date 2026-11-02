//! A text of Clarity's own words carries its key and its English: shown in
//! the person's language when the program has the key, else in English,
//! never as the bare key (as the C# runtime's `new Text(english) { Key = key }`).

use clarity_runtime::ui::Text;

#[test]
fn a_keyed_text_falls_back_to_its_english() {
    let text = Text::keyed("undoChangedSince", "This was changed since, so it cannot be taken back");
    assert_eq!(text.show(), "This was changed since, so it cannot be taken back");
    // A key alone, with nothing to fall back to, is still the key.
    assert_eq!(Text::key("noSuchKeyAnywhere").show(), "noSuchKeyAnywhere");
}

#[test]
fn a_refused_undo_says_why_in_english_without_translations() {
    let refused = clarity_runtime::undo_changed_since();
    assert_eq!(refused.show(), "This was changed since, so it cannot be taken back");
}
