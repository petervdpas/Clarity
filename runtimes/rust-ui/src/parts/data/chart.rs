//! A chart (U1c), the same as on the web (conformance/charts.json): the rows
//! of the list it binds grouped by a field, each group counted, summed or
//! averaged (the runtime's `charts::groups`), drawn as bars or a line on an
//! axis in steps of 1, 2, 2.5 or 5 times a power of ten, or as the slices of
//! a pie in the palette's colors with a legend beside it. Bound to a list of
//! numbers instead (DESIGN 5.26), it draws them as they are, the last
//! `points` of them, on an axis up to `max` when it has one. Each bar, point
//! and slice tells its group and number when pointed at. It reads its rows
//! again every time it draws. A trend (`trend.rs`) draws its values with the
//! same drawing.

use std::rc::Rc;

use clarity_runtime::charts::{self, ChartRow};
use serde_json::{Map, Value};
use skia_safe::canvas::PointMode;
use skia_safe::{Color, Paint, PaintCap, PaintJoin, PaintStyle};

use super::rows;
use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::PointerInput;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::inputs::formats::{format_of, formats_of, FieldFormat, FormatKind};
use crate::parts::{Part, PartBase, PartHost};
use crate::theme::parse_color;

/// What a chart draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartShape {
    Bar,
    Line,
    Pie,
}

/// A group as a chart shows it: its label, its number, and that number as people read it.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartShown {
    pub label: String,
    pub value: f64,
    pub text: String,
}

/// What a chart drew for one group: a bar, a point of its line, or a slice of its pie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkKind {
    Bar,
    Point,
    Slice,
}

/// Where a chart drew a group: a bar's place; the place of a point of the
/// line (`dot` when a dot is drawn there); the box of the pie a slice is
/// part of, with the angles it runs over, in degrees clockwise from the
/// right, and its color.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartMark {
    pub kind: MarkKind,
    pub group: ChartShown,
    pub place: Rect,
    pub dot: bool,
    pub from: f64,
    pub sweep: f64,
    pub color: Option<&'static str>,
}

impl ChartMark {
    /// A point well inside it: the middle of a bar or point, a point halfway out along the middle of a slice.
    pub fn center(&self) -> (f64, f64) {
        let (cx, cy) = (self.place.x + self.place.width / 2.0, self.place.y + self.place.height / 2.0);
        if self.kind != MarkKind::Slice {
            return (cx, cy);
        }
        let angle = (self.from + self.sweep / 2.0).to_radians();
        let r = self.place.width / 2.0 * 0.6;
        (cx + r * angle.cos(), cy + r * angle.sin())
    }

    /// Whether a point of the screen is on it.
    pub fn holds(&self, x: f64, y: f64) -> bool {
        match self.kind {
            MarkKind::Bar => self.place.contains(x, y),
            MarkKind::Point => {
                let (px, py) = self.center();
                (x - px).abs() <= POINT_REACH && (y - py).abs() <= POINT_REACH
            }
            MarkKind::Slice => {
                let (cx, cy) = (self.place.x + self.place.width / 2.0, self.place.y + self.place.height / 2.0);
                let (dx, dy) = (x - cx, y - cy);
                if dx * dx + dy * dy > self.place.width * self.place.width / 4.0 {
                    return false;
                }
                let angle = dy.atan2(dx).to_degrees();
                let from = ((angle - self.from) % 360.0 + 360.0) % 360.0;
                from < self.sweep
            }
        }
    }
}

/// The axis of a bar or line chart: from `bottom` to `top` in steps of `step`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartScale {
    pub bottom: f64,
    pub top: f64,
    pub step: f64,
}

fn round12(n: f64) -> f64 {
    (n * 1e12).round() / 1e12
}

