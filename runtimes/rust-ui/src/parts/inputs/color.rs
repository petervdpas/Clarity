//! The color part, the same as on every stack (conformance/colors.json): a
//! swatch that opens the palette every renderer shares over the window, and
//! the color as "#rrggbb" to type (at most 7 characters). Typed text is
//! taken when the input is left or Enter is pressed (Enter then submits the
//! form); text that is no color stays as typed and is marked, and the value
//! stays. The palette works by keyboard too: the arrows and Tab move through
//! its colors, Enter or Space picks one; each color says its "#rrggbb" when
//! the pointer rests on it. Only a color that differs is a change; what code
//! sets is never one. A bound part writes its field; an unbound named one
//! keeps its color in its state's text.
//!
//! The part takes the focus for its text editor, as a text box does; the
//! swatch is a child of its own, a Tab stop that opens the palette with a
//! click, Enter, Space or Down and then hands the palette its keys.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use serde_json::Value;
use skia_safe::{Color, Paint, PaintStyle};

use crate::element::{Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::markup::PartSpec;
use crate::overlay::Overlay;
use crate::painter::Painter;
use crate::part_element;
use crate::parts::inputs::file::Facts;
use crate::parts::text_editor::TextEditor;
use crate::parts::{Part, PartBase, PartHost};
use crate::theme::parse_color;

/// The room between the swatch and the input.
const GAP: f64 = 4.0;

/// How wide the swatch is.
pub const SWATCH_WIDTH: f64 = 32.0;

/// How many characters the box takes: "#rrggbb".
pub const MAX_LENGTH: usize = 7;

/// The palette every renderer offers, in rows (conformance/colors.json).
pub const PALETTE: [[&str; 10]; 6] = [
    ["#ffffff", "#f8fafc", "#e5e7eb", "#d1d5db", "#9ca3af", "#6b7280", "#4b5563", "#374151", "#1f2937", "#000000"],
    ["#e2e8f0", "#fecaca", "#fed7aa", "#fde68a", "#bbf7d0", "#99f6e4", "#bfdbfe", "#c7d2fe", "#e9d5ff", "#fbcfe8"],
    ["#94a3b8", "#f87171", "#fb923c", "#fbbf24", "#4ade80", "#2dd4bf", "#60a5fa", "#818cf8", "#c084fc", "#f472b6"],
    ["#64748b", "#ef4444", "#f97316", "#f59e0b", "#22c55e", "#14b8a6", "#3b82f6", "#6366f1", "#a855f7", "#ec4899"],
    ["#475569", "#dc2626", "#ea580c", "#d97706", "#16a34a", "#0d9488", "#2563eb", "#4f46e5", "#9333ea", "#db2777"],
    ["#1e293b", "#991b1b", "#9a3412", "#92400e", "#166534", "#115e59", "#1e40af", "#3730a3", "#6b21a8", "#9d174d"],
];

/// The color a typed text is: with or without '#', three or six hexadecimal
/// digits in any case, around spaces, written back as '#' and six lowercase
/// digits. Empty text is no color (""); anything else is none at all.
pub fn read_color(typed: &str) -> Option<String> {
    let text = typed.trim();
    if text.is_empty() {
        return Some(String::new());
    }
    let hex = text.strip_prefix('#').unwrap_or(text);
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let hex = hex.to_ascii_lowercase();
    match hex.len() {
        6 => Some(format!("#{hex}")),
        3 => Some(hex.chars().fold(String::from("#"), |mut out, c| {
            out.push(c);
            out.push(c);
            out
        })),
        _ => None,
    }
}

/// The color to draw of "#rrggbb"; none for "" or text that is no color.
pub fn color_of(text: &str) -> Option<Color> {
    read_color(text).filter(|c| c.len() == 7).and_then(|c| parse_color(&c))
}

/// What the palette shares with the swatch that holds it: the color the
/// keys marked, and whether the swatch has the focus (the marked color has
/// the focus ring then).
#[derive(Default)]
pub struct Marks {
    pub marked: Cell<usize>,
    pub holder_focused: Cell<bool>,
}

/// What the part and its swatch share.
struct Shared {
    facts: Facts,
    /// What an unbound part without a name keeps.
    own: RefCell<String>,
    /// The palette's overlay, while it is open.
    open: Cell<Option<ElementId>>,
    /// The color picked in the palette, which it hands over as it closes.
    picked: Rc<RefCell<Option<String>>>,
    marks: Rc<Marks>,
    /// Where the part and its swatch are, on the screen.
    bounds: Cell<Rect>,
    swatch: Cell<Rect>,
    swatch_id: Cell<ElementId>,
}

impl Shared {
    /// The color it has, "#rrggbb", or "" for none.
    fn value(&self) -> String {
        match self.facts.bound() {
            Some(path) => match self.facts.host.read(path) {
                Some(Value::String(s)) => s,
                _ => String::new(),
            },
            None => self.facts.host.state(self.facts.spec.name()).and_then(|s| s.text).unwrap_or_else(|| self.own.borrow().clone()),
        }
    }

    fn usable(&self) -> bool {
        self.facts.enabled() && !self.facts.read_only()
    }

    /// Opens the palette below the swatch (above it when there is no room below), or closes it when it is open.
    fn toggle(&self, cx: &mut Cx) {
        if let Some(open) = self.open.take() {
            cx.close(open);
            return;
        }
        if !self.usable() {
            return;
        }
        let whole = cx.whole();
        let bounds = self.bounds.get();
        let (width, height) = (PaletteGrid::width(), PaletteGrid::height());
        let x = self.swatch.get().x.min(whole.width - width).max(0.0);
        let below = bounds.bottom() + 4.0;
        let y = if below + height > whole.height && bounds.y - 4.0 - height >= 0.0 { bounds.y - 4.0 - height } else { below };
        *self.picked.borrow_mut() = None;
        let grid = PaletteGrid::held(Some(self.value()), self.picked.clone(), self.swatch_id.get(), self.marks.clone());
        let mut overlay = Overlay::new(Box::new(grid), Rect::new(x, y, width, height));
        overlay.anchor = Some(self.swatch_id.get());
        overlay.opener = Some(self.facts.part);
        self.open.set(Some(cx.open(overlay)));
        cx.invalidate();
    }

    /// A key from the swatch while the palette is open: the arrows a color or
    /// a row on, Tab the next color, Home and End the ends, Enter or Space
    /// picks and closes it. False for any other key (Escape closes it, picking nothing).
    fn palette_key(&self, key: KeyInput, cx: &mut Cx) -> bool {
        let Some(open) = self.open.get() else { return false };
        let count = PaletteGrid::colors().len() as i64;
        let marked = self.marks.marked.get() as i64;
        let across = ACROSS as i64;
        let to = match key.key {
            Key::Left => Some(marked - 1),
            Key::Right => Some(marked + 1),
            Key::Up => Some(marked - across),
            Key::Down => Some(marked + across),
            Key::Tab => Some(marked + if key.shift() { -1 } else { 1 }),
            Key::Home => Some(0),
            Key::End => Some(count - 1),
            _ => None,
        };
        if let Some(next) = to {
            // Tab goes round; the arrows stop at the edges.
            let next = if key.key == Key::Tab { (next + count) % count } else { next.clamp(0, count - 1) };
            self.marks.marked.set(next as usize);
            cx.invalidate();
            return true;
        }
        if matches!(key.key, Key::Enter | Key::Space) {
            *self.picked.borrow_mut() = Some(PaletteGrid::colors()[marked as usize].to_string());
            cx.close(open);
            return true;
        }
        false
    }
}

pub struct ColorPart {
    base: PartBase,
    editor: TextEditor,
    shared: Rc<Shared>,
    /// The value as it was last shown; none before the first.
    shown: Option<String>,
    bad: bool,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(ColorPart::new(spec, host))
}

impl ColorPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> ColorPart {
        let mut base = PartBase::new(spec, host);
        let mut editor = TextEditor::new(false);
        editor.placeholder = base.spec.text("placeholder").map(|p| base.host.text(Some(p)));
        editor.max_length = Some(MAX_LENGTH);
        editor.undoes = !base.spec.flag("noUndo");
        let shared = Rc::new(Shared {
            facts: Facts { host: base.host.clone(), spec: base.spec.clone(), part: base.node.id() },
            own: RefCell::new(String::new()),
            open: Cell::new(None),
            picked: Rc::new(RefCell::new(None)),
            marks: Rc::new(Marks::default()),
            bounds: Cell::new(Rect::default()),
            swatch: Cell::new(Rect::default()),
            swatch_id: Cell::new(0),
        });
        let swatch = ColorSwatch { node: Node::new(), shared: shared.clone() };
        shared.swatch_id.set(swatch.node.id());
        base.node.add(Box::new(swatch));
        let mut part = ColorPart { base, editor, shared, shown: None, bad: false };
        part.sync();
        part
    }

    /// Where the person types the color.
    pub fn editor(&self) -> &TextEditor {
        &self.editor
    }

    pub fn editor_mut(&mut self) -> &mut TextEditor {
        &mut self.editor
    }

    /// Where the swatch is, on the screen: it shows the color and opens the palette.
    pub fn swatch(&self) -> Rect {
        self.shared.swatch.get()
    }

    /// The swatch's id: a Tab stop of its own.
    pub fn swatch_id(&self) -> ElementId {
        self.shared.swatch_id.get()
    }

    /// The palette's overlay, while it is open.
    pub fn palette(&self) -> Option<ElementId> {
        self.shared.open.get()
    }

    /// The color of the palette the keys marked, while it is open.
    pub fn marked(&self) -> Option<usize> {
        self.shared.open.get().map(|_| self.shared.marks.marked.get())
    }

    /// How many characters its box takes.
    pub fn max_length(&self) -> Option<usize> {
        self.editor.max_length
    }

    /// Whether what is typed is no color: the input is marked, the value stays.
    pub fn is_bad(&self) -> bool {
        self.bad
    }

    /// The color it has, "#rrggbb", or "" for none.
    pub fn value(&self) -> String {
        self.shared.value()
    }

    /// A value set from elsewhere shows as it is, and what was marked is no longer.
    pub fn sync(&mut self) {
        let now = self.value();
        if self.shown.as_deref() == Some(now.as_str()) {
            return;
        }
        self.bad = false;
        self.editor.set_text(&now);
        self.shown = Some(now);
    }

    /// Takes what is typed: a change when it is a color that differs from the value; text that is no color is marked.
    pub fn take(&mut self) {
        self.sync();
        match read_color(&self.editor.text()) {
            Some(color) => self.offer(color),
            None => self.bad = true,
        }
    }

    /// The person picked `color` from the palette: a change when it differs.
    pub fn pick(&mut self, color: &str) {
        self.offer(color.to_string());
    }

    fn offer(&mut self, color: String) {
        self.bad = false;
        if !self.shared.usable() || color == self.value() {
            self.shown = None;
            self.sync();
            return;
        }
        let name = self.base.name().map(str::to_string);
        if let Some(path) = self.base.bound().map(str::to_string) {
            self.base.host.write(name.as_deref().unwrap_or(""), &path, Value::String(color));
        } else if let Some(name) = name {
            crate::parts::change_state(&*self.base.host, &name, |s| s.text = Some(color.clone()));
            self.base.host.edited(&name);
        } else {
            *self.shared.own.borrow_mut() = color;
        }
        self.sync();
        self.base.fire("change");
    }

    /// Opens the palette below the swatch, or closes it when it is open.
    pub fn toggle(&mut self, cx: &mut Cx) {
        self.shared.toggle(cx);
    }

    /// Makes the editor ready for what the person does: their state and the part's look.
    fn ready(&mut self) {
        let state = self.base.state();
        self.editor.enabled = self.base.enabled();
        self.editor.read_only = self.base.read_only();
        self.editor.marked = self.bad || state.and_then(|s| s.error).is_some_and(|e| !e.show().is_empty());
        self.editor.mirror(&self.base.node);
    }
}

