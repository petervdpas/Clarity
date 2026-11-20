//! A report (`reports/<Name>.report`, docs/ledger/reports-pdf.md, step 6): XML
//! naming its paper, the data it takes and its bands, each band's parts at
//! their places in millimetres. Laid out with the data (JSON, by the Data's
//! names) and the texts (JSON, by key) as Typst code: the page header and
//! footer on every page, the report header once at the start, the detail
//! once per item of its list, the report footer once at the end. Every text
//! goes in as a Typst string, so nothing in the data is read as Typst.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde_json::Value;

use crate::style::{self, Stylesheet};
use crate::typst_out;

/// The barcodes an Image draws.
pub(crate) const BARCODES: [&str; 3] = ["qr", "code128", "ean13"];

/// A barcode of `value` as SVG: a QR code, Code 128 (any text of its character set) or EAN-13 (12 or 13 digits).
fn barcode(kind: &str, value: &str) -> Result<String, String> {
    use barcoders::generators::svg::SVG;
    let drawn = |encoded: Vec<u8>| SVG::new(80).generate(&encoded).map_err(|e| format!("the {kind} barcode of '{value}' cannot be drawn: {e}"));
    match kind {
        "qr" => {
            let code = qrcode::QrCode::new(value.as_bytes()).map_err(|e| format!("'{value}' makes no qr code: {e}"))?;
            Ok(code.render::<qrcode::render::svg::Color>().quiet_zone(false).min_dimensions(200, 200).build())
        }
        // Code set B: letters, digits and signs.
        "code128" => drawn(barcoders::sym::code128::Code128::new(format!("\u{0181}{value}")).map_err(|e| format!("'{value}' makes no code128 barcode: {e}"))?.encode()),
        "ean13" => drawn(barcoders::sym::ean13::EAN13::new(value).map_err(|e| format!("'{value}' makes no ean13 barcode (12 or 13 digits): {e}"))?.encode()),
        other => Err(format!("an Image draws no barcode '{other}': it draws {}", BARCODES.join(", "))),
    }
}

/// A value shown as its field's type says (`extras.types`, by the bind as
/// the report writes it) in the language and currency given, or as its
/// `format` says; none for a value shown as it is (a text, a field of no
/// type given and no format).
fn shown(extras: &crate::ReportExtras, bind: &str, value: &Value, format: Option<&str>) -> Option<String> {
    use clarity_runtime::shown::{FieldFormat, FormatKind, Formats};
    if matches!(value, Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_)) {
        return None;
    }
    let formats = Formats::new(extras.language.as_deref(), extras.currency.as_deref());
    let typed = extras.types.get(bind).and_then(|t| FieldFormat::parse(t));
    match (format, typed) {
        (Some(format), Some(typed)) => Some(formats.show_as(typed.kind, value, format)),
        (Some(format), None) => {
            // No type given: a date by its look, else a number.
            let kind = if value.as_str().and_then(clarity_runtime::shown::Moment::parse).is_some() { FormatKind::Date } else { FormatKind::Decimal };
            Some(formats.show_as(kind, value, format))
        }
        (None, Some(typed)) => Some(formats.show(&typed, value)),
        (None, None) => None,
    }
}

/// A picture held in a record's JSON: base64, with or without a `data:` start.
fn decoded(text: &str) -> Option<Vec<u8>> {
    use base64::Engine as _;
    let text = text.split_once(";base64,").map_or(text, |(_, b)| b);
    base64::engine::general_purpose::STANDARD.decode(text.trim()).ok()
}

/// What a band is, by its element's name.
#[derive(Debug, Clone, PartialEq)]
enum BandKind {
    ReportHeader,
    PageHeader,
    Detail { list: String },
    /// Prints before the detail's items where the value it groups by changes.
    GroupHeader { by: String },
    /// Prints after a group's items; its sums add that group.
    GroupFooter { by: String },
    ReportFooter,
    PageFooter,
}

#[derive(Debug, Clone, PartialEq)]
enum PartKind {
    Text { key: String },
    Field { bind: String },
    Sum { of: String },
    Markdown { bind: String },
    PageNumber,
    /// A picture of the program's resources (`source`) or a record's image field (`bind`).
    Image { source: Option<String>, bind: Option<String>, barcode: Option<String> },
    /// A line from its place to the far corner of its size; a height of 0 is a level line.
    Line,
    /// A frame round its place and size, filled when its class has a background.
    Box,
}

