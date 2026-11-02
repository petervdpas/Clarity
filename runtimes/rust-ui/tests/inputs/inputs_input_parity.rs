//! The inputs as the Avalonia version had them, where the audit found
//! Clarity.UI apart: a text input's right-click menu, its length limit and
//! its pointer; the password's stars; the checkbox and switch by Enter and
//! by dragging; the radio group's Tab stops; the slider that snaps while
//! dragged; numbers read from the right and a spin box that repeats while
//! held; the typed dropdown's list that goes round and an Enter that stays
//! there; the color part's palette by keyboard, its tooltips and its Enter
//! that submits.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use clarity_ui::element::{find, take_typed_from, Cx, ElementId, GridPanel, Node};
use clarity_ui::input::{Cursor, Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::parts::check_box::TRACK_WIDTH;
use clarity_ui::parts::inputs::choice_list;
use clarity_ui::parts::inputs::color::{ColorPart, PaletteGrid, SWATCH_WIDTH};
use clarity_ui::parts::inputs::number::{NumberPart, REPEAT_DELAY, REPEAT_INTERVAL};
use clarity_ui::parts::inputs::radio::RadioPart;
use clarity_ui::parts::inputs::slider::{SliderPart, THUMB};
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::inputs::typed_dropdown::TypeAheadPart;
use clarity_ui::parts::text_box::TextBoxPart;
use clarity_ui::parts::text_editor::TextEditor;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::{element_node, locale, Element, Painter, Rect, Screen, TextAlign, ThemeLook};
use serde_json::{json, Value};

fn press(screen: &mut Screen, key: Key) {
    screen.key_pressed(KeyInput::plain(key));
}

fn click(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn middle(b: Rect) -> (f64, f64) {
    (b.x + b.width / 2.0, b.y + b.height / 2.0)
}

fn window(markup: &str, host: Rc<InputsHost>) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(800.0, 400.0, 1.0);
    Shot::of(&mut screen, 800, 400, 1.0);
    screen
}

fn part<'a, T: Element>(screen: &'a Screen, name: &str) -> &'a T {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<T>()).expect("a part of that kind")
}

/// A form around a grid of parts: Enter in its one-line inputs that they leave to it takes what is typed and submits it.
struct TestForm {
    node: Node,
    submitted: Rc<Cell<u32>>,
}

impl Element for TestForm {
    element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        for child in self.node.children_mut() {
            child.arrange(area, cx);
        }
    }

    fn draw(&self, p: &Painter) {
        clarity_ui::element::draw_children(&self.node, p);
    }

    fn window_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.key != Key::Enter {
            return false;
        }
        let focused = cx.focused();
        if focused.and_then(|f| find(self, f)).is_none_or(|e| e.multiline()) {
            return false;
        }
        take_typed_from(self, focused, cx);
        self.submitted.set(self.submitted.get() + 1);
        true
    }
}

/// The parts of `markup` in a form, on a screen of 800 by 400; how often it submitted.
fn in_form(markup: &str, host: Rc<InputsHost>) -> (Screen, Rc<Cell<u32>>) {
    let spec = clarity_ui::markup::read(&format!("<Window name=\"Test\">{markup}</Window>")).expect("the markup reads");
    let mut grid = GridPanel::new(12);
    for p in spec.parts {
        let made = clarity_ui::parts::make(p, host.clone());
        let place = clarity_ui::parts::place_of(made.as_ref());
        grid.add(made, place);
    }
    let submitted = Rc::new(Cell::new(0));
    let mut form = TestForm { node: Node::new(), submitted: submitted.clone() };
    form.node.add(Box::new(grid));
    let mut screen = Screen::new(Box::new(form), ThemeLook::clarity(false));
    screen.resize(800.0, 400.0, 1.0);
    Shot::of(&mut screen, 800, 400, 1.0);
    (screen, submitted)
}

/// Puts `text` in the input `name` as the person types it, over what it had.
fn type_in(screen: &mut Screen, name: &str, text: &str) {
    let id = part_named(screen, name);
    screen.focus(Some(id));
    screen.key_pressed(KeyInput::new(Key::A, Modifiers::CONTROL));
    press(screen, Key::Backspace);
    if !text.is_empty() {
        screen.text_typed(text);
    }
}

// -- Text inputs ----------------------------------------------------------

