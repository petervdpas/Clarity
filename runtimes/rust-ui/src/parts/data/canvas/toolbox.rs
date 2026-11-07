//! The toolbox of a canvas (`toolbox`, docs/IMAGE-PLAN.md): a panel holding
//! the parts of a toolbox page (`page Painting: toolbox`, parts/tools.rs) on
//! their grid, or the built-in toolbox of the canvas's mode. It floats over
//! the window (`toolbox popup`), dragged by its title and folded, or lies
//! against the canvas's edge (`toolbox slideout`). The tool, brush and ink
//! are the canvas's state, which code reads and sets too; opacity and the
//! background colour are the toolbox's.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use skia_safe::Color;

use super::modes::{PAINT_TOOLS, VECTOR_TOOLS};
use crate::element::{Cx, Element, ElementId, GridPanel, Node};
use crate::geometry::Rect;
use crate::input::{Cursor, PointerInput};
use crate::locale;
use crate::markup::PartSpec;
use crate::overlay::Overlay;
use crate::painter::{Painter, TextAlign};
use crate::parts::PartHost;

const PAD: f64 = 8.0;
const COLUMN: f64 = 36.0;
const GAP: f64 = 4.0;
const ROW: f64 = 32.0;
const HEADER: f64 = 26.0;

/// Each built-in tool's icon (clarity-spec's TOOLBOX_ICONS carries them); select is a dashed square on a picture, a pointer on a drawing.
pub fn icon_of(tool: &str, paints: bool) -> &'static str {
    match tool {
        "pen" => "paintbrush",
        "pencil" => "pencil",
        "spray" => "spray-can",
        "eraser" => "eraser",
        "fill" => "paint-bucket",
        "picker" => "pipette",
        "select" if paints => "square-dashed",
        "ellipseSelect" => "circle-dashed",
        "lasso" => "lasso",
        "select" => "mouse-pointer-2",
        "line" => "slash",
        "rect" => "square",
        "ellipse" => "circle",
        "path" => "spline",
        "text" => "type",
        "rotate" => "rotate-cw",
        "flipAcross" => "move-horizontal",
        "flipDown" => "move-vertical",
        "crop" => "crop",
        "import" => "image-up",
        "export" => "image-down",
        "zoomIn" => "zoom-in",
        "zoomOut" => "zoom-out",
        "adjust" => "sliders-horizontal",
        _ => "paintbrush",
    }
}

/// The built-in toolbox of a painting canvas, as a toolbox page would be written (4 columns).
const PAINT_TOOLBOX: &str = r#"<Window name="Toolbox">
  <ToolGroup name="selectGroup" at="1,1"><Tool does="select" /><Tool does="ellipseSelect" /><Tool does="lasso" /></ToolGroup>
  <ToolGroup name="drawGroup" at="2,1"><Tool does="pen" /><Tool does="pencil" /><Tool does="spray" /></ToolGroup>
  <Tool does="eraser" at="3,1" />
  <Tool does="fill" at="4,1" />
  <Tool does="picker" at="1,2" />
  <ToolGroup name="shapeGroup" at="2,2"><Tool does="line" /><Tool does="rect" /><Tool does="ellipse" /></ToolGroup>
  <Tool does="zoomIn" at="3,2" />
  <Tool does="zoomOut" at="4,2" />
  <Tool does="rotate" at="1,3" />
  <Tool does="flipAcross" at="2,3" />
  <Tool does="flipDown" at="3,3" />
  <Tool does="crop" at="4,3" />
  <ToolSize at="1,4" span="4" />
  <ToolOpacity at="1,5" span="4" />
  <ToolColors at="1,6" span="2" />
  <Tool does="adjust" at="3,6" />
  <Tool does="import" at="1,7" />
  <Tool does="export" at="2,7" />
  <ToolLayers at="1,8" span="4" rows="4" />
</Window>"#;

