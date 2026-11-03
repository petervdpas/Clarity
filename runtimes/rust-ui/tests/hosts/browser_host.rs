//! The browser host (round B) without a browser: the page's keys, pointer,
//! wheel and shapes as Clarity.UI takes them, the program's files as the
//! server serves them, and the host on Skia's raster surface, telling the
//! page what only the page can do.

use std::sync::{Arc, Mutex};

use clarity_runtime::program::Program;
use clarity_ui::Key;
use clarity_ui::browser::host::{BrowserHost, PointerKind};
use clarity_ui::browser::input::{button_of, cursor_css, key_of, modifiers_of, typed_text, wheel_of};
use clarity_ui::browser::page::{PageClipboard, PageFiles, Tell};
use clarity_ui::files::FilePicker;
use clarity_ui::input::{Cursor, Modifiers, PointerButton};
use clarity_ui::overlay::Clipboard;
use clarity_ui::Element;
use serde_json::{Value, json};

use super::core_windows_fake::{FILES, Fake};

#[test]
fn keys_are_what_they_write_else_where_they_lie() {
    assert_eq!(key_of("a", "KeyA"), Key::A);
    assert_eq!(key_of("A", "KeyA"), Key::A);
    // A key that writes no Latin letter is the key where it lies.
    assert_eq!(key_of("ф", "KeyA"), Key::A);
    assert_eq!(key_of("Enter", "Enter"), Key::Enter);
    assert_eq!(key_of("Enter", "NumpadEnter"), Key::Enter);
    assert_eq!(key_of("ArrowLeft", "ArrowLeft"), Key::Left);
    assert_eq!(key_of("F5", "F5"), Key::F5);
    assert_eq!(key_of("+", "Equal"), Key::Plus);
    assert_eq!(key_of("=", "Equal"), Key::Equal);
    assert_eq!(key_of(" ", "Space"), Key::Space);
    assert_eq!(key_of("Dead", "Digit6"), Key::D6);
    assert_eq!(key_of("1", "Numpad1"), Key::D1);
    assert_eq!(key_of("Control", "ControlLeft"), Key::Control);
    assert_eq!(key_of("AltGraph", "AltRight"), Key::Alt);
    assert_eq!(key_of("Unidentified", ""), Key::Other);
}

#[test]
fn held_keys_buttons_wheel_and_shapes_as_the_page_sends_them() {
    let held = modifiers_of(1 | 2);
    assert!(held.has(Modifiers::SHIFT) && held.has(Modifiers::CONTROL) && !held.has(Modifiers::ALT));
    assert!(modifiers_of(8).has(Modifiers::META));
    assert_eq!(modifiers_of(0), Modifiers::NONE);
    assert_eq!(button_of(0), Some(PointerButton::Left));
    assert_eq!(button_of(2), Some(PointerButton::Right));
    assert_eq!(button_of(3), None);
    // Pixels as they are, a line a notch as on the desktop.
    // As the desktop: CSS pixels are the screen's units, a line a notch; a page the canvas's size.
    assert_eq!(wheel_of(0.0, 120.0, 0, (900.0, 600.0)), (0.0, 120.0));
    assert_eq!(wheel_of(1.0, -3.0, 1, (900.0, 600.0)), (clarity_ui::input::WHEEL_NOTCH, -3.0 * clarity_ui::input::WHEEL_NOTCH));
    assert_eq!(wheel_of(0.0, 1.0, 2, (900.0, 600.0)), (0.0, 600.0));
    assert_eq!(cursor_css(Cursor::Hand), "pointer");
    assert_eq!(cursor_css(Cursor::ResizeLeftRight), "ew-resize");
    assert_eq!(typed_text("a", Modifiers::NONE), Some("a".to_string()));
    assert_eq!(typed_text("a", Modifiers::CONTROL), None);
    assert_eq!(typed_text("\n", Modifiers::NONE), None);
    assert_eq!(PointerKind::of(1), Some(PointerKind::Press));
    assert_eq!(PointerKind::of(9), None);
}

