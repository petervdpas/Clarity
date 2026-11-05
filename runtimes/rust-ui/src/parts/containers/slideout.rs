//! A slide-out (conformance/slideouts.json): parts on a grid of their own
//! that slide in over the window from one edge of its content (the right
//! without a side; the left, the top or the bottom) while its part's state
//! is visible, as high (or as wide) as the window's content, below the menu
//! bar, toolbar and ribbon and above the footer, with a shadow. What is under
//! it stays where it is. Escape, a press beside it or the close button in
//! its header closes it, and its state hears that it is no longer visible.
//! A pinnable one has a pin in its header: pinned, it stays open beside the
//! window's parts, which make room for it (the window view narrows them),
//! and a press beside it leaves it open. A resizable one is dragged by its
//! inner edge between its min and max, as a sidebar is. Its size and
//! whether it is pinned are kept per window for the next start
//! (`slideout.<window>.<name>`), as a table's column widths are. It runs
//! `on open` and `on close`; the focus moves into it when it opens and back
//! where it was when it closes. In the window's grid it takes no room.
//!
//! One with a caption (every one, unless its caption is written empty;
//! the catalog's is "Details") has a handle: a tab on its edge of the window with
//! the caption written along it, always there, open or closed, as an IDE's
//! side panels have. A click (or Enter or Space) on it slides it out, and
//! another slides it back. The handles of one side stand in a strip the
//! window's parts make room for, one after the other; those of the bottom
//! on top of the footer.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use serde_json::Value;
use skia_safe::Color;

use crate::element::{draw_children, Cx, Element, ElementId, Node};
use crate::frame::rules;
use crate::geometry::Rect;
use crate::grid::GAP;
use crate::input::{Cursor, Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::markup::PartSpec;
use crate::overlay::Overlay;
use crate::painter::{Painter, TextAlign};
use crate::parts::{Part, PartBase, PartHost};

use super::container;

/// How wide the strip of handles is, and the space along a handle beside its caption.
pub const HANDLE_WIDTH: f64 = 28.0;
pub const HANDLE_PADDING: f64 = 14.0;

/// How long it takes to slide in.
pub const SLIDE_TIME: Duration = Duration::from_millis(150);

/// Its width when its markup gives none (clarity_spec::model::SLIDEOUT_WIDTH),
/// and its height at the top or the bottom (clarity_spec::model::SLIDEOUT_HEIGHT).
pub const WIDTH: f64 = 420.0;
pub const HEIGHT: f64 = 300.0;

/// The edge of the window's content a slide-out comes in from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    /// As its markup says it (`side="top"`); the right without one or for a word it does not know.
    pub fn of(word: Option<&str>) -> Side {
        match word {
            Some("left") => Side::Left,
            Some("top") => Side::Top,
            Some("bottom") => Side::Bottom,
            _ => Side::Right,
        }
    }

    /// Whether it lies across the window, at the top or the bottom: its size is a height.
    pub fn across(self) -> bool {
        matches!(self, Side::Top | Side::Bottom)
    }

    /// Whether it comes from the start of its axis (the left or the top), so it grows toward the end.
    pub fn from_start(self) -> bool {
        matches!(self, Side::Left | Side::Top)
    }

    /// The thin edge of `b` toward the window's parts, `thick` deep.
    pub fn inner_edge(self, b: Rect, thick: f64) -> Rect {
        match self {
            Side::Left => Rect::new(b.right() - thick, b.y, thick, b.height),
            Side::Right => Rect::new(b.x, b.y, thick, b.height),
            Side::Top => Rect::new(b.x, b.bottom() - thick, b.width, thick),
            Side::Bottom => Rect::new(b.x, b.y, b.width, thick),
        }
    }
}

/// How high its header is, how wide the edge the person drags, and the space inside it.
pub const HEADER_HEIGHT: f64 = 36.0;
pub const EDGE_WIDTH: f64 = 6.0;
pub const PADDING: f64 = 12.0;
const BUTTON: f64 = 24.0;
const SHADOW: f64 = 12.0;

