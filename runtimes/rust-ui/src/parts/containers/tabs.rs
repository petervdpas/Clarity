//! Tabs: a strip of tabs in the theme's tab look, and under it the open
//! tab's page, a grid of its parts as wide as the part spans. The open tab
//! is its part's state's `tab`, counted from 1, so code reads and sets it; a
//! tab whose parts code has all hidden gives way to the first one that
//! shows, as a ribbon's tab does (conformance/ribbons.json `shown`). A page
//! whose parts go further down than its room scrolls, so none is out of reach.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::element::{self, Cx, Element, ElementId, GridPanel, Node};
use crate::geometry::Rect;
use crate::grid::GAP;
use crate::input::{Key, KeyInput};
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

use super::container::{self, LookNow, Page};
use super::scroll_view::ScrollView;

/// How high the strip of tabs is.
pub const STRIP_HEIGHT: f64 = 32.0;

/// What the tabs share with their strip's tabs: the part, its own open tab, which tabs show and the tabs' ids.
struct Shared {
    part: PartBase,
    here: Cell<i64>,
    visible: RefCell<Vec<bool>>,
    tabs: RefCell<Vec<ElementId>>,
}

impl Shared {
    /// The tab asked to be open, from 1: the part's state's, or its own without a name.
    fn tab(&self) -> i64 {
        self.part.state().and_then(|s| s.tab).unwrap_or(self.here.get())
    }

    fn set_tab(&self, tab: i64) {
        match self.part.name() {
            Some(name) if self.part.state().is_some() => self.part.host.change_state(name, &mut |s| s.tab = Some(tab)),
            _ => self.here.set(tab),
        }
    }

    /// The tab open now, from 1; none when no tab shows.
    fn open(&self) -> Option<i64> {
        shown_tab(&self.visible.borrow(), self.tab())
    }
}

/// The tab shown, from 1, of tabs that show or not, when `asked` is wanted:
/// it, when it is there and shows, else the first that shows; none when none does.
pub fn shown_tab(visible: &[bool], asked: i64) -> Option<i64> {
    if asked >= 1 && asked as usize <= visible.len() && visible[asked as usize - 1] {
        return Some(asked);
    }
    visible.iter().position(|v| *v).map(|i| i as i64 + 1)
}

pub struct TabsPart {
    base: PartBase,
    shared: Rc<Shared>,
    count: usize,
    look: Cell<LookNow>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let columns = container::columns_of(&spec);
    let shared = Rc::new(Shared {
        part: PartBase::new(spec.clone(), host.clone()),
        here: Cell::new(1),
        visible: RefCell::new(vec![true; spec.pages.len()]),
        tabs: RefCell::new(Vec::new()),
    });
    let mut base = PartBase::new(spec.clone(), host.clone());
    for (i, page) in spec.pages.iter().enumerate() {
        let title = page.title.clone();
        let tab = TabButton { node: Node::new(), shared: shared.clone(), number: i as i64 + 1, title };
        shared.tabs.borrow_mut().push(tab.node.id());
        base.node.add(Box::new(tab));
    }
    for page in &spec.pages {
        let grid = container::grid_of(&page.children, columns, GAP, &host);
        base.node.add(Box::new(Page::new(Box::new(ScrollView::new(Box::new(grid)).keeping_bar_room()))));
    }
    Box::new(TabsPart { base, shared, count: spec.pages.len(), look: Cell::new(LookNow::default()) })
}

impl TabsPart {
    /// The tab asked to be open, from 1.
    pub fn tab(&self) -> i64 {
        self.shared.tab()
    }

    /// The tab open now, from 1; none when no tab shows.
    pub fn open(&self) -> Option<i64> {
        self.shared.open()
    }

    /// The tabs of the strip, in order.
    pub fn tabs(&self) -> Vec<&dyn Element> {
        self.base.node.children()[..self.count].iter().map(|c| c.as_ref()).collect()
    }

    /// The pages, each its grid in a place that scrolls; only the open one shows.
    pub fn pages(&self) -> Vec<&Page> {
        self.base.node.children()[self.count..].iter().filter_map(|c| c.as_any().downcast_ref::<Page>()).collect()
    }

    /// The place of each page, which scrolls its grid when the grid is higher.
    pub fn views(&self) -> Vec<&ScrollView> {
        self.pages().into_iter().filter_map(|p| p.content().as_any().downcast_ref::<ScrollView>()).collect()
    }

    /// The grid of each page.
    pub fn grids(&self) -> Vec<&GridPanel> {
        self.pages().into_iter().map(Page::grid).collect()
    }

    /// A page shows unless code hid every part on it.
    fn page_shows(grid: &GridPanel) -> bool {
        grid.node().children().is_empty() || grid.node().children().iter().any(|c| c.is_shown())
    }
}

impl Element for TabsPart {
    part_element!();

