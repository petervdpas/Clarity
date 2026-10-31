//! Canvas modes (DESIGN 5.31), decided here in the runtime: the screen sends
//! what the person did on a canvas (an object dropped, a stroke painted, a
//! shape added, changed, moved or deleted) and the runtime changes the
//! window's data, snapped to the canvas's grid, and chooses what it touched.
//! A vector drawing is kept as SVG text in one plain form, which every runtime
//! writes and reads alike (conformance/canvas-modes.json).

use crate::reorder::same_id;
use crate::ui::{Reply, WindowState, data_at, set_data_at};
use crate::{Db, Error, Result};
use serde::Deserialize;
use serde_json::{Map, Value};

/// A canvas's mode and the mode's parameters, as a `canvas_event` carries them.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct CanvasArgs {
    pub part: String,
    pub mode: String,
    /// objects: the data path of the list, and its x and y fields.
    pub list: String,
    pub at: Option<[String; 2]>,
    /// paint and vector: the data path of the field.
    pub path: String,
    /// vector: the drawing's width and height; 1000 by 1000 without.
    pub units: Option<[i64; 2]>,
    pub grid: i64,
    pub snap: bool,
    pub readonly: bool,
}

/// A drawing's units when the canvas names none.
const DEFAULT_UNITS: [i64; 2] = [1000, 1000];

fn number(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) if !s.trim().is_empty() => s.trim().parse().ok(),
        _ => None,
    }
}

/// A whole number, rounded half away from zero.
fn whole(n: f64) -> i64 {
    n.round() as i64
}

type Shape = Map<String, Value>;

/// Applies the person's `action` on a canvas to `state`, and says whether it
/// changed anything; an action that changes nothing, or names no object or
/// shape, is refused (false), as every action on a readonly canvas.
pub fn apply(state: &mut WindowState, a: &CanvasArgs, action: &Map<String, Value>) -> Result<bool> {
    if a.readonly {
        return Ok(false);
    }
    let place = |v: f64| if a.snap && a.grid > 0 { whole(v / a.grid as f64) * a.grid } else { whole(v) };
    let unfit = |name: &str| Error::Invalid(format!("a canvas's {name} does not fit: {}", action.get(name).unwrap_or(&Value::Null)));
    match a.mode.as_str() {
        "objects" => {
            let mv = action.get("move").ok_or_else(|| Error::Invalid("an object is moved".into()))?;
            let [fx, fy] = a.at.clone().ok_or_else(|| Error::Invalid("a canvas of objects names their x and y fields, as at: [\"x\", \"y\"]".into()))?;
            let (x, y) = match (number(mv.get("x")), number(mv.get("y"))) {
                (Some(x), Some(y)) => (place(x), place(y)),
                _ => return Err(unfit("move")),
            };
            let id = mv.get("id").cloned().unwrap_or(Value::Null);
            let mut list = match data_at(&state.data, &a.list) {
                Some(Value::Array(list)) => list.clone(),
                _ => return Err(Error::Invalid(format!("the window has no list {}", a.list))),
            };
            let Some((at, record)) = list.iter_mut().enumerate().filter_map(|(i, r)| Some((i, r.as_object_mut()?))).find(|(_, r)| same_id(r.get("id").unwrap_or(&Value::Null), &id)) else { return Ok(false) };
            if number(record.get(&fx)) == Some(x as f64) && number(record.get(&fy)) == Some(y as f64) {
                return Ok(false);
            }
            record.insert(fx, Value::from(x));
            record.insert(fy, Value::from(y));
            set_data_at(&mut state.data, &a.list, Value::Array(list))?;
            // Chosen as a table's row is: by its place in the list.
            state.selected.insert(a.part.clone(), at);
            Ok(true)
        }
        "paint" => {
            // The picture after the stroke, as an image field holds a new one (conformance/pictures.json).
            let picture = action.get("stroke").ok_or_else(|| Error::Invalid("a painted canvas is stroked".into()))?;
            if data_at(&state.data, &a.path) == Some(picture) {
                return Ok(false);
            }
            // A stroke that is no picture is refused: the record never holds it.
            if !is_picture(picture) {
                return Ok(false);
            }
            set_data_at(&mut state.data, &a.path, picture.clone())?;
            state.selected.remove(&a.part);
            Ok(true)
        }
        "vector" => {
            let text = data_at(&state.data, &a.path).and_then(Value::as_str).unwrap_or_default().to_string();
            let (units, mut shapes) = drawing_shapes(&text, a.units.unwrap_or(DEFAULT_UNITS));
            let index = |v: Option<&Value>| v.and_then(Value::as_i64).filter(|i| *i >= 1 && *i as usize <= shapes.len()).map(|i| i as usize);
            let selected = if let Some(shape) = action.get("add") {
                shapes.push(fit_shape(shape.as_object().ok_or_else(|| unfit("add"))?, &place)?);
                Some(shapes.len())
            } else if let Some(change) = action.get("change") {
                let Some(at) = index(change.get("index")) else { return Ok(false) };
                shapes[at - 1] = fit_shape(change.get("shape").and_then(Value::as_object).ok_or_else(|| unfit("change"))?, &place)?;
                Some(at)
            } else if let Some(mv) = action.get("move") {
                let Some(at) = index(mv.get("index")) else { return Ok(false) };
                let (Some(x), Some(y)) = (number(mv.get("x")), number(mv.get("y"))) else { return Err(unfit("move")) };
                if !moved_shape(&mut shapes[at - 1], place(x), place(y)) {
                    return Ok(false);
                }
                Some(at)
            } else if let Some(delete) = action.get("delete") {
                if !delete.is_i64() && !delete.is_u64() {
                    return Err(unfit("delete"));
                }
                let Some(at) = index(Some(delete)) else { return Ok(false) };
                shapes.remove(at - 1);
                None
            } else {
                return Err(Error::Invalid("a vector canvas adds, changes, moves or deletes a shape".into()));
            };
            let drawn = drawing_text(units, &shapes);
            if drawn == text {
                return Ok(false);
            }
            set_data_at(&mut state.data, &a.path, Value::from(drawn))?;
            // The shape's place in the drawing, from 0, as a table's chosen row.
            match selected {
                Some(at) => state.selected.insert(a.part.clone(), at - 1),
                None => state.selected.remove(&a.part),
            };
            Ok(true)
        }
        other => Err(Error::Invalid(format!("a canvas's mode is objects, paint or vector, not {other:?}"))),
    }
}

