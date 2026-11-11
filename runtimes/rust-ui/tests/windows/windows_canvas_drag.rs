//! A drag and a bend on a canvas with undo, through the window's host as
//! the desktop runs a program (examples/modelling.clarity's diagrams): the
//! drag handler picks the box it started on and moves it, snapped to the
//! grid, the drop puts it there; a bend sends the link's points. A drag
//! whose first run changed what the window holds moves the box, not the
//! view, even when the snapped box stays where it was that time. On a
//! canvas with `on connect` a press on a box's handle draws a new line to
//! another box: the connect handler finds the two boxes, as the modelling
//! example's does, and keeps a link between them. On a canvas with `on
//! resize` a press on the grip at a box's bottom-right corner sizes it: the
//! resize handler finds the box by where that corner was and keeps its new
//! size.

use std::sync::{Arc, Mutex};

use clarity_runtime::program::{self, Answer, Program, Started};
use clarity_runtime::ui::{RefOption, Reply, WindowState};
use clarity_ui::input::{Modifiers, PointerButton};
use clarity_ui::parts::data::canvas::{CanvasPart, Viewport};
use clarity_ui::window::WindowStack;
use clarity_ui::{AppFiles, Element, Screen};
use serde_json::{json, Map, Value};

const BOARD: &str = r#"<Window name="Main">
  <Canvas name="board" at="1,1" span="12" undoScope="board" units="{&quot;width&quot;:4000,&quot;height&quot;:3000}" zoom="true" grid="20" snap="true" ondrag="moved" ondrop="dropped" onbend="bent" />
</Window>
"#;

/// The board with `on connect`: a line drawn from box to box links them.
const LINKED_BOARD: &str = r#"<Window name="Main">
  <Canvas name="board" at="1,1" span="12" undoScope="board" units="{&quot;width&quot;:4000,&quot;height&quot;:3000}" zoom="true" grid="20" snap="true" ondrag="moved" ondrop="dropped" onbend="bent" onconnect="linked" />
</Window>
"#;

/// The board as the modelling example's diagrams are: `on drop` and no `on
/// drag`, so the canvas carries a box itself; `on connect`; and a popup menu.
const CARRY_BOARD: &str = r#"<Window name="Main">
  <Canvas name="board" at="1,1" span="12" undoScope="board" units="{&quot;width&quot;:4000,&quot;height&quot;:3000}" zoom="true" grid="20" snap="true" ondrop="carried" onbend="bent" onconnect="linked">
    <Menu name="boardMenu">
      <Item name="removeItem" caption="Remove" onclick="removed" />
    </Menu>
  </Canvas>
</Window>
"#;

/// The carrying board with `on resize`: a box's grip sizes it.
const SIZED_BOARD: &str = r#"<Window name="Main">
  <Canvas name="board" at="1,1" span="12" undoScope="board" units="{&quot;width&quot;:4000,&quot;height&quot;:3000}" zoom="true" grid="20" snap="true" ondrop="carried" onbend="bent" onconnect="linked" onresize="sized" />
</Window>
"#;

/// The board in a slide-out, as the modelling example's diagram slides up from the bottom.
const SLID_BOARD: &str = r#"<Window name="Main">
  <Slideout name="details" caption="Diagram" side="bottom" slideHeight="400" span="12" rows="8">
    <Radio name="kindBox" at="1,1" span="3" />
    <Canvas name="board" at="1,2" span="12" rows="7" undoScope="board" units="{&quot;width&quot;:4000,&quot;height&quot;:3000}" zoom="true" grid="20" snap="true" ondrag="moved" ondrop="dropped" onbend="bent" />
  </Slideout>
</Window>
"#;

const MODEL: &str = r#"{"project": {"locale": {"language": "en"}, "files": [{"pages": [{"name": {"name": "Main"}, "kind": "window", "title": "Board"}]}]}, "keys": []}"#;

const FILES: AppFiles = AppFiles { windows: &[("Main", BOARD)], apps: &[], themes: &[], model: MODEL, resources: &[], title: "Board", width: 800.0, height: 500.0 };

const LINKED_FILES: AppFiles = AppFiles { windows: &[("Main", LINKED_BOARD)], ..FILES };

const CARRY_FILES: AppFiles = AppFiles { windows: &[("Main", CARRY_BOARD)], ..FILES };

const SIZED_FILES: AppFiles = AppFiles { windows: &[("Main", SIZED_BOARD)], ..FILES };

const SLID_FILES: AppFiles = AppFiles { windows: &[("Main", SLID_BOARD)], ..FILES };

/// The box: where it is and how big.
const BOX: (i64, i64, i64, i64) = (1000, 1000, 400, 200);

/// The second box, beside the first, that a connect test draws too.
const OTHER: (i64, i64) = (2400, 1000);

/// A program whose window holds one box and one link from it, its handlers
/// as the modelling example writes them, each edit logged.
#[derive(Default)]
struct Board {
    log: Mutex<Vec<String>>,
    /// Whether its drag handler does nothing at all.
    idle: bool,
    /// Whether it draws the second box.
    two: bool,
    /// Whether its window starts with `canvas.connecting` set, as a line tool sets it.
    line_tool: bool,
    /// Whether its canvas starts read-only.
    readonly: bool,
    /// Whether its window starts with `canvas.resizable` set false.
    fixed: bool,
    /// Whether its window starts with the slide-out `details` out.
    slid: bool,
}

fn reply(state: WindowState) -> Reply {
    Reply { state, messages: Vec::new(), shown: Default::default(), open: Vec::new(), close: false, asks: Vec::new(), result: None, hears: Vec::new(), copy: None, step: 0, refused: false, quit: false, keys: false }
}

