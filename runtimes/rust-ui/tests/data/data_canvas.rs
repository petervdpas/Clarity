//! A canvas drawn by code (conformance/drawing.json): its shapes in its own
//! units, fitted in proportion and centered; where a click, a drag, a drop and
//! a bend are in its units; the view of a canvas that zooms; its grid. With a
//! mode (conformance/canvas-modes.json) the person drags its objects, paints
//! or draws, and the canvas sends what was done to the window as one action.

use std::cell::RefCell;
use std::rc::Rc;

use clarity_runtime::canvas::{apply, CanvasArgs};
use clarity_runtime::picture::Picture;
use clarity_runtime::ui::{PartState, WindowState};
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::markup::HandlerCall;
use clarity_ui::parts::data::canvas::{self, CanvasPart, Shape, Viewport};
use clarity_ui::parts::{Part, PartHost};
use clarity_ui::testing::{Shot, TestHost};
use clarity_ui::{Element, Screen, ThemeLook};
use serde_json::{json, Map, Value};
use skia_safe::Color;

fn conformance(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance").join(name);
    serde_json::from_str(&std::fs::read_to_string(path).expect("the conformance file reads")).expect("it is JSON")
}

const SHOTS: &str = "/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots";

/// A window that runs a canvas's commands, as a generated program has them, and logs each.
#[derive(Default)]
pub struct CanvasHost {
    pub inner: TestHost,
    pub points: RefCell<Vec<(String, String, Map<String, Value>)>>,
    pub actions: RefCell<Vec<(String, Value)>>,
    /// What a pointer event does to the drawing, as a handler would draw anew.
    pub on_point: RefCell<Option<Box<dyn Fn(&CanvasHost, &str, &Map<String, Value>)>>>,
}

impl CanvasHost {
    /// Draws `shapes` on the canvas's state, as its draw handler does.
    pub fn draw(&self, part: &str, shapes: Vec<Value>) {
        self.inner.set_state(part, |s| s.drawing = Some(shapes));
    }

    fn events(&self) -> Vec<String> {
        self.points.borrow().iter().map(|p| p.1.clone()).collect()
    }
}

impl PartHost for CanvasHost {
    fn read(&self, path: &str) -> Option<Value> {
        self.inner.read(path)
    }
    fn write(&self, part: &str, path: &str, value: Value) {
        self.inner.write(part, path, value)
    }
    fn edited(&self, part: &str) {
        self.inner.edited(part)
    }
    fn state(&self, name: Option<&str>) -> Option<PartState> {
        self.inner.state(name)
    }
    fn set_state(&self, name: &str, state: PartState) {
        clarity_ui::PartHost::set_state(&self.inner, name, state)
    }
    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        self.inner.run(part, call)
    }
    fn text(&self, written: Option<&str>) -> String {
        self.inner.text(written)
    }
    fn changes(&self) -> u64 {
        self.inner.changes()
    }
    fn pointed(&self, part: &str, event: &str, at: &Map<String, Value>) -> Option<bool> {
        self.points.borrow_mut().push((part.to_string(), event.to_string(), at.clone()));
        self.inner.set_state(part, |s| s.pointer = Some(at.clone()));
        if let Some(on_point) = self.on_point.borrow().as_ref() {
            on_point(self, event, at);
        }
        Some(true)
    }
    fn selected(&self, part: &str) -> Option<usize> {
        self.inner.selected(part)
    }
    fn select(&self, part: &str, place: Option<usize>) {
        self.inner.select(part, place)
    }
    fn acted(&self, part: &str, action: &Map<String, Value>) -> Option<bool> {
        self.actions.borrow_mut().push((part.to_string(), Value::Object(action.clone())));
        Some(true)
    }
}

/// A part of `markup` as the whole of a screen `width` by `height`, so its pixels are the screen's.
pub fn alone(markup: &str, host: Rc<dyn PartHost>, width: f64, height: f64) -> Screen {
    let spec = clarity_ui::markup::read(&format!("<Window name=\"Test\">{markup}</Window>")).expect("the markup reads");
    let part: Box<dyn Element> = clarity_ui::parts::make(spec.parts[0].clone(), host);
    let mut screen = Screen::new(part, ThemeLook::clarity(false));
    screen.resize(width, height, 1.0);
    screen
}

const UNITS: &str = r#"units="{&quot;width&quot;:100,&quot;height&quot;:100}""#;