#[test]
fn a_right_press_on_a_text_box_offers_cut_copy_paste_and_select_all() {
    let host = InputsHost::new();
    host.set("task.title", json!("Write"));
    let mut screen = window(r#"<Textbox at="1,1" span="6" name="title" bind="task.title" />"#, host);
    let id = part_named(&screen, "title");
    let (x, y) = middle(part::<TextBoxPart>(&screen, "title").editor().node().bounds);
    screen.pointer_pressed(x, y, PointerButton::Right, 1);
    assert!(screen.context_menus().is_open());
    let entries = |screen: &Screen| screen.element(id).unwrap().context_menu_at(x, y).expect("its menu");
    let captions: Vec<String> = entries(&screen).iter().map(|e| e.caption.clone()).collect();
    let wanted = [locale::translate("cutText", "Cut"), locale::translate("copyText", "Copy"), locale::translate("pasteText", "Paste"), locale::translate("selectAllText", "Select all")];
    assert_eq!(wanted.to_vec(), captions);
    // Nothing selected: Cut and Copy wait.
    assert_eq!(vec![false, false, true, true], entries(&screen).iter().map(|e| e.enabled).collect::<Vec<_>>());
    // A picked item runs once its holder wakes, which the menu asks for.
    assert!(entries(&screen).iter().all(|e| e.wake == Some(id)));
    let pick = |screen: &mut Screen, at: usize| {
        let entry = entries(screen)[at].clone();
        (entry.run.expect("it runs something"))();
        screen.act(id, |e, cx| e.on_wake(cx));
    };
    pick(&mut screen, 3);
    assert_eq!("Write", part::<TextBoxPart>(&screen, "title").editor().selected_text());
    pick(&mut screen, 0);
    assert_eq!("", part::<TextBoxPart>(&screen, "title").editor().text());
    assert_eq!(Some("Write".to_string()), screen.clipboard_text());
    pick(&mut screen, 2);
    pick(&mut screen, 2);
    assert_eq!("WriteWrite", part::<TextBoxPart>(&screen, "title").editor().text());
    // A read-only box copies but neither cuts nor pastes.
    screen.act(id, |e, _| {
        let editor = e.as_any_mut().downcast_mut::<TextBoxPart>().unwrap().editor_mut();
        editor.read_only = true;
        editor.select_all();
    });
    let read_only = screen.element(id).unwrap().as_any().downcast_ref::<TextBoxPart>().unwrap().editor().menu(id).unwrap();
    assert_eq!(vec![false, true, false, true], read_only.iter().map(|e| e.enabled).collect::<Vec<_>>());
}

#[test]
fn a_text_input_shows_the_text_cursor_and_a_password_shows_stars() {
    let host = InputsHost::new();
    host.set("user.secret", json!("abc"));
    let mut screen = window(r#"<Password at="1,1" span="6" name="secret" bind="user.secret" />"#, host);
    let b = part::<TextBoxPart>(&screen, "secret").editor().node().bounds;
    assert_eq!("***", part::<TextBoxPart>(&screen, "secret").editor().shown());
    screen.pointer_moved(b.x + 10.0, b.y + b.height / 2.0);
    assert_eq!(Cursor::Text, screen.cursor());
    // Its buttons take no focus: the box is the one Tab stop.
    assert_eq!(vec![part_named(&screen, "secret")], screen.tab_order());
}

#[test]
fn a_text_editor_takes_at_most_its_length_of_typing() {
    let mut editor = TextEditor::new(false);
    editor.max_length = Some(7);
    let mut screen = Screen::new(Box::new(editor), ThemeLook::clarity(false));
    screen.resize(200.0, 40.0, 1.0);
    let id = screen.root().node().id();
    screen.focus(Some(id));
    let text = |screen: &Screen| screen.root().as_any().downcast_ref::<TextEditor>().unwrap().text();
    screen.text_typed("#ff00aa11");
    assert_eq!("#ff00aa", text(&screen));
    screen.text_typed("x");
    assert_eq!("#ff00aa", text(&screen));
    // Text set from elsewhere is not cut.
    screen.act_root::<TextEditor, _>(|e, _| e.set_text("#1234567"));
    assert_eq!("#1234567", text(&screen));
}

// -- Checkbox and switch ------------------------------------------------------

#[test]
fn enter_ticks_the_focused_checkbox_and_a_read_only_one_is_no_tab_stop() {
    let host = InputsHost::new();
    host.set("task.done", json!(false));
    let mut screen = window(
        r#"<Checkbox at="1,1" span="6" name="done" bind="task.done" caption="Done" />
           <Checkbox at="1,2" span="6" name="kept" bind="task.done" caption="Kept" readonly="true" />"#,
        host.clone(),
    );
    press(&mut screen, Key::Tab);
    assert_eq!(Some(part_named(&screen, "done")), screen.focused());
    press(&mut screen, Key::Enter);
    assert_eq!(Some(json!(true)), host.read("task.done"));
    let kept = part_named(&screen, "kept");
    assert!(!screen.element(kept).unwrap().focusable());
    assert!(!screen.tab_order().contains(&kept));
}

#[test]
fn a_switchs_knob_dragged_to_the_other_side_flips_it_once() {
    let host = InputsHost::new();
    host.set("task.done", json!(false));
    let mut screen = window(r#"<Switch at="1,1" span="6" name="done" bind="task.done" caption="Done" onchange="flipped" />"#, host.clone());
    let b = screen.element(part_named(&screen, "done")).unwrap().node().bounds;
    let y = b.y + b.height / 2.0;
    let x = b.x + 8.0;
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x + 10.0, y);
    screen.pointer_moved(x + 22.0, y);
    screen.pointer_released(x + 22.0, y, PointerButton::Left);
    assert_eq!(Some(json!(true)), host.read("task.done"));
    assert_eq!(1, host.logged().iter().filter(|l| l.starts_with("run done change")).count());
    // Dragged only a little, back on its own side: no flip.
    let x = b.x + TRACK_WIDTH - 8.0;
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x - 6.0, y);
    screen.pointer_released(x - 6.0, y, PointerButton::Left);
    assert_eq!(Some(json!(true)), host.read("task.done"));
    // A click still flips it.
    click(&mut screen, x, y);
    assert_eq!(Some(json!(false)), host.read("task.done"));
}

// -- Radio group ---------------------------------------------------------------

#[test]
fn every_choice_is_a_tab_stop_and_enter_or_space_picks_it() {
    let host = InputsHost::new();
    host.set("task.size", json!("Small"));
    host.offer("task.size", &[(json!("Small"), "Small"), (json!("Medium"), "Medium"), (json!("Large"), "Large")]);
    let mut screen = window(
        r#"<Radio at="1,1" span="12" name="size" bind="task.size" />
           <Button at="1,2" span="2" name="after" caption="After" />"#,
        host.clone(),
    );
    let buttons = part::<RadioPart>(&screen, "size").buttons();
    press(&mut screen, Key::Tab);
    assert_eq!(Some(buttons[0]), screen.focused());
    press(&mut screen, Key::Tab);
    assert_eq!(Some(buttons[1]), screen.focused());
    // Tab only moves the focus.
    assert_eq!(Some(json!("Small")), host.read("task.size"));
    press(&mut screen, Key::Enter);
    assert_eq!(Some(json!("Medium")), host.read("task.size"));
    press(&mut screen, Key::Tab);
    press(&mut screen, Key::Space);
    assert_eq!(Some(json!("Large")), host.read("task.size"));
    press(&mut screen, Key::Tab);
    assert_eq!(Some(part_named(&screen, "after")), screen.focused());
    screen.key_pressed(KeyInput::new(Key::Tab, Modifiers::SHIFT));
    assert_eq!(Some(buttons[2]), screen.focused());
    // Side by side the choices stand 12 apart, as Avalonia's row did.
    let places = part::<RadioPart>(&screen, "size").places().to_vec();
    assert!((places[1].x - places[0].right() - 12.0).abs() <= 3.0, "{places:?}");
}

// -- Slider -------------------------------------------------------------------

#[test]
fn a_dragged_thumb_snaps_to_the_steps_and_no_value_shows_at_the_lowest() {
    let host = InputsHost::new();
    let mut screen = window(r#"<Slider at="1,1" span="12" name="rating" bind="task.rating" min="2" max="10" step="2" />"#, host.clone());
    let slider = |screen: &Screen| part::<SliderPart>(screen, "rating").shown();
    assert_eq!(2.0, slider(&screen));
    let b = part::<SliderPart>(&screen, "rating").node().bounds;
    let track = b.width - THUMB;
    let y = b.y + b.height / 2.0;
    screen.pointer_pressed(b.x + THUMB / 2.0 + track * 0.3, y, PointerButton::Left, 1);
    assert_eq!(4.0, slider(&screen));
    screen.pointer_moved(b.x + THUMB / 2.0 + track * 0.6, y);
    assert_eq!(6.0, slider(&screen));
    assert!(host.read("task.rating").is_none_or(|v| v.is_null()));
    screen.pointer_released(b.x + THUMB / 2.0 + track * 0.6, y, PointerButton::Left);
    assert_eq!(Some(json!(6)), host.read("task.rating"));
}

// -- Number and spin box --------------------------------------------------------

#[test]
fn numbers_read_from_the_right_and_a_spin_boxs_arrows_take_no_focus() {
    let host = InputsHost::new();
    host.set("line.quantity", json!(3));
    let screen = window(
        r#"<Number at="1,1" span="6" name="amount" bind="line.amount" />
           <Spinbox at="1,2" span="6" name="quantity" bind="line.quantity" min="0" max="10" />"#,
        host,
    );
    assert_eq!(TextAlign::Right, part::<NumberPart>(&screen, "amount").editor().align);
    assert_eq!(TextAlign::Right, part::<NumberPart>(&screen, "quantity").editor().align);
    assert!(part::<NumberPart>(&screen, "quantity").up().is_some());
    assert_eq!(vec![part_named(&screen, "amount"), part_named(&screen, "quantity")], screen.tab_order());
}

#[test]
fn a_spin_boxs_arrow_held_keeps_spinning_and_a_spin_clears_the_mark() {
    let host = InputsHost::new();
    host.set("line.quantity", json!("3"));
    let mut screen = window(r#"<Spinbox at="1,1" span="6" name="quantity" min="0" max="100" />"#, host);
    type_in(&mut screen, "quantity", "x");
    screen.focus(None);
    assert!(part::<NumberPart>(&screen, "quantity").bad().is_some());
    let clock = Rc::new(Cell::new(Instant::now()));
    let now = clock.clone();
    screen.now = Box::new(move || now.get());
    let (x, y) = middle(part::<NumberPart>(&screen, "quantity").up().unwrap().bounds);
    screen.pointer_moved(x, y);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    assert!(part::<NumberPart>(&screen, "quantity").bad().is_none());
    let shown = |screen: &Screen| part::<NumberPart>(screen, "quantity").editor().text().parse::<f64>().unwrap();
    let first = shown(&screen);
    let wait = |screen: &mut Screen, by: Duration| {
        clock.set(clock.get() + by);
        screen.wake_due(clock.get());
    };
    // Held: after a pause it repeats, and goes on.
    wait(&mut screen, REPEAT_DELAY);
    assert_eq!(first + 1.0, shown(&screen));
    wait(&mut screen, REPEAT_INTERVAL);
    wait(&mut screen, REPEAT_INTERVAL);
    assert_eq!(first + 3.0, shown(&screen));
    screen.pointer_released(x, y, PointerButton::Left);
    wait(&mut screen, REPEAT_INTERVAL);
    assert_eq!(first + 3.0, shown(&screen));
}

// -- Typed and search dropdowns -----------------------------------------------------

fn typed(value: Option<&str>, bind: &str) -> (Screen, Rc<InputsHost>, Rc<Cell<u32>>) {
    let host = InputsHost::new();
    host.set("person.city", value.map(Value::from).unwrap_or(Value::Null));
    host.offer("person.city", &[(json!("Utrecht"), "Utrecht"), (json!("Gent"), "Gent"), (json!("Graz"), "Graz")]);
    let (screen, submitted) = in_form(&format!(r#"<Dropdown at="1,1" span="4" name="city" {bind} typed="true" />"#), host.clone());
    (screen, host, submitted)
}

fn typed_part(screen: &Screen) -> &TypeAheadPart {
    part::<TypeAheadPart>(screen, "city")
}

#[test]
fn up_and_down_go_round_the_typed_list_and_enter_with_no_row_does_not_submit() {
    let (mut screen, host, submitted) = typed(Some("Gent"), r#"bind="person.city""#);
    // Emptied, the box offers every value.
    type_in(&mut screen, "city", "");
    let list = screen.overlays().last().expect("its list").place;
    assert_eq!(3.0 * choice_list::LINE + 2.0, list.height);
    press(&mut screen, Key::Up);
    assert_eq!(Some(2), typed_part(&screen).highlighted());
    press(&mut screen, Key::Down);
    assert_eq!(Some(0), typed_part(&screen).highlighted());
    press(&mut screen, Key::Up);
    assert_eq!(Some(2), typed_part(&screen).highlighted());
    press(&mut screen, Key::Escape);
    screen.key_pressed(KeyInput::new(Key::A, Modifiers::CONTROL));
    screen.text_typed("Rome");
    assert!(screen.overlays().is_empty());
    press(&mut screen, Key::Enter);
    assert_eq!(Some(json!("Rome")), host.read("person.city"));
    assert_eq!(0, submitted.get());
    // The list is at most 240 high, and scrolls.
    let b = typed_part(&screen).node().bounds;
    assert!(choice_list::below(screen.whole(), b, 100.0, 50).height <= 240.0);
}

#[test]
fn the_typed_list_closes_when_nothing_fits_and_leaving_over_it_takes_nothing() {
    let (mut screen, host, _) = typed(Some("Gent"), r#"bind="person.city""#);
    type_in(&mut screen, "city", "G");
    let list = screen.overlays().last().expect("its list").place;
    // The pointer over the list: the focus going for a press there is no leaving.
    screen.pointer_moved(list.x + 5.0, list.y + 5.0);
    screen.focus(None);
    assert_eq!(Some(json!("Gent")), host.read("person.city"));
    type_in(&mut screen, "city", "zz");
    assert!(typed_part(&screen).list().is_none());
}

#[test]
fn an_unbound_typed_dropdown_keeps_what_was_typed() {
    let (mut screen, host, _) = typed(None, "");
    let id: ElementId = part_named(&screen, "city");
    screen.focus(Some(id));
    screen.text_typed("Lima");
    screen.focus(None);
    assert_eq!("Lima", typed_part(&screen).editor().text());
    assert_eq!(Some("Lima".to_string()), host.state(Some("city")).and_then(|s| s.text));
}

// -- Color ---------------------------------------------------------------------------

fn colored() -> (Screen, Rc<InputsHost>, Rc<Cell<u32>>) {
    let host = InputsHost::new();
    host.set("task.tint", json!("#3b82f6"));
    let (screen, submitted) = in_form(r#"<Color at="1,1" span="6" name="tint" bind="task.tint" />"#, host.clone());
    (screen, host, submitted)
}

#[test]
fn the_palette_works_by_keyboard() {
    let (mut screen, host, _) = colored();
    let swatch = part::<ColorPart>(&screen, "tint").swatch_id();
    screen.focus(Some(swatch));
    press(&mut screen, Key::Enter);
    let colors = PaletteGrid::colors();
    let marked = |screen: &Screen| part::<ColorPart>(screen, "tint").marked();
    let chosen = colors.iter().position(|c| *c == "#3b82f6").unwrap_or(0);
    assert_eq!(Some(chosen), marked(&screen));
    press(&mut screen, Key::Home);
    assert_eq!(Some(0), marked(&screen));
    press(&mut screen, Key::Right);
    press(&mut screen, Key::Down);
    assert_eq!(Some(11), marked(&screen));
    press(&mut screen, Key::Tab);
    assert_eq!(Some(12), marked(&screen));
    assert_eq!(Some(swatch), screen.focused());
    screen.key_pressed(KeyInput::new(Key::Tab, Modifiers::SHIFT));
    press(&mut screen, Key::Space);
    assert_eq!(Some(json!(colors[11])), host.read("task.tint"));
    assert!(part::<ColorPart>(&screen, "tint").palette().is_none());
    // Opened again, Escape closes it and picks nothing.
    press(&mut screen, Key::Down);
    assert!(part::<ColorPart>(&screen, "tint").palette().is_some());
    press(&mut screen, Key::Escape);
    assert!(part::<ColorPart>(&screen, "tint").palette().is_none());
    assert_eq!(Some(json!(colors[11])), host.read("task.tint"));
}

#[test]
fn each_palette_color_says_its_code_and_the_box_takes_seven_characters() {
    let (mut screen, _, _) = colored();
    let id = part_named(&screen, "tint");
    screen.act(id, |e, cx| e.as_any_mut().downcast_mut::<ColorPart>().unwrap().toggle(cx));
    Shot::of(&mut screen, 800, 400, 1.0);
    let grid = screen.overlays().last().unwrap().content.as_any().downcast_ref::<PaletteGrid>().expect("the palette");
    let color = PaletteGrid::colors()[5];
    let place = grid.place(color).unwrap();
    assert_eq!(Some(color.to_string()), grid.tooltip_at(place.x + 3.0, place.y + 3.0));
    let b = grid.node().bounds;
    assert_eq!(None, grid.tooltip_at(b.x + 1.0, b.y + 1.0));
    screen.act(id, |e, cx| e.as_any_mut().downcast_mut::<ColorPart>().unwrap().toggle(cx));
    assert_eq!(Some(7), part::<ColorPart>(&screen, "tint").max_length());
    assert_eq!(SWATCH_WIDTH, part::<ColorPart>(&screen, "tint").swatch().width);
}

#[test]
fn enter_in_the_color_box_takes_the_color_and_submits_the_form() {
    let (mut screen, host, submitted) = colored();
    type_in(&mut screen, "tint", "#ff0000");
    press(&mut screen, Key::Enter);
    assert_eq!(Some(json!("#ff0000")), host.read("task.tint"));
    assert_eq!(1, submitted.get());
}
