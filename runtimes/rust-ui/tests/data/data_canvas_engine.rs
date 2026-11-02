//! A canvas an engine beside the screen draws (conformance/screen-engines.json):
//! a frame at the screen's pace, the milliseconds since the last; pointer and
//! keys wait for the next frame; what the engine tells runs the window's
//! handler; an engine that fails says why in its place.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clarity_runtime::ui::PartState;
use clarity_ui::element::Node;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::markup::HandlerCall;
use clarity_ui::parts::data::canvas::engine::{key_name, Drawn, EngineCanvasPart, Frames, HeldKeys, Hears, ScreenInput, ScreenTell};
use clarity_ui::engines::Engines;
use clarity_ui::parts::data::canvas::{self, Shape};
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{Shot, TestHost};
use clarity_ui::{element_node, Element, Screen, ThemeLook};
use serde_json::{json, Map, Value};

fn conformance(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance").join(name);
    serde_json::from_str(&std::fs::read_to_string(path).expect("the conformance file reads")).expect("it is JSON")
}

/// What the test engine did, shared with the test: each frame's milliseconds and input, whether it was dropped.
#[derive(Default)]
struct Log {
    frames: Vec<(f64, Vec<ScreenInput>)>,
    fail_with: Option<String>,
    telling: Vec<ScreenTell>,
}

/// An engine that logs each frame and draws a square in the accent.
struct Engine {
    hears: Hears,
    log: Arc<Mutex<Log>>,
    dropped: Arc<AtomicBool>,
}

impl Frames for Engine {
    fn hears(&self) -> Hears {
        self.hears
    }