fn canvas_screen(attributes: &str) -> (Rc<CanvasHost>, Screen) {
    let host = Rc::new(CanvasHost::default());
    let screen = alone(&format!(r#"<Canvas name="pad" {attributes} />"#), host.clone(), 800.0, 400.0);
    (host, screen)
}

fn the_canvas(screen: &Screen) -> &CanvasPart {
    screen.root().as_any().downcast_ref::<CanvasPart>().expect("the screen is a canvas")
}

fn pixel(screen: &mut Screen, x: i32, y: i32) -> Color {
    Shot::of(screen, 800, 400, 1.0).pixel(x, y)
}

fn press(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
}

fn release(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_released(x, y, PointerButton::Left);
}

fn pair(v: &Value) -> (f64, f64) {
    (v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
}

fn view(v: &Value) -> Viewport {
    Viewport { scale: v[0].as_f64().unwrap(), x: v[1].as_f64().unwrap(), y: v[2].as_f64().unwrap() }
}

fn point(v: &Value) -> Option<(i64, i64)> {
    if v.is_null() {
        None
    } else {
        Some((v[0].as_i64().unwrap(), v[1].as_i64().unwrap()))
    }
}

fn near_view(want: Viewport, got: Viewport) {
    assert!((want.scale - got.scale).abs() < 1e-6 && (want.x - got.x).abs() < 1e-6 && (want.y - got.y).abs() < 1e-6, "{want:?} {got:?}");
}

fn rect(x: f64, y: f64, w: f64, h: f64, color: &str) -> Value {
    json!({"op": "rect", "x": x, "y": y, "width": w, "height": h, "color": color})
}

fn clear() -> Value {
    json!({"op": "clear"})
}

#[test]
fn a_click_is_where_in_the_units_as_on_every_renderer() {
    for c in conformance("drawing.json")["clicks"].as_array().unwrap() {
        let (uw, uh) = pair(&c["units"]);
        let (pw, ph) = pair(&c["part"]);
        let (x, y) = pair(&c["at"]);
        assert_eq!(point(&c["point"]), canvas::point_of(uw, uh, pw, ph, x, y), "{c}");
    }
}

#[test]
fn its_view_starts_whole_zooms_around_the_pointer_and_pans_as_on_every_renderer() {
    let views = &conformance("drawing.json")["views"];
    for c in views["whole"].as_array().unwrap() {
        let (uw, uh) = pair(&c["units"]);
        let (pw, ph) = pair(&c["part"]);
        near_view(view(&c["view"]), canvas::whole(uw, uh, pw, ph));
    }
    for c in views["zoomed"].as_array().unwrap() {
        let (uw, uh) = pair(&c["units"]);
        let (pw, ph) = pair(&c["part"]);
        let (ax, ay) = pair(&c["at"]);
        near_view(view(&c["result"]), canvas::zoomed(uw, uh, pw, ph, view(&c["view"]), c["factor"].as_f64().unwrap(), ax, ay));
    }
    for c in views["scrolled"].as_array().unwrap() {
        let (bx, by) = pair(&c["by"]);
        near_view(view(&c["result"]), canvas::scrolled(view(&c["view"]), bx, by));
    }
    for c in views["points"].as_array().unwrap() {
        let (uw, uh) = pair(&c["units"]);
        let (ax, ay) = pair(&c["at"]);
        assert_eq!(point(&c["point"]), canvas::point_in(uw, uh, view(&c["view"]), ax, ay), "{c}");
    }
}

#[test]
fn its_grid_is_thinned_to_dots_eight_pixels_apart_as_on_every_renderer() {
    for c in conformance("drawing.json")["grids"]["lines"].as_array().unwrap() {
        assert_eq!(c["every"].as_f64().unwrap(), canvas::grid_every(c["grid"].as_f64().unwrap(), c["scale"].as_f64().unwrap()));
    }
}

#[test]
fn every_shape_of_the_cases_reads_and_words_start_by_their_alignment() {
    for c in conformance("drawing.json")["shapes"].as_array().unwrap() {
        assert!(Shape::of(&c["sends"]).is_ok(), "{c}");
    }
    assert_eq!((0.0, 40.0, 20.0), (canvas::text_left("left", 40.0), canvas::text_left("right", 40.0), canvas::text_left("center", 40.0)));
}

#[test]
fn it_draws_its_shapes_in_its_units_fitted_and_centered_in_the_theme_s_colors() {
    // 100 x 100 units in 800 x 400 pixels: 4 pixels a unit, starting 200 pixels in.
    let (host, mut screen) = canvas_screen(UNITS);
    host.draw("pad", vec![clear(), rect(10.0, 10.0, 20.0, 20.0, "accent"), json!({"op": "circle", "x": 75, "y": 75, "r": 5, "color": "#ff0000"})]);
    let look = ThemeLook::clarity(false);
    assert_eq!(look.color("accent"), pixel(&mut screen, 200 + 80, 80));
    assert_eq!(Color::from_rgb(255, 0, 0), pixel(&mut screen, 200 + 300, 300));
    // A clear without a color fills the units with the theme's panel; beside them is the window.
    assert_eq!(look.color("panel"), pixel(&mut screen, 200 + 200, 200));
    assert_eq!(look.color("background"), pixel(&mut screen, 100, 200));
}

#[test]
fn a_clear_without_a_color_fills_it_with_its_background() {
    let (host, mut screen) = canvas_screen(UNITS);
    host.inner.set_state("pad", |s| s.background = Some("accent".into()));
    host.draw("pad", vec![clear()]);
    assert_eq!(ThemeLook::clarity(false).color("accent"), pixel(&mut screen, 400, 200));
}

#[test]
fn a_name_the_theme_has_not_is_its_text_color() {
    let look = ThemeLook::clarity(false);
    assert_eq!(look.color("text"), canvas::color_of(&look, "no_such_role"));
}

#[test]
fn a_click_runs_its_handler_with_where_it_was_in_its_units() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} onclick="paint""#));
    press(&mut screen, 204.0, 8.0);
    release(&mut screen, 204.0, 8.0);
    let points = host.points.borrow().clone();
    assert_eq!(1, points.len());
    assert_eq!(("pad", "click"), (points[0].0.as_str(), points[0].1.as_str()));
    assert_eq!(json!({"x": 1, "y": 2}), Value::Object(points[0].2.clone()));
    // Beside the drawing it runs nothing.
    press(&mut screen, 100.0, 8.0);
    release(&mut screen, 100.0, 8.0);
    assert_eq!(1, host.points.borrow().len());
}

#[test]
fn without_the_window_s_commands_a_click_sets_its_pointer_and_runs_its_handler() {
    let host = TestHost::new();
    let mut screen = alone(&format!(r#"<Canvas name="pad" {UNITS} onclick="paint" />"#), host.clone(), 800.0, 400.0);
    press(&mut screen, 204.0, 8.0);
    release(&mut screen, 204.0, 8.0);
    assert_eq!(Some(json!({"x": 1, "y": 2})), host.states.borrow()["pad"].pointer.clone().map(Value::Object));
    assert!(host.logged().contains(&"run pad click paint".to_string()), "{:?}", host.logged());
}

#[test]
fn a_drag_runs_its_handler_with_where_it_started_and_the_drop_after() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} ondrag="moved" ondrop="dropped""#));
    // A handler that draws anew: the drag moves something.
    *host.on_point.borrow_mut() = Some(Box::new(|host, _, at| {
        let (x, y) = (at["x"].as_f64().unwrap(), at["y"].as_f64().unwrap());
        host.draw("pad", vec![rect(x, y, 1.0, 1.0, "accent")]);
    }));
    press(&mut screen, 240.0, 40.0);
    screen.pointer_moved(260.0, 40.0);
    screen.pointer_moved(280.0, 80.0);
    release(&mut screen, 280.0, 80.0);
    assert_eq!(vec!["drag", "drag", "drop"], host.events());
    let points = host.points.borrow();
    assert_eq!(json!({"x": 20, "y": 20, "fromX": 10, "fromY": 10}), Value::Object(points[2].2.clone()));
    assert_eq!(json!({"x": 15, "y": 10, "fromX": 10, "fromY": 10}), Value::Object(points[0].2.clone()));
}

#[test]
fn a_drag_on_empty_paper_pans_a_canvas_that_zooms_and_asks_no_handler() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} zoom="true" ondrag="moved""#));
    let before = the_canvas(&screen).viewport();
    press(&mut screen, 300.0, 200.0);
    screen.pointer_moved(320.0, 200.0);
    screen.pointer_moved(340.0, 200.0);
    release(&mut screen, 340.0, 200.0);
    assert!(host.points.borrow().is_empty());
    assert!((before.x - 40.0 / before.scale - the_canvas(&screen).viewport().x).abs() < 1e-6);
}

