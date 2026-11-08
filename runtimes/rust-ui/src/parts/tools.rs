//! The parts of a toolbox (`page Painting: toolbox`, docs/IMAGE-PLAN.md),
//! their look and kind given: a tool, a group of tools, the brush size, the
//! opacity and the colours. What a tool does is built in (`does pencil`, an
//! action on the canvas the toolbox serves) or coded (`on click: handler`).
//! In a toolbox over a canvas they work on it through the toolbox the canvas
//! shares with them (`link`); on their own, as the designer draws them, they
//! only show.

use std::rc::Rc;
use std::time::Duration;

use skia_safe::Color;

use crate::element::{Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::input::{Cursor, PointerButton, PointerInput};
use crate::locale;
use crate::markup::PartSpec;
use crate::overlay::Overlay;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::data::canvas::tool_word;
use crate::parts::data::canvas::toolbox::{icon_of, Toolbox};
use crate::parts::inputs::color::PaletteGrid;
use crate::parts::{Part, PartBase, PartHost};

/// How long a press on a group is held before its other tools show.
const HOLD: Duration = Duration::from_millis(350);
const CELL: f64 = 32.0;
const GAP: f64 = 4.0;
const PAD: f64 = 6.0;

/// A tool's button: its icon, in the accent while it is in hand.
fn draw_button(p: &Painter, place: Rect, icon: Option<String>, word: &str, in_hand: bool) {
    let look = p.look;
    let place = Rect::new(place.x, place.y, place.width.min(place.height), place.height.min(place.width));
    p.fill(place, look.color(if in_hand { "accent_soft" } else { "raised" }), 4.0);
    if in_hand {
        p.stroke(place, look.color("accent"), 1.5, 4.0);
    }
    if !crate::parts::containers::icon::draw_icon(p, icon.as_deref(), place.inset(7.0), look.color("text")) {
        p.text(&word.chars().take(2).collect::<String>(), place, look.color("text"), look.size("small"), "text", false, TextAlign::Center);
    }
}

/// What a tool's spec says: what it does built in, its icon (its own, else the built-in one's), its word.
struct ToolLook {
    does: Option<String>,
    icon: Option<String>,
    word: String,
}

fn look_of(spec: &PartSpec, host: &dyn PartHost, paints: bool) -> ToolLook {
    let does = spec.text("does").filter(|d| !d.is_empty()).map(str::to_string);
    let icon = spec.text("icon").filter(|i| !i.is_empty()).map(str::to_string).or_else(|| does.as_deref().map(|d| icon_of(d, paints).to_string()));
    let word = spec.text("tooltip").map(|t| host.text(Some(t))).or_else(|| does.as_deref().map(tool_word)).or_else(|| spec.name().map(str::to_string)).unwrap_or_default();
    ToolLook { does, icon: icon.and_then(|i| host.icon_path(&i)), word }
}

/// A tool of a spec pressed: a built-in canvas tool taken in hand, a built-in action asked of the canvas, or its handlers run.
fn use_tool(spec: &PartSpec, host: &dyn PartHost, toolbox: Option<&Rc<Toolbox>>) {
    match (spec.text("does").filter(|d| !d.is_empty()), toolbox) {
        (Some(does), Some(toolbox)) => toolbox.does(does),
        (Some(_), None) => {}
        (None, _) => {
            for call in spec.on("click") {
                host.run(spec.name(), call);
            }
        }
    }
    // A memo's toolbox at the caret goes once a tool did its action.
    if let Some(toolbox) = toolbox {
        toolbox.acted();
    }
}

/// `tool name pencilTool icon "pencil" does pencil`, or `on click: handler`.
pub struct ToolPart {
    base: PartBase,
    toolbox: Option<Rc<Toolbox>>,
}

pub fn make_tool(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(ToolPart { base: PartBase::new(spec, host), toolbox: None })
}

impl ToolPart {
    pub fn link(&mut self, toolbox: Rc<Toolbox>) {
        self.toolbox = Some(toolbox);
    }

    /// What it does built in; none for a coded tool.
    pub fn does(&self) -> Option<&str> {
        self.base.spec.text("does").filter(|d| !d.is_empty())
    }

    fn paints(&self) -> bool {
        self.toolbox.as_ref().is_none_or(|t| t.paints())
    }
}

impl Element for ToolPart {
    part_element!();

    fn cursor_at(&self, _x: f64, _y: f64) -> Option<Cursor> {
        Some(Cursor::Hand)
    }

    fn tooltip_at(&self, _x: f64, _y: f64) -> Option<String> {
        Some(look_of(&self.base.spec, self.base.host.as_ref(), self.paints()).word)
    }

    fn on_click(&mut self, cx: &mut Cx) {
        use_tool(&self.base.spec, self.base.host.as_ref(), self.toolbox.as_ref());
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        let look = look_of(&self.base.spec, self.base.host.as_ref(), self.paints());
        let in_hand = self.toolbox.as_ref().is_some_and(|t| look.does.as_deref() == Some(t.tool().as_str()));
        draw_button(p, self.base.node.bounds, look.icon, &look.word, in_hand);
    }
}

impl Part for ToolPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

/// `toolgroup` with its tools inside: one button showing the tool last used, the others beside it when it is held.
pub struct ToolGroupPart {
    base: PartBase,
    toolbox: Option<Rc<Toolbox>>,
    holding: bool,
    flyout: Option<ElementId>,
}

pub fn make_group(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(ToolGroupPart { base: PartBase::new(spec, host), toolbox: None, holding: false, flyout: None })
}

impl ToolGroupPart {
    pub fn link(&mut self, toolbox: Rc<Toolbox>) {
        self.toolbox = Some(toolbox);
    }

    /// Its tools, as written.
    pub fn tools(&self) -> &[PartSpec] {
        &self.base.spec.children
    }

    fn key(&self) -> String {
        self.base.name().map(str::to_string).unwrap_or_else(|| format!("{:?}", self.base.node.id()))
    }

    /// The tool its button shows: the one in hand when it is one of its own, else the last used here, else its first.
    pub fn shown(&self) -> Option<&PartSpec> {
        let tools = self.tools();
        let toolbox = self.toolbox.as_ref();
        let in_hand = toolbox.map(|t| t.tool()).unwrap_or_default();
        tools
            .iter()
            .find(|t| t.text("does") == Some(in_hand.as_str()))
            .or_else(|| toolbox.and_then(|t| t.last_of(&self.key())).and_then(|n| tools.iter().find(|s| s.name() == Some(n.as_str()) || s.text("does") == Some(n.as_str()))))
            .or_else(|| tools.first())
    }

    fn open_flyout(&mut self, cx: &mut Cx) {
        if self.tools().len() < 2 {
            return;
        }
        if let Some(open) = self.flyout.take() {
            cx.close(open);
        }
        let b = self.base.node.bounds;
        let width = PAD * 2.0 + self.tools().len() as f64 * (CELL + GAP) - GAP;
        let whole = cx.whole();
        let x = if b.right() + 4.0 + width <= whole.right() { b.right() + 4.0 } else { (b.x - 4.0 - width).max(whole.x) };
        let place = Rect::new(x, b.y - PAD, width, CELL + PAD * 2.0);
        let flyout = ToolFlyout { node: Node::new(), group: self.key(), tools: self.tools().to_vec(), host: self.base.host.clone(), toolbox: self.toolbox.clone(), place, buttons: Vec::new() };
        let mut overlay = Overlay::new(Box::new(flyout), place);
        overlay.through = true;
        self.flyout = Some(cx.open(overlay));
    }
}

impl Element for ToolGroupPart {
    part_element!();

    fn cursor_at(&self, _x: f64, _y: f64) -> Option<Cursor> {
        Some(Cursor::Hand)
    }

    fn tooltip_at(&self, _x: f64, _y: f64) -> Option<String> {
        let paints = self.toolbox.as_ref().is_none_or(|t| t.paints());
        let word = self.shown().map(|s| look_of(s, self.base.host.as_ref(), paints).word).unwrap_or_default();
        Some(if self.tools().len() > 1 { format!("{word} ({})", locale::translate("toolboxHold", "hold for more")) } else { word })
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        if at.button == PointerButton::Right {
            self.open_flyout(cx);
            return;
        }
        self.holding = true;
        cx.wake(self.base.node.id(), cx.now() + HOLD);
    }

    fn on_pointer_released(&mut self, _at: PointerInput, cx: &mut Cx) {
        if std::mem::take(&mut self.holding) {
            if let Some(spec) = self.shown().cloned() {
                use_tool(&spec, self.base.host.as_ref(), self.toolbox.as_ref());
            }
            cx.invalidate();
        }
    }

    /// Held long enough: its other tools show.
    fn on_wake(&mut self, cx: &mut Cx) {
        if std::mem::take(&mut self.holding) {
            self.open_flyout(cx);
            cx.invalidate();
        }
    }

    fn on_detached(&mut self, cx: &mut Cx) {
        if let Some(open) = self.flyout.take() {
            cx.close(open);
        }
    }

    fn draw(&self, p: &Painter) {
        let paints = self.toolbox.as_ref().is_none_or(|t| t.paints());
        let Some(spec) = self.shown() else { return };
        let look = look_of(spec, self.base.host.as_ref(), paints);
        let in_hand = self.toolbox.as_ref().is_some_and(|t| look.does.as_deref() == Some(t.tool().as_str()));
        let b = self.base.node.bounds;
        draw_button(p, b, look.icon, &look.word, in_hand);
        // A group with more tools has a small corner mark, as GIMP's.
        if self.tools().len() > 1 {
            let side = b.width.min(b.height);
            let (rx, by) = (b.x + side - 3.0, b.y + side - 3.0);
            let corner = [skia_safe::Point::new(rx as f32, (by - 6.0) as f32), skia_safe::Point::new(rx as f32, by as f32), skia_safe::Point::new((rx - 6.0) as f32, by as f32)];
            let path = skia_safe::Path::polygon(&corner, true, None, None);
            let mut mark = skia_safe::Paint::default();
            mark.set_color(p.look.color("muted"));
            mark.set_anti_alias(true);
            p.canvas.draw_path(&path, &mark);
        }
    }
}

impl Part for ToolGroupPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

/// A group's other tools, beside it: a click takes one, and the flyout goes.
pub struct ToolFlyout {
    node: Node,
    group: String,
    tools: Vec<PartSpec>,
    host: Rc<dyn PartHost>,
    toolbox: Option<Rc<Toolbox>>,
    place: Rect,
    buttons: Vec<(usize, Rect)>,
}

impl ToolFlyout {
    /// Each of its tools, by what it does (or its name), with where it lies.
    pub fn tools(&self) -> Vec<(String, Rect)> {
        self.buttons.iter().map(|(i, r)| (self.tools[*i].text("does").or(self.tools[*i].name()).unwrap_or("").to_string(), *r)).collect()
    }
}

impl Element for ToolFlyout {
    crate::element_node!();

    fn arrange(&mut self, _area: Rect, _cx: &mut Cx) {
        self.node.bounds = self.place;
        self.buttons = (0..self.tools.len()).map(|i| (i, Rect::new(self.place.x + PAD + i as f64 * (CELL + GAP), self.place.y + PAD, CELL, CELL))).collect();
    }

    fn cursor_at(&self, x: f64, y: f64) -> Option<Cursor> {
        self.buttons.iter().any(|(_, r)| r.contains(x, y)).then_some(Cursor::Hand)
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        let paints = self.toolbox.as_ref().is_none_or(|t| t.paints());
        self.buttons.iter().find(|(_, r)| r.contains(x, y)).map(|(i, _)| look_of(&self.tools[*i], self.host.as_ref(), paints).word)
    }

    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        let Some(i) = self.buttons.iter().find(|(_, r)| r.contains(at.screen_x, at.screen_y)).map(|(i, _)| *i) else { return };
        let spec = self.tools[i].clone();
        if let Some(toolbox) = self.toolbox.as_ref() {
            toolbox.remember(&self.group, spec.text("does").or(spec.name()).unwrap_or(""));
        }
        use_tool(&spec, self.host.as_ref(), self.toolbox.as_ref());
        cx.close(self.node.id());
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        p.fill(Rect::new(b.x + 2.0, b.y + 2.0, b.width, b.height), Color::from_argb(20, 0, 0, 0), 6.0);
        p.fill(b, look.color("panel"), 6.0);
        p.stroke(b, look.color("strong_border"), look.border(), 6.0);
        let paints = self.toolbox.as_ref().is_none_or(|t| t.paints());
        let in_hand = self.toolbox.as_ref().map(|t| t.tool()).unwrap_or_default();
        for (i, place) in &self.buttons {
            let tool = look_of(&self.tools[*i], self.host.as_ref(), paints);
            let chosen = tool.does.as_deref() == Some(in_hand.as_str());
            draw_button(p, *place, tool.icon, &tool.word, chosen);
        }
    }
}

