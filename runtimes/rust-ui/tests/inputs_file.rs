//! The file part (U1c): a record's file name or that it has none, a chosen
//! file and a removed one as changes, the bytes of a stored file fetched only
//! to open it or save a copy, and its buttons by what may be done; with the
//! names a chosen file is kept under (conformance/record-files.json). A
//! picker of the test's own stands for the system's dialogs.

#[path = "inputs_test_picker/mod.rs"]
mod picker;

use std::rc::Rc;

use clarity_runtime::picture::Picture;
use clarity_runtime::record_file::{name_of, RecordFile};
use clarity_ui::input::PointerButton;
use clarity_ui::locale;
use clarity_ui::parts::containers::container::ContainerButton;
use clarity_ui::parts::inputs::file::FilePart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::{Element, Screen};
use picker::TestPicker;
use serde_json::{json, Value};

const MARKUP: &str = r#"<File at="1,1" span="12" name="doc" bind="task.attachment" onchange="attached" />"#;

fn make(file: Option<RecordFile>, markup: &str) -> (Screen, Rc<InputsHost>, std::sync::Arc<TestPicker>) {
    let picker = TestPicker::installed();
    let host = InputsHost::new();
    host.set("task", json!({}));
    host.set("task.attachment", file.map(|f| serde_json::to_value(f).unwrap()).unwrap_or(Value::Null));
    let mut screen = window_of(markup, host.clone());
    screen.resize(800.0, 60.0, 1.0);
    Shot::of(&mut screen, 800, 60, 1.0);
    (screen, host, picker)
}

fn stored(name: &str) -> RecordFile {
    RecordFile { name: name.to_string(), content: Picture::Stored }
}

fn part(screen: &Screen) -> &FilePart {
    screen.element(part_named(screen, "doc")).and_then(|e| e.as_any().downcast_ref::<FilePart>()).expect("a file part")
}

/// Draws again (the buttons that show take their places), then clicks the middle of a button and finishes what it asked.
fn click(screen: &mut Screen, button: fn(&FilePart) -> &ContainerButton) {
    Shot::of(screen, 800, 60, 1.0);
    let b = button(part(screen)).node().bounds;
    let (x, y) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
    screen.finish_later();
}

fn set(screen: &mut Screen, host: &InputsHost, file: Option<RecordFile>) {
    host.set("task.attachment", file.map(|f| serde_json::to_value(f).unwrap()).unwrap_or(Value::Null));
    Shot::of(screen, 800, 60, 1.0);
}

#[test]
fn it_shows_the_files_name_or_that_there_is_none() {
    let (mut screen, host, _) = make(None, MARKUP);
    assert_eq!(locale::translate("noFile", "No file"), part(&screen).shown());
    assert!(part(&screen).is_empty());
    set(&mut screen, &host, Some(stored("manual.pdf")));
    assert_eq!("manual.pdf", part(&screen).shown());
    assert!(!part(&screen).is_empty());
}

#[test]
fn open_and_save_show_only_with_a_file_and_remove_is_usable_only_then() {
    let (mut screen, host, _) = make(None, MARKUP);
    assert!(!part(&screen).open_button().is_shown());
    assert!(!part(&screen).save_button().is_shown());
    assert!(part(&screen).choose_button().is_shown());
    assert!(!part(&screen).remove_button().focusable());
    set(&mut screen, &host, Some(stored("manual.pdf")));
    assert!(part(&screen).open_button().is_shown());
    assert!(part(&screen).save_button().is_shown());
    assert!(part(&screen).remove_button().focusable());
    // Clarity's own words name them.
    assert_eq!(locale::translate("chooseFile", "Choose a file"), part(&screen).choose_button().text());
    assert_eq!(locale::translate("openFile", "Open"), part(&screen).open_button().text());
    assert_eq!(locale::translate("saveFileCopy", "Save a copy"), part(&screen).save_button().text());
    assert_eq!(locale::translate("removeFile", "Remove the file"), part(&screen).remove_button().text());
}

