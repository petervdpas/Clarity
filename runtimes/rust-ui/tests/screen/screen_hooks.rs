//! What the screen tells its elements beyond the pointer: keys let go, the
//! keys held while the wheel turns, joining and leaving the screen, a timed
//! wake-up, and a tooltip after the pointer rests. A dialog on top hears keys
//! before the part with the focus, and an overlay that covers the screen
//! follows its size.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use clarity_ui::element::{Cx, Element, Node};
use clarity_ui::element_node;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::overlay::Overlay;
use clarity_ui::screen::TIP_PAUSE;
use clarity_ui::testing::Shot;
use clarity_ui::{Rect, Screen, ThemeLook};

type Seen = Rc<RefCell<Vec<String>>>;

struct Hooked {
    node: Node,
    seen: Seen,
    tip: Rc<RefCell<Option<String>>>,
}

impl Hooked {
    fn new() -> (Hooked, Seen) {
        let seen = Seen::default();
        (Hooked { node: Node::new(), seen: seen.clone(), tip: Rc::default() }, seen)
    }

    fn say(&self, what: String) {
        self.seen.borrow_mut().push(what);
    }
}

impl Element for Hooked {
    element_node!();

    fn focusable(&self) -> bool {
        true
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        for child in self.node.children_mut() {
            child.arrange(area.inset(10.0), cx);
        }
    }

    fn on_attached(&mut self, _cx: &mut Cx) {
        self.say("joined".into());
    }

    fn on_detached(&mut self, _cx: &mut Cx) {
        self.say("left".into());
    }

    fn on_key_up(&mut self, key: KeyInput, _cx: &mut Cx) {
        self.say(format!("up {:?}", key.key));
    }

    fn on_key(&mut self, key: KeyInput, _cx: &mut Cx) -> bool {
        self.say(format!("down {:?}", key.key));
        true
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, held: Modifiers, _cx: &mut Cx) -> bool {
        self.say(format!("wheel {dy} {}", if held.has(Modifiers::CONTROL) { "Control" } else { "None" }));
        true
    }

    fn on_wake(&mut self, _cx: &mut Cx) {
        self.say("woke".into());
    }

    fn tooltip_at(&self, _x: f64, _y: f64) -> Option<String> {
        self.tip.borrow().clone()
    }
}

/// A dialog that hears every key first and takes Escape.
struct KeyDialog {
    node: Node,
    heard: Rc<RefCell<Vec<Key>>>,
}

impl Element for KeyDialog {
    element_node!();

    fn dialog_key(&mut self, key: KeyInput, _cx: &mut Cx) -> bool {
        self.heard.borrow_mut().push(key.key);
        key.key == Key::Escape
    }
}

fn shown() -> (Screen, Seen, Rc<RefCell<Option<String>>>) {
    let (root, seen) = Hooked::new();
    let tip = root.tip.clone();
    let mut screen = Screen::new(Box::new(root), ThemeLook::clarity(false));
    screen.resize(400.0, 300.0, 1.0);
    (screen, seen, tip)
}

fn root_id(screen: &Screen) -> u64 {
    screen.root().node().id()
}

fn seen(log: &Seen) -> Vec<String> {
    log.borrow().clone()
}

#[test]
fn a_key_let_go_reaches_the_focused_element_and_modifier_keys_are_keys() {
    let (mut screen, log, _) = shown();
    screen.focus(Some(root_id(&screen)));
    screen.key_pressed(KeyInput::new(Key::Shift, Modifiers::SHIFT));
    screen.key_released(KeyInput::plain(Key::Shift));
    screen.key_released(KeyInput::plain(Key::Control));
    screen.key_released(KeyInput::plain(Key::Alt));
    assert_eq!(seen(&log), vec!["joined", "down Shift", "up Shift", "up Control", "up Alt"]);
}

#[test]
fn the_wheel_carries_the_keys_held() {
    let (mut screen, log, _) = shown();
    screen.wheel(50.0, 50.0, 0.0, 40.0, Modifiers::CONTROL);
    assert_eq!(seen(&log).last().map(String::as_str), Some("wheel 40 Control"));
}

#[test]
fn an_element_hears_when_it_joins_and_leaves_its_screen() {
    let (mut screen, _, _) = shown();
    let (mut inner, inner_seen) = Hooked::new();
    let (deeper, deeper_seen) = Hooked::new();
    let deeper_id = deeper.node.id();
    inner.node.add(Box::new(deeper));
    assert!(seen(&inner_seen).is_empty());
    let root = root_id(&screen);
    screen.act(root, |e, _| e.node_mut().add(Box::new(inner)));
    assert_eq!(seen(&inner_seen), vec!["joined"]);
    assert_eq!(seen(&deeper_seen), vec!["joined"]);
    assert!(screen.element(deeper_id).unwrap().node().attached());
    let removed = screen.act(root, |e, cx| e.node_mut().remove(0, cx)).unwrap();
    assert_eq!(seen(&deeper_seen), vec!["joined", "left"]);
    assert!(!removed.node().children()[0].node().attached());
    // An overlay's content joins as it opens and leaves as it closes.
    let (over, over_seen) = Hooked::new();
    let id = screen.open(Overlay::new(Box::new(over), Rect::new(0.0, 0.0, 50.0, 50.0)));
    screen.close(id);
    assert_eq!(seen(&over_seen), vec!["joined", "left"]);
}

#[test]
fn a_screen_disposed_closes_its_overlays_and_its_elements_leave_it() {
    let (mut screen, log, _) = shown();
    let (over, over_seen) = Hooked::new();
    screen.open(Overlay::new(Box::new(over), Rect::new(0.0, 0.0, 10.0, 10.0)));
    screen.dispose();
    assert!(screen.overlays().is_empty());
    assert_eq!(seen(&log), vec!["joined", "left"]);
    assert_eq!(seen(&over_seen), vec!["joined", "left"]);
}