/// The built-in toolbox of a vector canvas.
const VECTOR_TOOLBOX: &str = r#"<Window name="Toolbox">
  <Tool does="select" at="1,1" />
  <Tool does="line" at="2,1" />
  <Tool does="rect" at="3,1" />
  <Tool does="ellipse" at="4,1" />
  <Tool does="path" at="1,2" />
  <Tool does="text" at="2,2" />
  <Tool does="zoomIn" at="3,2" />
  <Tool does="zoomOut" at="4,2" />
  <ToolSize at="1,3" span="4" />
  <ToolColors at="1,4" span="2" />
</Window>"#;

/// The built-in toolbox of a mode, without the canvas tools it does not have (`tools pen, eraser`).
pub fn built_in(paints: bool, tools: &[&str]) -> Vec<PartSpec> {
    let markup = if paints { PAINT_TOOLBOX } else { VECTOR_TOOLBOX };
    let parts = crate::markup::read(markup).map(|w| w.parts).unwrap_or_default();
    let canvas_tool = |spec: &PartSpec| spec.text("does").is_some_and(|d| PAINT_TOOLS.contains(&d) || VECTOR_TOOLS.contains(&d));
    let kept = |spec: &PartSpec| !canvas_tool(spec) || spec.text("does").is_some_and(|d| tools.contains(&d));
    parts
        .into_iter()
        .filter(|p| kept(p))
        .map(|mut p| {
            p.children.retain(kept);
            p
        })
        .filter(|p| p.kind != "toolGroup" || !p.children.is_empty())
        .collect()
}

/// How the toolbox shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shows {
    Popup,
    Slideout,
}

/// What a tool asks of its canvas, which holds the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// A picture file chosen by the person, laid on the paper.
    Import,
    /// The picture saved where the person chooses.
    Export,
    /// A turn, a flip or a crop, of the selection or the whole picture.
    Transform(super::raster::Transform),
    /// Closer (true) or farther.
    Zoom(bool),
    /// The colour dialog: hue, saturation, brightness, contrast.
    Adjust,
    /// A change of the layers.
    Layer(super::layers::LayerAsk),
}

/// What the canvas and its toolbox's parts share.
pub struct Toolbox {
    opacity: Cell<f64>,
    background: RefCell<String>,
    folded: Cell<bool>,
    /// Where the person dragged the popup, its top left on the screen; none: beside the canvas's top right.
    place: Cell<Option<(f64, f64)>>,
    /// The canvas on the screen, as last laid out.
    canvas: Cell<Rect>,
    shows: Cell<Shows>,
    /// The canvas's host and name, whose state holds the tool, brush and ink.
    link: RefCell<Option<(Rc<dyn PartHost>, String)>>,
    /// The overlay it lies in while it shows.
    opened: Cell<Option<ElementId>>,
    /// The canvas's own brush and ink, where its state names none (`brush 8 ink accent` on its line).
    brush_written: Cell<i64>,
    ink_written: RefCell<String>,
    /// What a tool asked of the canvas, taken once.
    asked: Cell<Option<Ask>>,
    /// A painting canvas's: opacity, import and export are for pictures, not for vector drawings.
    paints: Cell<bool>,
    /// The tool last taken in each group, by the group.
    last: RefCell<std::collections::HashMap<String, String>>,
    /// The canvas's layers as the layers tool shows them, top first: name, shown, in hand.
    layer_rows: RefCell<Vec<(String, bool, bool)>>,
}

impl Default for Toolbox {
    fn default() -> Toolbox {
        Toolbox {
            opacity: Cell::new(100.0),
            background: RefCell::new("#ffffff".to_string()),
            folded: Cell::new(false),
            place: Cell::new(None),
            canvas: Cell::new(Rect::default()),
            shows: Cell::new(Shows::Popup),
            link: RefCell::new(None),
            opened: Cell::new(None),
            brush_written: Cell::new(2),
            ink_written: RefCell::new("text".to_string()),
            asked: Cell::new(None),
            paints: Cell::new(true),
            last: RefCell::new(std::collections::HashMap::new()),
            layer_rows: RefCell::new(Vec::new()),
        }
    }
}