fn int(data: &Map<String, Value>, name: &str) -> i64 {
    data.get(name).and_then(Value::as_i64).unwrap_or(0)
}

/// The first box's width and height: as the resize handler kept them, else BOX's.
fn size(data: &Map<String, Value>) -> (i64, i64) {
    (data.get("w").and_then(Value::as_i64).unwrap_or(BOX.2), data.get("h").and_then(Value::as_i64).unwrap_or(BOX.3))
}

fn snapped(value: i64, step: i64) -> i64 {
    (2 * value + step).div_euclid(2 * step) * step
}

/// The drawing of what the window holds: the link first, the box over its end.
fn draw(state: &mut WindowState) {
    let d = &state.data;
    let moving = int(d, "movingId") != 0;
    let (x, y) = if moving { (int(d, "movedX"), int(d, "movedY")) } else { (int(d, "x"), int(d, "y")) };
    let via = d.get("via").cloned().unwrap_or(json!([]));
    let (w, h) = size(d);
    let mut drawing = vec![
        json!({"op": "link", "name": "l1", "x1": x + w / 2, "y1": y + h / 2, "x2": 3000, "y2": 2500, "via": via, "color": "border", "width": 2}),
        json!({"op": "rect", "x": x, "y": y, "width": w, "height": h, "color": "raised"}),
    ];
    if d.contains_key("x2") {
        drawing.push(json!({"op": "rect", "x": int(d, "x2"), "y": int(d, "y2"), "width": BOX.2, "height": BOX.3, "color": "raised"}));
    }
    state.parts.entry("board".into()).or_default().drawing = Some(drawing);
}

impl Program for Board {
    fn started(&self) -> Started {
        Started { main: "Main".into(), unconnected: None, signin: None }
    }
    fn start_app(&self) -> Answer<Reply> {
        Ok(reply(WindowState::new("", Map::new())))
    }
    fn open_window(&self, window: &str, _with: Map<String, Value>) -> Answer<Reply> {
        let mut data = Map::new();
        data.insert("x".into(), json!(BOX.0));
        data.insert("y".into(), json!(BOX.1));
        if self.two {
            data.insert("x2".into(), json!(OTHER.0));
            data.insert("y2".into(), json!(OTHER.1));
        }
        let mut state = WindowState::new(window, data);
        if self.line_tool {
            state.parts.entry("board".into()).or_default().connecting = Some(true);
        }
        if self.readonly {
            state.parts.entry("board".into()).or_default().readonly = Some(true);
        }
        if self.fixed {
            state.parts.entry("board".into()).or_default().resizable = Some(false);
        }
        if self.slid {
            state.parts.entry("details".into()).or_default().visible = Some(true);
        }
        draw(&mut state);
        Ok(reply(state))
    }
    fn close_window(&self, state: WindowState) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn return_to(&self, state: WindowState) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn dispatch_event(&self, state: WindowState, handler: &str) -> Answer<Reply> {
        self.log.lock().unwrap().push(format!("run {handler}"));
        Ok(reply(state))
    }
    fn edit_event(&self, mut state: WindowState, edit: program::Edit) -> Answer<Reply> {
        let handler = edit.handler.unwrap_or_default();
        self.log.lock().unwrap().push(format!("{} {handler}", edit.scope));
        let at = state.parts.get("board").and_then(|p| p.pointer.clone()).unwrap_or_default();
        let d = &mut state.data;
        match handler.as_str() {
            "moved" if !self.idle => {
                let (fx, fy) = (int(&at, "fromX"), int(&at, "fromY"));
                if int(d, "movingId") == 0 && fx >= int(d, "x") && fx <= int(d, "x") + BOX.2 && fy >= int(d, "y") && fy <= int(d, "y") + BOX.3 {
                    d.insert("movingId".into(), json!(1));
                    d.insert("grabX".into(), json!(fx - int(d, "x")));
                    d.insert("grabY".into(), json!(fy - int(d, "y")));
                }
                if int(d, "movingId") != 0 {
                    let moved_x = snapped(int(&at, "x") - int(d, "grabX"), 20);
                    let moved_y = snapped(int(&at, "y") - int(d, "grabY"), 20);
                    d.insert("movedX".into(), json!(moved_x));
                    d.insert("movedY".into(), json!(moved_y));
                }
            }
            "dropped" => {
                if int(d, "movingId") != 0 {
                    let (x, y) = (int(d, "movedX"), int(d, "movedY"));
                    d.insert("x".into(), json!(x));
                    d.insert("y".into(), json!(y));
                }
                d.insert("movingId".into(), json!(0));
            }
            "carried" => {
                // The box it was carried from moves by as much as it was carried.
                let (fx, fy) = (int(&at, "fromX"), int(&at, "fromY"));
                if fx >= int(d, "x") && fx <= int(d, "x") + BOX.2 && fy >= int(d, "y") && fy <= int(d, "y") + BOX.3 {
                    let (x, y) = (int(d, "x") + int(&at, "x") - fx, int(d, "y") + int(&at, "y") - fy);
                    d.insert("x".into(), json!(x));
                    d.insert("y".into(), json!(y));
                }
            }
            "linked" => {
                // The boxes the line started and ended on, found as the modelling example finds them.
                let inside = |px: i64, py: i64, bx: i64, by: i64| px >= bx && px <= bx + BOX.2 && py >= by && py <= by + BOX.3;
                let which = |px: i64, py: i64| {
                    if inside(px, py, int(d, "x"), int(d, "y")) {
                        1
                    } else if d.contains_key("x2") && inside(px, py, int(d, "x2"), int(d, "y2")) {
                        2
                    } else {
                        0
                    }
                };
                let (from, to) = (which(int(&at, "fromX"), int(&at, "fromY")), which(int(&at, "x"), int(&at, "y")));
                d.insert("linked".into(), json!([from, to]));
            }
            "sized" => {
                // The box whose bottom-right corner was at fromX, fromY: found just inside it, as the modelling example finds it.
                let (fx, fy) = (int(&at, "fromX") - 1, int(&at, "fromY") - 1);
                let (w, h) = size(d);
                if fx >= int(d, "x") && fx <= int(d, "x") + w && fy >= int(d, "y") && fy <= int(d, "y") + h {
                    let (w, h) = (int(&at, "x") - int(d, "x"), int(&at, "y") - int(d, "y"));
                    d.insert("w".into(), json!(w));
                    d.insert("h".into(), json!(h));
                    d.insert("sizedFrom".into(), json!([int(&at, "fromX"), int(&at, "fromY")]));
                }
            }
            "bent" if at.get("bent") == Some(&json!("l1")) => {
                d.insert("via".into(), at.get("via").cloned().unwrap_or(json!([])));
            }
            _ => {}
        }
        draw(&mut state);
        Ok(reply(state))
    }
    fn undo_edit(&self, state: WindowState, scope: &str, back: bool) -> Answer<Reply> {
        self.log.lock().unwrap().push(format!("{} {scope}", if back { "undo" } else { "redo" }));
        Ok(reply(state))
    }
    fn reorder_event(&self, state: WindowState, _reorder: program::Reorder) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn item_event(&self, state: WindowState, _item: program::ItemEdit) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn range_event(&self, state: WindowState, _range: program::RangeEdit) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn canvas_event(&self, state: WindowState, _canvas: program::CanvasEdit) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn step_event(&self, state: WindowState, _step: program::StepPress) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn view_event(&self, state: WindowState, _part: &str, _action: Map<String, Value>) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn answer_event(&self, state: WindowState, _handler: &str, _values: Vec<Value>) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn ref_options(&self, _entity: &str) -> Answer<Vec<RefOption>> {
        Ok(Vec::new())
    }
    fn field_values(&self, _entity: &str, _field: &str) -> Answer<Vec<String>> {
        Ok(Vec::new())
    }
    fn find_options(&self, _kind: &str, _entity: &str, _field: Option<&str>, _show: &[String], _typed: &str) -> Answer<Value> {
        Ok(Value::Null)
    }
    fn field_bytes(&self, _entity: &str, _id: i64, _field: &str) -> Answer<Option<Vec<u8>>> {
        Ok(None)
    }
    fn store_file(&self, _store: &str, _path: &str) -> Answer<Vec<u8>> {
        Ok(Vec::new())
    }
    fn read_settings(&self) -> Map<String, Value> {
        Map::new()
    }
    fn keep_settings(&self, _settings: Map<String, Value>) -> Answer<()> {
        Ok(())
    }
}