#[test]
fn the_wheel_pans_the_buttons_zoom_and_a_redraw_keeps_the_view() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} zoom="true""#));
    let whole = the_canvas(&screen).viewport();
    screen.wheel(400.0, 200.0, 0.0, 50.0, Modifiers::NONE);
    assert!((whole.y + 50.0 / whole.scale - the_canvas(&screen).viewport().y).abs() < 1e-6);
    let (zoom_out, back, zoom_in) = the_canvas(&screen).zoom_buttons();
    press(&mut screen, zoom_in.x + 5.0, zoom_in.y + 5.0);
    release(&mut screen, zoom_in.x + 5.0, zoom_in.y + 5.0);
    assert!((whole.scale * 1.2 - the_canvas(&screen).viewport().scale).abs() < 1e-6);
    host.draw("pad", vec![clear()]);
    Shot::of(&mut screen, 800, 400, 1.0);
    assert!((whole.scale * 1.2 - the_canvas(&screen).viewport().scale).abs() < 1e-6);
    press(&mut screen, back.x + 5.0, back.y + 5.0);
    release(&mut screen, back.x + 5.0, back.y + 5.0);
    assert_eq!(whole, the_canvas(&screen).viewport());
    assert!(zoom_out.x < back.x);
}

#[test]
fn ctrl_and_the_wheel_zoom_at_the_pointer_and_its_buttons_say_what_they_do() {
    let (_, mut screen) = canvas_screen(&format!(r#"{UNITS} zoom="true""#));
    let whole = the_canvas(&screen).viewport();
    screen.pointer_moved(400.0, 200.0);
    screen.wheel(400.0, 200.0, 0.0, -40.0, Modifiers::CONTROL);
    let zoomed = the_canvas(&screen).viewport();
    assert!((whole.scale * 1.2 - zoomed.scale).abs() < 1e-6);
    // The point under the pointer stays where it was.
    assert!((whole.x + 400.0 / whole.scale - (zoomed.x + 400.0 / zoomed.scale)).abs() < 1e-6);
    screen.wheel(400.0, 200.0, 0.0, 40.0, Modifiers::CONTROL);
    assert!((whole.scale - the_canvas(&screen).viewport().scale).abs() < 1e-6);
    let canvas = the_canvas(&screen);
    let (_, back, zoom_in) = canvas.zoom_buttons();
    assert_eq!(Some(canvas::zoom_word("zoomIn")), canvas.tooltip_at(zoom_in.x + 5.0, zoom_in.y + 5.0));
    assert_eq!(Some(canvas::zoom_word("zoomWhole")), canvas.tooltip_at(back.x + 5.0, back.y + 5.0));
    assert_eq!(None, canvas.tooltip_at(100.0, 300.0));
}

#[test]
fn its_grid_shows_dots_in_the_theme_s_border_color() {
    let (host, mut screen) = canvas_screen(UNITS);
    host.inner.set_state("pad", |s| {
        s.grid = Some(10);
        s.dot_size = Some(4);
    });
    host.draw("pad", vec![clear()]);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    shot.save(format!("{SHOTS}/data-canvas-grid.png")).unwrap();
    // A dot every 10 units: at unit 10, 10 is pixel 240, 40.
    assert_eq!(ThemeLook::clarity(false).color("border"), shot.pixel(240, 40));
    assert_eq!(ThemeLook::clarity(false).color("panel"), shot.pixel(260, 60));
}

fn link() -> Value {
    json!({"op": "link", "name": "a-b", "x1": 10, "y1": 50, "x2": 90, "y2": 50, "via": [], "color": "text", "width": 1})
}

#[test]
fn a_link_bends_and_its_handler_hears_its_points() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} onbend="bent""#));
    host.draw("pad", vec![link()]);
    Shot::of(&mut screen, 800, 400, 1.0);
    // A press on the link at unit 50, 50 adds a point there; dragging moves it to 50, 30.
    press(&mut screen, 400.0, 200.0);
    screen.pointer_moved(400.0, 160.0);
    screen.pointer_moved(400.0, 120.0);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    shot.save(format!("{SHOTS}/data-canvas-bend.png")).unwrap();
    release(&mut screen, 400.0, 120.0);
    let points = host.points.borrow();
    assert_eq!(1, points.len());
    assert_eq!("bend", points[0].1);
    assert_eq!(json!("a-b"), points[0].2["bent"]);
    assert_eq!(json!([50, 30]), points[0].2["via"]);
}

#[test]
fn a_press_on_a_shape_drawn_over_a_link_is_no_bend() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} onbend="bent""#));
    host.draw("pad", vec![link(), rect(40.0, 40.0, 20.0, 20.0, "panel")]);
    press(&mut screen, 400.0, 200.0);
    screen.pointer_moved(400.0, 120.0);
    release(&mut screen, 400.0, 120.0);
    assert!(host.points.borrow().is_empty());
}

// -- Modes (conformance/canvas-modes.json) -----------------------------------