/// The axis for the lowest and highest number shown (conformance/charts.json, scale).
pub fn scale(low: f64, high: f64) -> ChartScale {
    let lo = low.min(0.0);
    let mut hi = high.max(0.0);
    if lo == 0.0 && hi == 0.0 {
        hi = 1.0;
    }
    let raw = (hi - lo) / 5.0;
    let power = 10f64.powf(raw.log10().floor());
    let rest = round12(raw / power);
    let factor = if rest <= 1.0 {
        1.0
    } else if rest <= 2.0 {
        2.0
    } else if rest <= 2.5 {
        2.5
    } else if rest <= 5.0 {
        5.0
    } else {
        10.0
    };
    let step = round12(factor * power);
    let bottom = round12(round12(lo / step).floor() * step);
    let top = round12(round12(hi / step).ceil() * step);
    ChartScale { bottom: if bottom == 0.0 { 0.0 } else { bottom }, top, step }
}

/// The palette's fourth row (conformance/colors.json), whose colors a pie's slices take in turn.
const SLICE_ROW: [&str; 10] = crate::parts::inputs::color::PALETTE[3];

/// The colors of a pie's slices: none for a slice of 0 or less (conformance/charts.json, slices).
pub fn slice_colors(values: &[f64]) -> Vec<Option<&'static str>> {
    let mut next = 0;
    values
        .iter()
        .map(|v| {
            if *v > 0.0 {
                let color = SLICE_ROW[next % SLICE_ROW.len()];
                next += 1;
                Some(color)
            } else {
                None
            }
        })
        .collect()
}

/// A number as a chart says it: whole numbers without decimals, others as short as they are exact.
pub fn number_text(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{}", round12(value))
    }
}

const LEFT: f64 = 52.0;
const BOTTOM: f64 = 22.0;
const TOP: f64 = 8.0;
const RIGHT: f64 = 8.0;

/// How near a point of the line the pointer is on it.
pub const POINT_REACH: f64 = 6.0;

/// How many points a line draws with a dot each; more draw as a line alone (DESIGN 5.26).
pub const MOST_DOTS: usize = 30;

fn scale_of(groups: &[ChartShown], max: Option<f64>) -> ChartScale {
    let low = groups.iter().map(|g| g.value).fold(0.0f64, f64::min);
    let high = groups.iter().map(|g| g.value).fold(0.0f64, f64::max);
    scale(low.min(0.0), max.unwrap_or(high.max(0.0)))
}

fn pie_radius(place: Rect) -> f64 {
    ((place.width * 0.6).min(place.height) / 2.0 - 8.0).max(0.0)
}

/// The plot of a bar or line chart: where a number lies down, and where a group lies across.
struct Plot {
    scale: ChartScale,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    band: f64,
}

impl Plot {
    fn of(place: Rect, groups: &[ChartShown], max: Option<f64>) -> Plot {
        let width = (place.width - LEFT - RIGHT).max(0.0);
        let height = (place.height - TOP - BOTTOM).max(0.0);
        let band = if groups.is_empty() { 0.0 } else { width / groups.len() as f64 };
        Plot { scale: scale_of(groups, max), left: place.x + LEFT, top: place.y + TOP, width, height, band }
    }

    fn y(&self, v: f64) -> f64 {
        self.top + self.height * (1.0 - (v - self.scale.bottom) / (self.scale.top - self.scale.bottom))
    }

    fn x(&self, i: usize) -> f64 {
        self.left + self.band * (i as f64 + 0.5)
    }
}