#[derive(Debug, Clone, PartialEq)]
struct Part {
    kind: PartKind,
    at: (f64, f64),
    size: (f64, f64),
    align: String,
    /// The class of the report's stylesheet it takes its look from.
    class: Option<String>,
    /// How a Field or Sum shows its value instead of as its type says: `0.00`, `dd-MM-yyyy`, `percent`.
    format: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct Band {
    kind: BandKind,
    height: f64,
    parts: Vec<Part>,
}

/// A report as its file says it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Report {
    paper: (f64, f64),
    margin: f64,
    /// The data it takes: its name and its type.
    data: Vec<(String, String)>,
    bands: Vec<Band>,
    /// `headerFirst`: the report header tops the first page, the page header
    /// following it there and topping every later page (true, as written
    /// nothing); false puts the page header above the report header on page 1 too.
    header_first: bool,
}

fn attribute<'a>(node: roxmltree::Node<'a, '_>, name: &str) -> Result<&'a str, String> {
    node.attribute(name).ok_or_else(|| format!("{} needs '{name}'", node.tag_name().name()))
}

fn millimetres(node: roxmltree::Node, name: &str, text: &str) -> Result<f64, String> {
    style::length(text).and_then(style::mm).or_else(|| text.trim().parse().ok()).ok_or_else(|| format!("'{name}' of {} is no length: '{text}'", node.tag_name().name()))
}

/// Two numbers in millimetres, `x,y` or `width,height`.
fn pair(node: roxmltree::Node, name: &str) -> Result<(f64, f64), String> {
    let text = attribute(node, name)?;
    let numbers: Vec<f64> = text.split(',').map(|n| n.trim().parse::<f64>()).collect::<Result<_, _>>().map_err(|_| format!("'{name}' of {} is two numbers in mm, as in \"10,20\": '{text}'", node.tag_name().name()))?;
    match numbers.as_slice() {
        [a, b] => Ok((*a, *b)),
        _ => Err(format!("'{name}' of {} is two numbers in mm, as in \"10,20\": '{text}'", node.tag_name().name())),
    }
}

impl Report {
    /// Reads a report's file.
    pub(crate) fn read(xml: &str) -> Result<Report, String> {
        let document = roxmltree::Document::parse(xml).map_err(|e| format!("the report is no XML: {e}"))?;
        let root = document.root_element();
        if root.tag_name().name() != "Report" {
            return Err(format!("a report starts with <Report>, not <{}>", root.tag_name().name()));
        }
        let paper = root.attribute("paper").map_or(Some((210.0, 297.0)), style::paper).ok_or_else(|| format!("there is no paper '{}'", root.attribute("paper").unwrap_or_default()))?;
        let margin = root.attribute("margin").map_or(Ok(20.0), |m| millimetres(root, "margin", m))?;
        let header_first = match root.attribute("headerFirst") {
            None | Some("true") => true,
            Some("false") => false,
            Some(other) => return Err(format!("'headerFirst' of Report is true or false, not '{other}'")),
        };
        let mut data = Vec::new();
        let mut bands = Vec::new();
        for node in root.children().filter(roxmltree::Node::is_element) {
            let name = node.tag_name().name();
            if name == "Data" {
                data.push((attribute(node, "name")?.to_string(), attribute(node, "type")?.to_string()));
                continue;
            }
            let kind = match name {
                "ReportHeader" => BandKind::ReportHeader,
                "PageHeader" => BandKind::PageHeader,
                "Detail" => BandKind::Detail { list: attribute(node, "list")?.to_string() },
                "GroupHeader" => BandKind::GroupHeader { by: attribute(node, "by")?.to_string() },
                "GroupFooter" => BandKind::GroupFooter { by: attribute(node, "by")?.to_string() },
                "ReportFooter" => BandKind::ReportFooter,
                "PageFooter" => BandKind::PageFooter,
                other => return Err(format!("a report has no '{other}': its parts are Data, ReportHeader, PageHeader, GroupHeader, Detail, GroupFooter, ReportFooter and PageFooter")),
            };
            let height = millimetres(node, "height", attribute(node, "height")?)?;
            let mut parts = Vec::new();
            for part in node.children().filter(roxmltree::Node::is_element) {
                let kind = match part.tag_name().name() {
                    "Text" => PartKind::Text { key: attribute(part, "key")?.to_string() },
                    "Field" => PartKind::Field { bind: attribute(part, "bind")?.to_string() },
                    "Sum" => PartKind::Sum { of: attribute(part, "of")?.to_string() },
                    "Markdown" => PartKind::Markdown { bind: attribute(part, "bind")?.to_string() },
                    "PageNumber" => PartKind::PageNumber,
                    "Line" => PartKind::Line,
                    "Box" => PartKind::Box,
                    "Image" => match (part.attribute("source"), part.attribute("bind")) {
                        (None, None) => return Err("Image needs 'source', a picture of the resources, or 'bind', an image field".into()),
                        (source, bind) => PartKind::Image { source: source.map(str::to_string), bind: bind.map(str::to_string), barcode: part.attribute("barcode").map(str::to_string) },
                    },
                    other => return Err(format!("a band has no '{other}': its parts are Text, Field, Sum, Markdown, PageNumber, Image, Line and Box")),
                };
                let align = part.attribute("align").unwrap_or("left");
                if !["left", "center", "right"].contains(&align) {
                    return Err(format!("'align' of {} is left, center or right, not '{align}'", part.tag_name().name()));
                }
                let class = part.attribute("class").map(str::to_string);
                let format = part.attribute("format").map(str::to_string);
                parts.push(Part { kind, at: pair(part, "at")?, size: pair(part, "size")?, align: align.to_string(), class, format });
            }
            bands.push(Band { kind, height, parts });
        }
        Ok(Report { paper, margin, data, bands, header_first })
    }

