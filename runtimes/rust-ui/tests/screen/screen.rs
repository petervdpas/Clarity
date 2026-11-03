//! What the screen does for every element in its tree: the pointer held by
//! the element pressed, the wheel going up to the one that scrolls, overlays
//! over everything, focus and Tab, keys asked of the focused element first, wake-ups,
//! tooltips, joining and leaving the screen, and drawing at any scale.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use clarity_ui::element::{self, Cx, Element, ElementId, GridPanel, Node, Plate};
use clarity_ui::element_node;
use clarity_ui::grid::GridItem;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton, PointerInput};
use clarity_ui::overlay::Overlay;
use clarity_ui::testing::Shot;
use clarity_ui::{Rect, Screen, ThemeLook};

/// An element that tells what it heard.
#[derive(Default)]
struct Probe {
    node: Node,
    seen: Rc<RefCell<Vec<String>>>,
    takes_wheel: bool,
    focusable: bool,
    tip: Option<String>,
    hidden: bool,
    takes_keys: bool,
}

impl Probe {
    fn new(seen: &Rc<RefCell<Vec<String>>>) -> Probe {
        Probe { seen: seen.clone(), focusable: true, ..Probe::default() }
    }

    fn say(&self, what: String) {
        self.seen.borrow_mut().push(what);
    }
}

impl Element for Probe {
    element_node!();

    fn focusable(&self) -> bool {
        self.focusable
    }

    fn is_shown(&self) -> bool {
        !self.hidden
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, _cx: &mut Cx) {
        self.say(format!("down {:.0},{:.0}", at.x, at.y));
    }

    fn on_pointer_moved(&mut self, at: PointerInput, _cx: &mut Cx) {
        if self.node.pressed {
            self.say(format!("drag {:.0},{:.0}", at.x, at.y));
        }
    }

    fn on_pointer_released(&mut self, at: PointerInput, _cx: &mut Cx) {
        self.say(format!("up {:.0},{:.0}", at.x, at.y));
    }

    fn on_click(&mut self, _cx: &mut Cx) {
        self.say("click".to_string());
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, _cx: &mut Cx) -> bool {
        if self.takes_wheel {
            self.say(format!("wheel {dy:.0}"));
        }
        self.takes_wheel
    }

    fn on_focus_changed(&mut self, focused: bool, _cx: &mut Cx) {
        self.say(format!("focus {focused}"));
    }

    fn on_key(&mut self, key: KeyInput, _cx: &mut Cx) -> bool {
        self.say(format!("key {:?}", key.key));
        self.takes_keys
    }

    fn on_text(&mut self, text: &str, _cx: &mut Cx) {
        self.say(format!("text {text}"));
    }

    fn on_wake(&mut self, _cx: &mut Cx) {
        self.say("wake".to_string());
    }

    fn on_attached(&mut self, _cx: &mut Cx) {
        self.say("attached".to_string());
    }

    fn on_detached(&mut self, _cx: &mut Cx) {
        self.say("detached".to_string());
    }

    fn tooltip_at(&self, _x: f64, _y: f64) -> Option<String> {
        self.tip.clone()
    }
}

/// An element that holds another 10 in from its edges.
struct Holder {
    node: Node,
}

impl Holder {
    fn new(inner: Box<dyn Element>) -> Holder {
        let mut node = Node::new();
        node.add(inner);
        Holder { node }
    }
}

impl Element for Holder {
    element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        for child in self.node.children_mut() {
            child.arrange(area.inset(10.0), cx);
        }
    }
}

fn log() -> Rc<RefCell<Vec<String>>> {
    Rc::new(RefCell::new(Vec::new()))
}

fn take(seen: &Rc<RefCell<Vec<String>>>) -> Vec<String> {
    std::mem::take(&mut *seen.borrow_mut())
}

fn two_probes() -> (Screen, ElementId, ElementId, Rc<RefCell<Vec<String>>>, Rc<RefCell<Vec<String>>>) {
    let (a, b) = (log(), log());
    let mut grid = GridPanel::new(2);
    let left = Probe::new(&a);
    let right = Probe::new(&b);
    let (l, r) = (left.node().id(), right.node().id());
    grid.add(Box::new(left), GridItem::new(1, 1, 1, 1, false));
    grid.add(Box::new(right), GridItem::new(2, 1, 1, 1, false));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    screen.resize(400.0, 200.0, 1.0);
    take(&a);
    take(&b);
    (screen, l, r, a, b)
}