/// Where each group is drawn in `place`: a bar, a point, or a slice (none for a slice of 0 or less).
pub fn marks(place: Rect, shape: ChartShape, groups: &[ChartShown], max: Option<f64>) -> Vec<ChartMark> {
    let mut marks = Vec::new();
    if shape == ChartShape::Pie {
        let values: Vec<f64> = groups.iter().map(|g| g.value).collect();
        let colors = slice_colors(&values);
        let total: f64 = groups.iter().map(|g| g.value.max(0.0)).sum();
        let radius = pie_radius(place);
        let circle = Rect::new(place.x + 8.0, place.y + place.height / 2.0 - radius, radius * 2.0, radius * 2.0);
        let mut start = -90.0;
        for (group, color) in groups.iter().zip(colors) {
            let Some(color) = color.filter(|_| total > 0.0) else { continue };
            let sweep = group.value / total * 360.0;
            marks.push(ChartMark { kind: MarkKind::Slice, group: group.clone(), place: circle, dot: false, from: start, sweep, color: Some(color) });
            start += sweep;
        }
        return marks;
    }
    let plot = Plot::of(place, groups, max);
    let dots = groups.len() <= MOST_DOTS;
    for (i, g) in groups.iter().enumerate() {
        let mark = if shape == ChartShape::Bar {
            let top = plot.y(g.value).min(plot.y(0.0));
            let bar = Rect::new(plot.x(i) - plot.band * 0.3, top, plot.band * 0.6, (plot.y(g.value) - plot.y(0.0)).abs());
            ChartMark { kind: MarkKind::Bar, group: g.clone(), place: bar, dot: false, from: 0.0, sweep: 0.0, color: None }
        } else {
            let point = Rect::new(plot.x(i) - 3.5, plot.y(g.value) - 3.5, 7.0, 7.0);
            ChartMark { kind: MarkKind::Point, group: g.clone(), place: point, dot: dots, from: 0.0, sweep: 0.0, color: None }
        };
        marks.push(mark);
    }
    marks
}

/// What a group pointed at tells: its label and number, or its number alone without a label.
pub fn told(group: &ChartShown) -> String {
    if group.label.is_empty() {
        group.text.clone()
    } else {
        format!("{}: {}", group.label, group.text)
    }
}

/// The group at a point of the screen, as it tells itself; none where there is none.
pub fn tip_at(place: Rect, shape: ChartShape, groups: &[ChartShown], max: Option<f64>, x: f64, y: f64) -> Option<String> {
    marks(place, shape, groups, max).iter().rev().find(|m| m.holds(x, y)).map(|m| told(&m.group))
}

fn paint(color: Color, stroke: bool, width: f64) -> Paint {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_anti_alias(true);
    paint.set_style(if stroke { PaintStyle::Stroke } else { PaintStyle::Fill });
    paint.set_stroke_width(width as f32);
    paint
}

/// Draws a chart in `place`, and the tip of what the pointer is on at `pointer`.
pub fn draw(p: &Painter, place: Rect, shape: ChartShape, groups: &[ChartShown], max: Option<f64>, pointer: Option<(f64, f64)>) {
    let _clip = p.clip(place);
    if shape == ChartShape::Pie {
        draw_pie(p, place, groups);
    } else {
        draw_axis(p, place, shape, groups, max);
    }
    if let Some((x, y)) = pointer {
        if let Some(tip) = tip_at(place, shape, groups, max, x, y) {
            draw_tip(p, place, &tip, (x, y));
        }
    }
}

