//! A diagram's toolbox on every stack: a canvas of objects in a view, with a
//! toolbox page whose tools code shows and hides, `on connect`, and
//! `canvas.tool` set from code.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module diagram

record Box
    id: integer
    x: integer
    y: integer

page Main: window
    slideout name diagramPanel side bottom height 300 span 12 rows 6
        content name diagramContent at 1,1 span 12 rows 6
            view Diagram

page Diagram: view
    data boxes: list<Box>
    data erd: bool
    canvas name sheet at 1,1 span 12 rows 5 units 4000 x 3000 zoom mode objects boxes at x, y tools select, connect toolbox DiagramTools on draw: drawn on change: moved on connect: connected

code Diagram
    handler drawn
        sheet.clear()

    handler moved
        let b = sheet.selected

    handler connected
        newTool.visible = not erd
        sheet.tool = \"select\"

page DiagramTools: toolbox
    grid 2 columns
    tool name selectTool at 1,1 icon \"mouse-pointer-2\" does select
    tool name connectTool at 2,1 icon \"spline\" does connect
    tool name newTool at 1,2 icon \"plus\" on click: connected
";

fn generated(backend: Backend) -> String {
    let project = clarity_spec::keyed::load_keyed([("diagram.clarity", SOURCE)], &[], "en");
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings = starter("Diagram", "diagram");
    settings.stack.backend = backend;
    settings.stack.ui = Ui::Clarity;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
    files.iter().map(|f| format!("// {}\n{}", f.path, f.text)).collect::<Vec<_>>().join("\n")
}

fn has(text: &str, expected: &str) {
    let near: Vec<&str> = text.lines().filter(|l| l.len() < 220 && (l.contains("newTool") || l.contains("sheet") || l.contains("Tool"))).collect();
    assert!(text.contains(expected), "expected\n{expected}\nin the generated program, whose lines about the diagram are\n{}", near.join("\n"));
}

#[test]
fn every_stack_shows_a_diagram_s_toolbox_and_sets_its_tools() {
    let rust = generated(Backend::Rust);
    has(&rust, "ui.set_visible(\"newTool\"");
    has(&rust, "ui.set_tool(\"sheet\"");
    has(&rust, "does=\"connect\"");
    let go = generated(Backend::Go);
    has(&go, "ui.SetVisible(\"newTool\"");
    has(&go, "ui.SetTool(\"sheet\"");
    let c = generated(Backend::C);
    has(&c, "clarity_ui_set_visible(ui, \"newTool\"");
    has(&c, "clarity_ui_set_tool(ui, \"sheet\"");
    let cpp = generated(Backend::Cpp);
    has(&cpp, "ui.set_visible(\"newTool\"");
    has(&cpp, "ui.set_tool(\"sheet\"");
    let csharp = generated(Backend::Csharp);
    has(&csharp, "newTool");
    has(&csharp, "does=\"connect\"");
}
