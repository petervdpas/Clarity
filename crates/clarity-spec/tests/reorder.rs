//! Reordering (DESIGN 5.30): `reorder by field` lets the person put the rows
//! of a list, a table, a tree or a tree table in their own order, by dragging
//! a row or with Alt and the arrow keys. The screen numbers the rows in the
//! integer field and `on reorder` saves them. It is an option: without it the
//! rows stay where the data has them.

use clarity_spec::widgets::{self, Event, WidgetKind};
use clarity_spec::{Severity, load_sources};

const APP: &str = "module m

entity Task
    title: text, label \"Title\"
    position: integer
    done: bool

entity Folder
    name: text(80), required
    position: integer
    parent: ref Folder
    children: list<Folder>

page Main: window
    data tasks: list<Task>
    data folders: list<Folder>
    list name taskList at 1,1 span 4 rows 6 bind tasks show title reorder by position on reorder: moved
    table name taskGrid at 5,1 span 8 rows 6 bind tasks columns title, done reorder by position on reorder: moved
    tree name folderTree at 1,7 span 4 rows 6 bind folders children children reorder by position on reorder: folderMoved
    treetable name folderTable at 5,7 span 8 rows 6 bind folders children children columns name reorder by position on reorder: folderMoved

code Main
    handler moved
        for t in tasks
            save t

    handler folderMoved
        for f in folders
            save f
";

fn diagnostics(source: &str, severity: Severity) -> Vec<String> {
    load_sources([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == severity).map(|d| d.message).collect()
}

fn errors(source: &str) -> Vec<String> {
    diagnostics(source, Severity::Error)
}

#[test]
fn a_list_a_table_a_tree_and_a_tree_table_reorder_as_an_option() {
    assert!(errors(APP).is_empty(), "{:#?}", errors(APP));
    assert!(diagnostics(APP, Severity::Warning).is_empty(), "{:#?}", diagnostics(APP, Severity::Warning));
    let project = load_sources([("m.clarity", APP)]);
    let widgets = &project.files[0].pages[0].widgets;
    for w in widgets {
        assert_eq!(w.props.reorder.as_deref(), Some("position"), "{:?}", w.props.name);
        assert_eq!(w.props.handlers[0].event, Event::Reorder);
        // The designer writes it back as it is written.
        assert!(widgets::widget_line(&w.props).contains("reorder by position"), "{}", widgets::widget_line(&w.props));
    }
    for kind in [WidgetKind::List, WidgetKind::Table, WidgetKind::Tree, WidgetKind::TreeTable] {
        assert!(widgets::has_property(kind, widgets::WidgetProperty::Reorder), "{kind:?}");
        assert!(widgets::has_event(kind, Event::Reorder), "{kind:?}");
    }
    assert_eq!(Event::by_keyword("reorder"), Some(Event::Reorder));
    // Without the option the rows stay where the data has them.
    let plain = APP.replace(" reorder by position on reorder: moved", "").replace(" reorder by position on reorder: folderMoved", "");
    let project = load_sources([("m.clarity", plain.as_str())]);
    assert!(project.files[0].pages[0].widgets.iter().all(|w| w.props.reorder.is_none()));
}

#[test]
fn the_field_is_an_integer_of_the_listed_entity() {
    let missing = errors(&APP.replace("show title reorder by position", "show title reorder by place"));
    assert!(missing.iter().any(|e| e.contains("no field 'place'")), "{missing:#?}");
    let text = errors(&APP.replace("show title reorder by position", "show title reorder by title"));
    assert!(text.iter().any(|e| e.contains("'title' is text") && e.contains("integer")), "{text:#?}");
}

/// A table's rows follow either the person's order or a sort, not both.
#[test]
fn a_reordered_table_is_not_sorted() {
    let sorted = errors(&APP.replace("columns title, done reorder", "columns title, done sortable reorder"));
    assert!(sorted.iter().any(|e| e.contains("'sortable'") && e.contains("reorder")), "{sorted:#?}");
    let ordered = errors(&APP.replace("columns title, done reorder", "columns title, done order by title reorder"));
    assert!(ordered.iter().any(|e| e.contains("'order by'") && e.contains("reorder")), "{ordered:#?}");
}

/// Like a table that edits, a part that reorders without `on reorder` loses its order.
#[test]
fn reordering_without_a_handler_is_a_warning() {
    let source = APP.replace("show title reorder by position on reorder: moved", "show title reorder by position");
    let warnings = diagnostics(&source, Severity::Warning);
    assert!(warnings.iter().any(|w| w.contains("list at 1,1") && w.contains("'on reorder'")), "{warnings:#?}");
}

/// A reorder is an edit: Ctrl+Z takes it back, unless the part says `no undo`.
#[test]
fn a_reorder_is_an_edit_with_its_undo() {
    let project = load_sources([("m.clarity", APP)]);
    let widgets = &project.files[0].pages[0].widgets;
    assert_eq!(widgets[0].props.undo_scope.as_deref(), Some("taskList"));
    assert_eq!(widgets[2].props.undo_scope.as_deref(), Some("folderTree"));
    let off = APP.replace("show title reorder by position on reorder: moved", "show title reorder by position no undo on reorder: moved");
    assert!(errors(&off).is_empty(), "{:#?}", errors(&off));
    let project = load_sources([("m.clarity", off.as_str())]);
    assert_eq!(project.files[0].pages[0].widgets[0].props.undo_scope, None);
    // A list that does not reorder only selects: it has no undo.
    let plain = load_sources([("m.clarity", APP.replace(" reorder by position on reorder: moved", "").as_str())]);
    assert_eq!(plain.files[0].pages[0].widgets[0].props.undo_scope, None);
}

#[test]
fn reorder_is_a_word_of_the_language() {
    assert!(clarity_spec::highlight::keywords().iter().any(|w| w == "reorder"), "reorder is highlighted");
}
