//! What the screen does for every part: a disabled container disables what
//! is in it, a right press opens a part's menu, a part's tooltip and style
//! show, taborder sorts Tab, the part with the focus hears a key before the
//! window's shortcuts, a symbol shortcut answers the character typed, a
//! part that fails stays in its place, and the pointer takes the shape of
//! what is under it.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use clarity_ui::element::{self, Cx, Element, Node};
use clarity_ui::element_node;
use clarity_ui::frame::FrameMenuList;
use clarity_ui::geometry::Rect;
use clarity_ui::input::{Cursor, Key, KeyInput, Modifiers, PointerButton, PointerInput};
use clarity_ui::markup;
use clarity_ui::painter::Painter;
use clarity_ui::parts::containers::steps::StepsPart;
use clarity_ui::parts::text_box::TextBoxPart;
use clarity_ui::parts::PartHost;
use clarity_ui::screen::TIP_PAUSE;
use clarity_ui::testing::{part_name, Shot, TestHost};
use clarity_ui::window::view::CONTENT_PADDING;
use clarity_ui::window::WindowView;
use clarity_ui::{Screen, ThemeLook};
use serde_json::json;

fn shown(parts: &str, width: f64, height: f64) -> (Screen, Rc<TestHost>) {
    let host = TestHost::new();
    let spec = markup::read(&format!("<Window name=\"Parity\">{parts}</Window>")).expect("the markup reads");
    let as_host: Rc<dyn PartHost> = host.clone();
    let view = WindowView::new(&spec, as_host);
    let mut screen = Screen::new(Box::new(view), ThemeLook::clarity(false));
    screen.resize(width, height, 1.0);
    (screen, host)
}

fn view(screen: &Screen) -> &WindowView {
    screen.root().as_any().downcast_ref::<WindowView>().expect("a window view")
}

/// The id and place of the part named `name`.
fn part(screen: &Screen, name: &str) -> (u64, Rect) {
    let e = element::in_order(screen.root()).into_iter().find(|e| part_name(*e) == Some(name)).unwrap_or_else(|| panic!("no part {name}"));
    (e.node().id(), e.node().bounds)
}

fn click(screen: &mut Screen, at: Rect, button: PointerButton) {
    let (x, y) = (at.x + at.width / 2.0, at.y + at.height / 2.0);
    screen.pointer_pressed(x, y, button, 1);
    screen.pointer_released(x, y, button);
}

fn key(screen: &mut Screen, key: Key, held: Modifiers) -> bool {
    screen.key_pressed(KeyInput::new(key, held))
}

fn focused_name(screen: &Screen) -> Option<String> {
    let id = screen.focused()?;
    let chain = element::chain(screen.root(), id)?;
    chain.iter().rev().filter_map(|i| element::find(screen.root(), *i)).find_map(|e| part_name(e).map(str::to_string))
}

fn part_named_in(screen: &Screen, name: &str) -> u64 {
    part(screen, name).0
}

fn text_box_editor(screen: &Screen, name: &str) -> u64 {
    // What takes the focus in a text box: its editor where it is in the tree, else the text box itself.
    let id = part_named_in(screen, name);
    element::in_order(screen.element(id).unwrap()).into_iter().find(|e| e.focusable()).map(|e| e.node().id()).unwrap_or(id)
}

