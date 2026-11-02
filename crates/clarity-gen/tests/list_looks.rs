//! A list's look and its cards' (DESIGN 1.5): they come from the theme,
//! alike on every stack.

/// A list's and a card's look come from the theme (`part list`, `part card`,
/// `part card_line`, with `hover` and `selected`), alike on every stack.
#[test]
fn a_list_and_its_cards_look_as_the_theme_says() {
    let theme = "module m\n\ntheme Shop\n    part card background panel, border accent, radius 10\n    part card selected background success\n    part card_line text warning\n    part list selected background error_soft\n";
    let (file, diagnostics) = clarity_spec::parser::parse(theme, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let css = clarity_gen::web::theme_css(&file.themes[0]);
    for line in ["--card-bg: var(--panel)", "--card-radius: 10px", "--card-selected-bg: var(--success)", "--card-line-text: var(--warning)", "--list-selected-bg: var(--error-soft)"] {
        assert!(css.contains(line), "{line} in {css}");
    }
    let base = clarity_gen::base_css();
    for line in ["--list-bg:", "--list-selected-bg:", "--card-bg:", "--card-selected-bg:", "--card-line-text:"] {
        assert!(base.contains(line), "Clarity's own theme sets {line}: {base}");
    }
}
