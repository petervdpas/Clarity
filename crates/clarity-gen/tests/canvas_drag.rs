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
    canvas name sheet at 1,1 span 12 rows 8 units 3000 x 2000 zoom grid 20 snap background panel dots 3 muted on draw: drawSheet on drag: dragged on drop: dropped

code Main
    handler drawSheet
        sheet.clear(\"panel\")
        sheet.rect(boxX, boxY, 200, 100, \"raised\")

    handler dragged
        boxX = boxX + sheet.x - sheet.fromX
        boxY = boxY + sheet.y - sheet.fromY

    handler dropped
        moved = moved + 1
        if sheet.snap
            boxX = snapped(boxX, sheet.grid)
        sheet.snap = false
        sheet.grid = 40
        sheet.background = \"#f5f0e1\"
        sheet.dotSize = sheet.dotSize + 1
        sheet.dotColor = \"text\"
        if sheet.background == \"panel\"
            moved = 0
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
    let near: Vec<&str> = text.lines().filter(|l| l.len() < 200 && (l.contains("sheet") || l.contains("Sheet"))).collect();
    assert!(text.contains(expected), "expected\n{expected}\nin the generated program, whose lines about the canvas are\n{}", near.join("\n"));
}

#[test]
fn every_stack_reads_where_a_drag_started() {
    let rust = generated(Backend::Rust, Ui::Web);
    has(&rust, "ui.from_x(\"sheet\")");
    has(&rust, "clarity_runtime::snapped(");
    has(&rust, "ui.snap(\"sheet\", true)");
    has(&rust, "ui.grid(\"sheet\", 20)");
    has(&rust, "ui.set_snap(\"sheet\", false)");
    has(&rust, "ui.set_grid(\"sheet\", ");
    has(&rust, "ui.set_background(\"sheet\", ");
    has(&rust, "ui.background(\"sheet\", \"panel\")");
    has(&rust, "ui.dot_size(\"sheet\", 3)");
    has(&rust, "ui.set_dot_color(\"sheet\", ");
    let go = generated(Backend::Go, Ui::Web);
    has(&go, "ui.FromX(\"sheet\")");
    has(&go, "clarity.Snapped(");
    has(&go, "ui.Snap(\"sheet\", true)");
    has(&go, "ui.SetGrid(\"sheet\", ");
    has(&go, "ui.SetBackground(\"sheet\", ");
    has(&go, "ui.DotSize(\"sheet\", 3)");
    let c = generated(Backend::C, Ui::Web);
    has(&c, "clarity_ui_from_x(ui, \"sheet\")");
    has(&c, "clarity_snapped(");
    has(&c, "clarity_ui_snap(ui, \"sheet\", true)");
    has(&c, "clarity_ui_set_grid(ui, \"sheet\", ");
    has(&c, "clarity_ui_set_background(ui, \"sheet\", ");
    has(&c, "clarity_ui_set_dot_size(ui, \"sheet\", ");
    let cpp = generated(Backend::Cpp, Ui::Web);
    has(&cpp, "ui.from_x(\"sheet\")");
    has(&cpp, "clarity::snapped(");
    has(&cpp, "ui.snap(\"sheet\", true)");
    has(&cpp, "ui.set_grid(\"sheet\", ");
    has(&cpp, "ui.set_background(\"sheet\", ");
    has(&cpp, "ui.set_dot_color(\"sheet\", ");
    let csharp = generated(Backend::Csharp, Ui::Avalonia);
    has(&csharp, " Zoom=\"True\" Grid=\"{Binding Sheet.Grid}\" Paper=\"{Binding Sheet.Background}\" DotSize=\"{Binding Sheet.DotSize}\" DotColor=\"{Binding Sheet.DotColor}\"");
    has(&csharp, "Sheet.Background = ");
    has(&csharp, "Numbers.Snapped(");
    has(&csharp, "Sheet.Snap = false");
    has(&csharp, "Sheet.Grid = 40");
    // The line's grid and snap are where the part's state starts.
    has(&csharp, "{ Grid = 20, Snap = true, Background = \"panel\", DotSize = 3, DotColor = \"muted\" }");
    has(&csharp, " DragCommand=\"{Binding DragAt_SheetCommand}\"");
    has(&csharp, " DropCommand=\"{Binding DropAt_SheetCommand}\"");
    has(&csharp, "Sheet.FromX");
}
