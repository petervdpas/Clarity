//! A form's label column (U1b): `labels 3` on a form gives each input in it
//! a label in the first 3 columns of its row, showing its field's label
//! (`customerNameLabel`), as a part of its own named after the input
//! (`nameBoxLabel`). A checkbox, a switch and a toggle label themselves.

use clarity_spec::keyed::load_keyed;
use clarity_spec::model::Widget;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Project, Severity, load_sources};

const SHOP: &str = "module shop

entity Customer
    name: text(80), required, label \"Full name\"
    city: text
    vip: bool

page CustomerCard: window
    data customer: Customer
    form name customerForm at 1,1 span 12 rows 4 bind customer labels 3
        textbox name nameBox at 4,1 span 9 bind customer.name
        textbox name cityBox at 4,2 span 9 bind customer.city
        checkbox \"VIP\" name vipBox at 4,3 span 3 bind customer.vip
";

const CODE: &str = "
code CustomerCard
    handler hideCity
        cityBoxLabel.visible = false
";

fn project(page: &str) -> Project {
    let text = format!("{page}{CODE}");
    load_keyed([("shop.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str) -> Vec<String> {
    project(page).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn made_labels(project: &Project) -> Vec<&Widget> {
    project.files[0].pages[0].widgets[0].children.iter().filter(|w| w.props.made).collect()
}

#[test]
fn a_form_writes_its_label_column() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let form = &file.pages[0].widgets[0];
    assert_eq!(form.props.labels, Some(3));
    assert_eq!(widgets::widget_line(&form.props), "form name customerForm at 1,1 span 12 rows 4 bind customer labels 3");
    assert!(form.children.iter().all(|w| !w.props.made), "the parser reads only what is written");
}

#[test]
fn each_input_gets_a_label_part_named_after_it_showing_its_fields_label() {
    let p = project(SHOP);
    let found: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(found.is_empty(), "code finds the made label by its name: {found:#?}");
    let labels = made_labels(&p);
    assert_eq!(labels.len(), 2, "a checkbox labels itself");
    let name = labels[0];
    assert_eq!(name.props.kind, WidgetKind::Label);
    assert_eq!(name.props.name.as_deref(), Some("nameBoxLabel"));
    assert_eq!(name.props.for_part.as_deref(), Some("nameBox"));
    assert_eq!(name.props.caption.as_deref(), Some("@customerNameLabel"), "the field's own label key");
    assert_eq!((name.props.col, name.props.row, name.props.width, name.props.height), (1, 1, 3, 1));
    assert_eq!(labels[1].props.name.as_deref(), Some("cityBoxLabel"));
    assert_eq!(labels[1].props.caption.as_deref(), Some("@customerCityLabel"));
    assert_eq!(labels[1].props.row, 2);
    // Without keys yet, the label is the field's label as written, or made from its name.
    let unkeyed = load_sources([("shop.clarity", format!("{SHOP}{CODE}").as_str())]);
    let captions: Vec<Option<&str>> = made_labels(&unkeyed).iter().map(|w| w.props.caption.as_deref()).collect();
    assert_eq!(captions, vec![Some("Full name"), Some("City")]);
}

#[test]
fn the_rules_of_a_label_column() {
    let says = |page: String, wanted: &str| {
        let found = errors(&page);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    says(SHOP.replace("cityBox at 4,2 span 9", "cityBox at 2,2 span 9"), "'cityBox' starts at column 4 or later: its label takes the columns before");
    says(SHOP.replace("labels 3", "labels 12"), "a form's label column leaves room for its inputs: 'labels' is 1 to 11 here");
    says(
        SHOP.replace("        checkbox", "        label \"Town\" name townLabel at 1,4 span 3 for cityBox\n        checkbox"),
        "the form labels 'cityBox' itself; remove this label",
    );
    says(SHOP.replace("    form name", "    text name cityBoxLabel at 1,6 span 3\n    form name"), "duplicate part name 'cityBoxLabel'");
}

/// A checkbox, a switch or a toggle without a caption of its own shows its
/// field's label as its caption, so nothing in a labelled form goes unnamed.
#[test]
fn a_checkbox_without_a_caption_shows_its_fields_label() {
    let page = SHOP.replace("    vip: bool\n", "    vip: bool, label \"Valued customer\"\n").replace("        checkbox \"VIP\" name vipBox", "        checkbox name vipBox");
    let p = load_sources([("shop.clarity", format!("{page}{CODE}").as_str())]);
    let vip = p.files[0].pages[0].widgets[0].children.iter().find(|w| w.props.name.as_deref() == Some("vipBox")).unwrap();
    assert_eq!(vip.props.caption.as_deref(), Some("Valued customer"));
    // One with a caption keeps its own.
    let p = load_sources([("shop.clarity", format!("{SHOP}{CODE}").as_str())]);
    let vip = p.files[0].pages[0].widgets[0].children.iter().find(|w| w.props.name.as_deref() == Some("vipBox")).unwrap();
    assert_eq!(vip.props.caption.as_deref(), Some("VIP"));
}
