//! A graph (docs/GRAPH-PLAN.md), drawn from what Clarity.Graph
//! (`clarity_graph`) reads: the records of the list it binds as nodes and
//! the records of its `edges` list as edges between them; or, a graph of
//! the data model, the records of the lists it names (`lists`) as nodes of
//! their entity's kind, joined by their references to each other (what the
//! project worked out of the model, `graphModel`). It is laid out
//! (`layout`) and drawn by kind (`nodeLooks`, `edgeLooks`).
//!
//! The person zooms with the buttons in its corner or Ctrl and the wheel,
//! pans with the wheel or by dragging the background, drags nodes to move
//! them (until the graph changes), picks a node (`select`) and opens it
//! with a double click or Enter (`open`). With `hops` it shows only what
//! lies that many edges from the picked node, with a control to change
//! it; with `legend` its kinds stand as chips that hide and show them.
//! Pointing at a node or an edge tells its fields. A graph of the data
//! model writes the picked node's entity and id into the data `pickedKind`
//! and `pickedId` name, and takes code's changes of them as the pick.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use clarity_graph::hops;
use clarity_graph::layout::{self, Layout};
use clarity_graph::model::{self, ModelList};
use clarity_graph::read::{self, key_of, Graph, Node, Roles};
use serde_json::Value;
use skia_safe::{Paint, PaintStyle, Path, PathEffect};

use super::canvas::{color_of, zoom_word};
use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Cursor, Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};
use crate::theme::ThemeLook;

/// The most nodes a graph shows without a `limit` (widgets::GRAPH_LIMIT).
pub const LIMIT: usize = 500;
/// A node's size across at 100%, when its look gives none.
const NODE_SIZE: f64 = 16.0;
/// Room around the drawing when it is fitted into its place.
const MARGIN: f64 = 48.0;
/// How far a press moves before it is a drag.
const DRAG_START: f64 = 4.0;
/// How near the pointer comes to an edge to point at it.
const EDGE_REACH: f64 = 5.0;
/// Below this scale nodes stand too close for their labels; only the picked and pointed-at ones show theirs.
const LABELS_FROM: f64 = 0.45;
/// The most hops the control goes to (widgets::GRAPH_MAX_HOPS).
const MAX_HOPS: usize = 10;

/// The palette's fourth row (conformance/colors.json), whose colors kinds without a look take in turn.
const KIND_COLORS: [&str; 10] = crate::parts::inputs::color::PALETTE[3];

/// A node as the part keeps it across changes of the graph: its list and its place in that list.
pub type At = (usize, usize);

/// How a node is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Circle,
    Square,
    Diamond,
    Box,
}

/// How the nodes of one kind look, as the part's `nodeLooks` and the palette give it.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeLook {
    pub shape: Shape,
    pub color: String,
    pub size: f64,
}

/// How the edges of one kind look.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeLook {
    pub dash: Option<[f32; 2]>,
    pub color: String,
    pub arrow: bool,
}

/// Where a graph's nodes come from.
#[derive(Debug, Clone, PartialEq)]
enum Source {
    /// The list it binds, with its `edges` list.
    List,
    /// The lists it names, each with what the model says of it.
    Model(Vec<(String, ModelList)>),
}

/// The graph as last laid out: all of it (hops and the legend only keep
/// some of it from showing, so they never move a node), where each node
/// stands in the graph's units, the kinds of all its nodes in their order
/// (which gives each its color), and how many it shows of how many when
/// its limit cut it short.
struct Laid {
    signature: u64,
    whole: Graph,
    places: Vec<(f64, f64)>,
    /// Each node's lasting name (`node_key`), which a moved node is kept by.
    keys: Vec<String>,
    kinds: Vec<String>,
    cut: Option<(usize, usize)>,
}

/// A press: where, on what, and whether it became a drag.
#[derive(Debug, Clone)]
struct Press {
    at: (f64, f64),
    on: Pressed,
    dragging: bool,
}

#[derive(Debug, Clone)]
enum Pressed {
    /// A node, by its lasting name, at its place in the graph's units when the press began.
    Node(String, (f64, f64)),
    /// The background, with the pan when the press began.
    Background((f64, f64)),
}

/// A control in the top left corner: the hops' minus and plus, or a kind's chip.
#[derive(Debug, Clone, PartialEq)]
enum Control {
    FewerHops,
    MoreHops,
    Kind(String),
}

/// From the graph's units to the screen: the screen point of the units' origin and the scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub origin: (f64, f64),
    pub scale: f64,
}

impl View {
    pub fn screen(&self, at: (f64, f64)) -> (f64, f64) {
        (self.origin.0 + at.0 * self.scale, self.origin.1 + at.1 * self.scale)
    }

    pub fn units(&self, at: (f64, f64)) -> (f64, f64) {
        ((at.0 - self.origin.0) / self.scale, (at.1 - self.origin.1) / self.scale)
    }
}

/// The view that fits `places` into `area`, zoomed by `zoom` around its middle and moved by `pan`.
pub fn view_of(places: &[(f64, f64)], area: Rect, zoom: f64, pan: (f64, f64)) -> View {
    let (mut left, mut top, mut right, mut bottom) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    if let Some(first) = places.first() {
        (left, top, right, bottom) = (first.0, first.1, first.0, first.1);
        for p in places {
            left = left.min(p.0);
            right = right.max(p.0);
            top = top.min(p.1);
            bottom = bottom.max(p.1);
        }
    }
    let room = ((area.width - 2.0 * MARGIN).max(20.0), (area.height - 2.0 * MARGIN).max(20.0));
    let fit = (room.0 / (right - left).max(1.0)).min(room.1 / (bottom - top).max(1.0)).min(1.5);
    let scale = fit * zoom;
    let middle = ((left + right) / 2.0, (top + bottom) / 2.0);
    let center = (area.x + area.width / 2.0 + pan.0, area.y + area.height / 2.0 + pan.1);
    View { origin: (center.0 - middle.0 * scale, center.1 - middle.1 * scale), scale }
}