/// `canvas_event` (DESIGN 5.31): the person's action on the canvas `a.part`.
/// A change runs `run`, the handler, as an edit ends (`edits::ended`); a
/// refused action answers the window as it is (`draw`), saying `refused`.
pub fn event(
    db: &Db,
    mut state: WindowState,
    a: &CanvasArgs,
    action: &Map<String, Value>,
    scope: Option<&str>,
    run: impl FnOnce(WindowState) -> Result<Reply>,
    draw: impl FnOnce(WindowState) -> Result<Reply>,
) -> Result<Reply> {
    let before = state.clone();
    if !apply(&mut state, a, action)? {
        let mut reply = draw(before)?;
        reply.refused = true;
        return Ok(reply);
    }
    crate::edits::ended(db, state, &a.part, scope, run)
}

/// The numbers of each shape a vector canvas keeps, and which of them are places.
fn shape_fields(op: &str) -> Option<(&'static [&'static str], &'static [&'static str])> {
    Some(match op {
        "rect" => (&["x", "y", "width", "height"], &["x", "y"]),
        "line" => (&["x1", "y1", "x2", "y2", "width"], &["x1", "y1", "x2", "y2"]),
        "ellipse" => (&["x", "y", "rx", "ry"], &["x", "y"]),
        "path" => (&["width"], &[]),
        "text" => (&["x", "y", "size"], &["x", "y"]),
        _ => return None,
    })
}

/// What a shape's left-out number is (conformance/drawing.json).
fn shape_default(op: &str, name: &str) -> f64 {
    match (op, name) {
        ("line" | "path", "width") => 1.0,
        ("text", "size") => 10.0,
        _ => 0.0,
    }
}

/// A shape as a vector canvas keeps it: whole numbers, places snapped.
fn fit_shape(shape: &Shape, place: &impl Fn(f64) -> i64) -> Result<Shape> {
    let op = shape.get("op").and_then(Value::as_str).unwrap_or_default();
    let (numbers, places) = shape_fields(op).ok_or_else(|| Error::Invalid(format!("a vector canvas keeps rect, line, ellipse, path and text, not {op:?}")))?;
    let mut out = Shape::new();
    out.insert("op".into(), Value::from(op));
    for name in numbers {
        let n = number(shape.get(*name)).unwrap_or_else(|| shape_default(op, name));
        out.insert(name.to_string(), Value::from(if places.contains(name) { place(n) } else { whole(n) }));
    }
    out.insert("color".into(), Value::from(shape.get("color").and_then(Value::as_str).unwrap_or_default()));
    if op == "path" {
        let points = shape.get("points").and_then(Value::as_array).cloned().unwrap_or_default();
        out.insert("points".into(), points.iter().map(|p| Value::from(place(number(Some(p)).unwrap_or(0.0)))).collect());
    }
    if op == "text" {
        out.insert("text".into(), Value::from(shape.get("text").and_then(Value::as_str).unwrap_or_default()));
    }
    Ok(out)
}