    fn frame(&mut self, ms: f64, input: &[ScreenInput]) -> Result<Drawn, String> {
        let mut log = self.log.lock().unwrap();
        if let Some(why) = log.fail_with.clone() {
            return Err(why);
        }
        log.frames.push((ms, input.to_vec()));
        let tells = std::mem::take(&mut log.telling);
        Ok(Drawn { shapes: vec![json!({"op": "rect", "x": 0, "y": 0, "width": 10, "height": 10, "color": "accent"})], tells })
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

/// Makes a new instance of an engine beside the screen, or says why it cannot.
type MakeFrames = Rc<dyn Fn() -> Result<Box<dyn Frames>, String>>;

/// The shared test host with what the engine told.
#[derive(Default)]
struct EngineHost {
    inner: TestHost,
    tells: RefCell<Vec<(String, String, Vec<Value>)>>,
    /// The engines beside the screen the window has, by name.
    engines: RefCell<Vec<(String, MakeFrames)>>,
    /// The instances its canvases started, by number.
    running: Engines,
}

impl EngineHost {
    /// A host whose engines run each frame at once, as a test waits for nothing.
    fn here() -> Rc<EngineHost> {
        Rc::new(EngineHost { running: Engines::here(), ..EngineHost::default() })
    }
}

impl PartHost for EngineHost {
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
    fn hears(&self, part: &str) -> Vec<clarity_runtime::ui::Hear> {
        self.inner.hears(part)
    }
    fn told(&self, part: &str, tell: &str, values: &[Value]) {
        self.tells.borrow_mut().push((part.to_string(), tell.to_string(), values.to_vec()));
    }
    fn resource(&self, name: &str) -> Option<Vec<u8>> {
        self.inner.resource(name)
    }
    fn engine_start(&self, name: &str) -> Option<Result<u64, String>> {
        let make = self.engines.borrow().iter().find(|(n, _)| n == name).map(|(_, m)| m.clone())?;
        Some(make().map(|frames| self.running.start(frames)))
    }
    fn engine_hears(&self, id: u64) -> Hears {
        self.running.hears(id)
    }
    fn engine_frame(&self, id: u64, ms: f64, input: Vec<ScreenInput>) {
        self.running.frame(id, ms, input)
    }
    fn engine_drawn(&self, id: u64) -> Option<Result<Drawn, String>> {
        self.running.drawn(id)
    }
    fn engine_stop(&self, id: u64) {
        self.running.stop(id)
    }
}

struct Run {
    host: Rc<EngineHost>,
    screen: Screen,
    log: Arc<Mutex<Log>>,
    dropped: Arc<AtomicBool>,
    now: Rc<Cell<Instant>>,
}

impl Run {
    /// The screen draws a frame `ms` milliseconds after the last.
    fn frame(&mut self, ms: u64) {
        self.now.set(self.now.get() + Duration::from_millis(ms));
        Shot::of(&mut self.screen, 320, 200, 1.0);
    }

    fn part(&self) -> &EngineCanvasPart {
        self.screen.root().as_any().downcast_ref::<EngineCanvasPart>().expect("the screen is an engine canvas")
    }

    fn last_input(&self) -> Vec<ScreenInput> {
        self.log.lock().unwrap().frames.last().map(|f| f.1.clone()).unwrap_or_default()
    }
}

fn engine_of(hears: Hears, log: Arc<Mutex<Log>>, dropped: Arc<AtomicBool>) -> MakeFrames {
    Rc::new(move || Ok(Box::new(Engine { hears, log: log.clone(), dropped: dropped.clone() }) as Box<dyn Frames>))
}

/// A canvas that runs `sky`, which the host has when `runs` makes it.
fn engine_canvas(markup: &str, host: Rc<EngineHost>, runs: Option<MakeFrames>) -> Box<dyn Element> {
    if let Some(runs) = runs {
        host.engines.borrow_mut().push(("sky".to_string(), runs));
    }
    let spec = clarity_ui::markup::read(&format!("<Window name=\"W\">{markup}</Window>")).unwrap().parts[0].clone();
    canvas::make(spec, host)
}

fn running(hears: Hears) -> Run {
    let host = EngineHost::here();
    let log = Arc::new(Mutex::new(Log::default()));
    let dropped = Arc::new(AtomicBool::new(false));
    let part = engine_canvas(r#"<Canvas name="pad" runs="sky" units="{&quot;width&quot;:160,&quot;height&quot;:100}" />"#, host.clone(), Some(engine_of(hears, log.clone(), dropped.clone())));
    let mut screen = Screen::new(part, ThemeLook::clarity(false));
    let now = Rc::new(Cell::new(Instant::now()));
    let clock = now.clone();
    screen.now = Box::new(move || clock.get());
    Run { host, screen, log, dropped, now }
}

fn pointer(kind: &str, x: i64, y: i64) -> ScreenInput {
    ScreenInput::Pointer { kind: kind.into(), x, y }
}

fn key(kind: &str, name: &str) -> ScreenInput {
    ScreenInput::Key { kind: kind.into(), name: name.into() }
}

#[test]
fn a_canvas_that_runs_an_engine_is_drawn_by_it_frame_by_frame() {
    let mut run = running(Hears::default());
    run.frame(0);
    run.frame(16);
    run.frame(17);
    let ms: Vec<f64> = run.log.lock().unwrap().frames.iter().map(|f| f.0).collect();
    assert_eq!(vec![0.0, 16.0, 17.0], ms);
    assert!(matches!(run.part().drawing(), [Shape::Rect { .. }]));
    // It asks the screen for every frame while it runs.
    assert_eq!(1, run.screen.animating());
    let mut shot = Shot::of(&mut run.screen, 320, 200, 1.0);
    shot.save("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots/data-canvas-engine.png").unwrap();
    assert_eq!(ThemeLook::clarity(false).color("accent"), shot.pixel(5, 5));
}

#[test]
fn an_engine_that_fails_says_why_in_its_place_and_runs_no_more() {
    let mut run = running(Hears::default());
    run.frame(0);
    run.log.lock().unwrap().fail_with = Some("out of time".into());
    run.frame(16);
    assert_eq!(Some("out of time"), run.part().stopped());
    assert!(run.dropped.load(Ordering::SeqCst));
    assert!(run.part().stopped_text().unwrap().contains("out of time"));
    assert_eq!(0, run.screen.animating());
}

#[test]
fn a_pointer_that_is_down_is_at_the_drawing_s_nearest_edge_beside_it() {
    let engines = conformance("screen-engines.json");
    let (uw, uh) = (engines["units"]["width"].as_f64().unwrap(), engines["units"]["height"].as_f64().unwrap());
    for c in engines["near"].as_array().unwrap() {
        let (part, at) = (&c["part"], &c["at"]);
        let got = canvas::point_near(uw, uh, part["width"].as_f64().unwrap(), part["height"].as_f64().unwrap(), at["x"].as_f64().unwrap(), at["y"].as_f64().unwrap());
        let want = if c["x"].is_null() { None } else { Some((c["x"].as_i64().unwrap(), c["y"].as_i64().unwrap())) };
        assert_eq!(want, got, "{c}");
    }
}

#[test]
fn the_pointer_goes_to_an_engine_that_takes_it_with_the_next_frame() {
    let mut run = running(Hears { pointer: true, keys: false });
    run.frame(0);
    // 160 x 100 units in 320 x 200 pixels: 2 pixels a unit.
    run.screen.pointer_pressed(20.0, 40.0, PointerButton::Left, 1);
    run.screen.pointer_moved(30.0, 40.0);
    run.screen.pointer_moved(40.0, 40.0);
    run.screen.pointer_released(400.0, 40.0, PointerButton::Left);
    run.frame(16);
    // The pointer comes over the canvas before it goes down there.
    assert_eq!(vec![pointer("move", 10, 20), pointer("down", 10, 20), pointer("move", 20, 20), pointer("up", 159, 20)], run.last_input());
}

#[test]
fn an_engine_that_takes_no_pointer_gets_none() {
    let mut run = running(Hears::default());
    run.frame(0);
    run.screen.pointer_pressed(20.0, 40.0, PointerButton::Left, 1);
    run.screen.pointer_released(20.0, 40.0, PointerButton::Left);
    run.frame(16);
    assert!(run.last_input().is_empty());
}

#[test]
fn keys_go_to_an_engine_that_takes_them_by_clarity_s_names() {
    let mut run = running(Hears { pointer: false, keys: true });
    run.frame(0);
    run.screen.pointer_pressed(20.0, 40.0, PointerButton::Left, 1);
    run.screen.pointer_released(20.0, 40.0, PointerButton::Left);
    assert_eq!(Some(run.screen.root().node().id()), run.screen.focused());
    run.screen.key_pressed(KeyInput::plain(Key::Left));
    // A key held repeats, and goes down once; it goes up when it is let go.
    run.screen.key_pressed(KeyInput::plain(Key::Left));
    // A shortcut with Control is the window's; Control itself is a key held.
    run.screen.key_pressed(KeyInput::new(Key::S, Modifiers::CONTROL));
    run.screen.key_pressed(KeyInput::new(Key::Shift, Modifiers::SHIFT));
    run.frame(16);
    assert_eq!(vec![key("down", "ArrowLeft"), key("down", "Shift")], run.last_input());
    run.screen.key_released(KeyInput::new(Key::Left, Modifiers::SHIFT));
    // Leaving the canvas lets go of every key still down.
    run.screen.focus(None);
    run.frame(16);
    assert_eq!(vec![key("up", "ArrowLeft"), key("up", "Shift")], run.last_input());
}

/// A root that takes a part and gives it back, as a window does when another opens over it.
struct Hooked {
    node: Node,
}

impl Element for Hooked {
    element_node!();

    fn arrange(&mut self, area: clarity_ui::Rect, cx: &mut clarity_ui::Cx) {
        self.node.bounds = area;
        for child in self.node.children_mut() {
            child.arrange(area, cx);
        }
    }
}

#[test]
fn a_canvas_that_leaves_the_screen_ends_its_engine_and_one_that_joins_again_starts_a_new_one() {
    let host = EngineHost::here();
    let made = Rc::new(Cell::new(0));
    let last_dropped: Rc<RefCell<Arc<AtomicBool>>> = Rc::new(RefCell::new(Arc::new(AtomicBool::new(false))));
    let log = Arc::new(Mutex::new(Log::default()));
    let (counter, kept) = (made.clone(), last_dropped.clone());
    let make: MakeFrames = Rc::new(move || {
        counter.set(counter.get() + 1);
        let dropped = Arc::new(AtomicBool::new(false));
        *kept.borrow_mut() = dropped.clone();
        Ok(Box::new(Engine { hears: Hears::default(), log: log.clone(), dropped }) as Box<dyn Frames>)
    });
    let mut screen = Screen::new(Box::new(Hooked { node: Node::new() }), ThemeLook::clarity(false));
    let canvas = engine_canvas(r#"<Canvas name="pad" runs="sky" />"#, host, Some(make));
    screen.act_root::<Hooked, _>(|root, _| root.node.add(canvas));
    Shot::of(&mut screen, 320, 200, 1.0);
    assert_eq!(1, made.get());
    let first = last_dropped.borrow().clone();
    let canvas = screen.act_root::<Hooked, _>(|root, cx| root.node.remove(0, cx)).unwrap();
    assert!(first.load(Ordering::SeqCst));
    assert_eq!(0, screen.animating());
    screen.act_root::<Hooked, _>(|root, _| root.node.add(canvas));
    Shot::of(&mut screen, 320, 200, 1.0);
    assert_eq!(2, made.get());
    assert!(!last_dropped.borrow().load(Ordering::SeqCst));
}

#[test]
fn every_key_clarity_ui_has_goes_by_the_name_every_renderer_gives_it() {
    let all = [
        Key::Tab, Key::Enter, Key::Escape, Key::Space, Key::Backspace, Key::Delete, Key::Left, Key::Right, Key::Up, Key::Down, Key::Home, Key::End, Key::PageUp, Key::PageDown,
        Key::Shift, Key::Control, Key::Alt,
    ];
    let named: Vec<String> = all.iter().chain(Key::LETTERS.iter()).chain(Key::DIGITS.iter()).chain(Key::FUNCTIONS.iter()).filter_map(|k| key_name(*k)).collect();
    for name in conformance("screen-engines.json")["keys"].as_array().unwrap() {
        assert!(named.iter().any(|n| n == name.as_str().unwrap()), "{name}");
    }
    assert_eq!(Some("a".to_string()), key_name(Key::A));
    assert_eq!(Some("7".to_string()), key_name(Key::D7));
    assert_eq!(Some("Enter".to_string()), key_name(Key::Enter));
    assert_eq!(Some("F12".to_string()), key_name(Key::F12));
    assert_eq!(None, key_name(Key::Tab));
    let mut held = HeldKeys::default();
    assert!(held.down("a"));
    assert!(!held.down("a"));
    assert!(held.down("ArrowLeft"));
    assert!(held.up("a"));
    assert!(!held.up("a"));
    assert_eq!(vec!["ArrowLeft".to_string()], held.all());
    assert!(held.all().is_empty());
}

#[test]
fn what_a_handler_tells_it_goes_with_the_next_frame_and_what_it_tells_runs_the_window_s_handler() {
    let mut run = running(Hears::default());
    // Told before it showed, it waits for it.
    run.host.inner.hears.borrow_mut().push(clarity_runtime::ui::Hear { part: "pad".into(), hear: "start".into(), values: vec![json!(2)] });
    run.frame(0);
    assert_eq!(vec![ScreenInput::Hear { name: "start".into(), values: vec![json!(2)] }], run.last_input());
    run.log.lock().unwrap().telling = vec![ScreenTell { tell: "scored".into(), values: vec![json!(3)] }];
    run.frame(16);
    assert_eq!(vec![("pad".to_string(), "scored".to_string(), vec![json!(3)])], *run.host.tells.borrow());
}

#[test]
fn moves_in_a_row_keep_only_the_last() {
    let mut waiting = Vec::new();
    for input in [pointer("move", 1, 1), pointer("move", 2, 2), pointer("down", 2, 2), pointer("move", 3, 3)] {
        ScreenInput::queue(&mut waiting, input);
    }
    assert_eq!(vec![pointer("move", 2, 2), pointer("down", 2, 2), pointer("move", 3, 3)], waiting);
}

#[test]
fn a_canvas_whose_engine_the_window_has_not_says_so() {
    let host = EngineHost::here();
    let part = engine_canvas(r#"<Canvas name="pad" runs="sky" />"#, host.clone(), None);
    let mut screen = Screen::new(part, ThemeLook::clarity(false));
    Shot::of(&mut screen, 320, 200, 1.0);
    assert!(screen.root().as_any().downcast_ref::<EngineCanvasPart>().unwrap().stopped().is_some());
    // Made by its kind, a canvas that runs an engine is one, whatever the window has.
    let spec = clarity_ui::markup::read(r#"<Window name="W"><Canvas name="pad" runs="sky" /></Window>"#).unwrap().parts[0].clone();
    let made: Box<dyn Element> = clarity_ui::parts::make(spec, host);
    assert!(made.as_any().downcast_ref::<EngineCanvasPart>().is_some());
    let _ = Map::<String, Value>::new();
}

#[test]
fn an_engine_on_its_own_thread_draws_the_frame_it_finished() {
    let host = Rc::new(EngineHost::default());
    let log = Arc::new(Mutex::new(Log::default()));
    let dropped = Arc::new(AtomicBool::new(false));
    let part = engine_canvas(r#"<Canvas name="pad" runs="sky" />"#, host.clone(), Some(engine_of(Hears::default(), log.clone(), dropped.clone())));
    let mut screen = Screen::new(part, ThemeLook::clarity(false));
    for _ in 0..200 {
        Shot::of(&mut screen, 320, 200, 1.0);
        if !screen.root().as_any().downcast_ref::<EngineCanvasPart>().unwrap().drawing().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!screen.root().as_any().downcast_ref::<EngineCanvasPart>().unwrap().drawing().is_empty());
    drop(screen);
    for _ in 0..200 {
        if dropped.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(dropped.load(Ordering::SeqCst));
}

#[test]
fn a_canvas_made_by_its_kind_gets_its_engine_from_the_window() {
    let host = EngineHost::here();
    let log = Arc::new(Mutex::new(Log::default()));
    let dropped = Arc::new(AtomicBool::new(false));
    host.engines.borrow_mut().push(("sky".to_string(), engine_of(Hears::default(), log.clone(), dropped)));
    let spec = clarity_ui::markup::read(r#"<Window name="W"><Canvas name="pad" runs="sky" /></Window>"#).unwrap().parts[0].clone();
    let made: Box<dyn Element> = clarity_ui::parts::make(spec, host);
    assert!(made.as_any().downcast_ref::<EngineCanvasPart>().is_some(), "an engine canvas");
    let mut screen = Screen::new(made, ThemeLook::clarity(false));
    Shot::of(&mut screen, 320, 200, 1.0);
    assert!(screen.root().as_any().downcast_ref::<EngineCanvasPart>().unwrap().stopped().is_none());
    assert!(!log.lock().unwrap().frames.is_empty());
}

// -- C# CanvasShapeTests: an engine's image shapes and its keys -------------

/// An engine that draws one picture over the whole canvas.
struct PictureEngine;

impl Frames for PictureEngine {
    fn hears(&self) -> Hears {
        Hears::default()
    }
    fn frame(&mut self, _ms: f64, _input: &[ScreenInput]) -> Result<Drawn, String> {
        Ok(Drawn { shapes: vec![json!({"op": "image", "x": 0, "y": 0, "width": 100, "height": 100, "picture": "sprite.png"})], tells: Vec::new() })
    }
}

/// An engine that hears keys and draws nothing.
struct KeysEngine;

impl Frames for KeysEngine {
    fn hears(&self) -> Hears {
        Hears { pointer: false, keys: true }
    }
    fn frame(&mut self, _ms: f64, _input: &[ScreenInput]) -> Result<Drawn, String> {
        Ok(Drawn { shapes: Vec::new(), tells: Vec::new() })
    }
}

fn a_png(color: skia_safe::Color) -> Vec<u8> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((4, 4)).unwrap();
    surface.canvas().clear(color);
    surface.image_snapshot().encode(None, skia_safe::EncodedImageFormat::PNG, 100).unwrap().as_bytes().to_vec()
}

fn engine_screen(host: Rc<EngineHost>, runs: MakeFrames) -> Screen {
    let part = engine_canvas(r#"<Canvas name="pad" runs="sky" units="{&quot;width&quot;:100,&quot;height&quot;:100}" />"#, host, Some(runs));
    let mut screen = Screen::new(part, ThemeLook::clarity(false));
    screen.resize(800.0, 400.0, 1.0);
    screen
}

#[test]
fn an_engine_canvas_draws_its_image_shapes() {
    let host = EngineHost::here();
    host.inner.resources.borrow_mut().insert("sprite.png".into(), a_png(skia_safe::Color::GREEN));
    let mut screen = engine_screen(host, Rc::new(|| Ok(Box::new(PictureEngine) as Box<dyn Frames>)));
    Shot::of(&mut screen, 800, 400, 1.0);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!(skia_safe::Color::GREEN, shot.pixel(400, 200));
}

#[test]
fn an_engine_canvas_that_hears_keys_takes_the_focus_when_pressed() {
    let host = EngineHost::here();
    let mut screen = engine_screen(host, Rc::new(|| Ok(Box::new(KeysEngine) as Box<dyn Frames>)));
    Shot::of(&mut screen, 800, 400, 1.0);
    screen.pointer_pressed(400.0, 200.0, PointerButton::Left, 1);
    screen.pointer_released(400.0, 200.0, PointerButton::Left);
    assert_eq!(Some(screen.root().node().id()), screen.focused());
}

// -- The host side: instances by number, plain data both ways --------------

#[test]
fn a_host_runs_its_canvases_engines_by_number_and_ends_them_when_they_stop() {
    let engines = Engines::here();
    let log = Arc::new(Mutex::new(Log::default()));
    let dropped = Arc::new(AtomicBool::new(false));
    let id = engines.start(Box::new(Engine { hears: Hears { pointer: true, keys: false }, log: log.clone(), dropped: dropped.clone() }));
    assert!(engines.hears(id).pointer);
    // Nothing drawn before a frame; a frame run here is drawn at once, and asked once.
    assert!(engines.drawn(id).is_none());
    engines.frame(id, 16.0, vec![pointer("down", 1, 2)]);
    assert!(matches!(engines.drawn(id), Some(Ok(_))));
    assert!(engines.drawn(id).is_none());
    assert_eq!(vec![(16.0, vec![pointer("down", 1, 2)])], log.lock().unwrap().frames);
    // Stopped, it ends and is gone: a frame of it says so.
    engines.stop(id);
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(0, engines.count());
    assert!(matches!(engines.drawn(id), Some(Err(_))));
    assert!(!engines.hears(id).pointer);
}

#[test]
fn an_engine_whose_thread_ended_says_so_with_its_next_frame() {
    let engines = Engines::new();
    let log = Arc::new(Mutex::new(Log { fail_with: Some("broken".into()), ..Log::default() }));
    let id = engines.start(Box::new(Engine { hears: Hears::default(), log, dropped: Arc::new(AtomicBool::new(false)) }));
    engines.frame(id, 0.0, Vec::new());
    let mut said = None;
    for _ in 0..400 {
        said = engines.drawn(id);
        if said.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(Some(Err("broken".to_string())), said.map(|d| d.map(|_| ())));
    // Its thread ended after the failure: the next frame says the engine ended.
    engines.frame(id, 16.0, Vec::new());
    let mut next = None;
    for _ in 0..400 {
        next = engines.drawn(id);
        if next.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(next, Some(Err(_))));
}

#[test]
fn a_canvas_that_goes_ends_its_engine_in_the_host() {
    let host = EngineHost::here();
    let log = Arc::new(Mutex::new(Log::default()));
    let dropped = Arc::new(AtomicBool::new(false));
    let part = engine_canvas(r#"<Canvas name="pad" runs="sky" />"#, host.clone(), Some(engine_of(Hears::default(), log, dropped.clone())));
    let mut screen = Screen::new(part, ThemeLook::clarity(false));
    Shot::of(&mut screen, 320, 200, 1.0);
    assert_eq!(1, host.running.count());
    drop(screen);
    assert_eq!(0, host.running.count());
    assert!(dropped.load(Ordering::SeqCst));
}

#[test]
fn an_engine_on_its_own_thread_draws_frame_after_frame() {
    let host = Rc::new(EngineHost::default());
    let log = Arc::new(Mutex::new(Log::default()));
    let dropped = Arc::new(AtomicBool::new(false));
    let part = engine_canvas(r#"<Canvas name="pad" runs="sky" />"#, host.clone(), Some(engine_of(Hears::default(), log.clone(), dropped)));
    let mut screen = Screen::new(part, ThemeLook::clarity(false));
    for _ in 0..400 {
        Shot::of(&mut screen, 320, 200, 1.0);
        if log.lock().unwrap().frames.len() >= 3 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(log.lock().unwrap().frames.len() >= 3, "{} frames", log.lock().unwrap().frames.len());
}