/// What the canvas tells its toolbox as it is laid out.
pub struct Following {
    pub shows: Option<Shows>,
    pub paints: bool,
    pub canvas: Rect,
    pub link: (Rc<dyn PartHost>, String),
    /// The parts of its toolbox page, or the built-in ones, with the columns they are placed on.
    pub parts: Vec<PartSpec>,
    pub columns: i64,
    /// The canvas's own brush and ink.
    pub written: (f64, String),
}

impl Toolbox {
    /// How much paint the brush, pencil, shapes and fill lay down, 0 to 100.
    pub fn opacity(&self) -> f64 {
        self.opacity.get()
    }

    pub fn set_opacity(&self, opacity: f64) {
        self.opacity.set(opacity.clamp(0.0, 100.0));
    }

    pub fn background(&self) -> String {
        self.background.borrow().clone()
    }

    pub fn set_background(&self, color: String) {
        *self.background.borrow_mut() = color;
    }

    pub fn folded(&self) -> bool {
        self.folded.get()
    }

    pub fn paints(&self) -> bool {
        self.paints.get()
    }

    /// The overlay it lies in while it shows; none while it does not.
    pub fn opened(&self) -> Option<ElementId> {
        self.opened.get()
    }

    /// The canvas laid out: the toolbox opens over the window while the canvas may be used, and goes with it.
    pub fn follow(self: &Rc<Self>, following: Following, cx: &mut Cx) {
        self.canvas.set(following.canvas);
        self.paints.set(following.paints);
        self.brush_written.set(following.written.0.round() as i64);
        *self.ink_written.borrow_mut() = following.written.1;
        let host = following.link.0.clone();
        *self.link.borrow_mut() = Some(following.link);
        match following.shows {
            Some(shows) if following.canvas.width > 0.0 => {
                self.shows.set(shows);
                if self.opened.get().is_none() {
                    let panel = ToolboxPanel::new(self.clone(), &following.parts, following.columns, &host);
                    let mut overlay = Overlay::new(Box::new(panel), Rect::default());
                    // It stays: a press beside it goes on to the canvas and the window.
                    overlay.stays = Some(Rc::new(|| true));
                    overlay.through = true;
                    let me = self.clone();
                    overlay.closed = Some(Box::new(move || me.opened.set(None)));
                    self.opened.set(Some(cx.open(overlay)));
                }
            }
            _ => self.close(cx),
        }
    }

    /// The canvas left the screen or may no longer be used: the toolbox goes.
    pub fn close(&self, cx: &mut Cx) {
        if let Some(open) = self.opened.take() {
            cx.close(open);
        }
    }

    fn state(&self) -> Option<crate::parts::PartState> {
        let link = self.link.borrow();
        let (host, name) = link.as_ref()?;
        host.state(Some(name))
    }

    fn change(&self, change: impl FnOnce(&mut crate::parts::PartState)) {
        if let Some((host, name)) = self.link.borrow().as_ref() {
            crate::parts::change_state(&**host, name, change);
        }
    }

    /// The canvas tool in hand, as its state names it.
    pub fn tool(&self) -> String {
        self.state().and_then(|s| s.tool).unwrap_or_default()
    }

    /// A tool's built-in action: a canvas tool taken in hand, else asked of the canvas.
    pub fn does(&self, action: &str) {
        let asked = match action {
            "rotate" => Some(Ask::Transform(super::raster::Transform::Rotate)),
            "flipAcross" => Some(Ask::Transform(super::raster::Transform::FlipAcross)),
            "flipDown" => Some(Ask::Transform(super::raster::Transform::FlipDown)),
            "crop" => Some(Ask::Transform(super::raster::Transform::Crop)),
            "import" => Some(Ask::Import),
            "export" => Some(Ask::Export),
            "zoomIn" => Some(Ask::Zoom(true)),
            "zoomOut" => Some(Ask::Zoom(false)),
            "adjust" => Some(Ask::Adjust),
            _ => None,
        };
        match asked {
            Some(asked) => self.asked.set(Some(asked)),
            None => {
                let tool = action.to_string();
                self.change(|s| s.tool = Some(tool));
            }
        }
    }

    /// The tool last taken in a group; none before one was.
    pub fn last_of(&self, group: &str) -> Option<String> {
        self.last.borrow().get(group).cloned()
    }

