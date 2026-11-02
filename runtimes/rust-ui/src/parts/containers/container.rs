//! What every container part shares (DESIGN 7.8, 7.9): a panel, a form,
//! tabs, steps, an accordion, a splitter, a scroll box and a popover each
//! hold grids of parts, made by their kinds through `parts::make` and placed
//! as the window's grid places its own. A container grows when a part in it
//! does, and it frames what it holds: a list, table or tree inside it shows
//! no border of its own (DESIGN 7.8 "frames"), which such a part asks with
//! `is_inside` as it draws. Also here: the pieces containers draw with (a
//! button in Clarity's own words or a page's title, a header's chevron, a
//! page that shows or not) and what a container keeps of the look for the
//! room it asks before it is drawn.

use std::cell::Cell;
use std::rc::Rc;

use skia_safe::{Color, Paint, PaintStyle, Path, Point};

use crate::element::{draw_children, Cx, Element, GridPanel, Node};
use crate::geometry::Rect;
use crate::grid::{GAP, ROW_HEIGHT};
use crate::input::{Key, KeyInput};
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::parts::{self, PartHost};
use crate::theme::ThemeLook;

thread_local! {
    static FRAMING: Cell<u32> = const { Cell::new(0) };
}

/// While it lives, what is drawn is drawn inside a container that frames its parts.
pub struct Framing(());

impl Drop for Framing {
    fn drop(&mut self) {
        FRAMING.with(|f| f.set(f.get().saturating_sub(1)));
    }
}

/// A container draws its parts within this: a list, table or tree drawn then is inside it.
pub fn framing() -> Framing {
    FRAMING.with(|f| f.set(f.get() + 1));
    Framing(())
}

/// Whether the part drawing now is inside a container that frames it, so a
/// list, table or tree draws no border of its own; on the window and in a
/// sidebar it keeps one. Asked as it draws.
pub fn is_inside() -> bool {
    FRAMING.with(|f| f.get() > 0)
}

/// How many columns a container's own grids have: as many as it spans.
pub fn columns_of(spec: &PartSpec) -> i64 {
    spec.number("width").map(|w| w as i64).unwrap_or(1).max(1)
}

/// The grid of `parts`, each made by its kind with `host` behind it, at its
/// place. Each is told in its spec that it is inside a container
/// (`"inside": true`), so a list, table or tree knows it as it is made too.
pub fn grid_of(parts: &[PartSpec], columns: i64, gap: f64, host: &Rc<dyn PartHost>) -> GridPanel {
    let mut grid = GridPanel::new(columns);
    grid.gap = gap;
    for spec in parts {
        let mut spec = spec.clone();
        spec.props.insert("inside".to_string(), serde_json::Value::Bool(true));
        let part = parts::make(spec, host.clone());
        let place = parts::place_of(part.as_ref());
        grid.add(part, place);
    }
    grid
}

/// Whether a part in the grid grows.
pub fn grid_grows(grid: &GridPanel) -> bool {
    grid.node().children().iter().any(|c| c.as_part().is_some_and(|p| p.grows()))
}

/// How high a grid is at the least: its rows at 36, and the room its own containers ask around theirs.
pub fn least_of(grid: &GridPanel) -> f64 {
    if grid.places().is_empty() {
        return 0.0;
    }
    grid.content_for(1000.0, 0.0)
}

/// How high a part's own rows are at the least.
pub fn own_least(spec: &PartSpec) -> f64 {
    let rows = spec.place(false).height.max(1) as f64;
    rows * ROW_HEIGHT + (rows - 1.0) * GAP
}

/// The room a part asks beyond its rows: what it puts around its grid plus what the grid needs more than the rows give, never less than nothing.
pub fn around_for(spec: &PartSpec, frame: f64, needed: f64) -> f64 {
    (frame + needed - own_least(spec)).max(0.0)
}

/// The frame around what a container holds, out in the gap between columns, so its grid keeps the window's column lines.
pub fn frame_of(area: Rect) -> Rect {
    Rect::new(area.x - 3.0, area.y, area.width + 6.0, area.height)
}

/// How high a header is: a line of small text with room around it.
pub fn header_height(look: &ThemeLook) -> f64 {
    look.size("small") + 4.0 * look.padding()
}

/// A chevron pointing down (open) or right (folded), in `color`, in the middle of `area`.
pub fn chevron(p: &Painter, area: Rect, color: Color, open: bool) {
    let (w, h) = if open { (8.0, 5.0) } else { (5.0, 8.0) };
    let x = (area.x + (area.width - w) / 2.0) as f32;
    let y = (area.y + (area.height - h) / 2.0) as f32;
    let points: Vec<Point> = if open {
        vec![Point::new(x, y), Point::new(x + 8.0, y), Point::new(x + 4.0, y + 5.0)]
    } else {
        vec![Point::new(x, y), Point::new(x + 5.0, y + 4.0), Point::new(x, y + 8.0)]
    };
    let path = Path::polygon(&points, true, None, None);
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Fill);
    p.canvas.draw_path(&path, &paint);
}

/// What a container keeps of the look it was last laid out in: the room it
/// asks around its grids is asked before it is drawn, with no look at hand.
/// Until it is laid out it reads Clarity's own light look.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookNow {
    pub padding: f64,
    pub border: f64,
    pub small: f64,
    pub text: f64,
}

impl LookNow {
    pub fn of(look: &ThemeLook) -> LookNow {
        LookNow { padding: look.padding(), border: look.border(), small: look.size("small"), text: look.size("text") }
    }

    /// How high a header is in it.
    pub fn header(&self) -> f64 {
        self.small + 4.0 * self.padding
    }
}

impl Default for LookNow {
    fn default() -> Self {
        LookNow::of(&ThemeLook::clarity(false))
    }
}