/// The two small buttons of a value: less at the left of the value, more at its right.
fn value_parts(b: Rect) -> (Rect, Rect, Rect, Rect) {
    let small = (b.height - 6.0).clamp(12.0, 22.0);
    let y = b.y + (b.height - small) / 2.0;
    let more = Rect::new(b.right() - small, y, small, small);
    let value = Rect::new(more.x - 40.0, b.y, 40.0, b.height);
    let less = Rect::new(value.x - small, y, small, small);
    let label = Rect::new(b.x, b.y, (less.x - b.x - 4.0).max(0.0), b.height);
    (label, less, value, more)
}

fn draw_value(p: &Painter, b: Rect, label: &str, value: &str) {
    let look = p.look;
    let size = look.size("small");
    let (label_place, less, value_place, more) = value_parts(b);
    p.text(label, label_place, look.color("muted"), size, "text", false, TextAlign::Left);
    for (place, sign) in [(less, "-"), (more, "+")] {
        p.fill(place, look.color("raised"), 3.0);
        p.stroke(place, look.color("border"), 1.0, 3.0);
        p.text(sign, place, look.color("text"), size, "text", true, TextAlign::Center);
    }
    p.text(value, value_place, look.color("text"), size, "text", false, TextAlign::Center);
}

/// `toolsize` or `toolopacity`: the brush's size or the paint's opacity, less and more.
pub struct ToolValuePart {
    base: PartBase,
    toolbox: Option<Rc<Toolbox>>,
    opacity: bool,
}

