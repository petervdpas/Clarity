//! The color field type of U1c: kept as text `#rrggbb`, picked with the
//! `color` input, which offers a palette the same on every renderer
//! (conformance/colors.json).

use clarity_spec::colors;
use clarity_spec::keyed::load_keyed;
use clarity_spec::model::FieldType;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

entity Product
    name: text(80), required
    tint: color, default \"#3b82f6\"

page ProductCard: window
    data product: Product
    color name tintBox at 1,1 span 4 bind product.tint on change: tintPicked
    text name tintText at 5,1 span 4 bind product.tint
";

const CODE: &str = "
code ProductCard
    handler tintPicked
        if product.tint == \"#000000\"
            product.tint = \"#ffffff\"
";

fn project(page: &str) -> Project {
    let text = format!("{page}{CODE}");
    load_keyed([("shop.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str) -> Vec<String> {
    project(page).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn cases() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../conformance/colors.json")).unwrap()
}

#[test]
fn a_color_field_and_its_input_read_and_write_back() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let tint = file.entities[0].field("tint").unwrap();
    assert_eq!(tint.ty, FieldType::Color);
    let part = &file.pages[0].widgets[0];
    assert_eq!(part.props.kind, WidgetKind::Color);
    assert_eq!(widgets::widget_line(&part.props), "color name tintBox at 1,1 span 4 bind product.tint on change: tintPicked");
    assert_eq!(widgets::widget_for(&FieldType::Color), WidgetKind::Color, "a dropped color field gets a color input");
    let p = project(SHOP);
    let found: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(found.is_empty(), "code reads and sets a color as text: {found:#?}");
}

#[test]
fn a_color_is_read_as_on_every_renderer() {
    for c in cases()["read"].as_array().unwrap() {
        let typed = c["typed"].as_str().unwrap();
        assert_eq!(colors::read(typed).as_deref(), c["color"].as_str(), "{typed:?}");
    }
    let palette: Vec<Vec<String>> = serde_json::from_value(cases()["palette"].clone()).unwrap();
    let ours: Vec<Vec<String>> = colors::PALETTE.iter().map(|row| row.iter().map(|c| c.to_string()).collect()).collect();
    assert_eq!(ours, palette);
}

#[test]
fn the_rules_of_a_color() {
    let says = |page: String, wanted: &str| {
        let found = errors(&page);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    says(SHOP.replace("default \"#3b82f6\"", "default \"blue\""), "a color is written as \"#rrggbb\", as in \"#3b82f6\"");
    says(SHOP.replace("color name tintBox", "textbox name tintBox").replace(" on change: tintPicked", ""), "'tint' is color; a textbox shows text, email, url");
    let code = CODE.replace("product.tint = \"#ffffff\"", "product.tint = \"white\"");
    let found: Vec<String> = load_keyed([("shop.clarity", format!("{SHOP}{code}").as_str())], &[], "en").diagnostics.into_iter().map(|d| d.message).collect();
    assert!(found.iter().any(|m| m == "a color is written as \"#rrggbb\", as in \"#3b82f6\""), "code sets a color as it is kept: {found:#?}");
}

