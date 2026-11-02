//! A menu opened over the window (DESIGN 5.16): a menu bar's menu, the
//! choices of a toolbar button that picks the language or theme, and the
//! flyout of a sub menu. Each item shows its check mark, icon, caption and
//! key, an arrow for a sub menu; a line between items for a separator. An
//! item code hid is left out, one it disabled is greyed and takes no click,
//! as every part. Picking an item closes every menu first, then runs it.

use std::cell::RefCell;
use std::rc::Rc;

use skia_safe::Color;

use crate::element::{Cx, Element, ElementId, Node};
use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, PointerInput};
use crate::overlay::Overlay;
use crate::painter::{Painter, TextAlign};
use crate::parts::containers::icon::draw_icon;

/// An item of a menu as it shows: its caption, its icon's SVG path, its key,
/// check mark, whether it takes clicks, its sub menu and what picking it does.
#[derive(Clone, Default)]
pub struct MenuEntry {
    pub caption: String,
    pub icon: Option<String>,
    pub key: Option<String>,
    pub checked: bool,
    pub enabled: bool,
    pub separator: bool,
    pub items: Vec<MenuEntry>,
    pub run: Option<Rc<dyn Fn()>>,
    /// An element that wakes (its `on_wake` runs) once the item ran: a `run` has no context, so the element does the rest there.
    pub wake: Option<ElementId>,
}

impl MenuEntry {
    /// A line between items.
    pub fn line() -> MenuEntry {
        MenuEntry { separator: true, ..MenuEntry::default() }
    }
}

impl std::fmt::Debug for MenuEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuEntry").field("caption", &self.caption).field("key", &self.key).field("checked", &self.checked).field("enabled", &self.enabled).field("separator", &self.separator).field("items", &self.items).finish()
    }
}

/// What an open menu belongs to: the region it opened from and the place of its item there.
pub type MenuOwner = (ElementId, usize);

/// What the person going sideways in an open menu does (a menu bar opens the menu beside): -1 left, 1 right.
pub type Beside = Rc<dyn Fn(i64, &mut Cx)>;

#[derive(Default)]
struct MenuState {
    /// The menus open now, the first menu first: each list's id and the entry of the menu below it opened from.
    open: Vec<(ElementId, Option<usize>)>,
    owner: Option<MenuOwner>,
    beside: Option<Beside>,
}

/// The menus open over a window: one menu and the flyouts of its sub menus,
/// one per level. A press outside or Escape closes them all; picking an
/// item closes them and then runs it.
#[derive(Clone, Default)]
pub struct FrameMenus(Rc<RefCell<MenuState>>);

impl FrameMenus {
    pub fn new() -> FrameMenus {
        FrameMenus::default()
    }

    /// What the open menu belongs to; none while no menu is open.
    pub fn owner(&self) -> Option<MenuOwner> {
        self.0.borrow().owner
    }

    pub fn is_open(&self) -> bool {
        !self.0.borrow().open.is_empty()
    }

    /// The ids of the menus open now, the first menu first.
    pub fn lists(&self) -> Vec<ElementId> {
        self.0.borrow().open.iter().map(|(id, _)| *id).collect()
    }

    /// Opens `entries` below `under` for `owner`; a press on `anchor` goes to what opened it.
    pub fn open(&self, owner: MenuOwner, entries: Vec<MenuEntry>, under: Rect, anchor: Option<ElementId>, cx: &mut Cx) {
        self.open_with(owner, entries, under, anchor, false, cx);
    }

    /// Opens a menu as `open` does, its first item marked: opened by a key.
    pub fn open_marked(&self, owner: MenuOwner, entries: Vec<MenuEntry>, under: Rect, anchor: Option<ElementId>, cx: &mut Cx) {
        self.open_with(owner, entries, under, anchor, true, cx);
    }

    /// In the menus open now, Left and Right (where no sub menu takes them) do `beside`; it goes once they all close.
    pub fn set_beside(&self, beside: Beside) {
        self.0.borrow_mut().beside = Some(beside);
    }

    fn beside(&self) -> Option<Beside> {
        self.0.borrow().beside.clone()
    }

