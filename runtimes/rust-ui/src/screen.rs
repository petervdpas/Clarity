//! A window's screen: its elements, drawn in the theme on whatever the host
//! gives (the desktop's window, a picture in a test), and what the person
//! does sent to the element it is for. The host draws again when the screen
//! says it changed (`take_redraw`), on every frame while an element animates
//! (`animating`), and at the time an element asked to wake (`next_wake`,
//! then `wake_due`). Every host sends the pointer, the wheel, keys going down
//! and up and typed text through the same methods here.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use skia_safe::Canvas;

use crate::element::{self, Composition, Cx, Element, ElementId, Request};
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

pub struct Screen {
    root: Box<dyn Element>,
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
    looks: Option<LooksFor>,
    dark: bool,
    choices_seen: u64,
    cursor: Cursor,
    cursor_changed: bool,
    context_menus: crate::frame::menus::FrameMenus,
    /// Whether something took the last key pressed; the character it types is then no shortcut.
    key_taken: bool,
    pointing: bool,
    /// The time now, as the screen's clock tells it; a test sets its own.
    pub now: Box<dyn Fn() -> Instant>,
}

impl Screen {
    pub fn new(root: Box<dyn Element>, look: Arc<ThemeLook>) -> Screen {
        let fonts = Rc::new(Fonts::new(look.clone()));
        let mut screen = Screen {
            root,
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
            looks: None,
            dark: false,
            choices_seen: crate::choices::changes(),
            cursor: Cursor::Default,
            cursor_changed: false,
            context_menus: crate::frame::menus::FrameMenus::new(),
            held: PointerButton::Left,
            key_taken: false,
            pointing: false,
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
        let mut cx = self.cx();
        element::arrange_safely(self.root.as_mut(), whole, &mut cx);
        self.settle(cx);
        self.keep_places();
        self.redraw = true;
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
        let mut cx = self.cx();
        element::arrange_safely(self.root.as_mut(), whole, &mut cx);
        self.settle(cx);
        self.keep_places();
        let mut cx = self.cx();
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

    /// A tooltip in Clarity's look: the tooltip part's colors, else a panel with a strong border.
    fn draw_tip(p: &Painter, tip: &Tooltip) {
        let look = p.look.part("tooltip", None);
        let radius = look.radius.map(|r| r as f64).unwrap_or_else(|| p.look.radius("input"));
        p.fill(tip.place, look.background.unwrap_or_else(|| p.look.color("panel")), radius);
        p.stroke(tip.place, look.border.unwrap_or_else(|| p.look.color("strong_border")), p.look.border(), radius);
        p.text(&tip.text, tip.place.inset(TIP_PAD), look.text.unwrap_or_else(|| p.look.color("text")), p.look.size("small"), "text", false, TextAlign::Left);
    }

    // -- What elements ask -------------------------------------------------------

    fn cx(&self) -> Cx {
        Cx::new(self.look.clone(), self.fonts.clone(), self.clipboard.clone(), (self.now)(), self.whole(), self.focused).with_places(self.places.clone())
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
                }
            }
        }
    }

    fn set_flag(&mut self, id: Option<ElementId>, set: impl FnOnce(&mut element::Node)) {
        if let Some(e) = id.and_then(|id| self.element_mut(id)) {
            set(e.node_mut());
        }
    }

    fn focus_now(&mut self, id: Option<ElementId>, cx: &mut Cx) {
        let id = id.filter(|i| self.element(*i).is_some());
        if id == self.focused {
            return;
        }
        let was = self.focused;
        self.set_flag(was, |n| n.focused = false);
        self.focused = id;
        self.set_flag(id, |n| n.focused = true);
        if let Some(e) = was.and_then(|w| self.element_mut(w)) {
            element::safely(e, cx, (), |e, cx| e.on_focus_changed(false, cx));
        }
        if let Some(e) = id.and_then(|i| self.element_mut(i)) {
            element::safely(e, cx, (), |e, cx| e.on_focus_changed(true, cx));
        }
        let pointing = self.pointing;
        element::safely(self.root.as_mut(), cx, (), |root, cx| root.focus_moved(id, pointing, cx));
        self.redraw = true;
    }

    fn open_now(&mut self, mut overlay: Overlay, cx: &mut Cx) {
        self.hide_tip();
        if overlay.covers {
            overlay.place = self.whole();
        }
        element::arrange_safely(overlay.content.as_mut(), overlay.place, cx);
        element::attach_all(overlay.content.as_mut(), cx);
        self.overlays.push(overlay);
        self.redraw = true;
    }

    fn close_now(&mut self, id: ElementId, cx: &mut Cx) {
        let Some(index) = self.overlays.iter().position(|o| o.id() == id) else { return };
        let focused_inside = self.focused.is_some_and(|f| element::find(self.overlays[index].content.as_ref(), f).is_some());
        if focused_inside {
            self.focus_now(None, cx);
        }
        let mut overlay = self.overlays.remove(index);
        element::detach_all(overlay.content.as_mut(), cx);
        for gone in [&mut self.hovered, &mut self.pressed] {
            if gone.is_some_and(|g| element::find(overlay.content.as_ref(), g).is_some()) {
                *gone = None;
            }
        }
        self.redraw = true;
        if let Some(closed) = overlay.closed.take() {
            closed();
        }
        if let Some(e) = overlay.opener.and_then(|o| self.element_mut(o)) {
            e.on_overlay_closed(id, cx);
        }
    }

    /// Opens an overlay over the window.
    pub fn open(&mut self, overlay: Overlay) -> ElementId {
        let mut cx = self.cx();
        let id = cx.open(overlay);
        self.settle(cx);
        id
    }

    /// Closes the overlay of that id.
    pub fn close(&mut self, id: ElementId) {
        let mut cx = self.cx();
        cx.close(id);
        self.settle(cx);
    }

    /// What the person typed in the part with the focus is taken, as when they leave it.
    pub fn take_typed(&mut self) {
        let focused = self.focused;
        let mut cx = self.cx();
        let tree: &mut dyn Element = match self.overlays.iter_mut().rev().find(|o| focused.is_some_and(|f| element::find(o.content.as_ref(), f).is_some())) {
            Some(overlay) => overlay.content.as_mut(),
            None => self.root.as_mut(),
        };
        element::take_typed_from(tree, focused, &mut cx);
        self.settle(cx);
    }

    /// Gives the focus to the element of that id, or takes it away.
    pub fn focus(&mut self, id: Option<ElementId>) {
        let mut cx = self.cx();
        cx.focus(id);
        self.settle(cx);
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

    /// What the element at a point, or one it is in, says about that place.
    fn tip_of(&self, at: Option<ElementId>, x: f64, y: f64) -> Option<String> {
        let at = at?;
        let tree = self.overlays.iter().rev().map(|o| o.content.as_ref()).find(|c| element::find(*c, at).is_some()).unwrap_or(self.root.as_ref());
        let ids = element::chain(tree, at)?;
        ids.iter().rev().filter_map(|id| element::find(tree, *id)).find_map(|e| e.tooltip_at(x, y).or_else(|| Self::designed_tip(e)).filter(|t| !t.is_empty()))
    }

    /// The tooltip a part is designed with (`tooltip "@saveTooltip"`), in the person's language; none for none.
    fn designed_tip(e: &dyn Element) -> Option<String> {
        let base = e.as_part()?.base();
        let written = base.spec.text("tooltip").filter(|t| !t.is_empty())?;
        Some(base.host.text(Some(written)))
    }

    /// The pointer rests on `text` from now (none: on nothing that says something).
    fn rest(&mut self, text: Option<String>, x: f64, y: f64) {
        if text == self.tip_waiting {
            return;
        }
        self.hide_tip();
        self.tip_point = (x, y);
        if text.is_some() {
            self.tip_due = Some((self.now)() + TIP_PAUSE);
            self.wakes_changed = true;
        }
        self.tip_waiting = text;
    }

    /// The tooltip goes, and none waits.
    fn hide_tip(&mut self) {
        self.tip_waiting = None;
        self.tip_due = None;
        if self.tip.take().is_some() {
            self.redraw = true;
        }
    }

    /// Where a tooltip shows: under the pointer, inside the screen.
    fn tip_place(&self, text: &str) -> Rect {
        let size = self.look.size("small");
        let width = self.fonts.measure(text, "text", size, false) + 2.0 * TIP_PAD;
        let height = size + 2.0 * TIP_PAD + 2.0;
        let (px, py) = self.tip_point;
        let x = px.min(self.width - width).max(0.0);
        let y = if py + 20.0 + height > self.height { py - 8.0 - height } else { py + 20.0 };
        Rect::new(x, y.max(0.0), width, height)
    }

    // -- The pointer -------------------------------------------------------------

    /// The deepest element at a point: in the top overlay that has it, else in the window.
    pub fn at(&self, x: f64, y: f64) -> Option<ElementId> {
        for overlay in self.overlays.iter().rev() {
            if let Some(inside) = element::at(overlay.content.as_ref(), x, y) {
                return Some(inside);
            }
            if overlay.modal {
                return None;
            }
        }
        element::at(self.root.as_ref(), x, y)
    }

    fn input(e: &dyn Element, x: f64, y: f64, modifiers: Modifiers, clicks: u32) -> PointerInput {
        Self::input_of(e, x, y, modifiers, clicks, PointerButton::Left)
    }

    fn input_of(e: &dyn Element, x: f64, y: f64, modifiers: Modifiers, clicks: u32, button: PointerButton) -> PointerInput {
        let b = e.node().bounds;
        PointerInput { screen_x: x, screen_y: y, x: x - b.x, y: y - b.y, modifiers, clicks, button }
    }

    /// The tree an element is in: the top overlay that holds it, else the window.
    fn tree_of(&self, at: ElementId) -> &dyn Element {
        self.overlays.iter().rev().map(|o| o.content.as_ref()).find(|c| element::find(*c, at).is_some()).unwrap_or(self.root.as_ref())
    }

    /// The ids from the element of that id up to its tree's root, itself first.
    fn upward(&self, at: ElementId) -> Vec<ElementId> {
        let mut ids = element::chain(self.tree_of(at), at).unwrap_or_default();
        ids.reverse();
        ids
    }

    /// The shape over a point: of what holds the pointer, else of the deepest
    /// element there or one it is in; a disabled one shows the plain pointer.
    fn shape_at(&self, at: Option<ElementId>, x: f64, y: f64) -> Cursor {
        let Some(from) = self.pressed.or(at) else { return Cursor::Default };
        if !self.element(from).is_some_and(element::enabled_in_tree) {
            return Cursor::Default;
        }
        for id in self.upward(from) {
            if let Some(e) = self.element(id) {
                if let Some(shape) = element::safely_read(e, None, |e| e.cursor_at(x, y)) {
                    return shape;
                }
            }
        }
        Cursor::Default
    }

    fn shape(&mut self, shape: Cursor) {
        if shape != self.cursor {
            self.cursor = shape;
            self.cursor_changed = true;
        }
    }

    pub fn pointer_moved(&mut self, x: f64, y: f64) {
        self.mark();
        let mut cx = self.cx();
        let held = self.held;
        if let Some(e) = self.pressed.and_then(|p| self.element_mut(p)) {
            let input = Self::input_of(e, x, y, Modifiers::NONE, 1, held);
            element::safely(e, &mut cx, (), |e, cx| e.on_pointer_moved(input, cx));
        }
        let at = self.at(x, y);
        if at.is_some() && at != self.pressed {
            if let Some(e) = at.and_then(|a| self.element_mut(a)).filter(|e| element::enabled_in_tree(&**e)) {
                let input = Self::input(e, x, y, Modifiers::NONE, 1);
                element::safely(e, &mut cx, (), |e, cx| e.on_pointer_moved(input, cx));
            }
        }
        let shape = self.shape_at(at, x, y);
        self.shape(shape);
        let tip = if self.pressed.is_none() { self.tip_of(at, x, y) } else { None };
        self.rest(tip, x, y);
        if at != self.hovered {
            let was = self.hovered;
            self.set_flag(was, |n| n.hovered = false);
            self.hovered = at;
            self.set_flag(at, |n| n.hovered = true);
            self.redraw = true;
        }
        self.settle(cx);
    }

    pub fn pointer_left(&mut self) {
        self.hide_tip();
        if self.pressed.is_none() {
            self.shape(Cursor::Default);
        }
        if let Some(was) = self.hovered.take() {
            self.set_flag(Some(was), |n| n.hovered = false);
            self.redraw = true;
        }
    }

    /// The deepest element at or above `at` that takes the focus; none when that one is disabled.
    fn taker(&self, at: ElementId) -> Option<ElementId> {
        let found = self.upward(at).into_iter().find(|id| self.element(*id).is_some_and(|e| e.focusable()))?;
        self.element(found).is_some_and(element::enabled_in_tree).then_some(found)
    }

    pub fn pointer_pressed(&mut self, x: f64, y: f64, button: PointerButton, clicks: u32) {
        self.pointer_pressed_with(x, y, button, clicks, Modifiers::NONE);
    }

    pub fn pointer_pressed_with(&mut self, x: f64, y: f64, button: PointerButton, clicks: u32, modifiers: Modifiers) {
        self.hide_tip();
        // A press outside the overlays closes them, top first, and goes no
        // further, unless the one closed lets it through (a dropdown's list).
        if let Some(top) = self.overlays.last() {
            let anchored = top.anchor.and_then(|a| self.element(a)).is_some_and(|a| a.node().bounds.contains(x, y));
            if !top.modal && !top.content.node().bounds.contains(x, y) && !top.place.contains(x, y) && !anchored {
                let (id, through) = (top.id(), top.through);
                self.close(id);
                if !through {
                    return;
                }
                return self.pointer_pressed_with(x, y, button, clicks, modifiers);
            }
        }
        self.pointer_moved(x, y);
        // A press starts no new pause: the tip of what was pressed shows again only once the pointer went elsewhere.
        self.tip_due = None;
        // What is disabled (or in a disabled part) takes no press, no focus and no menu.
        let Some(at) = self.at(x, y) else { return };
        if !self.element(at).is_some_and(element::enabled_in_tree) {
            return;
        }
        // A right press is a drag of a part that takes right drags (a canvas draws a line with it); else it opens the menu there.
        let right_drag = button == PointerButton::Right
            && self.element(at).is_some_and(|e| {
                let b = e.node().bounds;
                element::safely_read(e, false, |e| e.right_drags(x - b.x, y - b.y))
            });
        if button == PointerButton::Right && !right_drag {
            self.context_pressed(at, x, y);
            return;
        }
        if button != PointerButton::Left && !right_drag {
            return;
        }
        // A right drag holds the pointer until the right button goes up.
        if self.pressed.is_some() && self.held == PointerButton::Right {
            return;
        }
        self.held = button;
        // Focus by the pointer: what was pressed stays where it is (no scrolling into view), so the release lands on it.
        self.pointing = true;
        let mut cx = self.cx();
        cx.focus(self.taker(at));
        self.settle(cx);
        self.pointing = false;
        self.pressed = Some(at);
        self.set_flag(Some(at), |n| n.pressed = true);
        let mut cx = self.cx();
        if let Some(e) = self.element_mut(at) {
            let input = Self::input_of(e, x, y, modifiers, clicks.max(1), button);
            element::safely(e, &mut cx, (), |e, cx| e.on_pointer_pressed(input, cx));
        }
        self.settle(cx);
        self.redraw = true;
    }

    /// A right press: what takes the focus there takes it (a text box keeps
    /// its selection for Copy), then the element there, or one it is in,
    /// gives its menu, which opens at the pointer (a part's `menu`); a table,
    /// list or tree chooses the row there first, as a left press does.
    fn context_pressed(&mut self, at: ElementId, x: f64, y: f64) {
        if let Some(taker) = self.taker(at) {
            self.focus(Some(taker));
        }
        // A table, list or tree chooses the row under the pointer first, menu or not.
        let rows = self.upward(at).into_iter().any(|id| {
            self.element(id).and_then(|e| e.as_part()).is_some_and(|p| matches!(p.base().spec.kind.as_str(), "table" | "list" | "tree" | "treeTable"))
        });
        if rows {
            self.pointer_pressed(x, y, PointerButton::Left, 1);
            self.pointer_released(x, y, PointerButton::Left);
        }
        let found = self.upward(at).into_iter().find_map(|id| {
            let e = self.element(id)?;
            let entries = element::safely_read(e, None, |e| e.context_menu_at(x, y)).filter(|entries| !entries.is_empty())?;
            Some((id, entries))
        });
        let Some((id, entries)) = found else { return };
        let mut cx = self.cx();
        self.context_menus.open((id, 0), entries, Rect::new(x, y - 1.0, 0.0, 0.0), None, &mut cx);
        self.settle(cx);
        self.redraw = true;
    }

    pub fn pointer_released(&mut self, x: f64, y: f64, button: PointerButton) {
        if button != self.held || self.pressed.is_none() {
            return;
        }
        if button == PointerButton::Right {
            return self.right_released(x, y);
        }
        let Some(was) = self.pressed.take() else { return };
        self.set_flag(Some(was), |n| n.pressed = false);
        let mut cx = self.cx();
        if let Some(e) = self.element_mut(was) {
            let input = Self::input(e, x, y, Modifiers::NONE, 1);
            element::safely(e, &mut cx, (), |e, cx| e.on_pointer_released(input, cx));
        }
        self.settle(cx);
        self.redraw = true;
        if self.at(x, y) == Some(was) {
            let mut cx = self.cx();
            if let Some(e) = self.element_mut(was) {
                element::safely(e, &mut cx, (), |e, cx| e.on_click(cx));
            }
            self.settle(cx);
        }
        self.mark();
        let shape = self.shape_at(self.at(x, y), x, y);
        self.shape(shape);
    }

    /// The right button went up on a right drag: the part used it, or it opens its menu where it was pressed.
    fn right_released(&mut self, x: f64, y: f64) {
        self.held = PointerButton::Left;
        let Some(was) = self.pressed.take() else { return };
        self.set_flag(Some(was), |n| n.pressed = false);
        let mut cx = self.cx();
        let used = match self.element_mut(was) {
            Some(e) => {
                let input = Self::input_of(e, x, y, Modifiers::NONE, 1, PointerButton::Right);
                element::safely(e, &mut cx, true, |e, cx| e.on_right_released(input, cx))
            }
            None => true,
        };
        self.settle(cx);
        self.redraw = true;
        if !used {
            self.context_pressed(was, x, y);
        }
        self.mark();
        let shape = self.shape_at(self.at(x, y), x, y);
        self.shape(shape);
    }

    /// The pointer was taken away mid-press (the window lost the focus
    /// during a drag): what holds it lets go without a release and without a
    /// click, so a drag in progress is dropped, not committed.
    pub fn pointer_cancelled(&mut self) {
        self.held = PointerButton::Left;
        let Some(was) = self.pressed.take() else { return };
        self.set_flag(Some(was), |n| n.pressed = false);
        let mut cx = self.cx();
        if let Some(e) = self.element_mut(was) {
            element::safely(e, &mut cx, (), |e, cx| e.on_pointer_cancelled(cx));
        }
        self.settle(cx);
        self.redraw = true;
        self.shape(Cursor::Default);
    }

    /// A press and its release at a point: a click.
    pub fn click(&mut self, x: f64, y: f64) {
        self.pointer_pressed(x, y, PointerButton::Left, 1);
        self.pointer_released(x, y, PointerButton::Left);
    }

    /// The wheel turned at a point: the deepest element there that scrolls
    /// takes it. In the screen's units: a positive `dy` shows what is further
    /// down, a positive `dx` what is further right, with the keys `held`.
    /// Shift turns a wheel that only goes up and down sideways, as every
    /// system's scrolling does.
    pub fn wheel(&mut self, x: f64, y: f64, dx: f64, dy: f64, held: Modifiers) {
        self.hide_tip();
        let (dx, dy) = if held.has(Modifiers::SHIFT) && dx == 0.0 { (dy, 0.0) } else { (dx, dy) };
        let Some(at) = self.at(x, y) else { return };
        let ids = self.upward(at);
        let mut cx = self.cx();
        for id in ids {
            if let Some(e) = self.element_mut(id) {
                if element::safely(e, &mut cx, false, |e, cx| e.on_wheel(dx, dy, held, cx)) {
                    cx.invalidate();
                    break;
                }
            }
        }
        self.settle(cx);
    }

    // -- Keys --------------------------------------------------------------------

    /// The elements Tab reaches, in Tab order: in the dialog on top, else in the window.
    pub fn tab_order(&self) -> Vec<ElementId> {
        match self.overlays.last() {
            Some(top) if top.modal => element::focus_takers(top.content.as_ref()),
            _ => element::focus_takers(self.root.as_ref()),
        }
    }

    /// The focus goes to the next element in Tab order, or the one before.
    pub fn move_focus(&mut self, back: bool) {
        let takers = self.tab_order();
        if takers.is_empty() {
            return;
        }
        let at = self.focused.and_then(|f| takers.iter().position(|t| *t == f));
        let next = if back {
            match at {
                Some(i) if i > 0 => i - 1,
                _ => takers.len() - 1,
            }
        } else {
            at.map(|i| (i + 1) % takers.len()).unwrap_or(0)
        };
        self.focus(Some(takers[next]));
    }

    fn modal_on_top(&self) -> bool {
        self.overlays.last().is_some_and(|o| o.modal)
    }

    /// The window's own keys (its hook, then the root's): a button's
    /// shortcut, Enter for the default button, Escape for the cancel
    /// button. Not asked while a dialog is on top: its keys are its own.
    fn window_keys(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if self.modal_on_top() {
            return false;
        }
        let hooked = self.keys.as_mut().is_some_and(|keys| keys(key, cx));
        hooked || element::safely(self.root.as_mut(), cx, false, |root, cx| root.window_key(key, cx))
    }

    /// The focused element, when the person may use it.
    fn focused_enabled(&self) -> Option<ElementId> {
        self.focused.filter(|f| self.element(*f).is_some_and(element::enabled_in_tree))
    }

    /// A key pressed: a dialog on top hears it first, then the part with the
    /// focus (a text box keeps Ctrl+A and its arrows), then the window's own
    /// keys. True when something on the screen took it.
    pub fn key_pressed(&mut self, key: KeyInput) -> bool {
        self.hide_tip();
        self.mark();
        self.key_taken = true;
        let mut cx = self.cx();
        let top_modal = self.modal_on_top();
        // A dialog on top hears its keys before the part with the focus.
        if top_modal {
            let took = match self.overlays.last_mut() {
                Some(top) => element::safely(top.content.as_mut(), &mut cx, false, |c, cx| c.dialog_key(key, cx)),
                None => false,
            };
            if took {
                cx.invalidate();
                self.settle(cx);
                return true;
            }
        }
        let focused = self.focused_enabled();
        let focused_key = |screen: &mut Screen, cx: &mut Cx| match focused.and_then(|f| screen.element_mut(f)) {
            Some(e) => element::safely(e, cx, false, |e, cx| e.on_key(key, cx)),
            None => false,
        };
        // Escape closes the overlay on top, unless the part with the focus takes it (a list that closes itself).
        if key.key == Key::Escape && !self.overlays.is_empty() && !top_modal {
            if !focused_key(self, &mut cx) {
                if let Some(top) = self.overlays.last() {
                    cx.close(top.id());
                }
            }
            self.settle(cx);
            return true;
        }
        if key.key == Key::Tab && !key.control() && !key.alt() {
            // A part that takes Tab itself but lets it go (past a table's last cell) has the focus move on as Tab does.
            let wants_tab = focused.and_then(|f| self.element(f)).is_some_and(|e| e.wants_tab());
            if wants_tab && focused_key(self, &mut cx) {
                self.settle(cx);
                return true;
            }
            self.settle(cx);
            self.move_focus(key.shift());
            return !self.tab_order().is_empty();
        }
        if focused_key(self, &mut cx) {
            cx.invalidate();
            self.settle(cx);
            return true;
        }
        if self.window_keys(key, &mut cx) {
            cx.invalidate();
            self.settle(cx);
            return true;
        }
        self.settle(cx);
        self.key_taken = false;
        false
    }

    /// A key let go: the window hears it first (Alt for its menu bar), then the part with the focus.
    pub fn key_released(&mut self, key: KeyInput) {
        let mut cx = self.cx();
        let top_modal = self.modal_on_top();
        if !top_modal && element::safely(self.root.as_mut(), &mut cx, false, |root, cx| root.window_key_up(key, cx)) {
            cx.invalidate();
            self.settle(cx);
            return;
        }
        if let Some(e) = self.focused.and_then(|f| self.element_mut(f)) {
            element::safely(e, &mut cx, (), |e, cx| e.on_key_up(key, cx));
        }
        self.settle(cx);
    }

    /// Text typed: after a key nothing took, a single character is first the
    /// window's (a character shortcut such as `*`, matched by what the
    /// keyboard typed, not where its key is), else the focused part's.
    pub fn text_typed(&mut self, text: &str) {
        self.mark();
        let mut cx = self.cx();
        let mut chars = text.chars();
        let single = match (chars.next(), chars.next()) {
            (Some(c), None) if !c.is_control() => Some(c),
            _ => None,
        };
        let shortcut = !self.key_taken && single.is_some_and(|c| self.window_keys(KeyInput::typed(c), &mut cx));
        self.key_taken = false;
        if shortcut {
            cx.invalidate();
            self.settle(cx);
            return;
        }
        if let Some(e) = self.focused_enabled().and_then(|f| self.element_mut(f)) {
            element::safely(e, &mut cx, (), |e, cx| e.on_text(text, cx));
        }
        self.settle(cx);
    }

    /// An input method composes text (none: it stopped): the part with the
    /// focus shows it at its caret. What it commits comes as `text_typed`.
    pub fn compose(&mut self, composing: Option<Composition>) {
        let mut cx = self.cx();
        if let Some(e) = self.focused.and_then(|f| self.element_mut(f)) {
            e.on_compose(composing, &mut cx);
        }
        cx.invalidate();
        self.settle(cx);
    }

    /// Where the caret of the part with the focus is, in the screen's units:
    /// an input method shows its window by it; none when no part types.
    pub fn caret_area(&self) -> Option<Rect> {
        self.focused.and_then(|f| self.element(f)).and_then(|e| e.caret_area())
    }

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