/// What the part, what lies over the window and its buttons share.
pub struct Slide {
    name: Option<String>,
    host: Rc<dyn PartHost>,
    side: Side,
    pinnable: bool,
    resizable: bool,
    min: Option<f64>,
    max: Option<f64>,
    designed: f64,
    caption: Option<String>,
    /// Its size (its width, or its height at the top or the bottom) as the person left it, when they dragged it now or before.
    dragged: Cell<Option<f64>>,
    /// Whether it was pinned when the window last closed, read once.
    kept_pinned: Cell<Option<bool>>,
    read: Cell<bool>,
    /// Where it lies when it is all the way in: the band at its side of the window's content.
    band: Cell<Rect>,
    /// When it began to slide in; none once it is in.
    since: Cell<Option<Instant>>,
    /// Its handle on the window's edge, once the window made it: a press there opens and closes it, never a press beside it.
    handle: Cell<Option<ElementId>>,
}

impl Slide {
    fn of(spec: &PartSpec, host: Rc<dyn PartHost>) -> Slide {
        let number = |field: &str| spec.number(field).or_else(|| spec.text(field).and_then(|t| t.trim().parse().ok())).filter(|n: &f64| *n > 0.0);
        let side = Side::of(spec.text("side"));
        let designed = if side.across() { number("slideHeight").unwrap_or(HEIGHT) } else { number("slideWidth").unwrap_or(WIDTH) };
        Slide {
            name: spec.name().map(str::to_string),
            host,
            side,
            pinnable: spec.flag("pinnable"),
            resizable: spec.flag("resizable"),
            min: number("min"),
            max: number("max"),
            designed,
            caption: spec.text("caption").filter(|c| !c.is_empty()).map(str::to_string),
            dragged: Cell::new(None),
            kept_pinned: Cell::new(None),
            read: Cell::new(false),
            band: Cell::new(Rect::default()),
            since: Cell::new(None),
            handle: Cell::new(None),
        }
    }

    /// Whether it has a handle on the window's edge: one with a caption has.
    pub fn has_handle(&self) -> bool {
        self.caption.is_some()
    }

    /// Its caption as shown, translated; empty for none.
    pub fn caption(&self) -> String {
        self.caption.as_deref().map(|c| self.host.text(Some(c))).unwrap_or_default()
    }

    /// Opens it: its state hears it is visible. One without a name never opens.
    pub fn open(&self) {
        if let Some(name) = self.name.as_deref() {
            crate::parts::change_state(&*self.host, name, |s| s.visible = Some(true));
        }
    }

    /// Where its size and pin are kept: `slideout.Main.details`; none for
    /// one without a name or outside a named window. Window and part names
    /// are the project's own, an open set, so the key is made from them.
    fn key(&self) -> Option<String> {
        let name = self.name.as_deref().filter(|n| !n.is_empty())?;
        let window = self.host.window_name().filter(|w| !w.is_empty())?;
        Some(format!("slideout.{window}.{name}"))
    }

    /// What was kept before, read once.
    fn read_kept(&self) {
        if self.read.replace(true) {
            return;
        }
        let Some(text) = self.key().and_then(|k| crate::choices::kept(&k)) else { return };
        // A damaged value: as designed.
        let Ok(Value::Object(kept)) = serde_json::from_str::<Value>(&text) else { return };
        if self.resizable {
            if let Some(size) = kept.get("size").or(kept.get("width")).and_then(Value::as_f64) {
                self.dragged.set(Some(rules::sidebar_width(size, self.min, self.max)));
            }
        }
        if self.pinnable {
            self.kept_pinned.set(kept.get("pinned").and_then(Value::as_bool));
        }
    }

    /// Keeps its size and pin for the next start.
    fn keep(&self) {
        let Some(key) = self.key() else { return };
        let mut kept = serde_json::Map::new();
        if let Some(size) = self.dragged.get() {
            kept.insert("size".into(), Value::from(size.round() as i64));
        }
        if self.pinnable {
            kept.insert("pinned".into(), Value::Bool(self.pinned()));
        }
        crate::choices::keep_value(&key, &Value::Object(kept).to_string());
    }

