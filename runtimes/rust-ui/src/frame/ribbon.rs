//! A ribbon (conformance/ribbons.json): its tabs in a strip, and under it
//! the groups of the open tab, each a row of buttons (a large one with its
//! icon above its caption) with its title below. The open tab, counted from
//! 1, and whether a collapsible ribbon is folded to its tabs are the
//! ribbon's part state (`tab`, `collapsed`), so code reads and sets them; an
//! unnamed ribbon keeps them itself. A hidden tab gives way to the first
//! visible one; opening a tab of a folded ribbon unfolds it, and a double
//! click on the tabs folds it again, as in an office suite.

use std::rc::Rc;

use clarity_runtime::ui::PartState;

use crate::element::{Cx, Element, Node};
use crate::geometry::Rect;
use crate::input::{Modifiers, PointerInput};
use crate::markup::CommandSpec;
use crate::painter::{Painter, TextAlign};

use super::spec::RibbonSpec;
use super::{buttons, over, owner, rules, WindowFrame};

/// How high the strip of tabs is, and the groups under it.
pub const TABS_HEIGHT: f64 = 30.0;
pub const BODY_HEIGHT: f64 = 92.0;

const TITLE_HEIGHT: f64 = 18.0;

pub struct FrameRibbon {
    node: Node,
    frame: Rc<WindowFrame>,
    tabs: Vec<(usize, Rect)>,
    /// The buttons of the tab shown: its group's place in the tab, its place in the group, and where it is.
    buttons: Vec<(usize, usize, Rect)>,
    groups: Vec<(usize, Rect)>,
    own_tab: i64,
    own_collapsed: bool,
    pressed_at: Option<usize>,
    pointer: Option<(f64, f64)>,
}

impl FrameRibbon {
    pub fn new(frame: Rc<WindowFrame>) -> FrameRibbon {
        let collapsed = frame.shown().ribbon.as_ref().is_some_and(|r| r.value.collapsed);
        FrameRibbon { node: Node::new(), frame, tabs: Vec::new(), buttons: Vec::new(), groups: Vec::new(), own_tab: 1, own_collapsed: collapsed, pressed_at: None, pointer: None }
    }

    pub fn ribbon(&self) -> &RibbonSpec {
        &self.frame.shown().ribbon.as_ref().expect("a ribbon has its spec").value
    }

    fn inherited(&self) -> bool {
        self.frame.shown().ribbon.as_ref().is_some_and(|r| r.inherited)
    }

    fn state(&self) -> Option<PartState> {
        self.ribbon().name.as_deref().and_then(|n| self.frame.host().state(Some(n)))
    }

    pub fn enabled(&self) -> bool {
        rules::takes(self.frame.host().as_ref(), self.ribbon().name.as_deref())
    }

    /// Whether it is folded to its tabs: only a collapsible one, as its state says.
    pub fn collapsed(&self) -> bool {
        let asked = match self.state() {
            Some(state) => state.collapsed.unwrap_or(self.own_collapsed),
            None => self.own_collapsed,
        };
        self.ribbon().collapsible && asked
    }

    /// The tab it shows, from 1: the open one while it is visible, else the first visible one; none when no tab is.
    pub fn shown_tab(&self) -> Option<usize> {
        let host = self.frame.host().as_ref();
        let visible: Vec<bool> = self.ribbon().tabs.iter().map(|t| rules::shows(host, t.name.as_deref())).collect();
        let asked = self.state().and_then(|s| s.tab).unwrap_or(self.own_tab);
        rules::shown_tab(&visible, asked)
    }

    pub fn wanted(&self) -> f64 {
        TABS_HEIGHT + if self.collapsed() || self.shown_tab().is_none() { 0.0 } else { BODY_HEIGHT }
    }

    /// Its tabs as they show, each with its place (from 1).
    pub fn tabs(&self) -> &[(usize, Rect)] {
        &self.tabs
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
        match self.ribbon().name.clone() {
            Some(name) => self.frame.host().change_state(&name, &mut |s| {
                s.tab = Some(tab as i64);
                s.collapsed = Some(false);
            }),
            None => {
                self.own_tab = tab as i64;
                self.own_collapsed = false;
            }
        }
        cx.invalidate();
    }

    /// Folds a collapsible ribbon to its tabs, or unfolds it.
    pub fn fold(&mut self, folded: bool, cx: &mut Cx) {
        if !self.ribbon().collapsible {
            return;
        }
        match self.ribbon().name.clone() {
            Some(name) => self.frame.host().change_state(&name, &mut |s| s.collapsed = Some(folded)),
            None => self.own_collapsed = folded,
        }
        cx.invalidate();
    }

    /// The button at place `at` of the tab shown clicked: it runs, unless it or the ribbon is disabled.
    pub fn click(&mut self, at: usize, cx: &mut Cx) {
        let Some((g, i, area)) = self.buttons.get(at).copied() else { return };
        let item = self.item(g, i).clone();
        if !self.enabled() || rules::is_separator(&item) || !rules::takes(self.frame.host().as_ref(), item.text("name")) {
            return;
        }
        self.frame.run(&item, area, owner(&self.node, g * 1000 + i), cx);
    }
}