impl Element for ColorPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    fn typing(&self) -> bool {
        true
    }

    fn take_typed(&mut self, _cx: &mut Cx) -> bool {
        self.take();
        true
    }

    /// The swatch at the left, as wide as it is set, the input beside it.
    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.ready();
        self.sync();
        let side = SWATCH_WIDTH.min(area.width);
        let swatch = Rect::new(area.x, area.y, side, area.height);
        self.shared.bounds.set(area);
        self.shared.swatch.set(swatch);
        self.base.node.children_mut()[0].arrange(swatch, cx);
        self.editor.place(Rect::new(area.x + side + GAP, area.y, (area.width - side - GAP).max(0.0), area.height), cx);
    }

    fn draw(&self, p: &Painter) {
        let _dimmed = p.dimmed(self.base.enabled());
        crate::element::draw_children(&self.base.node, p);
        self.editor.paint(p);
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.ready();
        self.editor.pressed(at, cx);
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.ready();
        self.editor.dragged(at, cx);
    }

    fn on_pointer_released(&mut self, _at: PointerInput, _cx: &mut Cx) {
        self.editor.released();
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        self.editor.wheel(dy, cx)
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        self.editor.focus_moved(focused, cx);
        if focused {
            self.base.fire("enter");
        } else {
            self.take();
            self.base.fire("leave");
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.ready();
        // Enter takes the color; the form it is in, or the window's default button, does the rest.
        if key.key == Key::Enter && key.modifiers.is_empty() {
            self.take();
            cx.invalidate();
            return false;
        }
        self.editor.key(key, cx)
    }

    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        self.ready();
        self.editor.typed(text, cx);
    }

    fn cursor_at(&self, x: f64, y: f64) -> Option<crate::input::Cursor> {
        self.editor.cursor_at(x, y)
    }

    fn context_menu_at(&self, _x: f64, _y: f64) -> Option<Vec<crate::frame::menus::MenuEntry>> {
        self.editor.menu(self.base.node.id())
    }

    fn on_wake(&mut self, cx: &mut Cx) {
        self.ready();
        self.editor.run_menu_asked(cx);
    }

    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        if self.shared.open.get() == Some(overlay) {
            self.shared.open.set(None);
        }
        let picked = self.shared.picked.borrow_mut().take();
        if let Some(color) = picked {
            self.pick(&color);
        }
        cx.invalidate();
    }
}

