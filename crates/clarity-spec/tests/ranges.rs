//! The slider and the spin box of U1c: inputs for a number between a
//! lowest and a highest value, moved a step at a time. Without `min` and
//! `max` written, they take the field's own; a slider without either runs
//! from 0 to 100, a spin box goes on without end. The step is 1 unless
//! written.

use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, NumberRange, WidgetKind};
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

entity Line
    quantity: integer, min 1, max 99
    discount: decimal(5, 2)
    price: money
    note: text

page LineCard: window
    data line: Line
    slider name discountSlider at 1,1 span 6 bind line.discount min 0 max 50 step 0.5 on change: discountMoved
    slider name quantitySlider at 1,2 span 6 bind line.quantity
    spinbox name quantityBox at 7,1 span 2 bind line.quantity step 2
    spinbox name priceBox at 7,2 span 3 bind line.price min -10
";

const CODE: &str = "
code LineCard
    handler discountMoved
        line.quantity = line.quantity + 1
";

fn project(page: &str) -> Project {
    let text = format!("{page}{CODE}");
    load_keyed([("shop.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str) -> Vec<String> {
    project(page).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn range(min: Option<&str>, max: Option<&str>, step: &str) -> NumberRange {
    NumberRange { min: min.map(str::to_string), max: max.map(str::to_string), step: step.to_string() }
}

#[test]
fn sliders_and_spin_boxes_read_and_write_back_as_written() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let parts = &file.pages[0].widgets;
    let kinds: Vec<WidgetKind> = parts.iter().map(|w| w.props.kind).collect();
    assert_eq!(kinds, vec![WidgetKind::Slider, WidgetKind::Slider, WidgetKind::Spinbox, WidgetKind::Spinbox]);
    let page = &SHOP[SHOP.find("page LineCard").unwrap()..];
    let lines = page.lines().filter(|l| l.starts_with("    ") && !l.trim_start().starts_with("data "));
    for (widget, line) in parts.iter().zip(lines) {
        assert_eq!(widgets::widget_line(&widget.props), line.trim(), "{line}");
    }
    assert_eq!(parts[0].props.min.as_deref(), Some("0"));
    assert_eq!(parts[0].props.step.as_deref(), Some("0.5"));
    assert_eq!(widgets::widget_for(&clarity_spec::model::FieldType::Integer), WidgetKind::Number, "a number field still gets a number input");
}

#[test]
fn the_range_is_as_written_else_the_fields_own() {
    let p = project(SHOP);
    let found: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(found.is_empty(), "{found:#?}");
    let entity = &p.files[0].entities[0];
    let parts = &p.files[0].pages[0].widgets;
    let field = |name: &str| Some(&entity.field(name).unwrap().modifiers);
    assert_eq!(widgets::number_range(&parts[0].props, field("discount")), range(Some("0"), Some("50"), "0.5"));
    assert_eq!(widgets::number_range(&parts[1].props, field("quantity")), range(Some("1"), Some("99"), "1"), "the field's own");
    assert_eq!(widgets::number_range(&parts[2].props, field("quantity")), range(Some("1"), Some("99"), "2"));
    assert_eq!(widgets::number_range(&parts[3].props, field("price")), range(Some("-10"), None, "1"), "a spin box may go on without end");
    let bare = widgets::new_props(WidgetKind::Slider, 1, 1);
    assert_eq!(widgets::number_range(&bare, None), range(Some("0"), Some("100"), "1"), "a slider runs from 0 to 100");
}

#[test]
fn the_rules_of_a_range() {
    let says = |page: String, wanted: &str| {
        let found = errors(&page);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    says(SHOP.replace("min 0 max 50", "min 50 max 0"), "'min' is below 'max': 50 is not below 0");
    says(SHOP.replace("step 0.5", "step 0"), "'step' is a number above 0");
    says(SHOP.replace("quantityBox at 7,1 span 2 bind line.quantity step 2", "quantityBox at 7,1 span 2 bind line.quantity step 0.5"), "'quantity' holds whole numbers: the step is a whole number");
    says(SHOP.replace("bind line.price min -10", "bind line.note"), "'note' is text; a spinbox shows integer, decimal, money");
    says(SHOP.replace("quantitySlider at 1,2 span 6 bind line.quantity", "quantitySlider at 1,2 span 6 bind line.quantity max 0"), "'min' is below 'max': 1 is not below 0");
    let (_, diagnostics) = parse(&SHOP.replace("    spinbox name quantityBox", "    number name countBox at 10,2 span 2 bind line.quantity step 2\n    spinbox name quantityBox"), "shop.clarity");
    assert!(diagnostics.iter().any(|d| d.message.contains("'step' does not apply to a number")), "{diagnostics:?}");
}