fn draw_axis(p: &Painter, place: Rect, shape: ChartShape, groups: &[ChartShown], max: Option<f64>) {
    let look = p.look;
    let small = look.size("small");
    let plot = Plot::of(place, groups, max);
    let s = plot.scale;
    let mut t = s.bottom;
    while t <= s.top + s.step / 1e6 {
        let y = plot.y(t).round() + 0.5;
        let line = paint(look.color(if t == 0.0 { "strong_border" } else { "border" }), true, 1.0);
        p.canvas.draw_line((plot.left as f32, y as f32), ((plot.left + plot.width) as f32, y as f32), &line);
        p.text(&number_text(t), Rect::new(place.x, plot.y(t) - 8.0, LEFT - 6.0, 16.0), look.color("muted"), small, "text", false, TextAlign::Right);
        t = ((t + s.step) * 1e12).round() / 1e12;
    }
    for (i, g) in groups.iter().enumerate() {
        if !g.label.is_empty() {
            let at = Rect::new(plot.x(i) - plot.band / 2.0, plot.top + plot.height + 4.0, plot.band, 16.0);
            p.text(&g.label, at, look.color("muted"), small, "text", false, TextAlign::Center);
        }
    }
    let marks = marks(place, shape, groups, max);
    let accent = look.color("accent");
    if shape == ChartShape::Bar {
        for mark in &marks {
            p.fill(mark.place, accent, 0.0);
        }
        return;
    }
    if marks.len() > 1 {
        let mut line = paint(accent, true, 2.0);
        line.set_stroke_join(PaintJoin::Round);
        line.set_stroke_cap(PaintCap::Round);
        let points: Vec<skia_safe::Point> = marks.iter().map(|m| m.center()).map(|(x, y)| skia_safe::Point::new(x as f32, y as f32)).collect();
        p.canvas.draw_points(PointMode::Polygon, &points, &line);
    }
    let dot = paint(accent, false, 1.0);
    for mark in marks.iter().filter(|m| m.dot) {
        let (x, y) = mark.center();
        p.canvas.draw_circle((x as f32, y as f32), 3.5, &dot);
    }
}

fn draw_pie(p: &Painter, place: Rect, groups: &[ChartShown]) {
    let look = p.look;
    for mark in marks(place, ChartShape::Pie, groups, None) {
        let color = mark.color.and_then(parse_color).unwrap_or_else(|| look.color("accent"));
        let fill = paint(color, false, 1.0);
        if mark.sweep >= 360.0 - 1e-9 {
            p.canvas.draw_oval(mark.place.sk(), &fill);
        } else {
            p.canvas.draw_arc(mark.place.sk(), mark.from as f32, mark.sweep as f32, true, &fill);
        }
    }
    // The legend beside the pie.
    let values: Vec<f64> = groups.iter().map(|g| g.value).collect();
    let left = place.x + pie_radius(place) * 2.0 + 28.0;
    let mut row = 0.0;
    for (group, color) in groups.iter().zip(slice_colors(&values)) {
        let Some(color) = color.and_then(parse_color) else { continue };
        let top = place.y + TOP + 16.0 * row;
        p.fill(Rect::new(left, top + 2.0, 10.0, 10.0), color, 0.0);
        let text = Rect::new(left + 16.0, top - 1.0, (place.right() - left - 16.0).max(0.0), 16.0);
        p.text(&group.label, text, look.color("text"), look.size("small"), "text", false, TextAlign::Left);
        row += 1.0;
    }
}

fn draw_tip(p: &Painter, place: Rect, tip: &str, at: (f64, f64)) {
    let look = p.look;
    let size = look.size("small");
    let pad = look.padding();
    let width = p.fonts.measure(tip, "text", size, false) + 2.0 * pad + 4.0;
    let height = size + 2.0 * pad + 4.0;
    let x = (at.0 + 12.0).clamp(place.x, place.x.max(place.right() - width));
    let y = if at.1 - height - 8.0 < place.y { at.1 + 16.0 } else { at.1 - height - 8.0 };
    let tip_place = Rect::new(x, y.min(place.y.max(place.bottom() - height)), width, height);
    let radius = look.radius("input");
    p.fill(tip_place, look.color("raised"), radius);
    p.stroke(tip_place, look.color("border"), look.border(), radius);
    p.text(tip, tip_place.inset(pad + 2.0), look.color("text"), size, "text", false, TextAlign::Left);
}

/// A number as a part's field or a value holds it: a number, or its text; none for anything else.
pub fn read_number(value: Option<&Value>) -> Option<f64> {
    let n = match value? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }?;
    n.is_finite().then_some(n)
}