/// A page of a container, a grid of parts, which shows or not: one that is
/// away draws nothing and takes no pointer, and nothing in it is reached by Tab.
pub struct Page {
    node: Node,
    pub shown: bool,
}

impl Page {
    /// A page holding `content` (a grid, or a scroll view around one).
    pub fn new(content: Box<dyn Element>) -> Page {
        let mut node = Node::new();
        node.add(content);
        Page { node, shown: true }
    }

    pub fn content(&self) -> &dyn Element {
        self.node.children()[0].as_ref()
    }

    pub fn content_mut(&mut self) -> &mut dyn Element {
        self.node.children_mut()[0].as_mut()
    }

    /// Its grid: the content, or the grid in the scroll view that is its content.
    pub fn grid(&self) -> &GridPanel {
        grid_in(self.content()).expect("a page holds a grid")
    }
}

/// The grid an element is, or the grid in the scroll view it is.
pub fn grid_in(e: &dyn Element) -> Option<&GridPanel> {
    if let Some(grid) = e.as_any().downcast_ref::<GridPanel>() {
        return Some(grid);
    }
    e.as_any().downcast_ref::<super::scroll_view::ScrollView>().and_then(|v| v.content().as_any().downcast_ref::<GridPanel>())
}

impl Element for Page {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        self.shown
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        self.content_mut().arrange(area, cx);
    }

    fn draw(&self, p: &Painter) {
        if self.shown {
            draw_children(&self.node, p);
        }
    }
}

/// A button a container draws in Clarity's own words or a page's title: a
/// wizard's Back, Next and Finish, an accordion's section header, a tab. It
/// looks as the theme's button does (hover, pressed), the accent one in the
/// accent. What it says, whether it shows and is usable and what a press
/// does are the container's, reached through what it shares with it.
pub struct ContainerButton {
    node: Node,
    text: Box<dyn Fn() -> String>,
    pressed: Box<dyn Fn(&mut Cx)>,
    /// The window's default action here (Next, Finish): drawn in the accent.
    pub accent: bool,
    /// Its text on the left with a chevron before it, open or not, as a section's header.
    pub chevron: Option<bool>,
    shows: Option<Box<dyn Fn() -> bool>>,
    usable: Option<Box<dyn Fn() -> bool>>,
}

impl ContainerButton {
    pub fn new(text: impl Fn() -> String + 'static, pressed: impl Fn(&mut Cx) + 'static) -> ContainerButton {
        ContainerButton { node: Node::new(), text: Box::new(text), pressed: Box::new(pressed), accent: false, chevron: None, shows: None, usable: None }
    }

    /// Shown only while `shows` says so: a wizard shows only the buttons its step has.
    pub fn shows(mut self, shows: impl Fn() -> bool + 'static) -> ContainerButton {
        self.shows = Some(Box::new(shows));
        self
    }

    /// Pressed only while `usable` says so.
    pub fn usable(mut self, usable: impl Fn() -> bool + 'static) -> ContainerButton {
        self.usable = Some(Box::new(usable));
        self
    }

    pub fn accent(mut self) -> ContainerButton {
        self.accent = true;
        self
    }

    pub fn text(&self) -> String {
        (self.text)()
    }

    pub fn is_usable(&self) -> bool {
        self.usable.as_ref().is_none_or(|u| u())
    }

    /// Pressed, as a click or Enter or Space does.
    pub fn press(&mut self, cx: &mut Cx) {
        if self.is_shown() && self.is_usable() {
            (self.pressed)(cx);
            cx.invalidate();
        }
    }

    /// How wide it is for its text.
    pub fn width_for(&self, cx: &Cx) -> f64 {
        let look = cx.look();
        cx.fonts().measure(&self.text(), "text", look.size("text"), false) + 6.0 * look.padding()
    }
}

impl Element for ContainerButton {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        self.shows.as_ref().is_none_or(|s| s())
    }

    fn focusable(&self) -> bool {
        self.is_usable()
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.press(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !matches!(key.key, Key::Enter | Key::Space) || !key.modifiers.is_empty() {
            return false;
        }
        self.press(cx);
        true
    }

    fn draw(&self, p: &Painter) {
        if !self.is_shown() {
            return;
        }
        let look = p.look;
        let bounds = self.node.bounds;
        let state = if self.node.pressed { Some("pressed") } else if self.node.hovered { Some("hover") } else { None };
        let values = look.part("button", state);
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("button"));
        let background = if self.accent { look.color("accent") } else { values.background.unwrap_or_else(|| look.color("raised")) };
        let mut color = if self.accent { look.color("on_accent") } else { values.text.unwrap_or_else(|| look.color("text")) };
        if !self.is_usable() {
            color = look.color("muted");
        }
        p.fill(bounds, background, radius);
        let line = if self.accent { background } else { values.border.unwrap_or_else(|| look.color("strong_border")) };
        p.stroke(bounds, line, look.border(), radius);
        if self.node.focused {
            p.stroke(bounds.inset(-2.0), look.color("focus"), 2.0, radius + 2.0);
        }
        let size = values.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
        let font = values.font.clone().unwrap_or_else(|| "text".to_string());
        match self.chevron {
            Some(open) => {
                let pad = 2.0 * look.padding();
                chevron(p, Rect::new(bounds.x + pad, bounds.y, 10.0, bounds.height), color, open);
                let text = Rect::new(bounds.x + pad + 18.0, bounds.y, (bounds.width - pad * 2.0 - 18.0).max(0.0), bounds.height);
                p.text(&self.text(), text, color, size, &font, values.bold() || open, TextAlign::Left);
            }
            None => p.text(&self.text(), bounds.inset(look.padding()), color, size, &font, values.bold(), TextAlign::Center),
        }
    }
}
