//! A theme as Clarity.UI reads it: Clarity's own, the same files every
//! version carries, and a project's theme on top of it (conformance/themes.json).

use clarity_ui::theme::{parse_color, ThemeLook};
use skia_safe::Color;

fn c(written: &str) -> Color {
    parse_color(written).unwrap()
}

#[test]
fn clarity_s_own_theme_is_carried_for_light_and_dark() {
    let light = ThemeLook::clarity(false);
    let dark = ThemeLook::clarity(true);
    assert!(!light.dark());
    assert!(dark.dark());
    assert_eq!(c("#1f5fbf"), light.color("accent"));
    assert_eq!(c("#6fa3f2"), dark.color("accent"));
    assert_eq!(14.0, light.size("text"));
    assert_eq!(6.0, light.radius("button"));
    assert_eq!(1.0, light.border());
    assert_eq!(4.0, light.padding());
}

#[test]
fn a_part_s_look_in_a_state_falls_back_to_the_part_s_own() {
    let look = ThemeLook::clarity(false);
    let hover = look.part("button", Some("hover"));
    assert_eq!(Some(c("#1f5fbf")), hover.border);
    // What the hover leaves out is the button's.
    assert_eq!(Some(c("#f6f7f9")), hover.background);
    assert_eq!(Some(c("#1d2330")), hover.text);
    assert_eq!(None, look.part("no_such_part", None).background);
}

#[test]
fn a_part_s_state_look_is_only_what_the_state_sets() {
    let look = ThemeLook::clarity(false);
    let pressed = look.part_own("button", Some("pressed"));
    assert_eq!(Some(look.color("accent_soft")), pressed.background);
    assert_eq!(None, pressed.radius);
    assert_eq!(clarity_ui::theme::LookValues::default(), look.part_own("button", None));
}

#[test]
fn a_project_s_theme_changes_only_what_it_sets() {
    let cases: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/themes.json")).unwrap()).unwrap();
    let first = &cases["resolves"][0];
    let look = ThemeLook::over(&first["gives"].to_string(), ThemeLook::clarity(first["dark"].as_bool().unwrap()));
    assert_eq!(c("#2f6fdf"), look.color("accent"));
    assert_eq!(c("#f5c400"), look.color("brand"));
    // What it leaves out is Clarity's own.
    assert_eq!(c("#eef0f3"), look.color("background"));
    assert_eq!(Some(c("#1d2330")), look.style("loud").text);
    assert_eq!(Some(c("#f5c400")), look.style("loud").background);
    assert_eq!("Inter", look.font("text"));
    assert_eq!(6.0, look.radius("button"));
}

#[test]
fn a_role_the_theme_does_not_know_is_the_text_s_color() {
    let look = ThemeLook::clarity(false);
    assert_eq!(look.color("text"), look.color("no_such_role"));
}