impl Part for ColorPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

/// A color part's swatch: the color in a framed square, a line across it
/// for none; a click or Enter, Space or Down opens the palette, whose keys
/// it then hands on.
pub struct ColorSwatch {
    node: Node,
    shared: Rc<Shared>,
}

impl Element for ColorSwatch {
    crate::element_node!();

    fn focusable(&self) -> bool {
        self.shared.usable()
    }

    /// While the palette is open, Tab moves through its colors.
    fn wants_tab(&self) -> bool {
        self.shared.open.get().is_some()
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.shared.toggle(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if self.shared.open.get().is_some() {
            return self.shared.palette_key(key, cx);
        }
        if matches!(key.key, Key::Enter | Key::Space | Key::Down) && key.modifiers.is_empty() {
            self.shared.toggle(cx);
            return true;
        }
        false
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        self.shared.marks.holder_focused.set(focused);
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let radius = look.radius("input");
        let focused = self.node.focused;
        let values = look.part("input", if focused { Some("focus") } else { None });
        let b = self.node.bounds;
        p.fill(b, values.background.unwrap_or_else(|| look.color("input")), radius);
        let inner = b.inset(3.0);
        match color_of(&self.shared.value()) {
            Some(color) => p.fill(inner, color, (radius - 2.0).max(0.0)),
            None => {
                let mut pen = Paint::default();
                pen.set_color(look.color("muted"));
                pen.set_anti_alias(true);
                pen.set_style(PaintStyle::Stroke);
                pen.set_stroke_width(1.2);
                p.canvas.draw_line((inner.x as f32, inner.bottom() as f32), (inner.right() as f32, inner.y as f32), &pen);
            }
        }
        let hovered = self.node.hovered && self.focusable();
        let line = values.border.unwrap_or_else(|| look.color(if hovered { "strong_border" } else { "border" }));
        p.stroke(b, line, look.border(), radius);
        if focused {
            p.focus_ring(b, radius);
        }
    }
}

/// The palette every renderer shares, ten colors a row, over the window; a
/// press on a color picks it, and so do Enter and Space on the color the
/// keys marked (through the swatch that holds it); it closes, handing the
/// color to the part that opened it. Each color says its "#rrggbb" when the
/// pointer rests on it.
pub struct PaletteGrid {
    node: Node,
    chosen: Option<String>,
    picked: Rc<RefCell<Option<String>>>,
    holder: Option<ElementId>,
    marks: Rc<Marks>,
    pressed_at: Option<usize>,
    hovered_at: Option<usize>,
}

const CELL: f64 = 22.0;
const SPACE: f64 = 3.0;
const PAD: f64 = 6.0;
const ACROSS: usize = 10;

impl PaletteGrid {
    /// A palette marking `chosen`, which puts a picked color in `picked`;
    /// `holder` keeps the focus while the person picks.
    pub fn new(chosen: Option<String>, picked: Rc<RefCell<Option<String>>>, holder: Option<ElementId>) -> PaletteGrid {
        let marks = Rc::new(Marks::default());
        let mut grid = PaletteGrid { node: Node::new(), chosen: None, picked, holder, marks, pressed_at: None, hovered_at: None };
        grid.choose(chosen);
        grid
    }