pub fn make_size(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(ToolValuePart { base: PartBase::new(spec, host), toolbox: None, opacity: false })
}

pub fn make_opacity(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(ToolValuePart { base: PartBase::new(spec, host), toolbox: None, opacity: true })
}

impl ToolValuePart {
    pub fn link(&mut self, toolbox: Rc<Toolbox>) {
        self.toolbox = Some(toolbox);
    }
}

impl Element for ToolValuePart {
    part_element!();

    fn cursor_at(&self, x: f64, y: f64) -> Option<Cursor> {
        let (_, less, _, more) = value_parts(self.base.node.bounds);
        (less.contains(x, y) || more.contains(x, y)).then_some(Cursor::Hand)
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        let Some(toolbox) = self.toolbox.as_ref() else { return };
        let (_, less, _, more) = value_parts(self.base.node.bounds);
        let (x, y) = (at.screen_x, at.screen_y);
        let up = if more.contains(x, y) { true } else if less.contains(x, y) { false } else { return };
        if self.opacity {
            toolbox.set_opacity(toolbox.opacity() + if up { 10.0 } else { -10.0 });
        } else {
            let now = toolbox.brush();
            let step = if now < 10 || (now == 10 && !up) { 1 } else { 5 };
            toolbox.set_brush(if up { now + step } else { now - step });
        }
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        let (label, value) = match (self.opacity, self.toolbox.as_ref()) {
            (true, t) => (locale::translate("toolOpacity", "Opacity"), format!("{}%", t.map_or(100.0, |t| t.opacity()).round())),
            (false, t) => (locale::translate("toolSize", "Size"), t.map_or(2, |t| t.brush()).to_string()),
        };
        draw_value(p, self.base.node.bounds, &label, &value);
    }
}