    /// The room it needs beyond its rows: the strip of tabs and the page's padding over its highest page.
    fn around(&self) -> f64 {
        let highest = self.grids().into_iter().map(container::least_of).fold(0.0, f64::max);
        container::around_for(&self.base.spec, STRIP_HEIGHT + 4.0 * self.look.get().padding, highest)
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        let look = LookNow::of(cx.look());
        self.look.set(look);
        let visible: Vec<bool> = self.grids().into_iter().map(Self::page_shows).collect();
        *self.shared.visible.borrow_mut() = visible;
        let open = self.shared.open();
        let count = self.count;
        let mut x = area.x;
        for child in self.base.node.children_mut()[..count].iter_mut() {
            let width = child.as_any().downcast_ref::<TabButton>().map(|t| t.width_for(cx)).unwrap_or(0.0);
            child.arrange(Rect::new(x, area.y, width, STRIP_HEIGHT), cx);
            x += width + 2.0;
        }
        let place = Rect::new(area.x, area.y + STRIP_HEIGHT + 2.0 * look.padding, area.width, (area.height - STRIP_HEIGHT - 4.0 * look.padding).max(0.0));
        for (i, child) in self.base.node.children_mut()[count..].iter_mut().enumerate() {
            let shown = open == Some(i as i64 + 1);
            if let Some(page) = child.as_any_mut().downcast_mut::<Page>() {
                page.shown = shown;
            }
            if shown {
                child.arrange(place, cx);
            }
        }
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.base.node.bounds;
        let radius = look.radius("panel");
        let page = container::frame_of(Rect::new(b.x, b.y + STRIP_HEIGHT, b.width, (b.height - STRIP_HEIGHT).max(0.0)));
        p.fill(page, look.color("panel"), radius);
        p.stroke(page, look.color("strong_border"), look.border(), radius);
        let _framed = container::framing();
        element::draw_children(&self.base.node, p);
    }
}

impl Part for TabsPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn grows(&self) -> bool {
        self.grids().into_iter().any(container::grid_grows)
    }
}

/// A tab of the strip: pressed, it opens its page.
pub struct TabButton {
    node: Node,
    shared: Rc<Shared>,
    number: i64,
    title: Option<String>,
}

impl TabButton {
    /// What the tab says: its title in the language shown.
    pub fn title(&self) -> String {
        match &self.title {
            Some(t) if !t.is_empty() => self.shared.part.host.text(Some(t)),
            _ => String::new(),
        }
    }

    fn width_for(&self, cx: &Cx) -> f64 {
        let look = cx.look();
        cx.fonts().measure(&self.title(), "text", look.size("text"), false) + 6.0 * look.padding()
    }

    fn enabled(&self) -> bool {
        self.shared.part.enabled()
    }
}

impl Element for TabButton {
    crate::element_node!();

    fn focusable(&self) -> bool {
        self.enabled()
    }

    fn is_shown(&self) -> bool {
        self.shared.visible.borrow().get(self.number as usize - 1).copied().unwrap_or(true)
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if self.enabled() {
            self.shared.set_tab(self.number);
            cx.invalidate();
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !key.modifiers.is_empty() {
            return false;
        }
        match key.key {
            Key::Enter | Key::Space => {
                self.on_click(cx);
                true
            }
            Key::Left | Key::Right if self.enabled() => {
                let next = self.number + if key.key == Key::Left { -1 } else { 1 };
                let id = self.shared.tabs.borrow().get((next - 1).max(0) as usize).copied();
                match id.filter(|_| next >= 1) {
                    Some(id) => {
                        self.shared.set_tab(next);
                        cx.focus(Some(id));
                        true
                    }
                    None => false,
                }
            }
            _ => false,
        }
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        let open = self.shared.open() == Some(self.number);
        let values = look.part("tab", if self.node.hovered && !open { Some("hover") } else { None });
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("button"));
        let background = if open { look.color("panel") } else { values.background.unwrap_or_else(|| look.color("raised")) };
        let color = if open { look.color("text") } else { values.text.unwrap_or_else(|| look.color("muted")) };
        // Its top corners rounded: the bottom runs on into the page under it.
        let plate = Rect { height: b.height + radius, ..b };
        {
            let _clip = p.clip(Rect { height: b.height + if open { look.border() } else { 0.0 }, ..b });
            p.fill(plate, background, radius);
            let line = if open { look.color("strong_border") } else { values.border.unwrap_or_else(|| look.color("border")) };
            p.stroke(plate, line, look.border(), radius);
            if open {
                p.fill(Rect::new(b.x + look.border(), b.bottom() - look.border(), b.width - 2.0 * look.border(), 2.0 * look.border()), background, 0.0);
            }
        }
        if self.node.focused {
            p.stroke(b.inset(2.0), look.color("focus"), 2.0, radius);
        }
        let size = values.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
        let font = values.font.clone().unwrap_or_else(|| "text".to_string());
        p.text(&self.title(), b.inset(look.padding()), color, size, &font, values.bold(), TextAlign::Center);
    }
}