    pub fn remember(&self, group: &str, tool: &str) {
        self.last.borrow_mut().insert(group.to_string(), tool.to_string());
    }

    pub fn brush(&self) -> i64 {
        self.state().and_then(|s| s.brush).unwrap_or_else(|| self.brush_written.get())
    }

    pub fn set_brush(&self, brush: i64) {
        let brush = brush.clamp(1, 100);
        self.change(|s| s.brush = Some(brush));
    }

    pub fn ink(&self) -> String {
        self.state().and_then(|s| s.ink).filter(|i| !i.is_empty()).unwrap_or_else(|| self.ink_written.borrow().clone())
    }

    pub fn set_ink(&self, ink: String) {
        self.change(|s| s.ink = Some(ink));
    }

    /// The size and the foreground colour it shows: the canvas's state, else its own line's.
    pub fn shown(&self) -> (i64, String) {
        (self.brush(), self.ink())
    }

    /// The layers as the layers tool shows them, top first: name, shown, in hand; none while the picture is the only layer.
    pub fn layer_rows(&self) -> Vec<(String, bool, bool)> {
        self.layer_rows.borrow().clone()
    }

    pub fn set_layer_rows(&self, rows: Vec<(String, bool, bool)>) {
        *self.layer_rows.borrow_mut() = rows;
    }

    /// A change of the layers, asked of the canvas.
    pub fn ask_layers(&self, ask: super::layers::LayerAsk) {
        self.asked.set(Some(Ask::Layer(ask)));
    }

    /// What a tool asked of the canvas, taken once.
    pub fn take_ask(&self) -> Option<Ask> {
        self.asked.take()
    }

    fn icon_path(&self, name: &str) -> Option<String> {
        self.link.borrow().as_ref().and_then(|(host, _)| host.icon_path(name))
    }
}

/// The toolbox as it lies over the window: its title, and its parts on their grid.
pub struct ToolboxPanel {
    node: Node,
    toolbox: Rc<Toolbox>,
    columns: i64,
    rows: i64,
    /// How high its parts' grid is, as the grid lays its rows out.
    content: f64,
    /// A drag of its title: where the pointer was pressed and where the toolbox was.
    dragging: Option<((f64, f64), (f64, f64))>,
}

impl ToolboxPanel {
    fn new(toolbox: Rc<Toolbox>, parts: &[PartSpec], columns: i64, host: &Rc<dyn PartHost>) -> ToolboxPanel {
        let rows = parts.iter().map(|p| p.place(false)).map(|g| i64::from(g.row) + i64::from(g.height) - 1).max().unwrap_or(1).max(1);
        let columns = columns.max(1);
        let mut grid: GridPanel = crate::parts::containers::container::grid_of(parts, columns, GAP, host);
        crate::parts::tools::link_all(&mut grid, &toolbox);
        let mut node = Node::new();
        node.add(Box::new(grid));
        let content = rows as f64 * ROW;
        ToolboxPanel { node, toolbox, columns, rows, content, dragging: None }
    }

    fn width(&self) -> f64 {
        PAD * 2.0 + self.columns as f64 * COLUMN - GAP
    }

    fn height(&self) -> f64 {
        if self.toolbox.folded() { HEADER } else { HEADER + PAD / 2.0 + self.content + PAD }
    }

    fn place(&self, whole: Rect) -> Rect {
        let canvas = self.toolbox.canvas.get();
        let (width, height) = (self.width(), self.height());
        match self.toolbox.shows.get() {
            Shows::Slideout => {
                // Against the canvas's right edge, outside it where the window has room, as high as the canvas when unfolded.
                let x = if canvas.right() + width <= whole.right() { canvas.right() } else { (canvas.right() - width).max(whole.x) };
                let tall = if self.toolbox.folded() { HEADER } else { height.max(canvas.height) };
                Rect::new(x, canvas.y, width, tall)
            }
            Shows::Popup => {
                let (x, y) = self.toolbox.place.get().unwrap_or((canvas.right() - width - PAD, canvas.y + PAD));
                Rect::new(x.clamp(whole.x, (whole.right() - width).max(whole.x)), y.clamp(whole.y, (whole.bottom() - HEADER).max(whole.y)), width, height)
            }
        }
    }