    /// A palette held by a swatch, which hands it its keys through `marks`.
    fn held(chosen: Option<String>, picked: Rc<RefCell<Option<String>>>, holder: ElementId, marks: Rc<Marks>) -> PaletteGrid {
        let mut grid = PaletteGrid { node: Node::new(), chosen: None, picked, holder: Some(holder), marks, pressed_at: None, hovered_at: None };
        grid.choose(chosen);
        grid
    }

    /// Marks `chosen`: the keys start at it, else at the first color.
    fn choose(&mut self, chosen: Option<String>) {
        let at = chosen.as_deref().and_then(|c| Self::colors().iter().position(|x| *x == c)).unwrap_or(0);
        self.marks.marked.set(at);
        self.chosen = chosen;
    }

    /// How wide it is.
    pub fn width() -> f64 {
        2.0 * PAD + ACROSS as f64 * CELL + (ACROSS - 1) as f64 * SPACE
    }

    /// How high it is.
    pub fn height() -> f64 {
        let rows = Self::colors().len().div_ceil(ACROSS) as f64;
        2.0 * PAD + rows * CELL + (rows - 1.0) * SPACE
    }

    /// The colors it offers, in order: row by row.
    pub fn colors() -> Vec<&'static str> {
        PALETTE.iter().flatten().copied().collect()
    }

