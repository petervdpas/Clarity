//! What a window is made of: elements in a tree, each with its place, which
//! draw themselves and take what the person does. A part is an element; so
//! are the plate and the line of text everything is drawn with.
//!
//! Each element keeps its place, its children and its state on the screen
//! in a `Node`. The screen knows elements by their id; what an element wants
//! of the screen (the focus, an overlay, a wake-up, drawing again) it asks
//! through the `Cx` its handlers get.

use std::any::Any;
use std::cell::{Cell, RefCell, RefMut};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::grid::{self, GridItem};
use crate::input::{Cursor, KeyInput, Modifiers, PointerInput};
use crate::overlay::{Clipboard, Overlay};
use crate::painter::{Painter, TextAlign};
use crate::theme::ThemeLook;

/// The number an element is known by on its screen, its own for as long as it lives.
pub type ElementId = u64;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// What every element has: its id, its place, its children and its state on the screen.
pub struct Node {
    id: ElementId,
    /// Where it is on the screen, as its parent placed it.
    pub bounds: Rect,
    children: Vec<Box<dyn Element>>,
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    attached: bool,
    /// Why its drawing, placing or input failed the last time; none while it works.
    failed: RefCell<Option<String>>,
    /// Whether it failed to take its place the last time it was placed.
    placing_failed: Cell<bool>,
    /// Whether an element it is in is disabled, as the screen last saw its tree.
    within_disabled: Cell<bool>,
}

impl Default for Node {
    fn default() -> Self {
        Node::new()
    }
}

impl Node {
    pub fn new() -> Node {
        Node {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            bounds: Rect::default(),
            children: Vec::new(),
            hovered: false,
            pressed: false,
            focused: false,
            attached: false,
            failed: RefCell::new(None),
            placing_failed: Cell::new(false),
            within_disabled: Cell::new(false),
        }
    }

    pub fn id(&self) -> ElementId {
        self.id
    }

    pub fn children(&self) -> &[Box<dyn Element>] {
        &self.children
    }

    /// The children to change in place; one taken out this way must be detached (`detach_all`).
    pub fn children_mut(&mut self) -> &mut Vec<Box<dyn Element>> {
        &mut self.children
    }

    /// Adds a child last; on a shown tree it joins the screen once the event is over.
    pub fn add(&mut self, child: Box<dyn Element>) {
        self.children.push(child);
    }

    /// Takes the child at `index` out; it leaves the screen, so what it started stops.
    pub fn remove(&mut self, index: usize, cx: &mut Cx) -> Box<dyn Element> {
        let mut child = self.children.remove(index);
        detach_all(child.as_mut(), cx);
        child
    }

    /// Takes every child out, each leaving the screen.
    pub fn clear(&mut self, cx: &mut Cx) {
        while !self.children.is_empty() {
            self.remove(self.children.len() - 1, cx);
        }
    }

    /// Whether it is on a screen: its tree is the screen's, or an overlay's on it.
    pub fn attached(&self) -> bool {
        self.attached
    }

    /// Why its drawing, placing or input failed the last time; none while it works.
    pub fn failed(&self) -> Option<String> {
        self.failed.borrow().clone()
    }

    /// Whether an element it is in is disabled (a disabled panel disables its parts), as the screen last saw its tree.
    pub fn within_disabled(&self) -> bool {
        self.within_disabled.get()
    }
}

/// What an element asks of its screen while it handles something.
pub enum Request {
    Focus(Option<ElementId>),
    Open(Overlay),
    Close(ElementId),
    Wake(ElementId, Instant),
    Animate(ElementId, bool),
    Later(ElementId, crate::later::Work),
    Ask(ElementId, crate::later::Asked),
}

/// What an element's handlers reach of the screen: its look, fonts,
/// clipboard and time, and the requests it makes of it.
/// The part with the focus, or the first part the focused element is in.
#[derive(Clone, Debug, PartialEq)]
pub struct FocusedPart {
    pub name: String,
    pub button: bool,
}