    fn state(&self) -> Option<clarity_runtime::ui::PartState> {
        self.host.state(self.name.as_deref())
    }

    /// Whether it is open: its part's state is visible. One without a name never opens.
    pub fn is_open(&self) -> bool {
        self.state().and_then(|s| s.visible).unwrap_or(false)
    }

    /// Whether it is pinned open beside the window's parts: as its state says, else as kept before.
    pub fn pinned(&self) -> bool {
        self.read_kept();
        self.pinnable && self.state().and_then(|s| s.pinned).or(self.kept_pinned.get()).unwrap_or(false)
    }

    /// Pins it open beside the window's parts, or lets it slide away again.
    pub fn pin(&self, pinned: bool) {
        let Some(name) = self.name.as_deref().filter(|_| self.pinnable) else { return };
        crate::parts::change_state(&*self.host, name, |s| s.pinned = Some(pinned));
        self.keep();
    }

    /// Closes it: its state hears it is no longer visible.
    pub fn close(&self) {
        if let Some(name) = self.name.as_deref() {
            crate::parts::change_state(&*self.host, name, |s| s.visible = Some(false));
        }
    }

    /// Its size from its edge in: its width, or its height at the top or the
    /// bottom; as designed, or as the person dragged it, now or before.
    pub fn size(&self) -> f64 {
        self.read_kept();
        self.dragged.get().unwrap_or(self.designed)
    }

    /// The edge it comes in from.
    pub fn side(&self) -> Side {
        self.side
    }

    /// The person dragged its inner edge to make it `size` deep: within its limits.
    fn drag_to(&self, size: f64) {
        self.dragged.set(Some(rules::sidebar_width(size, self.min, self.max)));
    }

    /// How far in it is, from 0 to 1: all the way in once its time passed.
    fn shown_part(&self, now: Instant) -> f64 {
        match self.since.get() {
            Some(since) => (now.saturating_duration_since(since).as_secs_f64() / SLIDE_TIME.as_secs_f64()).min(1.0),
            None => 1.0,
        }
    }

    /// Where it lies now: its band, still partly beyond its edge while it slides in.
    fn place(&self, now: Instant) -> Rect {
        let band = self.band.get();
        let part = 1.0 - self.shown_part(now);
        match self.side {
            Side::Left => Rect::new(band.x - part * band.width, band.y, band.width, band.height),
            Side::Right => Rect::new(band.x + part * band.width, band.y, band.width, band.height),
            Side::Top => Rect::new(band.x, band.y - part * band.height, band.width, band.height),
            Side::Bottom => Rect::new(band.x, band.y + part * band.height, band.width, band.height),
        }
    }
}

pub struct SlideoutPart {
    base: PartBase,
    slide: Rc<Slide>,
    /// The id of the overlay it lies in while open; shared with the overlay's `closed`, which empties it.
    opened: Rc<Cell<Option<ElementId>>>,
    /// Whether it was open the last time it was laid out, and whether its `on open` or `on close` waits to run.
    was_open: bool,
    due: Option<&'static str>,
    /// What had the focus before it opened, and what of it takes the focus.
    before: Option<ElementId>,
    inside: Vec<ElementId>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let slide = Rc::new(Slide::of(&spec, host.clone()));
    Box::new(SlideoutPart { base: PartBase::new(spec, host), slide, opened: Rc::new(Cell::new(None)), was_open: false, due: None, before: None, inside: Vec::new() })
}

impl SlideoutPart {
    /// What it shares with what lies over the window.
    pub fn slide(&self) -> &Rc<Slide> {
        &self.slide
    }

    /// The id of what lies over the window while it is open; none while closed.
    pub fn opened(&self) -> Option<ElementId> {
        self.opened.get()
    }

    pub fn is_open(&self) -> bool {
        self.slide.is_open()
    }

    /// Whether it lies beside the window's parts now: open and pinned.
    pub fn docked(&self) -> bool {
        self.is_open() && self.slide.pinned()
    }

    /// The window view gives it its band: the side of the window's content it lies over.
    pub fn set_band(&self, band: Rect) {
        self.slide.band.set(band);
    }