/// How far a point is from the line between two others.
fn from_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length).clamp(0.0, 1.0) } else { 0.0 };
    let (x, y) = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - x).powi(2) + (p.1 - y).powi(2)).sqrt()
}

pub struct GraphPart {
    base: PartBase,
    laid: RefCell<Option<Laid>>,
    /// Nodes the person moved, by their lasting name, in the graph's units;
    /// kept for the person (`choices`) when the graph has a name.
    moved: HashMap<String, (f64, f64)>,
    /// Whether the kept moves were read.
    moves_read: bool,
    /// What the view was last fitted to: the nodes shown and their places then.
    fitted: RefCell<Option<(u64, Vec<(f64, f64)>)>>,
    zoom: f64,
    pan: (f64, f64),
    pointer: Option<(f64, f64)>,
    press: Option<Press>,
    /// The picked node when no data holds it: a graph without a name, or of lists without `pickedKind` and `pickedId`.
    own: Option<At>,
    /// How many hops from the picked node it shows now; none: all of it.
    hops: Option<usize>,
    /// The kinds the person hid with the legend.
    hidden: Vec<String>,
    /// Where the controls in its top left corner were drawn, on the screen.
    controls: RefCell<Vec<(Rect, Control)>>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let hops = spec.number("hops").filter(|h| *h >= 1.0).map(|h| (h as usize).min(MAX_HOPS));
    Box::new(GraphPart {
        base: PartBase::new(spec, host),
        laid: RefCell::new(None),
        moved: HashMap::new(),
        moves_read: false,
        fitted: RefCell::new(None),
        zoom: 1.0,
        pan: (0.0, 0.0),
        pointer: None,
        press: None,
        own: None,
        hops,
        hidden: Vec::new(),
        controls: RefCell::new(Vec::new()),
    })
}

impl GraphPart {
    /// The fields that play a role in a graph of one list, as the part names them.
    pub fn roles(&self) -> Roles {
        let spec = &self.base.spec;
        let named = |field: &str| spec.text(field).filter(|t| !t.is_empty()).map(str::to_string);
        let defaults = Roles::default();
        Roles {
            key: named("nodeKey").unwrap_or(defaults.key),
            label: named("nodeLabel"),
            kind: named("nodeKind"),
            from: named("edgeFrom").unwrap_or(defaults.from),
            to: named("edgeTo").unwrap_or(defaults.to),
            edge_label: named("edgeLabel"),
            edge_kind: named("edgeKind"),
        }
    }

    pub fn layout(&self) -> Layout {
        Layout::read(self.base.spec.text("layout"))
    }

    fn limit(&self) -> usize {
        self.base.spec.number("limit").filter(|n| *n >= 1.0).map(|n| n as usize).unwrap_or(LIMIT)
    }

    /// Where its nodes come from: the lists the project worked out of the model, else the list it binds.
    fn source(&self) -> Source {
        let lists = self.base.spec.json("graphModel").and_then(|m| m.get("lists")).and_then(Value::as_array);
        let Some(lists) = lists.filter(|l| !l.is_empty()) else { return Source::List };
        let text = |v: &Value, f: &str| v.get(f).and_then(Value::as_str).unwrap_or_default().to_string();
        Source::Model(
            lists
                .iter()
                .map(|l| {
                    let refs = l.get("refs").and_then(Value::as_array).map(|r| r.iter().map(|r| (text(r, "field"), text(r, "to"))).collect()).unwrap_or_default();
                    let label = l.get("label").and_then(Value::as_str).map(str::to_string);
                    (text(l, "list"), ModelList { entity: text(l, "entity"), label, refs })
                })
                .collect(),
        )
    }

    fn rows_of(&self, path: Option<&str>) -> Vec<Value> {
        path.filter(|p| !p.is_empty()).map(|p| self.base.host.rows(p)).unwrap_or_default()
    }

    /// Every node and edge its lists hold, up to its limit, before hops and the legend keep some.
    fn whole_graph(&self) -> Graph {
        match self.source() {
            Source::List => read::read(&self.rows_of(self.base.bound()), &self.rows_of(self.base.spec.text("edges")), &self.roles(), self.limit()),
            Source::Model(lists) => {
                let lists: Vec<(ModelList, Vec<Value>)> = lists.into_iter().map(|(list, model)| (model, self.base.host.rows(&list))).collect();
                model::read(&lists, self.limit())
            }
        }
    }

    /// The record a node stands for.
    fn record_of(&self, node: &Node) -> Option<Value> {
        let rows = match self.source() {
            Source::List => self.rows_of(self.base.bound()),
            Source::Model(lists) => self.rows_of(lists.get(node.list).map(|(l, _)| l.as_str())),
        };
        rows.get(node.row).cloned()
    }

    /// The data a graph of lists writes the picked node's entity and id into, when it names both.
    fn picked_data(&self) -> Option<(String, String)> {
        let named = |f: &str| self.base.spec.text(f).filter(|t| !t.is_empty()).map(str::to_string);
        match self.source() {
            Source::Model(_) => Some((named("pickedKind")?, named("pickedId")?)),
            Source::List => None,
        }
    }

    /// The node the person or code picked, as it stands in `whole`.
    fn picked_in(&self, whole: &Graph) -> Option<At> {
        if let Some((kind, id)) = self.picked_data() {
            let kind = self.base.host.read(&kind)?;
            let id = key_of(&self.base.host.read(&id)?)?;
            let node = whole.nodes.iter().find(|n| Some(n.kind.as_str()) == kind.as_str() && self.record_of(n).as_ref().and_then(|r| read::field(r, "id")).and_then(key_of).as_ref() == Some(&id))?;
            return Some((node.list, node.row));
        }
        match (self.source(), self.base.name()) {
            (Source::List, Some(name)) => self.base.host.selected(name).map(|row| (0, row)),
            _ => self.own,
        }
    }

