//! The file part (U1c): a record's file name or that it has none, a chosen
//! file and a removed one as changes, the bytes of a stored file fetched only
//! to open it or save a copy, and its buttons by what may be done; with the
//! names a chosen file is kept under (conformance/record-files.json). A
//! picker of the test's own stands for the system's dialogs.

use super::inputs_test_picker as picker;

use std::rc::Rc;

use clarity_runtime::picture::Picture;
use clarity_runtime::record_file::{name_of, RecordFile};
use clarity_ui::input::PointerButton;
use clarity_ui::locale;
use clarity_ui::parts::containers::container::ContainerButton;
use clarity_ui::parts::inputs::file::{saved_where, FilePart};
use clarity_ui::parts::inputs::path::PathPart;
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
    // The person hears where it really went, among the window's messages.
    let said = locale::fill(&locale::translate("fileCopySavedAt", "A copy is saved as {place}"), "place", "/home/ada/Downloads/copy");
    assert_eq!(vec![format!("Info: {said}")], host.said.borrow().clone());
    assert!(host.logged().is_empty());
}

#[test]
fn a_saved_copy_says_where_it_went_the_chosen_place_or_the_downloads_folder() {
    let (mut screen, host, picker) = make(Some(RecordFile { name: "manual.pdf".into(), content: Picture::New(vec![1]) }), MARKUP);
    picker.save_to("/srv/elsewhere/manual.pdf");
    click(&mut screen, FilePart::save_button);
    let said = host.said.borrow().clone();
    assert_eq!(1, said.len());
    assert!(said[0].contains("/srv/elsewhere/manual.pdf"), "{said:?}");
    // A browser's download has only the name: it is in the Downloads folder.
    assert_eq!(locale::translate("fileCopySaved", "A copy is in your Downloads folder"), saved_where("manual.pdf"));
}

