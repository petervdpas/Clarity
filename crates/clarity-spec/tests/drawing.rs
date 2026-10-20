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
    canvas name dial at 1,1 span 6 rows 6 units 200 x 100 on draw: drawDial
    button \"Faster\" name faster at 7,1 span 2 on click: fasterClicked

code Main
    handler drawDial
        dial.clear(\"#101810\")
        dial.circle(50, 50, 40, \"accent\")
        dial.ring(50, 50, 40, \"border\", 2)
        dial.line(50, 50, 50 + 35 * cos(angle), 50 - 35 * sin(angle), \"text\", 3)
        dial.rect(0, 95, 200, 5, \"muted\")
        dial.text(50, 90, \"{speed} km/h\")
        dial.text(150, 50, \"fast\", \"accent\", 12)

    handler fasterClicked
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
    assert_eq!(widget_line(&dial.props), "canvas name dial at 1,1 span 6 rows 6 units 200 x 100 on draw: drawDial");
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
    assert_has(&errors(&elsewhere), "'dial' is drawn only by its draw handler, drawDial");
}

#[test]
fn a_canvas_that_draws_nothing_is_a_warning() {
    let quiet = DIAL.replace(" on draw: drawDial", "");
    let warnings: Vec<String> = problems(&quiet).into_iter().filter(|(s, _)| *s == Severity::Warning).map(|(_, m)| m).collect();
    assert_has(&warnings, "canvas at 1,1 draws nothing yet; wire 'on draw: handler'");
    assert_has(&errors(&DIAL.replace("units 200 x 100", "units 0 x 100")), "a canvas is 1 to 10000 units wide and high");
}

/// A canvas can be clicked: `on click: handler`, which reads where in its
/// own units as `canvas.x` and `canvas.y`.
#[test]
fn a_click_on_a_canvas_says_where() {
    let board = "module game

page Main: window
    data square: integer
    canvas name board at 1,1 span 6 rows 6 units 300 x 300 on draw: drawBoard on click: boardClicked

code Main
    handler drawBoard
        board.clear(\"panel\")

    handler boardClicked
        square = 1
        if board.x >= 100
            square = 2
        if board.y >= 100
            square = square + 3
";
    assert!(errors(board).is_empty(), "{:#?}", errors(board));
    assert_has(&errors(&board.replace("        square = 1\n", "        board.x = 5\n")), "board.x is read, not set");
}

/// A draw handler can hand part of the drawing to a handler it calls, such
/// as one mark of a board; that handler draws only when nothing else runs it.
#[test]
fn a_draw_handler_can_call_handlers_that_draw() {
    let board = "module game

page Main: window
    canvas name board at 1,1 span 6 rows 6 units 300 x 300 on draw: drawBoard
    button \"Again\" name again at 7,1 on click: againClicked

code Main
    handler drawBoard
        board.clear(\"panel\")
        drawSquare(0, 0)
        drawSquare(100, 0)

    handler drawSquare(left: integer, top: integer)
        board.rect(left + 5, top + 5, 90, 90, \"accent\")
        drawCross(left, top)

    handler drawCross(left: integer, top: integer)
        board.line(left, top, left + 100, top + 100, \"text\", 4)

    handler againClicked
        message \"again\"
";
    assert!(errors(board).is_empty(), "{:#?}", errors(board));
    // Run from elsewhere, or by an event, its shapes would land outside a drawing.
    let elsewhere = board.replace("        message \"again\"\n", "        drawSquare(0, 0)\n");
    assert_has(&errors(&elsewhere), "'board' is drawn only by its draw handler, drawBoard, and the handlers only it runs");
    let wired = board.replace("on click: againClicked", "on click: drawCross(0, 0)");
    assert_has(&errors(&wired), "'board' is drawn only by its draw handler, drawBoard, and the handlers only it runs");
}
