//! `fold buttons`: a tree or tree table shows Expand all and Collapse all
//! above its rows. A property of the part like any other: its line, its
//! window file's `foldButtons="true"`, the designer's property, and code
//! reads and sets it as `tree.foldButtons`.

use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind, WidgetProperty};
use clarity_spec::{Severity, load_converted};

const WINDOW: &str = "module m

entity Folder
    name: text
    parentId: integer

page Main: window
    data folders: list<Folder>
    tree name folderTree at 1,1 span 6 rows 6 bind folders show name key id parent parentId fold buttons
    treetable name folderTable at 7,1 span 6 rows 6 bind folders columns name key id parent parentId fold buttons
    button \"Hide\" name hideButtons at 1,7 span 2 on click: hideClicked

code Main
    handler hideClicked
        folderTree.foldButtons = not folderTree.foldButtons
        folderTable.foldButtons = false
";

fn errors(source: &str) -> Vec<String> {
    load_converted([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn fold_buttons_read_and_write_back_on_trees_and_tree_tables() {
    let (file, diagnostics) = parse(WINDOW, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let parts = &file.pages[0].widgets;
    assert!(parts[0].props.fold_buttons && parts[1].props.fold_buttons);
    assert!(widgets::widget_line(&parts[0].props).ends_with("fold buttons"), "{}", widgets::widget_line(&parts[0].props));
    for kind in [WidgetKind::Tree, WidgetKind::TreeTable] {
        assert!(widgets::has_property(kind, WidgetProperty::FoldButtons), "{kind:?}");
    }
    assert!(!widgets::has_property(WidgetKind::Table, WidgetProperty::FoldButtons), "a plain table has no rows to fold");
}

#[test]
fn code_reads_and_sets_fold_buttons_of_a_tree_only() {
    assert!(errors(WINDOW).is_empty(), "{:#?}", errors(WINDOW));
    let on_a_button = WINDOW.replace("folderTable.foldButtons = false", "hideButtons.foldButtons = false");
    assert!(!errors(&on_a_button).is_empty(), "a button has no fold buttons");
    let on_a_table = WINDOW.replace("treetable name folderTable at 7,1 span 6 rows 6 bind folders columns name key id parent parentId fold buttons", "table name folderTable at 7,1 span 6 rows 6 bind folders columns name key id parent parentId fold buttons");
    assert!(!errors(&on_a_table).is_empty(), "a plain table refuses the line");
}