fn started(board: Board) -> (Screen, Arc<Board>) {
    started_with(board, FILES)
}

fn started_with(board: Board, files: AppFiles) -> (Screen, Arc<Board>) {
    let board = Arc::new(board);
    let program: Arc<dyn Program> = board.clone();
    let mut screen = Screen::new(Box::new(WindowStack::start(program, files)), files.look(false));
    screen.resize(800.0, 500.0, 1.0);
    clarity_ui::testing::Shot::of(&mut screen, 800, 500, 1.0);
    (screen, board)
}

fn canvas(screen: &Screen) -> &CanvasPart {
    screen.find::<CanvasPart>().expect("the window shows its canvas")
}

fn view(screen: &Screen) -> Viewport {
    canvas(screen).viewport()
}

/// Where on the screen a unit of the canvas shows.
fn on_screen(screen: &Screen, x: f64, y: f64) -> (f64, f64) {
    let b = canvas(screen).node().bounds;
    let v = view(screen);
    (b.x + (x - v.x) * v.scale, b.y + (y - v.y) * v.scale)
}

fn data(screen: &Screen) -> Map<String, Value> {
    screen.root().as_any().downcast_ref::<WindowStack>().unwrap().current().unwrap().state_now().data
}

fn drawn_box(screen: &Screen) -> (f64, f64) {
    let drawing = canvas(screen).drawing_sent();
    let rect = drawing.iter().find(|s| s["op"] == "rect").expect("the box is drawn");
    (rect["x"].as_f64().unwrap(), rect["y"].as_f64().unwrap())
}

/// Zooms in on the middle of the box, so a first move of a few pixels is less than half the grid.
fn zoomed_on_the_box(screen: &mut Screen) -> (f64, f64) {
    let (mx, my) = on_screen(screen, (BOX.0 + BOX.2 / 2) as f64, (BOX.1 + BOX.3 / 2) as f64);
    screen.pointer_moved(mx, my);
    for _ in 0..8 {
        screen.wheel(mx, my, 0.0, -40.0, Modifiers::CONTROL);
    }
    assert!(view(screen).scale > 0.6, "zoomed in: {:?}", view(screen));
    on_screen(screen, (BOX.0 + BOX.2 / 2) as f64, (BOX.1 + BOX.3 / 2) as f64)
}

