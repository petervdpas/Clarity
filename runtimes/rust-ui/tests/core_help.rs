//! A help part under an input says its caption, else the input's error, else
//! the help its field declares (`help @customerVatNumberHelp`), as the C#
//! HelpPart says `Caption() ?? target.Help`.

use clarity_runtime::ui::Text;
use clarity_ui::facts::ModelFacts;
use clarity_ui::parts::FieldFacts;
use clarity_ui::parts::containers::small::HelpPart;
use clarity_ui::testing::{TestHost, part_named, window_of};

const PARTS: &str = r#"<Textbox name="vat" at="1,1" span="4" bind="customer.vatNumber" />
<Help name="vatHelp" at="1,2" span="4" for="vat" />"#;

fn help_text(screen: &clarity_ui::Screen) -> String {
    let id = part_named(screen, "vatHelp");
    screen.element(id).unwrap().as_any().downcast_ref::<HelpPart>().unwrap().text()
}

#[test]
fn a_help_part_says_its_fields_help() {
    let host = TestHost::new();
    host.fields.borrow_mut().insert("customer.vatNumber".into(), FieldFacts { label: "VAT".into(), help: Some("As on the invoice".into()), ..FieldFacts::default() });
    host.binds.borrow_mut().insert("vat".into(), "customer.vatNumber".into());
    let screen = window_of(PARTS, host.clone());
    assert_eq!(help_text(&screen), "As on the invoice");
    // The input's error goes before it.
    host.set_state("vat", |s| s.error = Some(Text::plain("Not a VAT number")));
    assert_eq!(help_text(&screen), "Not a VAT number");
}

#[test]
fn the_model_carries_a_fields_help() {
    let model = r#"{"project": {"files": [{"pages": [{"name": {"name": "Card"}, "data": [{"name": {"name": "customer"}, "of": {"kind": "record", "entity": {"name": "Customer"}}}]}],
        "entities": [{"name": {"name": "Customer"}, "fields": [{"name": {"name": "vatNumber"}, "type": {"kind": "text"}, "modifiers": {"help": "@customerVatNumberHelp"}}]}]}]}}"#;
    let facts = ModelFacts::read(model);
    assert_eq!(facts.field("Card", "customer.vatNumber").and_then(|f| f.help).as_deref(), Some("@customerVatNumberHelp"));
}