#[test]
fn the_pointer_hovers_one_element_at_a_time() {
    let (mut screen, left, right, _, _) = two_probes();
    screen.take_redraw();
    let hovered = |screen: &Screen, id: ElementId| screen.element(id).unwrap().node().hovered;
    let mut drawings = 0;
    for (x, y) in [(50.0, 10.0), (60.0, 12.0), (300.0, 10.0), (300.0, 150.0)] {
        screen.pointer_moved(x, y);
        drawings += screen.take_redraw() as usize;
        match (x, y) {
            (300.0, 10.0) => assert!(!hovered(&screen, left) && hovered(&screen, right)),
            (300.0, 150.0) => assert!(!hovered(&screen, left) && !hovered(&screen, right)),
            _ => assert!(hovered(&screen, left) && !hovered(&screen, right)),
        }
    }
    assert!(screen.hovered() != Some(left) && screen.hovered() != Some(right));
    // One redraw for each change of what is hovered; a move within the same element asks none.
    assert_eq!(3, drawings);
}

#[test]
fn a_press_is_held_until_it_is_let_go_wherever_the_pointer_goes() {
    let (mut screen, left, _, seen, _) = two_probes();
    screen.pointer_pressed(10.0, 10.0, PointerButton::Left, 1);
    screen.pointer_moved(300.0, 150.0);
    screen.pointer_released(300.0, 150.0, PointerButton::Left);
    // Where it is in the element's own place, from its top left; no click, as it was let go elsewhere.
    assert_eq!(vec!["focus true", "down 10,10", "drag 300,150", "up 300,150"], take(&seen));
    assert!(!screen.element(left).unwrap().node().pressed);
}

#[test]
fn a_press_and_release_on_the_same_element_is_a_click() {
    let (mut screen, left, _, seen, _) = two_probes();
    screen.click(10.0, 10.0);
    assert_eq!(vec!["focus true", "down 10,10", "up 10,10", "click"], take(&seen));
    assert_eq!(Some(left), screen.focused());
}

#[test]
fn the_wheel_goes_up_to_the_element_that_scrolls() {
    let seen = log();
    let inner = Probe::new(&seen);
    let inner_id = inner.node().id();
    let mut screen = Screen::new(Box::new(Holder::new(Box::new(inner))), ThemeLook::clarity(false));
    screen.resize(200.0, 200.0, 1.0);
    take(&seen);
    screen.wheel(50.0, 50.0, 0.0, 40.0, Modifiers::NONE);
    assert!(take(&seen).is_empty());
    screen.act(inner_id, |e, _| e.as_any_mut().downcast_mut::<Probe>().unwrap().takes_wheel = true);
    screen.wheel(50.0, 50.0, 0.0, 40.0, Modifiers::NONE);
    assert_eq!(vec!["wheel 40"], take(&seen));
}

#[test]
fn an_overlay_lies_over_everything_and_a_press_outside_closes_it() {
    let (under_seen, list_seen) = (log(), log());
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(Probe::new(&under_seen)), GridItem::new(1, 1, 1, 4, false));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    screen.resize(400.0, 200.0, 1.0);
    let closed = Rc::new(RefCell::new(0));
    let list = Probe::new(&list_seen);
    let list_id = list.node().id();
    let counter = closed.clone();
    screen.open(Overlay { closed: Some(Box::new(move || *counter.borrow_mut() += 1)), ..Overlay::new(Box::new(list), Rect::new(0.0, 0.0, 200.0, 100.0)) });
    assert_eq!(Some(list_id), screen.at(50.0, 50.0));
    take(&under_seen);
    screen.click(50.0, 50.0);
    assert!(take(&list_seen).contains(&"click".to_string()));
    // A press outside closes it, and does not reach what is under.
    screen.click(300.0, 150.0);
    assert_eq!(1, *closed.borrow());
    assert!(!take(&under_seen).contains(&"click".to_string()));
    assert!(screen.overlays().is_empty());
    // Escape closes the one on top too.
    let counter = closed.clone();
    screen.open(Overlay { closed: Some(Box::new(move || *counter.borrow_mut() += 1)), ..Overlay::new(Box::new(Plate::new("accent", None)), Rect::new(0.0, 0.0, 10.0, 10.0)) });
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert_eq!(2, *closed.borrow());
}

/// A dropdown's list or a calendar lets a press outside it go on to what is
/// under once it closed: a click on a button while a list is open presses it.
#[test]
fn a_press_outside_a_list_that_lets_it_through_reaches_what_is_under() {
    let (under_seen, list_seen) = (log(), log());
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(Probe::new(&under_seen)), GridItem::new(1, 1, 1, 4, false));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    screen.resize(400.0, 200.0, 1.0);
    screen.open(Overlay { through: true, ..Overlay::new(Box::new(Probe::new(&list_seen)), Rect::new(0.0, 0.0, 200.0, 100.0)) });
    take(&under_seen);
    screen.click(300.0, 150.0);
    assert!(screen.overlays().is_empty());
    assert!(take(&under_seen).contains(&"click".to_string()));
}

