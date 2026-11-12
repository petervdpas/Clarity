//! A window's content, which shows one view at a time (DESIGN 5.33):
//! `content name main` with a `view X` line for each view it can show. It has
//! no look of its own (a view brings its heading, buttons and tabs); code
//! picks the view with `main.swap(X)`. Each view's parts, data and code join
//! the window's, as in a slide-out, so the generators get them placed, the
//! views as pages of the part and `main.swap(X)` as the page's number.

use clarity_spec::code::{ExprKind, StmtKind};
use clarity_spec::keyed::load_keyed;
use clarity_spec::widgets::{ContainerKind, WidgetKind};
use clarity_spec::{Project, Severity};

const MODEL: &str = "module model

page Workspace: window
    data picked: text
    content name main at 1,1 span 8 rows 10
        view DomainEditor on saved: domainSaved
        view ProjectEditor
    button \"Project\" name projectButton at 9,1 span 2 on click: projectClicked

page DomainEditor: view
    tells saved
    data domainName: text
    heading \"Domain\" name domainHeading at 1,1 span 6
    textbox name domainBox at 1,2 span 4 bind domainName
    button \"Save\" name saveDomainButton at 5,2 span 2 on click: saveDomainClicked

page ProjectEditor: view
    data projectName: text
    textbox name projectBox at 1,1 span 4 bind projectName

code Workspace
    handler projectClicked
        projectName = picked
        main.swap(ProjectEditor)

    handler domainSaved
        main.swap(DomainEditor)

code DomainEditor
    handler saveDomainClicked
        tell saved()
";

fn project(text: &str) -> Project {
    load_keyed([("model.clarity", text)], &[], "en")
}