pub struct Cx {
    look: Arc<ThemeLook>,
    fonts: Rc<Fonts>,
    clipboard: Rc<RefCell<Box<dyn Clipboard>>>,
    now: Instant,
    whole: Rect,
    focused: Option<ElementId>,
    places: Rc<RefCell<std::collections::HashMap<String, (ElementId, Rect)>>>,
    /// The part with the focus, wherever it lies: in the window or in an overlay over it (a slide-out, a popover).
    pub(crate) focused_part: Option<FocusedPart>,
    pub(crate) requests: Vec<Request>,
    pub(crate) redraw: bool,
}

impl Cx {
    pub(crate) fn new(look: Arc<ThemeLook>, fonts: Rc<Fonts>, clipboard: Rc<RefCell<Box<dyn Clipboard>>>, now: Instant, whole: Rect, focused: Option<ElementId>) -> Cx {
        Cx { look, fonts, clipboard, now, whole, focused, places: Rc::default(), focused_part: None, requests: Vec::new(), redraw: false }
    }

    /// The places of the window's named parts as the screen last laid them out.
    pub(crate) fn with_places(mut self, places: Rc<RefCell<std::collections::HashMap<String, (ElementId, Rect)>>>) -> Cx {
        self.places = places;
        self
    }

    /// The id and place of the shown part named `name` in the window, as the
    /// screen last laid it out (a popover lies under the part its `for`
    /// names); none for no such part.
    pub fn part_place(&self, name: &str) -> Option<(ElementId, Rect)> {
        self.places.borrow().get(name).copied()
    }

    /// A context of no screen, in Clarity's light look: for an element used on its own, as in a test.
    pub fn alone() -> Cx {
        let look = ThemeLook::clarity(false);
        let fonts = Rc::new(Fonts::new(look.clone()));
        Cx::new(look, fonts, Rc::new(RefCell::new(Box::new(crate::overlay::OwnClipboard::default()))), Instant::now(), Rect::default(), None)
    }

    pub fn look(&self) -> &ThemeLook {
        &self.look
    }

    pub fn fonts(&self) -> &Fonts {
        &self.fonts
    }

    /// The system's clipboard, as the host gives it.
    pub fn clipboard(&self) -> RefMut<'_, Box<dyn Clipboard>> {
        self.clipboard.borrow_mut()
    }

    /// The time now, by the screen's clock.
    pub fn now(&self) -> Instant {
        self.now
    }

    /// The whole screen, where an overlay that covers it lies.
    pub fn whole(&self) -> Rect {
        self.whole
    }

    /// The element with the focus, as the latest request leaves it.
    pub fn focused(&self) -> Option<ElementId> {
        self.focused
    }

    /// Something changed: the screen draws again.
    pub fn invalidate(&mut self) {
        self.redraw = true;
    }

    /// Gives the focus to `element`, or takes it away.
    pub fn focus(&mut self, element: Option<ElementId>) {
        self.focused = element;
        self.requests.push(Request::Focus(element));
        self.redraw = true;
    }

    /// Opens an overlay over the window; its id is its content's.
    pub fn open(&mut self, overlay: Overlay) -> ElementId {
        let id = overlay.id();
        self.requests.push(Request::Open(overlay));
        self.redraw = true;
        id
    }

    /// Closes the overlay of that id.
    pub fn close(&mut self, overlay: ElementId) {
        self.requests.push(Request::Close(overlay));
        self.redraw = true;
    }

    /// `element` wakes at `at` (its `on_wake` runs then), once; asking again moves its time.
    pub fn wake(&mut self, element: ElementId, at: Instant) {
        self.requests.push(Request::Wake(element, at));
    }

    /// `element` animates (or no longer does): the host draws every frame while one does.
    pub fn animate(&mut self, element: ElementId, on: bool) {
        self.requests.push(Request::Animate(element, on));
        self.redraw = true;
    }

    /// Runs `work` off the screen's thread (a file dialog, a long handler);
    /// what it gives comes back to `element` on the screen's thread, in its
    /// `on_later`, once it is done. The screen goes on drawing meanwhile.
    pub fn later<T: Send + 'static>(&mut self, element: ElementId, work: impl FnOnce() -> T + Send + 'static) {
        let work: crate::later::Work = Box::new(move || Box::new(work()) as crate::later::Done);
        self.requests.push(Request::Later(element, work));
    }

    /// One request of the program for `element`: sent without waiting where
    /// the host can, its answer coming back to `on_later`; else `asked.work`
    /// runs off the screen's thread as `later` does.
    pub fn later_ask(&mut self, element: ElementId, asked: crate::later::Asked) {
        self.requests.push(Request::Ask(element, asked));
    }
}