/// The numbers of a list as a chart shows them: its last `points` (all for 0), each that is a number (the runtime's `views::numbers_of`).
pub fn numbers_shown(numbers: &[Value], points: usize) -> Vec<ChartShown> {
    let mut data = Map::new();
    data.insert("numbers".to_string(), Value::Array(numbers.to_vec()));
    clarity_runtime::views::numbers_of(&data, "numbers", points)
        .into_iter()
        .map(|v| ChartShown { label: String::new(), value: v, text: number_text(v) })
        .collect()
}

/// Each group's number: its rows counted, or a field summed or averaged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Measure {
    Count,
    Sum(String),
    Average(String),
}

pub struct ChartPart {
    base: PartBase,
    /// Where the pointer is over it, on the screen.
    pointer: Option<(f64, f64)>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(ChartPart { base: PartBase::new(spec, host), pointer: None })
}

/// How a chart groups by a value of its rows: what orders its groups.
fn kind_of(type_name: Option<&str>, keys: &[&Value]) -> &'static str {
    match type_name.map(str::to_ascii_lowercase).as_deref() {
        Some("enum") => return "enum",
        Some("bool" | "boolean") => return "bool",
        Some("date" | "datetime") => return "date",
        Some("integer" | "int" | "long" | "number" | "decimal" | "money" | "double" | "float") => return "number",
        Some("text" | "string") => return "text",
        _ => {}
    }
    let present: Vec<&&Value> = keys.iter().filter(|k| !k.is_null()).collect();
    if present.is_empty() {
        "text"
    } else if present.iter().all(|k| k.is_boolean()) {
        "bool"
    } else if present.iter().all(|k| k.is_number()) {
        "number"
    } else if present.iter().all(|k| k.as_str().is_some_and(is_date)) {
        "date"
    } else {
        "text"
    }
}

fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() >= 10 && b[4] == b'-' && b[7] == b'-' && b[..4].iter().chain(&b[5..7]).chain(&b[8..10]).all(u8::is_ascii_digit)
}

/// Whether two values name the same: `1` and `1.0` are one number.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        _ => a == b,
    }
}

