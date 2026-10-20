//! Forms and text parts (U1a): `form`, `label for`, `help`, `heading`,
//! `text`, `memo`, `password`, `switch` and `radio`, and the properties
//! every input or button can have: placeholder, tooltip, readonly, tab
//! order, shortcut, default and cancel.

use clarity_spec::model::{Orientation, Widget};
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, Category, Event, WidgetKind, widget_line};
use clarity_spec::{Severity, load_sources};

const SHOP: &str = "module shop

enum Kind
    Private
    Business

entity Customer
    name: text(80), required
    kind: Kind
    active: bool
    secret: text(40)
    notes: text, help \"Internal notes\"

entity Note
    text: text

page Card: window
    data customer: Customer
    data other: Customer
    data note_list: list<Note>
    heading \"Customer\" name heading1 at 1,1 span 12
    form \"Details\" name details at 1,2 span 12 rows 6 bind customer on submit: detailsSubmitted
        label \"Name\" name label1 at 1,1 span 3 for name
        textbox name name at 4,1 span 6 bind customer.name placeholder \"Full name\" tooltip \"As on the invoice\" taborder 1
        label \"Kind\" name label2 at 1,2 span 3 for kind
        radio name kind at 4,2 span 6 vertical bind customer.kind
        switch \"Active\" name active at 4,3 span 3 bind customer.active readonly
        password name secret at 4,4 span 4 bind customer.secret
        memo name notes at 4,5 span 6 rows 2 bind customer.notes
        help name help1 at 10,5 span 3 for notes
    text name text1 at 1,8 span 6 bind customer.name
    button \"Save\" name save at 9,9 span 2 key \"Ctrl+S\" default on click: saveClicked
    button \"Close\" name closeButton at 11,9 span 2 cancel on click: closeClicked

code Card
    handler detailsSubmitted
        name.error = \"\"
        if customer.name == \"\"
            name.error = \"Name is required\"
        notes.readonly = true
    handler saveClicked
        save customer
    handler closeClicked
        close
";

fn problems(source: &str) -> Vec<(Severity, String)> {
    load_sources([("shop.clarity", source)])
        .diagnostics
        .into_iter()
        .map(|d| (d.severity, d.message))
        .collect()
}

fn errors(source: &str) -> Vec<String> {
    problems(source)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Error)
        .map(|(_, m)| m)
        .collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(
        messages.iter().any(|m| m.contains(needle)),
        "missing {needle:?} in {messages:#?}"
    );
}

/// The shop source with one line replaced.
fn with(old: &str, new: &str) -> String {
    assert!(SHOP.contains(old), "{old:?} is not in the source");
    SHOP.replace(old, new)
}

fn all_widgets(widgets: &[Widget]) -> Vec<&Widget> {
    let mut out = Vec::new();
    for w in widgets {
        out.push(w);
        out.extend(all_widgets(&w.children));
        for page in &w.pages {
            out.extend(all_widgets(&page.children));
        }
    }
    out
}