#[test]
fn the_programs_files_read_as_the_server_serves_them() {
    let served = json!({
        "title": "Orders",
        "width": 800,
        "height": 500,
        "windows": { "Main": "<Window name=\"Main\" />" },
        "apps": {},
        "themes": { "shop.light": { "colors": {} } },
        "model": { "project": {} },
        "resources": ["logo.png", "missing.png"]
    });
    let files = clarity_ui::browser::files_from_json(&served, &|name| (name == "logo.png").then(|| vec![1, 2, 3]));
    assert_eq!(files.title, "Orders");
    assert_eq!((files.width, files.height), (800.0, 500.0));
    assert_eq!(files.window("Main"), Some("<Window name=\"Main\" />"));
    // A theme and the model come as JSON values or as their text.
    assert_eq!(files.theme(false), Some(r#"{"colors":{}}"#));
    assert_eq!(files.model, r#"{"project":{}}"#);
    // A resource the server does not give is left out.
    assert_eq!(files.resource("logo.png"), Some(&[1u8, 2, 3][..]));
    assert_eq!(files.resources.len(), 1);
}

/// The person's choices are the process's: the tests that start a host take turns.
static ONE_HOST: Mutex<()> = Mutex::new(());

fn one_host() -> std::sync::MutexGuard<'static, ()> {
    ONE_HOST.lock().unwrap_or_else(|p| p.into_inner())
}

fn told() -> (Tell, Arc<Mutex<Vec<Value>>>) {
    let said: Arc<Mutex<Vec<Value>>> = Arc::default();
    let kept = said.clone();
    (Arc::new(move |message| kept.lock().unwrap().push(message)), said)
}

fn tells(said: &Arc<Mutex<Vec<Value>>>, tell: &str) -> Vec<Value> {
    said.lock().unwrap().iter().filter(|m| m["tell"] == json!(tell)).cloned().collect()
}

#[test]
fn the_host_runs_the_program_from_the_pages_pointer_and_tells_the_page() {
    let _turn = one_host();
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let (tell, said) = told();
    let mut host = BrowserHost::start(program, FILES, 900.0, 600.0, 2.0, false, tell);
    assert_eq!(tells(&said, "title"), [json!({ "tell": "title", "title": "Orders" })]);
    assert_eq!(tells(&said, "cursor").len(), 1);
    assert!(host.frame(), "the first frame draws");
    let mut surface = skia_safe::surfaces::raster_n32_premul((1800, 1200)).unwrap();
    host.draw(surface.canvas());

    let read = host.step(&json!({ "do": "read" }));
    let at = read["shown"]["save"].as_array().unwrap_or_else(|| panic!("{read}")).iter().map(|v| v.as_f64().unwrap()).collect::<Vec<_>>();
    let (x, y) = (at[0] + at[2] / 2.0, at[1] + at[3] / 2.0);
    fake.calls();
    host.pointer(PointerKind::Move, x, y, 0, 0);
    host.pointer(PointerKind::Press, x, y, 0, 0);
    host.pointer(PointerKind::Release, x, y, 0, 0);
    assert_eq!(fake.calls(), ["dispatch save"]);
    assert!(host.frame(), "the reply is drawn");
    // The pointer's shape is told when it changes only: a button keeps the arrow.
    assert_eq!(tells(&said, "cursor"), [json!({ "tell": "cursor", "cursor": "default" })]);
    // Ctrl+S runs it too.
    host.key(true, "s", "KeyS", 2);
    host.key(false, "s", "KeyS", 2);
    assert_eq!(fake.calls(), ["dispatch save"]);
    assert!(!host.ended());
}

#[test]
fn the_clipboard_and_files_go_through_the_page() {
    let (tell, said) = told();
    let known = std::rc::Rc::new(std::cell::RefCell::new(Some("pasted".to_string())));
    let mut clipboard = PageClipboard::new(tell.clone(), known.clone());
    assert_eq!(clipboard.get_text().as_deref(), Some("pasted"));
    clipboard.set_text("copied");
    assert_eq!(tells(&said, "copy"), [json!({ "tell": "copy", "text": "copied" })]);
    assert_eq!(clipboard.get_text().as_deref(), Some("copied"));

    let files = PageFiles::new(tell);
    assert!(!files.picks_folders());
    assert_eq!(files.save_file("Save", "a/b:c.txt", b"hi").as_deref(), Some("b_c.txt"));
    assert_eq!(tells(&said, "download"), [json!({ "tell": "download", "name": "b_c.txt", "data": "aGk=" })]);
    // Opening asks the page's file input; the file it gives is taken by the next open.
    assert_eq!(files.open_file("Open", None), None);
    assert_eq!(tells(&said, "pick").len(), 1);
    files.give(clarity_ui::files::ChosenFile { name: "x.txt".into(), bytes: b"x".to_vec() });
    assert_eq!(files.open_file("Open", None).map(|f| f.name).as_deref(), Some("x.txt"));
}

/// A long call in the browser: the handler's request goes out without
/// waiting (it names its watch), the bar shows and asks how far it is,
/// Stop is sent while the handler runs, and the answer is taken when it comes.
#[test]
fn a_long_call_is_not_waited_for_and_its_bar_asks_and_stops_meanwhile() {
    let _turn = one_host();
    let fake = Arc::new(Fake { watches: true, ..Fake::default() });
    let program: Arc<dyn Program> = fake.clone();
    let (tell, _) = told();
    let mut host = BrowserHost::start(program, FILES, 900.0, 600.0, 1.0, false, tell);
    type Sent = Vec<(String, serde_json::Map<String, Value>, Box<dyn FnOnce(Option<Value>)>)>;
    let sent: std::rc::Rc<std::cell::RefCell<Sent>> = Default::default();
    let keeps = sent.clone();
    host.set_asker(std::rc::Rc::new(move |command: &str, args, answered| keeps.borrow_mut().push((command.to_string(), args, answered))));
    host.frame();
    let read = host.step(&json!({ "do": "read" }));
    let at: Vec<f64> = read["shown"]["long"].as_array().unwrap_or_else(|| panic!("{read}")).iter().map(|v| v.as_f64().unwrap()).collect();
    let (x, y) = (at[0] + at[2] / 2.0, at[1] + at[3] / 2.0);
    fake.calls();
    host.pointer(PointerKind::Press, x, y, 0, 0);
    host.pointer(PointerKind::Release, x, y, 0, 0);
    host.frame();

    // Sent, not run: the server runs the handler under the watch the request names.
    let (command, args) = {
        let sent = sent.borrow();
        assert_eq!(sent.len(), 1, "one request went out");
        (sent[0].0.clone(), sent[0].1.clone())
    };
    assert_eq!(command, "dispatch_event");
    assert_eq!(args["handler"], json!("long"));
    let watch = args["watch"].as_u64().expect("the request names its watch");
    assert!(fake.calls().is_empty(), "the program was not asked to wait");
    fake.start_watch(watch);
    *fake.watch.lock().unwrap().as_ref().unwrap().said.lock().unwrap() = Some((1, 4));

    // The page draws on, the bar asks how far the call is.
    let screen = host.screen();
    assert!(screen.later_asking() == 1);
    screen.wake_due(std::time::Instant::now() + std::time::Duration::from_millis(250));
    host.frame();
    let bar = host.screen().find::<clarity_ui::dialogs::LongCallBar>().expect("the bar lies over the window");
    assert_eq!(bar.fraction(), 0.25);
    let stop = bar.stop_button().node().bounds;
    host.pointer(PointerKind::Press, stop.x + 4.0, stop.y + 4.0, 0, 0);
    host.pointer(PointerKind::Release, stop.x + 4.0, stop.y + 4.0, 0, 0);
    assert!(fake.watch.lock().unwrap().as_ref().unwrap().stopped(), "Stop reached the server's watch");

    // The answer comes: the window takes it, the bar goes, the watch ends.
    let (_, args, answered) = sent.borrow_mut().remove(0);
    let mut state: clarity_runtime::ui::WindowState = serde_json::from_value(args["state"].clone()).unwrap();
    state.data.insert("stopped".into(), json!(true));
    answered(Some(json!({ "value": super::core_windows_fake::reply(state) })));
    assert!(host.frame(), "the answer is drawn");
    assert!(host.screen().find::<clarity_ui::dialogs::LongCallBar>().is_none());
    assert!(fake.watch.lock().unwrap().is_none(), "end_watch followed");
    let read = host.step(&json!({ "do": "read" }));
    assert_eq!(read["data"]["stopped"], json!(true), "{read}");
}

/// The person's choices are the page's to keep: the ones it kept start the
/// program, and each choice made goes to the page with the others.
#[test]
fn the_persons_choices_start_from_and_go_to_the_page() {
    let _turn = one_host();
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    let (tell, said) = told();
    let kept = std::collections::BTreeMap::from([("language".to_string(), "nl".to_string()), ("columns.Main.grid".to_string(), "a,b".to_string())]);
    let _host = BrowserHost::start_kept(program, FILES, 900.0, 600.0, 1.0, false, tell, kept);
    assert_eq!(clarity_ui::choices::language(), "nl");
    assert_eq!(clarity_ui::choices::kept("columns.Main.grid").as_deref(), Some("a,b"));
    clarity_ui::choices::choose_language("en");
    let told = tells(&said, "choices");
    assert_eq!(told.last(), Some(&json!({ "tell": "choices", "choices": { "language": "en", "columns.Main.grid": "a,b" } })), "{told:?}");
}

/// A file shown in a tab goes as its real type, and only a kind that runs
/// nothing: HTML, SVG and the unknown are downloaded, never shown in the
/// page's origin.
#[test]
fn a_shown_file_has_its_type_and_html_or_svg_is_downloaded() {
    let (tell, said) = told();
    let files = PageFiles::new(tell);
    files.show("report.pdf", b"%PDF");
    files.show("notes.TXT", b"hi");
    files.show("page.html", b"<script>alert(1)</script>");
    files.show("logo.svg", b"<svg onload=\"alert(1)\"/>");
    files.show("blob", b"?");
    let shown = tells(&said, "show");
    assert_eq!(shown.iter().map(|t| (t["name"].clone(), t["type"].clone())).collect::<Vec<_>>(), [(json!("report.pdf"), json!("application/pdf")), (json!("notes.TXT"), json!("text/plain; charset=utf-8"))]);
    let downloaded: Vec<Value> = tells(&said, "download").iter().map(|t| t["name"].clone()).collect();
    assert_eq!(downloaded, [json!("page.html"), json!("logo.svg"), json!("blob")]);
    assert_eq!(clarity_ui::browser::page::shown_type("x.xhtml"), None);
}

/// A link opens only what it opens on the desktop: web pages, mail and phone numbers.
#[test]
fn a_link_opens_only_web_pages_mail_and_phone_numbers() {
    let _turn = one_host();
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    let (tell, said) = told();
    let _host = BrowserHost::start(program, FILES, 900.0, 600.0, 1.0, false, tell);
    for address in ["https://example.org/a", "http://example.org", "mailto:ada@example.org", "tel:+31201234567", "javascript:alert(1)", "JavaScript:alert(1)", "data:text/html,<script>alert(1)</script>", "file:///etc/passwd", "vbscript:x", "https://example.org/\nx"] {
        clarity_ui::window::address::open_with_host(address);
    }
    let opened: Vec<Value> = tells(&said, "open").iter().map(|t| t["address"].clone()).collect();
    assert_eq!(opened, [json!("https://example.org/a"), json!("http://example.org"), json!("mailto:ada@example.org"), json!("tel:+31201234567")]);
}

/// A touch the page takes away (it became a scroll) presses nothing: the release after it clicks nothing.
#[test]
fn a_cancelled_touch_clicks_nothing() {
    let _turn = one_host();
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let (tell, _) = told();
    let mut host = BrowserHost::start(program, FILES, 900.0, 600.0, 1.0, false, tell);
    let read = host.step(&json!({ "do": "read" }));
    let at: Vec<f64> = read["shown"]["save"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
    let (x, y) = (at[0] + at[2] / 2.0, at[1] + at[3] / 2.0);
    fake.calls();
    host.pointer(PointerKind::Press, x, y, 0, 0);
    host.pointer(PointerKind::Cancel, x, y, 0, 0);
    host.pointer(PointerKind::Release, x, y, 0, 0);
    assert!(fake.calls().is_empty(), "a cancelled touch ran a handler");
    host.pointer(PointerKind::Press, x, y, 0, 0);
    host.pointer(PointerKind::Release, x, y, 0, 0);
    assert_eq!(fake.calls(), ["dispatch save"]);
}

/// Sound plays through the page: the WAV goes to it, Stop tells it, and it is over once the page says it ended.
#[test]
fn sound_plays_through_the_page() {
    use clarity_ui::parts::sound::SoundOutput;
    let (tell, said) = told();
    let sounds: std::rc::Rc<std::cell::RefCell<clarity_ui::browser::page::Sounds>> = Default::default();
    let output = clarity_ui::browser::page::PageSound::new(tell, sounds.clone());
    let mut first = output.play(b"RIFF".to_vec(), 0.5, false);
    let second = output.play(b"RIFF".to_vec(), 1.0, true);
    let played = tells(&said, "play");
    assert_eq!(played[0], json!({ "tell": "play", "id": 1, "data": "UklGRg==", "volume": 0.5, "repeat": false }));
    assert_eq!((played[1]["id"].clone(), played[1]["repeat"].clone()), (json!(2), json!(true)));
    assert!(!first.ended() && !second.ended());
    sounds.borrow_mut().ended(2);
    assert!(second.ended());
    first.stop();
    first.stop();
    assert_eq!(tells(&said, "stop"), [json!({ "tell": "stop", "id": 1 })], "stopped once");
    assert!(first.ended());
}

/// files.json without a size (the main window has none): the program
/// starts all the same, on the page's canvas, which is the window's size.
#[test]
fn files_without_a_size_start_on_the_canvas() {
    let _turn = one_host();
    let served = json!({ "title": "Orders", "width": null, "height": null, "windows": { "Main": "<Window name=\"Main\"><Button name=\"go\" at=\"1,1\" span=\"3\" caption=\"Go\" /></Window>" }, "apps": {}, "themes": {}, "model": "", "resources": [] });
    let files = clarity_ui::browser::files_from_json(&served, &|_| None);
    assert_eq!((files.width, files.height), (0.0, 0.0), "no size: the library's own");
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    let (tell, _) = told();
    let mut host = BrowserHost::start(program, files, 390.0, 844.0, 2.0, false, tell);
    assert_eq!((host.screen().width(), host.screen().height()), (390.0, 844.0));
    assert!(host.frame());
    let read = host.step(&json!({ "do": "read" }));
    assert!(read["shown"]["go"].is_array(), "{read}");
}