/// Text an input method is composing, not typed yet: shown at the caret,
/// underlined, with the part of it the method marks.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Composition {
    pub text: String,
    /// The marked part, in bytes of `text`; none for none.
    pub marked: Option<(usize, usize)>,
}

/// An element: it takes its place, draws itself and takes what the person does.
pub trait Element: Any {
    fn node(&self) -> &Node;
    fn node_mut(&mut self) -> &mut Node;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Whether it takes the focus: pressed, or reached with Tab.
    fn focusable(&self) -> bool {
        false
    }

    /// Whether it takes Tab itself (a code editor) instead of Tab moving the focus on.
    fn wants_tab(&self) -> bool {
        false
    }

    /// Whether it shows: one hidden takes no focus, no pointer and draws nothing, and neither does anything in it.
    fn is_shown(&self) -> bool {
        true
    }

    /// Whether the person may use it by its own state; a disabled one takes
    /// no pointer, focus or keys, and neither does anything in it. A part
    /// asks its state.
    fn is_enabled(&self) -> bool {
        self.as_part().is_none_or(|p| p.base().own_enabled())
    }

    /// The order Tab goes through its children, by their places among them
    /// (the window's frame before its grid, wherever the frame is docked);
    /// none for the order they are drawn in.
    fn tab_children(&self) -> Option<Vec<usize>> {
        None
    }

    /// Its place in Tab order among those that take the focus (a part's
    /// `taborder`): lower first, none after those with one.
    fn tab_index(&self) -> Option<i64> {
        self.as_part().and_then(|p| p.base().spec.number("tabOrder")).map(|n| n as i64)
    }

    /// The pointer's shape over a point on the screen (a link's hand, a text
    /// box's I-beam); none to leave it to the element it is in.
    fn cursor_at(&self, x: f64, y: f64) -> Option<Cursor> {
        let _ = (x, y);
        None
    }

    /// The items of its menu for a right press at a point on the screen (a
    /// part's `menu`, a text box's Cut, Copy, Paste and Select all); none to
    /// leave it to the element it is in. The screen opens them as a menu at
    /// the pointer, and an item runs once the menu closed.
    fn context_menu_at(&self, x: f64, y: f64) -> Option<Vec<crate::frame::menus::MenuEntry>> {
        let _ = (x, y);
        self.as_part().and_then(crate::parts::designed_menu)
    }

    /// The look it draws with: the window's, or one its style changes (a
    /// part's `style`); it may draw what its style puts under it here. None
    /// for the window's own.
    fn styled(&self, painter: &Painter) -> Option<crate::painter::StyledLook> {
        let part = self.as_part()?;
        if part.draws_own_style() {
            return None;
        }
        crate::parts::style_of(part.base(), painter)
    }