fn errors(text: &str) -> Vec<String> {
    project(text).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn has(text: &str, words: &str) {
    let found = errors(text);
    assert!(found.iter().any(|m| m.contains(words)), "{words}: {found:#?}");
}

#[test]
fn a_windows_content_lists_the_views_it_shows_and_code_picks_one() {
    assert!(errors(MODEL).is_empty(), "{:#?}", errors(MODEL));
    let project = project(MODEL);
    let main = &project.pages().nth(0).unwrap().widgets[0];
    assert_eq!(main.props.kind, WidgetKind::Content);
    assert_eq!(clarity_spec::widgets::container(WidgetKind::Content), Some(ContainerKind::Content));
    let shown: Vec<_> = main.pages.iter().map(|p| p.view.as_deref()).collect();
    assert_eq!(shown, [Some("DomainEditor"), Some("ProjectEditor")]);
    assert!(main.pages.iter().all(|p| p.title.is_none()));
    assert_eq!(main.pages[0].tells[0].handler, "domainSaved");
}

#[test]
fn the_rules_of_content() {
    has(&MODEL.replace("        main.swap(ProjectEditor)\n", "        main.swap(Nothing)\n"), "main shows no view 'Nothing'");
    has(&MODEL.replace("        main.swap(ProjectEditor)\n", "        main.swap(picked)\n"), "main shows no view 'picked'");
    has(&MODEL.replace("        view ProjectEditor\n", "").replace("    data projectName: text\n", "    data projectName: text\n    data other: text\n").replace("        projectName = picked\n", ""), "main shows no view 'ProjectEditor'");
    has(&MODEL.replace("        view DomainEditor on saved: domainSaved\n        view ProjectEditor\n", ""), "content needs at least one 'view'");
    has(&MODEL.replace("        view ProjectEditor\n", "        view ProjectEditor\n            label \"x\" name own at 1,1 span 2\n"), "holds no parts of its own");
    has(&MODEL.replace("view ProjectEditor\n", "view Nothing\n"), "there is no view 'Nothing'");
}

#[test]
fn placed_the_views_are_pages_and_picking_one_opens_its_page() {
    let placed = clarity_spec::views::placed(&project(MODEL));
    let file = &placed.files[0];
    let main = &file.pages[0].widgets[0];
    let names: Vec<Vec<&str>> = main.pages.iter().map(|p| p.children.iter().filter_map(|w| w.props.name.as_deref()).collect()).collect();
    assert_eq!(names, [vec!["domainHeading", "domainBox", "saveDomainButton"], vec!["projectBox"]]);
    let code = file.code.iter().find(|c| c.name.name == "Workspace").unwrap();
    let opened = |handler: &str| {
        let h = code.handlers.iter().find(|h| h.name.name == handler).unwrap();
        h.body.iter().find_map(|s| match &s.kind {
            StmtKind::Assign { target, value } => match (&target.kind, &value.kind) {
                (ExprKind::Member { member, .. }, ExprKind::Number { value }) if member.name == "tab" => Some(value.clone()),
                _ => None,
            },
            _ => None,
        })
    };
    assert_eq!(opened("projectClicked").as_deref(), Some("2"));
    assert_eq!(opened("domainSaved").as_deref(), Some("1"));
    // The view tells its window through the place: saving runs domainSaved.
    let saved = code.handlers.iter().find(|h| h.name.name == "saveDomainClicked").unwrap();
    assert!(matches!(&saved.body[0].kind, StmtKind::Call { call } if matches!(&call.kind, ExprKind::Call { function, .. } if function.name == "domainSaved")));
}

#[test]
fn a_content_line_writes_itself_back() {
    let project = project(MODEL);
    let main = &project.pages().nth(0).unwrap().widgets[0];
    assert_eq!(clarity_spec::widgets::widget_line(&main.props), "content name main at 1,1 span 8 rows 10");
}

/// The designer adds a view to the content, swaps the one a line shows, and
/// keeps what the window hears of it.
#[test]
fn the_designer_edits_the_views_of_content() {
    use clarity_spec::cxaml::{apply, read_page};
    use clarity_spec::edit::Edit;
    let markup = clarity_spec::converted([("model.clarity", MODEL)]).into_iter().find(|(p, _)| p.ends_with("design/Workspace.cxaml")).map(|(_, t)| t).expect("the window's file");
    let page = read_page(&markup, "").0.expect("the page");
    let main = &page.widgets[0];
    let added = apply(None, &markup, &Edit::AddTab { item: main.span.start, title: "OtherEditor".into() }).unwrap().text;
    assert!(added.contains("    <View name=\"ProjectEditor\" />\n    <View name=\"OtherEditor\" />\n  </Content>"), "{added}");
    let swapped = apply(None, &markup, &Edit::SetPageView { target: main.pages[0].span.start, view: Some("OtherEditor".into()) }).unwrap().text;
    assert!(swapped.contains("    <View name=\"OtherEditor\" onsaved=\"domainSaved\" />\n"), "{swapped}");
    assert!(apply(None, &markup, &Edit::AddTab { item: main.span.start, title: "not a name".into() }).is_err());
}

/// A view declares what it tells, and the content line that shows it wires
/// a tell to a handler of the window; both are designer edits.
#[test]
fn the_designer_edits_what_a_view_tells_and_who_hears_it() {
    use clarity_spec::cxaml::{apply, read_page};
    use clarity_spec::edit::Edit;
    let file = |name: &str| clarity_spec::converted([("model.clarity", MODEL)]).into_iter().find(|(p, _)| p.ends_with(&format!("design/{name}.cxaml"))).map(|(_, t)| t).expect("the file");

    let view = file("ProjectEditor");
    let page = read_page(&view, "").0.expect("the view").span.start;
    let told = apply(None, &view, &Edit::SetPageTell { page, name: "saved".into(), message: Some("saved(name: text)".into()) }).unwrap().text;
    assert!(told.contains("<Tells message=\"saved(name: text)\" />"), "{told}");
    let page = read_page(&told, "").0.expect("the view").span.start;
    let changed = apply(None, &told, &Edit::SetPageTell { page, name: "saved".into(), message: Some("saved".into()) }).unwrap().text;
    assert_eq!(changed.matches("<Tells").count(), 1, "{changed}");
    assert!(changed.contains("<Tells message=\"saved\" />"), "{changed}");
    let page = read_page(&changed, "").0.expect("the view").span.start;
    let gone = apply(None, &changed, &Edit::SetPageTell { page, name: "saved".into(), message: None }).unwrap().text;
    assert!(!gone.contains("<Tells"), "{gone}");
    assert!(apply(None, &view, &Edit::SetPageTell { page, name: "saved".into(), message: Some("closed".into()) }).is_err(), "the message names the tell");
    assert!(apply(None, &view, &Edit::SetPageTell { page, name: "saved".into(), message: Some("saved(name".into()) }).is_err());

    let window = file("Workspace");
    let main = &read_page(&window, "").0.expect("the window").widgets[0];
    let line = main.pages[1].span.start;
    let wired = apply(None, &window, &Edit::SetViewTell { target: line, tell: "saved".into(), handler: Some("projectSaved".into()) }).unwrap().text;
    assert!(wired.contains("<View name=\"ProjectEditor\" onsaved=\"projectSaved\" />"), "{wired}");
    let main = &read_page(&wired, "").0.expect("the window").widgets[0];
    let unwired = apply(None, &wired, &Edit::SetViewTell { target: main.pages[1].span.start, tell: "saved".into(), handler: None }).unwrap().text;
    assert!(unwired.contains("<View name=\"ProjectEditor\" />"), "{unwired}");
    assert!(apply(None, &window, &Edit::SetViewTell { target: line, tell: "saved".into(), handler: Some("not a name".into()) }).is_err());
}

const SHARED: &str = "module model

page Shop: window
    data note: text
    on open: shopOpened
    content name main at 1,1 span 12 rows 8
        view OrderList
    slideout \"Order\" name orderPanel span 6 rows 6
        content name orderContent at 1,1 span 6 rows 6
            view OrderEditor

page OrderList: view
    data picked: text
    button \"Open\" name openButton at 1,1 span 2 on click: openClicked
    content name inner at 1,2 span 12 rows 4
        view OrderTotals

page OrderTotals: view
    data total: integer
    label \"Total\" name totalLabel at 1,1 span 2

page OrderEditor: view
    data orderName: text
    textbox name orderBox at 1,1 span 4 bind orderName

page Card: window
    data customer: text
    content name main at 1,1 span 12 rows 4
        view CardView

page CardView: view
    data shown: text
    label \"Card\" name cardLabel at 1,1 span 2

code Shop
    handler shopOpened
        refreshList()
        open Card with shown = note

code OrderList
    handler openClicked
        orderName = picked
        total = 1
        note = picked
        orderPanel.visible = true
        loaded()

    handler refreshList
        picked = \"\"

code OrderEditor
    handler loaded
        orderBox.enabled = true
";

/// Every window shows its parts in a view in its content, so a view's code
/// reaches what shares its window (the window's parts, data and handlers,
/// those of the other views, deeper ones too), the window's code reaches
/// the views' handlers, and opening a window `with` sets its view's data.
#[test]
fn a_view_reaches_what_shares_its_window_and_views_nest() {
    assert!(errors(SHARED).is_empty(), "{:#?}", errors(SHARED));
    // A name only one of the windows showing a view has is not the view's to reach.
    let two = SHARED.replace("        view CardView\n", "        view CardView\n        view OrderEditor\n").replace("    slideout \"Order\" name orderPanel span 6 rows 6\n        content name orderContent at 1,1 span 6 rows 6\n            view OrderEditor\n", "");
    has(&two, "orderPanel");
    has(&SHARED.replace("open Card with shown = note", "open Card with missing = note"), "Card has no data 'missing'");
}

/// A new window from a template is built window, content, view; a blank one shows an empty view.
#[test]
fn a_new_window_shows_its_parts_in_a_view() {
    use clarity_spec::edit::Edit;
    use clarity_spec::templates::PageTemplate;
    let added = clarity_spec::cxaml::added(&project("module model\n"), std::path::Path::new("model.clarity"), "module model\n", &Edit::AddWindow { name: "Fresh".into(), template: PageTemplate::Blank }).unwrap().unwrap();
    let (window, view) = (&added.files[0].1, &added.files[1].1);
    assert!(window.contains("<Window class=\"model.Fresh\"") && window.contains("  <Content name=\"main\" at=\"1,1\" span=\"12\">\n    <View name=\"FreshView\" />\n"), "{window}");
    assert!(view.contains("<View class=\"model.FreshView\""), "{view}");
}