#[test]
fn a_modal_overlay_keeps_the_pointer_from_what_is_under() {
    let under_seen = log();
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(Probe::new(&under_seen)), GridItem::new(1, 1, 1, 4, false));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    screen.resize(400.0, 200.0, 1.0);
    screen.open(Overlay { modal: true, ..Overlay::new(Box::new(Plate::new("panel", None)), Rect::new(300.0, 150.0, 20.0, 20.0)) });
    assert_eq!(None, screen.at(10.0, 10.0));
    screen.click(10.0, 10.0);
    // A press outside a dialog does not close it.
    assert_eq!(1, screen.overlays().len());
}

#[test]
fn an_overlay_is_drawn_over_the_window() {
    let mut screen = Screen::new(Box::new(GridPanel::new(1)), ThemeLook::clarity(false));
    screen.open(Overlay::new(Box::new(Plate::new("accent", None)), Rect::new(10.0, 10.0, 50.0, 50.0)));
    let mut shot = Shot::of(&mut screen, 200, 100, 1.0);
    assert_eq!(screen.look().color("accent"), shot.pixel(30, 30));
    assert_eq!(screen.look().color("background"), shot.pixel(150, 80));
}

#[test]
fn an_overlay_that_covers_follows_the_screen_s_size() {
    let mut screen = Screen::new(Box::new(GridPanel::new(1)), ThemeLook::clarity(false));
    screen.resize(200.0, 100.0, 1.0);
    screen.open(Overlay::dialog(Box::new(Plate::new("panel", None))));
    assert_eq!(Rect::new(0.0, 0.0, 200.0, 100.0), screen.overlays()[0].place);
    screen.resize(300.0, 150.0, 1.0);
    assert_eq!(Rect::new(0.0, 0.0, 300.0, 150.0), screen.overlays()[0].place);
}

#[test]
fn text_is_copied_and_pasted_through_the_host_s_clipboard() {
    let mut screen = Screen::new(Box::new(GridPanel::new(1)), ThemeLook::clarity(false));
    let root = screen.root().node().id();
    screen.act(root, |_, cx| cx.clipboard().set_text("kept"));
    assert_eq!(Some("kept".to_string()), screen.clipboard_text());
}

#[test]
fn an_element_hears_when_it_gets_and_loses_the_focus() {
    let (mut screen, left, right, a, b) = two_probes();
    screen.focus(Some(left));
    screen.focus(Some(right));
    assert_eq!(vec!["focus true", "focus false"], take(&a));
    assert_eq!(vec!["focus true"], take(&b));
}

#[test]
fn tab_goes_round_the_elements_that_take_the_focus_and_shift_tab_back() {
    let (mut screen, left, right, _, _) = two_probes();
    screen.key_pressed(KeyInput::plain(Key::Tab));
    assert_eq!(Some(left), screen.focused());
    screen.key_pressed(KeyInput::plain(Key::Tab));
    assert_eq!(Some(right), screen.focused());
    screen.key_pressed(KeyInput::plain(Key::Tab));
    assert_eq!(Some(left), screen.focused());
    screen.key_pressed(KeyInput::new(Key::Tab, Modifiers::SHIFT));
    assert_eq!(Some(right), screen.focused());
}

#[test]
fn an_element_inside_a_hidden_one_takes_no_tab_no_pointer_and_draws_nothing() {
    let seen = log();
    let mut grid = GridPanel::new(1);
    let first = Probe::new(&seen);
    let first_id = first.node().id();
    let hidden = Holder::new(Box::new(Plate::new("accent", None)));
    let hidden_id = hidden.node().id();
    grid.add(Box::new(first), GridItem::new(1, 1, 1, 1, false));
    grid.add(Box::new(HiddenBox { inner: hidden }), GridItem::new(1, 2, 1, 2, false));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    let mut shot = Shot::of(&mut screen, 400, 200, 1.0);
    screen.key_pressed(KeyInput::plain(Key::Tab));
    screen.key_pressed(KeyInput::plain(Key::Tab));
    assert_eq!(Some(first_id), screen.focused());
    assert!(!screen.shows(hidden_id));
    assert_eq!(screen.look().color("background"), shot.pixel(100, 80));
    assert_ne!(Some(hidden_id), screen.at(100.0, 80.0));
}

/// A box that does not show, holding what it holds.
struct HiddenBox {
    inner: Holder,
}