    /// Its type's own name, as a failure says which element failed.
    fn type_name(&self) -> &'static str {
        let full = std::any::type_name::<Self>();
        full.rsplit("::").next().unwrap_or(full)
    }

    /// The room it needs beyond the rows it spans in a grid.
    fn around(&self) -> f64 {
        0.0
    }

    /// Whether a point on the screen is its own; a part may say a point within its place is not.
    fn hit(&self, x: f64, y: f64) -> bool {
        self.node().bounds.contains(x, y)
    }

    /// Takes its place; a container places its children in it.
    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        let _ = cx;
        self.node_mut().bounds = area;
    }

    /// Draws itself; an element draws the children that show.
    fn draw(&self, painter: &Painter) {
        draw_children(self.node(), painter);
    }

    /// Pressed and released with the main button.
    fn on_click(&mut self, cx: &mut Cx) {
        let _ = cx;
    }

    /// The main button went down on it; it holds the pointer until the button goes up.
    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        let _ = (at, cx);
    }

    /// The pointer moved over it, or anywhere while it holds the pointer.
    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let _ = (at, cx);
    }

    /// The main button went up after it was pressed on it, wherever the pointer is.
    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        let _ = (at, cx);
    }

    /// Whether a right press at a point (from its top left) is a drag it
    /// takes, as the left one is (a canvas draws a new line with it); else
    /// the right press opens its menu.
    fn right_drags(&self, x: f64, y: f64) -> bool {
        let _ = (x, y);
        false
    }

    /// The right button went up after a press it took: true when that was a
    /// drag it used, false to open its menu where it was pressed.
    fn on_right_released(&mut self, at: PointerInput, cx: &mut Cx) -> bool {
        let _ = (at, cx);
        false
    }

    /// The pointer it held was taken away before the button went up (the
    /// window lost the focus mid-drag): what the press started ends without
    /// its release and without a click.
    fn on_pointer_cancelled(&mut self, cx: &mut Cx) {
        let _ = cx;
    }

    /// The wheel turned over it with keys `held`: true when it took the turn; else its parent may.
    fn on_wheel(&mut self, dx: f64, dy: f64, held: Modifiers, cx: &mut Cx) -> bool {
        let _ = (dx, dy, held, cx);
        false
    }

    /// It got or lost the focus.
    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        let _ = (focused, cx);
    }

    /// A key, while it has the focus; true when it took the key.
    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let _ = (key, cx);
        false
    }

    /// A key let go, while it has the focus.
    fn on_key_up(&mut self, key: KeyInput, cx: &mut Cx) {
        let _ = (key, cx);
    }

    /// Text typed while it has the focus.
    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        let _ = (text, cx);
    }

    /// The time it asked for came (`Cx::wake`).
    fn on_wake(&mut self, cx: &mut Cx) {
        let _ = cx;
    }

    /// It joined a screen: it may start what runs while it is there.
    fn on_attached(&mut self, cx: &mut Cx) {
        let _ = cx;
    }

    /// It left its screen: it stops what it started.
    fn on_detached(&mut self, cx: &mut Cx) {
        let _ = cx;
    }

    /// What it says about the place at a point when the pointer rests there; none for nothing.
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        let _ = (x, y);
        None
    }

    /// As the content of a dialog on top, a key before the part with the focus: true when it took it.
    fn dialog_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let _ = (key, cx);
        false
    }

    /// As the screen's root, the window's own keys before the part with the focus (shortcuts, the default button): true when it took the key.
    fn window_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let _ = (key, cx);
        false
    }

    /// As the screen's root, a key let go before the part with the focus hears it (Alt for the menu bar's access letters): true when it took it.
    fn window_key_up(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let _ = (key, cx);
        false
    }

    /// As the screen's root, what the person did was handled: it does what that left to do (an edit to send, a window to open).
    fn after_event(&mut self, cx: &mut Cx) {
        let _ = cx;
    }

    /// As the screen's root, the focus moved to `focused` (none: nothing has
    /// it); `pointing` when a press of the pointer moved it, not Tab or code.
    /// A scrolling window shows the part with the focus, unless pointing.
    fn focus_moved(&mut self, focused: Option<ElementId>, pointing: bool, cx: &mut Cx) {
        let _ = (focused, pointing, cx);
    }

    /// Whether it is an input the person types in: a letter is then its, not a shortcut's.
    fn typing(&self) -> bool {
        false
    }

    /// Whether Enter makes a new line in it (a memo), so it presses no default button.
    fn multiline(&self) -> bool {
        false
    }

    /// An input that takes what is typed in it only on leaving or Enter takes it now: true when it is one.
    fn take_typed(&mut self, cx: &mut Cx) -> bool {
        let _ = cx;
        false
    }

    /// Itself as a part of a window, when it is one.
    fn as_part(&self) -> Option<&dyn crate::parts::Part> {
        None
    }

    /// Itself as a part of a window, to change.
    fn as_part_mut(&mut self) -> Option<&mut dyn crate::parts::Part> {
        None
    }

    /// An overlay it opened closed.
    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        let _ = (overlay, cx);
    }

    /// What work it started with `Cx::later` gave, back on the screen's thread.
    fn on_later(&mut self, done: crate::later::Done, cx: &mut Cx) {
        let _ = (done, cx);
    }

    /// An input method composes text in it while it has the focus (none: the
    /// composing ended); what it commits comes as typed text (`on_text`).
    fn on_compose(&mut self, composing: Option<Composition>, cx: &mut Cx) {
        let _ = (composing, cx);
    }

    /// Where its caret is on the screen, while it has the focus and takes
    /// typed text: an input method shows its window by it. None for none.
    fn caret_area(&self) -> Option<Rect> {
        None
    }
}