fn part<'a>(widgets: &'a [&'a Widget], name: &str) -> &'a Widget {
    widgets
        .iter()
        .find(|w| w.props.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no part {name}"))
}

#[test]
fn the_example_checks_clean() {
    assert_eq!(problems(SHOP), Vec::new());
}

#[test]
fn parses_forms_text_parts_and_their_properties() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let widgets = all_widgets(&file.pages[0].widgets);

    let details = part(&widgets, "details");
    assert_eq!(details.props.kind, WidgetKind::Form);
    assert_eq!(details.props.bind.as_deref(), Some("customer"));
    assert_eq!(details.props.handlers[0].event, Event::Submit);
    assert_eq!(details.children.len(), 8);

    let label = &details.children[0];
    assert_eq!(label.props.kind, WidgetKind::Label);
    assert_eq!(label.props.for_part.as_deref(), Some("name"));

    let name = part(&widgets, "name");
    assert_eq!(name.props.placeholder.as_deref(), Some("Full name"));
    assert_eq!(name.props.tooltip.as_deref(), Some("As on the invoice"));
    assert_eq!(name.props.tab_order, Some(1));

    let kind = part(&widgets, "kind");
    assert_eq!(kind.props.kind, WidgetKind::Radio);
    assert_eq!(kind.props.orientation, Some(Orientation::Vertical));

    assert!(part(&widgets, "active").props.readonly);
    assert_eq!(part(&widgets, "secret").props.kind, WidgetKind::Password);
    assert_eq!(part(&widgets, "notes").props.kind, WidgetKind::Memo);

    let save = part(&widgets, "save");
    assert_eq!(save.props.shortcut.as_deref(), Some("Ctrl+S"));
    assert!(save.props.default_button);
    assert!(part(&widgets, "closeButton").props.cancel);

    let kinds: Vec<WidgetKind> = widgets.iter().map(|w| w.props.kind).collect();
    assert!(kinds.contains(&WidgetKind::Heading));
    assert!(kinds.contains(&WidgetKind::Help));
    assert!(kinds.contains(&WidgetKind::Text));
}

#[test]
fn every_part_line_writes_back_the_same() {
    let (file, _) = parse(SHOP, "shop.clarity");
    for widget in all_widgets(&file.pages[0].widgets) {
        let line = widget_line(&widget.props);
        let source = format!(
            "module shop\npage P: window\n    data customer: Customer\n    {line}\n"
        );
        let (again, diagnostics) = parse(&source, "p.clarity");
        assert!(diagnostics.is_empty(), "{line}: {diagnostics:#?}");
        assert_eq!(again.pages[0].widgets[0].props, widget.props, "{line}");
    }
}

#[test]
fn a_one_row_container_keeps_its_height() {
    let source = "module shop\npage P: window\n    panel name panel1 at 1,1 span 6 rows 1\n";
    let (file, _) = parse(source, "p.clarity");
    let line = widget_line(&file.pages[0].widgets[0].props);
    assert_eq!(line, "panel name panel1 at 1,1 span 6 rows 1");
}

#[test]
fn the_catalog_has_a_text_group_and_picks_inputs_per_type() {
    let text: Vec<WidgetKind> = widgets::catalog()
        .into_iter()
        .filter(|w| w.category == Category::Text)
        .map(|w| w.kind)
        .collect();
    assert_eq!(
        text,
        [WidgetKind::Label, WidgetKind::Help, WidgetKind::Heading, WidgetKind::Text]
    );
    use clarity_spec::model::FieldType;
    assert_eq!(
        widgets::widget_for(&FieldType::Text { max_length: Some(10) }),
        WidgetKind::Textbox
    );
    assert_eq!(widgets::widget_for(&FieldType::Bool), WidgetKind::Checkbox);
}

#[test]
fn a_label_is_only_a_label() {
    let source = with(
        "    text name text1 at 1,8 span 6 bind customer.name",
        "    label name label3 at 1,8 span 6 bind customer.name",
    );
    assert_has(&errors(&source), "a label cannot be bound to data");
}

#[test]
fn for_names_an_input_in_the_window() {
    let unknown = with("span 3 for name", "span 3 for nobody");
    assert_has(&errors(&unknown), "label at 1,1: there is no part 'nobody' in this window");

    let not_input = with("span 3 for name", "span 3 for save");
    assert_has(&errors(&not_input), "label at 1,1: 'save' is a button, not an input");

    let on_input = with("placeholder \"Full name\"", "for kind placeholder \"Full name\"");
    assert_has(&errors(&on_input), "'for' does not apply to a textbox");
}

#[test]
fn a_form_binds_one_record_and_holds_its_inputs() {
    let list = with("rows 6 bind customer on", "rows 6 bind note_list on");
    assert_has(&errors(&list), "form at 1,2: a form binds to one record, as in 'bind customer'");

    let field = with("rows 6 bind customer on", "rows 6 bind customer.name on");
    assert_has(&errors(&field), "form at 1,2: a form binds to one record, as in 'bind customer'");

    let outside = with("bind customer.secret", "bind other.secret");
    assert_has(
        &errors(&outside),
        "password at 4,4: binds to 'other', but its form 'details' holds 'customer'",
    );

    let nested = with(
        "        help name help1 at 10,5 span 3 for notes",
        "        form name form1 at 10,5 span 3 bind customer",
    );
    assert_has(&errors(&nested), "form at 10,5: a form cannot hold another form");
}

#[test]
fn submit_belongs_to_forms() {
    let source = with(
        "taborder 1",
        "taborder 1 on submit: nameSubmitted",
    );
    assert_has(&errors(&source), "a textbox has no submit event");
}

#[test]
fn one_default_and_one_cancel_button_per_window() {
    let source = with(" cancel on click", " default on click");
    assert_has(&errors(&source), "more than one default button in this window");
}

#[test]
fn properties_apply_to_their_parts() {
    let readonly = with("key \"Ctrl+S\" default", "key \"Ctrl+S\" default readonly");
    assert_has(&errors(&readonly), "'readonly' does not apply to a button");

    let placeholder = with("bind customer.active readonly", "bind customer.active placeholder \"x\"");
    assert_has(&errors(&placeholder), "'placeholder' does not apply to a switch");

    let shortcut = with("key \"Ctrl+S\"", "key \"Ctrl+\"");
    assert_has(&errors(&shortcut), "'Ctrl+' is not a shortcut");

    let radio = with("bind customer.kind", "bind customer.name");
    assert_has(&errors(&radio), "'name' is text; a radio shows enum");
}

#[test]
fn handlers_set_errors_and_readonly_on_inputs_only() {
    let caption = with("notes.readonly = true", "save.error = \"x\"");
    assert_has(&errors(&caption), "button 'save' has no property 'error'");

    let wrong = with("notes.readonly = true", "notes.readonly = \"yes\"");
    assert!(!errors(&wrong).is_empty());
}

#[test]
fn menu_items_and_buttons_take_the_same_keys() {
    let menu = |key: &str| {
        format!("module m\napp A\n    menu\n        item \"File\"\n            item \"Save\" key \"{key}\"\n")
    };
    assert!(errors(&menu("Ctrl+Shift+F2")).is_empty());
    assert_has(&errors(&menu("Hyper+S")), "'Hyper+S' is not a shortcut");
}

/// A calculator's keys: the operators, the decimal point and equals are keys
/// too, alone or with modifiers (`Ctrl++`).
#[test]
fn operator_keys_are_keys() {
    use clarity_spec::widgets::{check_shortcut, shortcut_keys};
    for key in ["+", "-", "*", "/", ".", ",", "=", "Ctrl++", "Ctrl+-", "Shift+=", "7", "Ctrl+S"] {
        assert_eq!(check_shortcut(key), Ok(()), "{key}");
    }
    for key in ["%", "++", "Ctrl+", "+Ctrl", "Ctrl+Ctrl+S"] {
        assert!(check_shortcut(key).is_err(), "{key}");
    }
    // The modifiers and the key, as every renderer reads them.
    assert_eq!(shortcut_keys("Ctrl++"), (vec!["Ctrl"], "+"));
    assert_eq!(shortcut_keys("+"), (vec![], "+"));
    assert_eq!(shortcut_keys("Ctrl+Shift+F2"), (vec!["Ctrl", "Shift"], "F2"));
}
