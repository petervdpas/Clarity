//! Dragging on a canvas, on every stack: `on drag` and `on drop` run their
//! handlers, which read where the pointer is (`x`, `y`) and where it was
//! pressed (`fromX`, `fromY`), in the canvas's units.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module diagram

page Main: window
    data boxX: integer, default 20
    data boxY: integer, default 20
    data moved: integer
    canvas name sheet at 1,1 span 12 rows 8 units 3000 x 2000 zoom on draw: drawSheet on drag: dragged on drop: dropped

code Main
    handler drawSheet
        sheet.clear(\"panel\")
        sheet.rect(boxX, boxY, 200, 100, \"raised\")

    handler dragged
        boxX = boxX + sheet.x - sheet.fromX
        boxY = boxY + sheet.y - sheet.fromY

    handler dropped
        moved = moved + 1
";

fn generated(backend: Backend, ui: Ui) -> String {
    let project = clarity_spec::keyed::load_keyed([("diagram.clarity", SOURCE)], &[], "en");
    let mut settings = starter("Diagram", "diagram");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}", f.path, f.text)).collect::<Vec<_>>().join("\n")
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin the generated program");
}

#[test]
fn every_stack_reads_where_a_drag_started() {
    has(&generated(Backend::Rust, Ui::Web), "ui.from_x(\"sheet\")");
    has(&generated(Backend::Go, Ui::Web), "ui.FromX(\"sheet\")");
    has(&generated(Backend::C, Ui::Web), "clarity_ui_from_x(ui, \"sheet\")");
    has(&generated(Backend::Cpp, Ui::Web), "ui.from_x(\"sheet\")");
    let csharp = generated(Backend::Csharp, Ui::Avalonia);
    has(&csharp, " Zoom=\"True\"");
    has(&csharp, " DragCommand=\"{Binding DragAt_SheetCommand}\"");
    has(&csharp, " DropCommand=\"{Binding DropAt_SheetCommand}\"");
    has(&csharp, "Sheet.FromX");
}
