//! Tabs: a strip of tabs in the theme's tab look, and under it the open
//! tab's page, a grid of its parts as wide as the part spans. The open tab
//! is its part's state's `tab`, counted from 1, so code reads and sets it; a
//! tab whose parts code has all hidden gives way to the first one that
//! shows, as a ribbon's tab does (conformance/ribbons.json `shown`). A page
//! whose parts go further down than its room scrolls, so none is out of reach.
//! A window's content (`content`) is tabs without a look of its own: no
//! strip, no frame, no fill; code opens the view it shows, as a tab.
//!
//! Options, off by default: `closing` gives each tab an x that closes it,
//! but a tab marked `permanent` (the last tab showing never closes), and
//! runs the part's `close`; code opening a closed tab (`tab`, which
//! `reopen` is) brings it back. `cycling` shows two sideways triangles at
//! the end of the tabs when they do not fit, going to the tab before or
//! after, round from the last to the first, the open tab kept in view.
//! `available` shows a dropdown of the closed tabs at the right end, which
//! brings the one picked back, open. Closed tabs stay closed while the window is open.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::element::{self, Cx, Element, ElementId, GridPanel, Node};
use crate::geometry::Rect;
use crate::grid::GAP;
use crate::input::{Key, KeyInput, PointerInput};
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::inputs::choice_list::{self, ListState, SharedList};
use crate::parts::{Part, PartBase, PartHost};

use super::container::{self, LookNow, Page};
use super::scroll_view::ScrollView;

/// How high the strip of tabs is.
pub const STRIP_HEIGHT: f64 = 32.0;

/// How wide and high a tab's x is.
const CLOSE_MARK: f64 = 14.0;

/// How wide each of the two triangles is.
const CYCLE_WIDTH: f64 = 24.0;

/// What the tabs share with their strip's tabs: the part, its own open tab,
/// which tabs show, the tabs' ids, and with the options which are closed,
/// which are permanent, which are in the strip now and where it starts.
struct Shared {
    part: PartBase,
    here: Cell<i64>,
    visible: RefCell<Vec<bool>>,
    tabs: RefCell<Vec<ElementId>>,
    closing: bool,
    cycling: bool,
    available: bool,
    permanent: Vec<bool>,
    closed: RefCell<Vec<bool>>,
    placed: RefCell<Vec<bool>>,
    first: Cell<usize>,
}

impl Shared {
    /// The tab asked to be open, from 1: the part's state's, or its own without a name.
    fn tab(&self) -> i64 {
        self.part.state().and_then(|s| s.tab).unwrap_or(self.here.get())
    }

    fn set_tab(&self, tab: i64) {
        match self.part.name() {
            Some(name) if self.part.state().is_some() => crate::parts::change_state(&*self.part.host, name, |s| s.tab = Some(tab)),
            _ => self.here.set(tab),
        }
    }

    /// The tab open now, from 1; none when no tab shows.
    fn open(&self) -> Option<i64> {
        shown_tab(&self.shows(), self.tab())
    }

    /// Each tab shows: code has not hidden its parts and the person has not closed it.
    fn shows(&self) -> Vec<bool> {
        let closed = self.closed.borrow();
        self.visible.borrow().iter().enumerate().map(|(i, v)| *v && !closed.get(i).copied().unwrap_or(false)).collect()
    }

    /// Whether the tab `number` (from 1) has an x.
    fn closable(&self, number: i64) -> bool {
        self.closing && !self.permanent.get(number as usize - 1).copied().unwrap_or(false)
    }

    /// Closes the tab `number`, unless it is the last that shows; the open one gives way to the next that shows, else the one before.
    fn close(&self, number: i64) -> bool {
        let shows = self.shows();
        let at = number as usize - 1;
        if !self.closable(number) || !shows.get(at).copied().unwrap_or(false) || shows.iter().filter(|s| **s).count() < 2 {
            return false;
        }
        if self.open() == Some(number) {
            let next = (at + 1..shows.len()).find(|i| shows[*i]).or_else(|| (0..at).rev().find(|i| shows[*i]));
            if let Some(next) = next {
                self.set_tab(next as i64 + 1);
            }
        }
        if let Some(c) = self.closed.borrow_mut().get_mut(at) {
            *c = true;
        }
        true
    }

    /// The closed tabs, from 1.
    fn closed_tabs(&self) -> Vec<i64> {
        self.closed.borrow().iter().enumerate().filter(|(_, c)| **c).map(|(i, _)| i as i64 + 1).collect()
    }