/// Writes the four methods every element has the same way, for a struct with a field `node`.
#[macro_export]
macro_rules! element_node {
    () => {
        fn node(&self) -> &$crate::element::Node {
            &self.node
        }
        fn node_mut(&mut self) -> &mut $crate::element::Node {
            &mut self.node
        }
        fn as_any(&self) -> &dyn ::std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
            self
        }
    };
}

/// Draws the children of a node that show, in order, each fail-safe.
pub fn draw_children(node: &Node, painter: &Painter) {
    for child in node.children() {
        if child.is_shown() {
            draw_safely(child.as_ref(), painter);
        }
    }
}

/// What a panic carried, as a sentence: its message, else that it failed.
pub fn panic_text(panic: &(dyn Any + Send)) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| crate::locale::translate("elementFailedUnknown", "it failed"))
}

/// It failed in `doing`: it keeps why, and says so once on the error output.
fn fail(element: &dyn Element, doing: &str, why: String) {
    let node = element.node();
    if node.failed.borrow().as_deref() != Some(why.as_str()) {
        eprintln!("{} failed in {doing}: {why}", element.type_name());
    }
    *node.failed.borrow_mut() = Some(why);
}

/// Draws it, fail-safe: a drawing that fails shows why in its own place, in
/// the theme's error color, and the rest of the window draws on. Its style,
/// when it has one, is the look it draws in.
pub fn draw_safely(element: &dyn Element, painter: &Painter) {
    // One that could not take its place says why there instead of drawing half placed.
    if element.node().placing_failed.get() {
        draw_failure(element, painter);
        return;
    }
    let saved = painter.canvas.save_count();
    let drawn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match element.styled(painter) {
        Some(styled) => element.draw(&styled.painter(painter)),
        None => element.draw(painter),
    }));
    if let Err(panic) = drawn {
        painter.canvas.restore_to_count(saved);
        fail(element, "drawing", panic_text(panic.as_ref()));
        draw_failure(element, painter);
    }
}

/// Takes its place, fail-safe: one that fails to place what it holds keeps the place and says why.
pub fn arrange_safely(element: &mut dyn Element, area: Rect, cx: &mut Cx) {
    let placed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| element.arrange(area, cx)));
    match placed {
        Ok(()) => element.node().placing_failed.set(false),
        Err(panic) => {
            element.node_mut().bounds = area;
            element.node().placing_failed.set(true);
            fail(element, "placing", panic_text(panic.as_ref()));
        }
    }
}

