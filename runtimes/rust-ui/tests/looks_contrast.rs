//! A button stays readable in every state: its text against its plate, at
//! rest, hovered and pressed, for Clarity's own theme in light and dark, with
//! no style and with a primary and a danger style, and for a theme whose
//! hover sets a soft background (the shop's Day theme) under a primary
//! style. A style that fills the button keeps its colors in a state; the
//! state shades its fill away from its text.

use std::sync::Arc;

use clarity_ui::theme::{contrast, parse_color, ThemeLook};

/// The least contrast a button's text keeps against its plate (WCAG 1.4.3).
const LEAST: f64 = 4.5;

/// A project's theme with a primary and a danger style over Clarity's own, as Clarity resolves it.
fn styled(dark: bool) -> Arc<ThemeLook> {
    let theme = serde_json::json!({
        "dark": dark,
        "styles": {
            "primary": { "background": "@accent", "text": "@on_accent", "border": "@accent", "weight": "bold" },
            "danger": { "background": "@error", "text": "@on_accent", "border": "@error" }
        }
    });
    ThemeLook::over(&theme.to_string(), ThemeLook::clarity(dark))
}

/// The shop's Day theme: a light button whose hover fills it with the soft accent, and a primary style.
fn day() -> Arc<ThemeLook> {
    let theme = serde_json::json!({
        "dark": false,
        "colors": { "background": "#f4f6f9", "panel": "#ffffff", "accent": "#1f5fbf", "accent_soft": "#e4edfb" },
        "parts": {
            "button": { "background": "#ffffff", "text": "#1f5fbf", "border": "#1f5fbf" },
            "button:hover": { "background": "#e4edfb" }
        },
        "styles": { "primary": { "background": "#1f5fbf", "text": "#ffffff", "border": "#1f5fbf", "weight": "bold" } }
    });
    ThemeLook::over(&theme.to_string(), ThemeLook::clarity(false))
}

fn check(name: &str, look: &ThemeLook, style: Option<&str>) {
    for state in [None, Some("hover"), Some("pressed")] {
        let values = look.pressable("button", style, state);
        let fill = values.background.unwrap_or_else(|| look.color("raised"));
        let text = values.text.unwrap_or_else(|| look.color("text"));
        let ratio = contrast(text, fill);
        assert!(ratio >= LEAST, "{name}, style {style:?}, state {state:?}: {ratio:.2} ({text:?} on {fill:?})");
    }
}

#[test]
fn every_built_in_theme_keeps_its_buttons_readable_in_every_state() {
    for dark in [false, true] {
        let name = if dark { "Clarity dark" } else { "Clarity light" };
        check(name, &ThemeLook::clarity(dark), None);
        let look = styled(dark);
        for style in [None, Some("primary"), Some("danger")] {
            check(name, &look, style);
        }
    }
}

#[test]
fn a_hover_that_fills_the_button_keeps_a_primary_button_readable() {
    let look = day();
    check("Day", &look, Some("primary"));
    check("Day", &look, None);
    // A plain button still takes the hover's soft fill.
    assert_eq!(parse_color("#e4edfb"), look.pressable("button", None, Some("hover")).background);
}

#[test]
fn a_styled_button_still_shows_it_is_hovered_and_pressed() {
    let look = day();
    let rest = look.pressable("button", Some("primary"), None).background;
    let hover = look.pressable("button", Some("primary"), Some("hover")).background;
    let pressed = look.pressable("button", Some("primary"), Some("pressed")).background;
    assert_eq!(parse_color("#1f5fbf"), rest);
    assert_ne!(rest, hover);
    assert_ne!(hover, pressed);
    assert_eq!(parse_color("#ffffff"), look.pressable("button", Some("primary"), Some("hover")).text);
}

#[test]
fn contrast_is_measured_as_wcag_says() {
    let black = parse_color("#000000").unwrap();
    let white = parse_color("#ffffff").unwrap();
    assert!((contrast(black, white) - 21.0).abs() < 0.01);
    assert!((contrast(white, white) - 1.0).abs() < 0.01);
}
