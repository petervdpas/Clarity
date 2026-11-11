//! The designer's edits of a window, made on its `.cxaml` (`cxaml::apply`):
//! the window written the old way below is converted as a project holds it,
//! and each edit is made on its file.

use clarity_spec::cxaml::{apply, read_page};
use clarity_spec::edit::Edit;
use clarity_spec::model::PageKind;
use clarity_spec::widgets::{WidgetKind, new_props};

const PAGE: &str = "module p

page Card: window
    title \"Card\"
    data c: Customer

    label \"Name\" at 1,1 span 2
    textbox at 3,1 span 6 bind c.name // the name
    panel \"Box\" at 1,3 span 6 rows 3
        label \"Inside\" at 1,1
    tabs at 1,7 span 12 rows 4
        tab \"One\"
            label \"First\" at 1,1
";

/// Offset of the last line of Clarity's text whose content starts with
/// `needle`: for the edits that stay text edits (a new page, a whole window).
fn at(source: &str, needle: &str) -> u32 {
    let mut offset = 0;
    let mut found = None;
    for line in source.split('\n') {
        let indent = line.len() - line.trim_start().len();
        if line[indent..].starts_with(needle) {
            found = Some((offset + indent) as u32);
        }
        offset += line.len() + 1;
    }
    found.unwrap_or_else(|| panic!("no line starts with {needle:?}"))
}

/// The window Card of PAGE in its own file, as a project holds it.
fn card() -> String {
    design(&[("p.clarity", PAGE)], "Card")
}

/// The design file of `page` among `modules` converted as a project holds them.
fn design(modules: &[(&str, &str)], page: &str) -> String {
    let file = format!("design/{page}.cxaml");
    clarity_spec::converted(modules.iter().copied()).into_iter().find(|(p, _)| p.ends_with(&file)).map(|(_, t)| t).unwrap_or_else(|| panic!("no {file}"))
}

/// Where the first element that starts with `needle` starts in `markup`.
fn el(markup: &str, needle: &str) -> u32 {
    markup.find(needle).unwrap_or_else(|| panic!("no {needle:?} in {markup}")) as u32
}

/// Where the window's own element starts.
fn root(markup: &str) -> u32 {
    read_page(markup, "").0.expect("the page").span.start
}

/// `edit` made on the window's file, which then reads back clean.
fn edit(markup: &str, edit: Edit) -> (String, Option<u32>) {
    let outcome = apply(None, markup, &edit).unwrap_or_else(|e| panic!("edit failed: {e}"));
    let (page, diags) = read_page(&outcome.text, "Card.cxaml");
    assert!(page.is_some() && diags.is_empty(), "the edit wrote a file that does not read back: {diags:#?}\n{}", outcome.text);
    (outcome.text, outcome.focus)
}

fn error(markup: &str, edit: Edit) -> String {
    apply(None, markup, &edit).expect_err("edit should fail").0
}

#[test]
fn places_a_widget_in_reading_order() {
    let markup = card();
    let mut props = new_props(WidgetKind::Checkbox, 9, 1);
    props.bind = Some("c.active".into());
    let (text, focus) = edit(&markup, Edit::PlaceWidget { container: root(&markup), props });
    assert!(
        text.contains("  <Textbox at=\"3,1\" span=\"6\" bind=\"c.name\" />\n  <Checkbox name=\"active\" at=\"9,1\" span=\"3\" bind=\"c.active\" />\n  <Panel"),
        "{text}"
    );
    assert!(text[focus.unwrap() as usize..].starts_with("<Checkbox"));
}

#[test]
fn places_widgets_inside_containers_and_tabs() {
    let markup = card();
    let (text, _) = edit(&markup, Edit::PlaceWidget { container: el(&markup, "<Panel"), props: new_props(WidgetKind::Button, 1, 2) });
    assert!(text.contains("    <Label at=\"1,1\" span=\"3\" caption=\"Inside\" />\n    <Button name=\"button1\" at=\"1,2\" span=\"2\" />\n  </Panel>"), "{text}");

    let (text, _) = edit(&markup, Edit::PlaceWidget { container: el(&markup, "<Tab title=\"One\""), props: new_props(WidgetKind::Date, 1, 2) });
    assert!(
        text.ends_with("      <Label at=\"1,1\" span=\"3\" caption=\"First\" />\n      <Date name=\"date1\" at=\"1,2\" span=\"3\" />\n    </Tab>\n  </Tabs>\n</Window>\n"),
        "{text}"
    );
}

