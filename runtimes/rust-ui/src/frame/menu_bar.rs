//! The menu bar at the top of a window: its menus by their captions. A
//! press on one opens its menu below it, and another press closes it; while
//! one is open, the pointer over another opens that one. A top item without
//! items runs as a click. The bar hides and is greyed as its part state
//! says, and so does each item. Each top item has an access letter (the
//! first of its caption no item before it took), underlined while Alt is
//! held or after Alt went alone; Alt with it opens the item's menu. F10, or
//! Alt alone, gives the bar the focus (`focus_first`) with its first top
//! item marked: Left and Right move the mark round the bar, Down, Enter or
//! Space open its menu with the first item marked, Escape gives the focus
//! back. In an open menu Left and Right open the menu beside it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::element::{Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::markup::{CommandSpec, MenuSpec};
use crate::painter::{Painter, TextAlign};

use super::{choices, over, owner, plate, rules, Access, WindowFrame};

/// How high the menu bar is.
pub const HEIGHT: f64 = 30.0;

const PADDING: f64 = 10.0;

/// What the bar and the menus it opens share: the frame, its id, where its
/// top items are, the one marked while it has the focus from the keyboard,
/// and the element that had the focus before it, to give it back.
struct BarCore {
    frame: Rc<WindowFrame>,
    id: ElementId,
    placed: RefCell<Vec<(usize, Rect)>>,
    marked: Cell<Option<usize>>,
    before: Cell<Option<ElementId>>,
}

impl BarCore {
    fn menu(&self) -> &MenuSpec {
        &self.frame.shown().menu.as_ref().expect("a menu bar has its menu").value
    }

    fn shows(&self) -> bool {
        let host = self.frame.host().as_ref();
        rules::shows(host, self.menu().name.as_deref()) && self.menu().items.iter().any(|i| !rules::is_separator(i) && rules::shows(host, i.text("name")))
    }

    fn enabled(&self) -> bool {
        rules::takes(self.frame.host().as_ref(), self.menu().name.as_deref())
    }

    fn opens(item: &CommandSpec) -> bool {
        !item.items.is_empty() || rules::picker_of(item).is_some()
    }

    fn place_of(&self, at: usize) -> Option<Rect> {
        self.placed.borrow().iter().find(|(i, _)| *i == at).map(|(_, r)| *r)
    }

    /// The top items the keyboard reaches: shown, no separator, taking clicks.
    fn reachable(&self) -> Vec<(usize, Rect)> {
        let host = self.frame.host().as_ref();
        self.placed.borrow().iter().copied().filter(|(i, _)| {
            let item = &self.menu().items[*i];
            !rules::is_separator(item) && rules::takes(host, item.text("name"))
        }).collect()
    }

    /// The top item `by` places on from `from`, round the ends.
    fn beside(&self, from: Option<usize>, by: i64) -> Option<(usize, Rect)> {
        let reachable = self.reachable();
        if reachable.is_empty() {
            return None;
        }
        let count = reachable.len() as i64;
        let at = match reachable.iter().position(|(i, _)| Some(*i) == from) {
            Some(at) => at as i64 + by,
            None => 0,
        };
        Some(reachable[at.rem_euclid(count) as usize])
    }

    /// Each placed top item's access letter: its place in the caption, by the item's index.
    fn access_keys(&self) -> Vec<(usize, Option<usize>)> {
        let host = self.frame.host().as_ref();
        let items: Vec<usize> = self.placed.borrow().iter().map(|(i, _)| *i).filter(|i| !rules::is_separator(&self.menu().items[*i])).collect();
        let captions: Vec<String> = items.iter().map(|i| host.text(self.menu().items[*i].text("caption"))).collect();
        items.into_iter().zip(rules::access_keys(&captions)).collect()
    }

    /// Opens the menu of the top item at `at` below it, or closes it when it is the one open; one without a menu runs.
    fn press(self: &Rc<Self>, at: usize, area: Rect, cx: &mut Cx) {
        let item = self.menu().items[at].clone();
        if !self.enabled() || !rules::takes(self.frame.host().as_ref(), item.text("name")) {
            return;
        }
        if !Self::opens(&item) {
            self.frame.run(&item, area, (self.id, at), cx);
            return;
        }
        if self.frame.menus().owner() == Some((self.id, at)) {
            self.frame.menus().close_all(cx);
            return;
        }
        self.open(at, area, false, cx);
    }

    /// Opens the menu of the top item at `at` below it; `keyed`, its first item marked.
    fn open(self: &Rc<Self>, at: usize, area: Rect, keyed: bool, cx: &mut Cx) {
        let item = &self.menu().items[at];
        let entries = match rules::picker_of(item) {
            Some(target) if item.items.is_empty() => choices(&self.frame.picker_of(target)),
            _ => self.frame.entries_of(&item.items),
        };
        let menus = self.frame.menus();
        if keyed {
            menus.open_marked((self.id, at), entries, area, Some(self.id), cx);
        } else {
            menus.open((self.id, at), entries, area, Some(self.id), cx);
        }
        let core = self.clone();
        menus.set_beside(Rc::new(move |by, cx| core.open_beside(by, cx)));
        cx.invalidate();
    }

