//! What a window is made of: elements in a tree, each with its place, which
//! draw themselves and take what the person does. A part is an element; so
//! are the plate and the line of text everything is drawn with.
//!
//! Each element keeps its place, its children and its state on the screen
//! in a `Node`. The screen knows elements by their id; what an element wants
//! of the screen (the focus, an overlay, a wake-up, drawing again) it asks
//! through the `Cx` its handlers get.

use std::any::Any;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::grid::{self, GridItem};
use crate::input::{KeyInput, Modifiers, PointerInput};
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
}

/// What an element asks of its screen while it handles something.
pub enum Request {
    Focus(Option<ElementId>),
    Open(Overlay),
    Close(ElementId),
    Wake(ElementId, Instant),
    Animate(ElementId, bool),
    Later(ElementId, crate::later::Work),
}

/// What an element's handlers reach of the screen: its look, fonts,
/// clipboard and time, and the requests it makes of it.
pub struct Cx {
    look: Arc<ThemeLook>,
    fonts: Rc<Fonts>,
    clipboard: Rc<RefCell<Box<dyn Clipboard>>>,
    now: Instant,
    whole: Rect,
    focused: Option<ElementId>,
    places: Rc<RefCell<std::collections::HashMap<String, (ElementId, Rect)>>>,
    pub(crate) requests: Vec<Request>,
    pub(crate) redraw: bool,
}

impl Cx {
    pub(crate) fn new(look: Arc<ThemeLook>, fonts: Rc<Fonts>, clipboard: Rc<RefCell<Box<dyn Clipboard>>>, now: Instant, whole: Rect, focused: Option<ElementId>) -> Cx {
        Cx { look, fonts, clipboard, now, whole, focused, places: Rc::default(), requests: Vec::new(), redraw: false }
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

    /// As the screen's root, what the person did was handled: it does what that left to do (an edit to send, a window to open).
    fn after_event(&mut self, cx: &mut Cx) {
        let _ = cx;
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

/// Draws the children of a node that show, in order.
pub fn draw_children(node: &Node, painter: &Painter) {
    for child in node.children() {
        if child.is_shown() {
            child.draw(painter);
        }
    }
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

/// The ids of the elements that take the focus and show, in the order they are drawn: what Tab goes through.
pub fn focus_takers(root: &dyn Element) -> Vec<ElementId> {
    fn walk(e: &dyn Element, out: &mut Vec<ElementId>) {
        if !e.is_shown() {
            return;
        }
        if e.focusable() {
            out.push(e.node().id());
        }
        for child in e.node().children() {
            walk(child.as_ref(), out);
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
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
}

impl GridPanel {
    pub fn new(columns: i64) -> GridPanel {
        GridPanel { node: Node::new(), places: Vec::new(), columns: columns.max(1), gap: grid::GAP, content: 0.0 }
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
        for (child, place) in self.node.children_mut().iter_mut().zip(placed.boxes) {
            child.arrange(place.offset(area.x, area.y), cx);
        }
    }

    fn draw(&self, p: &Painter) {
        let _clip = p.clip(self.node.bounds);
        draw_children(&self.node, p);
    }
}
