//! The core's parts and seams of G8b: sound and timer (C# SoundPartTests,
//! TimerPartTests, conformance/melody.json), work off the screen's thread,
//! and the file dialogs a host gives. Custom parts are in parts_custom.rs.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clarity_runtime::ui::Hear;
use clarity_ui::element::{Composition, Cx, Element, Node};
use clarity_ui::files::{ChosenFile, FileFilter, FilePicker};
use clarity_ui::later::Done;
use clarity_ui::parts::sound::{self, Playing, SoundOutput, SoundPart};
use clarity_ui::parts::timer::TimerPart;
use clarity_ui::testing::{part_named, window_of, Shot, TestHost};
use clarity_ui::{melodies, Screen, ThemeLook};
use serde_json::{json, Value};

// -- Melodies --------------------------------------------------------------------

#[test]
fn melodies_read_as_every_runtime_reads_them() {
    let cases: Value = serde_json::from_str(include_str!("../../../../conformance/melody.json")).unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let (plays, skips) = melodies::read(case["notes"].as_str().unwrap(), case["tempo"].as_i64().unwrap());
        let round = |v: f64, places: i32| (v * 10f64.powi(places)).round() / 10f64.powi(places);
        let got: Vec<Value> = plays.iter().map(|n| json!({ "hertz": n.hertz.map(|h| round(h, 2)), "seconds": round(n.seconds, 4) })).collect();
        let want: Vec<Value> = case["plays"].as_array().unwrap().iter().map(|p| json!({ "hertz": p["hertz"].as_f64(), "seconds": p["seconds"].as_f64().unwrap() })).collect();
        assert_eq!(want, got, "{}", case["notes"]);
        let want_skips: Vec<String> = case["skips"].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect();
        assert_eq!(want_skips, skips, "{}", case["notes"]);
    }
}

#[test]
fn tones_become_wav_data() {
    let wav = melodies::wav(&[melodies::Note { hertz: Some(440.0), seconds: 0.5 }, melodies::Note { hertz: None, seconds: 0.25 }]);
    assert_eq!(b"RIFF", &wav[0..4]);
    assert_eq!(b"WAVE", &wav[8..12]);
    let samples = (0.75 * melodies::SAMPLE_RATE as f64) as usize;
    assert_eq!(44 + samples * 2, wav.len());
    // A tone starts without a click: its first sample is silent.
    assert_eq!([0u8, 0u8], [wav[44], wav[45]]);
}

// -- Sound -----------------------------------------------------------------------

#[derive(Default)]
struct Speakers {
    asked: RefCell<Vec<String>>,
    stopped: Rc<RefCell<usize>>,
}

struct Sounding(Rc<RefCell<usize>>);

impl Playing for Sounding {
    fn stop(&mut self) {
        *self.0.borrow_mut() += 1;
    }
}

impl SoundOutput for Speakers {
    fn play(&self, file: Vec<u8>, volume: f32, repeat: bool) -> Box<dyn Playing> {
        self.asked.borrow_mut().push(format!("{} bytes at {volume} {}", file.len(), if repeat { "repeating" } else { "once" }));
        Box::new(Sounding(self.stopped.clone()))
    }
}

fn hear(host: &TestHost, part: &str, hear: &str, values: Vec<Value>) {
    host.hears.borrow_mut().push(Hear { part: part.to_string(), hear: hear.to_string(), values });
    host.change();
}

fn sound(screen: &Screen, name: &str) -> clarity_ui::element::ElementId {
    part_named(screen, name)
}

