//! A window's screen: its elements, drawn in the theme on whatever the host
//! gives (the desktop's window, a picture in a test), and what the person
//! does sent to the element it is for. The host draws again when the screen
//! says it changed (`take_redraw`), on every frame while an element animates
//! (`animating`), and at the time an element asked to wake (`next_wake`,
//! then `wake_due`). Every host sends the pointer, the wheel, keys going down
//! and up and typed text through the same methods here.

mod focus;
mod overlays;
mod pointer;
mod tips;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use skia_safe::Canvas;

use crate::element::{self, Composition, Cx, Element, ElementId, FocusedPart, Request};
use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::input::{Cursor, Key, KeyInput, Modifiers, PointerButton, PointerInput};
use crate::overlay::{Clipboard, OwnClipboard, Overlay};
use crate::painter::{Painter, TextAlign};
use crate::theme::ThemeLook;

/// How long the pointer rests before a tooltip shows.
pub const TIP_PAUSE: Duration = Duration::from_millis(600);

const TIP_PAD: f64 = 6.0;

/// A tooltip as it shows: its text and its place.
#[derive(Debug, Clone, PartialEq)]
pub struct Tooltip {
    pub text: String,
    pub place: Rect,
}

/// The window's own keys, asked before the part with the focus; true when it took the key.
pub type KeysHook = Box<dyn FnMut(KeyInput, &mut Cx) -> bool>;

/// The look for the system's light (false) or dark (true), in the theme the person chose (`choices`).
pub type LooksFor = Box<dyn Fn(bool) -> Arc<ThemeLook>>;

/// A press that may ask for something: the main button at a place, or a key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Press {
    At(f64, f64),
    Key(KeyInput),
}

pub struct Screen {
    root: Box<dyn Element>,
    /// The part the focus was last found in, by the element with the focus (`Cx::focused_part`).
    focus_part: RefCell<(Option<ElementId>, Option<FocusedPart>)>,
    look: Arc<ThemeLook>,
    fonts: Rc<Fonts>,
    clipboard: Rc<RefCell<Box<dyn Clipboard>>>,
    width: f64,
    height: f64,
    scale: f32,
    focused: Option<ElementId>,
    hovered: Option<ElementId>,
    pressed: Option<ElementId>,
    /// The button that pressed it: the right one for a part that takes right drags.
    held: PointerButton,
    overlays: Vec<Overlay>,
    wakes: HashMap<ElementId, Instant>,
    animating: HashSet<ElementId>,
    redraw: bool,
    wakes_changed: bool,
    /// Disposed: its tree left the screen for good and joins it no more (C# FrameTicksTests).
    disposed: bool,
    keys: Option<KeysHook>,
    tip: Option<Tooltip>,
    tip_waiting: Option<String>,
    tip_due: Option<Instant>,
    tip_point: (f64, f64),
    later: crate::later::Queue,
    places: Rc<RefCell<HashMap<String, (ElementId, Rect)>>>,
    /// The strips the handles beside parts took in the last layout, which the parts under them make room for.
    strips: Vec<Rect>,
    looks: Option<LooksFor>,
    dark: bool,
    choices_seen: u64,
    cursor: Cursor,
    cursor_changed: bool,
    context_menus: crate::frame::menus::FrameMenus,
    /// Whether something took the last key pressed; the character it types is then no shortcut.
    key_taken: bool,
    pointing: bool,
    /// The last press that may have asked for something: the main button at
    /// a place, or a key (`Screen::last_press`).
    last_press: Option<Press>,
    /// The time now, as the screen's clock tells it; a test sets its own.
    pub now: Box<dyn Fn() -> Instant>,
}

impl Screen {
    /// The last press that may have asked for something, from whatever
    /// pressed it (the pointer, a key, a driven step, a screen reader): what a
    /// host does again when an answer comes later than the part could wait
    /// (a file the page gives), so the part that asked takes it.
    pub fn last_press(&self) -> Option<Press> {
        self.last_press
    }

    /// Does `press` again, as it was done.
    pub fn press_again(&mut self, press: Press) {
        match press {
            Press::At(x, y) => {
                self.pointer_moved(x, y);
                self.click(x, y);
            }
            Press::Key(key) => {
                self.key_pressed(key);
                self.key_released(key);
            }
        }
    }

