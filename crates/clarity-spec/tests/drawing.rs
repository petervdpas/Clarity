//! A canvas drawn by code: `on draw: handler` and the shapes a draw
//! handler draws (conformance/drawing.json).

use clarity_spec::code::StmtKind;
use clarity_spec::model::CanvasUnits;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{Event, widget_line};
use clarity_spec::{Severity, load_sources};

const DIAL: &str = "module dial

page Main: window
    data angle: decimal, default 30
    data speed: integer, default 80
    canvas name dial at 1,1 span 6 rows 6 units 200 x 100 on draw: draw_dial
    button \"Faster\" name faster at 7,1 span 2 on click: faster_clicked

code Main
    handler draw_dial
        dial.clear(\"#101810\")
        dial.circle(50, 50, 40, \"accent\")
        dial.ring(50, 50, 40, \"border\", 2)
        dial.line(50, 50, 50 + 35 * cos(angle), 50 - 35 * sin(angle), \"text\", 3)
        dial.rect(0, 95, 200, 5, \"muted\")
        dial.text(50, 90, \"{speed} km/h\")
        dial.text(150, 50, \"fast\", \"accent\", 12)

    handler faster_clicked
        speed = speed + 10
        angle = angle - 10
";

fn problems(source: &str) -> Vec<(Severity, String)> {
    load_sources([("dial.clarity", source)]).diagnostics.into_iter().map(|d| (d.severity, d.message)).collect()
}

fn errors(source: &str) -> Vec<String> {
    problems(source).into_iter().filter(|(s, _)| *s == Severity::Error).map(|(_, m)| m).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn a_canvas_has_units_and_a_draw_handler() {
    let (file, diags) = parse(DIAL, "dial.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let dial = &file.pages[0].widgets[0];
    assert_eq!(dial.props.units, Some(CanvasUnits { width: 200, height: 100 }));
    assert_eq!(dial.props.handlers[0].event, Event::Draw);
    assert_eq!(widget_line(&dial.props), "canvas name dial at 1,1 span 6 rows 6 units 200 x 100 on draw: draw_dial");
    assert!(problems(DIAL).is_empty(), "{:#?}", problems(DIAL));

    let handler = &file.code[0].handlers[0];
    let StmtKind::Draw { canvas, shape, args } = &handler.body[3].kind else { panic!("{:#?}", handler.body[3]) };
    assert_eq!((canvas.name.as_str(), shape.name.as_str(), args.len()), ("dial", "line", 6));
}

#[test]
fn a_draw_handler_draws_the_shapes_a_canvas_has() {
    let with = |line: &str| errors(&DIAL.replace("        dial.rect(0, 95, 200, 5, \"muted\")\n", &format!("        {line}\n")));
    assert_has(&with("dial.wobble(1)"), "a canvas draws clear, line, rect, circle, ring and text, not 'wobble'");
    assert_has(&with("dial.line(1, 2, 3)"), "'line' takes x1, y1, x2, y2, color and width (width can be left out)");
    assert_has(&with("dial.circle(\"a\", 1, 2, \"x\")"), "x of 'circle' is a number, not text");
    assert_has(&with("dial.clear(3)"), "color of 'clear' is text, not integer");
    assert_has(&with("faster.line(1, 2, 3, 4, \"text\")"), "'faster' is a button; only a canvas draws");
    assert_has(&with("let x = sin(\"a\")"), "'sin' takes degrees, a number, not text");
    // Only the canvas's own draw handler draws on it.
    let elsewhere = DIAL.replace("        speed = speed + 10\n", "        speed = speed + 10\n        dial.clear(\"#000\")\n");
    assert_has(&errors(&elsewhere), "'dial' is drawn only by its draw handler, draw_dial");
}

#[test]
fn a_canvas_that_draws_nothing_is_a_warning() {
    let quiet = DIAL.replace(" on draw: draw_dial", "");
    let warnings: Vec<String> = problems(&quiet).into_iter().filter(|(s, _)| *s == Severity::Warning).map(|(_, m)| m).collect();
    assert_has(&warnings, "canvas at 1,1 draws nothing yet; wire 'on draw: handler'");
    assert_has(&errors(&DIAL.replace("units 200 x 100", "units 0 x 100")), "a canvas is 1 to 10000 units wide and high");
}