#[test]
fn a_drag_on_a_box_moves_it_and_its_drop_keeps_it_even_when_the_first_move_snaps_back() {
    let (mut screen, board) = started(Board::default());
    let (x, y) = zoomed_on_the_box(&mut screen);
    let before = view(&screen);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    // A first move under half the grid: the snapped box stays where it was this time.
    screen.pointer_moved(x + 5.0, y);
    assert_eq!(json!(1), data(&screen)["movingId"], "the drag handler picked the box");
    screen.pointer_moved(x + 60.0, y + 30.0);
    let moved = drawn_box(&screen);
    assert!(moved.0 > BOX.0 as f64, "the box follows the pointer: {moved:?}");
    screen.pointer_released(x + 60.0, y + 30.0, PointerButton::Left);
    assert_eq!(before, view(&screen), "the view did not pan");
    assert_eq!(vec!["board moved", "board moved", "board dropped"], *board.log.lock().unwrap());
    let kept = data(&screen);
    assert_eq!((json!(moved.0 as i64), json!(moved.1 as i64)), (kept["x"].clone(), kept["y"].clone()));
    assert_eq!(moved, drawn_box(&screen));
}

#[test]
fn a_drag_on_empty_paper_pans_the_canvas_and_asks_no_handler() {
    let (mut screen, board) = started(Board::default());
    zoomed_on_the_box(&mut screen);
    // Left of the box and above its link: nothing is drawn there.
    let (x, y) = on_screen(&screen, (BOX.0 - 60) as f64, (BOX.1 - 60) as f64);
    let before = view(&screen);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x + 20.0, y);
    screen.pointer_moved(x + 40.0, y);
    screen.pointer_released(x + 40.0, y, PointerButton::Left);
    assert!(board.log.lock().unwrap().is_empty(), "no handler ran: {:?}", board.log.lock().unwrap());
    assert!((before.x - 40.0 / before.scale - view(&screen).x).abs() < 1e-6, "the view panned: {before:?} {:?}", view(&screen));
}

#[test]
fn a_drag_on_a_box_is_its_handlers_even_when_the_handler_does_nothing() {
    let (mut screen, board) = started(Board { idle: true, ..Board::default() });
    let (x, y) = zoomed_on_the_box(&mut screen);
    let before = view(&screen);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x + 20.0, y);
    screen.pointer_moved(x + 40.0, y);
    screen.pointer_released(x + 40.0, y, PointerButton::Left);
    assert_eq!(before, view(&screen), "the view stayed");
    assert!(board.log.lock().unwrap().iter().filter(|l| *l == "board moved").count() >= 2, "every move went to the handler: {:?}", board.log.lock().unwrap());
}

#[test]
fn a_bend_on_a_link_sends_its_points_and_the_link_is_drawn_through_them() {
    let (mut screen, board) = started(Board::default());
    let before = view(&screen);
    // Halfway along the link, beside the box.
    let (x, y) = on_screen(&screen, 2100.0, 1800.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x + 10.0, y);
    screen.pointer_moved(x + 30.0, y - 20.0);
    screen.pointer_released(x + 30.0, y - 20.0, PointerButton::Left);
    assert_eq!(vec!["board bent"], *board.log.lock().unwrap());
    assert_eq!(before, view(&screen), "the view did not pan");
    let via = data(&screen)["via"].as_array().cloned().expect("the program kept the link's points");
    assert_eq!(2, via.len());
    let (vx, vy) = (via[0].as_f64().unwrap(), via[1].as_f64().unwrap());
    assert!((vx - (2100.0 + 30.0 / before.scale)).abs() < 3.0 && (vy - (1800.0 - 20.0 / before.scale)).abs() < 3.0, "{via:?}");
    let drawing = canvas(&screen).drawing_sent();
    let link = drawing.iter().find(|s| s["op"] == "link").unwrap();
    assert_eq!(json!(via), link["via"]);
}

// -- New lines between boxes (`on connect`) ------------------------------------

fn linked_board(board: Board) -> (Screen, Arc<Board>) {
    started_with(Board { two: true, ..board }, LINKED_FILES)
}

/// Where on the screen the handle on the right side of the first box is.
fn right_handle(screen: &Screen) -> (f64, f64) {
    on_screen(screen, (BOX.0 + BOX.2) as f64, (BOX.1 + BOX.3 / 2) as f64)
}

/// Where on the screen the middle of the second box is.
fn other_middle(screen: &Screen) -> (f64, f64) {
    on_screen(screen, (OTHER.0 + BOX.2 / 2) as f64, (OTHER.1 + BOX.3 / 2) as f64)
}

/// The handles shown, in the canvas's units.
fn handles(screen: &Screen) -> Vec<(f64, f64)> {
    canvas(screen).connect_handles()
}

#[test]
fn a_canvas_with_on_connect_shows_handles_on_the_box_under_the_pointer() {
    let (mut screen, _) = linked_board(Board::default());
    assert!(handles(&screen).is_empty(), "no handles before the pointer is over a box");
    let (x, y) = on_screen(&screen, (BOX.0 + BOX.2 / 2) as f64, (BOX.1 + BOX.3 / 2) as f64);
    screen.pointer_moved(x, y);
    let mut shown = handles(&screen);
    shown.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let (bx, by, bw, bh) = (BOX.0 as f64, BOX.1 as f64, BOX.2 as f64, BOX.3 as f64);
    assert_eq!(vec![(bx, by + bh / 2.0), (bx + bw / 2.0, by), (bx + bw / 2.0, by + bh), (bx + bw, by + bh / 2.0)], shown);
    // The crosshair over a handle, as a line starts there.
    let (hx, hy) = right_handle(&screen);
    screen.pointer_moved(hx + 2.0, hy);
    assert_eq!(4, handles(&screen).len(), "a handle just outside the box keeps its handles");
    assert_eq!(clarity_ui::input::Cursor::Crosshair, screen.cursor());
    // On empty paper: none.
    let (ex, ey) = on_screen(&screen, (BOX.0 - 200) as f64, (BOX.1 - 200) as f64);
    screen.pointer_moved(ex, ey);
    assert!(handles(&screen).is_empty());
}