/// Runs what the person did on it, fail-safe: an element that fails to take
/// a press or a key says why in its place, and the window keeps working;
/// `otherwise` is what it gives then.
pub fn safely<R>(element: &mut dyn Element, cx: &mut Cx, otherwise: R, input: impl FnOnce(&mut dyn Element, &mut Cx) -> R) -> R {
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| input(&mut *element, &mut *cx)));
    match ran {
        Ok(given) => given,
        Err(panic) => {
            fail(element, "input", panic_text(panic.as_ref()));
            cx.invalidate();
            otherwise
        }
    }
}

/// As `safely`, for what reads an element and changes nothing (its cursor, its menu).
pub fn safely_read<R>(element: &dyn Element, otherwise: R, read: impl FnOnce(&dyn Element) -> R) -> R {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| read(element))) {
        Ok(given) => given,
        Err(panic) => {
            fail(element, "input", panic_text(panic.as_ref()));
            otherwise
        }
    }
}

/// Why it failed, in its place: a red outline and the reason in small text.
fn draw_failure(element: &dyn Element, p: &Painter) {
    let shown = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let bounds = element.node().bounds;
        let why = element.node().failed().unwrap_or_default();
        let _clip = p.clip(bounds);
        p.stroke(bounds, p.look.color("error"), 1.0, p.look.radius("input"));
        p.text(&format!("{}: {why}", element.type_name()), bounds.inset(6.0), p.look.color("error"), p.look.size("small"), "text", false, TextAlign::Left);
    }));
    if let Err(panic) = shown {
        eprintln!("{} could not say why it failed: {}", element.type_name(), panic_text(panic.as_ref()));
    }
}

/// Marks for each element of a tree whether an element it is in is
/// disabled, so a part in a disabled panel draws and takes nothing as one.
pub fn mark_enabled(element: &dyn Element, within_disabled: bool) {
    element.node().within_disabled.set(within_disabled);
    let disabled = within_disabled || !element.is_enabled();
    for child in element.node().children() {
        mark_enabled(child.as_ref(), disabled);
    }
}

/// Whether the person may use it on its screen: it and every element it is
/// in are enabled, as the screen last saw its tree.
pub fn enabled_in_tree(element: &dyn Element) -> bool {
    element.is_enabled() && !element.node().within_disabled.get()
}

/// The element of that id in a tree (itself included); none when it is not there.
pub fn find(root: &dyn Element, id: ElementId) -> Option<&dyn Element> {
    if root.node().id() == id {
        return Some(root);
    }
    root.node().children().iter().find_map(|c| find(c.as_ref(), id))
}

/// The element of that id in a tree, to change.
pub fn find_mut(root: &mut dyn Element, id: ElementId) -> Option<&mut dyn Element> {
    if root.node().id() == id {
        return Some(root);
    }
    for child in root.node_mut().children_mut().iter_mut() {
        if let Some(found) = find_mut(child.as_mut(), id) {
            return Some(found);
        }
    }
    None
}

/// The ids from the root down to the element of that id; none when it is not in the tree.
pub fn chain(root: &dyn Element, id: ElementId) -> Option<Vec<ElementId>> {
    if root.node().id() == id {
        return Some(vec![id]);
    }
    for child in root.node().children() {
        if let Some(mut below) = chain(child.as_ref(), id) {
            below.insert(0, root.node().id());
            return Some(below);
        }
    }
    None
}

/// Whether the element of that id shows: it and every element it is in show.
pub fn shows_in_tree(root: &dyn Element, id: ElementId) -> bool {
    match chain(root, id) {
        Some(ids) => ids.iter().all(|i| find(root, *i).is_some_and(|e| e.is_shown())),
        None => false,
    }
}