#[test]
fn an_object_is_dragged_by_its_box_and_its_drop_is_sent() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} mode="objects""#));
    host.draw("pad", vec![json!({"op": "object", "record": 7, "x": 10, "y": 10, "width": 20, "height": 20}), rect(10.0, 10.0, 20.0, 20.0, "accent")]);
    press(&mut screen, 260.0, 60.0);
    screen.pointer_moved(280.0, 60.0);
    screen.pointer_moved(300.0, 80.0);
    // While dragged its box and what is drawn after it move with the pointer.
    let shown = the_canvas(&screen).shown();
    assert!(matches!(shown[1], Shape::Rect { x, .. } if x == 20.0), "{shown:?}");
    release(&mut screen, 300.0, 80.0);
    let actions = host.actions.borrow();
    assert_eq!(1, actions.len());
    assert_eq!("pad", actions[0].0);
    assert_eq!(json!({"move": {"id": 7, "x": 20, "y": 15}}), actions[0].1);
}

/// A diagram (mode objects with `on connect`): with select in hand a press
/// drags an object; with connect in hand it draws a line from one object to
/// another, which runs `on connect` and moves nothing.
#[test]
fn a_diagram_s_connect_tool_draws_a_line_between_objects_and_select_drags() {
    let objects = || {
        vec![
            json!({"op": "object", "record": 1, "x": 10, "y": 40, "width": 20, "height": 20}),
            rect(10.0, 40.0, 20.0, 20.0, "raised"),
            json!({"op": "object", "record": 2, "x": 60, "y": 40, "width": 20, "height": 20}),
            rect(60.0, 40.0, 20.0, 20.0, "raised"),
        ]
    };
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} mode="objects" onconnect="linked""#));
    host.draw("pad", objects());
    assert_eq!(vec!["select", "connect"], the_canvas(&screen).tools_shown());
    assert_eq!("select", the_canvas(&screen).tool_in_hand());
    // Select: a press drags the object; no handles show over it.
    screen.pointer_moved(280.0, 200.0);
    assert!(the_canvas(&screen).connect_handles().is_empty(), "select shows no handles");
    press(&mut screen, 280.0, 200.0);
    screen.pointer_moved(300.0, 200.0);
    screen.pointer_moved(320.0, 200.0);
    release(&mut screen, 320.0, 200.0);
    assert_eq!(json!({"move": {"id": 1, "x": 20, "y": 40}}), host.actions.borrow()[0].1);
    assert!(host.points.borrow().is_empty());
    // Connect: the same press draws a line to the other object.
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} mode="objects" onconnect="linked""#));
    host.draw("pad", objects());
    host.inner.set_state("pad", |s| s.tool = Some("connect".into()));
    assert!(the_canvas(&screen).connecting());
    screen.pointer_moved(280.0, 200.0);
    press(&mut screen, 280.0, 200.0);
    screen.pointer_moved(380.0, 200.0);
    screen.pointer_moved(480.0, 200.0);
    release(&mut screen, 480.0, 200.0);
    assert!(host.actions.borrow().is_empty(), "nothing moved: {:?}", host.actions.borrow());
    let points = host.points.borrow();
    assert_eq!(vec!["connect"], points.iter().map(|p| p.1.clone()).collect::<Vec<_>>());
    assert_eq!(json!({"x": 70, "y": 50, "fromX": 20, "fromY": 50}), Value::Object(points[0].2.clone()));
}

/// A diagram without `on connect` keeps its press for dragging and shows no toolbar.
#[test]
fn a_diagram_without_on_connect_has_no_tools() {
    let (_, screen) = canvas_screen(&format!(r#"{UNITS} mode="objects""#));
    assert!(the_canvas(&screen).tools_shown().is_empty());
}

#[test]
fn a_vector_canvas_draws_moves_and_deletes_with_its_tools() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} mode="vector" bind="drawing""#));
    host.inner.set("drawing", json!(""));
    host.inner.set_state("pad", |s| {
        s.tool = Some("rect".into());
        s.ink = Some("accent".into());
    });
    press(&mut screen, 240.0, 40.0);
    screen.pointer_moved(280.0, 80.0);
    release(&mut screen, 280.0, 80.0);
    let added = host.actions.borrow()[0].1.clone();
    assert_eq!(json!({"add": {"op": "rect", "x": 10, "y": 10, "width": 10, "height": 10, "color": "accent"}}), added);

    // What the runtime kept comes back as the bound text; select and drag moves it.
    let mut state = WindowState::default();
    state.data.insert("drawing".into(), json!(""));
    let args = CanvasArgs { part: "pad".into(), mode: "vector".into(), path: "drawing".into(), units: Some([100, 100]), ..CanvasArgs::default() };
    assert!(apply(&mut state, &args, added.as_object().unwrap()).unwrap());
    host.inner.set("drawing", state.data["drawing"].clone());
    host.inner.set_state("pad", |s| s.tool = Some("select".into()));
    press(&mut screen, 260.0, 60.0);
    assert_eq!(Some(0), the_canvas(&screen).chosen());
    screen.pointer_moved(280.0, 60.0);
    release(&mut screen, 280.0, 60.0);
    assert_eq!(json!({"move": {"index": 1, "x": 15, "y": 10}}), host.actions.borrow().last().unwrap().1);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    shot.save(format!("{SHOTS}/data-canvas-vector.png")).unwrap();

    // Delete deletes the chosen one.
    screen.key_pressed(KeyInput::plain(Key::Delete));
    assert_eq!(json!({"delete": 1}), host.actions.borrow().last().unwrap().1);
    let words: Vec<String> = the_canvas(&screen).tools_shown().into_iter().map(canvas::tool_word).collect();
    assert_eq!(vec!["Select", "Line", "Rectangle", "Ellipse", "Path", "Text"], words);
}

#[test]
fn the_text_tool_takes_its_text_in_place_and_enter_adds_it() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} mode="vector" bind="drawing""#));
    host.inner.set("drawing", json!(""));
    host.inner.set_state("pad", |s| s.tool = Some("text".into()));
    press(&mut screen, 240.0, 40.0);
    release(&mut screen, 240.0, 40.0);
    assert_eq!(Some(""), the_canvas(&screen).typing().as_deref());
    screen.text_typed("Hi");
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(None, the_canvas(&screen).typing());
    let actions = host.actions.borrow();
    assert_eq!(1, actions.len());
    assert_eq!(json!({"add": {"op": "text", "x": 10, "y": 10, "text": "Hi", "color": "text", "size": 10}}), actions[0].1);
}