    /// The node picked now.
    pub fn picked(&self) -> Option<At> {
        self.picked_in(&self.whole_graph())
    }

    /// The record picked in a graph of one list, by its place in the list.
    pub fn chosen_row(&self) -> Option<usize> {
        self.picked().map(|(_, row)| row)
    }

    /// A node's lasting name, which outlives its place in a list: its
    /// entity and id in a graph of lists, its key in a graph of one list.
    fn node_key(&self, node: &Node) -> String {
        let record = self.record_of(node);
        let named = |field: &str| record.as_ref().and_then(|r| read::field(r, field)).and_then(key_of);
        match self.source() {
            Source::Model(_) => format!("{}:{}", node.kind, named("id").unwrap_or_else(|| format!("row{}", node.row))),
            Source::List => named(&self.roles().key).unwrap_or_else(|| format!("row{}", node.row)),
        }
    }

    /// The graph as it shows now (its hops and legend applied) and where
    /// each node stands: the whole graph is laid out again only when it,
    /// its layout or (for a radial layout) its picked node changed, and a
    /// node the person moved stands where they left it.
    pub fn laid(&self) -> (Graph, Vec<(f64, f64)>) {
        let whole = self.whole_graph();
        let picked = self.picked_in(&whole);
        let focus = picked.and_then(|(list, row)| whole.node_at(list, row));
        let layout = self.layout();
        let center = if layout == Layout::Radial { focus } else { None };
        let signature = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            format!("{:?}{layout:?}{center:?}", whole).hash(&mut h);
            h.finish()
        };
        let mut laid = self.laid.borrow_mut();
        if laid.as_ref().is_none_or(|l| l.signature != signature) {
            let joined: Vec<(usize, usize)> = whole.edges.iter().map(|e| (e.from, e.to)).collect();
            let places = layout::place(layout, whole.nodes.len(), &joined, center);
            let cut = whole.cut().then_some((whole.nodes.len(), whole.count));
            let keys = whole.nodes.iter().map(|n| self.node_key(n)).collect();
            *laid = Some(Laid { signature, kinds: whole.kinds(), whole, places, keys, cut });
        }
        let laid = laid.as_ref().expect("laid out above");
        let whole = &laid.whole;
        let near = match (self.hops, focus) {
            (Some(hops), Some(focus)) => hops::within(whole, focus, hops),
            _ => vec![true; whole.nodes.len()],
        };
        let keep: Vec<bool> = whole.nodes.iter().enumerate().map(|(n, node)| Some(n) == focus || (near[n] && !self.hidden.contains(&node.kind))).collect();
        let shown = hops::kept(whole, &keep);
        let places = (0..whole.nodes.len()).filter(|n| keep[*n]).map(|n| self.moved.get(&laid.keys[n]).copied().unwrap_or(laid.places[n])).collect();
        (shown, places)
    }

    /// The lasting name of a node shown, by its place in the graph shown.
    fn shown_key(&self, graph: &Graph, n: usize) -> Option<String> {
        let node = graph.nodes.get(n)?;
        let laid = self.laid.borrow();
        let laid = laid.as_ref()?;
        let at = laid.whole.node_at(node.list, node.row)?;
        laid.keys.get(at).cloned()
    }

    /// Where its kept moves are kept for the person: by window and part.
    fn remember_key(&self) -> Option<String> {
        let name = self.base.name().filter(|n| !n.is_empty())?;
        let window = self.base.host.window_name().filter(|w| !w.is_empty())?;
        Some(format!("graph.{window}.{name}"))
    }

    /// The moves the person left before, read once.
    fn read_moves(&mut self) {
        if self.moves_read {
            return;
        }
        self.moves_read = true;
        let Some(text) = self.remember_key().and_then(|k| crate::choices::kept(&k)).filter(|t| !t.is_empty()) else { return };
        // A damaged value: the layout's places.
        let Ok(kept) = serde_json::from_str::<serde_json::Map<String, Value>>(&text) else { return };
        for (key, place) in kept {
            let xy = place.as_array().map(|a| (a.first().and_then(Value::as_f64), a.get(1).and_then(Value::as_f64)));
            if let Some((Some(x), Some(y))) = xy {
                if x.is_finite() && y.is_finite() {
                    self.moved.insert(key, (x, y));
                }
            }
        }
    }

    /// Keeps the person's moves for the next time the window opens.
    fn keep_moves(&self) {
        let Some(key) = self.remember_key() else { return };
        let mut kept: Vec<(&String, &(f64, f64))> = self.moved.iter().collect();
        kept.sort_by(|a, b| a.0.cmp(b.0));
        let map: serde_json::Map<String, Value> = kept.into_iter().map(|(k, (x, y))| (k.clone(), serde_json::json!([x.round(), y.round()]))).collect();
        crate::choices::keep_value(&key, &Value::Object(map).to_string());
    }

    /// The person's moves forgotten: every node where the layout puts it.
    pub fn forget_moves(&mut self) {
        self.moved.clear();
        self.keep_moves();
    }

    /// The kinds of all its nodes in their order, which gives each kind without a color its color.
    fn kinds(&self) -> Vec<String> {
        let _ = self.laid();
        self.laid.borrow().as_ref().map(|l| l.kinds.clone()).unwrap_or_default()
    }

    /// Where the node standing for the record at `row` of the list it binds is on the screen; none when it is not shown.
    pub fn node_point(&self, row: usize) -> Option<(f64, f64)> {
        self.node_point_at((0, row))
    }

    /// Where the node standing for the record at `row` of list `list` is on the screen; none when it is not shown.
    pub fn node_point_at(&self, at: At) -> Option<(f64, f64)> {
        let (graph, places) = self.laid();
        let n = graph.node_at(at.0, at.1)?;
        Some(self.view().screen(places[n]))
    }

    /// The view of the graph now: fitted to the nodes shown where they stood
    /// when they came to show, so moving a node does not move the others.
    pub fn view(&self) -> View {
        let (shown, places) = self.laid();
        let signature = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            format!("{:?}", shown.nodes).hash(&mut h);
            self.laid.borrow().as_ref().map(|l| l.signature).hash(&mut h);
            h.finish()
        };
        let mut fitted = self.fitted.borrow_mut();
        if fitted.as_ref().is_none_or(|(seen, _)| *seen != signature) {
            *fitted = Some((signature, places));
        }
        view_of(&fitted.as_ref().expect("fitted above").1, self.base.node.bounds, self.zoom, self.pan)
    }

    /// How the nodes of `kind` look: its `nodeLooks`, the rest filled in, its color from the palette by the kind's turn.
    pub fn node_look(&self, kind: &str, kinds: &[String]) -> NodeLook {
        let written = self.base.spec.json("nodeLooks").and_then(|l| l.get(kind));
        let text = |field: &str| written.and_then(|w| w.get(field)).and_then(Value::as_str);
        let shape = match text("shape") {
            Some("square") => Shape::Square,
            Some("diamond") => Shape::Diamond,
            Some("box") => Shape::Box,
            _ => Shape::Circle,
        };
        let turn = kinds.iter().position(|k| k == kind).unwrap_or(0);
        let color = text("color").map(str::to_string).unwrap_or_else(|| KIND_COLORS[turn % KIND_COLORS.len()].to_string());
        let size = written.and_then(|w| w.get("size")).and_then(Value::as_f64).filter(|s| *s > 0.0).unwrap_or(NODE_SIZE);
        NodeLook { shape, color, size }
    }

    /// How the edges of `kind` look: its `edgeLooks`, the rest a solid line in the border's color with an arrow.
    pub fn edge_look(&self, kind: &str) -> EdgeLook {
        let written = self.base.spec.json("edgeLooks").and_then(|l| l.get(kind));
        let text = |field: &str| written.and_then(|w| w.get(field)).and_then(Value::as_str);
        let dash = match text("line") {
            Some("dashed") => Some([6.0, 4.0]),
            Some("dotted") => Some([1.5, 3.0]),
            _ => None,
        };
        let color = text("color").unwrap_or("strong_border").to_string();
        let arrow = written.and_then(|w| w.get("arrow")).and_then(Value::as_bool).unwrap_or(true);
        EdgeLook { dash, color, arrow }
    }

    /// How far a node reaches from its middle on the screen, for pointing at it.
    fn reach(look: &NodeLook, scale: f64, label_width: f64) -> (f64, f64) {
        let size = look.size * scale.clamp(0.5, 2.0);
        match look.shape {
            Shape::Box => ((label_width + 16.0) / 2.0, (size.max(12.0) + 8.0) / 2.0),
            _ => (size / 2.0, size / 2.0),
        }
    }

    /// The node a point of the screen is on: the one drawn last, on top.
    fn node_at(&self, x: f64, y: f64, graph: &Graph, places: &[(f64, f64)], fonts: Option<&crate::fonts::Fonts>) -> Option<usize> {
        let view = self.view();
        let kinds = self.kinds();
        (0..graph.nodes.len()).rev().find(|&n| {
            let look = self.node_look(&graph.nodes[n].kind, &kinds);
            let width = match (look.shape, fonts) {
                (Shape::Box, Some(f)) => f.measure(&graph.nodes[n].label, "text", 12.0, false),
                (Shape::Box, None) => graph.nodes[n].label.chars().count() as f64 * 7.0,
                _ => 0.0,
            };
            let (rx, ry) = Self::reach(&look, view.scale, width);
            let (cx, cy) = view.screen(places[n]);
            (x - cx).abs() <= rx.max(5.0) && (y - cy).abs() <= ry.max(5.0)
        })
    }

    /// The edge a point of the screen is near, when it is on no node.
    fn edge_at(&self, x: f64, y: f64, graph: &Graph, places: &[(f64, f64)]) -> Option<usize> {
        let view = self.view();
        graph.edges.iter().position(|e| e.from != e.to && from_segment((x, y), view.screen(places[e.from]), view.screen(places[e.to])) <= EDGE_REACH)
    }

    /// The zoom buttons in its top right corner, on the screen, as a canvas's: out, back to whole, in.
    pub fn zoom_buttons(&self) -> (Rect, Rect, Rect) {
        let b = self.base.node.bounds;
        let top = b.y + 6.0;
        let right = b.right() - 6.0;
        let zoom_in = Rect::new(right - 26.0, top, 26.0, 24.0);
        let back = Rect::new(zoom_in.x - 2.0 - 52.0, top, 52.0, 24.0);
        let zoom_out = Rect::new(back.x - 2.0 - 26.0, top, 26.0, 24.0);
        (zoom_out, back, zoom_in)
    }

    fn zoom_button_at(&self, x: f64, y: f64) -> Option<&'static str> {
        let (zoom_out, back, zoom_in) = self.zoom_buttons();
        [(zoom_out, "zoomOut"), (back, "zoomWhole"), (zoom_in, "zoomIn")].into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, b)| b)
    }

    /// The hops control in its top left corner, on the screen: minus, how many, plus; none without `hops`.
    pub fn hops_control(&self) -> Option<(Rect, Rect, Rect)> {
        self.hops?;
        let b = self.base.node.bounds;
        let (left, top) = (b.x + 6.0, b.y + 6.0);
        let fewer = Rect::new(left, top, 26.0, 24.0);
        let count = Rect::new(fewer.right() + 2.0, top, 64.0, 24.0);
        let more = Rect::new(count.right() + 2.0, top, 26.0, 24.0);
        Some((fewer, count, more))
    }

    /// Where a kind's chip of the legend was drawn; none without `legend` or before it is drawn.
    pub fn chip_of(&self, kind: &str) -> Option<Rect> {
        self.controls.borrow().iter().find(|(_, c)| *c == Control::Kind(kind.to_string())).map(|(r, _)| *r)
    }

    fn control_at(&self, x: f64, y: f64) -> Option<Control> {
        self.controls.borrow().iter().find(|(r, _)| r.contains(x, y)).map(|(_, c)| c.clone())
    }

    pub fn hops(&self) -> Option<usize> {
        self.hops
    }

    /// The person changed how many hops it shows: 1 to `MAX_HOPS`.
    pub fn set_hops(&mut self, hops: usize) {
        if self.hops.is_some() {
            self.hops = Some(hops.clamp(1, MAX_HOPS));
        }
    }

    /// The person hid or showed a kind with its chip.
    pub fn toggle_kind(&mut self, kind: &str) {
        match self.hidden.iter().position(|k| k == kind) {
            Some(i) => {
                self.hidden.remove(i);
            }
            None => self.hidden.push(kind.to_string()),
        }
    }

    /// Zooms by `factor` keeping the point `at` of the screen where it is.
    pub fn zoom_by(&mut self, factor: f64, at: Option<(f64, f64)>) {
        let before = self.view();
        let b = self.base.node.bounds;
        let at = at.unwrap_or((b.x + b.width / 2.0, b.y + b.height / 2.0));
        let kept = before.units(at);
        self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
        let after = self.view();
        let now = after.screen(kept);
        self.pan = (self.pan.0 + at.0 - now.0, self.pan.1 + at.1 - now.1);
    }

    /// Shows the whole graph again: no zoom, no pan.
    pub fn whole(&mut self) {
        self.zoom = 1.0;
        self.pan = (0.0, 0.0);
    }

    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    /// The person picked `node`: kept where the graph keeps its pick (the
    /// window's selection, or the data of a graph of lists), and `select`
    /// runs when it is another one.
    fn choose(&mut self, node: &Node) {
        let at = (node.list, node.row);
        if self.picked() == Some(at) {
            return;
        }
        self.own = Some(at);
        let part = self.base.name().unwrap_or("").to_string();
        if let Some((kind, id)) = self.picked_data() {
            let record_id = self.record_of(node).and_then(|r| read::field(&r, "id").cloned()).unwrap_or(Value::Null);
            self.base.host.write(&part, &kind, Value::String(node.kind.clone()));
            self.base.host.write(&part, &id, record_id);
        } else if let (Source::List, Some(name)) = (self.source(), self.base.name()) {
            self.base.host.select(name, Some(node.row));
        }
        self.base.fire("select");
    }

    /// What pointing at a node tells: its label, then its fields: the ones `show` names, else every one that plays no part.
    fn tells_node(&self, node: &Node) -> String {
        let roles = self.roles();
        let mut named: Vec<String> = vec!["id".into()];
        match self.source() {
            Source::List => named.extend([Some(roles.key), roles.label, roles.kind].into_iter().flatten()),
            Source::Model(lists) => named.extend(lists.get(node.list).and_then(|(_, m)| m.label.clone())),
        }
        let named: Vec<&str> = named.iter().map(String::as_str).collect();
        let show = self.base.spec.names("show");
        let fields = self.record_of(node).map(|r| read::told(&r, &show, &named)).unwrap_or_default();
        let mut lines = vec![node.label.clone()];
        lines.extend(fields.into_iter().map(|(name, value)| format!("{name}: {value}")));
        lines.retain(|l| !l.is_empty());
        lines.join("\n")
    }

    fn draw_graph(&self, p: &Painter) {
        let look = p.look;
        let b = self.base.node.bounds;
        p.fill(b, look.color("input"), look.radius("input"));
        let (graph, places) = self.laid();
        let view = self.view();
        let _clip = p.clip(b);
        let kinds = self.kinds();
        let hovered = self.pointer.filter(|_| self.base.node.hovered && self.press.as_ref().is_none_or(|p| !p.dragging));
        let hovered_node = hovered.and_then(|(x, y)| self.node_at(x, y, &graph, &places, Some(p.fonts)));
        let chosen = self.picked().and_then(|(list, row)| graph.node_at(list, row));
        let lit = |n: usize| Some(n) == hovered_node || Some(n) == chosen;
        let text_size = look.size("small");
        let looks: Vec<NodeLook> = graph.nodes.iter().map(|n| self.node_look(&n.kind, &kinds)).collect();
        let widths: Vec<f64> = graph.nodes.iter().zip(&looks).map(|(n, l)| if l.shape == Shape::Box { p.fonts.measure(&n.label, "text", text_size, false) } else { 0.0 }).collect();
        // Edges that join the same two nodes stand side by side.
        let mut pairs: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        for e in &graph.edges {
            pairs.entry((e.from.min(e.to), e.from.max(e.to))).or_insert((0, 0)).1 += 1;
        }
        for edge in &graph.edges {
            let pair = pairs.get_mut(&(edge.from.min(edge.to), edge.from.max(edge.to))).expect("counted above");
            let (turn, of) = (pair.0, pair.1);
            pair.0 += 1;
            let edge_look = self.edge_look(&edge.kind);
            let near = lit(edge.from) || lit(edge.to);
            let color = if near { look.color("accent") } else { color_of(look, &edge_look.color) };
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            paint.set_style(PaintStyle::Stroke);
            paint.set_color(color);
            paint.set_stroke_width(if near { 2.0 } else { 1.2 });
            if let Some(dash) = edge_look.dash {
                paint.set_path_effect(PathEffect::dash(&dash, 0.0));
            }
            let (a, z) = (view.screen(places[edge.from]), view.screen(places[edge.to]));
            if edge.from == edge.to {
                let r = 10.0 * view.scale.clamp(0.5, 2.0);
                let (_, ry) = Self::reach(&looks[edge.from], view.scale, widths[edge.from]);
                p.canvas.draw_circle((a.0 as f32, (a.1 - ry - r) as f32), r as f32, &paint);
                continue;
            }
            let length = ((z.0 - a.0).powi(2) + (z.1 - a.1).powi(2)).sqrt().max(0.1);
            let (ux, uy) = ((z.0 - a.0) / length, (z.1 - a.1) / length);
            // Side by side: each edge of the same pair moved across its direction, the pair's middle on the straight line.
            let shift = (turn as f64 - (of as f64 - 1.0) / 2.0) * 8.0;
            let shift = if edge.from > edge.to { -shift } else { shift };
            let (nx, ny) = (-uy * shift, ux * shift);
            let (a, z) = ((a.0 + nx, a.1 + ny), (z.0 + nx, z.1 + ny));
            let (rx, ry) = Self::reach(&looks[edge.to], view.scale, widths[edge.to]);
            let stop = if looks[edge.to].shape == Shape::Box { (rx / ux.abs().max(1e-6)).min(ry / uy.abs().max(1e-6)) } else { rx };
            let end = (z.0 - ux * (stop + 1.0), z.1 - uy * (stop + 1.0));
            p.canvas.draw_line((a.0 as f32, a.1 as f32), (end.0 as f32, end.1 as f32), &paint);
            if edge_look.arrow {
                let size = 7.0;
                let corners = [
                    skia_safe::Point::new(end.0 as f32, end.1 as f32),
                    skia_safe::Point::new((end.0 - ux * size - uy * size * 0.5) as f32, (end.1 - uy * size + ux * size * 0.5) as f32),
                    skia_safe::Point::new((end.0 - ux * size + uy * size * 0.5) as f32, (end.1 - uy * size - ux * size * 0.5) as f32),
                ];
                let head = Path::polygon(&corners, true, None, None);
                let mut fill = Paint::default();
                fill.set_anti_alias(true);
                fill.set_color(color);
                p.canvas.draw_path(&head, &fill);
            }
            if !edge.label.is_empty() && (view.scale >= LABELS_FROM || near) {
                let middle = ((a.0 + z.0) / 2.0, (a.1 + z.1) / 2.0);
                let width = p.fonts.measure(&edge.label, "text", text_size, false).min(160.0) + 8.0;
                let place = Rect::new(middle.0 - width / 2.0, middle.1 - 9.0, width, 18.0);
                p.fill(place, look.color("input"), 3.0);
                p.text(&p.fit(&edge.label, width - 4.0, text_size, "text", false), place, look.color("muted"), text_size, "text", false, TextAlign::Center);
            }
        }
        for (n, node) in graph.nodes.iter().enumerate() {
            let node_look = &looks[n];
            let fill = color_of(look, &node_look.color);
            let (cx, cy) = view.screen(places[n]);
            let (rx, ry) = Self::reach(node_look, view.scale, widths[n]);
            let ring = if lit(n) { look.color("accent") } else { look.color("input") };
            let ring_width = if Some(n) == chosen { 3.0 } else { 1.5 };
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            paint.set_color(fill);
            let mut outline = paint.clone();
            outline.set_style(PaintStyle::Stroke);
            outline.set_stroke_width(ring_width);
            outline.set_color(ring);
            match node_look.shape {
                Shape::Circle => {
                    p.canvas.draw_circle((cx as f32, cy as f32), rx as f32, &paint);
                    p.canvas.draw_circle((cx as f32, cy as f32), rx as f32, &outline);
                }
                Shape::Square => {
                    let r = Rect::new(cx - rx, cy - ry, rx * 2.0, ry * 2.0).sk();
                    p.canvas.draw_rect(r, &paint);
                    p.canvas.draw_rect(r, &outline);
                }
                Shape::Diamond => {
                    let corners = [
                        skia_safe::Point::new(cx as f32, (cy - ry * 1.2) as f32),
                        skia_safe::Point::new((cx + rx * 1.2) as f32, cy as f32),
                        skia_safe::Point::new(cx as f32, (cy + ry * 1.2) as f32),
                        skia_safe::Point::new((cx - rx * 1.2) as f32, cy as f32),
                    ];
                    let d = Path::polygon(&corners, true, None, None);
                    p.canvas.draw_path(&d, &paint);
                    p.canvas.draw_path(&d, &outline);
                }
                Shape::Box => {
                    let r = Rect::new(cx - rx, cy - ry, rx * 2.0, ry * 2.0);
                    p.canvas.draw_round_rect(r.sk(), 5.0, 5.0, &paint);
                    p.canvas.draw_round_rect(r.sk(), 5.0, 5.0, &outline);
                    p.text(&node.label, r, look.color("on_accent"), text_size, "text", lit(n), TextAlign::Center);
                }
            }
            if node_look.shape != Shape::Box && !node.label.is_empty() && (view.scale >= LABELS_FROM || lit(n)) {
                let width = p.fonts.measure(&node.label, "text", text_size, lit(n)).min(220.0) + 4.0;
                let place = Rect::new(cx + rx + 4.0, cy - 9.0, width, 18.0);
                p.text(&p.fit(&node.label, width, text_size, "text", lit(n)), place, look.color("text"), text_size, "text", lit(n), TextAlign::Left);
            }
        }
        let cut = self.laid.borrow().as_ref().and_then(|l| l.cut);
        if let Some((shown, count)) = cut {
            let words = locale::fill(&locale::fill(&locale::translate("graphCut", "{shown} of {count} nodes shown"), "shown", &shown.to_string()), "count", &count.to_string());
            let place = Rect::new(b.x + 8.0, b.bottom() - 24.0, (b.width - 16.0).max(0.0), 18.0);
            p.text(&words, place, look.color("muted"), text_size, "text", false, TextAlign::Left);
        }
        self.draw_controls(p, look, &kinds);
        self.draw_zoom_buttons(p, look);
    }

    /// The hops control and the legend's chips, in its top left corner, kept for the pointer to find.
    fn draw_controls(&self, p: &Painter, look: &ThemeLook, kinds: &[String]) {
        let mut controls = Vec::new();
        let b = self.base.node.bounds;
        let size = look.size("small");
        let mut left = b.x + 6.0;
        let top = b.y + 6.0;
        if let (Some((fewer, count, more)), Some(hops)) = (self.hops_control(), self.hops) {
            let words = locale::fill(&locale::translate("graphHops", "{hops} hops"), "hops", &hops.to_string());
            for (place, label) in [(fewer, "-"), (more, "+")] {
                p.fill(place, look.color("panel"), look.radius("button"));
                p.stroke(place, look.color("border"), look.border(), look.radius("button"));
                p.text(label, place, look.color("text"), size, "text", false, TextAlign::Center);
            }
            p.text(&words, count, look.color("text"), size, "text", false, TextAlign::Center);
            controls.push((fewer, Control::FewerHops));
            controls.push((more, Control::MoreHops));
            left = more.right() + 10.0;
        }
        if self.base.spec.flag("legend") {
            let (zoom_out, _, _) = self.zoom_buttons();
            let mut x = left;
            let mut y = top;
            for kind in kinds {
                let shown = if kind.is_empty() { "-" } else { kind.as_str() };
                let width = p.fonts.measure(shown, "text", size, false).min(160.0) + 26.0;
                let room = if y == top { zoom_out.x - 8.0 } else { b.right() - 6.0 };
                if x + width > room && x > left {
                    x = b.x + 6.0;
                    y += 28.0;
                }
                let chip = Rect::new(x, y, width, 24.0);
                let hidden = self.hidden.contains(kind);
                p.fill(chip, look.color("panel"), 12.0);
                p.stroke(chip, look.color("border"), look.border(), 12.0);
                let dot = color_of(look, &self.node_look(kind, kinds).color);
                let mut paint = Paint::default();
                paint.set_anti_alias(true);
                paint.set_color(dot);
                if hidden {
                    paint.set_style(PaintStyle::Stroke);
                    paint.set_stroke_width(1.5);
                }
                p.canvas.draw_circle(((chip.x + 12.0) as f32, (chip.y + 12.0) as f32), 5.0, &paint);
                let text = Rect::new(chip.x + 20.0, chip.y, width - 24.0, 24.0);
                p.text(&p.fit(shown, text.width, size, "text", false), text, if hidden { look.color("muted") } else { look.color("text") }, size, "text", false, TextAlign::Left);
                controls.push((chip, Control::Kind(kind.clone())));
                x += width + 6.0;
            }
        }
        *self.controls.borrow_mut() = controls;
    }

    fn draw_zoom_buttons(&self, p: &Painter, look: &ThemeLook) {
        let (zoom_out, back, zoom_in) = self.zoom_buttons();
        let percent = format!("{}%", (self.zoom * 100.0).round());
        for (place, label) in [(zoom_out, "-"), (back, percent.as_str()), (zoom_in, "+")] {
            p.fill(place, look.color("panel"), look.radius("button"));
            p.stroke(place, look.color("border"), look.border(), look.radius("button"));
            p.text(label, place, look.color("text"), look.size("small"), "text", false, TextAlign::Center);
        }
    }
}