#[test]
fn a_line_drawn_from_a_handle_to_another_box_runs_connect_once_and_moves_nothing() {
    let (mut screen, board) = linked_board(Board::default());
    let before = view(&screen);
    let (hx, hy) = right_handle(&screen);
    screen.pointer_moved(hx, hy);
    screen.pointer_pressed(hx, hy, PointerButton::Left, 1);
    let (tx, ty) = other_middle(&screen);
    screen.pointer_moved(hx + 10.0, hy);
    screen.pointer_moved((hx + tx) / 2.0, ty);
    screen.pointer_moved(tx, ty);
    // While dragging: a line from the handle to the pointer, the box under it a target; no handler yet.
    let (from, to) = canvas(&screen).connect_line().expect("a line follows the pointer");
    assert!((from.0 - (BOX.0 + BOX.2) as f64).abs() < 2.0, "the line shows from the handle: {from:?}");
    assert!((to.0 - (OTHER.0 + BOX.2 / 2) as f64).abs() < 2.0, "{to:?}");
    let target = canvas(&screen).connect_target().expect("the box under the pointer is the target");
    assert_eq!((OTHER.0 as f64, OTHER.1 as f64), (target.x, target.y));
    assert!(board.log.lock().unwrap().is_empty(), "no handler while dragging: {:?}", board.log.lock().unwrap());
    screen.pointer_released(tx, ty, PointerButton::Left);
    assert_eq!(vec!["board linked"], *board.log.lock().unwrap());
    assert_eq!(json!([1, 2]), data(&screen)["linked"], "from the first box to the second");
    assert_eq!((BOX.0 as f64, BOX.1 as f64), drawn_box(&screen), "the box did not move");
    assert_eq!(before, view(&screen), "the view did not pan");
    assert!(canvas(&screen).connect_line().is_none(), "the line is gone once let go");
    assert!(canvas(&screen).drawing_sent().iter().all(|s| s["op"] != "line"), "the line was never the program's drawing");
}

#[test]
fn while_connecting_is_set_a_press_anywhere_on_a_box_draws_a_line() {
    let (mut screen, board) = linked_board(Board { line_tool: true, ..Board::default() });
    let (x, y) = on_screen(&screen, (BOX.0 + 100) as f64, (BOX.1 + 50) as f64);
    screen.pointer_moved(x, y);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    let (tx, ty) = other_middle(&screen);
    screen.pointer_moved(x + 10.0, y);
    screen.pointer_moved(tx, ty);
    screen.pointer_released(tx, ty, PointerButton::Left);
    assert_eq!(vec!["board linked"], *board.log.lock().unwrap(), "a connect, not a drag");
    assert_eq!(json!([1, 2]), data(&screen)["linked"]);
    assert_eq!((BOX.0 as f64, BOX.1 as f64), drawn_box(&screen));
}

#[test]
fn a_press_on_a_handle_let_go_without_moving_runs_nothing() {
    let (mut screen, board) = linked_board(Board::default());
    let (hx, hy) = right_handle(&screen);
    screen.pointer_moved(hx, hy);
    screen.pointer_pressed(hx, hy, PointerButton::Left, 1);
    screen.pointer_moved(hx + 1.0, hy);
    screen.pointer_released(hx + 1.0, hy, PointerButton::Left);
    assert!(board.log.lock().unwrap().is_empty(), "{:?}", board.log.lock().unwrap());
}

#[test]
fn a_press_inside_a_box_away_from_its_handles_still_drags_it() {
    let (mut screen, board) = linked_board(Board::default());
    let (x, y) = zoomed_on_the_box(&mut screen);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x + 5.0, y);
    screen.pointer_moved(x + 60.0, y + 30.0);
    screen.pointer_released(x + 60.0, y + 30.0, PointerButton::Left);
    assert_eq!(vec!["board moved", "board moved", "board dropped"], *board.log.lock().unwrap());
}

#[test]
fn a_read_only_canvas_shows_no_handles_and_connects_nothing() {
    let (mut screen, board) = linked_board(Board { readonly: true, ..Board::default() });
    let (hx, hy) = right_handle(&screen);
    screen.pointer_moved(hx, hy);
    assert!(handles(&screen).is_empty());
    screen.pointer_pressed(hx, hy, PointerButton::Left, 1);
    let (tx, ty) = other_middle(&screen);
    screen.pointer_moved(tx, ty);
    screen.pointer_released(tx, ty, PointerButton::Left);
    assert!(!board.log.lock().unwrap().contains(&"board linked".to_string()), "{:?}", board.log.lock().unwrap());
}

#[test]
fn a_canvas_without_on_connect_shows_no_handles_and_ignores_connecting() {
    let (mut screen, board) = started(Board { two: true, line_tool: true, ..Board::default() });
    let (hx, hy) = right_handle(&screen);
    screen.pointer_moved(hx - 3.0, hy);
    assert!(handles(&screen).is_empty());
    assert_ne!(clarity_ui::input::Cursor::Crosshair, screen.cursor());
    // A press on the box drags it as before, `connecting` or not.
    let (x, y) = on_screen(&screen, (BOX.0 + 100) as f64, (BOX.1 + 50) as f64);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x + 10.0, y);
    screen.pointer_moved(x + 40.0, y);
    screen.pointer_released(x + 40.0, y, PointerButton::Left);
    let log = board.log.lock().unwrap();
    assert!(log.first().is_some_and(|l| l == "board moved") && log.last().is_some_and(|l| l == "board dropped"), "{log:?}");
    assert!(canvas(&screen).connect_line().is_none());
}

