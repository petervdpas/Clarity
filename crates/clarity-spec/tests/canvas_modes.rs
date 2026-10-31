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

#[test]
fn paint_binds_a_picture_and_vector_a_text_kept_as_svg() {
    let unbound = errors(&APP.replace(" bind note.picture mode paint", " mode paint"));
    assert!(unbound.iter().any(|e| e.contains("paint") && e.contains("image")), "{unbound:#?}");
    let vector_on_picture = errors(&APP.replace("bind note.drawing mode vector", "bind note.picture mode vector"));
    assert!(vector_on_picture.iter().any(|e| e.contains("vector") && e.contains("text")), "{vector_on_picture:#?}");
    let bound_drawn = errors(&APP.replace(" mode paint tools pen, eraser brush 4 ink accent", ""));
    assert!(bound_drawn.iter().any(|e| e.contains("mode")), "{bound_drawn:#?}");
    let wrong_tool = errors(&APP.replace("tools pen, eraser", "tools pen, rect"));
    assert!(wrong_tool.iter().any(|e| e.contains("rect") && e.contains("paint")), "{wrong_tool:#?}");
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