impl Element for GraphPart {
    part_element!();

    fn arrange(&mut self, area: Rect, _cx: &mut Cx) {
        self.base.node.bounds = area;
        self.read_moves();
    }

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        let (x, y) = (at.screen_x, at.screen_y);
        self.press = None;
        if let Some(button) = self.zoom_button_at(x, y) {
            match button {
                "zoomOut" => self.zoom_by(1.0 / 1.2, None),
                "zoomIn" => self.zoom_by(1.2, None),
                _ => self.whole(),
            }
            cx.invalidate();
            return;
        }
        if let Some(control) = self.control_at(x, y) {
            match control {
                Control::FewerHops => self.set_hops(self.hops.unwrap_or(1).saturating_sub(1)),
                Control::MoreHops => self.set_hops(self.hops.unwrap_or(1) + 1),
                Control::Kind(kind) => self.toggle_kind(&kind),
            }
            cx.invalidate();
            return;
        }
        let (graph, places) = self.laid();
        match self.node_at(x, y, &graph, &places, None) {
            Some(n) => {
                let node = graph.nodes[n].clone();
                if self.base.enabled() {
                    self.choose(&node);
                    // Every click on a node: `on click`, or what `opens` names (a slide-out with its details).
                    self.base.fire("click");
                    if at.clicks == 2 {
                        self.base.fire("open");
                    }
                }
                let key = self.shown_key(&graph, n).unwrap_or_default();
                self.press = Some(Press { at: (x, y), on: Pressed::Node(key, places[n]), dragging: false });
            }
            None => self.press = Some(Press { at: (x, y), on: Pressed::Background(self.pan), dragging: false }),
        }
        cx.invalidate();
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let (x, y) = (at.screen_x, at.screen_y);
        self.pointer = Some((x, y));
        if let Some(press) = self.press.as_mut() {
            let (dx, dy) = (x - press.at.0, y - press.at.1);
            if !press.dragging && dx.abs().max(dy.abs()) >= DRAG_START {
                press.dragging = true;
            }
            if press.dragging {
                let press = press.clone();
                match press.on {
                    Pressed::Node(node, from) => {
                        let scale = self.view().scale.max(1e-6);
                        self.moved.insert(node, (from.0 + dx / scale, from.1 + dy / scale));
                    }
                    Pressed::Background(pan) => self.pan = (pan.0 + dx, pan.1 + dy),
                }
            }
        }
        cx.invalidate();
    }

    fn on_pointer_released(&mut self, _at: PointerInput, cx: &mut Cx) {
        if let Some(Press { dragging: true, on: Pressed::Node(..), .. }) = self.press {
            self.keep_moves();
        }
        self.press = None;
        cx.invalidate();
    }

    fn on_pointer_cancelled(&mut self, cx: &mut Cx) {
        self.press = None;
        cx.invalidate();
    }

    fn on_wheel(&mut self, dx: f64, dy: f64, held: Modifiers, cx: &mut Cx) -> bool {
        // The wheel with Ctrl zooms, closer as it turns up, at the pointer; without, it pans.
        if held.has(Modifiers::CONTROL) && dy != 0.0 {
            self.zoom_by(if dy < 0.0 { 1.2 } else { 1.0 / 1.2 }, self.pointer);
        } else {
            self.pan = (self.pan.0 - dx, self.pan.1 - dy);
        }
        cx.invalidate();
        true
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        match key.key {
            Key::Enter if self.picked().is_some() && self.base.enabled() => {
                self.base.fire("open");
            }
            Key::Plus | Key::Equal => self.zoom_by(1.2, None),
            Key::Minus => self.zoom_by(1.0 / 1.2, None),
            Key::D0 => self.whole(),
            _ => return false,
        }
        cx.invalidate();
        true
    }

    fn cursor_at(&self, x: f64, y: f64) -> Option<Cursor> {
        if self.zoom_button_at(x, y).is_some() || self.control_at(x, y).is_some() {
            return Some(Cursor::Hand);
        }
        let (graph, places) = self.laid();
        match self.press {
            Some(Press { dragging: true, on: Pressed::Background(_), .. }) => Some(Cursor::Move),
            _ if self.node_at(x, y, &graph, &places, None).is_some() => Some(Cursor::Hand),
            _ => None,
        }
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        if let Some(button) = self.zoom_button_at(x, y) {
            return Some(zoom_word(button));
        }
        match self.control_at(x, y) {
            Some(Control::FewerHops) => return Some(locale::translate("graphFewerHops", "Fewer hops")),
            Some(Control::MoreHops) => return Some(locale::translate("graphMoreHops", "More hops")),
            Some(Control::Kind(kind)) => return Some(locale::fill(&locale::translate("graphKind", "Hide or show {kind}"), "kind", &kind)),
            None => {}
        }
        let (graph, places) = self.laid();
        if let Some(n) = self.node_at(x, y, &graph, &places, None) {
            return Some(self.tells_node(&graph.nodes[n])).filter(|t| !t.is_empty());
        }
        let e = self.edge_at(x, y, &graph, &places)?;
        let edge = &graph.edges[e];
        let roles = self.roles();
        let named: Vec<&str> = [Some(roles.from.as_str()), Some(roles.to.as_str()), roles.edge_label.as_deref(), roles.edge_kind.as_deref(), Some("id")].into_iter().flatten().collect();
        let record = edge.row.and_then(|row| self.rows_of(self.base.spec.text("edges")).get(row).cloned());
        let fields = record.map(|r| read::told(&r, &[], &named)).unwrap_or_default();
        let ends = format!("{} - {}", graph.nodes[edge.from].label, graph.nodes[edge.to].label);
        let mut lines = vec![if edge.label.is_empty() { ends } else { format!("{} ({ends})", edge.label) }];
        lines.extend(fields.into_iter().map(|(name, value)| format!("{name}: {value}")));
        Some(lines.join("\n"))
    }

    fn draw(&self, p: &Painter) {
        self.draw_graph(p);
        p.stroke(self.base.node.bounds, p.look.color("strong_border"), p.look.border(), p.look.radius("input"));
        if self.base.node.focused {
            p.focus_ring(self.base.node.bounds, p.look.radius("input"));
        }
    }
}

