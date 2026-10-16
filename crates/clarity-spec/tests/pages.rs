use clarity_spec::model::{PageKind, Widget};
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
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/shop/spec");
    let project = clarity_spec::load_dir(std::path::Path::new(dir)).unwrap();
    let page = project
        .pages()
        .find(|p| p.name.name == "CustomerCard")
        .expect("example has the customer card");
    assert_eq!(page.name.name, "CustomerCard");
    assert_eq!(page.kind, PageKind::Window);
    assert_eq!(page.data.len(), 2);
    assert!(page.data[1].list);
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
    button \"Save\" at 5,4 on click -> Done
    panel \"Box\" at 1,1 span 6 rows 3
        textbox at 1,1 span 4 bind c.name
    splitter at 1,5 span 6 rows 4 vertical
        pane
            label \"Left\" at 1,1
        pane
            canvas at 1,1 surface gpu renderer Game
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
    let source = include_str!("../../../examples/shop/spec/desk.clarity");
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
    slider at 1,1
    textbox span 3
    button \"x\" at 1,2 columns a
    textbox at 1,3 bind c.name bind c.kind
    label at 1,4 on hover -> X
    checkbox at 1,5 on click -> X
",
        "p.clarity",
    );
    let messages: Vec<_> = diags.into_iter().map(|d| d.message).collect();
    assert_eq!(messages.len(), 6, "{messages:#?}");
    assert_has(&messages, "unknown widget 'slider'");
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
    textbox at 10,1 span 4 bind c.name
    label \"A\" at 1,2 span 4
    label \"B\" at 3,2 span 2
    panel at 1,4 span 4 rows 2
        label \"Inside\" at 3,1 span 3
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
    textbox at 1,1 bind c.limit
    checkbox at 1,2 bind c.name
    textbox at 1,3 bind many.name
    textbox at 1,4 bind c.nope
    textbox at 1,5 bind ghost.name
    table at 1,6 bind c
    list at 1,12 bind many show name, notes
    number at 6,1 bind c.limit
    dropdown at 6,2 bind c.kind
    checkbox at 6,3 bind c.active
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
    textbox at 1,1
    button \"Open\" at 1,2 on click -> CustomerBrowse
    button \"Save\" at 3,2 on click -> save_customer
    button \"Bad\" at 5,2 on click -> Customer
    canvas at 1,3
";
    let warns = warnings(page);
    assert_has(&warns, "'save_customer' is not a window");
    assert_has(&warns, "canvas at 1,3 has no renderer yet");
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
    tabs at 1,1
    splitter at 1,8
        pane
",
    );
    assert_has(&errs, "tabs need at least one 'tab'");
    assert_has(&errs, "exactly two 'pane'");
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