#[test]
fn a_stroke_is_sent_as_the_picture_after_it() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} mode="paint" bind="picture""#));
    press(&mut screen, 240.0, 40.0);
    screen.pointer_moved(280.0, 80.0);
    release(&mut screen, 280.0, 80.0);
    let actions = host.actions.borrow();
    assert_eq!(1, actions.len());
    let Ok(Picture::New(bytes)) = serde_json::from_value::<Picture>(actions[0].1["stroke"].clone()) else { panic!("a stroke is a picture: {}", actions[0].1) };
    let image = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&bytes)).expect("the stroke is a PNG");
    assert_eq!((100, 100), (image.width(), image.height()));
    // The pen painted the theme's text color along the line, and nothing far from it.
    let mut surface = skia_safe::surfaces::raster_n32_premul((100, 100)).unwrap();
    surface.canvas().clear(Color::TRANSPARENT);
    surface.canvas().draw_image(&image, (0, 0), None);
    let pixmap = surface.peek_pixels().unwrap();
    assert_eq!(ThemeLook::clarity(false).color("text"), clarity_ui::testing::pixel_at(&pixmap, 15, 15));
    assert_eq!(0, clarity_ui::testing::pixel_at(&pixmap, 80, 20).a());
}

#[test]
fn a_readonly_canvas_sends_nothing() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} mode="paint" bind="picture" readonly="true""#));
    press(&mut screen, 240.0, 40.0);
    screen.pointer_moved(280.0, 80.0);
    release(&mut screen, 280.0, 80.0);
    assert!(host.actions.borrow().is_empty());
}

#[test]
fn a_shape_of_no_number_draws_nothing_and_the_rest_draws() {
    let (host, mut screen) = canvas_screen(UNITS);
    host.draw("pad", vec![json!({"op": "rect", "x": "wide", "y": 0, "width": 10, "height": 10, "color": "accent"}), rect(10.0, 10.0, 20.0, 20.0, "accent")]);
    assert_eq!(ThemeLook::clarity(false).color("accent"), pixel(&mut screen, 280, 80));
    assert_eq!(None, the_canvas(&screen).failed());
}

#[test]
fn a_canvas_text_keeps_its_spaces_and_width_at_every_zoom() {
    let fonts = clarity_ui::fonts::Fonts::new(ThemeLook::clarity(true));
    let whole = canvas::words_font(&fonts, 11.0);
    let small = canvas::words_font(&fonts, 11.0 * 0.52);
    let text = "string? medewerkerBk and DateTimeOffset? begindatum";
    assert!((whole.measure_str(text, None).0 * 0.52 - small.measure_str(text, None).0).abs() < 0.5);
    assert!(small.measure_str(" ", None).0 > 1.0);
}