#[test]
fn its_buttons_read_choose_open_save_a_copy_remove_and_a_long_name_ends_in_an_ellipsis() {
    let long = format!("{}.pdf", "n".repeat(300));
    let (mut screen, _, _) = make(Some(RecordFile { name: long, content: Picture::New(vec![1]) }), MARKUP);
    Shot::of(&mut screen, 800, 60, 1.0);
    let order: Vec<fn(&FilePart) -> &ContainerButton> = vec![FilePart::choose_button, FilePart::open_button, FilePart::save_button, FilePart::remove_button];
    let places: Vec<_> = order.iter().map(|b| b(part(&screen)).node().bounds).collect();
    for i in 1..places.len() {
        assert!(places[i - 1].right() <= places[i].x, "button {i} stands after the one before");
    }
    let ids: Vec<_> = order.iter().map(|b| b(part(&screen)).node().id()).collect();
    let tabbed: Vec<_> = screen.tab_order().into_iter().filter(|id| ids.contains(id)).collect();
    assert_eq!(ids, tabbed);
    // The name the part draws is cut off inside its place, its end shown as dots.
    let drawn = part(&screen).drawn_name();
    let shown = part(&screen).shown();
    assert!(drawn.ends_with('\u{2026}'));
    assert!(drawn.chars().count() < shown.chars().count());
    assert!(shown.starts_with(drawn.trim_end_matches('\u{2026}')));
    let look = screen.look().clone();
    let width = screen.fonts().measure(&drawn, "text", look.size("text"), false);
    assert!(width <= part(&screen).name_box().width - 2.0 * look.padding());
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

/// `extensions` and `start folder` (docs/ledger/files-pictures.md): the
/// system's dialog offers only those files, named by their patterns, and
/// opens in the start folder, a relative one inside the program's folder;
/// a copy is saved from there too. Without them, any file, where the system opens it.
#[test]
fn its_dialog_offers_its_extensions_and_opens_in_its_start_folder() {
    let (mut screen, _, picker) = make(None, r#"<File at="1,1" span="12" name="doc" bind="task.attachment" extensions="*.pdf *.docx" startFolder="papers" />"#);
    click(&mut screen, FilePart::choose_button);
    let filter = picker.filter().expect("a filter");
    assert_eq!(filter.patterns, ["*.pdf", "*.docx"]);
    assert_eq!(filter.name, "*.pdf *.docx");
    let papers = std::env::current_dir().unwrap().join("papers").to_string_lossy().into_owned();
    assert_eq!(picker.starts(), [Some(papers.clone())]);
    let (mut screen, _, picker) = make(Some(RecordFile { name: "manual.pdf".into(), content: Picture::New(vec![1]) }), r#"<File at="1,1" span="12" name="doc" bind="task.attachment" startFolder="/srv/papers" />"#);
    click(&mut screen, FilePart::save_button);
    assert_eq!(picker.starts(), [Some("/srv/papers".to_string())]);
    let (mut screen, _, picker) = make(None, MARKUP);
    click(&mut screen, FilePart::choose_button);
    assert_eq!((picker.filter(), picker.starts()), (None, vec![None]));
}

// -- A file part bound to a `file` field: a file on the disk, by its path ------------

fn make_path(markup: &str) -> (Screen, Rc<InputsHost>, std::sync::Arc<TestPicker>) {
    let picker = TestPicker::installed();
    let host = InputsHost::new();
    host.set("movie", json!({}));
    host.set("movie.video", json!("/home/ada/videos/old.mp4"));
    host.types.borrow_mut().insert("movie.video".into(), "file".into());
    let mut screen = window_of(markup, host.clone());
    screen.resize(800.0, 60.0, 1.0);
    Shot::of(&mut screen, 800, 60, 1.0);
    (screen, host, picker)
}

fn path_part(screen: &Screen) -> &PathPart {
    screen.element(part_named(screen, "video")).and_then(|e| e.as_any().downcast_ref::<PathPart>()).expect("a file part bound to a file field shows its path")
}

fn click_path_button(screen: &mut Screen) {
    let b = path_part(screen).choose_button().bounds;
    let (x, y) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
    screen.finish_later();
}

/// It shows the path, typed or chosen with the system's open dialog of its
/// extensions; the path chosen is written, and nothing is read.
#[test]
fn a_file_field_shows_its_path_and_chooses_a_file_on_the_disk() {
    let (mut screen, host, picker) = make_path(r#"<File at="1,1" span="12" name="video" bind="movie.video" extensions="*.mp4" onchange="moved" />"#);
    assert_eq!("/home/ada/videos/old.mp4", path_part(&screen).editor().text());
    picker.next_file_at("/home/ada/videos/new.mp4");
    click_path_button(&mut screen);
    assert_eq!(Some(json!("/home/ada/videos/new.mp4")), host.read("movie.video"));
    assert_eq!(vec![format!("open {}", locale::translate("chooseFile", "Choose a file"))], picker.asked());
    assert_eq!(picker.filter().map(|f| f.patterns), Some(vec!["*.mp4".to_string()]));
    // It opens beside the file it holds.
    assert_eq!(picker.starts(), [Some("/home/ada/videos".to_string())]);
    assert!(host.logged().iter().any(|l| l == "run video change moved"), "{:?}", host.logged());
}

/// With `new file` it asks the save dialog, suggesting the file it holds, so a file that is not there yet can be named.
#[test]
fn with_new_file_it_names_a_file_that_is_not_there_yet() {
    let (mut screen, host, picker) = make_path(r#"<File at="1,1" span="12" name="video" bind="movie.video" newFile="true" />"#);
    picker.save_to("/home/ada/videos/cut.mp4");
    click_path_button(&mut screen);
    assert_eq!(Some(json!("/home/ada/videos/cut.mp4")), host.read("movie.video"));
    assert_eq!(vec![format!("save {} old.mp4", locale::translate("chooseFile", "Choose a file"))], picker.asked());
    assert!(picker.saved().is_empty(), "choosing where writes nothing");
}

/// A file of a type the part does not take is refused in Clarity's own
/// words and not written: chosen or typed, a hidden one only with `hidden`.
#[test]
fn a_file_of_another_type_is_refused() {
    let (mut screen, host, picker) = make_path(r#"<File at="1,1" span="12" name="video" bind="movie.video" extensions=".mp4, .webm" onchange="moved" />"#);
    picker.next_file_at("/home/ada/notes.txt");
    click_path_button(&mut screen);
    assert_eq!(Some(json!("/home/ada/videos/old.mp4")), host.read("movie.video"), "not written");
    assert_eq!(host.said.borrow().clone(), ["Error: video takes *.mp4 *.webm files, not notes.txt"]);
    let (mut screen, host, picker) = make_path(r#"<File at="1,1" span="12" name="video" bind="movie.video" extensions="*.*" />"#);
    picker.next_file_at("/home/ada/.env");
    click_path_button(&mut screen);
    assert_eq!(Some(json!("/home/ada/videos/old.mp4")), host.read("movie.video"), "a hidden one only with hidden");
    let (mut screen, host, picker) = make_path(r#"<File at="1,1" span="12" name="video" bind="movie.video" extensions="*.*" hidden="true" />"#);
    picker.next_file_at("/home/ada/.env");
    click_path_button(&mut screen);
    assert_eq!(Some(json!("/home/ada/.env")), host.read("movie.video"));
    // A record's file too.
    let (mut screen, host, picker) = make(None, r#"<File at="1,1" span="12" name="doc" bind="task.attachment" extensions=".pdf" />"#);
    picker.next_file("budget.xlsx", vec![1]);
    click(&mut screen, FilePart::choose_button);
    assert_eq!(Some(Value::Null), host.read("task.attachment"));
    assert_eq!(host.said.borrow().clone(), ["Error: doc takes *.pdf files, not budget.xlsx"]);
}

/// A field's own extensions hold for a part that names none.
#[test]
fn the_field_s_extensions_hold_where_the_part_names_none() {
    let picker = TestPicker::installed();
    let host = InputsHost::new();
    host.set("movie", json!({}));
    host.set("movie.video", json!(""));
    host.types.borrow_mut().insert("movie.video".into(), "file".into());
    host.facts("movie.video", clarity_ui::parts::FieldFacts { label: "Video".into(), extensions: vec!["*.mp4".into()], ..Default::default() });
    let mut screen = window_of(r#"<File at="1,1" span="12" name="video" bind="movie.video" />"#, host.clone());
    screen.resize(800.0, 60.0, 1.0);
    Shot::of(&mut screen, 800, 60, 1.0);
    picker.next_file_at("/srv/a.avi");
    click_path_button(&mut screen);
    assert_eq!(Some(json!("")), host.read("movie.video"));
    assert_eq!(host.said.borrow().clone(), ["Error: Video takes *.mp4 files, not a.avi"]);
    assert_eq!(picker.filter().map(|f| f.patterns), Some(vec!["*.mp4".to_string()]), "its dialog offers them");
}

/// In a web app the browser gives the picked file's bytes and no path: the
/// part keeps it in its store on the server and writes the path it got
/// there (docs/ledger/files-pictures.md, step 5).
#[test]
fn in_a_browser_a_picked_file_is_kept_in_the_part_s_store() {
    let (mut screen, host, picker) = make_path(r#"<File at="1,1" span="12" name="video" bind="movie.video" extensions=".mp4" store="uploads" />"#);
    picker.no_folders();
    picker.next_file("clip.mp4", b"film".to_vec());
    click_path_button(&mut screen);
    assert_eq!(host.uploaded.borrow().clone(), [("uploads".to_string(), "clip.mp4".to_string(), b"film".to_vec())]);
    assert_eq!(Some(json!("/srv/uploads/clip.mp4")), host.read("movie.video"));
    // One of another type is refused before it is sent.
    picker.next_file("notes.txt", b"x".to_vec());
    click_path_button(&mut screen);
    assert_eq!(host.uploaded.borrow().len(), 1);
}