    fn open_with(&self, owner: MenuOwner, entries: Vec<MenuEntry>, under: Rect, anchor: Option<ElementId>, marked: bool, cx: &mut Cx) {
        self.close_all(cx);
        let whole = cx.whole();
        let (width, height) = FrameMenuList::size_of(&entries, cx.fonts(), cx.look().size("text"));
        let x = under.x.min(whole.width - width).max(0.0);
        let mut y = under.bottom() + 1.0;
        if y + height > whole.height && under.y - 1.0 - height >= 0.0 {
            y = under.y - 1.0 - height;
        }
        let mut list = FrameMenuList::new(self.clone(), entries, 0);
        if marked {
            list.marked = list.entries.iter().position(|e| !e.separator);
        }
        let id = list.node.id();
        self.0.borrow_mut().owner = Some(owner);
        self.add(list, Rect::new(x, y, width, height), anchor, None, cx);
        cx.focus(Some(id));
    }

    /// Opens the sub menu of `entry` (at place `from` of the menu at `level`, whose row is `row` in `parent`) beside it.
    /// Opened by a key, it takes the focus with its first item marked.
    fn open_sub(&self, level: usize, from: usize, entry: &MenuEntry, row: Rect, parent: Rect, by_key: bool, cx: &mut Cx) {
        let open_now = self.0.borrow().open.clone();
        if level >= open_now.len() {
            return;
        }
        if open_now.get(level + 1).is_some_and(|(_, f)| *f == Some(from)) {
            return;
        }
        self.close_from(level + 1, cx);
        let whole = cx.whole();
        let (width, height) = FrameMenuList::size_of(&entry.items, cx.fonts(), cx.look().size("text"));
        let mut x = parent.right() - 2.0;
        if x + width > whole.width {
            x = (parent.x - width + 2.0).max(0.0);
        }
        let y = (row.y - INNER).min(whole.height - height).max(0.0);
        let mut list = FrameMenuList::new(self.clone(), entry.items.clone(), level + 1);
        let id = list.node.id();
        if by_key {
            list.marked = entry.items.iter().position(|e| !e.separator);
        }
        let anchor = open_now[level].0;
        self.add(list, Rect::new(x, y, width, height), Some(anchor), Some(from), cx);
        if by_key {
            cx.focus(Some(id));
        }
    }

    fn add(&self, list: FrameMenuList, place: Rect, anchor: Option<ElementId>, from: Option<usize>, cx: &mut Cx) {
        let id = list.node.id();
        self.0.borrow_mut().open.push((id, from));
        cx.open(Overlay { anchor, ..Overlay::new(Box::new(list), place) });
    }

    /// Closes the menus from `level` on.
    pub fn close_from(&self, level: usize, cx: &mut Cx) {
        let gone: Vec<ElementId> = {
            let mut state = self.0.borrow_mut();
            let at = level.min(state.open.len());
            let gone = state.open.split_off(at).into_iter().map(|(id, _)| id).rev().collect();
            if state.open.is_empty() {
                state.owner = None;
                state.beside = None;
            }
            gone
        };
        for id in gone {
            cx.close(id);
        }
    }

    pub fn close_all(&self, cx: &mut Cx) {
        self.close_from(0, cx);
    }

    /// The person picked `entry`: the menus close, then it runs.
    pub fn picked(&self, entry: &MenuEntry, cx: &mut Cx) {
        self.close_all(cx);
        if let Some(run) = &entry.run {
            run();
        }
        if let Some(id) = entry.wake {
            cx.wake(id, cx.now());
        }

    }

    /// A menu closed from outside (a press elsewhere, Escape): every other menu goes too.
    fn closed_outside(&self, id: ElementId, cx: &mut Cx) {
        let others: Vec<ElementId> = {
            let mut state = self.0.borrow_mut();
            if !state.open.iter().any(|(i, _)| *i == id) {
                return;
            }
            state.owner = None;
            state.beside = None;
            state.open.drain(..).map(|(i, _)| i).filter(|i| *i != id).collect()
        };
        for other in others.into_iter().rev() {
            cx.close(other);
        }
    }

    fn at(&self, level: usize) -> Option<ElementId> {
        self.0.borrow().open.get(level).map(|(id, _)| *id)
    }
}

/// How high an item's line is, a separator's, and the space inside the menu's border.
pub const LINE: f64 = 28.0;
pub const SEPARATOR_LINE: f64 = 9.0;
pub const INNER: f64 = 4.0;