#[test]
fn a_canvas_whose_drags_and_bends_are_edits_takes_the_focus_so_its_undo_key_reaches_it() {
    let (_, mut screen) = canvas_screen(&format!(r#"{UNITS} undoScope="pad" ondrag="moved" ondrop="dropped" onbend="bent""#));
    assert!(the_canvas(&screen).focusable());
    press(&mut screen, 240.0, 40.0);
    release(&mut screen, 240.0, 40.0);
    assert_eq!(Some(screen.root().node().id()), screen.focused());
    // Without undo, or with nothing the person changes on it, it takes no focus.
    let (_, plain) = canvas_screen(&format!(r#"{UNITS} noUndo="true" ondrop="dropped""#));
    assert!(!the_canvas(&plain).focusable());
    let (_, drawn) = canvas_screen(&format!(r#"{UNITS} ondraw="drawn""#));
    assert!(!the_canvas(&drawn).focusable());
    assert!(the_canvas(&drawn).grows());
}

/// Shift and the wheel pan a canvas sideways (the screen turns the wheel), as Avalonia's ScrollViewer did.
#[test]
fn shift_and_the_wheel_pan_the_canvas_sideways() {
    let (_, mut screen) = canvas_screen(&format!(r#"{UNITS} zoom="true""#));
    let whole = the_canvas(&screen).viewport();
    screen.wheel(400.0, 200.0, 0.0, 50.0, clarity_ui::Modifiers::SHIFT);
    let now = the_canvas(&screen).viewport();
    assert!((whole.x + 50.0 / whole.scale - now.x).abs() < 1e-6, "{whole:?} {now:?}");
    assert!((whole.y - now.y).abs() < 1e-6);
}

#[test]
fn a_canvas_grows_as_every_stack_grows_it_and_so_do_tabs_that_hold_one() {
    let (_, screen) = canvas_screen(&format!(r#"{UNITS} ondraw="drawn""#));
    assert!(clarity_ui::Part::grows(the_canvas(&screen)));
    let host = Rc::new(CanvasHost::default());
    let mut screen = clarity_ui::testing::window_of(r#"<Tabs at="1,1" span="12" rows="3"><Tab title="One"><Canvas name="pad" at="1,1" span="12" rows="2" /></Tab></Tabs><Button at="1,4" span="2" />"#, host);
    Shot::of(&mut screen, 808, 800, 1.0);
    let tabs = screen.find::<clarity_ui::parts::containers::tabs::TabsPart>().expect("the window has its tabs");
    assert!(clarity_ui::Part::grows(tabs));
    assert!(tabs.node().bounds.height > 600.0, "{:?}", tabs.node().bounds);
}

#[test]
fn a_drawing_s_svg_is_read_into_its_shapes_as_every_runtime_reads_it() {
    use clarity_ui::parts::data::canvas::drawing::drawing_shapes;
    let all = conformance("canvas-modes.json");
    let units = |v: &Value| -> [i64; 2] { serde_json::from_value(v.clone()).unwrap() };
    for case in all["svg"].as_array().unwrap() {
        let (read_units, read) = drawing_shapes(case["text"].as_str().unwrap(), [1000, 1000]);
        assert_eq!(read_units, units(&case["units"]), "{}", case["name"]);
        assert_eq!(serde_json::json!(read), case["shapes"], "{}", case["name"]);
    }
    for case in all["read"].as_array().unwrap() {
        let (read_units, read) = drawing_shapes(case["text"].as_str().unwrap(), [1000, 1000]);
        if !case["units"].is_null() {
            assert_eq!(read_units, units(&case["units"]), "{}", case["name"]);
        }
        assert_eq!(serde_json::json!(read), case["shapes"], "{}", case["name"]);
    }
}

// -- New lines between boxes (`on connect`) ------------------------------------

/// Two boxes: units 10..30 and 60..80 across, 40..60 down.
fn two_boxes(host: &CanvasHost) {
    host.draw("pad", vec![clear(), rect(10.0, 40.0, 20.0, 20.0, "raised"), rect(60.0, 40.0, 20.0, 20.0, "raised")]);
}

#[test]
fn the_box_under_the_pointer_shows_its_handles_in_the_accent_over_the_drawing() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} onconnect="linked""#));
    two_boxes(&host);
    let accent = ThemeLook::clarity(false).color("accent");
    // The right handle of the first box: unit 30, 50 is pixel 320, 200.
    assert_ne!(accent, pixel(&mut screen, 320, 200), "no handles before the pointer is over a box");
    screen.pointer_moved(280.0, 200.0);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    shot.save(format!("{SHOTS}/data-canvas-connect-handles.png")).unwrap();
    assert_eq!(accent, shot.pixel(320, 200));
    assert_eq!(accent, shot.pixel(280, 160), "the top handle");
    assert_eq!(4, the_canvas(&screen).connect_handles().len());
    assert_eq!(3, the_canvas(&screen).drawing_sent().len(), "the handles are not in the drawing");
    screen.pointer_left();
    assert!(the_canvas(&screen).connect_handles().is_empty(), "the pointer left: no handles");
}

#[test]
fn a_line_drawn_from_a_handle_is_dashed_in_the_accent_and_outlines_its_target() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} onconnect="linked" ondrag="moved""#));
    two_boxes(&host);
    screen.pointer_moved(320.0, 200.0);
    press(&mut screen, 320.0, 200.0);
    screen.pointer_moved(400.0, 200.0);
    screen.pointer_moved(480.0, 200.0);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    shot.save(format!("{SHOTS}/data-canvas-connect-line.png")).unwrap();
    let accent = ThemeLook::clarity(false).color("accent");
    // The target box's outline, 2 pixels outside its left edge at pixel 440.
    assert_eq!(accent, shot.pixel(438, 220));
    let dashed: usize = (330..430).filter(|x| shot.pixel(*x, 200) == accent).count();
    assert!(dashed > 30 && dashed < 90, "a dashed line, accent and gaps: {dashed}");
    assert!(host.points.borrow().is_empty(), "no handler while dragging");
    release(&mut screen, 480.0, 200.0);
    let points = host.points.borrow();
    assert_eq!(vec!["connect"], points.iter().map(|p| p.1.clone()).collect::<Vec<_>>());
    // From a handle the line starts at its box's middle, which the handler's hit test finds in the box.
    assert_eq!(json!({"x": 70, "y": 50, "fromX": 20, "fromY": 50}), Value::Object(points[0].2.clone()));
}

#[test]
fn a_box_drawn_as_an_outline_a_body_and_a_header_shows_one_set_of_handles_around_it_all() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} onconnect="linked""#));
    // A full-paper rect is the paper, no box; the header is part of the box under it.
    host.draw("pad", vec![rect(0.0, 0.0, 100.0, 100.0, "panel"), rect(9.0, 39.0, 22.0, 22.0, "border"), rect(10.0, 40.0, 20.0, 20.0, "raised"), rect(10.0, 40.0, 20.0, 5.0, "accent")]);
    // Over the header, unit 20, 42.
    screen.pointer_moved(280.0, 168.0);
    let mut shown = the_canvas(&screen).connect_handles();
    shown.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(vec![(9.0, 50.0), (20.0, 39.0), (20.0, 61.0), (31.0, 50.0)], shown);
    // On the paper beside it: none.
    screen.pointer_moved(600.0, 300.0);
    assert!(the_canvas(&screen).connect_handles().is_empty());
}

#[test]
fn a_canvas_without_on_connect_shows_no_handles_and_drags_as_before() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} ondrag="moved" ondrop="dropped""#));
    two_boxes(&host);
    host.inner.set_state("pad", |s| s.connecting = Some(true));
    let before = Shot::of(&mut screen, 800, 400, 1.0).pixel(320, 200);
    screen.pointer_moved(280.0, 200.0);
    assert!(the_canvas(&screen).connect_handles().is_empty());
    assert!(!the_canvas(&screen).connecting());
    assert_eq!(before, pixel(&mut screen, 320, 200), "nothing drawn over the box");
    press(&mut screen, 318.0, 200.0);
    screen.pointer_moved(400.0, 200.0);
    release(&mut screen, 400.0, 200.0);
    assert_eq!(vec!["drag", "drop"], host.events());
}

// -- Boxes people resize (`on resize`) ----------------------------------------------

#[test]
fn the_box_under_the_pointer_shows_a_grip_in_the_accent_at_its_corner() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} onresize="sized""#));
    two_boxes(&host);
    let accent = ThemeLook::clarity(false).color("accent");
    // The first box's bottom-right corner: unit 30, 60 is pixel 320, 240.
    assert_ne!(accent, pixel(&mut screen, 320, 240), "no grip before the pointer is over a box");
    screen.pointer_moved(280.0, 200.0);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    shot.save(format!("{SHOTS}/data-canvas-resize-grip.png")).unwrap();
    assert_eq!(accent, shot.pixel(320, 240));
    assert_eq!(Some((30.0, 60.0)), the_canvas(&screen).resize_grip());
    assert_eq!(3, the_canvas(&screen).drawing_sent().len(), "the grip is not in the drawing");
    screen.pointer_left();
    assert_eq!(None, the_canvas(&screen).resize_grip(), "the pointer left: no grip");
}

#[test]
fn a_grip_dragged_outlines_the_new_box_dashed_and_resize_reads_its_corners() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} onresize="sized" ondrag="moved""#));
    two_boxes(&host);
    screen.pointer_moved(320.0, 240.0);
    press(&mut screen, 320.0, 240.0);
    screen.pointer_moved(360.0, 280.0);
    screen.pointer_moved(400.0, 320.0);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    shot.save(format!("{SHOTS}/data-canvas-resize-outline.png")).unwrap();
    let accent = ThemeLook::clarity(false).color("accent");
    // The new right edge, unit 50, is pixel 400, from 160 down to 320.
    let dashed: usize = (170..310).filter(|y| shot.pixel(400, *y) == accent).count();
    assert!(dashed > 40 && dashed < 120, "a dashed outline, accent and gaps: {dashed}");
    assert!(host.points.borrow().is_empty(), "no handler while resizing");
    release(&mut screen, 400.0, 320.0);
    let points = host.points.borrow();
    assert_eq!(vec!["resize"], points.iter().map(|p| p.1.clone()).collect::<Vec<_>>(), "a resize, not a drag");
    assert_eq!(json!({"x": 50, "y": 80, "fromX": 30, "fromY": 60}), Value::Object(points[0].2.clone()));
}

