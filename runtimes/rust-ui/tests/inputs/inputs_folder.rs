//! The folder part (X2): a folder setting's path, typed (taken on leaving or
//! Enter, only a differing text a change) or chosen with the system's folder
//! picker through its button, named in Clarity's words; read-only, and where
//! nothing picks a folder, there is no button.

use super::inputs_test_picker as picker;

use std::rc::Rc;

use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::locale;
use clarity_ui::parts::inputs::folder::{choose_name, type_name, FolderPart};
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::text_box::InputIcon;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::{Element, Screen};
use picker::TestPicker;
use serde_json::json;

const MARKUP: &str = r#"<Folder at="1,1" span="12" name="export" bind="settings.exportFolder" onchange="moved" onenter="came" onleave="went" />"#;

fn make(markup: &str) -> (Screen, Rc<InputsHost>, std::sync::Arc<TestPicker>) {
    let picker = TestPicker::installed();
    let host = InputsHost::new();
    host.set("settings.exportFolder", json!("/mnt/photos"));
    let mut screen = window_of(markup, host.clone());
    screen.resize(800.0, 60.0, 1.0);
    Shot::of(&mut screen, 800, 60, 1.0);
    (screen, host, picker)
}

fn part(screen: &Screen) -> &FolderPart {
    screen.element(part_named(screen, "export")).and_then(|e| e.as_any().downcast_ref::<FolderPart>()).expect("a folder part")
}

fn with_part<R>(screen: &mut Screen, act: impl FnOnce(&mut FolderPart) -> R) -> R {
    let id = part_named(screen, "export");
    screen.act(id, |e, _| act(e.as_any_mut().downcast_mut::<FolderPart>().unwrap())).unwrap()
}

fn type_in(screen: &mut Screen, text: &str) {
    let id = part_named(screen, "export");
    screen.focus(Some(id));
    with_part(screen, |p| p.editor_mut().select_all());
    screen.key_pressed(KeyInput::plain(Key::Backspace));
    screen.text_typed(text);
}

fn click_button(screen: &mut Screen) {
    let b = part(screen).choose_button().bounds;
    let (x, y) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
    screen.finish_later();
}

