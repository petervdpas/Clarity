//! A whole window of the program (DESIGN 7.9): its parts in the window's
//! grid, with the content padding every stack keeps around it, scrolling
//! when the window is smaller than its rows. The window's own keys are here
//! too: a part's shortcut or one of its menu's items, Enter for its default
//! button and Escape for its cancel button. The frame around the parts
//! (menu bar, toolbar, ribbon, sidebars, footer) comes with G8b.

use std::rc::Rc;

use crate::element::{self, Cx, Element, ElementId, GridPanel, Node};
use crate::geometry::Rect;
use crate::grid::ROW_HEIGHT;
use crate::input::{Key, KeyInput, Modifiers};
use crate::markup::{CommandSpec, WindowSpec};
use crate::painter::{Painter, TextAlign};
use crate::parts::{self, PartHost};
use crate::shortcuts;

/// The space around the grid of a running window (clarity_spec::layout CONTENT_PADDING).
pub const CONTENT_PADDING: f64 = 12.0;

pub struct WindowView {
    node: Node,
    host: Rc<dyn PartHost>,
    /// The window's name (`Main`), by which the person's keys know its commands.
    pub name: Option<String>,
    /// Why the window shows nothing: its markup could not be read.
    pub problem: Option<String>,
    /// How far the window's content is scrolled down, and how far it can be.
    pub scroll_y: f64,
    pub max_scroll_y: f64,
}

/// A part that may answer a key: where it is, its name and what its markup says of keys.
struct Keyed {
    id: ElementId,
    name: Option<String>,
    shortcut: Option<String>,
    button: bool,
    default: bool,
    cancel: bool,
    menu: Vec<CommandSpec>,
}

impl WindowView {
    /// The window of `spec` over `host`: each of its parts made by its kind, at its place in the grid.
    pub fn new(spec: &WindowSpec, host: Rc<dyn PartHost>) -> WindowView {
        let mut grid = GridPanel::new(spec.columns);
        for part in &spec.parts {
            let made = parts::make(part.clone(), host.clone());
            let place = parts::place_of(made.as_ref());
            grid.add(made, place);
        }
        let mut node = Node::new();
        node.add(Box::new(grid));
        WindowView { node, host, name: spec.name.clone(), problem: None, scroll_y: 0.0, max_scroll_y: 0.0 }
    }

    /// A window whose markup could not be read: it says why in its place.
    pub fn failed(problem: String, host: Rc<dyn PartHost>) -> WindowView {
        let spec = WindowSpec { frame: Vec::new(), parts: Vec::new(), name: None, columns: crate::markup::STANDARD_COLUMNS };
        WindowView { problem: Some(problem), ..WindowView::new(&spec, host) }
    }

    pub fn host(&self) -> &Rc<dyn PartHost> {
        &self.host
    }

    fn grid(&self) -> &GridPanel {
        self.node.children()[0].as_any().downcast_ref::<GridPanel>().expect("a window view holds its grid")
    }

    /// The place of the grid: inside the padding.
    fn inside(&self) -> Rect {
        self.node.bounds.inset(CONTENT_PADDING)
    }

    /// How high the window's rows are with no spare height: what a window that names no height needs.
    pub fn content_height(&self, width: f64) -> f64 {
        self.grid().content_for((width - 2.0 * CONTENT_PADDING).max(0.0), 0.0) + 2.0 * CONTENT_PADDING
    }

    /// The parts that show and take clicks, in the order they are drawn.
    fn keyed(&self) -> Vec<Keyed> {
        fn walk(e: &dyn Element, out: &mut Vec<Keyed>) {
            if !e.is_shown() {
                return;
            }
            if let Some(part) = e.as_part() {
                let base = part.base();
                if base.enabled() {
                    let spec = &base.spec;
                    out.push(Keyed {
                        id: e.node().id(),
                        name: spec.name().map(str::to_string),
                        shortcut: spec.text("shortcut").filter(|s| !s.is_empty()).map(str::to_string),
                        button: spec.kind == "button",
                        default: spec.flag("default"),
                        cancel: spec.flag("cancel"),
                        menu: spec.menu.as_ref().map(|m| all_items(&m.items)).unwrap_or_default(),
                    });
                }
            }
            for child in e.node().children() {
                walk(child.as_ref(), out);
            }
        }
        let mut out = Vec::new();
        walk(self.grid(), &mut out);
        out
    }

