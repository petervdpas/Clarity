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

/// Songs play in the page's audio elements: the page is told to play, pause,
/// seek and stop, and what it says of the song is what the song reads.
#[test]
fn songs_play_through_the_page() {
    use clarity_ui::browser::page::{PageMusic, SongSaid, Songs};
    use clarity_ui::parts::sound::Music;
    use std::time::Duration;
    let (tell, said) = told();
    let songs: std::rc::Rc<std::cell::RefCell<Songs>> = Default::default();
    let music = PageMusic::new(tell, songs.clone());
    let mut song = music.player().unwrap();
    song.set_volume(0.5);
    song.play_bytes(b"ID3".to_vec(), Some("mp3")).unwrap();
    assert_eq!(tells(&said, "songPlay"), [json!({ "tell": "songPlay", "id": 1, "data": "SUQz", "kind": "mp3", "volume": 0.5 })]);
    assert!(song.playing());
    songs.borrow_mut().said(1, SongSaid { playing: false, paused: true, ended: false, position: 12.5, length: 200.0 });
    assert!(song.paused() && !song.playing());
    assert_eq!(Duration::from_secs_f64(12.5), song.position());
    assert_eq!(Some(Duration::from_secs(200)), song.length());
    song.pause();
    song.seek(Duration::from_secs(30)).unwrap();
    let done: Vec<(Value, Value)> = tells(&said, "songDo").iter().map(|t| (t["do"].clone(), t["value"].clone())).collect();
    assert_eq!(done, [(json!("volume"), json!(0.5)), (json!("pause"), json!(0.0)), (json!("seek"), json!(30.0))]);
    // A browser plays no files of the computer's disk.
    assert_eq!("audioNoDisk", song.play_file("/music/song.flac").unwrap_err().key);
    drop(song);
    assert_eq!(tells(&said, "songDo").last().map(|t| t["do"].clone()), Some(json!("stop")));
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

/// A window as the IDE's designer gives it: each window, part and page with its `designItem`.
const DESIGNED: &str = r#"<Window name="Main" designItem="10">
  <Button name="save" at="1,1" span="3" caption="Save" designItem="20" />
  <Label at="5,1" span="3" caption="Unnamed" designItem="25" />
  <Panel name="box" at="1,2" span="6" rows="3" designItem="30">
    <Textbox name="inner" at="1,1" span="4" designItem="40" />
  </Panel>
  <Tabs name="tabs" at="7,2" span="6" rows="3" designItem="50">
    <Tab title="One" designItem="60">
      <Checkbox name="ok" at="1,1" span="3" designItem="70" />
    </Tab>
    <Tab title="Two" designItem="80" />
  </Tabs>
</Window>
"#;

/// The same window after the designer moved the button and renamed its caption.
const MOVED: &str = r#"<Window name="Main" designItem="10">
  <Button name="save" at="4,5" span="3" caption="Store" designItem="20" />
</Window>
"#;

fn designed(markup: &'static str) -> clarity_ui::AppFiles {
    clarity_ui::AppFiles { windows: Box::leak(vec![("Main", markup)].into_boxed_slice()), ..super::core_windows_fake::FILES }
}

fn place(layout: &Value, list: &str, item: &str) -> Vec<f64> {
    let found = layout[list].as_array().unwrap().iter().find(|p| p["item"] == json!(item)).unwrap_or_else(|| panic!("no {item} in {layout}"));
    found["at"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect()
}

#[test]
fn the_designers_layer_finds_each_part_and_grid_by_its_design_item() {
    let _turn = one_host();
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    let (tell, _) = told();
    let mut host = BrowserHost::start(program, designed(DESIGNED), 900.0, 600.0, 1.0, false, tell);
    let mut surface = skia_safe::surfaces::raster_n32_premul((900, 600)).unwrap();
    host.draw(surface.canvas());
    let layout = host.layout();
    // Every part shows with its design item, an unnamed one too; its name and kind with it.
    let save = layout["parts"].as_array().unwrap().iter().find(|p| p["item"] == json!("20")).unwrap_or_else(|| panic!("{layout}"));
    assert_eq!((save["name"].clone(), save["kind"].clone()), (json!("save"), json!("button")));
    let unnamed = layout["parts"].as_array().unwrap().iter().find(|p| p["item"] == json!("25")).unwrap();
    assert_eq!(unnamed["name"], Value::Null);
    let button = place(&layout, "parts", "20");
    let label = place(&layout, "parts", "25");
    assert!(label[0] > button[0] + button[2], "the label is right of the button: {layout}");
    // The panel's child inside the panel, the shown tab's part inside the tabs; the hidden tab's grid shows not.
    let panel = place(&layout, "parts", "30");
    let inner = place(&layout, "parts", "40");
    assert!(inner[0] >= panel[0] && inner[1] >= panel[1] && inner[1] + inner[3] <= panel[1] + panel[3] + 0.5, "{layout}");
    place(&layout, "parts", "70");
    // Each grid by what holds it: the window, the panel, the shown tab; with its columns and its rows.
    let window = layout["grids"].as_array().unwrap().iter().find(|g| g["item"] == json!("10")).unwrap();
    assert_eq!(window["columns"], json!(12));
    assert_eq!(window["gap"], json!(8.0));
    let rows: Vec<f64> = window["rows"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
    assert_eq!(rows.len(), 4, "{window}");
    assert_eq!(rows[0], 36.0);
    let panel_grid = layout["grids"].as_array().unwrap().iter().find(|g| g["item"] == json!("30")).unwrap();
    assert_eq!(panel_grid["columns"], json!(6));
    place(&layout, "grids", "60");
    assert!(!layout["grids"].as_array().unwrap().iter().any(|g| g["item"] == json!("80")), "a hidden tab's grid shows not");
}

#[test]
fn new_files_redraw_on_the_same_canvas_without_a_new_host() {
    let _turn = one_host();
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let (tell, said) = told();
    let mut host = BrowserHost::start(program, designed(DESIGNED), 900.0, 600.0, 2.0, true, tell);
    let mut surface = skia_safe::surfaces::raster_n32_premul((1800, 1200)).unwrap();
    host.draw(surface.canvas());
    let before = place(&host.layout(), "parts", "20");
    fake.calls();
    host.reload(designed(MOVED));
    // The program started again; its window shows the new markup at the same size, still dark.
    assert!(fake.calls().iter().any(|c| c.starts_with("open")), "the window opened again");
    assert_eq!((host.screen().width(), host.screen().height(), host.screen().scale(), host.screen().dark()), (900.0, 600.0, 2.0, true));
    assert!(host.frame(), "the new window is drawn");
    host.draw(surface.canvas());
    let layout = host.layout();
    let after = place(&layout, "parts", "20");
    assert!(after[0] > before[0] && after[1] > before[1], "moved: {before:?} -> {after:?}");
    assert!(!layout["parts"].as_array().unwrap().iter().any(|p| p["item"] == json!("30")), "the panel is gone");
    let read = host.step(&json!({ "do": "read" }));
    assert_eq!(read["window"], json!("Main"));
    // The page is told the title again, as for a new start.
    assert_eq!(tells(&said, "title").len(), 2);
}

/// A window with tabs the designer switched to their second page, a table and a toolbar.
const SWITCHED: &str = r#"<Window name="Main" designItem="10">
  <Toolbar>
    <Item name="saveTool" caption="Save" onclick="save" designItem="90" />
  </Toolbar>
  <Tabs name="tabs" at="1,1" span="6" rows="3" designItem="50" designShown="2">
    <Tab title="One" designItem="60">
      <Checkbox name="ok" at="1,1" span="3" designItem="70" />
    </Tab>
    <Tab title="Two" designItem="80">
      <Textbox name="other" at="1,1" span="3" designItem="85" />
    </Tab>
  </Tabs>
  <Table name="grid" at="1,4" span="12" rows="4" bind="orders" columns="note, status" designItem="95" />
</Window>
"#;

#[test]
fn the_designer_chooses_the_page_shown_and_finds_tabs_columns_and_frame_buttons() {
    let _turn = one_host();
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    let (tell, _) = told();
    let mut host = BrowserHost::start(program, designed(SWITCHED), 900.0, 600.0, 1.0, false, tell);
    let mut surface = skia_safe::surfaces::raster_n32_premul((900, 600)).unwrap();
    host.draw(surface.canvas());
    let layout = host.layout();
    // The second tab shows, as the designer chose; each page with the header that shows it.
    place(&layout, "parts", "85");
    place(&layout, "grids", "80");
    assert!(!layout["grids"].as_array().unwrap().iter().any(|g| g["item"] == json!("60")), "{layout}");
    let tabs = layout["parts"].as_array().unwrap().iter().find(|p| p["item"] == json!("50")).unwrap();
    let pages = tabs["pages"].as_array().unwrap();
    assert_eq!(pages.iter().map(|p| p["item"].clone()).collect::<Vec<_>>(), [json!("60"), json!("80")]);
    assert!(pages.iter().all(|p| p["at"].as_array().is_some_and(|a| a[2].as_f64().unwrap() > 0.0)), "{tabs}");
    // A table's columns, by field, where their headers are.
    let table = layout["parts"].as_array().unwrap().iter().find(|p| p["item"] == json!("95")).unwrap();
    let columns = table["columns"].as_array().unwrap_or_else(|| panic!("{table}"));
    assert_eq!(columns.iter().map(|c| c["field"].clone()).collect::<Vec<_>>(), [json!("note"), json!("status")]);
    // A button of the window's toolbar, by its design item.
    let tool = layout["parts"].as_array().unwrap().iter().find(|p| p["item"] == json!("90")).unwrap_or_else(|| panic!("{layout}"));
    assert_eq!(tool["kind"], json!("command"));
}

#[test]
fn reloaded_files_are_given_up_once_the_next_are_drawn() {
    let _turn = one_host();
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    let (tell, _) = told();
    let served = |markup: &str| json!({ "title": "Orders", "width": 800, "height": 500, "windows": { "Main": markup }, "apps": {}, "themes": {}, "model": super::core_windows_fake::MODEL, "resources": [] });
    let mut host = BrowserHost::start(program, designed(DESIGNED), 900.0, 600.0, 1.0, false, tell);
    let (files, first) = clarity_ui::browser::owned_files_from_json(&served(DESIGNED), &|_| None);
    let first_alive = first.alive();
    host.reload_owned(files, Some(first));
    assert!(first_alive.upgrade().is_some(), "the files drawn now are kept");
    let (files, second) = clarity_ui::browser::owned_files_from_json(&served(MOVED), &|_| None);
    let second_alive = second.alive();
    host.reload_owned(files, Some(second));
    assert!(first_alive.upgrade().is_none(), "the files before are given up");
    assert!(second_alive.upgrade().is_some());
    let mut surface = skia_safe::surfaces::raster_n32_premul((900, 600)).unwrap();
    host.draw(surface.canvas());
    assert!(host.layout()["parts"].as_array().unwrap().iter().any(|p| p["item"] == json!("20")));
    // The person's choices win at the next reload: none here, the language as written.
    host.choose(std::collections::BTreeMap::from([("language".to_string(), "xx".to_string())]));
    host.reload(designed(MOVED));
    assert_eq!(host.step(&json!({ "do": "read" }))["window"], json!("Main"));
}

#[test]
fn a_models_facts_are_known_by_its_text() {
    use clarity_ui::facts::ModelFacts;
    let one = String::from(super::core_windows_fake::MODEL);
    let again = String::from(super::core_windows_fake::MODEL);
    assert!(Arc::ptr_eq(&ModelFacts::of(&one), &ModelFacts::of(&again)), "the same text, read once");
    let other = super::core_windows_fake::MODEL.replacen('{', "{\"other\": 1, ", 1);
    assert!(!Arc::ptr_eq(&ModelFacts::of(&one), &ModelFacts::of(&other)));
}

fn drawn(host: &mut BrowserHost) {
    host.frame();
    let mut surface = skia_safe::surfaces::raster_n32_premul((900, 600)).unwrap();
    host.draw(surface.canvas());
}

fn updates(said: &Arc<Mutex<Vec<Value>>>) -> Vec<Value> {
    let all = tells(said, "access").into_iter().map(|t| t["update"].clone()).collect();
    said.lock().unwrap().retain(|m| m["tell"] != json!("access"));
    all
}

fn node_of<'a>(update: &'a Value, role: &str, name: &str) -> &'a Value {
    update["nodes"].as_array().unwrap().iter().find(|n| n["role"] == json!(role) && n["name"] == json!(name)).unwrap_or_else(|| panic!("no {role} {name} in {update}"))
}

/// The page's accessible mirror (A1 phase 2): off until the page asks; then
/// the whole tree after the next drawn frame, only changes after that,
/// nothing when nothing changed; the mirror's actions run as the person's
/// input; a message is said; a reload sends the whole tree again; off, it
/// is told nothing.
#[test]
fn the_page_is_told_the_trees_changes_and_its_actions_run() {
    let _turn = one_host();
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let (tell, said) = told();
    let mut host = BrowserHost::start(program, FILES, 900.0, 600.0, 1.0, false, tell);
    drawn(&mut host);
    assert!(updates(&said).is_empty(), "no mirror until the page asks");

    host.set_access(true);
    assert!(host.frame(), "turning the mirror on draws again");
    let mut surface = skia_safe::surfaces::raster_n32_premul((900, 600)).unwrap();
    host.draw(surface.canvas());
    let first = updates(&said);
    assert_eq!(first.len(), 1);
    let whole = &first[0];
    assert_eq!(whole["whole"], json!(true));
    let save = node_of(whole, "button", "Save")["id"].as_str().expect("ids as text").to_string();
    assert_eq!(node_of(whole, "window", "Orders")["id"], whole["root"]);
    drawn(&mut host);
    assert!(updates(&said).is_empty(), "nothing changed, nothing told");

    // Focus from the mirror: the screen's moves, and the page is told.
    assert!(host.access_act(&save, "focus", None));
    drawn(&mut host);
    let moved = updates(&said);
    assert_eq!(moved.last().unwrap()["focus"], json!(save));
    assert_eq!(moved.last().unwrap()["whole"], json!(false));

    // A click from the mirror runs the handler; the message it brings is said.
    fake.calls();
    let warn = node_of(whole, "button", "Warn")["id"].as_str().unwrap().to_string();
    assert!(host.access_act(&warn, "click", None));
    assert_eq!(fake.calls(), ["dispatch warn"]);
    drawn(&mut host);
    let said_now: Vec<Value> = updates(&said).iter().flat_map(|u| u["say"].as_array().unwrap().clone()).collect();
    assert!(said_now.iter().any(|s| s["text"].as_str().unwrap_or_default().contains("Careful")), "{said_now:?}");
    // No such node, no such action: nothing done.
    assert!(!host.access_act("12345678", "click", None));
    assert!(!host.access_act(&save, "fly", None));
    assert!(!host.access_act("not a number", "click", None));

    // A reload is a new screen: the whole tree again.
    host.reload(FILES);
    drawn(&mut host);
    let again = updates(&said);
    assert_eq!(again.first().map(|u| u["whole"].clone()), Some(json!(true)), "{again:?}");

    host.set_access(false);
    host.access_act(&save, "focus", None);
    drawn(&mut host);
    assert!(updates(&said).is_empty(), "off: nothing told");
}

/// The program on the browser page test's server, asked in place; what it was asked.
struct PageServer(Arc<Mutex<Vec<String>>>);

impl clarity_ui::program_wire::Asks for PageServer {
    fn ask(&self, command: &str, args: serde_json::Map<String, Value>) -> Option<Value> {
        self.0.lock().unwrap().push(command.to_string());
        Some(super::browser_page::answer(command, &Value::Object(args)))
    }
}

/// A value set from the mirror on an input bound to a field of the model is
/// written to the window's data, told to the server as the person's edit
/// (`edit_event` over the wire) and kept, as in the browser: after the
/// handler filled another input and the person typed into this one.
#[test]
fn a_value_set_from_the_mirror_on_a_bound_input_is_kept() {
    let _turn = one_host();
    let served = json!({ "title": "Greeter", "windows": { "Main": super::browser_page::ACCESS_MAIN }, "apps": {}, "themes": {}, "model": super::browser_page::ACCESS_MODEL, "resources": [] });
    let files = clarity_ui::browser::files_from_json(&served, &|_| None);
    let asked: Arc<Mutex<Vec<String>>> = Arc::default();
    let program: Arc<dyn Program> = Arc::new(clarity_ui::program_wire::WireProgram::new(PageServer(asked.clone())));
    let (tell, said) = told();
    let mut host = BrowserHost::start(program, files, 900.0, 600.0, 1.0, false, tell);
    host.set_access(true);
    drawn(&mut host);
    let whole = updates(&said).remove(0);
    let answer = whole["nodes"].as_array().unwrap().iter().find(|n| n["part"] == json!("note")).expect("the bound input")["id"].as_str().unwrap().to_string();
    // As in the browser: the handler fills it, the person types into it, then a value is set.
    let greet = whole["nodes"].as_array().unwrap().iter().find(|n| n["part"] == json!("greet")).unwrap()["id"].as_str().unwrap().to_string();
    assert!(host.access_act(&greet, "click", None));
    drawn(&mut host);
    assert_eq!(host.step(&json!({ "do": "read" }))["data"]["answer"], json!("Hello from the server"));
    assert!(host.access_act(&answer, "focus", None));
    host.text("!");
    drawn(&mut host);
    let later = |host: &mut BrowserHost| {
        host.screen().wake_due(std::time::Instant::now() + std::time::Duration::from_secs(5));
        drawn(host);
    };
    later(&mut host);
    assert!(host.access_act(&answer, "setValue", Some("Grace")));
    drawn(&mut host);
    later(&mut host);
    let read = host.step(&json!({ "do": "read" }));
    assert_eq!(read["data"]["greeting"]["note"], json!("Grace"), "{read}");
    assert!(asked.lock().unwrap().contains(&"edit_event".to_string()), "{:?}", asked.lock().unwrap());
    let tree = host.step(&json!({ "do": "access" }));
    let node = tree["access"]["nodes"].as_array().unwrap().iter().find(|n| n["part"] == json!("note")).unwrap().clone();
    assert_eq!(node["value"], json!("Grace"));
}
