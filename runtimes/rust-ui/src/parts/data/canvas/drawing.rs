//! A vector drawing's SVG text read into its shapes, as every runtime
//! reads it (conformance/canvas-modes.json, `svg` and `read`). The canvas
//! reads the drawing it shows here, so its program needs to give only the
//! text.

use serde_json::{Map, Value};

/// One shape of a drawing: `op` and its numbers, colors and text.
pub type Shape = Map<String, Value>;

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

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn unescaped(text: &str) -> String {
    text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&amp;", "&")
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