    fn band(&self, kind: fn(&BandKind) -> bool) -> Option<&Band> {
        self.bands.iter().find(|b| kind(&b.kind))
    }

    /// The Markdown texts the report shows, to know its formulas.
    pub(crate) fn markdown_texts(&self, data: &Value) -> String {
        let mut all = String::new();
        for band in &self.bands {
            for part in &band.parts {
                let PartKind::Markdown { bind } = &part.kind else { continue };
                match &band.kind {
                    BandKind::Detail { list } => {
                        for item in self.items(data, list).unwrap_or_default() {
                            all.push_str(&self.text(data, Some(item), bind).unwrap_or_default());
                            all.push('\n');
                        }
                    }
                    _ => {
                        all.push_str(&self.text(data, None, bind).unwrap_or_default());
                        all.push('\n');
                    }
                }
            }
        }
        all
    }

    /// The value a bind names: the first name is the report's Data, or (in a detail) its item.
    fn value<'v>(&self, data: &'v Value, item: Option<&'v Value>, bind: &str) -> Result<&'v Value, String> {
        let mut names = bind.split('.');
        let first = names.next().unwrap_or_default();
        let mut at = if self.data.iter().any(|(n, _)| n == first) {
            data.get(first).ok_or_else(|| format!("the data '{first}' the report takes is not given"))?
        } else {
            item.ok_or_else(|| format!("'{bind}' names no data of the report: it takes {}", self.data.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", ")))?
        };
        for name in names {
            at = at.get(name).ok_or_else(|| format!("'{bind}' has no '{name}'"))?;
        }
        Ok(at)
    }

    fn items<'v>(&self, data: &'v Value, list: &str) -> Result<&'v [Value], String> {
        match self.value(data, None, list)? {
            Value::Array(items) => Ok(items),
            Value::Null => Ok(&[]),
            _ => Err(format!("the detail's list '{list}' is no list")),
        }
    }

    /// A bound value as the text a part shows.
    fn text(&self, data: &Value, item: Option<&Value>, bind: &str) -> Result<String, String> {
        Ok(match self.value(data, item, bind)? {
            Value::Null => String::new(),
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => return Err(format!("a field shows one value; '{bind}' holds more")),
        })
    }

    /// A text with its named slots (`{order.number}`) filled from the report's data, each value shown as its type says; a slot that names nothing stays as written.
    fn filled(&self, text: &str, data: &Value, item: Option<&Value>, extras: &crate::ReportExtras) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let Some(close) = rest[open..].find('}') else { break };
            let bind = &rest[open + 1..open + close];
            match self.value(data, item, bind) {
                Ok(value) => out.push_str(&shown(extras, bind, value, None).unwrap_or_else(|| self.text(data, item, bind).unwrap_or_default())),
                Err(_) => out.push_str(&rest[open..=open + close]),
            }
            rest = &rest[open + close + 1..];
        }
        out.push_str(rest);
        out
    }

    /// The sum of a field over the detail's items, exactly, with as many decimals as the most precise.
    fn sum(&self, data: &Value, of: &str, scope: Option<&[Value]>) -> Result<String, String> {
        let Some(Band { kind: BandKind::Detail { list }, .. }) = self.band(|k| matches!(k, BandKind::Detail { .. })) else {
            return Err(format!("the sum of '{of}' needs a Detail band, whose items it adds"));
        };
        let field = of.split_once('.').map_or(of, |(_, rest)| rest);
        let mut parts = Vec::new();
        // A group footer's sum adds its group; any other sum adds every item.
        let items = match scope {
            Some(items) => items,
            None => self.items(data, list)?,
        };
        for item in items {
            let value = match item.pointer(&format!("/{}", field.replace('.', "/"))) {
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::String(s)) => s.clone(),
                Some(Value::Null) | None => continue,
                Some(_) => return Err(format!("the sum of '{of}' adds numbers")),
            };
            parts.push(decimal(&value).ok_or_else(|| format!("the sum of '{of}' adds numbers, not '{value}'"))?);
        }
        let scale = parts.iter().map(|(_, s)| *s).max().unwrap_or(0);
        let total: i128 = parts.iter().map(|(m, s)| m * 10i128.pow(scale - s)).sum();
        Ok(decimal_text(total, scale))
    }

    /// The Typst source of the report laid out with its data and texts.
    pub(crate) fn source(&self, data: &Value, texts: &Value, sheet: &Stylesheet, broken: &BTreeSet<String>, extras: &crate::ReportExtras, files: &RefCell<Vec<Vec<u8>>>) -> Result<String, String> {
        for (name, _) in &self.data {
            if data.get(name).is_none() {
                return Err(format!("the data '{name}' the report takes is not given"));
            }
        }
        let mut out = typst_out::math_start(!typst_out::formulas(&self.markdown_texts(data)).is_empty());
        let header = self.band(|k| *k == BandKind::PageHeader);
        let footer = self.band(|k| *k == BandKind::PageFooter);
        let m = self.margin;
        let (top, bottom) = (m + header.map_or(0.0, |b| b.height), m + footer.map_or(0.0, |b| b.height));
        let _ = write!(out, "#set page(width: {}mm, height: {}mm, margin: (top: {top}mm, bottom: {bottom}mm, left: {m}mm, right: {m}mm), header-ascent: 0pt, footer-descent: 0pt", self.paper.0, self.paper.1);
        // With headerFirst the report header tops page 1, so the page header there follows it in the body instead.
        let first = self.header_first && self.band(|k| *k == BandKind::ReportHeader).is_some();
        if let Some(band) = header {
            let code = self.band_code(band, data, None, None, texts, sheet, broken, extras, files)?;
            if first {
                let _ = write!(out, ", header: context {{ if counter(page).get().first() > 1 {{ {code} }} }}");
            } else {
                let _ = write!(out, ", header: {code}");
            }
        }
        if let Some(band) = footer {
            let _ = write!(out, ", footer: {}", self.band_code(band, data, None, None, texts, sheet, broken, extras, files)?);
        }
        out.push_str(")\n");
        typst_out::looks(&mut out, sheet);
        // Page 1's header room, which its page header does not take, is the report header's.
        if let (true, Some(band)) = (first, header) {
            let _ = writeln!(out, "#v(-{}mm, weak: false)", band.height);
        }
        for band in &self.bands {
            match &band.kind {
                // Group bands print with the detail's items.
                BandKind::PageHeader | BandKind::PageFooter | BandKind::GroupHeader { .. } | BandKind::GroupFooter { .. } => {}
                BandKind::Detail { list } => {
                    let items = self.items(data, list)?;
                    // The groups, outermost first, each with its footer.
                    let groups: Vec<(&Band, Option<&Band>, &str)> = self
                        .bands
                        .iter()
                        .filter_map(|b| match &b.kind {
                            BandKind::GroupHeader { by } => Some((b, self.bands.iter().find(|f| matches!(&f.kind, BandKind::GroupFooter { by: other } if other == by)), by.as_str())),
                            _ => None,
                        })
                        .collect();
                    let key = |item: &Value, by: &str| self.text(data, Some(item), by).unwrap_or_default();
                    let mut starts = vec![0usize; groups.len()];
                    let footers = |out: &mut String, from: usize, end: usize, starts: &[usize]| -> Result<(), String> {
                        for level in (from..groups.len()).rev() {
                            if let (Some(footer), Some(last)) = (groups[level].1, items.get(end.wrapping_sub(1))) {
                                let scope = &items[starts[level]..end];
                                let _ = writeln!(out, "#{}", self.band_code(footer, data, Some(last), Some(scope), texts, sheet, broken, extras, files)?);
                            }
                        }
                        Ok(())
                    };
                    for (i, item) in items.iter().enumerate() {
                        // The outermost group whose value changed here; every group inside it starts again.
                        let changed = if i == 0 { Some(0) } else { (0..groups.len()).find(|&l| key(item, groups[l].2) != key(&items[i - 1], groups[l].2)) };
                        if let Some(level) = changed {
                            if i > 0 {
                                footers(&mut out, level, i, &starts)?;
                            }
                            for l in level..groups.len() {
                                starts[l] = i;
                                let _ = writeln!(out, "#{}", self.band_code(groups[l].0, data, Some(item), None, texts, sheet, broken, extras, files)?);
                            }
                        }
                        let _ = writeln!(out, "#{}", self.band_code(band, data, Some(item), None, texts, sheet, broken, extras, files)?);
                    }
                    if !items.is_empty() {
                        footers(&mut out, 0, items.len(), &starts)?;
                    }
                }
                _ => {
                    let _ = writeln!(out, "#{}", self.band_code(band, data, None, None, texts, sheet, broken, extras, files)?);
                    // On page 1 the page header follows the report header (headerFirst).
                    if let (true, BandKind::ReportHeader, Some(page_header)) = (first, &band.kind, header) {
                        let _ = writeln!(out, "#{}", self.band_code(page_header, data, None, None, texts, sheet, broken, extras, files)?);
                    }
                }
            }
        }
        Ok(out)
    }

    /// A band as one block of its height, its parts placed in it.
    #[allow(clippy::too_many_arguments)]
    fn band_code(&self, band: &Band, data: &Value, item: Option<&Value>, scope: Option<&[Value]>, texts: &Value, sheet: &Stylesheet, broken: &BTreeSet<String>, extras: &crate::ReportExtras, files: &RefCell<Vec<Vec<u8>>>) -> Result<String, String> {
        let mut placed = Vec::new();
        for part in &band.parts {
            let content = match &part.kind {
                PartKind::Text { key } => format!("#{};", typst_out::string(&self.filled(texts.get(key).and_then(Value::as_str).unwrap_or(key), data, item, extras))),
                PartKind::Field { bind } => format!("#{};", typst_out::string(&shown(extras, bind, self.value(data, item, bind)?, part.format.as_deref()).map_or_else(|| self.text(data, item, bind), Ok)?)),
                PartKind::Sum { of } => {
                    let total = Value::String(self.sum(data, of, scope)?);
                    format!("#{};", typst_out::string(&shown(extras, of, &total, part.format.as_deref()).unwrap_or_else(|| total.as_str().unwrap_or_default().to_string())))
                }
                PartKind::Markdown { bind } => typst_out::blocks(&self.text(data, item, bind)?, sheet, broken),
                PartKind::PageNumber => "#context counter(page).display();".to_string(),
                PartKind::Line | PartKind::Box => String::new(),
                PartKind::Image { source, bind, barcode: Some(kind) } => {
                    // A barcode: a picture made from its value, stretched to its box (a QR code kept square).
                    let value = match bind {
                        Some(bind) => self.text(data, item, bind)?,
                        None => source.clone().unwrap_or_default(),
                    };
                    if value.is_empty() {
                        String::new()
                    } else {
                        let svg = barcode(kind, &value)?;
                        let mut kept = files.borrow_mut();
                        kept.push(svg.into_bytes());
                        let fit = if kind == "qr" { "contain" } else { "stretch" };
                        format!("#image(read(\"{}{}\", encoding: none), format: \"svg\", width: 100%, height: 100%, fit: \"{fit}\");", crate::PICTURES, kept.len() - 1)
                    }
                }
                PartKind::Image { source, bind, barcode: None } => {
                    let bytes = match (source, bind) {
                        (Some(source), _) => extras.images.get(source).cloned().ok_or_else(|| format!("the program carries no picture {source}"))?,
                        (None, Some(bind)) => match self.value(data, item, bind)? {
                            Value::String(text) => decoded(text).ok_or_else(|| format!("'{bind}' holds no picture"))?,
                            // No picture: nothing in its place.
                            Value::Null => Vec::new(),
                            _ => return Err(format!("'{bind}' holds no picture")),
                        },
                        (None, None) => Vec::new(),
                    };
                    if bytes.is_empty() {
                        String::new()
                    } else {
                        // Each picture is a file of its own that Typst reads as bytes; it fits its box.
                        let mut kept = files.borrow_mut();
                        kept.push(bytes);
                        format!("#image(read(\"{}{}\", encoding: none), width: 100%, height: 100%, fit: \"contain\");", crate::PICTURES, kept.len() - 1)
                    }
                }
            };
            let (x, y) = part.at;
            let (w, h) = part.size;
            // A line or a box is drawn, not written: its class gives its colour and its fill.
            if matches!(part.kind, PartKind::Line | PartKind::Box) {
                let style = match &part.class {
                    Some(name) => Some(sheet.class(name).ok_or_else(|| format!("the report's stylesheet has no class '{name}' (.{name} {{ ... }})"))?),
                    None => None,
                };
                let ink = style.and_then(|s| s.color).or_else(|| sheet.style(crate::style::Element::Body).color).map_or("black".to_string(), |c| c.typst());
                let stroke = format!("0.75pt + {ink}");
                if part.kind == PartKind::Line {
                    placed.push(format!("place(top + left, dx: {x}mm, dy: {y}mm, line(start: (0mm, 0mm), end: ({w}mm, {h}mm), stroke: {stroke}))"));
                } else {
                    let fill = style.and_then(|s| s.background).map(|c| format!(", fill: {}", c.typst())).unwrap_or_default();
                    placed.push(format!("place(top + left, dx: {x}mm, dy: {y}mm, rect(width: {w}mm, height: {h}mm, stroke: {stroke}{fill}))"));
                }
                continue;
            }
            // Its class's look: the text set first, a background as the box's fill.
            let (look, fill) = match &part.class {
                Some(name) => {
                    let style = sheet.class(name).ok_or_else(|| format!("the report's stylesheet has no class '{name}' (.{name} {{ ... }})"))?;
                    (typst_out::settings(style), style.background.map(|c| format!(", fill: {}", c.typst())).unwrap_or_default())
                }
                None => (String::new(), String::new()),
            };
            placed.push(format!("place(top + left, dx: {x}mm, dy: {y}mm, box(width: {w}mm, height: {h}mm, clip: true{fill}, align({})[{look}{content}]))", part.align));
        }
        Ok(format!("block(width: 100%, height: {}mm, breakable: false, {{ {} }})", band.height, placed.join("\n")))
    }
}

/// A number's digits without its point, and how many decimals it had: "2.25" is (225, 2).
fn decimal(text: &str) -> Option<(i128, u32)> {
    let text = text.trim();
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if fraction.contains(|c: char| !c.is_ascii_digit()) || fraction.len() > 18 {
        return None;
    }
    let negative = whole.starts_with('-');
    let digits = whole.trim_start_matches(['-', '+']);
    if digits.is_empty() && fraction.is_empty() || digits.contains(|c: char| !c.is_ascii_digit()) {
        return None;
    }
    let mantissa: i128 = format!("{digits}{fraction}").parse().ok()?;
    Some((if negative { -mantissa } else { mantissa }, fraction.len() as u32))
}

fn decimal_text(total: i128, scale: u32) -> String {
    if scale == 0 {
        return total.to_string();
    }
    let sign = if total < 0 { "-" } else { "" };
    let digits = format!("{:0>width$}", total.unsigned_abs(), width = scale as usize + 1);
    let (whole, fraction) = digits.split_at(digits.len() - scale as usize);
    format!("{sign}{whole}.{fraction}")
}
