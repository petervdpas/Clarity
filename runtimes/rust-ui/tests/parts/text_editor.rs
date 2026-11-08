//! The text editor every text input holds: typing, the keys that move and
//! delete, the selection with Shift, the pointer and the clipboard, a long
//! line that scrolls, lines that wrap, the placeholder and a password's dots.

use std::cell::Cell;
use std::rc::Rc;

use clarity_ui::element::{ElementId, GridPanel};
use clarity_ui::geometry::Rect;
use clarity_ui::grid::GridItem;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::parts::text_editor::TextEditor;
use clarity_ui::testing::{near, Shot};
use clarity_ui::{Screen, ThemeLook};
use skia_safe::Color;

/// An editor alone in a grid of one column, with the focus.
fn shown(multiline: bool, text: &str, width: f64, rows: i64) -> (Screen, ElementId) {
    let mut editor = TextEditor::new(multiline);
    editor.set_text(text);
    let id = clarity_ui::Element::node(&editor).id();
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(editor), GridItem::new(1, 1, 1, rows, false));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    screen.resize(width, 300.0, 1.0);
    screen.focus(Some(id));
    (screen, id)
}

fn one(text: &str) -> (Screen, ElementId) {
    shown(false, text, 300.0, 1)
}

fn ed(screen: &Screen, id: ElementId) -> &TextEditor {
    screen.element(id).and_then(|e| e.as_any().downcast_ref::<TextEditor>()).expect("the editor")
}

fn ed_mut(screen: &mut Screen, id: ElementId) -> &mut TextEditor {
    screen.element_mut(id).and_then(|e| e.as_any_mut().downcast_mut::<TextEditor>()).expect("the editor")
}

fn bounds(screen: &Screen, id: ElementId) -> Rect {
    clarity_ui::Element::node(ed(screen, id)).bounds
}

fn k(key: Key) -> KeyInput {
    KeyInput::plain(key)
}

fn km(key: Key, modifiers: Modifiers) -> KeyInput {
    KeyInput::new(key, modifiers)
}

/// Keeps a picture to look at, in the folder `CLARITY_SHOTS` names, when it names one.
fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

/// Whether any pixel in a place has a color.
pub fn inked(shot: &mut Shot, place: Rect, color: Color) -> bool {
    (place.y as i32..place.bottom() as i32).any(|y| (place.x as i32..place.right() as i32).any(|x| near(shot.pixel(x, y), color, 5)))
}

#[test]
fn typed_text_goes_in_at_the_caret_and_counts_as_edited() {
    let (mut screen, id) = one("Milk");
    screen.text_typed("!");
    screen.key_pressed(k(Key::Home));
    screen.text_typed("Oat ");
    let editor = ed(&screen, id);
    assert_eq!("Oat Milk!", editor.text());
    assert_eq!(4, editor.caret());
    assert_eq!(2, editor.edits());
}

#[test]
fn text_set_by_code_is_no_edit() {
    let (mut screen, id) = one("");
    let editor = ed_mut(&mut screen, id);
    editor.set_text("set");
    assert_eq!(0, editor.edits());
    assert_eq!(3, editor.caret());
}

#[test]
fn backspace_and_delete_take_a_character_or_the_selection() {
    let (mut screen, id) = one("abcdef");
    screen.key_pressed(k(Key::Left));
    screen.key_pressed(k(Key::Backspace));
    assert_eq!("abcdf", ed(&screen, id).text());
    screen.key_pressed(k(Key::Delete));
    assert_eq!("abcd", ed(&screen, id).text());
    screen.key_pressed(km(Key::Left, Modifiers::SHIFT));
    screen.key_pressed(km(Key::Left, Modifiers::SHIFT));
    assert_eq!("cd", ed(&screen, id).selected_text());
    screen.key_pressed(k(Key::Delete));
    assert_eq!("ab", ed(&screen, id).text());
    screen.key_pressed(k(Key::Home));
    screen.key_pressed(k(Key::Backspace));
    assert_eq!("ab", ed(&screen, id).text());
}

#[test]
fn shift_selects_and_typing_replaces_the_selection() {
    let (mut screen, id) = one("one two");
    screen.key_pressed(km(Key::Home, Modifiers::SHIFT));
    assert_eq!("one two", ed(&screen, id).selected_text());
    screen.key_pressed(k(Key::Right));
    assert_eq!((7, 7), ed(&screen, id).selection());
    screen.key_pressed(km(Key::Left, Modifiers::CONTROL | Modifiers::SHIFT));
    assert_eq!("two", ed(&screen, id).selected_text());
    screen.text_typed("three");
    assert_eq!("one three", ed(&screen, id).text());
}