impl Part for ToolValuePart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

/// `toolcolors`: the foreground colour in front, the background behind it, a click on one opens the palette, the arrow swaps them.
pub struct ToolColorsPart {
    base: PartBase,
    toolbox: Option<Rc<Toolbox>>,
    picking: Option<(bool, ElementId, Rc<std::cell::RefCell<Option<String>>>)>,
}

pub fn make_colors(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(ToolColorsPart { base: PartBase::new(spec, host), toolbox: None, picking: None })
}

impl ToolColorsPart {
    pub fn link(&mut self, toolbox: Rc<Toolbox>) {
        self.toolbox = Some(toolbox);
    }

    /// The foreground swatch, the background one behind it, and the swap arrow.
    fn places(&self) -> (Rect, Rect, Rect) {
        let b = self.base.node.bounds;
        let side = (b.height * 0.7).clamp(14.0, 26.0);
        let front = Rect::new(b.x, b.y, side, side);
        let back = Rect::new(b.x + side * 0.5, b.y + side * 0.5, side, side);
        let swap = Rect::new(b.x + side * 1.7, b.y, side * 0.8, side * 0.8);
        (front, back, swap)
    }
}

impl Element for ToolColorsPart {
    part_element!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        // A colour picked in the palette becomes the foreground or the background.
        let picked = self.picking.as_ref().and_then(|(fore, overlay, picked)| picked.borrow_mut().take().map(|c| (*fore, *overlay, c)));
        if let (Some((fore, overlay, color)), Some(toolbox)) = (picked, self.toolbox.as_ref()) {
            if fore { toolbox.set_ink(color) } else { toolbox.set_background(color) }
            cx.close(overlay);
            self.picking = None;
        }
    }

    fn cursor_at(&self, _x: f64, _y: f64) -> Option<Cursor> {
        Some(Cursor::Hand)
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        let (front, back, swap) = self.places();
        if front.contains(x, y) {
            Some(locale::translate("foregroundColor", "Foreground colour"))
        } else if back.contains(x, y) {
            Some(locale::translate("backgroundColor", "Background colour"))
        } else {
            swap.contains(x, y).then(|| locale::translate("swapColors", "Swap the colours"))
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        let Some(toolbox) = self.toolbox.clone() else { return };
        let (front, back, swap) = self.places();
        let (x, y) = (at.screen_x, at.screen_y);
        if swap.contains(x, y) {
            let (ink, background) = (toolbox.ink(), toolbox.background());
            toolbox.set_background(ink);
            toolbox.set_ink(background);
        } else if front.contains(x, y) || back.contains(x, y) {
            let fore = front.contains(x, y);
            let chosen = if fore { toolbox.ink() } else { toolbox.background() };
            let picked = Rc::new(std::cell::RefCell::new(None));
            let grid = PaletteGrid::new(Some(chosen), picked.clone(), None);
            let (w, h) = (PaletteGrid::width(), PaletteGrid::height());
            let whole = cx.whole();
            let b = self.base.node.bounds;
            let px = (b.x).min(whole.right() - w).max(whole.x);
            let py = if b.bottom() + 4.0 + h <= whole.bottom() { b.bottom() + 4.0 } else { (b.y - 4.0 - h).max(whole.y) };
            if let Some((_, old, _)) = self.picking.take() {
                cx.close(old);
            }
            self.picking = Some((fore, cx.open(Overlay::new(Box::new(grid), Rect::new(px, py, w, h))), picked));
        }
        cx.invalidate();
    }

    fn on_detached(&mut self, cx: &mut Cx) {
        if let Some((_, overlay, _)) = self.picking.take() {
            cx.close(overlay);
        }
    }

    fn draw(&self, p: &Painter) {
        let (front, back, swap) = self.places();
        let (ink, background) = self.toolbox.as_ref().map_or(("#000000".to_string(), "#ffffff".to_string()), |t| (t.ink(), t.background()));
        for (place, color) in [(back, background), (front, ink)] {
            p.fill(place, crate::parts::data::canvas::color_of(p.look, &color), 3.0);
            p.stroke(place, p.look.color("strong_border"), 1.0, 3.0);
        }
        let swap_icon = self.base.host.icon_path("arrow-left-right");
        crate::parts::containers::icon::draw_icon(p, swap_icon.as_deref(), swap.inset(3.0), p.look.color("muted"));
    }
}