    /// Brings the closed tab `number` back and opens it.
    fn reopen(&self, number: i64) {
        if let Some(c) = self.closed.borrow_mut().get_mut(number as usize - 1) {
            *c = false;
        }
        self.set_tab(number);
    }

    /// Opens the tab `by` places on among those that show, round from the last to the first.
    fn step(&self, by: i64) {
        let shows: Vec<i64> = self.shows().iter().enumerate().filter(|(_, s)| **s).map(|(i, _)| i as i64 + 1).collect();
        if shows.is_empty() {
            return;
        }
        let at = self.open().and_then(|o| shows.iter().position(|n| *n == o)).unwrap_or(0) as i64;
        let next = (at + by).rem_euclid(shows.len() as i64) as usize;
        self.set_tab(shows[next]);
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
    /// The triangles and the dropdown of closed tabs, between the tabs and the pages; none for a window's content.
    controls: usize,
    /// A window's content: only the open page, nothing of its own around it.
    plain: bool,
    look: Cell<LookNow>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let columns = container::columns_of(&spec);
    let plain = spec.kind == "content";
    let count = spec.pages.len();
    let shared = Rc::new(Shared {
        part: PartBase::new(spec.clone(), host.clone()),
        here: Cell::new(crate::markup::design_shown(&spec)),
        visible: RefCell::new(vec![true; count]),
        tabs: RefCell::new(Vec::new()),
        closing: !plain && spec.flag("closing"),
        cycling: !plain && spec.flag("cycling"),
        available: !plain && spec.flag("closing") && spec.flag("available"),
        permanent: spec.pages.iter().map(|p| p.permanent).collect(),
        closed: RefCell::new(vec![false; count]),
        placed: RefCell::new(vec![true; count]),
        first: Cell::new(0),
    });
    let mut base = PartBase::new(spec.clone(), host.clone());
    for (i, page) in spec.pages.iter().enumerate().filter(|_| !plain) {
        let title = page.title.clone();
        let tab = TabButton { node: Node::new(), shared: shared.clone(), number: i as i64 + 1, title, on_mark: false };
        shared.tabs.borrow_mut().push(tab.node.id());
        base.node.add(Box::new(tab));
    }
    if !plain {
        base.node.add(Box::new(CycleButton { node: Node::new(), shared: shared.clone(), by: -1, shown: false }));
        base.node.add(Box::new(CycleButton { node: Node::new(), shared: shared.clone(), by: 1, shown: false }));
        base.node.add(Box::new(AvailableBox { node: Node::new(), shared: shared.clone(), list: None, shown: false }));
    }
    for page in &spec.pages {
        // A page with columns of its own (a view keeps its grid) lays its parts out on them.
        let grid = container::grid_of(&page.children, page.grid.unwrap_or(columns).max(1), GAP, &host).owned(page.item.as_deref());
        base.node.add(Box::new(Page::new(Box::new(ScrollView::new(Box::new(grid)).keeping_bar_room()))));
    }
    let (count, controls) = if plain { (0, 0) } else { (count, 3) };
    Box::new(TabsPart { base, shared, count, controls, plain, look: Cell::new(LookNow::default()) })
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
        self.base.node.children()[self.count + self.controls..].iter().filter_map(|c| c.as_any().downcast_ref::<Page>()).collect()
    }

    /// The tabs the person closed, from 1.
    pub fn closed(&self) -> Vec<i64> {
        self.shared.closed_tabs()
    }

    /// Where the triangles are, the tab before and the next; none when they do not show.
    pub fn cycle_buttons(&self) -> Option<(Rect, Rect)> {
        let controls = &self.base.node.children()[self.count..self.count + self.controls];
        let shown: Vec<Rect> = controls.iter().filter_map(|c| c.as_any().downcast_ref::<CycleButton>()).filter(|c| c.shown).map(|c| c.node.bounds).collect();
        match shown.as_slice() {
            [before, next] => Some((*before, *next)),
            _ => None,
        }
    }