// -- The canvas carries a box (`on drop` without `on drag`) ------------------------

fn carry_board() -> (Screen, Arc<Board>) {
    started_with(Board { two: true, ..Board::default() }, CARRY_FILES)
}

/// The first box as it shows now, and the start of the link from it.
fn shown_box(screen: &Screen) -> ((f64, f64), (f64, f64)) {
    let shown = canvas(screen).shown();
    let rect = shown.iter().find_map(|s| match s {
        clarity_ui::parts::data::canvas::Shape::Rect { x, y, .. } => Some((*x, *y)),
        _ => None,
    });
    let link = shown.iter().find_map(|s| match s {
        clarity_ui::parts::data::canvas::Shape::Link { x1, y1, .. } => Some((*x1, *y1)),
        _ => None,
    });
    (rect.unwrap(), link.unwrap())
}

#[test]
fn a_box_dragged_on_a_canvas_with_only_on_drop_follows_the_pointer_and_runs_drop_once() {
    let (mut screen, board) = carry_board();
    let (x, y) = on_screen(&screen, (BOX.0 + 100) as f64, (BOX.1 + 50) as f64);
    let scale = view(&screen).scale;
    let before = view(&screen);
    screen.pointer_moved(x, y);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x + 10.0, y);
    // 205 by 98 units: on the grid of 20 that is 200 by 100.
    screen.pointer_moved(x + 205.0 * scale, y + 98.0 * scale);
    let ((bx, by), (lx, ly)) = shown_box(&screen);
    assert_eq!(((BOX.0 + 200) as f64, (BOX.1 + 100) as f64), (bx, by), "the box follows the pointer by whole grid steps");
    assert_eq!(((BOX.0 + BOX.2 / 2 + 200) as f64, (BOX.1 + BOX.3 / 2 + 100) as f64), (lx, ly), "the end of the link in the box comes along");
    assert!(board.log.lock().unwrap().is_empty(), "no handler while carrying: {:?}", board.log.lock().unwrap());
    assert!(canvas(&screen).connect_line().is_none(), "a press inside a box carries it, it draws no line");
    screen.pointer_released(x + 205.0 * scale, y + 98.0 * scale, PointerButton::Left);
    assert_eq!(vec!["board carried"], *board.log.lock().unwrap());
    let kept = data(&screen);
    assert_eq!((json!(BOX.0 + 200), json!(BOX.1 + 100)), (kept["x"].clone(), kept["y"].clone()), "the drop moved it by as much as it was carried");
    assert_eq!(((BOX.0 + 200) as f64, (BOX.1 + 100) as f64), shown_box(&screen).0, "and it shows where it was put");
    assert_eq!(before, view(&screen), "the view did not pan");
}

#[test]
fn a_box_let_go_where_it_was_runs_no_drop_and_a_drag_on_paper_still_pans() {
    let (mut screen, board) = carry_board();
    let (x, y) = on_screen(&screen, (BOX.0 + 100) as f64, (BOX.1 + 50) as f64);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x + 1.0, y);
    screen.pointer_released(x + 1.0, y, PointerButton::Left);
    assert!(board.log.lock().unwrap().is_empty(), "{:?}", board.log.lock().unwrap());
    let (px, py) = on_screen(&screen, 200.0, 200.0);
    let before = view(&screen);
    screen.pointer_pressed(px, py, PointerButton::Left, 1);
    screen.pointer_moved(px + 20.0, py);
    screen.pointer_moved(px + 40.0, py);
    screen.pointer_released(px + 40.0, py, PointerButton::Left);
    assert!(board.log.lock().unwrap().is_empty(), "{:?}", board.log.lock().unwrap());
    assert_ne!(before, view(&screen), "the view panned");
}

#[test]
fn a_right_drag_from_a_box_to_another_draws_a_line_and_opens_no_menu() {
    let (mut screen, board) = carry_board();
    let (x, y) = on_screen(&screen, (BOX.0 + 100) as f64, (BOX.1 + 50) as f64);
    screen.pointer_moved(x, y);
    screen.pointer_pressed(x, y, PointerButton::Right, 1);
    let (tx, ty) = other_middle(&screen);
    screen.pointer_moved(x + 10.0, y);
    screen.pointer_moved(tx, ty);
    assert!(canvas(&screen).connect_line().is_some(), "a line follows the pointer");
    assert_eq!(Some((OTHER.0 as f64, OTHER.1 as f64)), canvas(&screen).connect_target().map(|t| (t.x, t.y)));
    screen.pointer_released(tx, ty, PointerButton::Right);
    assert_eq!(vec!["board linked"], *board.log.lock().unwrap());
    assert_eq!(json!([1, 2]), data(&screen)["linked"]);
    assert!(!screen.context_menus().is_open(), "a right drag opens no menu");
    assert_eq!((BOX.0 as f64, BOX.1 as f64), drawn_box(&screen), "the box did not move");
}

