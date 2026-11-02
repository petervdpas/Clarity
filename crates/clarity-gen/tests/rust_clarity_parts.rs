//! Custom parts of a Rust program on Clarity.UI (DESIGN 7.9, "Custom
//! controls", Rust): a crate of the project whose type implements the typed
//! trait Clarity writes from the declaration into the crate's
//! `src/clarity.rs`, drawn on Clarity.UI's own surface; the program
//! registers each part by its name in the generated `src/parts.rs`.

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

fn settings(ui: Ui) -> Settings {
    let mut settings = starter("Game", "game");
    settings.stack.backend = Backend::Rust;
    settings.stack.ui = ui;
    settings
}

/// The project with its part's crate as declaring it wrote it for `ui`.
fn project(ui: Ui) -> clarity_spec::Project {
    let mut p = load_keyed([("game.clarity", GAME)], &[], "en");
    assert!(p.diagnostics.is_empty(), "{:#?}", p.diagnostics);
    let files = parts::starter_for_ui(&p.files[0].parts[0], ui).into_iter().map(|(path, text)| PartFile { part: "Gauge".into(), path, bytes: text.into_bytes() }).collect();
    clarity_spec::native::attach(&mut p, files);
    p
}

fn files() -> Vec<OutFile> {
    generate(&project(Ui::Clarity), &settings(Ui::Clarity)).unwrap_or_else(|errors| panic!("{errors:#?}"))
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

/// The part's crate is a dependency of the program; the native part host
/// (the runtime's `parts` feature, GTK's crates) is the webview's alone.
#[test]
fn the_manifest_names_the_parts_crate_and_no_native_host() {
    let cargo = file(&files(), "Cargo.toml").to_string();
    has(&cargo, "gauge = { path = \"parts/Gauge\" }\n");
    assert!(!cargo.contains("gtk") && !cargo.contains("gdkx11"), "{cargo}");
    assert!(!cargo.contains("\"parts\""), "{cargo}");
}

/// `src/parts.rs` registers each part with Clarity.UI by its name, and main
/// does that before the first window opens.
#[test]
fn the_program_registers_its_parts_with_clarity_ui() {
    let files = files();
    let glue = file(&files, "src/parts.rs");
    has(glue, "pub fn register() {\n    clarity_ui::parts::custom::register(\"Gauge\", gauge::make);\n}\n");
    assert!(!glue.contains("clarity_runtime::parts"), "{glue}");
    let main = file(&files, "src/main.rs");
    has(main, "mod parts;\n");
    has(main, "    parts::register();\n    clarity_ui::desktop::run(app, FILES)\n");
}

/// The interface: a typed method per value, property and hear (as C#'s
/// ShowValue, Show<Property> and On<Hear>), the drawing and input of a
/// control, and the part's words to its window, `changed` and one per tell.
#[test]
fn the_interface_is_written_from_the_declaration() {
    let files = files();
    let interface = file(&files, "parts/Gauge/src/clarity.rs");
    assert_eq!(interface, parts::rust_ui_interface(&project(Ui::Clarity).files[0].parts[0]));
    has(interface, "pub trait GaugePart: Default + 'static {\n");
    has(interface, "    fn draw(&self, painter: &Painter, area: Rect);\n");
    has(interface, "    fn show_value(&mut self, value: i64, cx: &mut CustomCx);\n");
    has(interface, "    fn show_speed(&mut self, speed: i64, cx: &mut CustomCx);\n");
    has(interface, "    fn show_title(&mut self, title: String, cx: &mut CustomCx);\n");
    has(interface, "    fn on_restart(&mut self, level: i64, cx: &mut CustomCx);\n");
    has(interface, "    fn on_say(&mut self, words: String, loud: bool, cx: &mut CustomCx);\n");
    has(interface, "    fn key(&mut self, key: KeyInput, cx: &mut CustomCx) -> bool {\n");
    has(interface, "pub trait GaugeTells {\n");
    has(interface, "    fn changed(&mut self, value: i64);\n");
    has(interface, "    fn reached(&mut self, level: i64, by: &str);\n");
    has(interface, "impl GaugeTells for CustomCx<'_> {\n");
    has(interface, "        self.tell(\"reached\", vec![Value::from(level), Value::from(by)]);\n");
    has(interface, "            \"speed\" => self.0.show_speed(integer(value), cx),\n");
    has(interface, "            \"restart\" => self.0.on_restart(integer(values.get(0).unwrap_or(&Value::Null)), cx),\n");
    has(interface, "pub fn make() -> Box<dyn CustomPart> {\n    Box::new(Held(crate::Gauge::default()))\n}\n");
    assert!(!interface.contains("raw_window_handle"), "{interface}");
}

/// Declaring the part on Clarity.UI writes a crate on clarity-ui whose type
/// draws its value in a frame, with every method the interface asks for.
#[test]
fn the_starter_is_a_control_of_clarity_ui() {
    let part = &project(Ui::Clarity).files[0].parts[0];
    let starter = parts::starter_for_ui(part, Ui::Clarity);
    let of = |path: &str| starter.iter().find(|(p, _)| p == path).map(|(_, t)| t.clone()).unwrap_or_else(|| panic!("no {path}"));
    let cargo = of("Cargo.toml");
    has(&cargo, &format!("clarity-ui = \"{}\"\n", clarity_gen::rust_ui_version()));
    has(&cargo, "serde_json = \"1\"\n");
    assert!(!cargo.contains("raw-window-handle"), "{cargo}");
    let lib = of("src/lib.rs");
    has(&lib, "mod clarity;\npub use clarity::*;\n");
    has(&lib, "#[derive(Default)]\npub struct Gauge {\n");
    has(&lib, "impl GaugePart for Gauge {\n");
    has(&lib, "    fn draw(&self, painter: &Painter, area: Rect) {\n");
    has(&lib, "    fn show_value(&mut self, value: i64, cx: &mut CustomCx) {\n        self.shown = value.to_string();\n        cx.invalidate();\n    }\n");
    has(&lib, "    fn on_say(&mut self, words: String, loud: bool, cx: &mut CustomCx) {\n        let _ = (words, loud, cx);\n    }\n");
    assert_eq!(of("src/clarity.rs"), parts::rust_ui_interface(part));
    // The webview's starter is the native one, unchanged.
    assert!(parts::starter_for_ui(part, Ui::Webview).iter().any(|(p, t)| p == "Cargo.toml" && t.contains("raw-window-handle")));
}

/// A part's decimal and date values cross as text, read with their crates.
#[test]
fn decimals_and_dates_are_read_with_their_crates() {
    let text = GAME.replace("speed: integer, min 1, max 10, default 3", "rate: decimal\n    due: date");
    let p = load_keyed([("game.clarity", text.as_str())], &[], "en");
    let part = &p.files[0].parts[0];
    let interface = parts::rust_ui_interface(part);
    has(&interface, "    fn show_rate(&mut self, rate: rust_decimal::Decimal, cx: &mut CustomCx);\n");
    has(&interface, "    fn show_due(&mut self, due: Option<chrono::NaiveDate>, cx: &mut CustomCx);\n");
    let starter = parts::starter_for_ui(part, Ui::Clarity);
    let cargo = &starter.iter().find(|(p, _)| p == "Cargo.toml").unwrap().1;
    has(cargo, "rust_decimal = \"1\"\n");
    has(cargo, "chrono = \"0.4\"\n");
}
