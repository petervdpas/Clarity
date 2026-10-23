use clarity_spec::model::{PageKind, Widget, WindowSize};
use clarity_spec::parser::parse;
use clarity_spec::widgets::{WidgetKind, widget_line};
use clarity_spec::{Severity, load_sources};

const CUSTOMERS: &str = "module c
enum Kind
    A
    B
entity Customer
    name: text(80)
    kind: Kind
    active: bool
    limit: money
    notes: list<Note>
entity Note
    text: text
page CustomerBrowse: window
    title \"Customers\"
";

fn diagnostics(page: &str) -> Vec<(Severity, String)> {
    let source = format!("module p\nuse c\n{page}");
    load_sources([("c.clarity", CUSTOMERS), ("p.clarity", source.as_str())])
        .diagnostics
        .into_iter()
        .map(|d| (d.severity, d.message))
        .collect()
}

fn errors(page: &str) -> Vec<String> {
    diagnostics(page)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Error)
        .map(|(_, m)| m)
        .collect()
}

fn warnings(page: &str) -> Vec<String> {
    diagnostics(page)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Warning)
        .map(|(_, m)| m)
        .collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(
        messages.iter().any(|m| m.contains(needle)),
        "missing {needle:?} in {messages:#?}"
    );
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

#[test]
fn example_page_is_clean() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/shop");
    let project = clarity_spec::load_dir(std::path::Path::new(dir)).unwrap();
    let page = project
        .pages()
        .find(|p| p.name.name == "CustomerCard")
        .expect("example has the customer card");
    assert_eq!(page.name.name, "CustomerCard");
    assert_eq!(page.kind, PageKind::Window);
    assert_eq!(page.data.len(), 2);
    assert!(page.data[1].is_list());
    let problems: Vec<_> = project
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn parses_widgets_with_defaults_and_containers() {
    let (file, diags) = parse(
        "module p
page P: dialog
    title \"Edit\"
    grid 6 columns
    data c: Customer
    button \"Save\" name button1 at 5,4 on click -> Done
    panel \"Box\" name panel1 at 1,1 span 6 rows 3
        textbox name textbox1 at 1,1 span 4 bind c.name
    splitter name splitter1 at 1,5 span 6 rows 4 vertical
        pane
            label \"Left\" name label1 at 1,1
        pane
            canvas name canvas1 at 1,1 surface gpu renderer Game
",
        "p.clarity",
    );
    assert!(diags.is_empty(), "{diags:#?}");
    let page = &file.pages[0];
    assert_eq!(page.kind, PageKind::Dialog);
    assert_eq!(page.columns, 6);
    assert_eq!(page.title.as_deref(), Some("Edit"));

    let button = &page.widgets[0].props;
    assert_eq!(button.kind, WidgetKind::Button);
    assert_eq!(
        (button.width, button.height),
        (2, 1),
        "catalog default size"
    );
    assert_eq!(button.on_click.as_deref(), Some("Done"));

    let panel = &page.widgets[1];
    assert_eq!(panel.children[0].props.bind.as_deref(), Some("c.name"));

    let splitter = &page.widgets[2];
    assert_eq!(splitter.pages.len(), 2);
    assert_eq!(
        splitter.pages[1].children[0].props.renderer.as_deref(),
        Some("Game")
    );
}

#[test]
fn widget_lines_round_trip() {
    let source = include_str!("../../../examples/shop/desk.clarity");
    let (file, _) = parse(source, "desk.clarity");
    for widget in all_widgets(&file.pages[0].widgets) {
        let line = widget_line(&widget.props);
        let (reparsed, diags) = parse(&format!("module m\npage X: view\n    {line}\n"), "x");
        assert!(diags.is_empty(), "{line}: {diags:#?}");
        assert_eq!(reparsed.pages[0].widgets[0].props, widget.props, "{line}");
    }
}