#[test]
fn new_containers_get_their_blocks() {
    let markup = card();
    let (text, _) = edit(&markup, Edit::PlaceWidget { container: root(&markup), props: new_props(WidgetKind::Splitter, 1, 12) });
    assert!(text.ends_with("  <Splitter name=\"splitter1\" at=\"1,12\" span=\"12\" rows=\"6\">\n    <Pane />\n    <Pane />\n  </Splitter>\n</Window>\n"), "{text}");
    let (page, _) = read_page(&text, "");
    let splitter = page.unwrap().widgets.into_iter().last().unwrap();
    assert_eq!(splitter.props.orientation, Some(clarity_spec::model::Orientation::Horizontal), "a new splitter stands its panes side by side");

    let (text, _) = edit(&markup, Edit::PlaceWidget { container: root(&markup), props: new_props(WidgetKind::Tabs, 1, 12) });
    assert!(text.ends_with("  <Tabs name=\"tabs1\" at=\"1,12\" span=\"8\" rows=\"5\">\n    <Tab title=\"Tab 1\" />\n  </Tabs>\n</Window>\n"), "{text}");
}

/// A changed part keeps its place in reading order. (A comment written on
/// a part's line in Clarity has no place in a window's file.)
#[test]
fn updates_a_widget_in_place() {
    let markup = card();
    let mut props = new_props(WidgetKind::Textbox, 3, 2);
    props.caption = None;
    props.width = 8;
    props.bind = Some("c.email".into());
    let (text, focus) = edit(&markup, Edit::UpdateWidget { item: el(&markup, "<Textbox"), container: None, props });
    assert!(
        text.contains("  <Label at=\"1,1\" span=\"2\" caption=\"Name\" />\n  <Textbox name=\"textbox1\" at=\"3,2\" span=\"8\" bind=\"c.email\" />\n  <Panel"),
        "{text}"
    );
    assert!(text[focus.unwrap() as usize..].starts_with("<Textbox"));
}

#[test]
fn moves_a_widget_into_a_container() {
    let markup = card();
    let mut props = new_props(WidgetKind::Label, 1, 2);
    props.caption = Some("Name".into());
    props.width = 2;
    let (text, _) = edit(&markup, Edit::UpdateWidget { item: el(&markup, "<Label at=\"1,1\" span=\"2\" caption=\"Name\""), container: Some(el(&markup, "<Panel")), props });
    assert!(text.contains("    <Label at=\"1,1\" span=\"3\" caption=\"Inside\" />\n    <Label name=\"label1\" at=\"1,2\" span=\"2\" caption=\"Name\" />\n  </Panel>"), "{text}");
    assert!(!text.contains("caption=\"Name\" />\n  <Textbox"), "{text}");
}

#[test]
fn moves_a_container_with_its_children() {
    let markup = card();
    let mut props = new_props(WidgetKind::Panel, 7, 3);
    props.caption = Some("Box".into());
    props.width = 6;
    props.height = 3;
    let (text, _) = edit(&markup, Edit::UpdateWidget { item: el(&markup, "<Panel"), container: Some(el(&markup, "<Tab title=\"One\"")), props });
    assert!(
        text.contains("      <Label at=\"1,1\" span=\"3\" caption=\"First\" />\n      <Panel name=\"panel1\" at=\"7,3\" span=\"6\" rows=\"3\" caption=\"Box\">\n        <Label at=\"1,1\" span=\"3\" caption=\"Inside\" />\n      </Panel>\n"),
        "{text}"
    );
}