    /// The color it marks as the one the part has.
    pub fn chosen(&self) -> Option<&str> {
        self.chosen.as_deref()
    }

    /// The color the keys move and Enter picks: at first the one the part has, else the first.
    pub fn marked(&self) -> usize {
        self.marks.marked.get()
    }

    /// Where `color` lies on the screen; none for a color it does not offer.
    pub fn place(&self, color: &str) -> Option<Rect> {
        Self::colors().iter().position(|c| *c == color).map(|at| self.place_of(at))
    }

    fn place_of(&self, at: usize) -> Rect {
        let b = self.node.bounds;
        Rect::new(b.x + PAD + (at % ACROSS) as f64 * (CELL + SPACE), b.y + PAD + (at / ACROSS) as f64 * (CELL + SPACE), CELL, CELL)
    }

    fn color_at(&self, x: f64, y: f64) -> Option<usize> {
        (0..Self::colors().len()).find(|i| self.place_of(*i).contains(x, y))
    }
}

impl Element for PaletteGrid {
    crate::element_node!();

    /// A color says its "#rrggbb" when the pointer rests on it.
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        self.color_at(x, y).map(|at| Self::colors()[at].to_string())
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let over = self.color_at(at.screen_x, at.screen_y);
        if over != self.hovered_at {
            self.hovered_at = over;
            cx.invalidate();
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pressed_at = self.color_at(at.screen_x, at.screen_y);
        if self.holder.is_some() {
            cx.focus(self.holder);
        }
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if let Some(at) = self.pressed_at.take() {
            *self.picked.borrow_mut() = Some(Self::colors()[at].to_string());
            cx.close(self.node.id());
        }
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let radius = look.radius("input");
        let b = self.node.bounds;
        p.fill(b, look.color("panel"), radius);
        p.stroke(b, look.color("border"), look.border(), radius);
        let marked = self.marks.marked.get();
        for (i, color) in Self::colors().iter().enumerate() {
            let place = self.place_of(i);
            if let Some(c) = color_of(color) {
                p.fill(place, c, 3.0);
            }
            p.stroke(place, look.color(if self.hovered_at == Some(i) { "strong_border" } else { "border" }), 1.0, 3.0);
            if self.chosen.as_deref() == Some(*color) {
                p.stroke(Rect::new(place.x - 2.0, place.y - 2.0, place.width + 4.0, place.height + 4.0), look.color("accent"), 2.0, 4.0);
            }
            // The color the keys marked has the focus ring.
            if i == marked && self.marks.holder_focused.get() {
                p.focus_ring(place.inset(-1.0), 4.0);
            }
        }
    }
}