#[test]
fn choosing_and_removing_are_changes() {
    let (mut screen, host, picker) = make(None, MARKUP);
    picker.next_file("C:\\docs\\manual.pdf", vec![1, 2, 3]);
    click(&mut screen, FilePart::choose_button);
    let file: RecordFile = serde_json::from_value(host.read("task.attachment").unwrap()).unwrap();
    assert_eq!("manual.pdf", file.name);
    assert_eq!(Picture::New(vec![1, 2, 3]), file.content);
    assert_eq!(vec![format!("write doc task.attachment {}", serde_json::to_value(&file).unwrap()), "run doc change attached".to_string()], host.logged());
    host.log.borrow_mut().clear();
    click(&mut screen, FilePart::remove_button);
    assert_eq!(Some(Value::Null), host.read("task.attachment"));
    assert_eq!(vec!["write doc task.attachment null", "run doc change attached"], host.logged());
    host.log.borrow_mut().clear();
    // Removing when there is none is no change, nor is a choice the person left.
    part(&screen).remove();
    click(&mut screen, FilePart::choose_button);
    assert!(host.logged().is_empty());
    assert_eq!(2, picker.asked().len());
}

#[test]
fn a_read_only_file_part_neither_chooses_nor_removes() {
    let (screen, host, _) = make(Some(stored("manual.pdf")), r#"<File at="1,1" span="12" name="doc" bind="task.attachment" readonly="true" />"#);
    assert!(!part(&screen).choose_button().is_shown());
    assert!(!part(&screen).remove_button().is_shown());
    assert!(part(&screen).open_button().is_shown());
    part(&screen).take("other.pdf", vec![4]);
    part(&screen).remove();
    assert!(host.logged().is_empty());
    // Code makes one read-only through its state.
    let (screen, host, _) = make(None, MARKUP);
    assert!(part(&screen).choose_button().is_shown());
    host.set_state("doc", |s| s.readonly = Some(true));
    assert!(!part(&screen).choose_button().is_shown());
}

#[test]
fn a_chosen_file_is_kept_under_the_last_part_of_its_name() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/record-files.json");
    let cases: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut run = 0;
    for case in cases["storePaths"].as_array().unwrap() {
        let (screen, host, _) = make(None, MARKUP);
        part(&screen).take(case["chosen"].as_str().unwrap(), case["text"].as_str().unwrap().as_bytes().to_vec());
        let file: RecordFile = serde_json::from_value(host.read("task.attachment").unwrap()).unwrap();
        assert_eq!(name_of(case["path"].as_str().unwrap()), file.name);
        run += 1;
    }
    assert!(run >= 6);
}

#[test]
fn a_stored_file_is_fetched_only_when_asked_for() {
    let (mut screen, host, _) = make(Some(stored("manual.pdf")), MARKUP);
    host.files.borrow_mut().insert("task.attachment".into(), vec![9]);
    host.files.borrow_mut().insert("ab/c.pdf".into(), vec![7]);
    assert_eq!(Some(vec![9]), part(&screen).bytes());
    set(&mut screen, &host, Some(RecordFile { name: "c.pdf".into(), content: Picture::InStore("ab/c.pdf".into()) }));
    assert_eq!(Some(vec![7]), part(&screen).bytes());
    set(&mut screen, &host, Some(RecordFile { name: "new.pdf".into(), content: Picture::New(vec![5]) }));
    assert_eq!(Some(vec![5]), part(&screen).bytes());
}

#[test]
fn opening_and_saving_a_copy_are_no_changes() {
    let (mut screen, host, picker) = make(Some(stored("manual.pdf")), MARKUP);
    host.files.borrow_mut().insert("task.attachment".into(), vec![9]);
    click(&mut screen, FilePart::open_button);
    assert_eq!(vec![("manual.pdf".to_string(), vec![9])], picker.shown());
    click(&mut screen, FilePart::save_button);
    assert_eq!(vec![("manual.pdf".to_string(), vec![9])], picker.saved());
    assert_eq!(Some("/home/ada/Downloads/copy"), part(&screen).saved());
    // The person hears where it went, among the window's messages.
    assert_eq!(vec![format!("Info: {}", locale::translate("fileCopySaved", "A copy is in your Downloads folder"))], host.said.borrow().clone());
    assert!(host.logged().is_empty());
}

#[test]
fn a_fetch_that_fails_opens_nothing() {
    let (mut screen, _, picker) = make(Some(stored("manual.pdf")), MARKUP);
    click(&mut screen, FilePart::open_button);
    assert!(picker.shown().is_empty());
}

#[test]
fn it_draws_with_and_without_a_file() {
    let (mut screen, host, _) = make(None, MARKUP);
    let mut shot = Shot::of(&mut screen, 800, 60, 1.0);
    keep(&mut shot, "inputs-file-none.png");
    set(&mut screen, &host, Some(stored("manual.pdf")));
    let mut shot = Shot::of(&mut screen, 800, 60, 1.0);
    keep(&mut shot, "inputs-file.png");
    assert_eq!(800, shot.width());
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}