    /// From an open menu of the bar the person went `by` sideways: the menu
    /// beside opens, its first item marked; a top item without one is marked on the bar.
    fn open_beside(self: &Rc<Self>, by: i64, cx: &mut Cx) {
        let Some((region, open)) = self.frame.menus().owner() else { return };
        if region != self.id {
            return;
        }
        let Some((next, area)) = self.beside(Some(open), by) else { return };
        if Self::opens(&self.menu().items[next]) {
            self.open(next, area, true, cx);
        } else {
            self.frame.menus().close_all(cx);
            self.mark(next, cx);
        }
    }

    fn mark(&self, at: usize, cx: &mut Cx) {
        self.marked.set(Some(at));
        cx.focus(Some(self.id));
        cx.invalidate();
    }

    /// The focus goes back to where it was before the bar took it.
    fn leave(&self, cx: &mut Cx) {
        self.marked.set(None);
        let back = self.before.take();
        cx.focus(back);
        cx.invalidate();
    }

    fn focus_first(&self, cx: &mut Cx) -> bool {
        if !self.shows() || !self.enabled() {
            return false;
        }
        let Some((first, _)) = self.reachable().first().copied() else { return false };
        self.frame.menus().close_all(cx);
        if cx.focused() != Some(self.id) {
            self.before.set(cx.focused());
        }
        self.mark(first, cx);
        true
    }

    fn open_by_letter(self: &Rc<Self>, letter: char, cx: &mut Cx) -> bool {
        if !self.shows() || !self.enabled() {
            return false;
        }
        let host = self.frame.host().clone();
        let found = self.access_keys().into_iter().find(|(i, at)| {
            let caption = host.text(self.menu().items[*i].text("caption"));
            at.and_then(|at| caption.chars().nth(at)).is_some_and(|c| c.to_ascii_lowercase() == letter)
        });
        let Some((i, _)) = found else { return false };
        let Some(area) = self.place_of(i) else { return false };
        if self.frame.menus().owner() == Some((self.id, i)) {
            return true;
        }
        self.frame.menus().close_all(cx);
        self.press(i, area, cx);
        true
    }
}

pub struct FrameMenuBar {
    node: Node,
    core: Rc<BarCore>,
    pointer: Option<(f64, f64)>,
}

impl FrameMenuBar {
    pub fn new(frame: Rc<WindowFrame>) -> FrameMenuBar {
        let node = Node::new();
        let core = Rc::new(BarCore { frame, id: node.id(), placed: RefCell::default(), marked: Cell::new(None), before: Cell::new(None) });
        FrameMenuBar { node, core, pointer: None }
    }

    /// The host its captions are translated over.
    pub fn host(&self) -> &Rc<dyn crate::parts::PartHost> {
        self.core.frame.host()
    }

    pub fn menu(&self) -> &MenuSpec {
        self.core.menu()
    }

    pub fn wanted(&self) -> f64 {
        HEIGHT
    }

    /// Whether it takes clicks: its state is not disabled.
    pub fn enabled(&self) -> bool {
        self.core.enabled()
    }

    /// Its top items as they show, each with its place.
    pub fn placed(&self) -> Vec<(&CommandSpec, Rect)> {
        let placed = self.core.placed.borrow().clone();
        placed.into_iter().map(|(i, r)| (&self.menu().items[i], r)).collect()
    }

    /// The place of the top item with that caption, as written.
    pub fn place_of(&self, caption: &str) -> Option<Rect> {
        self.placed().into_iter().find(|(item, _)| item.text("caption") == Some(caption)).map(|(_, r)| r)
    }

    /// The top item marked while the bar has the focus from the keyboard; none otherwise.
    pub fn marked(&self) -> Option<&CommandSpec> {
        self.core.marked.get().map(|i| &self.menu().items[i])
    }

    fn item_at(&self, x: f64, y: f64) -> Option<(usize, Rect)> {
        self.core.placed.borrow().iter().copied().find(|(i, r)| r.contains(x, y) && !rules::is_separator(&self.menu().items[*i]))
    }

    /// Opens the menu of the top item at `at` below it, or closes it when it is the one open.
    pub fn press(&mut self, at: usize, area: Rect, cx: &mut Cx) {
        self.core.press(at, area, cx);
    }

    /// Whether its access letters show now.
    pub fn access_shown(&self) -> bool {
        self.core.frame.access() != Access::Off && self.is_shown() && self.enabled()
    }

    /// Opens the menu of the top item whose access letter is `letter` (or runs it, without items): false when none has it.
    pub fn open_by_letter(&mut self, letter: char, cx: &mut Cx) -> bool {
        self.core.open_by_letter(letter, cx)
    }

    /// The bar takes the focus with its first top item marked (F10, Alt
    /// alone), keeping the focus it had to give back on Escape; false when
    /// it shows none or takes no clicks.
    pub fn focus_first(&mut self, cx: &mut Cx) -> bool {
        self.core.focus_first(cx)
    }
}

impl Element for FrameMenuBar {
    crate::element_node!();