impl Element for HiddenBox {
    fn node(&self) -> &Node {
        self.inner.node()
    }
    fn node_mut(&mut self) -> &mut Node {
        self.inner.node_mut()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn is_shown(&self) -> bool {
        false
    }
}

#[test]
fn keys_go_to_the_element_with_the_focus_first_then_the_hook() {
    let (mut screen, left, _, seen, _) = two_probes();
    screen.focus(Some(left));
    take(&seen);
    let hooked = Rc::new(RefCell::new(Vec::new()));
    let kept = hooked.clone();
    screen.set_keys(Box::new(move |key, _| {
        kept.borrow_mut().push(key.key);
        key.control()
    }));
    assert!(screen.key_pressed(KeyInput::new(Key::S, Modifiers::CONTROL)));
    assert!(!screen.key_pressed(KeyInput::plain(Key::A)));
    assert_eq!(vec!["key S", "key A"], take(&seen));
    assert_eq!(vec![Key::S, Key::A], *hooked.borrow());
    // The character of a key nothing took is asked of the window's keys first, then typed.
    screen.text_typed("a");
    assert_eq!(vec![Key::S, Key::A, Key::Other], *hooked.borrow());
    assert_eq!(vec!["text a"], take(&seen));
}

#[test]
fn an_element_wakes_at_the_time_it_asked_for() {
    let (mut screen, left, _, seen, _) = two_probes();
    let start = Instant::now();
    screen.act(left, |e, cx| cx.wake(e.node().id(), start + Duration::from_secs(1)));
    assert_eq!(Some(start + Duration::from_secs(1)), screen.next_wake());
    screen.wake_due(start);
    assert!(take(&seen).is_empty());
    screen.wake_due(start + Duration::from_secs(2));
    assert_eq!(vec!["wake"], take(&seen));
    assert_eq!(None, screen.next_wake());
}

#[test]
fn a_tooltip_shows_after_the_pointer_rests_and_goes_on_a_press() {
    let seen = log();
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(Probe { tip: Some("Saves the order".to_string()), ..Probe::new(&seen) }), GridItem::new(1, 1, 1, 1, false));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    screen.resize(400.0, 200.0, 1.0);
    let start = Instant::now();
    screen.now = Box::new(move || start);
    screen.pointer_moved(20.0, 10.0);
    assert!(screen.tip().is_none());
    let due = screen.next_wake().unwrap();
    assert_eq!(start + clarity_ui::screen::TIP_PAUSE, due);
    screen.wake_due(due);
    assert_eq!("Saves the order", screen.tip().unwrap().text);
    screen.pointer_pressed(20.0, 10.0, PointerButton::Left, 1);
    assert!(screen.tip().is_none());
}

#[test]
fn elements_join_the_screen_and_leave_it_with_its_overlays() {
    let seen = log();
    let mut screen = Screen::new(Box::new(GridPanel::new(1)), ThemeLook::clarity(false));
    let id = screen.open(Overlay::new(Box::new(Probe::new(&seen)), Rect::new(0.0, 0.0, 10.0, 10.0)));
    assert_eq!(vec!["attached"], take(&seen));
    screen.close(id);
    assert_eq!(vec!["detached"], take(&seen));
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(Probe::new(&seen)), GridItem::new(1, 1, 1, 1, false));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    assert_eq!(vec!["attached"], take(&seen));
    screen.dispose();
    assert_eq!(vec!["detached"], take(&seen));
}

#[test]
fn the_screen_draws_at_its_scale() {
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(Plate::new("accent", None)), GridItem::new(1, 1, 1, 1, false));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    // At scale 2 a row of 36 units is 72 pixels high.
    let mut shot = Shot::of(&mut screen, 400, 200, 2.0);
    assert_eq!(200.0, screen.width());
    assert_eq!(screen.look().color("accent"), shot.pixel(10, 70));
    assert_eq!(screen.look().color("background"), shot.pixel(10, 76));
}

#[test]
fn a_change_asks_the_host_to_draw_again() {
    let (mut screen, left, _, _, _) = two_probes();
    let mut surface = skia_safe::surfaces::raster_n32_premul((400, 200)).unwrap();
    screen.draw(surface.canvas());
    assert!(!screen.take_redraw());
    screen.focus(Some(left));
    assert!(screen.take_redraw());
    assert!(!screen.take_redraw());
}

#[test]
fn finding_elements_in_a_tree() {
    let (screen, left, right, _, _) = two_probes();
    let all: Vec<ElementId> = element::in_order(screen.root()).iter().map(|e| e.node().id()).collect();
    assert_eq!(vec![screen.root().node().id(), left, right], all);
    assert_eq!(Some(vec![screen.root().node().id(), right]), element::chain(screen.root(), right));
    assert!(screen.find::<Probe>().is_some());
    assert_eq!(Some(left), element::find_type::<Probe>(screen.root(), |_| true).map(|p| p.node().id()));
}
