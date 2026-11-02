//! A ribbon (conformance/ribbons.json): its tabs in a strip, and under it
//! the groups of the open tab, each a row of buttons (a large one with its
//! icon above its caption) with its title below. The open tab, counted from
//! 1, and whether a collapsible ribbon is folded to its tabs are the
//! ribbon's part state (`tab`, `collapsed`), so code reads and sets them; an
//! unnamed ribbon keeps them itself. A hidden tab gives way to the first
//! visible one; opening a tab of a folded ribbon unfolds it, and a double
//! click anywhere on it but on a button folds it again, as in an office
//! suite. Tab reaches its strip of tabs, where Left and Right open the tab
//! beside, and goes on into the buttons of the open tab.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use clarity_runtime::ui::PartState;

use crate::element::{draw_children, Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::markup::CommandSpec;
use crate::painter::{Painter, TextAlign};

use super::buttons::FrameButton;
use super::spec::RibbonSpec;
use super::{buttons, over, owner, rules, WindowFrame};

/// How high the strip of tabs is, and the groups under it.
pub const TABS_HEIGHT: f64 = 30.0;
pub const BODY_HEIGHT: f64 = 92.0;

const TITLE_HEIGHT: f64 = 18.0;

/// The place of a ribbon's button among the menu owners of its region: its tab, group and item.
fn code(tab: usize, group: usize, at: usize) -> usize {
    tab * 1_000_000 + group * 1000 + at
}

/// What a ribbon and its strip of tabs share: the frame, the tab it keeps
/// and its folding when it has no name, and where its tabs are.
struct RibbonCore {
    frame: Rc<WindowFrame>,
    own_tab: Cell<i64>,
    own_collapsed: Cell<bool>,
    tabs: RefCell<Vec<(usize, Rect)>>,
}

impl RibbonCore {
    fn ribbon(&self) -> &RibbonSpec {
        &self.frame.shown().ribbon.as_ref().expect("a ribbon has its spec").value
    }

    fn state(&self) -> Option<PartState> {
        self.ribbon().name.as_deref().and_then(|n| self.frame.host().state(Some(n)))
    }

    fn enabled(&self) -> bool {
        rules::takes(self.frame.host().as_ref(), self.ribbon().name.as_deref())
    }

    fn collapsed(&self) -> bool {
        let asked = match self.state() {
            Some(state) => state.collapsed.unwrap_or(self.own_collapsed.get()),
            None => self.own_collapsed.get(),
        };
        self.ribbon().collapsible && asked
    }

    fn shown_tab(&self) -> Option<usize> {
        let host = self.frame.host().as_ref();
        let visible: Vec<bool> = self.ribbon().tabs.iter().map(|t| rules::shows(host, t.name.as_deref())).collect();
        let asked = self.state().and_then(|s| s.tab).unwrap_or(self.own_tab.get());
        rules::shown_tab(&visible, asked)
    }

    fn open(&self, tab: usize, cx: &mut Cx) {
        match self.ribbon().name.clone() {
            Some(name) => crate::parts::change_state(&**self.frame.host(), &name, |s| {
                s.tab = Some(tab as i64);
                s.collapsed = Some(false);
            }),
            None => {
                self.own_tab.set(tab as i64);
                self.own_collapsed.set(false);
            }
        }
        cx.invalidate();
    }

    fn fold(&self, folded: bool, cx: &mut Cx) {
        if !self.ribbon().collapsible {
            return;
        }
        match self.ribbon().name.clone() {
            Some(name) => crate::parts::change_state(&**self.frame.host(), &name, |s| s.collapsed = Some(folded)),
            None => self.own_collapsed.set(folded),
        }
        cx.invalidate();
    }

    /// Opens the visible tab `by` places on from the one shown; none past either end.
    fn step(&self, by: i64, cx: &mut Cx) {
        let shown = self.shown_tab();
        let tabs = self.tabs.borrow().clone();
        let Some(at) = tabs.iter().position(|(t, _)| Some(*t) == shown) else { return };
        let next = at as i64 + by;
        if next < 0 || next >= tabs.len() as i64 {
            return;
        }
        self.open(tabs[next as usize].0, cx);
    }
}

pub struct FrameRibbon {
    node: Node,
    core: Rc<RibbonCore>,
    /// The buttons of the tab shown: its group's place in the tab, its place in the group, and where it is.
    buttons: Vec<(usize, usize, Rect)>,
    groups: Vec<(usize, Rect)>,
}

impl FrameRibbon {
    pub fn new(frame: Rc<WindowFrame>) -> FrameRibbon {
        let ribbon = frame.shown().ribbon.as_ref().expect("a ribbon has its spec").value.clone();
        let core = Rc::new(RibbonCore { frame: frame.clone(), own_tab: Cell::new(1), own_collapsed: Cell::new(ribbon.collapsed), tabs: RefCell::default() });
        let mut node = Node::new();
        node.add(Box::new(RibbonTabStrip { node: Node::new(), core: core.clone(), pointer: None }));
        for (t, tab) in ribbon.tabs.iter().enumerate() {
            for (g, group) in tab.groups.iter().enumerate() {
                for (i, item) in group.items.iter().enumerate() {
                    let made = FrameButton::new(frame.clone(), item.clone(), true, ribbon.name.clone(), owner(&node, code(t, g, i)));
                    node.add(Box::new(made));
                }
            }
        }
        FrameRibbon { node, core, buttons: Vec::new(), groups: Vec::new() }
    }

    pub fn ribbon(&self) -> &RibbonSpec {
        self.core.ribbon()
    }

    fn inherited(&self) -> bool {
        self.core.frame.shown().ribbon.as_ref().is_some_and(|r| r.inherited)
    }

    pub fn enabled(&self) -> bool {
        self.core.enabled()
    }

    /// Whether it is folded to its tabs: only a collapsible one, as its state says.
    pub fn collapsed(&self) -> bool {
        self.core.collapsed()
    }

    /// The tab it shows, from 1: the open one while it is visible, else the first visible one; none when no tab is.
    pub fn shown_tab(&self) -> Option<usize> {
        self.core.shown_tab()
    }

    pub fn wanted(&self) -> f64 {
        TABS_HEIGHT + if self.collapsed() || self.shown_tab().is_none() { 0.0 } else { BODY_HEIGHT }
    }

    /// Its tabs as they show, each with its place (from 1).
    pub fn tabs(&self) -> Vec<(usize, Rect)> {
        self.core.tabs.borrow().clone()
    }

    /// Its strip of tabs: it takes the focus, and Left and Right open the tab beside.
    pub fn tab_strip(&self) -> &RibbonTabStrip {
        self.node.children()[0].as_any().downcast_ref::<RibbonTabStrip>().expect("a ribbon holds its strip of tabs")
    }

    /// The buttons of the tab shown, each with its place.
    pub fn buttons(&self) -> Vec<(&CommandSpec, Rect)> {
        self.buttons.iter().map(|(g, i, r)| (self.item(*g, *i), *r)).collect()
    }

    fn item(&self, group: usize, at: usize) -> &CommandSpec {
        let tab = self.shown_tab().unwrap_or(1) - 1;
        &self.ribbon().tabs[tab].groups[group].items[at]
    }

    /// The person opened tab `tab` (from 1): it shows, and a folded ribbon unfolds.
    pub fn open(&mut self, tab: usize, cx: &mut Cx) {
        self.core.open(tab, cx);
    }

    /// Folds a collapsible ribbon to its tabs, or unfolds it.
    pub fn fold(&mut self, folded: bool, cx: &mut Cx) {
        self.core.fold(folded, cx);
    }

    /// The button at place `at` of the tab shown clicked: it runs, unless it or the ribbon is disabled.
    pub fn click(&mut self, at: usize, cx: &mut Cx) {
        let Some((g, i, _)) = self.buttons.get(at).copied() else { return };
        let Some(tab) = self.shown_tab() else { return };
        let wanted = (self.node.id() as ElementId, code(tab - 1, g, i));
        let button = self.node.children_mut().iter_mut().filter_map(|c| c.as_any_mut().downcast_mut::<FrameButton>()).find(|b| b.owner() == wanted);
        if let Some(button) = button {
            button.press(cx);
        }
    }
}

impl Element for FrameRibbon {
    crate::element_node!();

    /// It shows while its state is not hidden and one of its tabs shows; with nothing to show it takes no room.
    fn is_shown(&self) -> bool {
        rules::shows(self.core.frame.host().as_ref(), self.ribbon().name.as_deref()) && self.shown_tab().is_some()
    }

    /// A disabled ribbon's tabs and buttons take no press, focus or key.
    fn is_enabled(&self) -> bool {
        self.enabled()
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        self.buttons.clear();
        self.groups.clear();
        let id = self.node.id();
        let host = self.core.frame.host().clone();
        let host = host.as_ref();
        let small = cx.look().size("small");
        let mut x = area.x + 6.0;
        let core = self.core.clone();
        let ribbon = core.ribbon();
        let mut tabs = Vec::new();
        for (i, tab) in ribbon.tabs.iter().enumerate() {
            if !rules::shows(host, tab.name.as_deref()) {
                continue;
            }
            let wide = cx.fonts().measure(&host.text(Some(&tab.title)), "text", small, false) + 28.0;
            tabs.push((i + 1, Rect::new(x, area.y + 4.0, wide, TABS_HEIGHT - 4.0)));
            x += wide + 2.0;
        }
        *core.tabs.borrow_mut() = tabs;
        // Every button leaves its place until it is placed again.
        let mut places = std::collections::HashMap::new();
        if let Some(shown) = self.shown_tab().filter(|_| !self.collapsed()) {
            let mut x = area.x + 6.0;
            let top = area.y + TABS_HEIGHT + 6.0;
            for (g, group) in ribbon.tabs[shown - 1].groups.iter().enumerate() {
                if !rules::shows(host, group.name.as_deref()) {
                    continue;
                }
                let start = x;
                x += 6.0;
                for (i, item) in group.items.iter().enumerate() {
                    if !rules::shows(host, item.text("name")) {
                        continue;
                    }
                    let wide = buttons::width(host, item, true, cx);
                    let high = if item.flag("large") || rules::is_separator(item) { buttons::LARGE_HEIGHT } else { 28.0 };
                    let place = Rect::new(x, top, wide, high);
                    self.buttons.push((g, i, place));
                    places.insert((id, code(shown - 1, g, i)), place);
                    x += wide + 4.0;
                }
                let title = cx.fonts().measure(&host.text(Some(&group.title)), "text", small, false) + 12.0;
                x = (x + 2.0).max(start + title);
                self.groups.push((g, Rect::new(start, top, x - start, BODY_HEIGHT - 12.0)));
                x += 4.0;
            }
        }
        for child in self.node.children_mut().iter_mut() {
            if let Some(button) = child.as_any_mut().downcast_mut::<FrameButton>() {
                let place = places.get(&button.owner()).copied();
                button.place(place, cx);
            } else {
                child.arrange(Rect::new(area.x, area.y, area.width, TABS_HEIGHT), cx);
            }
        }
    }

    /// A button says its caption and its keys when the pointer rests on it.
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        let (g, i, _) = self.buttons.iter().find(|(_, _, r)| r.contains(x, y))?;
        let item = self.item(*g, *i);
        if rules::is_separator(item) {
            return None;
        }
        rules::tip_of(&self.core.frame.host().text(item.text("caption")), &self.core.frame.keys_of(item, self.inherited()))
    }

    /// A double click on it, away from its buttons, folds or unfolds it.
    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        if at.clicks >= 2 && self.enabled() {
            let folded = self.collapsed();
            self.fold(!folded, cx);
        }
    }

    fn on_wheel(&mut self, _dx: f64, _dy: f64, _held: Modifiers, _cx: &mut Cx) -> bool {
        true
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        p.fill(b, look.color("panel"), 0.0);
        p.fill(Rect::new(b.x, b.y, b.width, TABS_HEIGHT), look.color("frame"), 0.0);
        p.fill(Rect::new(b.x, b.bottom() - look.border(), b.width, look.border()), look.color("border"), 0.0);
        let _dimmed = p.dimmed(self.enabled());
        let small = look.size("small");
        let host = self.core.frame.host().as_ref();
        if let Some(shown) = self.shown_tab().filter(|_| !self.collapsed()) {
            for (g, area) in &self.groups {
                let title = &self.ribbon().tabs[shown - 1].groups[*g].title;
                p.text(&host.text(Some(title)), Rect::new(area.x, area.bottom() - TITLE_HEIGHT, area.width, TITLE_HEIGHT), look.color("muted"), small, "text", false, TextAlign::Center);
                p.fill(Rect::new(area.right(), area.y, look.border(), area.height), look.color("border"), 0.0);
            }
        }
        draw_children(&self.node, p);
    }
}

