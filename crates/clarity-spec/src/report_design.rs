//! A report's file (`reports/<Name>.report`, docs/ledger/reports-pdf.md) as
//! the Reports tab's designer and Clarity's report tools change it: read
//! into its words, data, bands and parts, and written back as the same XML,
//! the bands in the order they print, lengths in millimetres. The designer
//! holds this model too: the IDE reads and writes it here (`to_json`,
//! `from_json`), as JSON whose words keep the order they are written in.
//! Pinned by conformance/report-design.json.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::xml::{self, Element};

/// Words (`attrs`) as a JSON object in the order they are written, both ways.
mod in_order {
    use serde::de::{MapAccess, Visitor};
    use serde::ser::SerializeMap;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(words: &[(String, String)], s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(words.len()))?;
        for (name, value) in words {
            map.serialize_entry(name, value)?;
        }
        map.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<(String, String)>, D::Error> {
        struct Words;
        impl<'de> Visitor<'de> for Words {
            type Value = Vec<(String, String)>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("words by name")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(entry) = map.next_entry::<String, String>()? {
                    out.push(entry);
                }
                Ok(out)
            }
        }
        d.deserialize_map(Words)
    }
}

/// A report's Data (name and type) as a JSON object each, both ways.
mod data_items {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct Item {
        name: String,
        #[serde(rename = "type")]
        ty: String,
    }

    pub fn serialize<S: Serializer>(data: &[(String, String)], s: S) -> Result<S::Ok, S::Error> {
        data.iter().map(|(name, ty)| Item { name: name.clone(), ty: ty.clone() }).collect::<Vec<_>>().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<(String, String)>, D::Error> {
        Ok(Vec::<Item>::deserialize(d)?.into_iter().map(|i| (i.name, i.ty)).collect())
    }
}

/// The bands in the order they print; a report has several group headers and footers, one of each per group.
pub const PRINT_ORDER: [&str; 7] = ["PageHeader", "ReportHeader", "GroupHeader", "Detail", "GroupFooter", "ReportFooter", "PageFooter"];
/// The bands a report may have more than one of.
pub const REPEATED: [&str; 2] = ["GroupHeader", "GroupFooter"];
/// The paper sizes a report names.
pub const PAPERS: [&str; 5] = ["A3", "A4", "A5", "Letter", "Legal"];

/// A part on a band: its kind, its place and size in millimetres within the
/// band, and its own words (key, bind, of, source, align, class) in the order written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReportPart {
    pub kind: String,
    pub at: (f64, f64),
    pub size: (f64, f64),
    #[serde(with = "in_order")]
    #[ts(type = "Record<string, string>")]
    pub attrs: Vec<(String, String)>,
}

/// A band: its kind, its height in millimetres, its own words (a group's `by`, a detail's `list`), its parts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReportBand {
    pub kind: String,
    pub height: f64,
    #[serde(with = "in_order")]
    #[ts(type = "Record<string, string>")]
    pub attrs: Vec<(String, String)>,
    pub parts: Vec<ReportPart>,
}

/// A report: its words (name, paper, margin, stylesheet, headerFirst), the data it takes (name and type), its bands.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReportDesign {
    #[serde(with = "in_order")]
    #[ts(type = "Record<string, string>")]
    pub attrs: Vec<(String, String)>,
    #[serde(with = "data_items")]
    #[ts(type = "{ name: string; type: string }[]")]
    pub data: Vec<(String, String)>,
    pub bands: Vec<ReportBand>,
}

/// The design as the IDE's designer holds it: JSON, its words in the order written.
pub fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// The design the IDE's designer sends, read in the order its words are written.
pub fn from_json(text: &str) -> Result<ReportDesign, String> {
    serde_json::from_str(text).map_err(|e| format!("the report's design does not read: {e}"))
}

/// The value of the word `name` among `attrs`.
pub fn word<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

/// `attrs` with the word `name` set to `value`, in its place, or at the end when new; an empty value takes it out.
pub fn set_word(attrs: &mut Vec<(String, String)>, name: &str, value: &str) {
    match attrs.iter().position(|(k, _)| k == name) {
        Some(at) if value.is_empty() => {
            attrs.remove(at);
        }
        Some(at) => attrs[at].1 = value.to_string(),
        None if !value.is_empty() => attrs.push((name.to_string(), value.to_string())),
        None => {}
    }
}