/// Moves `shape` so its first place is at `x`, `y`, the rest with it; false
/// when it is there already or is a line the runtime does not read.
fn moved_shape(shape: &mut Shape, x: i64, y: i64) -> bool {
    let num = |shape: &Shape, name: &str| number(shape.get(name)).unwrap_or(0.0) as i64;
    let op = shape.get("op").and_then(Value::as_str).unwrap_or_default().to_string();
    let (dx, dy) = match op.as_str() {
        "rect" | "ellipse" | "text" => {
            let d = (x - num(shape, "x"), y - num(shape, "y"));
            shape.insert("x".into(), Value::from(x));
            shape.insert("y".into(), Value::from(y));
            d
        }
        "line" => {
            let d = (x - num(shape, "x1"), y - num(shape, "y1"));
            let (x2, y2) = (num(shape, "x2") + d.0, num(shape, "y2") + d.1);
            for (name, v) in [("x1", x), ("y1", y), ("x2", x2), ("y2", y2)] {
                shape.insert(name.into(), Value::from(v));
            }
            d
        }
        "path" => {
            let Some(points) = shape.get_mut("points").and_then(Value::as_array_mut).filter(|p| p.len() >= 2) else { return false };
            let first = |p: &Value| number(Some(p)).unwrap_or(0.0) as i64;
            let d = (x - first(&points[0]), y - first(&points[1]));
            for (i, p) in points.iter_mut().enumerate() {
                *p = Value::from(first(p) + if i % 2 == 0 { d.0 } else { d.1 });
            }
            d
        }
        _ => return false,
    };
    dx != 0 || dy != 0
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn unescaped(text: &str) -> String {
    text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&amp;", "&")
}

fn svg_head(units: [i64; 2]) -> String {
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\">", units[0], units[1])
}

/// A drawing's SVG text: its units and its shapes, one element on a line.
pub fn drawing_text(units: [i64; 2], shapes: &[Shape]) -> String {
    let mut out = svg_head(units) + "\n";
    for shape in shapes {
        out.push_str(&shape_line(shape));
        out.push('\n');
    }
    out.push_str("</svg>\n");
    out
}

fn shape_line(shape: &Shape) -> String {
    let n = |name: &str| whole(number(shape.get(name)).unwrap_or(0.0)).to_string();
    let text = |name: &str| escaped(shape.get(name).and_then(Value::as_str).unwrap_or_default());
    match shape.get("op").and_then(Value::as_str).unwrap_or_default() {
        "rect" => format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>", n("x"), n("y"), n("width"), n("height"), text("color")),
        "line" => format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>", n("x1"), n("y1"), n("x2"), n("y2"), text("color"), n("width")),
        "ellipse" => format!("<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\"/>", n("x"), n("y"), n("rx"), n("ry"), text("color")),
        "path" => {
            let points = shape.get("points").and_then(Value::as_array).cloned().unwrap_or_default();
            let pairs: Vec<String> = points.chunks_exact(2).map(|p| format!("{},{}", whole(number(Some(&p[0])).unwrap_or(0.0)), whole(number(Some(&p[1])).unwrap_or(0.0)))).collect();
            format!("<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>", pairs.join(" "), text("color"), n("width"))
        }
        "text" => format!("<text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"{}\">{}</text>", n("x"), n("y"), text("color"), n("size"), text("text")),
        _ => shape.get("text").and_then(Value::as_str).unwrap_or_default().to_string(),
    }
}

/// Reads a drawing's SVG text: its units (`units` when it names none) and
/// its shapes. A line it does not read as a shape is kept as it is.
pub fn drawing_shapes(text: &str, units: [i64; 2]) -> ([i64; 2], Vec<Shape>) {
    let mut units = units;
    let mut shapes = vec![];
    for line in text.split('\n') {
        if line.trim().is_empty() || line == "</svg>" {
            continue;
        }
        if line.starts_with("<svg") {
            if let Some(read) = read_units(line) {
                units = read;
            }
            continue;
        }
        shapes.push(read_shape(line));
    }
    (units, shapes)
}