#[test]
fn reports_parse_errors_per_widget() {
    let (_, diags) = parse(
        "module p
page P: window
    gauge at 1,1
    textbox span 3
    button \"x\" name button1 at 1,2 columns a
    textbox name textbox5 at 1,3 bind c.name bind c.kind
    label name label1 at 1,4 on hover -> X
    checkbox name checkbox1 at 1,5 on click -> X
",
        "p.clarity",
    );
    let messages: Vec<_> = diags.into_iter().map(|d| d.message).collect();
    assert_eq!(messages.len(), 6, "{messages:#?}");
    assert_has(&messages, "unknown widget 'gauge'");
    assert_has(&messages, "needs a position");
    assert_has(&messages, "'columns' does not apply to a button");
    assert_has(&messages, "duplicate 'bind'");
    assert_has(&messages, "unknown event 'hover'");
    assert_has(&messages, "a checkbox has no click event");
}

#[test]
fn reports_grid_problems() {
    let errs = errors(
        "page P: window
    data c: Customer
    textbox name textbox1 at 10,1 span 4 bind c.name
    label \"A\" name label1 at 1,2 span 4
    label \"B\" name label2 at 3,2 span 2
    panel name panel1 at 1,4 span 4 rows 2
        label \"Inside\" name label3 at 3,1 span 3
",
    );
    assert_has(&errs, "runs past column 12");
    assert_has(&errs, "label at 3,2 overlaps the label at 1,2");
    assert_has(&errs, "label at 3,1 span 3 runs past column 4");
    assert_eq!(errs.len(), 3, "{errs:#?}");
}

#[test]
fn checks_bindings_against_the_dictionary() {
    let page = "page P: window
    data c: Customer
    data many: list<Customer>
    textbox name textbox1 at 1,1 bind c.limit
    checkbox name checkbox1 at 1,2 bind c.name
    textbox name textbox2 at 1,3 bind many.name
    textbox name textbox3 at 1,4 bind c.nope
    textbox name textbox4 at 1,5 bind ghost.name
    table name table1 at 1,6 bind c
    list name list1 at 1,12 bind many show name, notes
    number name number1 at 6,1 bind c.limit
    dropdown name dropdown1 at 6,2 bind c.kind
    checkbox name checkbox2 at 6,3 bind c.active
";
    let errs = errors(page);
    let expected = [
        "'limit' is money; a textbox shows text, email, url",
        "'name' is text; a checkbox shows bool",
        "'many' is a list",
        "'Customer' has no field 'nope'",
        "unknown data 'ghost'",
        "a table binds to a list",
        "list field 'notes' cannot be shown",
    ];
    for needle in expected {
        assert_has(&errs, needle);
    }
    assert_eq!(errs.len(), expected.len(), "{errs:#?}");
}

#[test]
fn unbound_inputs_are_fine_and_unknown_click_targets_warn() {
    let page = "page P: window
    textbox name textbox1 at 1,1
    button \"Open\" name button1 at 1,2 on click -> CustomerBrowse
    button \"Save\" name button2 at 3,2 on click -> save_customer
    button \"Bad\" name button3 at 5,2 on click -> Customer
    canvas name canvas1 at 1,3
";
    let warns = warnings(page);
    assert_has(&warns, "'save_customer' is not a window");
    assert_has(&warns, "canvas at 1,3 draws nothing yet; wire 'on draw: handler'");
    assert_eq!(warns.len(), 2, "{warns:#?}");
    let errs = errors(page);
    assert_has(
        &errs,
        "'Customer' is an entity, a click opens a window",
    );
}

#[test]
fn checks_containers_and_grid_size() {
    let errs = errors(
        "page P: window
    grid 30 columns
",
    );
    assert_has(&errs, "1 to 24 columns");

    let errs = errors(
        "page Q: window
    tabs name tabs1 at 1,1
    splitter name splitter1 at 1,8
        pane
",
    );
    assert_has(&errs, "tabs need at least one 'tab'");
    assert_has(&errs, "exactly two 'pane'");
}