    /// Opens or closes it as its part's state says.
    fn follow(&mut self, cx: &mut Cx) {
        let open = self.is_open() && self.slide.name.is_some();
        if !open {
            if let Some(overlay) = self.opened.take() {
                cx.close(overlay);
            }
            if self.was_open {
                self.was_open = false;
                self.slide.since.set(None);
                // The focus goes back where it was, when it was in here (or nowhere now).
                let focus_here = cx.focused().is_none_or(|f| self.inside.contains(&f));
                if focus_here && self.before.is_some() {
                    cx.focus(self.before.take());
                }
                self.inside.clear();
                self.due(cx, "close");
            }
            return;
        }
        if self.opened.get().is_some() {
            return;
        }
        let grid = container::grid_of(&self.base.spec.children, container::columns_of(&self.base.spec), GAP, &self.base.host).owned(self.base.spec.text(crate::markup::DESIGN_ITEM));
        let frame = SlideoutFrame::new(self.slide.clone(), grid);
        let takers = crate::element::focus_takers(&frame);
        self.inside = crate::element::in_order(&frame).into_iter().map(|e| e.node().id()).collect();
        // It places itself at its band; a press there is in it, as its content is.
        let mut overlay = Overlay::new(Box::new(frame), Rect::default());
        let stays = self.slide.clone();
        overlay.stays = Some(Rc::new(move || stays.pinned()));
        // A press on its handle is no press beside it: the handle closes it. A press on another handle closes it and opens that one.
        overlay.anchor = self.slide.handle.get();
        overlay.through = self.slide.has_handle();
        let (opened, slide) = (self.opened.clone(), self.slide.clone());
        overlay.closed = Some(Box::new(move || {
            // Closed by a press beside it or Escape: its state hears it is no longer visible.
            if opened.take().is_some() {
                slide.close();
            }
        }));
        // It slides in when it opens now, not when it was open already as the window came (a pinned one kept open).
        if !self.was_open {
            self.slide.since.set(Some(cx.now()));
            self.before = cx.focused();
        }
        self.opened.set(Some(cx.open(overlay)));
        // Its first part that takes the focus has it.
        if let Some(first) = takers.into_iter().find(|t| self.inside.contains(t)) {
            cx.focus(Some(first));
        }
        if !self.was_open {
            self.was_open = true;
            self.due(cx, "open");
        }
    }

    /// Its `on open` or `on close` runs once the screen is done laying out, as an event.
    fn due(&mut self, cx: &mut Cx, event: &'static str) {
        if self.base.spec.on(event).next().is_some() {
            self.due = Some(event);
            cx.wake(self.base.node.id(), cx.now());
        }
    }
}

impl Element for SlideoutPart {
    fn node(&self) -> &Node {
        &self.base.node
    }

    fn node_mut(&mut self) -> &mut Node {
        &mut self.base.node
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_part(&self) -> Option<&dyn Part> {
        Some(self)
    }

    fn as_part_mut(&mut self) -> Option<&mut dyn Part> {
        Some(self)
    }

    /// In the window's grid it shows nothing: what it holds lies over the window.
    fn is_shown(&self) -> bool {
        false
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.follow(cx);
    }

    fn draw(&self, _p: &Painter) {}

    fn on_wake(&mut self, _cx: &mut Cx) {
        if let Some(event) = self.due.take() {
            self.base.fire(event);
        }
    }

    /// Leaving the screen, what it opened goes with it; its state stays as it is.
    fn on_detached(&mut self, cx: &mut Cx) {
        if let Some(open) = self.opened.take() {
            cx.close(open);
        }
    }
}

impl Part for SlideoutPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    /// It takes no room in the window's grid, so it grows by none.
    fn grows(&self) -> bool {
        false
    }
}

/// What lies over the window: the panel's color with a shadow toward the
/// window, its header (its caption, the pin, the close button), and its grid.
pub struct SlideoutFrame {
    node: Node,
    slide: Rc<Slide>,
    dragging: Option<(f64, f64)>,
    pointer: Option<(f64, f64)>,
    sliding: bool,
}