impl Element for FrameRibbon {
    crate::element_node!();

    /// It shows while its state is not hidden and one of its tabs shows; with nothing to show it takes no room.
    fn is_shown(&self) -> bool {
        rules::shows(self.frame.host().as_ref(), self.ribbon().name.as_deref()) && self.shown_tab().is_some()
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        self.tabs.clear();
        self.buttons.clear();
        self.groups.clear();
        let host = self.frame.host().clone();
        let host = host.as_ref();
        let small = cx.look().size("small");
        let mut x = area.x + 6.0;
        let frame = self.frame.clone();
        let ribbon = &frame.shown().ribbon.as_ref().expect("a ribbon has its spec").value;
        for (i, tab) in ribbon.tabs.iter().enumerate() {
            if !rules::shows(host, tab.name.as_deref()) {
                continue;
            }
            let wide = cx.fonts().measure(&host.text(Some(&tab.title)), "text", small, false) + 28.0;
            self.tabs.push((i + 1, Rect::new(x, area.y + 4.0, wide, TABS_HEIGHT - 4.0)));
            x += wide + 2.0;
        }
        let Some(shown) = self.shown_tab().filter(|_| !self.collapsed()) else { return };
        let mut x = area.x + 6.0;
        let top = area.y + TABS_HEIGHT + 6.0;
        let mut buttons = Vec::new();
        let mut groups = Vec::new();
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
                buttons.push((g, i, Rect::new(x, top, wide, high)));
                x += wide + 4.0;
            }
            let title = cx.fonts().measure(&host.text(Some(&group.title)), "text", small, false) + 12.0;
            x = (x + 2.0).max(start + title);
            groups.push((g, Rect::new(start, top, x - start, BODY_HEIGHT - 12.0)));
            x += 4.0;
        }
        self.buttons = buttons;
        self.groups = groups;
    }

    /// A button says its caption and its keys when the pointer rests on it.
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        let (g, i, _) = self.buttons.iter().find(|(_, _, r)| r.contains(x, y))?;
        let item = self.item(*g, *i);
        if rules::is_separator(item) {
            return None;
        }
        rules::tip_of(&self.frame.host().text(item.text("caption")), &self.frame.keys_of(item, self.inherited()))
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        let (x, y) = (at.screen_x, at.screen_y);
        self.pressed_at = self.buttons.iter().position(|(_, _, r)| r.contains(x, y));
        if !self.enabled() {
            return;
        }
        let collapsible = self.ribbon().collapsible;
        if let Some((tab, _)) = self.tabs.iter().copied().find(|(_, r)| r.contains(x, y)) {
            // A double click on a tab folds or unfolds the ribbon; a press opens the tab.
            if at.clicks >= 2 && collapsible {
                let folded = self.collapsed();
                self.fold(!folded, cx);
            } else {
                self.open(tab, cx);
            }
            return;
        }
        if at.clicks >= 2 && collapsible && y < self.node.bounds.y + TABS_HEIGHT {
            let folded = self.collapsed();
            self.fold(!folded, cx);
        }
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if let Some(at) = self.pressed_at.take() {
            self.click(at, cx);
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pointer = Some((at.screen_x, at.screen_y));
        cx.invalidate();
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
        let shown = if self.collapsed() { None } else { self.shown_tab() };
        let small = look.size("small");
        let host = self.frame.host().as_ref();
        for (tab, area) in &self.tabs {
            let open = Some(*tab) == shown;
            let hover = over(&self.node, self.pointer, *area);
            if open {
                p.fill(*area, look.color("panel"), look.radius("button"));
            } else if hover {
                p.fill(*area, look.part("tab", Some("hover")).background.unwrap_or_else(|| look.color("accent_soft")), look.radius("button"));
            }
            let color = look.color(if open { "text" } else { "frame_text" });
            p.text(&host.text(Some(&self.ribbon().tabs[tab - 1].title)), *area, color, small, "text", open, TextAlign::Center);
        }
        if let Some(shown) = shown {
            for (g, area) in &self.groups {
                let title = &self.ribbon().tabs[shown - 1].groups[*g].title;
                p.text(&host.text(Some(title)), Rect::new(area.x, area.bottom() - TITLE_HEIGHT, area.width, TITLE_HEIGHT), look.color("muted"), small, "text", false, TextAlign::Center);
                p.fill(Rect::new(area.right(), area.y, look.border(), area.height), look.color("border"), 0.0);
            }
        }
        for (n, (g, i, area)) in self.buttons.iter().enumerate() {
            let hover = over(&self.node, self.pointer, *area);
            buttons::draw(p, host, self.item(*g, *i), *area, true, hover, self.node.pressed && self.pressed_at == Some(n) && hover);
        }
    }
}