impl Part for ToolColorsPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

/// Gives every toolbox part among `elements` (and inside them) the toolbox it works through.
pub fn link_all(element: &mut dyn Element, toolbox: &Rc<Toolbox>) {
    let any = element.as_any_mut();
    if let Some(part) = any.downcast_mut::<ToolPart>() {
        part.link(toolbox.clone());
    } else if let Some(part) = any.downcast_mut::<ToolGroupPart>() {
        part.link(toolbox.clone());
    } else if let Some(part) = any.downcast_mut::<ToolValuePart>() {
        part.link(toolbox.clone());
    } else if let Some(part) = any.downcast_mut::<ToolColorsPart>() {
        part.link(toolbox.clone());
    } else if let Some(part) = any.downcast_mut::<ToolLayersPart>() {
        part.link(toolbox.clone());
    }
    for child in element.node_mut().children_mut() {
        link_all(child.as_mut(), toolbox);
    }
}

/// `toollayers`: the canvas's layers, top first, each shown or hidden with its eye and taken in hand
/// with a click, with a row of buttons to add a layer, remove the one in hand, and move it up or down.
pub struct ToolLayersPart {
    base: PartBase,
    toolbox: Option<Rc<Toolbox>>,
}

pub fn make_layers(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(ToolLayersPart { base: PartBase::new(spec, host), toolbox: None })
}