    /// Where the dropdown of closed tabs is; none when it does not show.
    pub fn available_box(&self) -> Option<Rect> {
        let controls = &self.base.node.children()[self.count..self.count + self.controls];
        controls.iter().filter_map(|c| c.as_any().downcast_ref::<AvailableBox>()).find(|c| c.shown).map(|c| c.node.bounds)
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
        let own = if self.plain { 0.0 } else { STRIP_HEIGHT + 4.0 * self.look.get().padding };
        container::around_for(&self.base.spec, own, highest)
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        let look = LookNow::of(cx.look());
        self.look.set(look);
        let visible: Vec<bool> = self.grids().into_iter().map(Self::page_shows).collect();
        *self.shared.visible.borrow_mut() = visible;
        // Code opening a closed tab (`tab`, which `reopen` is) brings it back.
        let asked = self.shared.tab();
        if asked >= 1 && self.shared.closed.borrow().get(asked as usize - 1).copied().unwrap_or(false) {
            self.shared.reopen(asked);
        }
        let open = self.shared.open();
        let count = self.count;
        self.arrange_strip(area, open, cx);
        let place = if self.plain { area } else { Rect::new(area.x, area.y + STRIP_HEIGHT + 2.0 * look.padding, area.width, (area.height - STRIP_HEIGHT - 4.0 * look.padding).max(0.0)) };
        for (i, child) in self.base.node.children_mut()[count + self.controls..].iter_mut().enumerate() {
            let shown = open == Some(i as i64 + 1);
            if let Some(page) = child.as_any_mut().downcast_mut::<Page>() {
                page.shown = shown;
            }
            if shown {
                element::arrange_safely(child.as_mut(), place, cx);
            }
        }
    }

    fn draw(&self, p: &Painter) {
        if self.plain {
            element::draw_children(&self.base.node, p);
            return;
        }
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

impl TabsPart {
    /// Lays the strip out: the tabs that show from where it starts, the
    /// triangles after them when they do not fit and cycling is on, the
    /// dropdown of closed tabs at the right end when one is closed.
    fn arrange_strip(&mut self, area: Rect, open: Option<i64>, cx: &mut Cx) {
        let count = self.count;
        if count == 0 {
            return;
        }
        let shows = self.shared.shows();
        let widths: Vec<f64> = self.base.node.children()[..count].iter().map(|c| c.as_any().downcast_ref::<TabButton>().map(|t| t.width_for(cx)).unwrap_or(0.0)).collect();
        let any_closed = self.shared.available && !self.shared.closed_tabs().is_empty();
        let available_width = if any_closed { AvailableBox::width_for(cx) } else { 0.0 };
        let mut room = area.width - if any_closed { available_width + 6.0 } else { 0.0 };
        let total: f64 = widths.iter().zip(&shows).filter(|(_, s)| **s).map(|(w, _)| w + 2.0).sum();
        let cycles = self.shared.cycling && total > room + 0.5;
        if cycles {
            room -= 2.0 * CYCLE_WIDTH + 6.0;
        }
        // Where the strip starts: never after the open tab, and far enough on that the open tab fits.
        let mut first = if cycles { self.shared.first.get().min(count - 1) } else { 0 };
        if let Some(open) = open.map(|o| o as usize - 1) {
            first = first.min(open);
            while first < open && (first..=open).filter(|i| shows[*i]).map(|i| widths[i] + 2.0).sum::<f64>() > room {
                first += 1;
            }
        }
        self.shared.first.set(first);
        let mut placed = vec![false; count];
        let mut x = area.x;
        for (i, child) in self.base.node.children_mut()[..count].iter_mut().enumerate() {
            let fits = shows[i] && i >= first && (!cycles || x + widths[i] <= area.x + room + 0.5 || Some(i as i64 + 1) == open);
            if fits {
                element::arrange_safely(child.as_mut(), Rect::new(x, area.y, widths[i], STRIP_HEIGHT), cx);
                x += widths[i] + 2.0;
                placed[i] = true;
            } else {
                element::arrange_safely(child.as_mut(), Rect::new(area.x, area.y, 0.0, 0.0), cx);
            }
        }
        *self.shared.placed.borrow_mut() = placed;
        let cycle_x = area.x + room + 4.0;
        let controls = &mut self.base.node.children_mut()[count..count + self.controls];
        for (i, control) in controls.iter_mut().enumerate() {
            if let Some(button) = control.as_any_mut().downcast_mut::<CycleButton>() {
                button.shown = cycles;
            }
            if let Some(available) = control.as_any_mut().downcast_mut::<AvailableBox>() {
                available.shown = any_closed;
            }
            let place = match i {
                0 if cycles => Rect::new(cycle_x, area.y + 4.0, CYCLE_WIDTH, STRIP_HEIGHT - 8.0),
                1 if cycles => Rect::new(cycle_x + CYCLE_WIDTH, area.y + 4.0, CYCLE_WIDTH, STRIP_HEIGHT - 8.0),
                2 if any_closed => Rect::new(area.right() - available_width, area.y + 3.0, available_width, STRIP_HEIGHT - 6.0),
                _ => Rect::new(area.x, area.y, 0.0, 0.0),
            };
            element::arrange_safely(control.as_mut(), place, cx);
        }
    }
}

impl Part for TabsPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    /// A window's content always fills the rows it spans, as it is only a
    /// container: its view keeps its own rows inside. Tabs grow with a page that grows.
    fn grows(&self) -> bool {
        self.plain || self.grids().into_iter().any(container::grid_grows)
    }
}

/// A tab of the strip: pressed, it opens its page.
pub struct TabButton {
    node: Node,
    shared: Rc<Shared>,
    number: i64,
    title: Option<String>,
    /// The press began on its x.
    on_mark: bool,
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
        let mark = if self.shared.closable(self.number) { CLOSE_MARK + look.padding() } else { 0.0 };
        cx.fonts().measure(&self.title(), "text", look.size("text"), false) + 6.0 * look.padding() + mark
    }