#[test]
fn a_right_click_on_a_box_opens_the_canvas_menu_with_the_pointer_where_it_was() {
    let (mut screen, board) = carry_board();
    let (x, y) = on_screen(&screen, (BOX.0 + 100) as f64, (BOX.1 + 50) as f64);
    screen.pointer_moved(x, y);
    screen.pointer_pressed(x, y, PointerButton::Right, 1);
    screen.pointer_released(x, y, PointerButton::Right);
    assert!(screen.context_menus().is_open(), "a right click opens the menu");
    assert!(board.log.lock().unwrap().is_empty(), "{:?}", board.log.lock().unwrap());
    let pointer = screen.root().as_any().downcast_ref::<WindowStack>().unwrap().current().unwrap().state_now().parts["board"].pointer.clone().expect("the pointer is kept for the menu's handler");
    assert_eq!((json!(BOX.0 + 100), json!(BOX.1 + 50)), (pointer["x"].clone(), pointer["y"].clone()));
    // On empty paper the menu opens too.
    let (px, py) = on_screen(&screen, 200.0, 200.0);
    // A press beside the menu closes it.
    screen.pointer_pressed(px, py, PointerButton::Left, 1);
    screen.pointer_released(px, py, PointerButton::Left);
    assert!(!screen.context_menus().is_open());
    screen.pointer_pressed(px, py, PointerButton::Right, 1);
    screen.pointer_released(px, py, PointerButton::Right);
    assert!(screen.context_menus().is_open());
}

#[test]
fn a_right_click_on_a_line_takes_it_in_hand_and_the_menu_reads_which() {
    let (mut screen, board) = carry_board();
    let pointer = |screen: &Screen| screen.root().as_any().downcast_ref::<WindowStack>().unwrap().current().unwrap().state_now().parts["board"].pointer.clone().unwrap();
    // Halfway along the link, beside the box, as the bend test presses it.
    let (x, y) = on_screen(&screen, 2100.0, 1800.0);
    screen.pointer_moved(x, y);
    screen.pointer_pressed(x, y, PointerButton::Right, 1);
    screen.pointer_released(x, y, PointerButton::Right);
    assert!(screen.context_menus().is_open());
    assert!(board.log.lock().unwrap().is_empty(), "{:?}", board.log.lock().unwrap());
    assert_eq!(json!("l1"), pointer(&screen)["bent"], "the menu's handler reads which line");
    // On a box, no line.
    let (px, py) = on_screen(&screen, 200.0, 200.0);
    screen.pointer_pressed(px, py, PointerButton::Left, 1);
    screen.pointer_released(px, py, PointerButton::Left);
    let (bx, by) = on_screen(&screen, (BOX.0 + 100) as f64, (BOX.1 + 50) as f64);
    screen.pointer_pressed(bx, by, PointerButton::Right, 1);
    screen.pointer_released(bx, by, PointerButton::Right);
    assert_eq!(json!(""), pointer(&screen)["bent"]);
}

// -- Boxes people resize (`on resize`) ----------------------------------------------

fn sized_board(board: Board) -> (Screen, Arc<Board>) {
    started_with(Board { two: true, ..board }, SIZED_FILES)
}

/// Where on the screen the first box's bottom-right corner, its grip, is.
fn grip(screen: &Screen) -> (f64, f64) {
    on_screen(screen, (BOX.0 + BOX.2) as f64, (BOX.1 + BOX.3) as f64)
}

#[test]
fn a_canvas_with_on_resize_shows_a_grip_on_the_box_under_the_pointer() {
    let (mut screen, _) = sized_board(Board::default());
    assert_eq!(None, canvas(&screen).resize_grip(), "no grip before the pointer is over a box");
    let (x, y) = on_screen(&screen, (BOX.0 + BOX.2 / 2) as f64, (BOX.1 + BOX.3 / 2) as f64);
    screen.pointer_moved(x, y);
    assert_eq!(Some(((BOX.0 + BOX.2) as f64, (BOX.1 + BOX.3) as f64)), canvas(&screen).resize_grip());
    // Just outside the corner the grip still shows, with the resize cursor over it.
    let (gx, gy) = grip(&screen);
    screen.pointer_moved(gx + 3.0, gy + 3.0);
    assert!(canvas(&screen).resize_grip().is_some());
    assert_eq!(clarity_ui::input::Cursor::ResizeCorner, screen.cursor());
    let (ex, ey) = on_screen(&screen, (BOX.0 - 200) as f64, (BOX.1 - 200) as f64);
    screen.pointer_moved(ex, ey);
    assert_eq!(None, canvas(&screen).resize_grip());
}

#[test]
fn a_drag_on_the_grip_shows_the_new_box_and_runs_resize_once_on_the_grid() {
    let (mut screen, board) = sized_board(Board::default());
    let scale = view(&screen).scale;
    let before = view(&screen);
    let (gx, gy) = grip(&screen);
    screen.pointer_moved(gx, gy);
    screen.pointer_pressed(gx, gy, PointerButton::Left, 1);
    screen.pointer_moved(gx + 10.0, gy);
    // 205 by 98 units: the new corner snaps to the grid of 20, at 1605 -> 1600 and 1298 -> 1300.
    screen.pointer_moved(gx + 205.0 * scale, gy + 98.0 * scale);
    let outline = canvas(&screen).resize_outline().expect("the new box is outlined");
    assert_eq!((BOX.0 as f64, BOX.1 as f64, 600.0, 300.0), (outline.x, outline.y, outline.width, outline.height));
    assert!(board.log.lock().unwrap().is_empty(), "no handler while resizing: {:?}", board.log.lock().unwrap());
    assert_eq!(None, canvas(&screen).resize_grip(), "no grip while resizing");
    screen.pointer_released(gx + 205.0 * scale, gy + 98.0 * scale, PointerButton::Left);
    assert_eq!(vec!["board sized"], *board.log.lock().unwrap(), "one edit");
    let kept = data(&screen);
    assert_eq!((600, 300), size(&kept));
    assert_eq!(json!([BOX.0 + BOX.2, BOX.1 + BOX.3]), kept["sizedFrom"], "fromX, fromY: where the corner was");
    assert_eq!((BOX.0 as f64, BOX.1 as f64), drawn_box(&screen), "the box's top-left stays");
    assert_eq!(None, canvas(&screen).resize_outline());
    assert_eq!(before, view(&screen), "the view did not pan");
}

