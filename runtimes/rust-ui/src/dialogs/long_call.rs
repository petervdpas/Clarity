//! A long call (DESIGN 5.12): while a handler runs off the screen's thread,
//! the window keeps drawing and, once the call says how far it is, shows
//! that as a note at the bottom right, clear of the messages' notes at the
//! top: a bar, how much is done, and Stop, which ends the call at its next
//! look and then greys, as it can stop it only once. It lies over the whole
//! window from the start, so nothing under it is pressed while the call
//! runs, and no other handler runs meanwhile. The bar asks the call how far
//! it is each time it looks again and tells it Stop (`watching`): plain
//! calls, so a program of any language answers them.

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::dialogs::DialogButton;
use crate::element::{Cx, Element, Node};
use crate::geometry::Rect;
use crate::grid::ROW_HEIGHT;
use crate::input::{Key, KeyInput, Modifiers};
use crate::locale;
use crate::painter::{Painter, TextAlign};
use crate::window::view::CONTENT_PADDING;

const WIDTH: f64 = 360.0;
const HEIGHT: f64 = 64.0;
const BUTTON_WIDTH: f64 = 88.0;

/// How often the bar looks again at how far the call is.
const LOOK_AGAIN: Duration = Duration::from_millis(100);

/// How far a call is, as it last said: done of total; none while it said nothing.
pub type Said = Arc<Mutex<Option<(i64, i64)>>>;

/// How far a call is, asked of it: done of total, none while it said nothing.
pub type AskProgress = Rc<dyn Fn() -> Option<(i64, i64)>>;

pub struct LongCallBar {
    node: Node,
    said: Said,
    stop: Arc<AtomicBool>,
    /// Asked each time the bar looks; none when `said` is told from elsewhere.
    asks: Option<AskProgress>,
    note: Rect,
}

impl LongCallBar {
    /// A bar that shows how far `said` says the call is; Stop sets `stop`, once.
    pub fn new(said: Said, stop: Arc<AtomicBool>) -> LongCallBar {
        LongCallBar::made(said, stop, None, None)
    }

    /// A bar that asks the call how far it is (`progress`) each time it looks,
    /// and tells it `stop` once when Stop is pressed.
    pub fn watching(progress: AskProgress, stop: Rc<dyn Fn()>) -> LongCallBar {
        LongCallBar::made(Arc::new(std::sync::Mutex::new(None)), Arc::new(AtomicBool::new(false)), Some(progress), Some(stop))
    }

    fn made(said: Said, stop: Arc<AtomicBool>, asks: Option<AskProgress>, tells: Option<Rc<dyn Fn()>>) -> LongCallBar {
        let mut node = Node::new();
        let (sets, reads) = (stop.clone(), stop.clone());
        let button = DialogButton::new(locale::translate("stopCall", "Stop"), false, Rc::new(move |cx: &mut Cx| {
            if !sets.swap(true, Ordering::SeqCst) {
                if let Some(tell) = &tells {
                    tell();
                }
            }
            cx.invalidate();
        }))
        .usable_while(Rc::new(move || !reads.load(Ordering::SeqCst)));
        node.add(Box::new(button));
        LongCallBar { node, said, stop, asks, note: Rect::default() }
    }


    /// Whether Stop was pressed: it is then greyed and stops nothing more.
    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    fn now(&self) -> Option<(i64, i64)> {
        match &self.asks {
            Some(asks) => asks(),
            None => self.said.lock().ok().and_then(|s| *s),
        }
    }

    /// Whether the call said how far it is: only then the note shows.
    pub fn says(&self) -> bool {
        self.now().is_some()
    }

    /// How much is done, 0 to 1; 0 while the call says nothing.
    pub fn fraction(&self) -> f64 {
        match self.now() {
            Some((done, total)) if total > 0 => (done as f64 / total as f64).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }

    /// How much is done, as it shows: `25%`; nothing while the call says nothing.
    pub fn said_text(&self) -> String {
        match self.now() {
            Some((_, total)) if total > 0 => format!("{}%", (self.fraction() * 100.0).round()),
            _ => String::new(),
        }
    }

    /// Where the note is, at the bottom right of the window.
    pub fn note(&self) -> Rect {
        self.note
    }

    /// Stop at its place in the note once the call says how far it is; until then where nothing presses it.
    fn place_stop(&mut self, cx: &mut Cx) {
        let stop = if self.says() {
            Rect::new(self.note.right() - 12.0 - BUTTON_WIDTH, self.note.y + (HEIGHT - ROW_HEIGHT) / 2.0, BUTTON_WIDTH, ROW_HEIGHT)
        } else {
            Rect::new(-10_000.0, -10_000.0, 0.0, 0.0)
        };
        self.node.children_mut()[0].arrange(stop, cx);
    }

    /// Stop, as the person presses it.
    pub fn stop_button(&self) -> &DialogButton {
        self.node.children()[0].as_any().downcast_ref::<DialogButton>().expect("the bar holds its Stop")
    }
}

impl Element for LongCallBar {
    crate::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        let width = WIDTH.min((area.width - 2.0 * CONTENT_PADDING).max(160.0));
        // At the bottom right, so the window's info notes at the top right stay readable while a call runs.
        self.note = Rect::new(area.right() - CONTENT_PADDING - width, area.bottom() - CONTENT_PADDING - HEIGHT, width, HEIGHT);
        self.place_stop(cx);
    }

    fn draw(&self, p: &Painter) {
        if !self.says() {
            return;
        }
        let look = p.look;
        let radius = look.radius("panel");
        let note = self.note;
        p.fill(note, look.color("raised"), radius);
        p.stroke(note, look.color("border"), look.border(), radius);
        p.fill(Rect { width: 4.0, ..note }, look.color("accent"), radius);
        let stop = self.node.children()[0].node().bounds;
        let track = Rect::new(note.x + 16.0, note.y + HEIGHT / 2.0 - 4.0, (stop.x - 16.0 - 56.0 - note.x - 16.0).max(0.0), 8.0);
        p.fill(track, look.color("border"), 4.0);
        let fraction = self.fraction();
        if fraction > 0.0 {
            p.fill(Rect { width: track.width * fraction, ..track }, look.color("accent"), 4.0);
        }
        p.text(&self.said_text(), Rect::new(track.right() + 8.0, note.y, 48.0, HEIGHT), look.color("muted"), look.size("small"), "text", false, TextAlign::Left);
        crate::element::draw_children(&self.node, p);
    }

    fn on_attached(&mut self, cx: &mut Cx) {
        cx.wake(self.node.id(), cx.now() + LOOK_AGAIN);
    }

    /// It looks again at how far the call is, and draws that.
    fn on_wake(&mut self, cx: &mut Cx) {
        // Once the call says how far it is, Stop takes its place at once.
        self.place_stop(cx);
        cx.invalidate();
        cx.wake(self.node.id(), cx.now() + LOOK_AGAIN);
    }

    /// As a screen's root (a test, a host of its own), the same keys.
    fn window_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.dialog_key(key, cx)
    }

    /// Escape stops, as Stop does.
    fn dialog_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.modifiers != Modifiers::NONE || key.key != Key::Escape {
            return true;
        }
        if let Some(stop) = self.node.children_mut()[0].as_any_mut().downcast_mut::<DialogButton>() {
            stop.press(cx);
        }
        true
    }
}