    /// Where its x is, which closes it; none for a tab that does not close.
    pub fn close_mark(&self) -> Option<Rect> {
        let b = self.node.bounds;
        (self.shared.closable(self.number) && b.width > 0.0).then(|| Rect::new(b.right() - CLOSE_MARK - 6.0, b.y + (b.height - CLOSE_MARK) / 2.0, CLOSE_MARK, CLOSE_MARK))
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
        let at = self.number as usize - 1;
        self.shared.shows().get(at).copied().unwrap_or(true) && self.shared.placed.borrow().get(at).copied().unwrap_or(true)
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, _cx: &mut Cx) {
        let b = self.node.bounds;
        self.on_mark = self.close_mark().is_some_and(|m| m.contains(b.x + at.x, b.y + at.y));
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if !self.enabled() {
            return;
        }
        if std::mem::take(&mut self.on_mark) {
            if self.shared.close(self.number) {
                self.shared.part.fire("close");
            }
        } else {
            self.shared.set_tab(self.number);
        }
        cx.invalidate();
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
        // The open tab in the theme's selected look, the others plain or hovered.
        let state = if open { Some("selected") } else if self.node.hovered { Some("hover") } else { None };
        let values = look.part("tab", state);
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("button"));
        let background = values.background.unwrap_or_else(|| look.color(if open { "panel" } else { "raised" }));
        let color = values.text.unwrap_or_else(|| look.color(if open { "text" } else { "muted" }));
        let line = values.border.unwrap_or_else(|| look.color("border"));
        // Its top corners rounded: the bottom runs on into the page under it
        // when the open tab has a border, else it stands on the page's edge.
        let joins = open && line.a() > 0;
        let plate = Rect { height: b.height + radius, ..b };
        {
            let _clip = p.clip(Rect { height: b.height + if joins { look.border() } else { 0.0 }, ..b });
            p.fill(plate, background, radius);
            p.stroke(plate, line, look.border(), radius);
            if joins {
                p.fill(Rect::new(b.x + look.border(), b.bottom() - look.border(), b.width - 2.0 * look.border(), 2.0 * look.border()), background, 0.0);
            }
        }
        if self.node.focused {
            p.stroke(b.inset(2.0), look.color("focus"), 2.0, radius);
        }
        let size = values.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
        let font = values.font.clone().unwrap_or_else(|| "text".to_string());
        let text_area = match self.close_mark() {
            Some(mark) => Rect { width: (mark.x - b.x).max(0.0), ..b }.inset(look.padding()),
            None => b.inset(look.padding()),
        };
        p.text(&self.title(), text_area, color, size, &font, values.bold(), TextAlign::Center);
        // Its x, in the tab's text color.
        if let Some(mark) = self.close_mark() {
            let m = mark.inset(3.5);
            p.line((m.x, m.y), (m.right(), m.bottom()), color, 1.5);
            p.line((m.right(), m.y), (m.x, m.bottom()), color, 1.5);
        }
    }
}

/// One of the two triangles at the end of tabs that do not fit: the tab before, or the next.
pub struct CycleButton {
    node: Node,
    shared: Rc<Shared>,
    by: i64,
    shown: bool,
}