const CHECK_WIDTH: f64 = 26.0;
const KEY_GAP: f64 = 28.0;
const ARROW_WIDTH: f64 = 20.0;
const MIN_WIDTH: f64 = 180.0;

/// One open menu: its items in lines, the one under the pointer or reached with the arrow keys marked.
pub struct FrameMenuList {
    node: Node,
    menus: FrameMenus,
    entries: Vec<MenuEntry>,
    level: usize,
    marked: Option<usize>,
    pressed_at: Option<usize>,
}

impl FrameMenuList {
    fn new(menus: FrameMenus, entries: Vec<MenuEntry>, level: usize) -> FrameMenuList {
        FrameMenuList { node: Node::new(), menus, entries, level, marked: None, pressed_at: None }
    }

    pub fn entries(&self) -> &[MenuEntry] {
        &self.entries
    }

    pub fn level(&self) -> usize {
        self.level
    }

    /// The item marked now; none for none.
    pub fn marked(&self) -> Option<usize> {
        self.marked
    }

    /// How wide and high a menu of `entries` is.
    pub fn size_of(entries: &[MenuEntry], fonts: &Fonts, size: f64) -> (f64, f64) {
        let (mut captions, mut keys, mut arrow) = (0.0f64, 0.0f64, false);
        for entry in entries.iter().filter(|e| !e.separator) {
            captions = captions.max(fonts.measure(&entry.caption, "text", size, false) + if entry.icon.is_some() { 24.0 } else { 0.0 });
            if let Some(key) = &entry.key {
                keys = keys.max(fonts.measure(key, "text", size - 2.0, false));
            }
            arrow |= !entry.items.is_empty();
        }
        let width = CHECK_WIDTH + captions + if keys > 0.0 { KEY_GAP + keys } else { 0.0 } + if arrow { ARROW_WIDTH } else { 0.0 } + 16.0 + 2.0 * INNER;
        let height: f64 = entries.iter().map(|e| if e.separator { SEPARATOR_LINE } else { LINE }).sum::<f64>() + 2.0 * INNER;
        (width.max(MIN_WIDTH).ceil(), height.max(LINE))
    }

    /// Where the item at `at` is.
    pub fn row_of(&self, at: usize) -> Rect {
        let b = self.node.bounds;
        let y = b.y + INNER + self.entries[..at].iter().map(|e| if e.separator { SEPARATOR_LINE } else { LINE }).sum::<f64>();
        Rect::new(b.x + INNER, y, b.width - 2.0 * INNER, if self.entries[at].separator { SEPARATOR_LINE } else { LINE })
    }

    fn row_at(&self, y: f64) -> Option<usize> {
        (0..self.entries.len()).find(|i| {
            let row = self.row_of(*i);
            y >= row.y && y < row.bottom()
        }).filter(|i| !self.entries[*i].separator)
    }

    /// The item at `at` picked: a sub menu opens, an item that takes clicks runs.
    pub fn activate(&mut self, at: Option<usize>, cx: &mut Cx) {
        let Some(at) = at.filter(|a| *a < self.entries.len()) else { return };
        let entry = self.entries[at].clone();
        if entry.separator || !entry.enabled {
            return;
        }
        self.marked = Some(at);
        if !entry.items.is_empty() {
            self.menus.open_sub(self.level, at, &entry, self.row_of(at), self.node.bounds, true, cx);
            return;
        }
        self.menus.picked(&entry, cx);
    }

    /// Marks the next item that is no separator, `by` one down or up.
    pub fn step(&mut self, by: i64, cx: &mut Cx) {
        let count = self.entries.len() as i64;
        if self.entries.iter().all(|e| e.separator) {
            return;
        }
        let mut at = self.marked.map(|m| m as i64).unwrap_or(if by < 0 { count } else { -1 });
        loop {
            at = (at + by).rem_euclid(count);
            if !self.entries[at as usize].separator {
                break;
            }
        }
        self.marked = Some(at as usize);
        cx.invalidate();
    }
}

impl Element for FrameMenuList {
    crate::element_node!();

    fn focusable(&self) -> bool {
        true
    }

