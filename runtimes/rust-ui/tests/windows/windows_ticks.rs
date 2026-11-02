//! The clock the parts tick by (C# FrameTicksTests): in Rust it is the
//! screen's wake-ups (`Cx::wake`, `Screen::next_wake`, `wake_due`), on the
//! screen's thread. A clock runs when its time has come at a frame; time
//! lost in a long frame is not made up; a clock that left the screen runs no
//! more and the host's loop rests; a tick that fails is said and the others
//! run on; a clock started by a tick waits for its own time.

use super::core_windows_fake;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clarity_runtime::program::Program;
use clarity_ui::element::{Cx, Element, Node};
use clarity_ui::window::WindowStack;
use clarity_ui::{AppFiles, Painter, Rect, Screen, ThemeLook};
use core_windows_fake::*;

/// A clock as a part keeps one: every `every`, measured from its last run; it can start another on its first run.
struct Clock {
    node: Node,
    every: Duration,
    ran: Rc<Cell<u32>>,
    starts: Option<Rc<Cell<u32>>>,
}

impl Clock {
    fn new(every_ms: u64, ran: Rc<Cell<u32>>) -> Clock {
        Clock { node: Node::new(), every: Duration::from_millis(every_ms), ran, starts: None }
    }
}

impl Element for Clock {
    clarity_ui::element_node!();

    fn arrange(&mut self, area: Rect, _cx: &mut Cx) {
        self.node.bounds = area;
    }

    fn draw(&self, _p: &Painter) {}

    fn on_attached(&mut self, cx: &mut Cx) {
        let at = cx.now() + self.every;
        cx.wake(self.node.id(), at);
    }

    fn on_wake(&mut self, cx: &mut Cx) {
        self.ran.set(self.ran.get() + 1);
        if let Some(inner) = self.starts.take() {
            self.node.add(Box::new(Clock::new(100, inner)));
        }
        let at = cx.now() + self.every;
        cx.wake(self.node.id(), at);
    }
}

/// A screen whose clock the test turns: `at(ms)` is that long after the start.
struct Turned {
    screen: Screen,
    start: Instant,
    now: Rc<RefCell<Instant>>,
}

impl Turned {
    /// `every` is how often the first clock runs: the start is when it started, as its first run says.
    fn new(root: Box<dyn Element>, every_ms: u64) -> Turned {
        let now = Rc::new(RefCell::new(Instant::now()));
        let clock = now.clone();
        let mut screen = Screen::new(root, ThemeLook::clarity(false));
        screen.now = Box::new(move || *clock.borrow());
        screen.resize(200.0, 100.0, 1.0);
        let start = screen.next_wake().expect("a clock waits") - Duration::from_millis(every_ms);
        Turned { screen, start, now }
    }

    /// A frame at `ms` after the start: what is due runs.
    fn frame(&mut self, ms: u64) {
        let at = self.start + Duration::from_millis(ms);
        *self.now.borrow_mut() = at;
        self.screen.wake_due(at);
    }
}

#[test]
fn a_clock_runs_when_its_time_has_come_at_a_frame() {
    let ran = Rc::new(Cell::new(0));
    let mut t = Turned::new(Box::new(Clock::new(500, ran.clone())), 500);
    assert!(t.screen.next_wake().is_some());
    t.frame(400);
    assert_eq!(0, ran.get());
    t.frame(516);
    assert_eq!(1, ran.get());
    t.frame(900);
    assert_eq!(1, ran.get());
    t.frame(1016);
    assert_eq!(2, ran.get());
}

#[test]
fn time_lost_in_a_long_frame_is_not_made_up() {
    let ran = Rc::new(Cell::new(0));
    let mut t = Turned::new(Box::new(Clock::new(100, ran.clone())), 100);
    // A frame long after: it runs once, not once for each tenth it missed.
    t.frame(1000);
    assert_eq!(1, ran.get());
    t.frame(1050);
    assert_eq!(1, ran.get());
    t.frame(1100);
    assert_eq!(2, ran.get());
}

#[test]
fn a_clock_that_left_the_screen_runs_no_more_and_the_loop_rests() {
    let ran = Rc::new(Cell::new(0));
    let mut t = Turned::new(Box::new(Clock::new(100, ran.clone())), 100);
    t.screen.dispose();
    // Nothing waits: the host's loop rests.
    assert_eq!(None, t.screen.next_wake());
    t.frame(500);
    assert_eq!(0, ran.get());
}

#[test]
fn a_clock_started_by_a_tick_waits_for_its_own_time() {
    let (outer, inner) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let mut clock = Clock::new(100, outer.clone());
    clock.starts = Some(inner.clone());
    let mut t = Turned::new(Box::new(clock), 100);
    t.frame(100);
    assert_eq!((1, 0), (outer.get(), inner.get()));
    t.frame(200);
    assert_eq!((2, 1), (outer.get(), inner.get()));
}

#[test]
fn a_tick_that_fails_is_said_and_the_others_run_on() {
    const TWO: &str = r#"<Window name="Main">
      <Timer name="bad" at="1,1" ontick="refuse" />
      <Timer name="good" at="1,1" ontick="tick" />
    </Window>"#;
    const TWO_FILES: AppFiles = AppFiles { windows: &[("Main", TWO)], ..FILES };
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let mut t = Turned::new(Box::new(WindowStack::start(program, TWO_FILES)), 1000);
    fake.calls();
    t.frame(1000);
    let calls = fake.calls();
    assert!(calls.contains(&"dispatch refuse".to_string()) && calls.contains(&"dispatch tick".to_string()), "{calls:?}");
    // The one that failed says why in Clarity's dialog; the window goes on.
    t.screen.act_root::<WindowStack, _>(|_, _| ());
    let stack = t.screen.root().as_any().downcast_ref::<WindowStack>().unwrap();
    assert_eq!(vec!["A title first"], stack.said().iter().map(|m| m.text.show()).collect::<Vec<_>>());
    t.frame(2000);
    assert_eq!(2, fake.calls().len());
}