#[test]
fn a_disabled_panel_disables_its_parts_for_the_pointer_tab_and_keys() {
    let (mut screen, host) = shown(
        r#"<Panel name="box" at="1,1" span="6" rows="3" caption="Box">
              <Button name="inner" at="1,1" span="3" caption="Inner" key="F5" onclick="inner" />
            </Panel>
            <Button name="outer" at="7,1" span="3" caption="Outer" onclick="outer" />"#,
        800.0,
        400.0,
    );
    host.set_state("box", |s| s.enabled = Some(false));
    Shot::of(&mut screen, 800, 400, 1.0);
    let (inner, place) = part(&screen, "inner");
    assert!(!screen.enabled(inner));
    click(&mut screen, place, PointerButton::Left);
    key(&mut screen, Key::F5, Modifiers::NONE);
    assert!(host.logged().is_empty(), "{:?}", host.logged());
    assert_eq!(screen.focused(), None);
    key(&mut screen, Key::Tab, Modifiers::NONE);
    assert_eq!(focused_name(&screen).as_deref(), Some("outer"));
    host.set_state("box", |s| s.enabled = Some(true));
    click(&mut screen, place, PointerButton::Left);
    assert_eq!(host.logged(), vec!["run inner click inner"]);
}

fn menu_lists(screen: &Screen) -> Vec<&FrameMenuList> {
    screen.context_menus().lists().into_iter().filter_map(|id| screen.element(id)).filter_map(|e| e.as_any().downcast_ref::<FrameMenuList>()).collect()
}

#[test]
fn a_right_click_opens_a_parts_menu_and_its_item_runs() {
    let (mut screen, host) = shown(
        r#"<Label name="note" at="1,1" span="6" caption="Right click me">
              <Menu name="noteMenu">
                <Item name="copyItem" caption="Copy" onclick="copied" />
                <Item name="goneItem" caption="Gone" onclick="gone" />
              </Menu>
            </Label>"#,
        800.0,
        400.0,
    );
    host.set_state("goneItem", |s| s.visible = Some(false));
    let (_, label) = part(&screen, "note");
    click(&mut screen, label, PointerButton::Right);
    let lists = menu_lists(&screen);
    assert_eq!(lists.len(), 1);
    let captions: Vec<String> = lists[0].entries().iter().map(|e| e.caption.clone()).collect();
    assert_eq!(captions, vec!["Copy"]);
    let list = lists[0].node().id();
    screen.act(list, |e, cx| e.as_any_mut().downcast_mut::<FrameMenuList>().unwrap().activate(Some(0), cx));
    assert!(!screen.context_menus().is_open());
    assert_eq!(host.logged(), vec!["run note click copied"]);
    // A left click opens no menu.
    click(&mut screen, label, PointerButton::Left);
    assert!(!screen.context_menus().is_open());
}

#[test]
fn every_part_shows_its_tooltip_when_the_pointer_rests_on_it() {
    let (mut screen, host) = shown(
        r#"<Label name="note" at="1,1" span="6" caption="Note" tooltip="@noteTip" />
            <Textbox name="box" at="1,2" span="6" tooltip="Type here" />"#,
        800.0,
        400.0,
    );
    host.texts.borrow_mut().insert("noteTip".into(), "What the note says".into());
    let now = Rc::new(RefCell::new(Instant::now()));
    let clock = now.clone();
    screen.now = Box::new(move || *clock.borrow());
    for (name, tip) in [("note", "What the note says"), ("box", "Type here")] {
        let (_, place) = part(&screen, name);
        screen.pointer_moved(place.x + 5.0, place.y + 5.0);
        *now.borrow_mut() += TIP_PAUSE;
        let at = *now.borrow();
        screen.wake_due(at);
        assert_eq!(screen.tip().map(|t| t.text.as_str()), Some(tip));
    }
}