const LAYER_ROW: f64 = 24.0;
const LAYER_BUTTONS: [(&str, &str, &str); 4] = [("plus", "addLayer", "Add a layer"), ("trash", "removeLayer", "Remove the layer"), ("arrow-up", "layerUp", "Move the layer up"), ("arrow-down", "layerDown", "Move the layer down")];

impl ToolLayersPart {
    pub fn link(&mut self, toolbox: Rc<Toolbox>) {
        self.toolbox = Some(toolbox);
    }

    /// The rows shown, top first: the picture alone is one layer until the person adds another.
    fn rows(&self) -> Vec<(String, bool, bool)> {
        let rows = self.toolbox.as_ref().map(|t| t.layer_rows()).unwrap_or_default();
        if rows.is_empty() { vec![(locale::translate("layerName", "Layer {number}").replace("{number}", "1"), true, true)] } else { rows }
    }

    /// Each row's box and its eye's, top first; then the buttons' boxes.
    fn places(&self) -> (Vec<(Rect, Rect)>, Vec<Rect>) {
        let b = self.base.node.bounds;
        let rows = self.rows();
        let buttons_y = b.bottom() - 24.0;
        let room = ((buttons_y - b.y - 4.0) / LAYER_ROW).floor().max(0.0) as usize;
        let shown: Vec<(Rect, Rect)> = (0..rows.len().min(room)).map(|i| {
            let row = Rect::new(b.x, b.y + i as f64 * LAYER_ROW, b.width, LAYER_ROW - 2.0);
            (row, Rect::new(row.x + 2.0, row.y + 2.0, 18.0, 18.0))
        }).collect();
        let buttons = (0..LAYER_BUTTONS.len()).map(|i| Rect::new(b.x + i as f64 * 26.0, buttons_y, 22.0, 22.0)).collect();
        (shown, buttons)
    }
}

impl Element for ToolLayersPart {
    part_element!();

    fn cursor_at(&self, _x: f64, _y: f64) -> Option<Cursor> {
        Some(Cursor::Hand)
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        let (rows, buttons) = self.places();
        if let Some(i) = buttons.iter().position(|r| r.contains(x, y)) {
            let (_, key, english) = LAYER_BUTTONS[i];
            return Some(locale::translate(key, english));
        }
        rows.iter().any(|(_, eye)| eye.contains(x, y)).then(|| locale::translate("showLayer", "Show or hide the layer"))
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        use crate::parts::data::canvas::layers::LayerAsk;
        let Some(toolbox) = self.toolbox.clone() else { return };
        let (x, y) = (at.screen_x, at.screen_y);
        let (rows, buttons) = self.places();
        let count = self.rows().len();
        if let Some(i) = buttons.iter().position(|r| r.contains(x, y)) {
            toolbox.ask_layers([LayerAsk::Add, LayerAsk::Remove, LayerAsk::Up, LayerAsk::Down][i]);
        } else if let Some((top, (_, eye))) = rows.iter().enumerate().find(|(_, (row, _))| row.contains(x, y)) {
            // Rows are top first; the layers count from the bottom.
            let place = count - 1 - top;
            toolbox.ask_layers(if eye.contains(x, y) { LayerAsk::Toggle(place) } else { LayerAsk::Choose(place) });
        }
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let size = look.size("small");
        let (places, buttons) = self.places();
        for ((name, shown, in_hand), (row, eye)) in self.rows().iter().zip(places) {
            if *in_hand {
                p.fill(row, look.color("accent_soft"), 3.0);
            }
            let icon = self.base.host.icon_path(if *shown { "eye" } else { "eye-off" });
            crate::parts::containers::icon::draw_icon(p, icon.as_deref(), eye.inset(2.0), look.color("muted"));
            p.text(name, Rect::new(eye.right() + 6.0, row.y, row.width - eye.width - 10.0, row.height), look.color("text"), size, "text", *in_hand, TextAlign::Left);
        }
        for (place, (icon, _, _)) in buttons.iter().zip(LAYER_BUTTONS) {
            p.fill(*place, look.color("raised"), 3.0);
            let path = self.base.host.icon_path(icon);
            crate::parts::containers::icon::draw_icon(p, path.as_deref(), place.inset(4.0), look.color("text"));
        }
    }
}

impl Part for ToolLayersPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