#[test]
fn an_element_wakes_once_at_its_time_and_not_after_it_left() {
    let (mut screen, log, _) = shown();
    let now = Instant::now();
    screen.now = Box::new(move || now);
    let root = root_id(&screen);
    let second = now + Duration::from_secs(1);
    screen.act(root, |_, cx| cx.wake(root, second));
    assert_eq!(screen.next_wake(), Some(second));
    screen.wake_due(now);
    assert!(!seen(&log).contains(&"woke".to_string()));
    screen.wake_due(second);
    screen.wake_due(now + Duration::from_secs(2));
    assert_eq!(seen(&log).iter().filter(|s| *s == "woke").count(), 1);
    assert_eq!(screen.next_wake(), None);
    let (inner, inner_seen) = Hooked::new();
    let inner_id = inner.node.id();
    screen.act(root, |e, _| e.node_mut().add(Box::new(inner)));
    screen.act(root, |_, cx| cx.wake(inner_id, now));
    screen.act(root, |e, cx| e.node_mut().remove(0, cx));
    screen.wake_due(now);
    assert!(!seen(&inner_seen).contains(&"woke".to_string()));
}

#[test]
fn a_tooltip_shows_after_the_pointer_rests_and_goes_when_it_moves_off_or_presses() {
    let (mut screen, _, tip) = shown();
    let now = Instant::now();
    screen.now = Box::new(move || now);
    *tip.borrow_mut() = Some("Save (Ctrl+S)".into());
    screen.pointer_moved(100.0, 100.0);
    assert!(screen.tip().is_none());
    assert_eq!(screen.next_wake(), Some(now + TIP_PAUSE));
    screen.wake_due(now + TIP_PAUSE);
    assert_eq!(screen.tip().map(|t| t.text.as_str()), Some("Save (Ctrl+S)"));
    assert!(screen.tip().unwrap().place.y > 100.0);
    // Moving over the same place keeps it; pressing hides it.
    screen.pointer_moved(102.0, 100.0);
    assert!(screen.tip().is_some());
    screen.pointer_pressed(102.0, 100.0, PointerButton::Left, 1);
    assert!(screen.tip().is_none());
    screen.pointer_released(102.0, 100.0, PointerButton::Left);
    // Over nothing that says something, none waits.
    *tip.borrow_mut() = None;
    screen.pointer_moved(150.0, 100.0);
    assert_eq!(screen.next_wake(), None);
    // It draws in the theme without failing.
    *tip.borrow_mut() = Some("Zoom in".into());
    screen.pointer_moved(200.0, 100.0);
    screen.wake_due(now + TIP_PAUSE);
    Shot::of(&mut screen, 400, 300, 1.0);
    screen.pointer_left();
    assert!(screen.tip().is_none());
}

#[test]
fn a_press_starts_no_new_pause() {
    let (mut screen, _, tip) = shown();
    let now = Instant::now();
    screen.now = Box::new(move || now);
    *tip.borrow_mut() = Some("Users".into());
    screen.pointer_moved(100.0, 100.0);
    screen.pointer_pressed(100.0, 100.0, PointerButton::Left, 1);
    screen.pointer_released(100.0, 100.0, PointerButton::Left);
    screen.wake_due(now + TIP_PAUSE + TIP_PAUSE);
    assert!(screen.tip().is_none());
    // Resting on the same place after it starts no new pause until the pointer goes elsewhere.
    screen.pointer_moved(101.0, 100.0);
    assert_eq!(screen.next_wake(), None);
}

#[test]
fn what_is_under_the_pointer_no_more_shows_no_tip() {
    let (mut screen, _, tip) = shown();
    let now = Instant::now();
    screen.now = Box::new(move || now);
    *tip.borrow_mut() = Some("Users".into());
    screen.pointer_moved(100.0, 100.0);
    // Something else shows under the pointer before the pause is over.
    *tip.borrow_mut() = None;
    screen.wake_due(now + TIP_PAUSE);
    assert!(screen.tip().is_none());
}

#[test]
fn a_dialog_on_top_hears_keys_before_the_focused_element() {
    let (mut screen, log, _) = shown();
    let heard = Rc::new(RefCell::new(Vec::new()));
    let dialog = KeyDialog { node: Node::new(), heard: heard.clone() };
    let mut overlay = Overlay::new(Box::new(dialog), screen.whole());
    overlay.modal = true;
    overlay.covers = true;
    screen.open(overlay);
    screen.focus(Some(root_id(&screen)));
    screen.key_pressed(KeyInput::plain(Key::Escape));
    screen.key_pressed(KeyInput::plain(Key::A));
    assert_eq!(*heard.borrow(), vec![Key::Escape, Key::A]);
    // The dialog took Escape; A went on to the element with the focus.
    assert_eq!(seen(&log), vec!["joined", "down A"]);
}

#[test]
fn an_overlay_that_covers_the_screen_follows_its_size() {
    let (mut screen, _, _) = shown();
    let (over, _) = Hooked::new();
    let mut overlay = Overlay::new(Box::new(over), Rect::new(0.0, 0.0, 10.0, 10.0));
    overlay.covers = true;
    screen.open(overlay);
    assert_eq!(screen.overlays()[0].place, Rect::new(0.0, 0.0, 400.0, 300.0));
    screen.resize(640.0, 480.0, 1.0);
    assert_eq!(screen.overlays()[0].place, Rect::new(0.0, 0.0, 640.0, 480.0));
}