    /// The keys a part's command answers: the person's where they set their own, else its designed one.
    fn keys_of(&self, part: &Keyed) -> Vec<String> {
        match (&self.name, &part.name) {
            // A window's own command is known by `Window.name`: both are the project's own names.
            (Some(window), Some(name)) => self.host.keys_of(&format!("{window}.{name}"), part.shortcut.as_deref()),
            _ => part.shortcut.iter().cloned().collect(),
        }
    }

    fn press(&mut self, id: ElementId, cx: &mut Cx) {
        if let Some(part) = element::find_mut(self, id).and_then(|e| e.as_part_mut()) {
            part.press(cx);
        }
    }

    /// A key, before the part with the focus has it: a shortcut of a shown,
    /// enabled part runs it, then one of a part's menu items; Enter presses
    /// the default button and Escape the cancel button, once the part with
    /// the focus did not take them. True when the window took the key, or
    /// asked the part with the focus for it.
    pub fn key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let focused = cx.focused();
        let typing = focused.and_then(|f| element::find(self, f)).is_some_and(|e| e.typing());
        let keyed = self.keyed();
        for part in &keyed {
            if self.keys_of(part).iter().any(|k| shortcuts::matches(k, &key, typing)) {
                // What is typed in the input with the focus is taken first, as a click on the button does.
                element::take_typed_from(self, focused, cx);
                self.press(part.id, cx);
                return true;
            }
        }
        for part in &keyed {
            for item in &part.menu {
                if item.text("shortcut").is_some_and(|s| !s.is_empty() && shortcuts::matches(s, &key, typing)) {
                    element::take_typed_from(self, focused, cx);
                    parts::pick(self.host.as_ref(), part.name.as_deref(), item);
                    return true;
                }
            }
        }
        if key.modifiers != Modifiers::NONE || !matches!(key.key, Key::Enter | Key::Escape) {
            return false;
        }
        // The part with the focus is asked here, so the screen asks it no more.
        if let Some(e) = focused.and_then(|f| element::find_mut(self, f)) {
            if e.on_key(key, cx) {
                return true;
            }
            if key.key == Key::Enter && e.multiline() {
                return true;
            }
        }
        let wanted = |p: &&Keyed| p.button && if key.key == Key::Enter { p.default } else { p.cancel };
        if let Some(pressed) = keyed.iter().find(wanted) {
            element::take_typed_from(self, focused, cx);
            self.press(pressed.id, cx);
        }
        true
    }
}

/// A menu's items and the items under them.
fn all_items(items: &[CommandSpec]) -> Vec<CommandSpec> {
    let mut all = Vec::new();
    for item in items {
        all.push(item.clone());
        all.extend(all_items(&item.items));
    }
    all
}

impl Element for WindowView {
    crate::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        let inside = self.inside();
        let content = self.grid().content_for(inside.width, inside.height);
        self.max_scroll_y = (content - inside.height).max(0.0);
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_y);
        let place = if self.max_scroll_y > 0.0 { Rect::new(inside.x, inside.y - self.scroll_y, inside.width, content) } else { inside };
        self.node.children_mut()[0].arrange(place, cx);
    }

    fn draw(&self, p: &Painter) {
        let inside = self.inside();
        {
            let _clip = p.clip(inside);
            self.grid().draw(p);
        }
        let content = self.grid().content;
        if self.max_scroll_y > 0.0 && content > 0.0 {
            // A thin bar at the right says how much of the content shows, and where.
            let shown = inside.height / content;
            let bar = Rect::new(self.node.bounds.right() - CONTENT_PADDING / 2.0 - 2.0, inside.y + inside.height * (self.scroll_y / content), 4.0, (inside.height * shown).max(16.0));
            p.fill(bar, p.look.color("strong_border"), 2.0);
        }
        if let Some(problem) = &self.problem {
            p.text(problem, Rect::new(inside.x, inside.y, inside.width, ROW_HEIGHT), p.look.color("error"), p.look.size("text"), "text", false, TextAlign::Left);
        }
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        let to = (self.scroll_y + dy).clamp(0.0, self.max_scroll_y);
        if to == self.scroll_y {
            return false;
        }
        self.scroll_y = to;
        cx.invalidate();
        true
    }

    fn window_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.key(key, cx)
    }
}