impl SlideoutFrame {
    fn new(slide: Rc<Slide>, grid: crate::element::GridPanel) -> SlideoutFrame {
        let mut node = Node::new();
        // In Tab order: its parts, then the pin, then the close button.
        node.add(Box::new(grid));
        if slide.pinnable {
            node.add(Box::new(SlideoutButton { node: Node::new(), slide: slide.clone(), pins: true }));
        }
        node.add(Box::new(SlideoutButton { node: Node::new(), slide: slide.clone(), pins: false }));
        SlideoutFrame { node, slide, dragging: None, pointer: None, sliding: false }
    }

    /// What it shares with its part.
    pub fn slide(&self) -> &Rc<Slide> {
        &self.slide
    }

    /// Its caption as shown, translated; empty for none.
    pub fn caption(&self) -> String {
        self.slide.caption()
    }

    /// Its parts' grid.
    pub fn grid(&self) -> &crate::element::GridPanel {
        self.node.children()[0].as_any().downcast_ref().expect("a slide-out holds its parts on a grid")
    }

    /// Its pin (when it is pinnable) and close buttons, the pin first.
    pub fn buttons(&self) -> Vec<&SlideoutButton> {
        self.node.children().iter().filter_map(|c| c.as_any().downcast_ref::<SlideoutButton>()).collect()
    }

    /// The edge the person drags, on the side toward the window's parts; none for one that is not resizable.
    pub fn edge(&self) -> Option<Rect> {
        self.slide.resizable.then(|| self.slide.side.inner_edge(self.node.bounds, EDGE_WIDTH))
    }
}

impl Element for SlideoutFrame {
    crate::element_node!();

    fn arrange(&mut self, _area: Rect, cx: &mut Cx) {
        let place = self.slide.place(cx.now());
        // While it slides in, every frame is drawn; once it is in, none for it.
        let sliding = self.slide.shown_part(cx.now()) < 1.0;
        if !sliding {
            self.slide.since.set(None);
        }
        if sliding != self.sliding {
            self.sliding = sliding;
            cx.animate(self.node.id(), sliding);
        }
        // While it slides it comes from under its edge of the window's content: what is beyond (the handles) stays in sight and takes presses.
        let band = self.slide.band.get();
        self.node.bounds = match self.slide.side {
            Side::Left => {
                let x = place.x.max(band.x);
                Rect::new(x, place.y, (place.right() - x).max(0.0), place.height)
            }
            Side::Right => Rect::new(place.x, place.y, (place.right().min(band.right()) - place.x).max(0.0), place.height),
            Side::Top => {
                let y = place.y.max(band.y);
                Rect::new(place.x, y, place.width, (place.bottom() - y).max(0.0))
            }
            Side::Bottom => Rect::new(place.x, place.y, place.width, (place.bottom().min(band.bottom()) - place.y).max(0.0)),
        };
        let header = Rect::new(place.x, place.y, place.width, HEADER_HEIGHT);
        let mut right = header.right() - (HEADER_HEIGHT - BUTTON) / 2.0;
        // The buttons from the right: close, then the pin.
        for child in self.node.children_mut().iter_mut().rev() {
            if child.as_any().is::<SlideoutButton>() {
                right -= BUTTON;
                child.arrange(Rect::new(right, header.y + (HEADER_HEIGHT - BUTTON) / 2.0, BUTTON, BUTTON), cx);
                right -= 4.0;
            }
        }
        let inner = Rect::new(place.x + PADDING, place.y + HEADER_HEIGHT, (place.width - 2.0 * PADDING).max(0.0), (place.height - HEADER_HEIGHT - PADDING).max(0.0));
        crate::element::arrange_safely(self.node.children_mut()[0].as_mut(), inner, cx);
    }