#[test]
fn rejects_bad_widget_edits() {
    let markup = card();
    let page = root(&markup);
    let mut bad = new_props(WidgetKind::Button, 1, 1);
    bad.bind = Some("c.name".into());
    assert!(error(&markup, Edit::PlaceWidget { container: page, props: bad }).contains("cannot be bound"));

    let mut bad = new_props(WidgetKind::Label, 1, 1);
    bad.columns = vec!["x".into()];
    assert!(error(&markup, Edit::PlaceWidget { container: page, props: bad }).contains("'columns' does not apply"));

    let zero = new_props(WidgetKind::Label, 0, 1);
    assert!(error(&markup, Edit::PlaceWidget { container: page, props: zero }).contains("start at 1"));

    let inside_label = new_props(WidgetKind::Label, 1, 1);
    let message = error(&markup, Edit::PlaceWidget { container: el(&markup, "<Textbox"), props: inside_label });
    assert!(message.contains("no page, panel, tab or pane"), "{message}");

    let wrong_kind = new_props(WidgetKind::Button, 3, 1);
    assert!(error(&markup, Edit::UpdateWidget { item: el(&markup, "<Textbox"), container: None, props: wrong_kind }).contains("cannot be turned into"));

    let mut into_itself = new_props(WidgetKind::Panel, 1, 1);
    into_itself.caption = Some("Box".into());
    assert!(error(&markup, Edit::UpdateWidget { item: el(&markup, "<Panel"), container: Some(el(&markup, "<Panel")), props: into_itself }).contains("into itself"));
}

#[test]
fn removes_widgets_tabs_and_data() {
    let markup = card();
    let (text, _) = edit(&markup, Edit::Remove { item: el(&markup, "<Panel") });
    assert!(!text.contains("<Panel") && !text.contains("Inside"), "{text}");

    let (text, _) = edit(&markup, Edit::Remove { item: el(&markup, "<Data name=\"c\"") });
    assert!(!text.contains("<Data"), "{text}");

    let (text, _) = edit(&markup, Edit::Remove { item: el(&markup, "<Tab title=\"One\"") });
    assert!(text.ends_with("  <Tabs at=\"1,7\" span=\"12\" rows=\"4\" />\n</Window>\n"), "{text}");

    // The title is no part: nothing starts where it is written.
    let message = error(&markup, Edit::Remove { item: el(&markup, "title=") });
    assert!(message.contains("there is no window, part, tab, region, command or data declaration there"), "{message}");
}

#[test]
fn adds_tabs_data_grid_size_and_style() {
    let markup = card();
    let (text, focus) = edit(&markup, Edit::AddTab { item: el(&markup, "<Tabs"), title: "Two".into() });
    assert!(text.ends_with("    <Tab title=\"Two\" />\n  </Tabs>\n</Window>\n"), "{text}");
    assert!(text[focus.unwrap() as usize..].starts_with("<Tab title=\"Two\""));

    let page = root(&markup);
    let (text, _) = edit(&markup, Edit::AddData { page, name: "orders".into(), entity: "Order".into(), list: true });
    assert!(text.contains("  <Data name=\"c\" type=\"Customer\" />\n  <Data name=\"orders\" type=\"list&lt;Order&gt;\" />\n"), "{text}");

    let (text, _) = edit(&markup, Edit::SetGrid { page, columns: 16, rows: None });
    assert!(text.contains("<Window class=\"p.Card\" title=\"Card\" columns=\"16\">\n"), "{text}");
    let (text, _) = edit(&text, Edit::SetGrid { page: root(&text), columns: 8, rows: None });
    assert!(text.contains(" columns=\"8\">\n") && !text.contains("16"), "{text}");
    assert!(error(&markup, Edit::SetGrid { page, columns: 97, rows: None }).contains("1 to 96"));
    // A page's own exact grid: square cells of 10 pixels, and back to the project's rows.
    let (text, _) = edit(&text, Edit::SetGrid { page: root(&text), columns: 56, rows: Some(10) });
    assert!(text.contains(" columns=\"56\" cell=\"10\">\n"), "{text}");
    let (text, _) = edit(&text, Edit::SetGrid { page: root(&text), columns: 56, rows: None });
    assert!(text.contains(" columns=\"56\">\n") && !text.contains("cell="), "{text}");
    assert!(error(&markup, Edit::SetGrid { page, columns: 56, rows: Some(4) }).contains("8 to 64"));

    // The window's size: set, changed, and removed for the standard one.
    let (text, _) = edit(&markup, Edit::SetSize { page, width: Some(320), height: Some(460) });
    assert!(text.contains("<Window class=\"p.Card\" title=\"Card\" width=\"320\" height=\"460\">\n"), "{text}");
    let (text, _) = edit(&text, Edit::SetSize { page: root(&text), width: Some(400), height: Some(300) });
    assert!(text.contains(" width=\"400\" height=\"300\">") && !text.contains("320"), "{text}");
    let (text, _) = edit(&text, Edit::SetSize { page: root(&text), width: None, height: None });
    assert!(!text.contains("width=") && !text.contains("height="), "{text}");

    // The window's own style: set, changed, and removed.
    let (text, _) = edit(&markup, Edit::SetPageStyle { page, style: "splashBack".into() });
    assert!(text.contains("<Window class=\"p.Card\" title=\"Card\" style=\"splashBack\">\n"), "{text}");
    let (text, _) = edit(&text, Edit::SetPageStyle { page: root(&text), style: "quiet".into() });
    assert!(text.contains(" style=\"quiet\">") && !text.contains("splashBack"), "{text}");
    let (text, _) = edit(&text, Edit::SetPageStyle { page: root(&text), style: String::new() });
    assert!(!text.contains("style="), "{text}");
    assert!(error(&markup, Edit::SetPageStyle { page, style: "not a name".into() }).contains("not a style name"));

    // Who may open it: set, changed, and removed (docs/APP-CAPABILITIES-PLAN.md step 4).
    let (text, _) = edit(&markup, Edit::SetPageAllow { page, allow: "me.role == Admin".into() });
    assert!(text.contains("<Window class=\"p.Card\" allow=\"me.role == Admin\" title=\"Card\">"), "{text}");
    let (text, _) = edit(&text, Edit::SetPageAllow { page: root(&text), allow: "me.id > 0".into() });
    assert!(text.contains(" allow=\"me.id &gt; 0\" ") && !text.contains("Admin"), "{text}");
    let (text, _) = edit(&text, Edit::SetPageAllow { page: root(&text), allow: "  ".into() });
    assert!(!text.contains("allow="), "{text}");
}

