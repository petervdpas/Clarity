//! Custom parts of a Rust program on Clarity.UI (DESIGN 7.9, "Custom
//! controls", Rust): a crate of the project whose type implements the typed
//! trait Clarity writes from the declaration into the crate's
//! `src/clarity.rs`, in plain Rust, as a Go part's `clarity.go`. The program's
//! `src/parts.rs` makes each part one of the runtime's `native_ui` and
//! registers it with Clarity.UI's library by its name.

use clarity_gen::{OutFile, generate, parts};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;
use clarity_spec::native::PartFile;

const GAME: &str = "module game

entity Player
    name: text(80), required
    best: integer

part Gauge
    crate \"parts/Gauge\"
    binds integer
    speed: integer, min 1, max 10, default 3
    title: text
    tells reached(level: integer, by: text)
    hears restart(level: integer)
    hears say(words: text, loud: bool)
    size 8 x 6

page Game: window
    data player: Player
    Gauge name gauge at 1,1 span 8 rows 6 bind player.best speed 5 on reached: gaugeReached

code Game
    handler gaugeReached(level: integer, by: text)
        message \"Reached\"
";

fn settings() -> Settings {
    let mut settings = starter("Game", "game");
    settings.stack.backend = Backend::Rust;
    settings.stack.ui = Ui::Clarity;
    settings
}

/// The project with its part's crate as declaring it wrote it.
fn project() -> clarity_spec::Project {
    let mut p = load_keyed([("game.clarity", GAME)], &[], "en");
    assert!(p.diagnostics.is_empty(), "{:#?}", p.diagnostics);
    let files = parts::starter_for(&p.files[0].parts[0]).into_iter().map(|(path, text)| PartFile { part: "Gauge".into(), path, bytes: text.into_bytes() }).collect();
    clarity_spec::native::attach(&mut p, files);
    p
}