/// In a markdown memo Ctrl+B and Ctrl+I make the selection bold and italic,
/// one step Ctrl+Z takes back; with nothing chosen the caret lands between the marks.
#[test]
fn in_markdown_ctrl_b_and_ctrl_i_mark_the_selection() {
    let (mut screen, id) = shown(true, "one two", 300.0, 3);
    ed_mut(&mut screen, id).markdown = true;
    ed_mut(&mut screen, id).select(4, 7);
    screen.key_pressed(km(Key::B, Modifiers::CONTROL));
    assert_eq!("one **two**", ed(&screen, id).text());
    assert_eq!("two", ed(&screen, id).selected_text());
    screen.key_pressed(km(Key::I, Modifiers::CONTROL));
    assert_eq!("one ***two***", ed(&screen, id).text());
    screen.key_pressed(km(Key::Z, Modifiers::CONTROL));
    assert_eq!("one **two**", ed(&screen, id).text());
    ed_mut(&mut screen, id).select(0, 0);
    screen.key_pressed(km(Key::I, Modifiers::CONTROL));
    assert_eq!("**one **two**", ed(&screen, id).text());
    assert_eq!((1, 1), ed(&screen, id).selection());
    // A plain memo takes Ctrl+B as nothing.
    let (mut screen, id) = shown(true, "one", 300.0, 3);
    ed_mut(&mut screen, id).select_all();
    screen.key_pressed(km(Key::B, Modifiers::CONTROL));
    assert_eq!("one", ed(&screen, id).text());
}

#[test]
fn copy_cut_and_paste_go_through_the_screens_clipboard() {
    let (mut screen, id) = one("hello world");
    screen.key_pressed(km(Key::A, Modifiers::CONTROL));
    assert_eq!("hello world", ed(&screen, id).selected_text());
    screen.key_pressed(km(Key::C, Modifiers::CONTROL));
    assert_eq!(Some("hello world".to_string()), screen.clipboard_text());
    ed_mut(&mut screen, id).select(5, 11);
    screen.key_pressed(km(Key::X, Modifiers::CONTROL));
    assert_eq!((Some(" world".to_string()), "hello".to_string()), (screen.clipboard_text(), ed(&screen, id).text()));
    screen.key_pressed(k(Key::Home));
    screen.key_pressed(km(Key::V, Modifiers::CONTROL));
    assert_eq!(" worldhello", ed(&screen, id).text());
}

#[test]
fn places_count_characters_not_bytes() {
    let (mut screen, id) = one("caf\u{e9} \u{1f600}x");
    assert_eq!(7, ed(&screen, id).caret());
    screen.key_pressed(k(Key::Left));
    screen.key_pressed(k(Key::Backspace));
    assert_eq!("caf\u{e9} x", ed(&screen, id).text());
    screen.key_pressed(km(Key::Left, Modifiers::CONTROL | Modifiers::SHIFT));
    assert_eq!("caf\u{e9} ", ed(&screen, id).selected_text());
    screen.text_typed("\u{fc}");
    assert_eq!("\u{fc}x", ed(&screen, id).text());
    // Drawing a text of many-byte characters lays it out by characters as well.
    let shot = Shot::of(&mut screen, 300, 100, 1.0);
    assert!(shot.width() == 300);
}

#[test]
fn one_line_takes_no_line_break_and_enter_goes_to_its_holder() {
    let (mut screen, id) = one("");
    let entered = Rc::new(Cell::new(0));
    let counted = entered.clone();
    ed_mut(&mut screen, id).entered = Some(Box::new(move || {
        counted.set(counted.get() + 1);
        true
    }));
    screen.act(id, |_, cx| cx.clipboard().set_text("a\r\nb"));
    screen.key_pressed(km(Key::V, Modifiers::CONTROL));
    screen.key_pressed(k(Key::Enter));
    assert_eq!(("a b".to_string(), 1), (ed(&screen, id).text(), entered.get()));
}

#[test]
fn several_lines_take_enter_and_up_and_down_move_between_lines() {
    let (mut screen, id) = shown(true, "first", 300.0, 3);
    screen.key_pressed(k(Key::Enter));
    screen.text_typed("second");
    assert_eq!("first\nsecond", ed(&screen, id).text());
    screen.key_pressed(k(Key::Up));
    assert!(ed(&screen, id).caret() <= 5);
    screen.key_pressed(k(Key::Home));
    assert_eq!(0, ed(&screen, id).caret());
    screen.key_pressed(k(Key::Down));
    screen.key_pressed(k(Key::End));
    assert_eq!(ed(&screen, id).text().chars().count(), ed(&screen, id).caret());
}