    pub fn new(root: Box<dyn Element>, look: Arc<ThemeLook>) -> Screen {
        let fonts = Rc::new(Fonts::new(look.clone()));
        let mut screen = Screen {
            root,
            focus_part: RefCell::new((None, None)),
            look,
            fonts,
            clipboard: Rc::new(RefCell::new(Box::new(OwnClipboard::default()))),
            width: 0.0,
            height: 0.0,
            scale: 1.0,
            focused: None,
            hovered: None,
            pressed: None,
            overlays: Vec::new(),
            wakes: HashMap::new(),
            animating: HashSet::new(),
            redraw: true,
            wakes_changed: false,
            disposed: false,
            keys: None,
            tip: None,
            tip_waiting: None,
            tip_due: None,
            tip_point: (0.0, 0.0),
            later: crate::later::Queue::new(),
            places: Rc::default(),
            strips: Vec::new(),
            looks: None,
            dark: false,
            choices_seen: crate::choices::changes(),
            cursor: Cursor::Default,
            cursor_changed: false,
            context_menus: crate::frame::menus::FrameMenus::new(),
            held: PointerButton::Left,
            key_taken: false,
            pointing: false,
            last_press: None,
            now: Box::new(Instant::now),
        };
        let mut cx = screen.cx();
        element::attach_all(screen.root.as_mut(), &mut cx);
        screen.settle(cx);
        screen
    }

    // -- What it is ------------------------------------------------------------

    pub fn root(&self) -> &dyn Element {
        self.root.as_ref()
    }

    pub fn root_mut(&mut self) -> &mut dyn Element {
        self.root.as_mut()
    }

    pub fn look(&self) -> &Arc<ThemeLook> {
        &self.look
    }