    fn on_detached(&mut self, cx: &mut Cx) {
        self.menus.closed_outside(self.node.id(), cx);
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let Some(row) = self.row_at(at.screen_y) else { return };
        if Some(row) == self.marked {
            return;
        }
        self.marked = Some(row);
        // Over an item with a sub menu, its flyout opens; over another, an open flyout goes.
        let entry = self.entries[row].clone();
        if entry.enabled && !entry.items.is_empty() {
            self.menus.open_sub(self.level, row, &entry, self.row_of(row), self.node.bounds, false, cx);
        } else {
            self.menus.close_from(self.level + 1, cx);
        }
        cx.invalidate();
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, _cx: &mut Cx) {
        self.pressed_at = self.row_at(at.screen_y);
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if let Some(at) = self.pressed_at.take() {
            self.activate(Some(at), cx);
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !key.modifiers.is_empty() {
            return false;
        }
        match key.key {
            Key::Down => self.step(1, cx),
            Key::Up => self.step(-1, cx),
            Key::Enter | Key::Space => self.activate(self.marked, cx),
            Key::Right if self.marked.is_some_and(|m| !self.entries[m].items.is_empty()) => self.activate(self.marked, cx),
            Key::Left if self.level > 0 => {
                let below = self.menus.at(self.level - 1);
                self.menus.close_from(self.level, cx);
                cx.focus(below);
            }
            // In a menu bar's menu, Left and Right open the menu beside it on the bar.
            Key::Left | Key::Right => match self.menus.beside() {
                Some(beside) => beside(if key.key == Key::Right { 1 } else { -1 }, cx),
                None => return false,
            },
            _ => return false,
        }
        true
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let menu = look.part("menu", None);
        let bounds = self.node.bounds;
        let radius = menu.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("panel"));
        p.fill(bounds, menu.background.unwrap_or_else(|| look.color("raised")), radius);
        p.stroke(bounds, menu.border.unwrap_or_else(|| look.color("border")), look.border(), radius);
        let size = menu.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
        for (i, entry) in self.entries.iter().enumerate() {
            let row = self.row_of(i);
            if entry.separator {
                p.fill(Rect::new(row.x + 6.0, row.y + row.height / 2.0, row.width - 12.0, look.border()), look.color("strong_border"), 0.0);
                continue;
            }
            let lit = self.marked == Some(i) && entry.enabled;
            // A disabled entry is greyed as a disabled button is: muted and dimmed.
            let _dimmed = p.dimmed(entry.enabled);
            let values = if lit { look.part("menu", Some("hover")) } else { menu.clone() };
            if lit {
                p.fill(row, values.background.unwrap_or_else(|| look.color("accent_soft")), look.radius("button"));
            }
            let text = if entry.enabled { values.text.unwrap_or_else(|| look.color("text")) } else { look.color("muted") };
            if entry.checked {
                check(p, Rect::new(row.x, row.y, CHECK_WIDTH, row.height), text);
            }
            let mut x = row.x + CHECK_WIDTH;
            if let Some(path) = &entry.icon {
                draw_icon(p, Some(path), Rect::new(x, row.y + (row.height - 16.0) / 2.0, 16.0, 16.0), text);
                x += 24.0;
            }
            p.text(&entry.caption, Rect::new(x, row.y, row.right() - x, row.height), text, size, "text", false, TextAlign::Left);
            let mut right = row.right() - 8.0;
            if !entry.items.is_empty() {
                arrow(p, Rect::new(right - 12.0, row.y, 12.0, row.height), text);
                right -= ARROW_WIDTH;
            }
            if let Some(key) = &entry.key {
                p.text(key, Rect::new(row.x, row.y, right - row.x, row.height), look.color("muted"), size - 2.0, "text", false, TextAlign::Right);
            }
        }
    }
}

/// A check mark in the middle of `area`.
fn check(p: &Painter, area: Rect, color: Color) {
    let (cx, cy) = (area.x + area.width / 2.0, area.y + area.height / 2.0);
    p.line((cx - 5.0, cy), (cx - 1.5, cy + 3.5), color, 1.8);
    p.line((cx - 1.5, cy + 3.5), (cx + 5.0, cy - 4.0), color, 1.8);
}

/// A small arrow pointing right: the item opens a sub menu.
fn arrow(p: &Painter, area: Rect, color: Color) {
    let (cx, cy) = (area.x + area.width / 2.0, area.y + area.height / 2.0);
    p.line((cx - 2.0, cy - 4.0), (cx + 2.0, cy), color, 1.5);
    p.line((cx + 2.0, cy), (cx - 2.0, cy + 4.0), color, 1.5);
}