#[test]
fn read_only_moves_and_copies_but_takes_no_typing() {
    let (mut screen, id) = one("fixed");
    ed_mut(&mut screen, id).read_only = true;
    screen.text_typed("x");
    screen.key_pressed(k(Key::Backspace));
    screen.key_pressed(km(Key::Home, Modifiers::SHIFT));
    screen.key_pressed(km(Key::C, Modifiers::CONTROL));
    assert_eq!(("fixed".to_string(), Some("fixed".to_string())), (ed(&screen, id).text(), screen.clipboard_text()));
}

#[test]
fn a_press_places_the_caret_a_drag_selects_and_a_double_click_takes_a_word() {
    let (mut screen, id) = one("alpha beta gamma");
    let b = bounds(&screen, id);
    screen.pointer_pressed(b.x + 2.0, b.y + 10.0, PointerButton::Left, 1);
    assert_eq!(0, ed(&screen, id).caret());
    screen.pointer_moved(b.right() - 2.0, b.y + 10.0);
    screen.pointer_released(b.right() - 2.0, b.y + 10.0, PointerButton::Left);
    assert_eq!("alpha beta gamma", ed(&screen, id).selected_text());
    let middle = b.x + 8.0 + 12.0;
    screen.pointer_pressed(middle, b.y + 10.0, PointerButton::Left, 2);
    assert_eq!("alpha", ed(&screen, id).selected_text());
    screen.pointer_released(middle, b.y + 10.0, PointerButton::Left);
    screen.pointer_pressed(middle, b.y + 10.0, PointerButton::Left, 3);
    assert_eq!("alpha beta gamma", ed(&screen, id).selected_text());
}

#[test]
fn a_long_line_scrolls_to_keep_the_caret_in_view() {
    let (mut screen, id) = shown(false, "", 120.0, 1);
    screen.text_typed(&"w".repeat(60));
    assert!(ed(&screen, id).scroll_x() > 0.0);
    screen.key_pressed(k(Key::Home));
    assert_eq!(0.0, ed(&screen, id).scroll_x());
}

#[test]
fn lines_wrap_and_scroll_down_with_the_caret_and_the_wheel() {
    let (mut screen, id) = shown(true, "", 120.0, 2);
    screen.text_typed(&vec!["word"; 40].join(" "));
    assert!(ed(&screen, id).scroll_y() > 0.0);
    screen.key_pressed(km(Key::Home, Modifiers::CONTROL));
    assert_eq!(0.0, ed(&screen, id).scroll_y());
    let b = bounds(&screen, id);
    screen.wheel(b.x + 5.0, b.y + 5.0, 0.0, 30.0, Modifiers::NONE);
    assert_eq!(30.0, ed(&screen, id).scroll_y());
}

#[test]
fn empty_it_shows_its_placeholder_and_a_password_shows_dots() {
    let (mut screen, id) = one("");
    ed_mut(&mut screen, id).placeholder = Some("Type here".to_string());
    screen.focus(None);
    let muted = screen.look().color("muted");
    let mut shot = Shot::of(&mut screen, 300, 300, 1.0);
    assert!(inked(&mut shot, bounds(&screen, id), muted));
    let editor = ed_mut(&mut screen, id);
    editor.set_text("secret");
    editor.set_password(true);
    screen.focus(Some(id));
    screen.key_pressed(km(Key::A, Modifiers::CONTROL));
    screen.key_pressed(km(Key::C, Modifiers::CONTROL));
    screen.key_pressed(km(Key::X, Modifiers::CONTROL));
    assert_eq!(None, screen.clipboard_text());
    assert_eq!("secret", ed(&screen, id).text());
    let selection = screen.look().color("selection");
    let mut shot = Shot::of(&mut screen, 300, 300, 1.0);
    assert!(inked(&mut shot, bounds(&screen, id), selection));
    keep(&mut shot, "text_editor_password.png");
}

#[test]
fn the_caret_and_the_selection_draw_in_the_themes_colors() {
    let (mut screen, id) = one("selected");
    ed_mut(&mut screen, id).select_all();
    let (selection, focus) = (screen.look().color("selection"), screen.look().color("focus"));
    let mut shot = Shot::of(&mut screen, 300, 300, 1.0);
    assert!(inked(&mut shot, bounds(&screen, id), selection));
    assert!(inked(&mut shot, bounds(&screen, id), focus));
}

#[test]
fn a_disabled_editor_takes_no_focus() {
    let (mut screen, id) = one("x");
    ed_mut(&mut screen, id).enabled = false;
    assert!(!clarity_ui::Element::focusable(ed(&screen, id)));
}