impl Element for CycleButton {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        self.shown
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if self.shared.part.enabled() {
            self.shared.step(self.by);
            cx.invalidate();
        }
    }

    fn draw(&self, p: &Painter) {
        if !self.shown {
            return;
        }
        let look = p.look;
        let b = self.node.bounds;
        let background = look.color(if self.node.hovered { "raised" } else { "panel" });
        p.fill(b, background, 0.0);
        p.stroke(b, look.color("strong_border"), look.border(), 0.0);
        p.side_triangle(b, look.color("text"), self.by < 0);
    }
}

/// The dropdown of the closed tabs at the right end: the one picked comes back, open.
pub struct AvailableBox {
    node: Node,
    shared: Rc<Shared>,
    list: Option<(ElementId, SharedList)>,
    shown: bool,
}

impl AvailableBox {
    /// What it says, in the person's language.
    fn text() -> String {
        crate::locale::translate("tabsAddCaption", "+ Add a tab")
    }

    fn width_for(cx: &Cx) -> f64 {
        let look = cx.look();
        cx.fonts().measure(&Self::text(), "text", look.size("text"), false) + 4.0 * look.padding() + 16.0
    }

    /// The closed tabs, from 1, with what each says.
    fn closed(&self) -> Vec<(i64, String)> {
        let host = &self.shared.part.host;
        let titles: Vec<Option<String>> = self.shared.part.spec.pages.iter().map(|p| p.title.clone()).collect();
        self.shared.closed_tabs().into_iter().map(|n| (n, host.text(titles.get(n as usize - 1).cloned().flatten().as_deref()))).collect()
    }

    fn open_list(&mut self, cx: &mut Cx) {
        let closed = self.closed();
        if closed.is_empty() || self.list.is_some() {
            return;
        }
        let labels: Vec<String> = closed.into_iter().map(|(_, t)| t).collect();
        let bounds = self.node.bounds;
        let width = bounds.width.max(160.0);
        let place = choice_list::below(cx.whole(), Rect { x: bounds.right() - width, ..bounds }, width, labels.len());
        let state: SharedList = Rc::new(RefCell::new(ListState::new(labels, None)));
        let id = choice_list::open(cx, state.clone(), place, self.node.id());
        self.list = Some((id, state));
    }

    fn close_list(&mut self, cx: &mut Cx) {
        if let Some((id, _)) = self.list.take() {
            cx.close(id);
        }
        cx.invalidate();
    }

    fn pick(&mut self, at: usize, cx: &mut Cx) {
        if let Some((number, _)) = self.closed().get(at) {
            self.shared.reopen(*number);
        }
        cx.invalidate();
    }
}

impl Element for AvailableBox {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        self.shown
    }

    fn focusable(&self) -> bool {
        self.shown && self.shared.part.enabled()
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if !self.shared.part.enabled() {
            return;
        }
        if self.list.is_none() {
            self.open_list(cx);
        } else {
            self.close_list(cx);
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !key.modifiers.is_empty() {
            return false;
        }
        let Some((_, state)) = self.list.as_ref().map(|(i, s)| (*i, s.clone())) else {
            if matches!(key.key, Key::Enter | Key::Space | Key::Down) {
                self.open_list(cx);
                return true;
            }
            return false;
        };
        match key.key {
            Key::Down => state.borrow_mut().move_by(1),
            Key::Up => state.borrow_mut().move_by(-1),
            Key::Enter | Key::Space => {
                let at = state.borrow().highlighted;
                self.close_list(cx);
                if let Some(at) = at {
                    self.pick(at, cx);
                }
            }
            Key::Escape => self.close_list(cx),
            _ => return false,
        }
        cx.invalidate();
        true
    }

    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        let Some((id, state)) = self.list.take() else { return };
        if id != overlay {
            self.list = Some((id, state));
            return;
        }
        let picked = state.borrow().picked;
        if let Some(at) = picked {
            self.pick(at, cx);
        }
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        if !self.shown {
            return;
        }
        let look = p.look;
        let b = self.node.bounds;
        let radius = look.radius("button");
        p.fill(b, look.color(if self.node.hovered { "raised" } else { "panel" }), radius);
        p.stroke(b, look.color("strong_border"), look.border(), radius);
        if self.node.focused {
            p.focus_ring(b, radius);
        }
        let text = Rect { width: (b.width - 16.0).max(0.0), ..b }.inset(look.padding());
        p.text(&Self::text(), text, look.color("text"), look.size("text"), "text", false, TextAlign::Left);
        p.down_arrow(Rect::new(b.right() - 18.0, b.y, 16.0, b.height), look.color("text"));
    }
}
