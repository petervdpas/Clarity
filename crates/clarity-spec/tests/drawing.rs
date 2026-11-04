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
    assert_has(&with("dial.wobble(1)"), "a canvas draws clear, line, rect, circle, ring, text, link, object, ellipse, path and image, not 'wobble'");
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

/// A canvas can be zoomed and scrolled (`zoom`), and dragged on: `on drag`
/// runs while the pointer moves with its button down, `on drop` when it is
/// let go, both reading where it is now (`x`, `y`) and where it was pressed
/// (`fromX`, `fromY`), in the canvas's units.
#[test]
fn a_canvas_can_zoom_and_be_dragged_on() {
    let board = "module diagram

page Main: window
    data boxX: integer, default 20
    data boxY: integer, default 20
    data moved: integer
    canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom on draw: drawSheet on drag: dragged on drop: dropped

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
    let (file, diags) = parse(board, "diagram.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let sheet = &file.pages[0].widgets[0];
    assert!(sheet.props.zoom);
    assert_eq!(sheet.props.handlers.iter().map(|h| h.event).collect::<Vec<_>>(), [Event::Draw, Event::Drag, Event::Drop]);
    assert_eq!(widget_line(&sheet.props), "canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom on draw: drawSheet on drag: dragged on drop: dropped");
    assert!(errors(board).is_empty(), "{:#?}", errors(board));
    assert_has(&errors(&board.replace("        moved = moved + 1\n", "        sheet.fromX = 5\n")), "sheet.fromX is read, not set");
    // Only a canvas zooms and is dragged on.
    let button = "module x\n\npage Main: window\n    button \"Go\" name go at 1,1 span 2 zoom\n";
    assert_has(&errors(button), "zoom");
}

/// A canvas can show a grid (`grid 20`, in its units) and say that code snaps
/// to it (`snap`). Code reads and sets both while the app runs, and rounds
/// to the grid with `snapped(value, step)`.
#[test]
fn a_canvas_can_show_a_grid_and_code_snaps_to_it() {
    let sheet = "module diagram

page Main: window
    data boxX: integer
    canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom grid 20 snap on draw: drawSheet on drop: dropped
    button \"Grid\" name gridButton at 7,1 span 2 on click: gridClicked

code Main
    handler drawSheet
        sheet.clear(\"panel\")

    handler dropped
        boxX = sheet.x
        if sheet.snap
            boxX = snapped(boxX, sheet.grid)

    handler gridClicked
        sheet.snap = not sheet.snap
        if sheet.grid == 0
            sheet.grid = 20
        else
            sheet.grid = 0
";
    let (file, diags) = parse(sheet, "diagram.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let canvas = &file.pages[0].widgets[0];
    assert_eq!((canvas.props.grid, canvas.props.snap), (Some(20), true));
    assert_eq!(widget_line(&canvas.props), "canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom grid 20 snap on draw: drawSheet on drop: dropped");
    assert!(errors(sheet).is_empty(), "{:#?}", errors(sheet));
    assert_has(&errors(&sheet.replace(" grid 20 snap", " snap")), "'snap' snaps to the grid; give the canvas one, as in grid 20");
    assert_has(&errors(&sheet.replace("grid 20", "grid 0")), "a canvas's grid is 1 to 10000 units");
    assert_has(&errors(&sheet.replace("snapped(boxX, sheet.grid)", "snapped(\"a\", 2)")), "'snapped' takes a whole number and the step it rounds to, as in snapped(x, 20)");
    assert_has(&errors(&sheet.replace("sheet.snap = not sheet.snap", "gridButton.snap = true")), "gridButton");
}

/// A canvas that zooms has buttons to zoom out, back to whole and in, in Clarity's own words.
#[test]
fn a_canvas_that_zooms_has_clarity_s_own_words() {
    let sheet = "module diagram

page Main: window
    canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom on draw: drawSheet

code Main
    handler drawSheet
        sheet.clear(\"panel\")
";
    let project = clarity_spec::keyed::load_keyed([("diagram.clarity", sheet)], &[], "en");
    let en = &project.translations["en"];
    assert_eq!((en.get("zoomOut").map(String::as_str), en.get("zoomWhole").map(String::as_str), en.get("zoomIn").map(String::as_str)), (Some("Zoom out"), Some("Show it whole"), Some("Zoom in")));
    let fixed = sheet.replace(" zoom on draw", " on draw");
    let project = clarity_spec::keyed::load_keyed([("diagram.clarity", fixed.as_str())], &[], "en");
    assert_eq!(project.translations["en"].get("zoomIn"), None, "only a project with a canvas that zooms has them");
}

/// A canvas has a background (`background panel`, a theme color by name or
/// written out), which `clear()` without a color fills it with. Code reads
/// and sets it while the app runs; an empty one is the theme's panel.
#[test]
fn a_canvas_has_a_background_that_clear_fills() {
    let sheet = "module diagram

page Main: window
    data chosen: text
    canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom background panel on draw: drawSheet
    button \"Paper\" name paperButton at 7,1 span 2 on click: paperClicked

code Main
    handler drawSheet
        sheet.clear()

    handler paperClicked
        if sheet.background == \"panel\"
            sheet.background = \"#f5f0e1\"
        else
            sheet.background = chosen
";
    let (file, diags) = parse(sheet, "diagram.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let canvas = &file.pages[0].widgets[0];
    assert_eq!(canvas.props.background.as_deref(), Some("panel"));
    assert_eq!(widget_line(&canvas.props), "canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom background panel on draw: drawSheet");
    let written = sheet.replace("background panel", "background \"#101810\"");
    let (file, _) = parse(&written, "diagram.clarity");
    assert_eq!(file.pages[0].widgets[0].props.background.as_deref(), Some("#101810"));
    assert_eq!(widget_line(&file.pages[0].widgets[0].props), "canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom background \"#101810\" on draw: drawSheet");
    assert!(errors(sheet).is_empty(), "{:#?}", errors(sheet));
}

/// A canvas's grid dots have a size in pixels and a color: `dots 3 muted`
/// (the color can be left out, or written out). Code reads and sets both.
#[test]
fn a_canvas_grid_has_dots_of_a_size_and_color() {
    let sheet = "module diagram

page Main: window
    data chosen: text
    canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom grid 20 dots 3 muted on draw: drawSheet
    button \"Dots\" name dotsButton at 7,1 span 2 on click: dotsClicked

code Main
    handler drawSheet
        sheet.clear()

    handler dotsClicked
        sheet.dotSize = sheet.dotSize + 1
        sheet.dotColor = chosen
";
    let (file, diags) = parse(sheet, "diagram.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let canvas = &file.pages[0].widgets[0];
    assert_eq!((canvas.props.dot_size, canvas.props.dot_color.as_deref()), (Some(3), Some("muted")));
    assert_eq!(widget_line(&canvas.props), "canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom grid 20 dots 3 muted on draw: drawSheet");
    let plain = sheet.replace("dots 3 muted", "dots 4");
    let (file, _) = parse(&plain, "diagram.clarity");
    assert_eq!((file.pages[0].widgets[0].props.dot_size, file.pages[0].widgets[0].props.dot_color.as_deref()), (Some(4), None));
    let written = sheet.replace("dots 3 muted", "dots 3 \"#888888\"");
    let (file, _) = parse(&written, "diagram.clarity");
    assert_eq!(widget_line(&file.pages[0].widgets[0].props), "canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom grid 20 dots 3 \"#888888\" on draw: drawSheet");
    assert!(errors(sheet).is_empty(), "{:#?}", errors(sheet));
    assert_has(&errors(&sheet.replace("dots 3 muted", "dots 0")), "a grid's dots are 1 to 20 pixels");
}

/// A canvas draws a line people bend: `link(name, x1, y1, x2, y2, via, color,
/// width)` goes from one point to another through its bend points (a list of
/// whole numbers, x, y, x, y, ...). Bent and let go, `on bend` runs, reading
/// which line (`canvas.bent`, its name) and its points now (`canvas.via`).
#[test]
fn a_canvas_draws_lines_people_bend() {
    let sheet = "module diagram

page Main: window
    data points: list<integer>
    data bent: text
    canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom on draw: drawSheet on bend: bentLine

code Main
    handler drawSheet
        sheet.clear()
        sheet.link(\"a-b\", 100, 100, 600, 400, points, \"border\", 2)
        sheet.link(\"b-c\", 600, 400, 900, 100, [], \"border\")

    handler bentLine
        bent = sheet.bent
        points = sheet.via
";
    let (file, diags) = parse(sheet, "diagram.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(file.pages[0].widgets[0].props.handlers[1].event, Event::Bend);
    assert!(errors(sheet).is_empty(), "{:#?}", errors(sheet));
    assert_has(&errors(&sheet.replace("points, \"border\", 2)", "\"x\", \"border\", 2)")), "via of 'link' is a list of whole numbers");
    assert_has(&errors(&sheet.replace("        points = sheet.via\n", "        sheet.via = points\n")), "sheet.via is read, not set");
}

/// A link can show what its ends are (a data model's crow's feet, an
/// arrow, a base type's triangle), be dashed and carry a label; a text can
/// be bold, italic and in the code font. Words written out are checked.
#[test]
fn a_link_has_ends_a_label_and_a_style_and_a_text_a_style() {
    let sheet = "module diagram

page Main: window
    data label: text
    canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom on draw: drawSheet

code Main
    handler drawSheet
        sheet.clear()
        sheet.link(\"r1\", 100, 100, 600, 400, [], \"border\", 1, \"many\", \"one\", label, \"dashed\")
        sheet.link(\"r2\", 600, 400, 900, 100, [], \"border\", 1, \"\", \"triangle\")
        sheet.text(100, 100, \"Order\", \"text\", 12, \"left\", \"bold mono\")
";
    assert!(errors(sheet).is_empty(), "{:#?}", errors(sheet));
    assert_has(&errors(&sheet.replace("\"many\", \"one\"", "\"lots\", \"one\"")), "start of 'link' is \"arrow\", \"triangle\", \"diamond\", \"filledDiamond\", \"one\", \"many\", \"zeroOrOne\", \"zeroOrMany\" or \"oneOrMany\" or left empty, not \"lots\"");
    assert_has(&errors(&sheet.replace("\"dashed\")", "\"wavy\")")), "style of 'link' is \"solid\", \"dashed\" or \"dotted\" or left empty, not \"wavy\"");
    assert_has(&errors(&sheet.replace("\"bold mono\"", "\"bold huge\"")), "style of 'text' is made of \"bold\", \"italic\" and \"mono\", not \"huge\"");
    assert_has(&errors(&sheet.replace("\"left\"", "\"middle\"")), "align of 'text' is \"center\", \"left\" or \"right\" or left empty, not \"middle\"");
}

/// A canvas lets people draw a new line between two boxes: with `on connect`
/// a press on a box's handle (or anywhere on a box while code set
/// `canvas.connecting`) drags a line to where it is let go, and the handler
/// runs with where it started (`fromX`, `fromY`) and ended (`x`, `y`).
#[test]
fn a_canvas_connects_boxes_people_draw_a_line_between() {
    let sheet = "module diagram

page Main: window
    data linked: integer
    data lineTool: bool
    canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom on draw: drawSheet on connect: connected
    button \"Line\" name lineButton at 7,1 span 2 on click: lineClicked

code Main
    handler drawSheet
        sheet.clear()
        sheet.rect(100, 100, 200, 100, \"raised\")

    handler connected
        if sheet.fromX < sheet.x and sheet.fromY < sheet.y
            linked = linked + 1
        sheet.connecting = false

    handler lineClicked
        sheet.connecting = not sheet.connecting
        lineTool = sheet.connecting
";
    let (file, diags) = parse(sheet, "diagram.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(file.pages[0].widgets[0].props.handlers[1].event, Event::Connect);
    assert_eq!(widget_line(&file.pages[0].widgets[0].props), "canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom on draw: drawSheet on connect: connected");
    assert!(errors(sheet).is_empty(), "{:#?}", errors(sheet));
    assert!(!errors(&sheet.replace("        sheet.connecting = false\n", "        sheet.connecting = 3\n")).is_empty());
    assert!(!errors(&sheet.replace("sheet.connecting = not sheet.connecting", "lineButton.connecting = true")).is_empty());
    // Only a canvas with `on connect` has `connecting`.
    assert!(!errors(&sheet.replace(" on connect: connected", "")).is_empty());
    // A canvas with a mode has no boxes its code draws to connect.
    assert_has(&errors(&sheet.replace(" zoom on draw", " zoom mode vector on draw")), "'on connect' draws lines between the boxes");
}

/// A canvas lets people resize its boxes: with `on resize` a grip shows at
/// the bottom-right corner of the box under the pointer, dragging it sizes
/// the box, and the handler runs with where that corner was (`fromX`,
/// `fromY`) and is now (`x`, `y`). `canvas.resizable` (true until code sets
/// it) hides the grip.
#[test]
fn a_canvas_lets_people_resize_the_boxes_its_code_draws() {
    let sheet = "module diagram

page Main: window
    data width: integer
    data manual: bool
    canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom on draw: drawSheet on resize: resized
    button \"Manual\" name manualButton at 7,1 span 2 on click: manualClicked

code Main
    handler drawSheet
        sheet.clear()
        sheet.rect(100, 100, 200, 100, \"raised\")

    handler resized
        if sheet.fromX > 100 and sheet.fromY > 100
            width = sheet.x - 100

    handler manualClicked
        sheet.resizable = not sheet.resizable
        manual = sheet.resizable
";
    let (file, diags) = parse(sheet, "diagram.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(file.pages[0].widgets[0].props.handlers[1].event, Event::Resize);
    assert_eq!(widget_line(&file.pages[0].widgets[0].props), "canvas name sheet at 1,1 span 6 rows 6 units 3000 x 2000 zoom on draw: drawSheet on resize: resized");
    assert!(errors(sheet).is_empty(), "{:#?}", errors(sheet));
    assert!(!errors(&sheet.replace("sheet.resizable = not sheet.resizable", "sheet.resizable = 3")).is_empty());
    assert!(!errors(&sheet.replace("sheet.resizable = not sheet.resizable", "manualButton.resizable = true")).is_empty());
    // Only a canvas with `on resize` has `resizable`.
    assert!(!errors(&sheet.replace(" on resize: resized", "")).is_empty());
    // A canvas with a mode or an engine has no boxes its code draws to resize.
    assert_has(&errors(&sheet.replace(" zoom on draw", " zoom mode vector on draw")), "'on resize' sizes the boxes");
}