fn files() -> Vec<OutFile> {
    generate(&project(), &settings()).unwrap_or_else(|errors| panic!("{errors:#?}"))
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn a_project_on_clarity_ui_with_a_rust_part_is_generated() {
    let files = files();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    for path in ["parts/Gauge/Cargo.toml", "parts/Gauge/src/lib.rs", "parts/Gauge/src/clarity.rs", "src/parts.rs"] {
        assert!(paths.contains(&path), "no {path} in {paths:?}");
    }
}

/// The part's crate is a dependency of the program, which links Clarity.UI's
/// library: no clarity-ui crate, no native part host.
#[test]
fn the_manifest_names_the_parts_crate_on_the_library() {
    let cargo = file(&files(), "Cargo.toml").to_string();
    has(&cargo, "gauge = { path = \"parts/Gauge\" }\n");
    has(&cargo, "\"native-ui\"");
    assert!(!cargo.contains("clarity-ui"), "{cargo}");
    assert!(!cargo.contains("gtk") && !cargo.contains("gdkx11"), "{cargo}");
    assert!(!cargo.contains("\"parts\""), "{cargo}");
}

/// `src/parts.rs` makes each part's crate a custom part of the runtime's
/// `native_ui`, whose canvas and context it forwards, and registers it with
/// Clarity.UI's library by its name; main does that before the first window opens.
#[test]
fn the_program_registers_its_parts_with_clarity_ui() {
    let files = files();
    let glue = file(&files, "src/parts.rs");
    has(glue, "use clarity_runtime::native_ui as ui;\n");
    has(glue, "mod part_gauge {\n    glue!(gauge);\n}\n");
    has(glue, "        (\"Gauge\", part_gauge::make),\n");
    has(glue, "        if let Err(why) = ui::register_part(kind, *make) {\n");
    // The canvas and the context, forwarded to the runtime's own.
    has(glue, "        impl $krate::Canvas for Canvas<'_> {\n");
    has(glue, "            fn pixels(&mut self, rgba: &[u8], width: u32, height: u32, stride: usize, x: f64, y: f64, box_width: f64, box_height: f64) {\n                self.0.pixels(rgba, width, height, stride, x, y, box_width, box_height)\n            }\n");
    has(glue, "            fn tell(&mut self, tell: &str, values: Vec<Value>) {\n                self.0.tell(tell, &values)\n            }\n");
    has(glue, "            fn pointer_pressed(&mut self, at: ui::Pointer, cx: &mut ui::Cx) {\n                self.0.pointer_pressed(pointer(at), &mut Cx(cx))\n            }\n");
    has(glue, "            fn key(&mut self, name: &str, modifiers: u32, cx: &mut ui::Cx) -> bool {\n                self.0.key(name, modifiers, &mut Cx(cx))\n            }\n");
    has(glue, "            fn stop(&mut self) {\n                self.0.stop()\n            }\n");
    assert!(!glue.contains("clarity_ui::") && !glue.contains("clarity_runtime::parts"), "{glue}");
    let main = file(&files, "src/main.rs");
    has(main, "mod parts;\n");
    has(main, "    parts::register();\n    clarity_runtime::native_ui::run(app, FILES)\n");
}

/// The interface: the canvas and the context in plain Rust, a typed method
/// per value, property and hear (as C#'s ShowValue, Show<Property> and
/// On<Hear>), the drawing and input of a control, and the part's words to its
/// window, `changed` and one per tell. Nothing of Clarity's crates.
#[test]
fn the_interface_is_written_from_the_declaration() {
    let files = files();
    let interface = file(&files, "parts/Gauge/src/clarity.rs");
    assert_eq!(interface, parts::rust_ui_interface(&project().files[0].parts[0]));
    has(interface, "pub trait GaugePart: Default + 'static {\n");
    has(interface, "    fn draw(&self, canvas: &mut dyn Canvas, x: f64, y: f64, width: f64, height: f64);\n");
    has(interface, "pub trait Canvas {\n");
    has(interface, "    fn look_color(&mut self, role: &str) -> u32;\n");
    has(interface, "pub trait Cx {\n");
    has(interface, "    fn wake_in(&mut self, ms: f64);\n");
    has(interface, "    fn translate(&self, key: &str, english: &str) -> String;\n");
    has(interface, "pub struct Pointer {\n");
    // Started once its window opened, after the values of its line.
    has(interface, "    fn start(&mut self, cx: &mut dyn Cx) {\n");
    has(interface, "        self.0.start(cx)\n");
    has(interface, "    fn show_value(&mut self, value: i64, cx: &mut dyn Cx);\n");
    has(interface, "    fn show_speed(&mut self, speed: i64, cx: &mut dyn Cx);\n");
    has(interface, "    fn show_title(&mut self, title: String, cx: &mut dyn Cx);\n");
    has(interface, "    fn on_restart(&mut self, level: i64, cx: &mut dyn Cx);\n");
    has(interface, "    fn on_say(&mut self, words: String, loud: bool, cx: &mut dyn Cx);\n");
    has(interface, "    fn key(&mut self, name: &str, modifiers: u32, cx: &mut dyn Cx) -> bool {\n");
    has(interface, "pub trait GaugeTells {\n");
    has(interface, "    fn changed(&mut self, value: i64);\n");
    has(interface, "    fn reached(&mut self, level: i64, by: &str);\n");
    has(interface, "impl<C: Cx + ?Sized> GaugeTells for C {\n");
    has(interface, "        self.tell(\"reached\", vec![Value::from(level), Value::from(by)]);\n");
    has(interface, "            \"speed\" => self.0.show_speed(integer(value), cx),\n");
    has(interface, "            \"restart\" => self.0.on_restart(integer(values.get(0).unwrap_or(&Value::Null)), cx),\n");
    has(interface, "pub fn make() -> Box<dyn Custom> {\n    Box::new(Held(crate::Gauge::default()))\n}\n");
    assert!(!interface.contains("clarity_ui") && !interface.contains("clarity_runtime"), "{interface}");
}

/// Declaring the part writes a crate whose type draws its value in a frame,
/// with every method the interface asks for, and nothing of Clarity's crates.
#[test]
fn the_starter_is_a_control_of_clarity_ui() {
    let part = &project().files[0].parts[0];
    let starter = parts::starter_for(part);
    let of = |path: &str| starter.iter().find(|(p, _)| p == path).map(|(_, t)| t.clone()).unwrap_or_else(|| panic!("no {path}"));
    let cargo = of("Cargo.toml");
    assert!(!cargo.contains("clarity-ui"), "{cargo}");
    has(&cargo, "serde_json = \"1\"\n");
    assert!(!cargo.contains("raw-window-handle"), "{cargo}");
    let lib = of("src/lib.rs");
    has(&lib, "mod clarity;\npub use clarity::*;\n");
    has(&lib, "#[derive(Default)]\npub struct Gauge {\n");
    has(&lib, "impl GaugePart for Gauge {\n");
    has(&lib, "    fn draw(&self, canvas: &mut dyn Canvas, x: f64, y: f64, width: f64, height: f64) {\n");
    has(&lib, "    fn show_value(&mut self, value: i64, cx: &mut dyn Cx) {\n        self.shown = value.to_string();\n        cx.invalidate();\n    }\n");
    has(&lib, "    fn on_say(&mut self, words: String, loud: bool, cx: &mut dyn Cx) {\n        let _ = (words, loud, cx);\n    }\n");
    assert_eq!(of("src/clarity.rs"), parts::rust_ui_interface(part));
}

/// A part's decimal and date values cross as text, read with their crates.
#[test]
fn decimals_and_dates_are_read_with_their_crates() {
    let text = GAME.replace("speed: integer, min 1, max 10, default 3", "rate: decimal\n    due: date");
    let p = load_keyed([("game.clarity", text.as_str())], &[], "en");
    let part = &p.files[0].parts[0];
    let interface = parts::rust_ui_interface(part);
    has(&interface, "    fn show_rate(&mut self, rate: rust_decimal::Decimal, cx: &mut dyn Cx);\n");
    has(&interface, "    fn show_due(&mut self, due: Option<chrono::NaiveDate>, cx: &mut dyn Cx);\n");
    let starter = parts::starter_for(part);
    let cargo = &starter.iter().find(|(p, _)| p == "Cargo.toml").unwrap().1;
    has(cargo, "rust_decimal = \"1\"\n");
    has(cargo, "chrono = \"0.4\"\n");
}

/// With custom parts a debug build optimizes the dependencies (the parts'
/// crates and what they use, such as a game engine), so the windows run at
/// speed while the program's own code stays quick to build; the README says
/// so. Without them the debug build stays plain: Clarity.UI's library is
/// built optimized already.
#[test]
fn a_debug_build_optimizes_the_dependencies_of_custom_parts() {
    let files = files();
    let cargo = file(&files, "Cargo.toml");
    has(cargo, "[profile.dev.package.\"*\"]\nopt-level = 3\n");
    has(file(&files, "README.md"), "[profile.dev.package.\"*\"]");

    let plain = load_keyed([("game.clarity", "module game\n\npage Game: window\n    title \"Game\"\n")], &[], "en");
    let files = generate(&plain, &settings()).unwrap_or_else(|errors| panic!("{errors:#?}"));
    assert!(!file(&files, "Cargo.toml").contains("[profile.dev.package"));
    assert!(!file(&files, "README.md").contains("[profile.dev.package"));
}