fn read_units(line: &str) -> Option<[i64; 2]> {
    let rest = line.strip_prefix("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 ")?.strip_suffix("\">")?;
    let (w, h) = rest.split_once(' ')?;
    Some([w.parse().ok()?, h.parse().ok()?])
}

/// The shape of one line of a drawing: written back, it must be the line
/// again, or the line is kept as it is.
fn read_shape(line: &str) -> Shape {
    let mut kept = Shape::new();
    kept.insert("op".into(), Value::from("svg"));
    kept.insert("text".into(), Value::from(line));
    let Some((name, attrs, content)) = svg_element(line) else { return kept };
    let attr = |a: &str| attrs.iter().find(|(k, _)| k == a).map(|(_, v)| v.as_str());
    let mut shape = Shape::new();
    let pick = |shape: &mut Shape, pairs: &[(&str, &str)]| {
        pairs.iter().all(|(from, to)| match attr(from).and_then(|v| v.parse::<i64>().ok()) {
            Some(n) => {
                shape.insert(to.to_string(), Value::from(n));
                true
            }
            None => false,
        })
    };
    let color = |a: &str| Value::from(attr(a).unwrap_or_default());
    let fits = match name.as_str() {
        "rect" => {
            shape.insert("op".into(), Value::from("rect"));
            shape.insert("color".into(), color("fill"));
            pick(&mut shape, &[("x", "x"), ("y", "y"), ("width", "width"), ("height", "height")])
        }
        "line" => {
            shape.insert("op".into(), Value::from("line"));
            shape.insert("color".into(), color("stroke"));
            pick(&mut shape, &[("x1", "x1"), ("y1", "y1"), ("x2", "x2"), ("y2", "y2"), ("stroke-width", "width")])
        }
        "ellipse" => {
            shape.insert("op".into(), Value::from("ellipse"));
            shape.insert("color".into(), color("fill"));
            pick(&mut shape, &[("cx", "x"), ("cy", "y"), ("rx", "rx"), ("ry", "ry")])
        }
        "polyline" => {
            shape.insert("op".into(), Value::from("path"));
            shape.insert("color".into(), color("stroke"));
            let mut fits = pick(&mut shape, &[("stroke-width", "width")]);
            let mut points = vec![];
            for pair in attr("points").unwrap_or_default().split_whitespace() {
                match pair.split_once(',').and_then(|(x, y)| Some((x.parse::<i64>().ok()?, y.parse::<i64>().ok()?))) {
                    Some((x, y)) => points.extend([Value::from(x), Value::from(y)]),
                    None => fits = false,
                }
            }
            shape.insert("points".into(), Value::Array(points));
            fits
        }
        "text" => {
            shape.insert("op".into(), Value::from("text"));
            shape.insert("color".into(), color("fill"));
            shape.insert("text".into(), Value::from(unescaped(&content)));
            pick(&mut shape, &[("x", "x"), ("y", "y"), ("font-size", "size")])
        }
        _ => false,
    };
    if !fits || shape_line(&shape) != line {
        return kept;
    }
    shape
}

/// Reads `<name a="1" b="2"/>` or `<name a="1">content</name>`.
fn svg_element(line: &str) -> Option<(String, Vec<(String, String)>, String)> {
    let rest = line.strip_prefix('<')?;
    let end = rest.find([' ', '/', '>'])?;
    if end == 0 {
        return None;
    }
    let name = rest[..end].to_string();
    let mut rest = &rest[end..];
    let mut attrs = vec![];
    loop {
        rest = rest.trim_start_matches(' ');
        if rest.starts_with("/>") {
            return (rest == "/>").then_some((name, attrs, String::new()));
        }
        if let Some(inner) = rest.strip_prefix('>') {
            let content = inner.strip_suffix(&format!("</{name}>"))?;
            return Some((name, attrs, content.to_string()));
        }
        let eq = rest.find("=\"").filter(|eq| *eq > 0)?;
        let close = rest[eq + 2..].find('"')?;
        attrs.push((rest[..eq].to_string(), unescaped(&rest[eq + 2..eq + 2 + close])));
        rest = &rest[eq + 2 + close + 1..];
    }
}

/// Whether a stroke is a new picture: `{data: base64 of a picture's bytes}`.
fn is_picture(value: &Value) -> bool {
    use base64::Engine;
    value
        .get("data")
        .and_then(Value::as_str)
        .and_then(|data| base64::engine::general_purpose::STANDARD.decode(data).ok())
        .is_some_and(|bytes| crate::picture::kind_of(&bytes).is_some())
}