/// A new page is a file of its own in design/, titled after its name.
#[test]
fn adds_a_page() {
    let project = clarity_spec::load_converted([("p.clarity", PAGE)]);
    let add = |name: &str, kind| clarity_spec::cxaml::added(&project, std::path::Path::new("p.clarity"), "module p\n", &Edit::AddPage { name: name.into(), kind }).unwrap();
    let added = add("Settings", PageKind::Dialog).unwrap();
    assert_eq!(added.files[0].0, std::path::Path::new("design/Settings.cxaml"));
    assert!(added.files[0].1.contains("<Dialog class=\"p.Settings\" title=\"@settingsTitle\">"), "{}", added.files[0].1);
    assert!(added.keys.iter().any(|(k, v)| k == "settingsTitle" && v == "Settings"), "{:?}", added.keys);
    assert!(add("Card", PageKind::View).unwrap_err().contains("already declared"));
}

#[test]
fn renames_pages_and_widget_tabs() {
    let markup = card();
    let (text, _) = edit(&markup, Edit::SetTitle { target: root(&markup), title: "Customer".into() });
    assert!(text.contains("<Window class=\"p.Card\" title=\"Customer\">"), "{text}");
    let (text, _) = edit(&markup, Edit::SetTitle { target: el(&markup, "<Tab title=\"One\""), title: "Orders".into() });
    assert!(text.contains("    <Tab title=\"Orders\">\n"), "{text}");
}

