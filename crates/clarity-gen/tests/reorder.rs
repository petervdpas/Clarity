//! Reordering on Avalonia (DESIGN 5.30): a table, tree or tree table says
//! which property its rows are numbered in, a list gets the same through
//! RowMoves, and `on reorder` runs as an edit, so Ctrl+Z takes it back.

use clarity_gen::generate;
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

const APP: &str = "module todo

entity Task
    title: text, label \"Title\"
    position: integer
    parent: ref Task
    children: list<Task>

page Main: window
    data tasks: list<Task>
    list name taskList at 1,1 span 4 rows 6 bind tasks show title reorder by position on reorder: moved
    table name taskGrid at 5,1 span 8 rows 6 bind tasks columns title reorder by position on reorder: moved
    treetable name taskTree at 1,7 span 12 rows 6 bind tasks children children columns title reorder by position no undo on reorder: moved

code Main
    handler moved
        for t in tasks
            save t
";

fn window() -> (String, String) {
    let project = load_keyed([("todo.clarity", APP)], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings: Settings = starter("Todo", "todo");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let files = generate(&project, &settings).unwrap_or_else(|errors| panic!("{errors:#?}"));
    let text = |path: &str| files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text.clone();
    (text("Windows/MainView.axaml"), text("Windows/MainView.axaml.cs"))
}

#[test]
fn each_part_that_reorders_says_its_field_and_runs_on_reorder_as_an_edit() {
    let (view, code) = window();
    assert!(view.contains("<ListBox x:Name=\"taskList\"") && view.contains("c:RowMoves.By=\"Position\""), "{view}");
    assert!(view.contains("ReorderBy=\"Position\" TreeParent=\"Parent\""), "{view}");
    assert!(code.contains("RowMoves.OnReorder(taskList, () => Vm.EditOf(\"taskList\", \"taskList\", true, () => Vm.MovedCommand.Execute(null)));"), "{code}");
    assert!(code.contains("taskGrid.Reordered += (_, _) => Vm.EditOf(\"taskGrid\", \"taskGrid\", true, () => Vm.MovedCommand.Execute(null));"), "{code}");
    // `no undo`: the handler runs, but as no edit.
    assert!(code.contains("taskTree.Reordered += (_, _) => Vm.MovedCommand.Execute(null);"), "{code}");
}