/// A value as a label: text as it is, anything else as JSON writes it.
fn label_of(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

impl ChartPart {
    /// What it draws: `chart bar`, `line` or `pie`.
    pub fn shape(&self) -> ChartShape {
        match self.base.spec.text("chart") {
            Some("line") => ChartShape::Line,
            Some("pie") => ChartShape::Pie,
            _ => ChartShape::Bar,
        }
    }

    /// The field its rows are grouped by; none for a chart of a list of numbers.
    pub fn by(&self) -> Option<&str> {
        self.base.spec.text("by").filter(|b| !b.is_empty())
    }

    /// Each group's number, as `measure` writes it: `{"kind": "sum", "field": "total"}` or `sum total`.
    pub fn measure(&self) -> Measure {
        let (kind, field) = match self.base.spec.json("measure") {
            Some(Value::Object(o)) => (o.get("kind").and_then(Value::as_str).map(str::to_string), o.get("field").and_then(Value::as_str).map(str::to_string)),
            Some(Value::String(written)) => {
                let mut words = written.splitn(2, ' ');
                (words.next().map(str::to_string), words.next().map(str::to_string))
            }
            _ => (None, None),
        };
        match (kind.as_deref(), field) {
            (Some("sum"), Some(field)) => Measure::Sum(field),
            (Some("average"), Some(field)) => Measure::Average(field),
            _ => Measure::Count,
        }
    }

    /// The top of the axis, `max 100`; none lets the axis take in the largest number.
    pub fn max(&self) -> Option<f64> {
        read_number(self.base.spec.json("max"))
    }

    /// How many of the last numbers a chart of numbers shows; 0 shows them all.
    pub fn points(&self) -> usize {
        self.base.spec.number("points").unwrap_or(0.0).max(0.0) as usize
    }

    /// The groups it draws, as it shows them: its rows grouped, or its numbers.
    pub fn groups(&self) -> Vec<ChartShown> {
        let Some(bound) = self.base.bound() else { return Vec::new() };
        let host = &self.base.host;
        let rows = host.rows(bound);
        let Some(by) = self.by() else {
            return numbers_shown(&rows, self.points());
        };
        let measure = self.measure();
        let of = match &measure {
            Measure::Count => None,
            Measure::Sum(f) | Measure::Average(f) => Some(f.as_str()),
        };
        let by_path = format!("{bound}.{by}");
        let keys: Vec<&Value> = rows.iter().map(|r| r.get(by).unwrap_or(&Value::Null)).collect();
        let mut kind = kind_of(host.type_of(&by_path).as_deref(), &keys);
        let choices = host.choices(&by_path);
        // An enum's values are text in the window's data; a reference is a record's id.
        if !choices.is_empty() && kind == "text" && keys.iter().any(|k| k.is_string()) {
            kind = "enum";
        }
        // A reference groups, and orders, by the text of the record it names.
        let reference = !choices.is_empty() && kind != "enum" && kind != "bool";
        let named = |id: &Value| choices.iter().find(|c| same(&c.value, id)).map(|c| Value::from(c.label.clone())).unwrap_or(Value::Null);
        let chart_rows: Vec<ChartRow> = rows
            .iter()
            .map(|r| {
                let key = r.get(by).cloned().unwrap_or(Value::Null);
                ChartRow { by: if reference { named(&key) } else { key }, of: of.and_then(|f| r.get(f).cloned()).unwrap_or(Value::Null) }
            })
            .collect();
        let order: Vec<Value> = if kind == "enum" { choices.iter().map(|c| c.value.clone()).collect() } else { Vec::new() };
        let measure_word = match measure {
            Measure::Count => "count",
            Measure::Sum(_) => "sum",
            Measure::Average(_) => "average",
        };
        let formats = formats_of(host.as_ref());
        let decimal = FieldFormat::new(FormatKind::Decimal);
        let of_format = of.and_then(|f| format_of(host.as_ref(), &rows::column_path(bound, f)));
        charts::groups(&chart_rows, if reference { "text" } else { kind }, measure_word, &order)
            .into_iter()
            .map(|g| {
                let label = match (&g.key, reference) {
                    (Value::String(s), true) => s.clone(),
                    (key, _) => match choices.iter().find(|c| same(&c.value, key)) {
                        Some(choice) => choice.label.clone(),
                        None => label_of(key),
                    },
                };
                // A sum or an average shows as a decimal in the person's format, or in the format of its field.
                let text = match measure {
                    Measure::Count => number_text(g.value),
                    _ => match serde_json::Number::from_f64(round12(g.value)) {
                        Some(n) => rows::shown_formatted(&Value::Number(n), of_format.as_ref().or(Some(&decimal)), &formats),
                        None => number_text(g.value),
                    },
                };
                ChartShown { label, value: g.value, text }
            })
            .collect()
    }

    /// Where it draws each group in `place`.
    pub fn marks(&self, place: Rect) -> Vec<ChartMark> {
        marks(place, self.shape(), &self.groups(), self.max())
    }

    /// What the group at a point of the screen tells; none where there is none.
    pub fn tip_at(&self, x: f64, y: f64) -> Option<String> {
        tip_at(self.base.node.bounds, self.shape(), &self.groups(), self.max(), x, y)
    }

    /// What the group the pointer is on tells, while the pointer is over it.
    pub fn tip(&self) -> Option<String> {
        let (x, y) = self.pointer.filter(|_| self.base.node.hovered)?;
        self.tip_at(x, y)
    }
}

impl Element for ChartPart {
    part_element!();

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pointer = Some((at.screen_x, at.screen_y));
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        let pointer = self.pointer.filter(|_| self.base.node.hovered);
        draw(p, self.base.node.bounds, self.shape(), &self.groups(), self.max(), pointer);
    }
}

impl Part for ChartPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn grows(&self) -> bool {
        true
    }
}