/// The strip of a ribbon's tabs: a press on a tab opens it, a double click
/// anywhere on the strip folds or unfolds the ribbon. It takes the focus
/// (Tab reaches it, the open tab ringed); Left and Right open the visible
/// tab beside, Enter or Space opens the one shown (a folded ribbon unfolds).
pub struct RibbonTabStrip {
    node: Node,
    core: Rc<RibbonCore>,
    pointer: Option<(f64, f64)>,
}

impl Element for RibbonTabStrip {
    crate::element_node!();

    fn focusable(&self) -> bool {
        true
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pointer = Some((at.screen_x, at.screen_y));
        cx.invalidate();
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        if !self.core.enabled() {
            return;
        }
        if at.clicks >= 2 && self.core.ribbon().collapsible {
            let folded = self.core.collapsed();
            self.core.fold(!folded, cx);
            return;
        }
        let tab = self.core.tabs.borrow().iter().find(|(_, r)| r.contains(at.screen_x, at.screen_y)).map(|(t, _)| *t);
        if let Some(tab) = tab {
            self.core.open(tab, cx);
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.modifiers != Modifiers::NONE {
            return false;
        }
        match key.key {
            Key::Left => self.core.step(-1, cx),
            Key::Right => self.core.step(1, cx),
            Key::Enter | Key::Space => match self.core.shown_tab() {
                Some(shown) => self.core.open(shown, cx),
                None => return false,
            },
            _ => return false,
        }
        true
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let shown_now = self.core.shown_tab();
        let shown = if self.core.collapsed() { None } else { shown_now };
        let small = look.size("small");
        let host = self.core.frame.host().as_ref();
        for (tab, area) in self.core.tabs.borrow().iter() {
            let open = Some(*tab) == shown;
            let hover = over(&self.node, self.pointer, *area);
            if open {
                p.fill(*area, look.color("panel"), look.radius("button"));
            } else if hover {
                p.fill(*area, look.part("tab", Some("hover")).background.unwrap_or_else(|| look.color("accent_soft")), look.radius("button"));
            }
            let color = look.color(if open { "text" } else { "frame_text" });
            p.text(&host.text(Some(&self.core.ribbon().tabs[tab - 1].title)), *area, color, small, "text", open, TextAlign::Center);
            // The focus ring goes round the tab shown, open or folded.
            if self.node.focused && Some(*tab) == shown_now {
                p.focus_ring(*area, look.radius("button"));
            }
        }
    }
}