impl Part for GraphPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn grows(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_view_fits_the_places_and_zooms_around_the_middle() {
        let area = Rect::new(0.0, 0.0, 400.0, 300.0);
        let places = [(-100.0, -50.0), (100.0, 50.0)];
        let view = view_of(&places, area, 1.0, (0.0, 0.0));
        let (a, b) = (view.screen(places[0]), view.screen(places[1]));
        assert!(((a.0 + b.0) / 2.0 - 200.0).abs() < 1e-9 && ((a.1 + b.1) / 2.0 - 150.0).abs() < 1e-9, "the middle in the middle");
        assert!(a.0 >= MARGIN - 1e-9 && b.0 <= 400.0 - MARGIN + 1e-9, "inside the margin");
        let zoomed = view_of(&places, area, 2.0, (10.0, 0.0));
        assert!((zoomed.scale - view.scale * 2.0).abs() < 1e-9);
        assert_eq!(zoomed.units(zoomed.screen((3.0, 4.0))), (3.0, 4.0));
    }

    #[test]
    fn a_point_is_near_a_line_only_beside_it() {
        assert!(from_segment((5.0, 1.0), (0.0, 0.0), (10.0, 0.0)) <= 1.0 + 1e-9);
        assert!(from_segment((15.0, 0.0), (0.0, 0.0), (10.0, 0.0)) > 4.0);
    }
}