fn pair(text: Option<&str>, fallback: (f64, f64)) -> (f64, f64) {
    let parts: Vec<Option<f64>> = text.unwrap_or_default().split(',').map(|n| n.trim().parse::<f64>().ok().filter(|v| v.is_finite())).collect();
    match parts.as_slice() {
        [Some(a), Some(b)] => (*a, *b),
        _ => fallback,
    }
}

/// Millimetres of a length as a report writes it: `20mm`, `2cm`, `1in`, `12pt`, or a plain number.
pub fn mm(text: Option<&str>, fallback: f64) -> f64 {
    let text = text.unwrap_or_default().trim();
    let digits = text.trim_start_matches('-').chars().take_while(|c| c.is_ascii_digit() || *c == '.').count() + usize::from(text.starts_with('-'));
    let (number, unit) = text.split_at(digits);
    let Ok(n) = number.parse::<f64>() else { return fallback };
    match unit.trim() {
        "" | "mm" => n,
        "cm" => n * 10.0,
        "in" => n * 25.4,
        "pt" => n * 25.4 / 72.0,
        _ => fallback,
    }
}

fn words(element: &Element, leaving: &[&str]) -> Vec<(String, String)> {
    element.attributes.iter().filter(|a| !leaving.contains(&a.name.as_str())).map(|a| (a.name.clone(), a.value.clone())).collect()
}

/// A report's text as the designer holds it, or why it cannot be read.
pub fn read(text: &str) -> Result<ReportDesign, String> {
    let root = xml::read(text).map_err(|d| d.message)?;
    if root.name != "Report" {
        return Err(format!("a report starts with <Report>, not <{}>", root.name));
    }
    let mut design = ReportDesign { attrs: words(&root, &[]), ..Default::default() };
    for child in &root.children {
        if child.name == "Data" {
            design.data.push((child.get("name").unwrap_or_default().to_string(), child.get("type").unwrap_or_default().to_string()));
            continue;
        }
        if !PRINT_ORDER.contains(&child.name.as_str()) {
            return Err(format!("a report has no '{}'", child.name));
        }
        let parts = child
            .children
            .iter()
            .map(|p| {
                if !crate::reports::PARTS.contains(&p.name.as_str()) {
                    return Err(format!("a band has no '{}'", p.name));
                }
                Ok(ReportPart { kind: p.name.clone(), at: pair(p.get("at"), (0.0, 0.0)), size: pair(p.get("size"), (40.0, 6.0)), attrs: words(p, &["at", "size"]) })
            })
            .collect::<Result<Vec<_>, String>>()?;
        design.bands.push(ReportBand { kind: child.name.clone(), height: mm(child.get("height"), 10.0), attrs: words(child, &["height"]), parts });
    }
    Ok(design)
}

/// A length kept to whole tenths of a millimetre and never below 0, as the designer snaps it.
pub fn snapped(n: f64) -> f64 {
    ((n * 10.0).round() / 10.0).max(0.0)
}

/// A length kept to tenths of a millimetre, written without a needless `.0`.
pub fn written(n: f64) -> String {
    let n = (n * 10.0).round() / 10.0;
    if n == n.trunc() { format!("{}", n as i64) } else { format!("{n}") }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn attributes(attrs: &[(String, String)]) -> String {
    attrs.iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| format!(" {k}=\"{}\"", escape(v))).collect()
}

/// The report as its file's text, the bands in the order they print.
pub fn write(design: &ReportDesign) -> String {
    let mut lines = vec![format!("<Report{}>", attributes(&design.attrs))];
    for (name, ty) in &design.data {
        lines.push(format!("  <Data name=\"{}\" type=\"{}\" />", escape(name), escape(ty)));
    }
    let mut ordered: Vec<&ReportBand> = design.bands.iter().collect();
    ordered.sort_by_key(|b| PRINT_ORDER.iter().position(|k| *k == b.kind).unwrap_or(PRINT_ORDER.len()));
    for band in ordered {
        let head = format!("  <{} height=\"{}mm\"{}", band.kind, written(band.height), attributes(&band.attrs));
        if band.parts.is_empty() {
            lines.push(format!("{head} />"));
            continue;
        }
        lines.push(format!("{head}>"));
        for p in &band.parts {
            lines.push(format!("    <{} at=\"{},{}\" size=\"{},{}\"{} />", p.kind, written(p.at.0), written(p.at.1), written(p.size.0), written(p.size.1), attributes(&p.attrs)));
        }
        lines.push(format!("  </{}>", band.kind));
    }
    lines.push("</Report>".into());
    lines.push(String::new());
    lines.join("\n")
}