#[test]
fn a_parts_style_fills_it_and_colors_its_text() {
    let (mut screen, _) = shown(r#"<Label name="loud" at="1,1" span="6" caption="Loud" style="loud" />"#, 800.0, 400.0);
    screen.show(ThemeLook::over(r##"{"styles":{"loud":{"background":"#ff0000","text":"#00ff00"}}}"##, ThemeLook::clarity(false)));
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let (_, label) = part(&screen, "loud");
    let red = shot.pixel((label.right() - 4.0) as i32, (label.y + label.height / 2.0) as i32);
    assert_eq!((red.r(), red.g(), red.b()), (255, 0, 0));
    let mut green = false;
    for x in label.x as i32..label.right() as i32 {
        for y in label.y as i32..(label.y + label.height) as i32 {
            let c = shot.pixel(x, y);
            green |= c.g() > 200 && c.r() < 60;
        }
    }
    assert!(green, "the style's text color draws the caption");
}

/// A style's weight and align reach a part that does not read the style itself (a label).
#[test]
fn a_parts_style_weight_and_align_reach_its_text() {
    fn ink(style: &str) -> (i32, usize) {
        let (mut screen, _) = shown(r#"<Label name="loud" at="1,1" span="6" caption="Loud words" style="loud" />"#, 800.0, 400.0);
        screen.show(ThemeLook::over(&format!("{{\"styles\":{{\"loud\":{{\"text\":\"#00ff00\"{style}}}}}}}"), ThemeLook::clarity(false)));
        let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
        let (_, label) = part(&screen, "loud");
        let (mut left, mut count) = (i32::MAX, 0);
        for x in label.x as i32..label.right() as i32 {
            for y in label.y as i32..(label.y + label.height) as i32 {
                let c = shot.pixel(x, y);
                if c.g() > 150 && c.r() < 100 {
                    left = left.min(x - label.x as i32);
                    count += 1;
                }
            }
        }
        (left, count)
    }
    let plain = ink("");
    let bold = ink(",\"weight\":\"bold\"");
    let right = ink(",\"align\":\"right\"");
    assert!(bold.1 > plain.1 * 11 / 10, "bold {} over plain {}", bold.1, plain.1);
    assert!(right.0 > plain.0 + 100, "right {} over left {}", right.0, plain.0);
}

#[test]
fn taborder_sorts_tab_before_the_tree_order() {
    let (mut screen, _) = shown(
        r#"<Button name="a" at="1,1" span="2" caption="A" />
            <Button name="b" at="3,1" span="2" caption="B" taborder="2" />
            <Button name="c" at="5,1" span="2" caption="C" taborder="1" />"#,
        800.0,
        400.0,
    );
    let mut order = Vec::new();
    for _ in 0..3 {
        key(&mut screen, Key::Tab, Modifiers::NONE);
        order.push(focused_name(&screen));
    }
    assert_eq!(order, vec![Some("c".to_string()), Some("b".to_string()), Some("a".to_string())]);
}

#[test]
fn the_focused_text_box_hears_its_keys_before_the_windows_shortcuts() {
    let (mut screen, host) = shown(
        r#"<Textbox name="note" at="1,1" span="6" />
            <Button name="all" at="7,1" span="3" caption="All" key="Ctrl+A" onclick="all" />
            <Button name="save" at="10,1" span="3" caption="Save" key="Ctrl+S" onclick="save" />"#,
        800.0,
        400.0,
    );
    let editor = text_box_editor(&screen, "note");
    screen.focus(Some(editor));
    screen.text_typed("hello");
    key(&mut screen, Key::A, Modifiers::CONTROL);
    let selected = screen.element(part(&screen, "note").0).unwrap().as_any().downcast_ref::<TextBoxPart>().unwrap().editor().selected_text();
    assert_eq!(selected, "hello");
    key(&mut screen, Key::S, Modifiers::CONTROL);
    assert!(!host.logged().contains(&"run all click all".to_string()));
    assert!(host.logged().contains(&"run save click save".to_string()));
}

#[test]
fn a_typed_symbol_shortcut_answers_the_character_on_any_keyboard() {
    let (mut screen, host) = shown(
        r#"<Button name="times" at="1,1" span="3" caption="Times" key="*" onclick="times" />
            <Button name="plus" at="4,1" span="3" caption="Plus" key="+" onclick="plus" />"#,
        800.0,
        400.0,
    );
    // Shift+8 on a US keyboard types the star; the key is a digit, the character is the shortcut.
    key(&mut screen, Key::D8, Modifiers::SHIFT);
    screen.text_typed("*");
    // The number pad's plus is the key itself: its character does not press it twice.
    key(&mut screen, Key::Plus, Modifiers::NONE);
    screen.text_typed("+");
    assert_eq!(host.logged(), vec!["run times click times", "run plus click plus"]);
    // A typed character is the person's while the focus is in a text box.
    assert!(!clarity_ui::shortcuts::matches("*", &KeyInput::typed('*'), true));
}

/// An element that fails to draw and to take a press.
struct Failing {
    node: Node,
}

impl Element for Failing {
    element_node!();

    fn focusable(&self) -> bool {
        true
    }

    fn draw(&self, _p: &Painter) {
        panic!("no paint today");
    }

    fn on_pointer_pressed(&mut self, _at: PointerInput, _cx: &mut Cx) {
        panic!("no press today");
    }
}

/// Two elements side by side.
struct Row {
    node: Node,
}

impl Row {
    fn of(left: Box<dyn Element>, right: Box<dyn Element>) -> Row {
        let mut node = Node::new();
        node.add(left);
        node.add(right);
        Row { node }
    }
}

impl Element for Row {
    element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        let half = area.width / 2.0;
        let children = self.node.children_mut();
        element::arrange_safely(children[0].as_mut(), Rect::new(area.x, area.y, half, area.height), cx);
        element::arrange_safely(children[1].as_mut(), Rect::new(area.x + half, area.y, half, area.height), cx);
    }
}

struct Filled {
    node: Node,
}

impl Element for Filled {
    element_node!();

    fn draw(&self, p: &Painter) {
        p.fill(self.node.bounds, p.look.color("accent"), 0.0);
    }
}

struct Handy {
    node: Node,
}

impl Element for Handy {
    element_node!();

    fn cursor_at(&self, _x: f64, _y: f64) -> Option<Cursor> {
        Some(Cursor::Hand)
    }
}

#[test]
fn a_control_whose_drawing_fails_shows_why_and_the_window_stays() {
    let failing = Failing { node: Node::new() };
    let failing_id = failing.node.id();
    let row = Row::of(Box::new(failing), Box::new(Filled { node: Node::new() }));
    let mut screen = Screen::new(Box::new(row), ThemeLook::clarity(false));
    let mut shot = Shot::of(&mut screen, 400, 100, 1.0);
    assert_eq!(screen.element(failing_id).unwrap().node().failed().as_deref(), Some("no paint today"));
    assert_eq!(shot.pixel(300, 50), screen.look().color("accent"));
    assert_eq!(shot.pixel(0, 50), screen.look().color("error"));
    // A press that fails says why too, and the screen goes on.
    screen.pointer_pressed(50.0, 50.0, PointerButton::Left, 1);
    screen.pointer_released(50.0, 50.0, PointerButton::Left);
    assert_eq!(screen.element(failing_id).unwrap().node().failed().as_deref(), Some("no press today"));
}

#[test]
fn the_pointer_takes_the_shape_of_what_is_under_it() {
    let row = Row::of(Box::new(Handy { node: Node::new() }), Box::new(Filled { node: Node::new() }));
    let mut screen = Screen::new(Box::new(row), ThemeLook::clarity(false));
    screen.resize(400.0, 100.0, 1.0);
    let mut changes = 0;
    screen.pointer_moved(50.0, 50.0);
    changes += screen.take_cursor_changed() as i32;
    assert_eq!(screen.cursor(), Cursor::Hand);
    screen.pointer_moved(300.0, 50.0);
    changes += screen.take_cursor_changed() as i32;
    assert_eq!(screen.cursor(), Cursor::Default);
    assert_eq!(changes, 2);
}

struct WheelProbe {
    node: Node,
    turns: Rc<RefCell<Vec<(f64, f64)>>>,
}

impl Element for WheelProbe {
    element_node!();

    fn on_wheel(&mut self, dx: f64, dy: f64, _held: Modifiers, _cx: &mut Cx) -> bool {
        self.turns.borrow_mut().push((dx, dy));
        true
    }
}

#[test]
fn shift_turns_the_wheel_sideways() {
    let turns = Rc::new(RefCell::new(Vec::new()));
    let mut screen = Screen::new(Box::new(WheelProbe { node: Node::new(), turns: turns.clone() }), ThemeLook::clarity(false));
    screen.resize(100.0, 100.0, 1.0);
    screen.wheel(10.0, 10.0, 0.0, 50.0, Modifiers::SHIFT);
    screen.wheel(10.0, 10.0, 0.0, 50.0, Modifiers::NONE);
    assert_eq!(*turns.borrow(), vec![(50.0, 0.0), (0.0, 50.0)]);
}

#[test]
fn enter_presses_a_wizards_next_and_on_its_last_step_finish() {
    let (mut screen, host) = shown(
        r#"<Steps name="wizard" at="1,1" span="12" rows="6" onfinish="done">
              <Step title="One"><Textbox name="first" at="1,1" span="4" /></Step>
              <Step title="Two"><Label at="1,1" span="4" caption="Two" /></Step>
            </Steps>"#,
        800.0,
        400.0,
    );
    let editor = text_box_editor(&screen, "first");
    screen.focus(Some(editor));
    key(&mut screen, Key::Enter, Modifiers::NONE);
    let (wizard, _) = part(&screen, "wizard");
    assert_eq!(screen.element(wizard).unwrap().as_any().downcast_ref::<StepsPart>().unwrap().step(), 2);
    key(&mut screen, Key::Enter, Modifiers::NONE);
    assert_eq!(host.logged(), vec!["run wizard finish done"]);
}

fn tall() -> String {
    (1..=20).map(|i| format!(r#"<Textbox name="box{i}" at="1,{i}" span="6" />"#)).collect()
}

#[test]
fn tab_to_a_part_below_the_fold_scrolls_it_into_view() {
    let (mut screen, _) = shown(&tall(), 800.0, 300.0);
    assert!(view(&screen).max_scroll_y > 0.0);
    let last = text_box_editor(&screen, "box20");
    screen.focus(Some(last));
    Shot::of(&mut screen, 800, 300, 1.0);
    assert_eq!(view(&screen).scroll_y, view(&screen).max_scroll_y);
    assert!(part(&screen, "box20").1.bottom() <= 300.0);
    let first = text_box_editor(&screen, "box1");
    screen.focus(Some(first));
    assert_eq!(view(&screen).scroll_y, 0.0);
}

#[test]
fn page_down_and_page_up_scroll_the_window_and_its_bar_drags() {
    let (mut screen, _) = shown(&tall(), 800.0, 300.0);
    key(&mut screen, Key::PageDown, Modifiers::NONE);
    let paged = view(&screen).scroll_y;
    assert!(paged > 100.0, "paged {paged}");
    key(&mut screen, Key::PageUp, Modifiers::NONE);
    assert_eq!(view(&screen).scroll_y, 0.0);
    // Dragging the bar's thumb to the bottom shows the end.
    // The bar's thumb lies at the right of the content, at its top while the window shows its start.
    let thumb = Rect::new(800.0 - CONTENT_PADDING / 2.0 - 2.0, CONTENT_PADDING, 4.0, 16.0);
    screen.pointer_pressed(thumb.x + 1.0, thumb.y + 2.0, PointerButton::Left, 1);
    screen.pointer_moved(thumb.x + 1.0, 299.0);
    screen.pointer_released(thumb.x + 1.0, 299.0, PointerButton::Left);
    assert_eq!(view(&screen).scroll_y, view(&screen).max_scroll_y);
}

#[test]
fn enter_on_a_slider_in_a_form_submits_it() {
    let (mut screen, host) = shown(
        r#"<Form name="order" at="1,1" span="12" rows="3" onsubmit="send">
              <Slider name="amount" at="1,1" span="4" min="0" max="10" />
            </Form>"#,
        800.0,
        400.0,
    );
    let (slider, _) = part(&screen, "amount");
    let taker = element::in_order(screen.element(slider).unwrap()).into_iter().find(|e| e.focusable()).map(|e| e.node().id());
    screen.focus(taker);
    key(&mut screen, Key::Enter, Modifiers::NONE);
    assert_eq!(host.logged(), vec!["run order submit send"]);
}

#[test]
fn a_key_nothing_takes_is_left_to_the_host() {
    let (mut screen, _) = shown(r#"<Button name="save" at="1,1" span="3" caption="Save" key="Ctrl+S" onclick="save" />"#, 800.0, 400.0);
    assert!(key(&mut screen, Key::S, Modifiers::CONTROL));
    assert!(!key(&mut screen, Key::F5, Modifiers::NONE));
    assert!(key(&mut screen, Key::Tab, Modifiers::NONE));
}

#[test]
fn a_splitters_divider_shows_the_way_it_drags() {
    let (mut screen, _) = shown(
        r#"<Splitter name="split" at="1,1" span="12" rows="4">
              <Pane><Label at="1,1" span="3" caption="Left" /></Pane>
              <Pane><Label at="1,1" span="3" caption="Right" /></Pane>
            </Splitter>"#,
        800.0,
        400.0,
    );
    Shot::of(&mut screen, 800, 400, 1.0);
    let (split, _) = part(&screen, "split");
    let divider = screen.element(split).unwrap().node().children().iter().map(|c| c.node().bounds).find(|b| b.width < 20.0 && b.width > 0.0).expect("a divider");
    screen.pointer_moved(divider.x + divider.width / 2.0, divider.y + 10.0);
    assert_eq!(screen.cursor(), Cursor::ResizeLeftRight);
}

#[test]
fn frame_keys_wait_while_a_frame_menu_is_open() {
    let (mut screen, host) = shown(
        r#"<Menu name="appMenu"><Item caption="File"><Item name="print" caption="Print" key="Ctrl+P" onclick="print" /></Item></Menu>"#,
        800.0,
        400.0,
    );
    Shot::of(&mut screen, 800, 400, 1.0);
    key(&mut screen, Key::F, Modifiers::ALT);
    assert!(view(&screen).frame().menus().is_open());
    key(&mut screen, Key::P, Modifiers::CONTROL);
    assert!(!host.logged().iter().any(|l| l.contains("print")), "{:?}", host.logged());
    key(&mut screen, Key::Escape, Modifiers::NONE);
    key(&mut screen, Key::P, Modifiers::CONTROL);
    assert!(host.logged().iter().any(|l| l.contains("print")), "{:?}", host.logged());
}

#[test]
fn a_list_field_shows_each_item_joined() {
    let host = TestHost::new();
    let tags = json!(["red", "blue"]);
    host.set("tags", tags.clone());
    assert_eq!(clarity_ui::parts::containers::small::shown_value(host.as_ref(), "tags", Some(&tags)), "red, blue");
}

#[test]
fn f10_and_alt_alone_put_the_focus_on_the_menu_bar() {
    let (mut screen, _) = shown(
        r#"<Menu name="appMenu"><Item caption="File"><Item name="print" caption="Print" onclick="print" /></Item><Item caption="Edit"><Item name="copy" caption="Copy" onclick="copy" /></Item></Menu>
            <Textbox name="note" at="1,1" span="6" />"#,
        800.0,
        400.0,
    );
    Shot::of(&mut screen, 800, 400, 1.0);
    let bar = view(&screen).menu_bar().expect("a menu bar").node().id();
    screen.key_released(KeyInput::plain(Key::Alt));
    key(&mut screen, Key::F10, Modifiers::NONE);
    assert_eq!(screen.focused(), Some(bar));
    key(&mut screen, Key::Escape, Modifiers::NONE);
    assert_ne!(screen.focused(), Some(bar));
    key(&mut screen, Key::Alt, Modifiers::ALT);
    screen.key_released(KeyInput::plain(Key::Alt));
    assert_eq!(screen.focused(), Some(bar));
}

#[test]
fn a_wizard_shows_the_titles_of_all_steps_the_current_one_marked() {
    let (mut screen, _) = shown(
        r#"<Steps name="wizard" at="1,1" span="12" rows="6">
              <Step title="One"><Label at="1,1" span="4" caption="1" /></Step>
              <Step title="Two"><Label at="1,1" span="4" caption="2" /></Step>
            </Steps>"#,
        800.0,
        400.0,
    );
    Shot::of(&mut screen, 800, 400, 1.0);
    let (wizard, place) = part(&screen, "wizard");
    let accent = screen.look().color("accent");
    let accent_in = |shot: &mut Shot, from: f64, to: f64| {
        for x in from as i32..to as i32 {
            for y in place.y as i32..(place.y + StepsPart::TITLES_HEIGHT) as i32 {
                if shot.pixel(x, y) == accent {
                    return true;
                }
            }
        }
        false
    };
    let first = place.x;
    let second = first + screen.fonts().measure("One", "text", screen.look().size("text"), true) + 6.0 * screen.look().padding();
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(accent_in(&mut shot, first, second - 4.0));
    assert!(!accent_in(&mut shot, second, second + 40.0));
    screen.element(wizard).unwrap().as_any().downcast_ref::<StepsPart>().unwrap().set_step(2);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(!accent_in(&mut shot, first, second - 8.0));
    assert!(accent_in(&mut shot, second, second + 40.0));
}

#[test]
fn a_press_on_a_part_half_out_of_view_clicks_it_without_scrolling_away() {
    let parts: String = (1..=8).map(|i| format!(r#"<Label at="1,{i}" span="6" caption="Row {i}" />"#)).collect::<String>()
        + r#"<Button name="low" at="1,9" span="3" rows="2" caption="Low" onclick="low" />"#;
    let (mut screen, host) = shown(&parts, 800.0, 900.0);
    let (_, button) = part(&screen, "low");
    // The window ends just below the button's top: it is half out of view.
    let height = button.y + 16.0 + CONTENT_PADDING;
    screen.resize(800.0, height, 1.0);
    assert!(view(&screen).max_scroll_y > 0.0);
    let y = button.y + 4.0;
    screen.pointer_pressed(button.x + 4.0, y, PointerButton::Left, 1);
    screen.pointer_released(button.x + 4.0, y, PointerButton::Left);
    assert_eq!(host.logged(), vec!["run low click low"]);
}

#[test]
fn a_right_click_on_a_panels_header_opens_its_menu() {
    let (mut screen, _) = shown(
        r#"<Panel name="box" at="1,1" span="6" rows="4" caption="Customer">
              <Menu name="boxMenu"><Item name="clearItem" caption="Clear" onclick="clear" /></Menu>
              <Label at="1,1" span="3" caption="Inside" />
            </Panel>"#,
        800.0,
        400.0,
    );
    let (_, panel) = part(&screen, "box");
    screen.pointer_pressed(panel.x + 20.0, panel.y + 6.0, PointerButton::Right, 1);
    assert!(screen.context_menus().is_open());
}

#[test]
fn each_typed_symbol_presses_its_shortcut_once_with_shift_held_and_let_go() {
    let (mut screen, host) = shown(r#"<Button name="times" at="1,1" span="3" caption="Times" key="*" onclick="times" />"#, 800.0, 400.0);
    for _ in 0..3 {
        key(&mut screen, Key::Shift, Modifiers::SHIFT);
        key(&mut screen, Key::D8, Modifiers::SHIFT);
        screen.text_typed("*");
        screen.key_released(KeyInput::new(Key::D8, Modifiers::SHIFT));
        screen.key_released(KeyInput::plain(Key::Shift));
    }
    assert_eq!(host.logged(), vec!["run times click times"; 3]);
}