#[test]
fn a_sound_hears_what_code_tells_it_and_plays_through_the_output() {
    let speakers = Rc::new(Speakers::default());
    sound::set_output(speakers.clone());
    let host = TestHost::new();
    host.resources.borrow_mut().insert("chime.wav".to_string(), vec![1, 2, 3]);
    let mut screen = window_of(r#"<Sound name="chime" source="chime.wav" volume="50" repeat="true" />"#, host.clone());
    Shot::of(&mut screen, 200, 100, 1.0);
    hear(&host, "chime", "play", vec![]);
    hear(&host, "chime", "volume", vec![json!(20)]);
    hear(&host, "chime", "tone", vec![json!(440), json!(0.1)]);
    hear(&host, "chime", "melody", vec![json!("C4 E4"), json!(120)]);
    Shot::of(&mut screen, 200, 100, 1.0);
    let id = sound(&screen, "chime");
    let part = screen.element(id).unwrap().as_any().downcast_ref::<SoundPart>().unwrap();
    assert_eq!(vec!["play", "volume", "tone", "melody"], part.heard);
    assert_eq!(3, part.playing());
    let asked = speakers.asked.borrow().clone();
    assert_eq!("3 bytes at 0.5 repeating", asked[0]);
    assert!(asked[1].ends_with("at 0.2 once"), "{asked:?}");
    hear(&host, "chime", "stop", vec![]);
    Shot::of(&mut screen, 200, 100, 1.0);
    let part = screen.element(id).unwrap().as_any().downcast_ref::<SoundPart>().unwrap();
    assert_eq!(0, part.playing());
    assert_eq!(3, *speakers.stopped.borrow());
}

#[test]
fn a_source_the_program_does_not_carry_plays_nothing_and_a_sound_takes_no_place() {
    let speakers = Rc::new(Speakers::default());
    sound::set_output(speakers.clone());
    let host = TestHost::new();
    let mut screen = window_of(r#"<Sound name="chime" source="missing.wav" at="1,1" span="4" />"#, host.clone());
    hear(&host, "chime", "play", vec![]);
    Shot::of(&mut screen, 200, 100, 1.0);
    assert!(speakers.asked.borrow().is_empty());
    let id = sound(&screen, "chime");
    let e = screen.element(id).unwrap();
    assert!(!e.is_shown());
    assert_eq!(0.0, e.node().bounds.width);
}

#[test]
fn a_sound_stops_as_its_window_leaves_the_screen() {
    let speakers = Rc::new(Speakers::default());
    sound::set_output(speakers.clone());
    let host = TestHost::new();
    let mut screen = window_of(r#"<Sound name="chime" />"#, host.clone());
    hear(&host, "chime", "tone", vec![json!(440), json!(1)]);
    Shot::of(&mut screen, 200, 100, 1.0);
    assert_eq!(1, speakers.asked.borrow().len());
    screen.dispose();
    assert_eq!(1, *speakers.stopped.borrow());
}

// -- Timer -----------------------------------------------------------------------

fn clocked(screen: &mut Screen) -> Rc<RefCell<Instant>> {
    let now = Rc::new(RefCell::new(Instant::now()));
    let clock = now.clone();
    screen.now = Box::new(move || *clock.borrow());
    now
}

#[test]
fn a_timer_runs_its_tick_handler_every_so_many_seconds() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Timer name="clock" every="0.5" ontick="tick" />"#, host.clone());
    let now = clocked(&mut screen);
    // Its clock started as its window joined the screen, by the real clock; it is due at the latest a half second on.
    let start = *now.borrow() + Duration::from_secs(1);
    *now.borrow_mut() = start;
    screen.wake_due(start);
    assert_eq!(1, host.logged().iter().filter(|l| l.contains("tick tick")).count());
    let next = screen.next_wake().unwrap();
    assert_eq!(Duration::from_millis(500), next - start);
    *now.borrow_mut() = next;
    screen.wake_due(next);
    assert_eq!(2, host.logged().iter().filter(|l| l.contains("tick tick")).count());
}

#[test]
fn a_timers_seconds_are_one_unless_given_and_kept_between_a_tenth_and_a_day() {
    let host = TestHost::new();
    let screen = window_of(r#"<Timer name="a" /><Timer name="b" every="0.01" /><Timer name="c" every="999999" /><Timer name="d" every="2" />"#, host);
    let every = |name: &str| screen.element(part_named(&screen, name)).unwrap().as_any().downcast_ref::<TimerPart>().unwrap().every();
    assert_eq!(1.0, every("a"));
    assert_eq!(0.1, every("b"));
    assert_eq!(86400.0, every("c"));
    assert_eq!(2.0, every("d"));
}

#[test]
fn a_disabled_timer_is_quiet_until_it_is_enabled_again() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Timer name="clock" ontick="tick" />"#, host.clone());
    let now = clocked(&mut screen);
    host.set_state("clock", |s| s.enabled = Some(false));
    let at = *now.borrow() + Duration::from_secs(2);
    *now.borrow_mut() = at;
    screen.wake_due(at);
    assert!(host.logged().is_empty());
    host.set_state("clock", |s| s.enabled = Some(true));
    let at = screen.next_wake().unwrap();
    *now.borrow_mut() = at;
    screen.wake_due(at);
    assert_eq!(1, host.logged().len());
}

#[test]
fn a_timer_stops_as_its_window_leaves_the_screen() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Timer name="clock" ontick="tick" />"#, host.clone());
    let id = part_named(&screen, "clock");
    assert!(screen.element(id).unwrap().as_any().downcast_ref::<TimerPart>().unwrap().running());
    screen.dispose();
    assert!(!screen.element(id).unwrap().as_any().downcast_ref::<TimerPart>().unwrap().running());
    screen.wake_due(Instant::now() + Duration::from_secs(5));
    assert!(host.logged().is_empty());
}

// -- Work off the screen's thread, and file dialogs ---------------------------------

/// An element that asks a file dialog through its host's picker, off the screen's thread.
struct Asker {
    node: Node,
    picker: Arc<dyn FilePicker>,
    got: Rc<RefCell<Vec<String>>>,
}

impl Element for Asker {
    clarity_ui::element_node!();

    fn on_click(&mut self, cx: &mut Cx) {
        let picker = self.picker.clone();
        let id = self.node.id();
        cx.later(id, move || picker.open_file("Open", Some(&FileFilter { name: "Pictures".into(), patterns: vec!["*.png".into()] })));
    }

    fn on_later(&mut self, done: Done, _cx: &mut Cx) {
        if let Ok(chosen) = done.downcast::<Option<ChosenFile>>() {
            self.got.borrow_mut().push(chosen.map(|c| format!("{} {}", c.name, c.bytes.len())).unwrap_or_else(|| "none".to_string()));
        }
    }

    fn on_compose(&mut self, composing: Option<Composition>, _cx: &mut Cx) {
        self.got.borrow_mut().push(format!("composing {:?}", composing.map(|c| c.text)));
    }
}

/// A picker that answers at once with a file, and keeps what it was asked.
struct Answers(Mutex<Vec<String>>);

impl FilePicker for Answers {
    fn picks_folders(&self) -> bool {
        true
    }
    fn open_file(&self, title: &str, filter: Option<&FileFilter>) -> Option<ChosenFile> {
        self.0.lock().unwrap().push(format!("{title} {}", filter.map(|f| f.patterns.join(" ")).unwrap_or_default()));
        Some(ChosenFile { name: "cat.png".into(), bytes: vec![1, 2, 3] })
    }
    fn save_file(&self, _: &str, _: &str, _: &[u8]) -> Option<String> {
        None
    }
    fn choose_folder(&self, _: &str) -> Option<String> {
        None
    }
    fn show(&self, _: &str, _: &[u8]) {}
}

#[test]
fn work_off_the_screens_thread_comes_back_to_the_element_that_asked() {
    let picker = Arc::new(Answers(Mutex::new(Vec::new())));
    let got = Rc::new(RefCell::new(Vec::new()));
    let mut screen = Screen::new(Box::new(Asker { node: Node::new(), picker: picker.clone(), got: got.clone() }), ThemeLook::clarity(false));
    screen.resize(100.0, 100.0, 1.0);
    let woken = Arc::new(Mutex::new(0));
    let count = woken.clone();
    screen.set_waker(Arc::new(move || *count.lock().unwrap() += 1));
    screen.click(10.0, 10.0);
    screen.finish_later();
    assert_eq!(vec!["cat.png 3"], got.borrow().clone());
    assert_eq!(vec!["Open *.png"], picker.0.lock().unwrap().clone());
    assert_eq!(1, *woken.lock().unwrap());
    assert_eq!(0, screen.later_running());
}

#[test]
fn a_host_without_file_dialogs_picks_nothing_and_a_files_name_is_made_safe() {
    // The dialogs are Clarity.UI's own: a host that set none has NoFilePicker.
    let picker = clarity_ui::files::system();
    assert!(!picker.picks_folders());
    assert_eq!(None, picker.open_file("Open", None));
    assert_eq!("b.txt", clarity_ui::files::safe_name("../a/b.txt"));
    assert_eq!("c_d.txt", clarity_ui::files::safe_name("C:\\x\\c:d.txt"));
    assert_eq!("file", clarity_ui::files::safe_name(".."));
}

#[test]
fn an_input_methods_composing_goes_to_the_element_with_the_focus() {
    let got = Rc::new(RefCell::new(Vec::new()));
    let picker: Arc<dyn FilePicker> = Arc::new(clarity_ui::files::NoFilePicker);
    let mut screen = Screen::new(Box::new(Asker { node: Node::new(), picker, got: got.clone() }), ThemeLook::clarity(false));
    screen.resize(100.0, 100.0, 1.0);
    let root = screen.root().node().id();
    screen.focus(Some(root));
    screen.compose(Some(Composition { text: "ni".into(), marked: None }));
    screen.compose(None);
    assert_eq!(vec!["composing Some(\"ni\")", "composing None"], got.borrow().clone());
}

// -- The desktop's file dialogs and sound, without showing any -------------------------

#[cfg(feature = "desktop")]
mod desktop {
    use clarity_ui::desktop::files::{answer, apple_script, commands, extensions, portal, show_command, Ask, Kind, Os};
    use clarity_ui::files::FileFilter;

    fn ask(kind: Kind) -> Ask {
        Ask { kind, title: "Pick \"one\"".into(), name: Some("report.pdf".into()), filter: Some(FileFilter { name: "Pictures".into(), patterns: vec!["*.png".into(), "*.jpg".into()] }) }
    }

    #[test]
    fn each_system_has_its_own_dialog_programs() {
        let linux = commands(Os::Linux, &ask(Kind::Open));
        assert_eq!(vec!["zenity", "kdialog"], linux.iter().map(|p| p.program.as_str()).collect::<Vec<_>>());
        assert!(linux[0].args.contains(&"--file-filter=Pictures | *.png *.jpg".to_string()));
        assert!(commands(Os::Linux, &ask(Kind::Save))[0].args.contains(&"--filename=report.pdf".to_string()));
        assert!(commands(Os::Linux, &ask(Kind::Folder))[0].args.contains(&"--directory".to_string()));
        assert!(commands(Os::Windows, &ask(Kind::Open)).is_empty());
        assert_eq!(r#"POSIX path of (choose file with prompt "Pick \"one\"" of type {"png", "jpg"})"#, apple_script(&ask(Kind::Open)));
        assert_eq!(r#"POSIX path of (choose folder with prompt "Pick \"one\"")"#, apple_script(&ask(Kind::Folder)));
        assert!(extensions(Some(&FileFilter { name: "All".into(), patterns: vec!["*".into()] })).is_empty());
        assert_eq!("xdg-open", show_command(Os::Linux, "/tmp/a.pdf").program);
        assert_eq!("open", show_command(Os::MacOs, "/tmp/a.pdf").program);
    }

    #[test]
    fn a_dialog_programs_answer_is_its_first_line() {
        assert_eq!(Some("/home/p/a.png".to_string()), answer("/home/p/a.png\n"));
        assert_eq!(Some("/home/p/folder".to_string()), answer("/home/p/folder/\r\n"));
        assert_eq!(None, answer("\n"));
        assert_eq!("/home/p/my file.png", portal::unescape("/home/p/my%20file.png"));
    }

    #[test]
    fn the_portals_messages_are_written_and_read_as_d_bus_writes_them() {
        use portal::{method_call, Dbus, Reader};
        let call = method_call(3, "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.FileChooser", "OpenFile", "ssa{sv}", &[
            Dbus::Str(String::new()),
            Dbus::Str("Open".into()),
            Dbus::Array(vec![Dbus::Entry(Box::new(Dbus::Str("modal".into())), Box::new(Dbus::Variant("b".into(), Box::new(Dbus::Bool(true)))))]),
        ]);
        assert_eq!([b'l', 1, 0, 1], call[0..4]);
        let body = u32::from_le_bytes(call[4..8].try_into().unwrap()) as usize;
        let fields = u32::from_le_bytes(call[12..16].try_into().unwrap()) as usize;
        let start = (16 + fields).div_ceil(8) * 8;
        assert_eq!(start + body, call.len());
        let mut reader = Reader::new(&call, start);
        assert_eq!(Some(Dbus::Str(String::new())), reader.value("s"));
        assert_eq!(Some(Dbus::Str("Open".into())), reader.value("s"));
        let options = reader.value("a{sv}").unwrap();
        assert_eq!(Dbus::Array(vec![Dbus::Entry(Box::new(Dbus::Str("modal".into())), Box::new(Dbus::Variant("b".into(), Box::new(Dbus::Bool(true)))))]), options);
    }

    #[test]
    fn wav_data_is_read_as_its_samples() {
        let wav = clarity_ui::melodies::wav(&[clarity_ui::melodies::Note { hertz: Some(440.0), seconds: 0.01 }]);
        let (channels, rate, samples) = clarity_ui::desktop::sound::read_wav(&wav).unwrap();
        assert_eq!((1, 44100, 441), (channels, rate, samples.len()));
        assert!(samples.iter().all(|s| (-1.0..=1.0).contains(s)));
        assert_eq!(None, clarity_ui::desktop::sound::read_wav(b"ID3 not a wav"));
    }
}

// -- Input methods in a text box --------------------------------------------------------

#[test]
fn a_text_box_shows_what_an_input_method_composes_at_its_caret_until_it_commits() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Textbox name="t" at="1,1" span="6" />"#, host.clone());
    let mut before = Shot::of(&mut screen, 400, 80, 1.0);
    let id = part_named(&screen, "t");
    screen.focus(Some(id));
    screen.text_typed("ab");
    let plain = Shot::of(&mut screen, 400, 80, 1.0).png();
    screen.compose(Some(Composition { text: "ni".into(), marked: Some((0, 2)) }));
    let mut composing = Shot::of(&mut screen, 400, 80, 1.0);
    assert_ne!(plain, composing.png());
    let caret = screen.caret_area().expect("the caret's place");
    assert!(caret.x > screen.element(id).unwrap().node().bounds.x);
    // Committed, it is typed text; the composing is gone.
    screen.compose(None);
    screen.text_typed("\u{4f60}");
    let editor = screen.element(id).unwrap().as_any().downcast_ref::<clarity_ui::parts::text_box::TextBoxPart>().unwrap().editor();
    assert_eq!("ab\u{4f60}", editor.text());
    assert!(editor.composing().is_none());
    let _ = before.png();
    if let Ok(folder) = std::env::var("CLARITY_SHOTS") {
        composing.save(std::path::Path::new(&folder).join("core-ime-composing.png")).unwrap();
    }
}

/// C# TimerPartTests.A_tick_whose_command_cannot_run_is_left_out: the host
/// says a handler cannot run now (a command's CanExecute), and the tick is skipped.
#[test]
fn a_tick_whose_command_cannot_run_is_left_out() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Timer name="clock" ontick="clockTicked" />"#, host.clone());
    let now = clocked(&mut screen);
    host.cannot_run.borrow_mut().insert("clockTicked".into());
    let at = *now.borrow() + Duration::from_secs(2);
    *now.borrow_mut() = at;
    screen.wake_due(at);
    assert!(host.logged().is_empty());
    host.cannot_run.borrow_mut().clear();
    let at = screen.next_wake().unwrap();
    *now.borrow_mut() = at;
    screen.wake_due(at);
    assert_eq!(1, host.logged().len());
}

/// A disposed screen joins its tree no more: a later event leaves a closed
/// window's clock stopped (C# FrameTicksTests, note W2).
#[test]
fn a_disposed_screen_does_not_start_its_clock_again() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Timer name="clock" ontick="tick" />"#, host.clone());
    let id = part_named(&screen, "clock");
    screen.dispose();
    screen.key_pressed(clarity_ui::KeyInput::plain(clarity_ui::Key::Tab));
    screen.wake_due(Instant::now() + Duration::from_secs(5));
    assert!(!screen.element(id).unwrap().as_any().downcast_ref::<TimerPart>().unwrap().running());
    assert_eq!(None, screen.next_wake());
    assert!(host.logged().is_empty());
}