#[test]
fn a_canvas_without_on_resize_draws_no_grip() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} ondrag="moved" ondrop="dropped""#));
    two_boxes(&host);
    let before = Shot::of(&mut screen, 800, 400, 1.0).pixel(320, 240);
    screen.pointer_moved(280.0, 200.0);
    assert_eq!(None, the_canvas(&screen).resize_grip());
    assert_eq!(before, pixel(&mut screen, 320, 240), "nothing drawn at the corner");
}

#[test]
fn a_paint_canvas_with_a_box_under_the_pointer_shows_no_handles() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} mode="paint" bind="picture" onconnect="linked""#));
    two_boxes(&host);
    screen.pointer_moved(280.0, 200.0);
    assert!(the_canvas(&screen).connect_handles().is_empty());
    press(&mut screen, 320.0, 200.0);
    screen.pointer_moved(480.0, 200.0);
    release(&mut screen, 480.0, 200.0);
    assert!(host.points.borrow().is_empty());
    assert_eq!(1, host.actions.borrow().len(), "the stroke is painted as before");
}

// -- What a link's ends show, its label and style, a text's style -------------------

#[test]
fn a_link_and_a_text_read_their_ends_label_and_style() {
    let link = Shape::of(&json!({"op": "link", "name": "r1", "x1": 1, "y1": 2, "x2": 3, "y2": 4, "via": [], "color": "border", "start": "many", "end": "one", "label": "orders", "style": "dashed"})).unwrap();
    let Shape::Link { start, end, label, style, .. } = link else { panic!("{link:?}") };
    assert_eq!(("many", "one", "orders", "dashed"), (start.as_str(), end.as_str(), label.as_str(), style.as_str()));
    // Left out, they are empty, as a link drawn before them.
    let Shape::Link { start, end, label, style, .. } = Shape::of(&json!({"op": "link", "name": "r1", "x1": 1, "y1": 2, "x2": 3, "y2": 4, "via": [], "color": "border"})).unwrap() else { panic!() };
    assert!(start.is_empty() && end.is_empty() && label.is_empty() && style.is_empty());
    let Shape::Words { style, .. } = Shape::of(&json!({"op": "text", "x": 1, "y": 2, "text": "Id", "style": "bold mono"})).unwrap() else { panic!() };
    assert_eq!("bold mono", style);
}

#[test]
fn a_link_ends_at_the_edge_of_the_box_it_starts_and_ends_in() {
    let boxes = [canvas::UnitBox { x: 10.0, y: 40.0, width: 20.0, height: 20.0 }, canvas::UnitBox { x: 70.0, y: 40.0, width: 20.0, height: 20.0 }];
    let link = Shape::of(&json!({"op": "link", "name": "r1", "x1": 20, "y1": 50, "x2": 80, "y2": 50, "via": [], "color": "border"})).unwrap();
    let Shape::Link { x1, y1, x2, y2, .. } = canvas::links_to_edges(vec![link], &boxes).remove(0) else { panic!() };
    assert_eq!((30.0, 50.0, 70.0, 50.0), (x1, y1, x2, y2));
    // An end on empty paper stays where it is.
    let free = Shape::of(&json!({"op": "link", "name": "r2", "x1": 20, "y1": 50, "x2": 50, "y2": 90, "via": [], "color": "border"})).unwrap();
    let Shape::Link { x2, y2, .. } = canvas::links_to_edges(vec![free], &boxes).remove(0) else { panic!() };
    assert_eq!((50.0, 90.0), (x2, y2));
}

#[test]
fn a_many_end_draws_a_crow_s_foot_at_the_box_edge_and_no_line_over_the_box() {
    // 100 x 100 units in 800 x 400 pixels: 4 pixels a unit, starting 200 pixels in.
    let (host, mut screen) = canvas_screen(UNITS);
    let drawing = |end: &str| {
        vec![
            clear(),
            rect(10.0, 40.0, 20.0, 20.0, "raised"),
            rect(70.0, 40.0, 20.0, 20.0, "raised"),
            json!({"op": "link", "name": "r1", "x1": 20, "y1": 50, "x2": 80, "y2": 50, "via": [], "color": "#ff0000", "width": 1, "end": end}),
        ]
    };
    host.draw("pad", drawing("many"));
    let look = ThemeLook::clarity(false);
    // A toe of the foot, just before the edge of the second box, 6.4 units off the line.
    let toe = pixel(&mut screen, 200 + 69 * 4, (56.4f64 * 4.0).round() as i32);
    assert!(toe.r() > 150 && toe.g() < 120, "the crow's foot spreads at the box: {toe:?}");
    assert_eq!(look.color("raised"), pixel(&mut screen, 200 + 75 * 4, 200), "the link stops at the box edge, drawn after it or not");
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    shot.save(format!("{SHOTS}/data-canvas-link-ends.png")).unwrap();
    host.draw("pad", drawing(""));
    assert_eq!(look.color("panel"), pixel(&mut screen, 200 + 69 * 4, (56.4f64 * 4.0).round() as i32), "a plain end shows nothing beside the line");
}