    fn cursor_at(&self, x: f64, y: f64) -> Option<Cursor> {
        self.edge().filter(|e| e.contains(x, y)).map(|_| if self.slide.side.across() { Cursor::ResizeUpDown } else { Cursor::ResizeLeftRight })
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, _cx: &mut Cx) {
        if self.edge().is_some_and(|e| e.contains(at.screen_x, at.screen_y)) {
            let along = if self.slide.side.across() { at.screen_y } else { at.screen_x };
            self.dragging = Some((along, self.slide.size()));
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pointer = Some((at.screen_x, at.screen_y));
        cx.invalidate();
        if let (true, Some((from, size))) = (self.node.pressed, self.dragging) {
            let side = self.slide.side;
            let along = if side.across() { at.screen_y } else { at.screen_x };
            let moved = if side.from_start() { along - from } else { from - along };
            self.slide.drag_to(size + moved);
        }
    }

    /// The person let go of its edge: the size they left is kept.
    fn on_pointer_released(&mut self, _at: PointerInput, _cx: &mut Cx) {
        if self.dragging.take().is_some() {
            self.slide.keep();
        }
    }

    fn on_pointer_cancelled(&mut self, _cx: &mut Cx) {
        self.dragging = None;
    }

    fn on_wheel(&mut self, _dx: f64, _dy: f64, _held: Modifiers, _cx: &mut Cx) -> bool {
        true
    }

    /// Leaving the screen, it no longer slides.
    fn on_detached(&mut self, cx: &mut Cx) {
        if self.sliding {
            self.sliding = false;
            cx.animate(self.node.id(), false);
        }
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        // The shadow toward the window, fading out.
        let steps = 6;
        for i in 0..steps {
            let reach = SHADOW * (i + 1) as f64 / steps as f64;
            let shade = match self.slide.side {
                Side::Left => Rect::new(b.right(), b.y, reach, b.height),
                Side::Right => Rect::new(b.x - reach, b.y, reach, b.height),
                Side::Top => Rect::new(b.x, b.bottom(), b.width, reach),
                Side::Bottom => Rect::new(b.x, b.y - reach, b.width, reach),
            };
            p.fill(shade, Color::from_argb(10, 0, 0, 0), 0.0);
        }
        p.fill(b, look.color("panel"), 0.0);
        p.fill(self.slide.side.inner_edge(b, look.border()), look.color("strong_border"), 0.0);
        if let Some(edge) = self.edge().filter(|e| self.node.hovered && self.pointer.is_some_and(|(x, y)| e.contains(x, y)) || self.dragging.is_some()) {
            p.fill(edge, look.color("accent_soft"), 0.0);
        }
        let _clip = p.clip(b);
        let buttons = self.buttons().len() as f64;
        let words = Rect::new(b.x + PADDING, b.y, (b.width - 2.0 * PADDING - buttons * (BUTTON + 4.0)).max(0.0), HEADER_HEIGHT);
        p.text(&self.caption(), words, look.color("text"), look.size("text"), "text", true, TextAlign::Left);
        p.fill(Rect::new(b.x, b.y + HEADER_HEIGHT - look.border(), b.width, look.border()), look.color("border"), 0.0);
        let _framing = container::framing();
        draw_children(&self.node, p);
    }
}

/// A slide-out's pin or close button: a click, Enter or Space pins it open
/// (or lets it slide away again) or closes it. It takes the focus, shows the
/// focus ring, and says what it does.
pub struct SlideoutButton {
    node: Node,
    slide: Rc<Slide>,
    pins: bool,
}

impl SlideoutButton {
    /// Whether it is the pin; else the close button.
    pub fn pins(&self) -> bool {
        self.pins
    }

    fn press(&mut self, cx: &mut Cx) {
        if self.pins {
            self.slide.pin(!self.slide.pinned());
        } else {
            self.slide.close();
        }
        cx.invalidate();
    }
}

impl Element for SlideoutButton {
    crate::element_node!();

