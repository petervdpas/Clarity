//! Editable table columns: `edit done` turns a column's cells into inputs
//! that change the row on screen; `on change` runs with that row selected,
//! so a handler can save it.

use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, Event, WidgetKind};
use clarity_spec::{Severity, load_sources};

const TODO: &str = "module todo

enum Priority
    Low
    High

entity Person
    name: text(80)

entity Task
    title: text(120), required
    done: bool
    priority: Priority
    due: date
    owner: ref Person
    added: datetime
    spent: duration

page Main: window
    data tasks: list<Task>
    table name grid at 1,1 span 12 rows 6 bind tasks columns title, done, priority edit done, priority on change: task_changed
";

const CODE: &str = "
code Main
    handler task_changed
        save grid.selected
";

fn messages(text: &str, severity: Severity) -> Vec<String> {
    let text = format!("{text}{CODE}");
    load_sources([("todo.clarity", text.as_str())])
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == severity)
        .map(|d| d.message)
        .collect()
}

#[test]
fn edit_lists_the_columns_whose_cells_are_inputs() {
    let (file, diagnostics) = parse(TODO, "todo.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let props = &file.pages[0].widgets[0].props;
    assert_eq!(props.edit, ["done", "priority"]);
    // Written back after the columns.
    assert_eq!(
        widgets::widget_line(props),
        "table name grid at 1,1 span 12 rows 6 bind tasks columns title, done, priority edit done, priority on change: task_changed"
    );
    assert!(widgets::has_event(WidgetKind::Table, Event::Change));
}

#[test]
fn a_table_with_edits_and_a_change_handler_is_clean() {
    assert_eq!(messages(TODO, Severity::Error), Vec::<String>::new());
    assert_eq!(messages(TODO, Severity::Warning), Vec::<String>::new());
}

#[test]
fn only_shown_columns_of_editable_types_can_be_edited() {
    let not_shown = TODO.replace("edit done, priority", "edit due");
    assert!(messages(&not_shown, Severity::Error).contains(&"table at 1,1: 'due' is edited but is not one of the columns".to_string()));

    // A reference is a dropdown of the records it can point at.
    let reference = TODO.replace("columns title, done, priority edit done, priority", "columns title, owner edit owner");
    assert_eq!(messages(&reference, Severity::Error), Vec::<String>::new());

    // A moment is picked like a date, with its time.
    let moment = TODO.replace("columns title, done, priority edit done, priority", "columns title, added edit added");
    assert_eq!(messages(&moment, Severity::Error), Vec::<String>::new());

    let length = TODO.replace("columns title, done, priority edit done, priority", "columns title, spent edit spent");
    assert_eq!(
        messages(&length, Severity::Error),
        ["table at 1,1: 'spent' is duration; a table edits text, numbers, bool, dates, enum and ref columns"]
    );
}

#[test]
fn edits_that_nothing_saves_are_a_warning() {
    let unsaved = TODO.replace(" on change: task_changed", "");
    assert!(messages(&unsaved, Severity::Warning).contains(&"table at 1,1 edits its rows but has no 'on change' handler to save them".to_string()));
}

#[test]
fn a_list_part_has_no_edit() {
    let list = TODO.replace(
        "table name grid at 1,1 span 12 rows 6 bind tasks columns title, done, priority edit done, priority on change: task_changed",
        "list at 1,1 span 4 rows 6 bind tasks show title edit title",
    );
    let (_, diagnostics) = parse(&list, "todo.clarity");
    assert!(diagnostics.iter().any(|d| d.message.contains("'edit' does not apply to a list")), "{diagnostics:?}");
}

#[test]
fn an_edited_bool_column_can_be_a_switch() {
    let switched = TODO.replace("edit done, priority", "edit done as switch, priority");
    let (file, diagnostics) = parse(&switched, "todo.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let props = &file.pages[0].widgets[0].props;
    assert_eq!(props.edit, ["done", "priority"]);
    assert_eq!(props.switches, ["done"]);
    // Written back as it was written.
    assert!(widgets::widget_line(props).contains(" edit done as switch, priority "));
    assert_eq!(messages(&switched, Severity::Error), Vec::<String>::new());

    let not_bool = TODO.replace("edit done, priority", "edit done, priority as switch");
    assert_eq!(messages(&not_bool, Severity::Error), ["table at 1,1: 'priority' is enum; only a bool column can be a switch"]);
}