/// The deepest shown element at a point: the last drawn first.
pub fn at(root: &dyn Element, x: f64, y: f64) -> Option<ElementId> {
    if !root.is_shown() || !root.hit(x, y) {
        return None;
    }
    for child in root.node().children().iter().rev() {
        if let Some(inside) = at(child.as_ref(), x, y) {
            return Some(inside);
        }
    }
    Some(root.node().id())
}

/// Every element of a tree in the order they are drawn, a parent before its children.
pub fn in_order(root: &dyn Element) -> Vec<&dyn Element> {
    let mut all = vec![root];
    for child in root.node().children() {
        all.extend(in_order(child.as_ref()));
    }
    all
}

/// The ids of the elements Tab reaches, in Tab order: those that take the
/// focus, show and are enabled, in the order they are drawn, except that a
/// `taborder` sorts one among them: lower first, those without one after,
/// in tree order. One without its own goes by that of the part it is in (a
/// text box's editor by the text box's).
pub fn focus_takers(root: &dyn Element) -> Vec<ElementId> {
    fn walk(e: &dyn Element, index: Option<i64>, enabled: bool, out: &mut Vec<(Option<i64>, ElementId)>) {
        if !e.is_shown() {
            return;
        }
        let enabled = enabled && e.is_enabled();
        let index = e.tab_index().or(index);
        if e.focusable() && enabled {
            out.push((index, e.node().id()));
        }
        let children = e.node().children();
        let order = e.tab_children().unwrap_or_else(|| (0..children.len()).collect());
        for child in order.into_iter().filter_map(|i| children.get(i)) {
            walk(child.as_ref(), index, enabled, out);
        }
    }
    let mut out = Vec::new();
    walk(root, None, true, &mut out);
    // A stable sort keeps the tree order among those of the same place.
    out.sort_by_key(|(index, _)| index.unwrap_or(i64::MAX));
    out.into_iter().map(|(_, id)| id).collect()
}

/// The first element of type `T` in a tree (itself included) that `where_` holds for.
pub fn find_type<T: Element>(root: &dyn Element, where_: impl Fn(&T) -> bool) -> Option<&T> {
    in_order(root).into_iter().filter_map(|e| e.as_any().downcast_ref::<T>()).find(|t| where_(t))
}

/// The first element of type `T` in a tree, to change.
pub fn find_type_mut<'a, T: Element>(root: &'a mut dyn Element, where_: &dyn Fn(&T) -> bool) -> Option<&'a mut T> {
    let fits = root.as_any().downcast_ref::<T>().is_some_and(where_);
    if fits {
        return root.as_any_mut().downcast_mut::<T>();
    }
    for child in root.node_mut().children_mut().iter_mut() {
        if let Some(found) = find_type_mut(child.as_mut(), where_) {
            return Some(found);
        }
    }
    None
}

/// It and everything in it join the screen, each told once, a parent first.
pub fn attach_all(element: &mut dyn Element, cx: &mut Cx) {
    if !element.node().attached {
        element.node_mut().attached = true;
        element.on_attached(cx);
    }
    for child in element.node_mut().children_mut().iter_mut() {
        attach_all(child.as_mut(), cx);
    }
}

/// It and everything in it leave the screen, each told once, its children first.
pub fn detach_all(element: &mut dyn Element, cx: &mut Cx) {
    for child in element.node_mut().children_mut().iter_mut() {
        detach_all(child.as_mut(), cx);
    }
    if element.node().attached {
        element.node_mut().attached = false;
        element.on_detached(cx);
    }
}

/// Has the input the focus is in, or the first one it is in, take what is typed in it.
pub fn take_typed_from(root: &mut dyn Element, focused: Option<ElementId>, cx: &mut Cx) {
    let Some(focused) = focused else { return };
    let Some(ids) = chain(root, focused) else { return };
    for id in ids.iter().rev() {
        if let Some(e) = find_mut(root, *id) {
            if e.take_typed(cx) {
                return;
            }
        }
    }
}