    fn fold_place(&self) -> Rect {
        let b = self.node.bounds;
        Rect::new(b.right() - HEADER, b.y, HEADER, HEADER)
    }

    /// Where the part doing `tool` lies (a tool, or the group holding it), as last laid out.
    pub fn tool_place(&self, tool: &str) -> Option<Rect> {
        fn find(e: &dyn Element, tool: &str) -> Option<Rect> {
            let any = e.as_any();
            if let Some(t) = any.downcast_ref::<crate::parts::tools::ToolPart>() {
                if t.does() == Some(tool) {
                    return Some(t.node().bounds);
                }
            }
            if let Some(g) = any.downcast_ref::<crate::parts::tools::ToolGroupPart>() {
                if g.tools().iter().any(|s| s.text("does") == Some(tool)) {
                    return Some(g.node().bounds);
                }
            }
            e.node().children().iter().find_map(|c| find(c.as_ref(), tool))
        }
        find(self, tool)
    }
}

impl Element for ToolboxPanel {
    crate::element_node!();

    fn arrange(&mut self, _area: Rect, cx: &mut Cx) {
        // The grid's rows are as high as its parts need: the panel is as high as they are, so none is cut off.
        let inner = self.width() - PAD * 2.0;
        let wanted = self.rows as f64 * ROW;
        if let Some(grid) = self.node.children().first().and_then(|g| g.as_any().downcast_ref::<GridPanel>()) {
            self.content = grid.content_for(inner, wanted).max(wanted);
        }
        self.node.bounds = self.place(cx.whole());
        let b = self.node.bounds;
        let folded = self.toolbox.folded();
        let area = if folded { Rect::default() } else { Rect::new(b.x + PAD, b.y + HEADER + PAD / 2.0, inner, self.content) };
        if let Some(grid) = self.node.children_mut().first_mut() {
            grid.arrange(area, cx);
        }
    }

    fn cursor_at(&self, x: f64, y: f64) -> Option<Cursor> {
        self.fold_place().contains(x, y).then_some(Cursor::Hand)
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        self.fold_place().contains(x, y).then(|| locale::translate("toolboxFold", "Fold the toolbox"))
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        let (x, y) = (at.screen_x, at.screen_y);
        if self.fold_place().contains(x, y) {
            self.toolbox.folded.set(!self.toolbox.folded());
            cx.invalidate();
            return;
        }
        // The title drags a popup toolbox.
        let b = self.node.bounds;
        if self.toolbox.shows.get() == Shows::Popup && y < b.y + HEADER {
            self.dragging = Some(((x, y), (b.x, b.y)));
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        if let Some(((px, py), (bx, by))) = self.dragging {
            self.toolbox.place.set(Some((bx + at.screen_x - px, by + at.screen_y - py)));
            cx.invalidate();
        }
    }

    fn on_pointer_released(&mut self, _at: PointerInput, _cx: &mut Cx) {
        self.dragging = None;
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        // A light shadow under it, then its panel.
        for i in 1..=3 {
            let reach = f64::from(i) * 1.5;
            p.fill(Rect::new(b.x + reach, b.y + reach, b.width, b.height), Color::from_argb(14, 0, 0, 0), 6.0);
        }
        p.fill(b, look.color("panel"), 6.0);
        p.stroke(b, look.color("strong_border"), look.border(), 6.0);
        let size = look.size("small");
        p.text(&locale::translate("toolboxTitle", "Tools"), Rect::new(b.x + PAD, b.y, b.width - HEADER - PAD, HEADER), look.color("text"), size, "text", true, TextAlign::Left);
        let icon = if self.toolbox.folded() { "chevron-down" } else { "chevron-up" };
        crate::parts::containers::icon::draw_icon(p, self.toolbox.icon_path(icon).as_deref(), self.fold_place().inset(7.0), look.color("muted"));
        if !self.toolbox.folded() {
            crate::element::draw_children(&self.node, p);
        }
    }
}
