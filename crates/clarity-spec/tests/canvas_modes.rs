//! Canvas modes (DESIGN 5.31), one option with shared parameters. `mode`
//! says what the person does on the canvas: draw (the default: code draws),
//! `objects` (the records of a list, dragged), `paint` (a picture, bound to an
//! image field) or `vector` (shapes, bound to a text field kept as SVG). The
//! modes share `tools`, `brush`, `ink`, `grid` and `snap`, one event (`on
//! change`, an edit of the canvas ended) and `selected` (what it touched).

use clarity_spec::widgets::{self, Event, WidgetKind, WidgetProperty};
use clarity_spec::{Severity, load_sources};

const APP: &str = "module m

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
    canvas name diagram at 1,1 span 12 rows 10 units 4000 x 3000 zoom grid 20 snap mode objects types at x, y on draw: drawTypes on change: typeMoved

code Diagram
    handler drawTypes
        diagram.clear()
        for t in types
            diagram.object(t, t.x, t.y, 260, 120)
            diagram.rect(t.x, t.y, 260, 120, \"raised\")
            diagram.text(t.x + 130, t.y + 14, t.name)

    handler typeMoved
        save diagram.selected

page Sketch: window
    data note: Note
    canvas name sketch at 1,1 span 6 rows 8 bind note.picture mode paint tools pen, eraser brush 4 ink accent on change: saveNote
    canvas name drawing at 7,1 span 6 rows 8 bind note.drawing mode vector tools select, line, rect, ellipse, path, text grid 10 snap on change: saveNote
    button \"Red\" name redButton at 1,9 span 2 on click: red

code Sketch
    handler saveNote
        save note

    handler red
        sketch.ink = \"#ff0000\"
        sketch.brush = sketch.brush + 2
        drawing.tool = \"rect\"
        let chosen = drawing.selected
";

fn diagnostics(source: &str, severity: Severity) -> Vec<String> {
    load_sources([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == severity).map(|d| d.message).collect()
}

fn errors(source: &str) -> Vec<String> {
    diagnostics(source, Severity::Error)
}

#[test]
fn one_mode_option_with_shared_parameters() {
    assert!(errors(APP).is_empty(), "{:#?}", errors(APP));
    assert!(diagnostics(APP, Severity::Warning).is_empty(), "{:#?}", diagnostics(APP, Severity::Warning));
    let project = load_sources([("m.clarity", APP)]);
    let diagram = &project.files[0].pages[0].widgets[0].props;
    assert_eq!(diagram.mode.map(|m| m.keyword()), Some("objects"));
    let objects = diagram.objects.as_ref().expect("objects");
    assert_eq!((objects.list.as_str(), objects.x.as_str(), objects.y.as_str()), ("types", "x", "y"));
    assert!(diagram.handlers.iter().any(|h| h.event == Event::Change));
    let sketch = &project.files[0].pages[1].widgets[0].props;
    assert_eq!(sketch.mode.map(|m| m.keyword()), Some("paint"));
    assert_eq!((sketch.tools.clone(), sketch.brush, sketch.ink.as_deref()), (vec!["pen".to_string(), "eraser".to_string()], Some(4), Some("accent")));
    let drawing = &project.files[0].pages[1].widgets[1].props;
    assert_eq!(drawing.mode.map(|m| m.keyword()), Some("vector"));
    // The designer writes them back as they are written.
    assert!(widgets::widget_line(diagram).contains("mode objects types at x, y"), "{}", widgets::widget_line(diagram));
    let line = widgets::widget_line(sketch);
    assert!(line.contains("mode paint") && line.contains("tools pen, eraser") && line.contains("brush 4") && line.contains("ink accent"), "{line}");
    for property in [WidgetProperty::Mode, WidgetProperty::Tools, WidgetProperty::Brush, WidgetProperty::Ink] {
        assert!(widgets::has_property(WidgetKind::Canvas, property), "{property:?}");
    }
    assert!(widgets::has_event(WidgetKind::Canvas, Event::Change));
    // A canvas without a mode is drawn by code, as before.
    let plain = load_sources([("m.clarity", APP.replace(" mode objects types at x, y on draw: drawTypes on change: typeMoved", " on draw: drawTypes").replace("    handler typeMoved\n        save diagram.selected\n", "").replace("            diagram.object(t, t.x, t.y, 260, 120)\n", "").as_str())]);
    assert!(plain.files[0].pages[0].widgets[0].props.mode.is_none());
}

#[test]
fn objects_are_records_of_a_list_with_integer_places() {
    let missing = errors(&APP.replace("mode objects types at x, y", "mode objects shapes at x, y"));
    assert!(missing.iter().any(|e| e.contains("shapes")), "{missing:#?}");
    let text = errors(&APP.replace("mode objects types at x, y", "mode objects types at name, y"));
    assert!(text.iter().any(|e| e.contains("'name'") && e.contains("integer")), "{text:#?}");
    // An edit lost: a mode without `on change` keeps nothing.
    let warnings = diagnostics(&APP.replace(" on change: typeMoved", "").replace("    handler typeMoved\n        save diagram.selected\n", ""), Severity::Warning);
    assert!(warnings.iter().any(|w| w.contains("'on change'")), "{warnings:#?}");
    // `object` and `selected` belong to a canvas of objects.
    let drawn = errors(&APP.replace(" mode objects types at x, y", ""));
    assert!(!drawn.is_empty(), "object() and on change on a canvas drawn by code: {drawn:#?}");
}

/// The canvas follows a dragged object by its id. A stored entity has one;
/// a record (built in code, as boxes of several kinds) must give one.
#[test]
fn objects_that_are_records_carry_an_id() {
    let record = APP.replace("entity ClassType\n    name: text, label \"Name\"", "record ClassType\n    name: text");
    let missing = errors(&record.replace("    handler typeMoved\n        save diagram.selected\n", "    handler typeMoved\n        let t = diagram.selected\n"));
    assert!(missing.iter().any(|e| e.contains("ClassType") && e.contains("'id'")), "{missing:#?}");
    let with_id = errors(&record.replace("record ClassType\n    name: text", "record ClassType\n    id: integer\n    name: text").replace("    handler typeMoved\n        save diagram.selected\n", "    handler typeMoved\n        let t = diagram.selected\n"));
    assert!(!with_id.iter().any(|e| e.contains("'id'")), "{with_id:#?}");
}

#[test]
fn paint_binds_a_picture_and_vector_a_text_kept_as_svg() {
    let unbound = errors(&APP.replace(" bind note.picture mode paint", " mode paint"));
    assert!(unbound.iter().any(|e| e.contains("paint") && e.contains("image")), "{unbound:#?}");
    let vector_on_picture = errors(&APP.replace("bind note.drawing mode vector", "bind note.picture mode vector"));
    assert!(vector_on_picture.iter().any(|e| e.contains("vector") && e.contains("text")), "{vector_on_picture:#?}");
    let bound_drawn = errors(&APP.replace(" mode paint tools pen, eraser brush 4 ink accent", ""));
    assert!(bound_drawn.iter().any(|e| e.contains("mode")), "{bound_drawn:#?}");
    let wrong_tool = errors(&APP.replace("tools pen, eraser", "tools pen, path"));
    assert!(wrong_tool.iter().any(|e| e.contains("path") && e.contains("paint")), "{wrong_tool:#?}");
    let wide = errors(&APP.replace("brush 4", "brush 500"));
    assert!(wide.iter().any(|e| e.contains("brush")), "{wide:#?}");
}

#[test]
fn the_canvas_words_are_words_of_the_language() {
    let words = clarity_spec::highlight::keywords();
    for word in ["mode", "objects", "paint", "vector", "tools", "brush", "ink"] {
        assert!(words.iter().any(|w| w == word), "{word}");
    }
}

/// A diagram's canvas (mode objects) has tools too: `select` drags its
/// objects, `connect` draws a new line from one to another, which runs
/// `on connect`. Code reads and sets `canvas.tool`, and a toolbox page
/// serves it with those tools, zoom and coded ones.
#[test]
fn a_canvas_of_objects_selects_connects_and_takes_a_toolbox() {
    let diagram = "module m

record Box
    id: integer
    x: integer
    y: integer

page Diagram: window
    data boxes: list<Box>
    canvas name sheet at 1,1 span 12 rows 10 units 4000 x 3000 zoom mode objects boxes at x, y tools select, connect toolbox DiagramTools on draw: drawBoxes on change: moved on connect: connected

code Diagram
    handler drawBoxes
        sheet.clear()

    handler moved
        let b = sheet.selected

    handler connected
        sheet.tool = \"select\"
        if sheet.tool == \"connect\"
            sheet.connecting = false

page DiagramTools: toolbox
    grid 2 columns
    tool name selectTool at 1,1 icon \"mouse-pointer\" does select
    tool name connectTool at 2,1 icon \"spline\" does connect
    tool name zoomInTool at 1,2 icon \"zoom-in\" does zoomIn
    tool name arrangeTool at 2,2 icon \"layout-grid\" on click: arrangeClicked

code DiagramTools
    handler arrangeClicked
        connectTool.visible = false
";
    let found = errors(diagram);
    assert!(found.is_empty(), "{found:#?}");
    // A painting tool is no diagram's.
    let pen = errors(&diagram.replace("does connect", "does pen"));
    assert!(pen.iter().any(|e| e.contains("'pen'") && e.contains("objects")), "{pen:#?}");
    // Connecting is a diagram's or a drawn canvas's: a vector drawing has its own lines.
    let vector = errors("module m\n\nentity Note\n    drawing: text\n\npage Main: window\n    data note: Note\n    canvas name sheet at 1,1 span 6 rows 8 bind note.drawing mode vector on change: saved on connect: connected\n\ncode Main\n    handler saved\n        save note\n\n    handler connected\n        save note\n");
    assert!(vector.iter().any(|e| e.contains("'on connect'")), "{vector:#?}");
}