#[test]
fn a_diagram_draws_boxes_with_columns_and_a_dashed_relationship_between_them() {
    let host = Rc::new(CanvasHost::default());
    let mut screen = alone(r#"<Canvas name="pad" units="{&quot;width&quot;:800,&quot;height&quot;:400}" />"#, host.clone(), 800.0, 400.0);
    let text = |x: i64, y: i64, t: &str, color: &str, size: i64, align: &str, style: &str| json!({"op": "text", "x": x, "y": y, "text": t, "color": color, "size": size, "align": align, "style": style});
    let mut drawing = vec![clear()];
    drawing.push(json!({"op": "link", "name": "r1", "x1": 160, "y1": 120, "x2": 560, "y2": 200, "via": [], "color": "muted", "width": 2, "start": "one", "end": "zeroOrMany", "label": "project", "style": "dashed"}));
    for (x, y, title, rows) in [(60, 60, "Project", vec![("text(80)", "name", "UK *"), ("text", "summary", ""), ("datetime", "created", "")]), (460, 140, "Association", vec![("ref Project", "project", "FK *"), ("text(60)", "label", "")])] {
        let h = 36 + 22 * rows.len() as i64;
        drawing.push(rect((x - 2) as f64, (y - 2) as f64, 204.0, (h + 4) as f64, "accent"));
        drawing.push(rect(x as f64, y as f64, 200.0, h as f64, "raised"));
        drawing.push(rect(x as f64, y as f64, 200.0, 30.0, "accent_soft"));
        drawing.push(text(x + 100, y + 15, title, "text", 13, "center", "bold mono"));
        for (i, (ty, name, keys)) in rows.iter().enumerate() {
            let row = y + 41 + 22 * i as i64;
            drawing.push(text(x + 12, row, ty, "accent", 11, "left", "mono"));
            drawing.push(text(x + 100, row, name, "text", 11, "left", "mono"));
            drawing.push(text(x + 188, row, keys, "muted", 11, "right", "mono"));
        }
    }
    host.draw("pad", drawing);
    let look = ThemeLook::clarity(false);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    shot.save(format!("{SHOTS}/data-canvas-diagram.png")).unwrap();
    assert_eq!(look.color("accent_soft"), shot.pixel(64, 64), "the header");
    assert_eq!(look.color("raised"), shot.pixel(560 + 80, 200), "the relationship ends at the box, not inside it");
}

/// How long panning a diagram as large as the modelling example's takes
/// (2221 shapes): a measure, not a check, run by hand with
/// `cargo test --release -p clarity-ui --test data timing_a_pan -- --ignored --nocapture`.
/// It was 65 ms a frame unoptimized before the canvas kept its shapes and
/// the parts their state; 7 ms optimized after.
#[test]
#[ignore]
fn timing_a_pan_over_a_big_diagram() {
    let host = Rc::new(CanvasHost::default());
    let mut screen = alone(r#"<Canvas name="pad" zoom="true" units="{&quot;width&quot;:4000,&quot;height&quot;:3000}" />"#, host.clone(), 1600.0, 900.0);
    let mut drawing = vec![clear()];
    for i in 0..60 {
        let (x, y) = (40 + (i % 10) * 380, 40 + (i / 10) * 480);
        drawing.push(json!({"op": "link", "name": format!("l{i}"), "x1": x + 100, "y1": y + 100, "x2": 2000, "y2": 1500, "via": [], "color": "muted", "width": 2, "end": "many", "label": "x", "style": "dashed"}));
    }
    for i in 0..60 {
        let (x, y) = (40 + (i % 10) * 380, 40 + (i / 10) * 480);
        drawing.push(rect((x - 2) as f64, (y - 2) as f64, 304.0, 404.0, "accent"));
        drawing.push(rect(x as f64, y as f64, 300.0, 400.0, "raised"));
        drawing.push(rect(x as f64, y as f64, 300.0, 30.0, "accent_soft"));
        drawing.push(json!({"op": "text", "x": x + 150, "y": y + 15, "text": "Title", "style": "bold mono"}));
        for r in 0..16 {
            drawing.push(json!({"op": "text", "x": x + 12, "y": y + 41 + r * 22, "text": "string?", "color": "accent", "size": 11, "align": "left", "style": "mono"}));
            drawing.push(json!({"op": "text", "x": x + 100, "y": y + 41 + r * 22, "text": "someName", "size": 11, "align": "left", "style": "mono"}));
        }
    }
    println!("{} shapes", drawing.len());
    host.draw("pad", drawing);
    Shot::of(&mut screen, 1600, 900, 1.0);
    let t = std::time::Instant::now();
    screen.pointer_pressed(800.0, 450.0, PointerButton::Left, 1);
    for i in 0..10 {
        screen.pointer_moved(800.0 + i as f64 * 10.0, 450.0);
        Shot::of(&mut screen, 1600, 900, 1.0);
    }
    println!("10 pan frames: {:?}", t.elapsed());
    let c = the_canvas(&screen);
    let t = std::time::Instant::now();
    for _ in 0..10 { let _ = c.shown(); }
    println!("10 shown(): {:?}", t.elapsed());
    let t = std::time::Instant::now();
    for _ in 0..10 { let _ = c.drawing_sent(); }
    println!("10 drawing_sent(): {:?}", t.elapsed());
    let t = std::time::Instant::now();
    for _ in 0..10 { let _ = c.drawing(); }
    println!("10 drawing(): {:?}", t.elapsed());
    let t = std::time::Instant::now();
    for _ in 0..10 { let _ = c.box_at((10.0, 10.0)); }
    println!("10 box_at(): {:?}", t.elapsed());
}