#[test]
fn a_typed_folder_is_taken_on_leaving_or_enter_when_it_differs() {
    let (mut screen, host, _) = make(MARKUP);
    assert_eq!("/mnt/photos", part(&screen).editor().text());
    type_in(&mut screen, "/mnt/photos");
    screen.focus(None);
    assert_eq!(vec!["run export enter came", "run export leave went"], host.logged());
    host.log.borrow_mut().clear();
    type_in(&mut screen, "/srv/export");
    // Typing alone writes nothing: only the focus coming in is told.
    assert_eq!(vec!["run export enter came"], host.logged());
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(Some(json!("/srv/export")), host.read("settings.exportFolder"));
    assert_eq!(vec!["run export enter came", r#"write export settings.exportFolder "/srv/export""#, "run export change moved"], host.logged());
    host.log.borrow_mut().clear();
    screen.focus(None);
    assert_eq!(vec!["run export leave went"], host.logged());
}

#[test]
fn a_chosen_folder_is_taken_at_once() {
    let (mut screen, host, picker) = make(MARKUP);
    picker.next_folder("/home/ada/Pictures");
    click_button(&mut screen);
    assert_eq!(Some(json!("/home/ada/Pictures")), host.read("settings.exportFolder"));
    assert_eq!("/home/ada/Pictures", part(&screen).editor().text());
    assert_eq!(vec![format!("folder {}", locale::translate("chooseFolder", "Choose a folder"))], picker.asked());
    let changes = |host: &InputsHost| host.logged().into_iter().filter(|l| !l.contains(" enter ") && !l.contains(" leave ")).collect::<Vec<_>>();
    assert_eq!(vec![r#"write export settings.exportFolder "/home/ada/Pictures""#, "run export change moved"], changes(&host));
    // The same folder again is no change.
    with_part(&mut screen, |p| p.chosen("/home/ada/Pictures"));
    assert_eq!(2, changes(&host).len());
    // Choosing none leaves it.
    click_button(&mut screen);
    assert_eq!(2, changes(&host).len());
}

/// It says "Choose a folder" when the pointer rests on it, and shows three dots.
#[test]
fn its_button_is_named_in_clarity_words() {
    let (screen, _, _) = make(MARKUP);
    let words = locale::translate("chooseFolder", "Choose a folder");
    assert_eq!(words, choose_name());
    let button = part(&screen).choose_button();
    assert_eq!(words, button.name);
    let b = button.bounds;
    assert_eq!(Some(words.clone()), button.tooltip_at(b.x + 2.0, b.y + 2.0));
    assert_eq!(Some(words), part(&screen).tooltip_at(b.x + 2.0, b.y + 2.0));
    assert_eq!(InputIcon::More, button.icon);
}

#[test]
fn its_button_sits_inside_the_box_and_is_no_tab_stop() {
    let (screen, _, _) = make(MARKUP);
    let p = part(&screen);
    let boxed = p.editor().node().bounds;
    let button = p.choose_button().bounds;
    assert_eq!(p.node().bounds, boxed);
    assert!(button.x > boxed.x && button.right() <= boxed.right() && button.y >= boxed.y && button.bottom() <= boxed.bottom());
    assert!(!p.choose_button().focusable());
    // Only the box is a Tab stop.
    assert_eq!(vec![part_named(&screen, "export")], screen.tab_order());
    // The typed path stops before the button.
    assert!(p.editor().right_inset >= button.width);
}

#[test]
fn a_read_only_folder_takes_no_chosen_folder_and_has_no_button() {
    let (mut screen, host, _) = make(r#"<Folder at="1,1" span="12" name="export" bind="settings.exportFolder" readonly="true" />"#);
    assert!(!part(&screen).choose_button().is_shown());
    with_part(&mut screen, |p| p.chosen("/elsewhere"));
    assert_eq!(Some(json!("/mnt/photos")), host.read("settings.exportFolder"));
    assert!(host.logged().is_empty());
}

#[test]
fn where_nothing_picks_a_folder_it_is_only_typed() {
    let (screen, _, picker) = make(MARKUP);
    picker.no_folders();
    assert!(!part(&screen).choose_button().is_shown());
}

/// In a browser nothing picks a folder: the program and its folders are on
/// the server, so the part says in Clarity's words that a path there is
/// typed; a placeholder of its own goes first.
#[test]
fn where_nothing_picks_a_folder_it_says_a_path_on_the_server_is_typed() {
    let (mut screen, _, picker) = make(r#"<Folder at="1,1" span="12" name="export" />"#);
    assert_eq!(None, part(&screen).editor().placeholder, "a picker: the button says it");
    picker.no_folders();
    screen.resize(810.0, 60.0, 1.0);
    Shot::of(&mut screen, 810, 60, 1.0);
    let words = locale::translate("typeServerFolder", "Type the path of a folder on the server");
    assert_eq!(words, type_name());
    assert_eq!(Some(words), part(&screen).editor().placeholder);
    let (mut own, _, picker) = make(r#"<Folder at="1,1" span="12" name="export" placeholder="Exports" />"#);
    picker.no_folders();
    own.resize(810.0, 60.0, 1.0);
    Shot::of(&mut own, 810, 60, 1.0);
    assert_eq!(Some("Exports".to_string()), part(&own).editor().placeholder);
}

#[test]
fn an_unbound_folder_keeps_its_text_in_its_state() {
    let (mut screen, host, _) = make(r#"<Folder at="1,1" span="12" name="export" />"#);
    type_in(&mut screen, "/tmp/out");
    screen.focus(None);
    assert_eq!(Some("/tmp/out".to_string()), host.state(Some("export")).unwrap().text);
    assert_eq!(vec!["export".to_string()], host.edits.borrow().clone());
    host.set_state("export", |s| s.text = Some("/var/x".into()));
    assert_eq!("/var/x", part(&screen).value());
}

#[test]
fn a_value_set_from_elsewhere_shows() {
    let (mut screen, host, _) = make(MARKUP);
    host.set("settings.exportFolder", json!("/data"));
    Shot::of(&mut screen, 800, 60, 1.0);
    assert_eq!("/data", part(&screen).editor().text());
    assert!(host.logged().is_empty());
}

#[test]
fn it_draws() {
    let (mut screen, _, _) = make(MARKUP);
    let mut shot = Shot::of(&mut screen, 800, 60, 1.0);
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join("inputs-folder.png")).unwrap();
    }
    assert_eq!(60, shot.height());
}