/// A window opens at the size it says, in pixels: `size 320 x 460`. Without
/// it, at the standard size every stack shares.
#[test]
fn a_window_says_its_size() {
    let (file, diags) = parse("module p\npage Calc: window\n    size 320 x 460\n", "p.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(file.pages[0].size, Some(WindowSize { width: 320, height: 460 }));
    let (file, _) = parse("module p\npage P: window\n", "p.clarity");
    assert_eq!(file.pages[0].size, None);

    let (_, diags) = parse("module p\npage P: window\n    size 320 x 460\n    size 400 x 300\n", "p.clarity");
    assert!(diags.iter().any(|d| d.message.contains("duplicate 'size'")), "{diags:#?}");
    let (_, diags) = parse("module p\npage P: window\n    size 320\n", "p.clarity");
    assert!(diags.iter().any(|d| d.message.contains("'size 320 x 460'")), "{diags:#?}");

    // Small enough to fit a screen, large enough to hold a part.
    assert_has(&errors("page P: window\n    size 100 x 460\n"), "200 to 3840 pixels wide and 150 to 2160 high");
    assert_has(&errors("page P: window\n    size 4000 x 460\n"), "200 to 3840 pixels wide");
}

#[test]
fn page_names_share_the_namespace() {
    let errs = errors(
        "page CustomerBrowse: window
",
    );
    assert!(
        errs.is_empty(),
        "different modules may reuse names: {errs:#?}"
    );
    let errs = errors(
        "page A: window
page A: view
",
    );
    assert_has(&errs, "'A' is already declared");
}

/// A window holds plain values besides records: a number it shows, a value
/// of an enum, a flag. Each starts at its default, or empty.
#[test]
fn a_window_holds_plain_values() {
    use clarity_spec::code::TypeRefKind;
    use clarity_spec::model::{DataKind, Literal};
    let page = "page Calc: window
    data shown: decimal
    data pending: Kind
    data fresh: bool, default true
    data customer: Customer
    data customers: list<Customer>
";
    let source = format!("module p\nuse c\n{page}");
    let project = load_sources([("c.clarity", CUSTOMERS), ("p.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let data = &project.files[1].pages[0].data;
    let of: Vec<&DataKind> = data.iter().map(|d| &d.of).collect();
    assert!(matches!(of[0], DataKind::Value { ty, default: None } if ty.kind == TypeRefKind::Decimal), "{of:#?}");
    // A name is a plain value when it names an enum, a record when it names an entity.
    assert!(matches!(of[1], DataKind::Value { ty, default: None } if ty.kind == TypeRefKind::Named { name: "Kind".into() }), "{of:#?}");
    assert!(matches!(of[2], DataKind::Value { ty, default: Some(Literal::Bool(true)) } if ty.kind == TypeRefKind::Bool), "{of:#?}");
    assert!(matches!(of[3], DataKind::Record { entity } if entity.name == "Customer"), "{of:#?}");
    assert!(matches!(of[4], DataKind::List { entity } if entity.name == "Customer"), "{of:#?}");

    // A default fits its type, and only a plain value has one.
    assert_has(&errors("page P: window\n    data fresh: bool, default 3\n"), "'fresh' is bool, not 3");
    assert_has(&errors("page P: window\n    data pending: Kind, default C\n"), "Kind has no value C");
    assert_has(&errors("page P: window\n    data customer: Customer, default 1\n"), "'customer' is a record, which starts new; only a plain value has a default");
    assert_has(&errors("page P: window\n    data x: Teleport\n"), "unknown entity or enum 'Teleport'");
    assert_has(&errors("page P: window\n    data x: list<decimal>\n"), "a list holds records, as in 'list<Order>'");
}

/// A part binds a plain value by its name, when it shows that type.
#[test]
fn a_part_binds_a_plain_value() {
    let page = "page P: window
    data shown: decimal
    data pending: Kind
    data fresh: bool
";
    let ok = format!("{page}    text name text9 at 1,1 span 4 bind shown\n    dropdown name dropdown2 at 1,2 span 4 bind pending\n    checkbox \"Fresh\" name checkbox3 at 1,3 span 4 bind fresh\n    number name number2 at 1,4 span 4 bind shown\n");
    assert!(errors(&ok).is_empty(), "{:#?}", errors(&ok));
    assert_has(&errors(&format!("{page}    textbox at 1,1 span 4 bind fresh\n")), "'fresh' is bool; a textbox shows");
    assert_has(&errors(&format!("{page}    text at 1,1 span 4 bind shown.digits\n")), "'shown' is a plain value; bind it as 'shown'");
    assert_has(&errors(&format!("{page}    table at 1,1 span 4 rows 3 bind shown columns name\n")), "a table binds to a list");
}
