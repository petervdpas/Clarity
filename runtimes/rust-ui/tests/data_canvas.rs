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
    fn change_state(&self, name: &str, change: &mut dyn FnMut(&mut PartState)) {
        self.inner.change_state(name, change)
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
fn a_drag_the_handler_does_nothing_with_pans_a_canvas_that_zooms() {
    let (host, mut screen) = canvas_screen(&format!(r#"{UNITS} zoom="true" ondrag="moved""#));
    let before = the_canvas(&screen).viewport();
    press(&mut screen, 300.0, 200.0);
    screen.pointer_moved(320.0, 200.0);
    screen.pointer_moved(340.0, 200.0);
    release(&mut screen, 340.0, 200.0);
    assert_eq!(1, host.points.borrow().len());
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
    assert_eq!(Some(""), the_canvas(&screen).typing());
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
    assert_eq!(ThemeLook::clarity(false).color("text"), pixmap.get_color((15, 15)));
    assert_eq!(0, pixmap.get_color((80, 20)).a());
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