#[test]
fn edits_keep_the_example_page_valid() {
    let (desk, customers, sales) = (shop("counter"), shop("customers"), shop("sales"));
    let modules = [("customers.clarity", customers.as_str()), ("sales.clarity", sales.as_str()), ("desk.clarity", desk.as_str())];
    let mut files = clarity_spec::converted(modules);
    let view = files.iter().position(|(p, _)| p.ends_with("design/CustomerCardView.cxaml")).expect("the card's view");
    let markup = files[view].1.clone();
    let mut props = new_props(WidgetKind::Number, 8, 3);
    props.bind = Some("customer.creditLimit".into());
    props.width = 2;
    let (text, _) = edit(&markup, Edit::UpdateWidget { item: el(&markup, "<Number"), container: None, props });
    files[view].1 = text;
    let project = clarity_spec::load_sources(files.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn creates_widgets_with_catalog_defaults() {
    let markup = card();
    let (text, _) = edit(
        &markup,
        Edit::CreateWidget { container: el(&markup, "<Panel"), kind: WidgetKind::Textbox, col: 1, row: 2, width: Some(5), caption: None, bind: Some("c.email".into()), part: None, height: None },
    );
    assert!(text.contains("    <Label at=\"1,1\" span=\"3\" caption=\"Inside\" />\n    <Textbox name=\"email\" at=\"1,2\" span=\"5\" bind=\"c.email\" />\n"), "{text}");
}

#[test]
fn applies_a_batch_as_one_edit() {
    let markup = card();
    let page = root(&markup);
    let (text, focus) = edit(
        &markup,
        Edit::Batch {
            edits: vec![
                Edit::CreateWidget { container: page, kind: WidgetKind::Label, col: 1, row: 2, width: Some(2), caption: Some("E-mail".into()), bind: None, part: None, height: None },
                Edit::CreateWidget { container: page, kind: WidgetKind::Textbox, col: 3, row: 2, width: None, caption: None, bind: Some("c.email".into()), part: None, height: None },
            ],
        },
    );
    assert!(
        text.contains("  <Label name=\"eMailLabel\" at=\"1,2\" span=\"2\" caption=\"E-mail\" />\n  <Textbox name=\"email\" at=\"3,2\" span=\"4\" bind=\"c.email\" />\n  <Panel"),
        "{text}"
    );
    assert!(text[focus.unwrap() as usize..].starts_with("<Textbox name=\"email\" at=\"3,2\""));

    // One failing edit refuses the whole batch.
    let failing = Edit::Batch {
        edits: vec![
            Edit::CreateWidget { container: page, kind: WidgetKind::Label, col: 1, row: 2, width: None, caption: None, bind: None, part: None, height: None },
            Edit::Remove { item: 9999 },
        ],
    };
    assert!(apply(None, &markup, &failing).is_err());
}

#[test]
fn no_op_edit_round_trips_exactly() {
    let (desk, customers, sales) = (shop("counter"), shop("customers"), shop("sales"));
    let sources = [
        PAGE,
        "module m\r\npage P: window\r\n    title \"T\"\r\n    label \"A\" at 1,1 // c\r\n",
        desk.as_str(),
        customers.as_str(),
        sales.as_str(),
    ];
    // Every edit writes the whole file again from its model, so an edit that
    // changes nothing must give back the file byte for byte.
    for source in sources {
        let files: Vec<(String, String)> = clarity_spec::converted([("m.clarity", source)]).into_iter().filter(|(p, _)| clarity_spec::cxaml::is_page_file(p)).collect();
        assert!(!files.is_empty(), "no page file from {source:?}");
        for (path, markup) in files {
            // Setting a page's title to what it already is changes nothing; a view has none.
            let page = read_page(&markup, &path).0.expect("the page");
            let title = page.title.clone().unwrap_or_default();
            let outcome = apply(None, &markup, &Edit::SetTitle { target: page.span.start, title }).unwrap();
            assert_eq!(outcome.text, markup, "{path}");
        }
    }
}

#[test]
fn rejects_stale_offsets() {
    let msg = apply(None, &card(), &Edit::Remove { item: 9999 }).unwrap_err().0;
    assert!(msg.contains("there is no window, part, tab, region, command or data declaration there"), "{msg}");
}

/// A whole window goes with its file (the IDE deletes it, and its code): no
/// edit removes it, from its own file or from a module's text.
#[test]
fn a_whole_window_goes_with_its_file() {
    let source = format!("{PAGE}\npage Other: dialog\n    title \"Other\"\n");
    assert!(clarity_spec::edit::apply(&source, &Edit::Remove { item: at(&source, "page Card") }).is_err());
    let markup = card();
    assert!(error(&markup, Edit::Remove { item: root(&markup) }).contains("removed with its file"));
}

#[test]
fn a_part_keeps_the_style_it_is_given() {
    let markup = card();
    let mut props = new_props(WidgetKind::Textbox, 3, 2);
    props.caption = None;
    props.style = Some("loud".into());
    let (text, _) = edit(&markup, Edit::UpdateWidget { item: el(&markup, "<Textbox"), container: None, props });
    let line = text.lines().find(|l| l.trim_start().starts_with("<Textbox")).unwrap();
    assert!(line.trim_start().starts_with("<Textbox name=\"textbox1\" at=\"3,2\" span=\"4\" ") && line.contains(" style=\"loud\""), "{text}");
    let (page, diagnostics) = read_page(&text, "Card.cxaml");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let page = page.unwrap();
    let textbox = page.widgets.iter().find(|w| w.props.kind == WidgetKind::Textbox).unwrap();
    assert_eq!(textbox.props.style.as_deref(), Some("loud"));
}

/// The designer resizes a container and scales its parts in one batch: the
/// container's element changes, and so do the elements inside it.
#[test]
fn a_batch_resizes_a_container_and_moves_its_parts() {
    let markup = card();
    let page = read_page(&markup, "").0.unwrap();
    let panel = page.widgets.iter().find(|w| w.props.kind == WidgetKind::Panel).unwrap();
    let inside = &panel.children[0];
    let mut wider = panel.props.clone();
    wider.width = 12;
    let mut moved = inside.props.clone();
    moved.col = 3;
    moved.width = 4;
    let (text, _) = edit(
        &markup,
        Edit::Batch {
            edits: vec![
                Edit::UpdateWidget { item: panel.span.start, container: None, props: wider },
                Edit::UpdateWidget { item: inside.span.start, container: None, props: moved },
            ],
        },
    );
    assert!(text.contains("  <Panel name=\"panel1\" at=\"1,3\" span=\"12\" rows=\"3\" caption=\"Box\">\n    <Label name=\"label1\" at=\"3,1\" span=\"4\" caption=\"Inside\" />\n"), "{text}");
}

/// Every part has a name, as in an IDE: a part written without one (by hand
/// or by the agent) gets one the first time the designer changes it, so its
/// events can be wired and its properties set from code.
#[test]
fn a_part_without_a_name_gets_one_when_it_is_changed() {
    let markup = card();
    let mut props = new_props(WidgetKind::Label, 1, 1);
    props.caption = Some("Name:".into());
    props.width = 2;
    let (text, _) = edit(&markup, Edit::UpdateWidget { item: el(&markup, "<Label at=\"1,1\" span=\"2\" caption=\"Name\""), container: None, props: props.clone() });
    assert!(text.contains("  <Label name=\"label1\" at=\"1,1\" span=\"2\" caption=\"Name:\" />\n"), "{text}");
    // The next one gets the next free name.
    let inside = el(&text, "<Label at=\"1,1\" span=\"3\" caption=\"Inside\"");
    let (text, _) = edit(&text, Edit::UpdateWidget { item: inside, container: None, props: { let mut p = props.clone(); p.caption = Some("In".into()); p } });
    assert!(text.contains("<Label name=\"label2\" at=\"1,1\" span=\"2\" caption=\"In\" />"), "{text}");
}

/// Name them: every part of the window that has no name gets one after its
/// purpose, or the first free one of its kind when it shows none; the rest
/// of each element stays as written.
#[test]
fn names_every_part_that_has_none() {
    let (text, _) = edit(&card(), Edit::NameParts);
    assert!(text.contains("  <Label name=\"nameLabel\" at=\"1,1\" span=\"2\" caption=\"Name\" />\n"), "{text}");
    assert!(text.contains("  <Textbox name=\"name\" at=\"3,1\" span=\"6\" bind=\"c.name\" />\n"), "{text}");
    assert!(text.contains("  <Panel name=\"boxPanel\" at=\"1,3\" span=\"6\" rows=\"3\" caption=\"Box\">\n    <Label name=\"insideLabel\" at=\"1,1\" span=\"3\" caption=\"Inside\" />\n"), "{text}");
    assert!(text.contains("      <Label name=\"firstLabel\" at=\"1,1\" span=\"3\" caption=\"First\" />\n"), "{text}");
    // Named already: nothing changes.
    assert_eq!(edit(&text, Edit::NameParts).0, text);
}

/// The properties panel's "Stick to bottom" sets `bottom` on the part.
#[test]
fn updating_a_widget_sticks_it_to_the_bottom() {
    let markup = card();
    let mut props = new_props(WidgetKind::Textbox, 3, 2);
    props.caption = None;
    props.width = 6;
    props.bind = Some("c.name".into());
    props.bottom = true;
    let (text, _) = edit(&markup, Edit::UpdateWidget { item: el(&markup, "<Textbox"), container: None, props });
    let line = text.lines().find(|l| l.trim_start().starts_with("<Textbox")).unwrap().trim();
    assert!(line.starts_with("<Textbox name=\"textbox1\" at=\"3,2\" span=\"6\" ") && line.contains(" bind=\"c.name\"") && line.contains(" bottom=\"true\""), "{text}");
}

/// A module of the shop example written the old way, its windows and views
/// as page blocks (tests/fixtures/old-pages): the rich page the edits of
/// Clarity's text are tried on.
fn shop(module: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/old-pages/{module}.clarity"));
    std::fs::read_to_string(path).unwrap()
}