    fn focusable(&self) -> bool {
        true
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.press(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !matches!(key.key, Key::Enter | Key::Space) || key.modifiers != Modifiers::NONE {
            return false;
        }
        self.press(cx);
        true
    }

    fn tooltip_at(&self, _x: f64, _y: f64) -> Option<String> {
        Some(match (self.pins, self.slide.pinned()) {
            (true, true) => locale::translate("slideoutUnpin", "Let it slide away"),
            (true, false) => locale::translate("slideoutPin", "Pin it open"),
            (false, _) => locale::translate("slideoutClose", "Close"),
        })
    }

    fn on_pointer_moved(&mut self, _at: PointerInput, cx: &mut Cx) {
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        let hover = self.node.hovered;
        let pinned = self.pins && self.slide.pinned();
        let values = look.part("button", if hover || pinned { Some("hover") } else { None });
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("button"));
        if hover || pinned {
            p.fill(b, values.background.unwrap_or_else(|| look.color("panel")), radius);
        }
        let ink = values.text.unwrap_or_else(|| look.color("text"));
        let (cx, cy) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
        let side = b.width.min(b.height) * 0.22;
        if self.pins {
            // A pin, drawn as lines so every font shows it: its head, its needle, and the bar it is held by.
            p.line((cx - side, cy - side * 0.4), (cx + side, cy - side * 0.4), ink, 1.6);
            p.line((cx - side * 0.6, cy - side * 1.4), (cx + side * 0.6, cy - side * 1.4), ink, 1.6);
            p.line((cx - side * 0.5, cy - side * 1.4), (cx - side * 0.5, cy - side * 0.4), ink, 1.6);
            p.line((cx + side * 0.5, cy - side * 1.4), (cx + side * 0.5, cy - side * 0.4), ink, 1.6);
            p.line((cx, cy - side * 0.4), (cx, cy + side * 1.5), ink, 1.6);
        } else {
            // A cross.
            p.line((cx - side, cy - side), (cx + side, cy + side), ink, 1.6);
            p.line((cx - side, cy + side), (cx + side, cy - side), ink, 1.6);
        }
        if self.node.focused {
            p.focus_ring(b, radius);
        }
    }
}

/// A slide-out's handle: a tab on its edge of the window with its caption
/// written along it (read from top to bottom at the left and the right,
/// as any line at the top and the bottom), there whether it is open or
/// not. A click, Enter or Space slides it out, or back when it is out. It
/// takes the focus, shows the focus ring, and is drawn as chosen while out.
pub struct SlideoutHandle {
    node: Node,
    slide: Rc<Slide>,
}

impl SlideoutHandle {
    pub fn new(slide: Rc<Slide>) -> SlideoutHandle {
        SlideoutHandle { node: Node::new(), slide }
    }

    /// What it shares with its slide-out.
    pub fn slide(&self) -> &Rc<Slide> {
        &self.slide
    }

    /// How long it is along the edge with its caption in `fonts`.
    pub fn length(&self, fonts: &crate::fonts::Fonts, size: f64) -> f64 {
        fonts.measure(&self.slide.caption(), "text", size, false) + 2.0 * HANDLE_PADDING
    }

    fn press(&mut self, cx: &mut Cx) {
        if self.slide.is_open() {
            self.slide.close();
        } else {
            self.slide.open();
        }
        cx.invalidate();
    }
}

impl Element for SlideoutHandle {
    crate::element_node!();

    fn arrange(&mut self, area: Rect, _cx: &mut Cx) {
        self.node.bounds = area;
        self.slide.handle.set(Some(self.node.id()));
    }

    fn focusable(&self) -> bool {
        true
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.press(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !matches!(key.key, Key::Enter | Key::Space) || key.modifiers != Modifiers::NONE {
            return false;
        }
        self.press(cx);
        true
    }

    fn on_pointer_moved(&mut self, _at: PointerInput, cx: &mut Cx) {
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        let open = self.slide.is_open();
        let hover = self.node.hovered;
        if open || hover {
            p.fill(b, look.color(if open { "accent_soft" } else { "panel" }), 0.0);
        }
        // A bar on the side toward the window's parts marks the one that is out.
        if open {
            p.fill(self.slide.side.inner_edge(b, 3.0), look.color("accent"), 0.0);
        }
        let ink = look.color(if open { "text" } else { "muted" });
        if self.slide.side.across() {
            p.text(&self.slide.caption(), b, ink, look.size("small"), "text", false, TextAlign::Center);
        } else {
            p.turned_text(&self.slide.caption(), b, ink, look.size("small"));
        }
        if self.node.focused {
            p.focus_ring(b, 0.0);
        }
    }
}