/// A filled place in a color role of the theme, with the radius of a radius role.
pub struct Plate {
    node: Node,
    color: String,
    radius: Option<String>,
}

impl Plate {
    pub fn new(color: &str, radius: Option<&str>) -> Plate {
        Plate { node: Node::new(), color: color.to_string(), radius: radius.map(str::to_string) }
    }
}

impl Element for Plate {
    element_node!();

    fn draw(&self, p: &Painter) {
        let radius = self.radius.as_deref().map(|r| p.look.radius(r)).unwrap_or(0.0);
        p.fill(self.node.bounds, p.look.color(&self.color), radius);
    }
}

/// A line of text, in the theme's text color and size unless told otherwise.
pub struct TextLine {
    node: Node,
    pub text: String,
    pub color: String,
    pub font_role: String,
    pub size_role: String,
    pub bold: bool,
    pub align: TextAlign,
}

impl TextLine {
    pub fn new(text: &str) -> TextLine {
        TextLine {
            node: Node::new(),
            text: text.to_string(),
            color: "text".to_string(),
            font_role: "text".to_string(),
            size_role: "text".to_string(),
            bold: false,
            align: TextAlign::Left,
        }
    }
}

impl Element for TextLine {
    element_node!();

    fn draw(&self, p: &Painter) {
        p.text(&self.text, self.node.bounds, p.look.color(&self.color), p.look.size(&self.size_role), &self.font_role, self.bold, self.align);
    }
}

/// Elements at their places in a grid (conformance/grid-layout.json).
pub struct GridPanel {
    node: Node,
    places: Vec<GridItem>,
    pub columns: i64,
    pub gap: f64,
    /// How high its content is; more than its place when it needs to scroll.
    pub content: f64,
    /// How high each of its rows is, as last placed.
    pub rows: Vec<f64>,
    /// What it is in the design it was made from (`designItem` in the
    /// markup the IDE's designer draws): the window, a container or a page
    /// that holds the grid. None in a program's own markup.
    pub item: Option<String>,
}

impl GridPanel {
    pub fn new(columns: i64) -> GridPanel {
        GridPanel { node: Node::new(), places: Vec::new(), columns: columns.max(1), gap: grid::GAP, content: 0.0, rows: Vec::new(), item: None }
    }

    /// The grid as held by the design's `item` (see `item`).
    pub fn owned(mut self, item: Option<&str>) -> GridPanel {
        self.item = item.map(str::to_string);
        self
    }

    pub fn add(&mut self, part: Box<dyn Element>, place: GridItem) {
        self.node.add(part);
        self.places.push(place);
    }

    /// Where its children are, by the order they were added.
    pub fn places(&self) -> &[GridItem] {
        &self.places
    }

    /// Places its children in `area` without drawing: how high its content is.
    pub fn content_for(&self, width: f64, height: f64) -> f64 {
        let asked: Vec<GridItem> = self.asked();
        grid::place(&asked, self.columns, width, height, self.gap).content
    }

    fn asked(&self) -> Vec<GridItem> {
        // Each part says the room it needs around its rows now, as its look may have changed.
        self.places
            .iter()
            .zip(self.node.children())
            .map(|(p, c)| GridItem { around: p.around.max(c.around()), ..*p })
            .collect()
    }
}

impl Element for GridPanel {
    element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        let placed = grid::place(&self.asked(), self.columns, area.width, area.height, self.gap);
        self.content = placed.content;
        self.rows = placed.rows;
        for (child, place) in self.node.children_mut().iter_mut().zip(placed.boxes) {
            arrange_safely(child.as_mut(), place.offset(area.x, area.y), cx);
        }
    }

    fn draw(&self, p: &Painter) {
        let _clip = p.clip(self.node.bounds);
        draw_children(&self.node, p);
    }
}
