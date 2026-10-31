//! Canvas modes (DESIGN 5.31) in the C# program: a canvas with a mode is the
//! runtime's CanvasView with its mode's options, its actions go to the view
//! model's CanvasEvent, which runs `on change` as one edit, and handlers read
//! and set its tool, brush, ink and selection. The build test is slow, so it
//! only runs when asked:
//! `cargo test -p clarity-gen --test csharp_canvas_modes -- --ignored`.

use std::path::Path;
use std::process::Command;

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module m

entity ClassType
    name: text, label \"Name\"
    x: integer
    y: integer

entity Note
    title: text, label \"Title\"
    picture: image
    drawing: text

page Diagram: window
    data types: list<ClassType>
    data moved: integer
    canvas name diagram at 1,1 span 12 rows 10 units 4000 x 3000 zoom grid 20 snap mode objects types at x, y on draw: drawTypes on change: typeMoved

code Diagram
    handler drawTypes
        diagram.clear()
        for t in types
            diagram.object(t, t.x, t.y, 260, 120)
            diagram.rect(t.x, t.y, 260, 120, \"raised\")
            diagram.ellipse(t.x + 20, t.y + 20, 10, 6, \"accent\")
            diagram.text(t.x + 130, t.y + 14, t.name)

    handler typeMoved
        save diagram.selected

page Sketch: window
    data note: Note
    data picked: integer
    canvas name sketch at 1,1 span 6 rows 8 bind note.picture mode paint tools pen, eraser brush 4 ink accent on change: saveNote
    canvas name drawing at 7,1 span 6 rows 8 units 400 x 300 bind note.drawing mode vector tools select, line, rect, ellipse, path, text grid 10 snap on change: saveNote
    button \"Red\" name redButton at 1,9 span 2 on click: red

code Sketch
    handler saveNote
        save note

    handler red
        sketch.ink = \"#ff0000\"
        sketch.brush = sketch.brush + 2
        drawing.tool = \"rect\"
        picked = drawing.selected
";

fn generated() -> Vec<OutFile> {
    let project = clarity_spec::keyed::load_keyed([("m.clarity", SOURCE)], &[], "en");
    let mut settings = starter("Canvases", "canvases");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn has(text: &str, expected: &str) {
    let near: Vec<&str> = text.lines().filter(|l| l.len() < 400 && (l.contains("Canvas") || l.contains("PartState(") || l.contains("Diagram.") || l.contains("Sketch.") || l.contains("Drawing."))).collect();
    assert!(text.contains(expected), "expected\n{expected}\nin the generated program, whose lines about the canvases are\n{}", near.join("\n"));
}

#[test]
fn a_canvas_with_a_mode_is_the_runtimes_canvas_with_its_options() {
    let text = generated().iter().map(|f| format!("// {}\n{}", f.path, f.text)).collect::<Vec<_>>().join("\n");
    // Objects: the drawing marks each record's box, the drop goes to the runtime.
    has(&text, "Diagram.Draw(new Shape.Object(t.Id, ");
    has(&text, "Diagram.Draw(new Shape.Ellipse(");
    has(&text, " Mode=\"objects\" ActionCommand=\"{Binding CanvasAt_DiagramCommand}\"");
    has(&text, "void CanvasAt_Diagram(System.Text.Json.Nodes.JsonObject action) => CanvasEvent(Diagram, \"diagram\", new CanvasArgs(\"objects\") { Grid = Diagram.Grid, Snap = Diagram.Snap, List = \"Types\", At = [\"X\", \"Y\"], Units = [4000, 3000] }, action, OnTypeMoved);");
    has(&text, "Db.Save(Selected<ClassType>(Diagram))");
    // Paint: the picture of the bound field, its tools, brush and ink.
    has(&text, " Mode=\"paint\" ActionCommand=\"{Binding CanvasAt_SketchCommand}\"");
    has(&text, " Tools=\"pen,eraser\" Tool=\"{Binding Sketch.Tool}\" Brush=\"{Binding Sketch.Brush}\" Ink=\"{Binding Sketch.Ink}\" Picture=\"{Binding Note.Picture}\" Fetch=\"{Binding NotePicturePicture}\"");
    has(&text, "Sketch { get; } = new(\"sketch\") { Tool = \"pen\", Brush = 4, Ink = \"accent\" }");
    has(&text, "new CanvasArgs(\"paint\") { Grid = Sketch.Grid, Snap = Sketch.Snap, Path = \"Note.Picture\" }");
    has(&text, "public Func<Task<byte[]?>> NotePicturePicture => ");
    // Vector: the SVG of the bound text field, its chosen shape.
    has(&text, " Svg=\"{Binding Note.Drawing}\" Chosen=\"{Binding Drawing.SelectedItem}\"");
    has(&text, "Path = \"Note.Drawing\", Units = [400, 300] }");
    // What handlers set and read.
    has(&text, "Sketch.Ink = \"#ff0000\"");
    has(&text, "Sketch.Brush = (Sketch.Brush + 2L)");
    has(&text, "Drawing.Tool = \"rect\"");
    has(&text, "Picked = Drawing.SelectedShape");
}

/// Where builds go: target/generated in the repository, never the system's temporary folder.
fn scratch() -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated");
    std::fs::create_dir_all(&dir).expect("target/generated can be made");
    dir
}

#[test]
#[ignore = "builds a C# program; run with --ignored"]
fn canvases_with_modes_build_in_csharp_without_warnings() {
    let out = scratch().join(format!("clarity-gen-csharp-canvas-modes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &generated(), &["bin", "obj"]).unwrap();
    // Nothing outlives the build: no compiler server, no build nodes.
    let built = Command::new("dotnet")
        .args(["build", "--nologo", "-v", "quiet", "-warnaserror", "-p:UseSharedCompilation=false", "-nodeReuse:false", "-p:GeneratePackageOnBuild=false"])
        .current_dir(&out)
        .env("CLARITY_RUNTIME", Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp"))
        .output()
        .expect("the .NET SDK is installed");
    assert!(built.status.success(), "the generated project does not build; it is in {}\n{}", out.display(), String::from_utf8_lossy(&built.stdout));
    std::fs::remove_dir_all(&out).unwrap();
}
