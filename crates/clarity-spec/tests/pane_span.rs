//! A splitter's first pane can start at a width of its own (`pane span 4`):
//! its grid has those columns, the second pane the rest, and the person
//! drags the divider between them.

use clarity_spec::{Severity, load_in};

fn page(first: &str, second: &str) -> String {
    format!(
        "module m\n\npage Main: window\n    splitter name split at 1,1 span 12 rows 4\n        {first}\n            text name a at 1,1 span 4\n        {second}\n            text name b at 1,1 span 8\n"
    )
}

fn errors(source: &str) -> Vec<String> {
    let project = load_in(clarity_spec::converted([("m.clarity", source)]).iter().map(|(p, t)| (p.as_str(), t.as_str())), [], &[]);
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

#[test]
fn the_first_pane_has_its_own_width_and_the_second_the_rest() {
    let source = page("pane span 4", "pane");
    assert!(errors(&source).is_empty(), "{:#?}", errors(&source));
    let (file, _) = clarity_spec::parser::parse(&source, "m.clarity");
    let splitter = &file.pages[0].widgets[0];
    assert_eq!(splitter.pages[0].columns, Some(4));
    assert_eq!(clarity_spec::widgets::pane_columns(splitter), [4, 8]);
}

#[test]
fn panes_without_a_width_each_have_the_splitter_s() {
    let source = page("pane", "pane");
    let (file, _) = clarity_spec::parser::parse(&source, "m.clarity");
    assert_eq!(clarity_spec::widgets::pane_columns(&file.pages[0].widgets[0]), [12, 12]);
}

#[test]
fn a_part_must_fit_its_pane() {
    let found = errors(&page("pane span 3", "pane"));
    assert!(found.iter().any(|m| m.contains("runs past column 3")), "{found:#?}");
}

#[test]
fn only_the_first_pane_of_a_splitter_side_by_side_has_a_width_and_leaves_room() {
    assert!(errors(&page("pane", "pane span 4")).iter().any(|m| m.contains("only the first pane")));
    assert!(errors(&page("pane span 12", "pane")).iter().any(|m| m.contains("1 to 11")));
    let vertical = page("pane span 4", "pane").replace("rows 4\n", "rows 4 vertical\n");
    assert!(errors(&vertical).iter().any(|m| m.contains("side by side")), "{:#?}", errors(&vertical));
}

/// The designer sets a pane's width (its Width field): `span="4"` on the
/// pane in the window's file, or back to none.
#[test]
fn the_designer_sets_the_first_pane_s_width() {
    use clarity_spec::edit::Edit;
    let markup = design(&page("pane", "pane"), "Main");
    let pane = |markup: &str| clarity_spec::cxaml::read_page(markup, "").0.expect("the page").widgets[0].pages[0].span.start;
    let out = clarity_spec::cxaml::apply(None, &markup, &Edit::SetPaneSpan { target: pane(&markup), columns: Some(4) }).unwrap();
    assert!(out.text.contains("    <Pane span=\"4\">\n      <Text name=\"a\""), "{}", out.text);
    let back = clarity_spec::cxaml::apply(None, &out.text, &Edit::SetPaneSpan { target: pane(&out.text), columns: None }).unwrap();
    assert_eq!(back.text, markup);
}

/// The window `page` of `module`, in its own file as a project holds it.
fn design(module: &str, page: &str) -> String {
    let file = format!("design/{page}.cxaml");
    clarity_spec::converted([("m.clarity", module)]).into_iter().find(|(p, _)| p.ends_with(&file)).map(|(_, t)| t).expect("the page's file")
}