    /// It shows while its state is not hidden and one of its items shows; with nothing to show it takes no room.
    fn is_shown(&self) -> bool {
        self.core.shows()
    }

    fn is_enabled(&self) -> bool {
        self.enabled()
    }

    /// It takes the focus only from the keyboard (F10, Alt), so Tab passes it by.
    fn focusable(&self) -> bool {
        self.core.marked.get().is_some()
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        let host = self.core.frame.host().clone();
        let size = cx.look().size("text");
        let mut x = area.x + 6.0;
        let mut placed = Vec::new();
        for (i, item) in self.menu().items.iter().enumerate() {
            if !rules::shows(host.as_ref(), item.text("name")) {
                continue;
            }
            let wide = if rules::is_separator(item) { 9.0 } else { cx.fonts().measure(&host.text(item.text("caption")), "text", size, false) + 2.0 * PADDING };
            placed.push((i, Rect::new(x, area.y + 2.0, wide, area.height - 4.0)));
            x += wide + 2.0;
        }
        *self.core.placed.borrow_mut() = placed;
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        if !focused && self.core.marked.get().is_some() {
            self.core.marked.set(None);
            cx.invalidate();
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let Some(item) = self.core.marked.get() else { return false };
        let frame = self.core.frame.clone();
        // Any key ends the access letters Alt alone kept (the bar still takes it).
        if key.key != Key::Alt && frame.access() == Access::Kept {
            frame.set_access(Access::Off);
            cx.invalidate();
        }
        if key.modifiers == Modifiers::NONE {
            if let Some(letter) = rules::access_char(key.key) {
                if self.core.open_by_letter(letter, cx) {
                    return true;
                }
            }
        }
        match key.key {
            Key::Left | Key::Right => {
                if let Some((next, _)) = self.core.beside(Some(item), if key.key == Key::Right { 1 } else { -1 }) {
                    self.core.mark(next, cx);
                }
            }
            Key::Down | Key::Enter | Key::Space => {
                let Some(area) = self.core.place_of(item) else { return true };
                if BarCore::opens(&self.menu().items[item]) {
                    self.core.open(item, area, true, cx);
                } else {
                    let spec = self.menu().items[item].clone();
                    self.core.leave(cx);
                    frame.run(&spec, area, (self.core.id, item), cx);
                }
            }
            Key::Escape => self.core.leave(cx),
            _ => return false,
        }
        true
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        if let Some((i, area)) = self.item_at(at.screen_x, at.screen_y) {
            self.core.press(i, area, cx);
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pointer = Some((at.screen_x, at.screen_y));
        cx.invalidate();
        // While a menu of the bar is open, the pointer over another opens that one.
        let Some((region, open)) = self.core.frame.menus().owner() else { return };
        if region != self.node.id() {
            return;
        }
        if let Some((i, area)) = self.item_at(at.screen_x, at.screen_y) {
            let item = &self.menu().items[i];
            if i != open && BarCore::opens(item) && rules::takes(self.core.frame.host().as_ref(), item.text("name")) {
                self.core.open(i, area, false, cx);
            }
        }
    }

    /// A frame does not scroll the window's parts.
    fn on_wheel(&mut self, _dx: f64, _dy: f64, _held: Modifiers, _cx: &mut Cx) -> bool {
        true
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        plate(p, b, Rect::new(b.x, b.bottom() - look.border(), b.width, look.border()));
        let _dimmed = p.dimmed(self.enabled());
        let size = look.size("text");
        let host = self.core.frame.host().as_ref();
        let open = self.core.frame.menus().owner();
        let marked = self.core.marked.get();
        let access = if self.access_shown() { self.core.access_keys() } else { Vec::new() };
        for (i, area) in self.core.placed.borrow().iter() {
            let item = &self.menu().items[*i];
            if rules::is_separator(item) {
                p.fill(Rect::new(area.x + 4.0, area.y + 4.0, look.border(), area.height - 8.0), look.color("strong_border"), 0.0);
                continue;
            }
            let takes = rules::takes(host, item.text("name"));
            if takes && (open == Some(owner(&self.node, *i)) || marked == Some(*i) || over(&self.node, self.pointer, *area)) {
                p.fill(*area, look.part("menu", Some("hover")).background.unwrap_or_else(|| look.color("accent_soft")), look.radius("button"));
            }
            let color = look.color(if takes { "frame_text" } else { "muted" });
            let caption = host.text(item.text("caption"));
            p.text(&caption, *area, color, size, "text", false, TextAlign::Center);
            // The access letter, underlined.
            if let Some(at) = access.iter().find(|(k, _)| k == i).and_then(|(_, at)| *at) {
                let before: String = caption.chars().take(at).collect();
                let letter: String = caption.chars().skip(at).take(1).collect();
                let whole = p.fonts.measure(&caption, "text", size, false);
                let x = area.x + (area.width - whole) / 2.0 + p.fonts.measure(&before, "text", size, false);
                let y = (area.y + area.height / 2.0 + size * 0.5).round();
                p.fill(Rect::new(x, y, p.fonts.measure(&letter, "text", size, false), 1.0), color, 0.0);
            }
        }
    }
}