    pub fn fonts(&self) -> &Fonts {
        &self.fonts
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// The whole screen, where an overlay that covers it lies.
    pub fn whole(&self) -> Rect {
        Rect::new(0.0, 0.0, self.width, self.height)
    }

    pub fn focused(&self) -> Option<ElementId> {
        self.focused
    }

    pub fn hovered(&self) -> Option<ElementId> {
        self.hovered
    }

    /// What lies over the window, the top one last.
    pub fn overlays(&self) -> &[Overlay] {
        &self.overlays
    }

    /// The tooltip shown now; none while none shows.
    pub fn tip(&self) -> Option<&Tooltip> {
        self.tip.as_ref()
    }

    /// The shape the pointer shows now; each host shows it as its system's.
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// Whether the pointer's shape changed since the last time it was asked: the host shows `cursor`.
    pub fn take_cursor_changed(&mut self) -> bool {
        std::mem::take(&mut self.cursor_changed)
    }

    /// The menu a right press opened; none open while it has no lists.
    pub fn context_menus(&self) -> &crate::frame::menus::FrameMenus {
        &self.context_menus
    }

    /// Whether the focus moves because of a press of the pointer (not Tab or code): a scrolling window then leaves its scroll alone.
    pub fn pointing(&self) -> bool {
        self.pointing
    }

    /// Whether the person may use the element of that id: it and every element it is in are enabled.
    pub fn enabled(&self, id: ElementId) -> bool {
        self.mark();
        self.element(id).is_some_and(element::enabled_in_tree)
    }

    /// Marks in the window and each overlay which elements are in a disabled one.
    fn mark(&self) {
        element::mark_enabled(self.root.as_ref(), false);
        for overlay in &self.overlays {
            element::mark_enabled(overlay.content.as_ref(), false);
        }
    }

    /// How many elements animate now: while above zero the host draws every frame.
    pub fn animating(&self) -> usize {
        self.animating.len()
    }

    /// The element of that id, in an overlay or in the window.
    pub fn element(&self, id: ElementId) -> Option<&dyn Element> {
        self.overlays.iter().rev().find_map(|o| element::find(o.content.as_ref(), id)).or_else(|| element::find(self.root.as_ref(), id))
    }

    /// The element of that id, to change.
    pub fn element_mut(&mut self, id: ElementId) -> Option<&mut dyn Element> {
        let in_overlay = self.overlays.iter().rposition(|o| element::find(o.content.as_ref(), id).is_some());
        match in_overlay {
            Some(i) => element::find_mut(self.overlays[i].content.as_mut(), id),
            None => element::find_mut(self.root.as_mut(), id),
        }
    }

    /// The first element of type `T`, in the window or an overlay.
    pub fn find<T: Element>(&self) -> Option<&T> {
        element::find_type::<T>(self.root.as_ref(), |_| true).or_else(|| self.overlays.iter().rev().find_map(|o| element::find_type::<T>(o.content.as_ref(), |_| true)))
    }

    /// Whether the element of that id shows: it and every element it is in show.
    pub fn shows(&self, id: ElementId) -> bool {
        self.overlays.iter().any(|o| element::shows_in_tree(o.content.as_ref(), id)) || element::shows_in_tree(self.root.as_ref(), id)
    }

    /// The system's clipboard, once the host gives it.
    pub fn set_clipboard(&mut self, clipboard: Box<dyn Clipboard>) {
        *self.clipboard.borrow_mut() = clipboard;
    }

    pub fn clipboard_text(&self) -> Option<String> {
        self.clipboard.borrow_mut().get_text()
    }

    /// The window's own keys, asked before the part with the focus and before the root's own.
    pub fn set_keys(&mut self, keys: KeysHook) {
        self.keys = Some(keys);
    }

    // -- Drawing ---------------------------------------------------------------

    /// Whether something changed since the last time it was asked: the host draws again.
    pub fn take_redraw(&mut self) -> bool {
        std::mem::take(&mut self.redraw)
    }

    /// Whether the time of `next_wake` changed since the last time it was asked: a host that waits for it waits anew.
    pub fn take_wakes_changed(&mut self) -> bool {
        std::mem::take(&mut self.wakes_changed)
    }

    /// Something changed: the host draws again.
    pub fn invalidate(&mut self) {
        self.redraw = true;
    }

    pub fn resize(&mut self, width: f64, height: f64, scale: f32) {
        self.width = width;
        self.height = height;
        self.scale = if scale <= 0.0 { 1.0 } else { scale };
        let whole = self.whole();
        for overlay in self.overlays.iter_mut().filter(|o| o.covers) {
            overlay.place = whole;
        }
        self.arrange_root(whole);
        self.keep_places();
        self.redraw = true;
    }

    /// Lays out the window's parts in `whole`; when the handles beside parts
    /// took other strips than the parts made room for, once more with those.
    fn arrange_root(&mut self, whole: Rect) {
        let mut cx = self.cx();
        element::arrange_safely(self.root.as_mut(), whole, &mut cx);
        let laid = std::mem::take(&mut cx.laid_strips);
        self.settle(cx);
        if laid != self.strips {
            self.strips = laid;
            let mut cx = self.cx();
            element::arrange_safely(self.root.as_mut(), whole, &mut cx);
            cx.laid_strips.clear();
            self.settle(cx);
        }
    }

    /// Another theme: the light or dark of the system changed, or the person chose one.
    pub fn show(&mut self, look: Arc<ThemeLook>) {
        self.fonts = Rc::new(Fonts::new(look.clone()));
        self.look = look;
        self.redraw = true;
    }

    /// The screen shows in the look `looks` gives for the system's light or
    /// dark, in the theme the person chose; it shows again when either changes.
    pub fn follow(&mut self, looks: LooksFor) {
        self.show(looks(self.dark));
        self.looks = Some(looks);
        self.choices_seen = crate::choices::changes();
    }

    /// The system shows dark (or light): a screen that follows the looks shows the theme's other mode.
    pub fn set_dark(&mut self, dark: bool) {
        self.dark = dark;
        if let Some(looks) = &self.looks {
            let look = looks(dark);
            self.show(look);
        }
    }

    /// Whether the system shows dark, as the host last said.
    pub fn dark(&self) -> bool {
        self.dark
    }

    /// The person chose another theme or language: the screen follows it.
    fn follow_choices(&mut self) {
        let now = crate::choices::changes();
        if now == self.choices_seen {
            return;
        }
        self.choices_seen = now;
        if let Some(looks) = &self.looks {
            let look = looks(self.dark);
            self.show(look);
        }
        self.redraw = true;
    }

    // -- Work off the screen's thread --------------------------------------------

    /// The host is woken by `waker` when work off the screen's thread is done; it then calls `take_later`.
    pub fn set_waker(&self, waker: crate::later::Waker) {
        self.later.set_waker(waker);
    }

    /// Requests of the program (a long call's) are sent through `asker` without waiting (the browser host).
    pub fn set_asker(&self, asker: crate::later::AskLater) {
        self.later.set_asker(asker);
    }

    /// How many requests sent without waiting wait for their answer.
    pub fn later_asking(&self) -> usize {
        self.later.asking()
    }

    /// What work off the screen's thread gave goes to the elements that asked for it; true when something came.
    pub fn take_later(&mut self) -> bool {
        let done = self.later.take();
        if done.is_empty() {
            return false;
        }
        let mut cx = self.cx();
        for (id, given) in done {
            if let Some(e) = self.element_mut(id) {
                e.on_later(given, &mut cx);
            }
        }
        cx.invalidate();
        self.settle(cx);
        true
    }

    /// How much work off the screen's thread still runs.
    pub fn later_running(&self) -> usize {
        self.later.running()
    }

    /// Waits for the work off the screen's thread and gives what it gave: a test's way to finish it.
    pub fn finish_later(&mut self) {
        for _ in 0..8 {
            self.later.wait(Duration::from_secs(10));
            self.take_later();
            if self.later.running() == 0 {
                break;
            }
        }
    }

    pub fn draw(&mut self, canvas: &Canvas) {
        let now = (self.now)();
        self.wake_due(now);
        let shown: HashSet<ElementId> = self.animating.iter().copied().filter(|id| self.shows(*id)).collect();
        self.animating = shown;
        let whole = self.whole();
        // Parts lay out by the enabled state of what they are in (a text box in a disabled panel), as it is now.
        self.mark();
        self.arrange_root(whole);
        self.keep_places();
        let mut cx = self.cx();
        // What lies over the window (a dialog, a popover) makes no room for the window's handles.
        cx.strips.clear();
        for overlay in self.overlays.iter_mut() {
            element::arrange_safely(overlay.content.as_mut(), overlay.place, &mut cx);
        }
        self.settle(cx);
        self.mark();
        canvas.save();
        canvas.scale((self.scale, self.scale));
        canvas.clear(self.look.color("background"));
        let painter = Painter::new(canvas, &self.look, &self.fonts);
        if self.root.is_shown() {
            element::draw_safely(self.root.as_ref(), &painter);
        }
        for overlay in &self.overlays {
            if overlay.content.is_shown() {
                element::draw_safely(overlay.content.as_ref(), &painter);
            }
        }
        if let Some(tip) = &self.tip {
            Self::draw_tip(&painter, tip);
        }
        canvas.restore();
        self.redraw = false;
    }

    // -- What elements ask -------------------------------------------------------

    fn cx(&self) -> Cx {
        let mut cx = Cx::new(self.look.clone(), self.fonts.clone(), self.clipboard.clone(), (self.now)(), self.whole(), self.focused).with_places(self.places.clone());
        cx.focused_part = self.focused_part();
        cx.strips = self.strips.clone();
        cx
    }

    /// Keeps where each shown named part of the window is, once it is laid out (`Cx::part_place`).
    fn keep_places(&mut self) {
        let mut places = HashMap::new();
        fn walk(e: &dyn Element, into: &mut HashMap<String, (ElementId, Rect)>) {
            if !e.is_shown() {
                return;
            }
            if let Some(name) = e.as_part().and_then(|p| p.base().spec.name()) {
                into.entry(name.to_string()).or_insert((e.node().id(), e.node().bounds));
            }
            for child in e.node().children() {
                walk(child.as_ref(), into);
            }
        }
        walk(self.root.as_ref(), &mut places);
        *self.places.borrow_mut() = places;
    }

    /// Runs `act` on the element of that id with the screen's context, and does what it asked; none when it is not there.
    pub fn act<R>(&mut self, id: ElementId, act: impl FnOnce(&mut dyn Element, &mut Cx) -> R) -> Option<R> {
        let mut cx = self.cx();
        let result = self.element_mut(id).map(|e| act(e, &mut cx));
        self.settle(cx);
        result
    }

    /// Runs `act` on the root as its own type, with the screen's context; none when the root is no `T`.
    pub fn act_root<T: Element, R>(&mut self, act: impl FnOnce(&mut T, &mut Cx) -> R) -> Option<R> {
        let mut cx = self.cx();
        let result = self.root.as_any_mut().downcast_mut::<T>().map(|root| act(root, &mut cx));
        self.settle(cx);
        result
    }

    /// Does what the elements asked, until they ask nothing more; then what joined the tree joins the screen.
    fn settle(&mut self, mut cx: Cx) {
        loop {
            if cx.redraw {
                self.redraw = true;
                cx.redraw = false;
            }
            let requests = std::mem::take(&mut cx.requests);
            if requests.is_empty() {
                self.root.after_event(&mut cx);
                if !cx.requests.is_empty() {
                    continue;
                }
                if !self.disposed {
                    element::attach_all(self.root.as_mut(), &mut cx);
                    for overlay in self.overlays.iter_mut() {
                        element::attach_all(overlay.content.as_mut(), &mut cx);
                    }
                }
                if cx.requests.is_empty() && !cx.redraw {
                    self.follow_choices();
                    break;
                }
                continue;
            }
            for request in requests {
                match request {
                    Request::Focus(id) => self.focus_now(id, &mut cx),
                    Request::Open(overlay) => self.open_now(overlay, &mut cx),
                    Request::Close(id) => self.close_now(id, &mut cx),
                    Request::Wake(_, _) if self.disposed => {}
                    Request::Wake(id, at) => {
                        self.wakes.insert(id, at);
                        self.wakes_changed = true;
                    }
                    Request::Animate(id, on) => {
                        if on {
                            self.animating.insert(id);
                        } else {
                            self.animating.remove(&id);
                        }
                    }
                    Request::Later(id, work) => self.later.start(id, work),
                    Request::Ask(id, asked) => self.later.ask(id, asked),
                    // A row dragged out of a part goes to the part under the point that takes it, the deepest first.
                    Request::Drop(from, x, y) => {
                        if let Some(at) = self.at(x, y) {
                            for id in self.upward(at) {
                                if self.element_mut(id).is_some_and(|e| e.take_drop(&from, x, y, &mut cx)) {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn set_flag(&mut self, id: Option<ElementId>, set: impl FnOnce(&mut element::Node)) {
        if let Some(e) = id.and_then(|id| self.element_mut(id)) {
            set(e.node_mut());
        }
    }

    // -- Wake-ups --------------------------------------------------------------

    /// When the next element or tooltip wakes; none when none waits. The host calls `wake_due` then.
    pub fn next_wake(&self) -> Option<Instant> {
        self.wakes.values().copied().chain(self.tip_due).min()
    }

    /// Wakes every element whose time came by `now`, and shows the tooltip whose pause is over; drawing does it too.
    pub fn wake_due(&mut self, now: Instant) {
        if self.disposed {
            return;
        }
        let due: Vec<ElementId> = self.wakes.iter().filter(|(_, at)| **at <= now).map(|(id, _)| *id).collect();
        let mut cx = self.cx();
        for id in due {
            self.wakes.remove(&id);
            if let Some(e) = self.element_mut(id) {
                if e.node().attached() {
                    element::safely(e, &mut cx, (), |e, cx| e.on_wake(cx));
                }
            }
        }
        self.settle(cx);
        if let (Some(due), Some(text)) = (self.tip_due, self.tip_waiting.clone()) {
            if due <= now {
                self.tip_due = None;
                // Only for what is under the pointer now: another window may show there since.
                let (x, y) = self.tip_point;
                if self.tip_of(self.at(x, y), x, y).as_deref() != Some(text.as_str()) {
                    self.tip_waiting = None;
                    return;
                }
                let place = self.tip_place(&text);
                self.tip = Some(Tooltip { text, place });
                self.redraw = true;
            }
        }
    }

    // -- Tooltips ----------------------------------------------------------------

    // -- The pointer -------------------------------------------------------------

    // -- Keys --------------------------------------------------------------------

    /// The screen is no longer shown: its overlays close and every element leaves it, so what they started stops.
    pub fn dispose(&mut self) {
        let ids: Vec<ElementId> = self.overlays.iter().map(Overlay::id).collect();
        for id in ids.into_iter().rev() {
            self.close(id);
        }
        self.wakes.clear();
        self.hide_tip();
        self.animating.clear();
        let mut cx = self.cx();
        element::detach_all(self.root.as_mut(), &mut cx);
        self.disposed = true;
    }
}