/// What a new part of `kind` starts with, at a place in its band; none for a kind a band has not.
pub fn new_part(kind: &str, at: (f64, f64)) -> Option<ReportPart> {
    let words: &[(&str, &str)] = match kind {
        "Text" => &[("key", "")],
        "Field" | "Markdown" => &[("bind", "")],
        "Sum" => &[("of", ""), ("align", "right")],
        "Image" => &[("source", "")],
        "Line" | "Box" => &[],
        "PageNumber" => &[("align", "center")],
        _ => return None,
    };
    let size = match kind {
        "Markdown" => (120.0, 20.0),
        "PageNumber" => (10.0, 5.0),
        "Image" => (40.0, 20.0),
        "Line" => (60.0, 0.0),
        "Box" => (60.0, 20.0),
        _ => (40.0, 6.0),
    };
    Some(ReportPart { kind: kind.to_string(), at, size, attrs: words.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() })
}

/// Where the rule of `selector` is in `css`: the start of its line's
/// whitespace, the end of its `{`, and its `}`; the first one.
fn rule_of(css: &str, selector: &str) -> Option<(usize, usize, usize)> {
    let starts = std::iter::once(0).chain(css.match_indices('\n').map(|(i, _)| i + 1));
    for start in starts {
        let rest = &css[start..];
        let lead = rest.len() - rest.trim_start().len();
        let Some(after) = rest[lead..].strip_prefix(selector) else { continue };
        let gap = after.len() - after.trim_start().len();
        if !after[gap..].starts_with('{') {
            continue;
        }
        let open = start + lead + selector.len() + gap + 1;
        let close = open + css[open..].find('}')?;
        return Some((start, open, close));
    }
    None
}

/// A stylesheet's text with one declaration of a selector set (`.title {
/// font-size: 18pt }`), or taken out when `value` is empty; the rest kept as written.
pub fn with_declaration(css: &str, selector: &str, property: &str, value: &str) -> String {
    let Some((_, open, close)) = rule_of(css, selector) else {
        if value.is_empty() {
            return css.to_string();
        }
        let sep = if css.is_empty() || css.ends_with('\n') { "" } else { "\n" };
        return format!("{css}{sep}{selector} {{ {property}: {value}; }}\n");
    };
    let mut decls: Vec<String> = css[open..close].split(';').map(str::trim).filter(|d| !d.is_empty() && d.split(':').next().unwrap_or_default().trim() != property).map(str::to_string).collect();
    if !value.is_empty() {
        decls.push(format!("{property}: {value}"));
    }
    let body = if decls.is_empty() { " ".to_string() } else { format!(" {}; ", decls.join("; ")) };
    format!("{}{body}{}", &css[..open], &css[close..])
}

/// The classes a stylesheet gives (`.title` as `title`) with their declarations, in the order written.
pub fn classes(css: &str) -> Vec<(String, Vec<(String, String)>)> {
    rules(css).into_iter().filter_map(|(selector, decls)| selector.strip_prefix('.').map(|name| (name.to_string(), decls))).collect()
}

/// The rules a stylesheet gives by selector (`.title`, `body`), with their
/// declarations, a later rule of the same selector setting over an earlier one.
pub fn rules(css: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut found: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut text = css.to_string();
    while let Some(start) = text.find("/*") {
        let end = text[start..].find("*/").map_or(text.len(), |e| start + e + 2);
        text.replace_range(start..end, "");
    }
    let mut rest = text.as_str();
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|c| open + c) else { break };
        let decls: Vec<(String, String)> = rest[open + 1..close]
            .split(';')
            .filter_map(|d| d.split_once(':').map(|(k, v)| (k.trim().to_string(), v.trim().to_string())))
            .filter(|(k, _)| !k.is_empty())
            .collect();
        for name in rest[..open].split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match found.iter_mut().find(|(n, _)| n == name) {
                Some((_, kept)) => {
                    for (k, v) in &decls {
                        set_word(kept, k, v);
                    }
                }
                None => found.push((name.to_string(), decls.clone())),
            }
        }
        rest = &rest[close + 1..];
    }
    found
}