#[test]
fn a_box_is_never_resized_smaller_than_the_least_size() {
    let (mut screen, board) = sized_board(Board::default());
    let scale = view(&screen).scale;
    let (gx, gy) = grip(&screen);
    screen.pointer_moved(gx, gy);
    screen.pointer_pressed(gx, gy, PointerButton::Left, 1);
    screen.pointer_moved(gx - 10.0, gy);
    screen.pointer_moved(gx - 1000.0 * scale, gy - 1000.0 * scale);
    screen.pointer_released(gx - 1000.0 * scale, gy - 1000.0 * scale, PointerButton::Left);
    assert_eq!(vec!["board sized"], *board.log.lock().unwrap());
    // 40 by 30 at least; on the grid of 20 the bottom edge goes to the next step, 40.
    assert_eq!((40, 40), size(&data(&screen)));
}

#[test]
fn a_grip_let_go_where_it_was_runs_nothing_and_a_press_inside_still_carries() {
    let (mut screen, board) = sized_board(Board::default());
    let (gx, gy) = grip(&screen);
    screen.pointer_moved(gx, gy);
    screen.pointer_pressed(gx, gy, PointerButton::Left, 1);
    screen.pointer_moved(gx + 1.0, gy);
    screen.pointer_released(gx + 1.0, gy, PointerButton::Left);
    assert!(board.log.lock().unwrap().is_empty(), "{:?}", board.log.lock().unwrap());
    let scale = view(&screen).scale;
    let (x, y) = on_screen(&screen, (BOX.0 + 100) as f64, (BOX.1 + 50) as f64);
    screen.pointer_moved(x, y);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x + 10.0, y);
    screen.pointer_moved(x + 200.0 * scale, y);
    screen.pointer_released(x + 200.0 * scale, y, PointerButton::Left);
    assert_eq!(vec!["board carried"], *board.log.lock().unwrap());
    assert_eq!((BOX.2, BOX.3), size(&data(&screen)), "carried, not resized");
}

#[test]
fn a_cancelled_resize_runs_nothing() {
    let (mut screen, board) = sized_board(Board::default());
    let scale = view(&screen).scale;
    let (gx, gy) = grip(&screen);
    screen.pointer_moved(gx, gy);
    screen.pointer_pressed(gx, gy, PointerButton::Left, 1);
    screen.pointer_moved(gx + 100.0 * scale, gy + 100.0 * scale);
    screen.pointer_cancelled();
    assert_eq!(None, canvas(&screen).resize_outline());
    assert!(board.log.lock().unwrap().is_empty(), "{:?}", board.log.lock().unwrap());
}

#[test]
fn a_canvas_whose_code_set_resizable_false_shows_no_grip_and_resizes_nothing() {
    let (mut screen, board) = sized_board(Board { fixed: true, ..Board::default() });
    let scale = view(&screen).scale;
    let (gx, gy) = grip(&screen);
    screen.pointer_moved(gx - 3.0, gy - 3.0);
    assert_eq!(None, canvas(&screen).resize_grip());
    assert_ne!(clarity_ui::input::Cursor::ResizeCorner, screen.cursor());
    screen.pointer_pressed(gx - 3.0, gy - 3.0, PointerButton::Left, 1);
    screen.pointer_moved(gx + 10.0, gy);
    screen.pointer_moved(gx + 200.0 * scale, gy + 100.0 * scale);
    screen.pointer_released(gx + 200.0 * scale, gy + 100.0 * scale, PointerButton::Left);
    assert!(!board.log.lock().unwrap().contains(&"board sized".to_string()), "{:?}", board.log.lock().unwrap());
    assert_eq!((BOX.2, BOX.3), size(&data(&screen)));
}

#[test]
fn a_canvas_without_on_resize_shows_no_grip() {
    let (mut screen, board) = carry_board();
    let (x, y) = on_screen(&screen, (BOX.0 + BOX.2 / 2) as f64, (BOX.1 + BOX.3 / 2) as f64);
    screen.pointer_moved(x, y);
    assert_eq!(None, canvas(&screen).resize_grip());
    let (gx, gy) = grip(&screen);
    screen.pointer_moved(gx - 2.0, gy - 2.0);
    assert_ne!(clarity_ui::input::Cursor::ResizeCorner, screen.cursor());
    assert!(board.log.lock().unwrap().is_empty());
}

#[test]
fn ctrl_z_takes_back_a_bend_on_a_canvas_in_a_slide_out() {
    let (mut screen, board) = started_with(Board { slid: true, ..Board::default() }, SLID_FILES);
    // All the way out.
    let later = std::time::Instant::now() + std::time::Duration::from_secs(2);
    screen.now = Box::new(move || later);
    clarity_ui::testing::Shot::of(&mut screen, 800, 500, 1.0);
    let (x, y) = on_screen(&screen, 2100.0, 1800.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x + 10.0, y);
    screen.pointer_moved(x + 30.0, y - 20.0);
    screen.pointer_released(x + 30.0, y - 20.0, PointerButton::Left);
    screen.key_pressed(clarity_ui::KeyInput::new(clarity_ui::Key::Z, Modifiers::CONTROL));
    assert_eq!(vec!["board bent", "undo board"], *board.log.lock().unwrap());
}
