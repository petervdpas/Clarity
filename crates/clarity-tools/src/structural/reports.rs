//! The reports' tools (docs/ledger/reports-pdf.md, Agent tools for reports):
//! every gesture of the Reports tab's designer, so an agent makes and
//! changes a report as a person does. A report is `reports/<Name>.report`
//! in the project's folder, its classes the rules of the stylesheet it names
//! there. Each change is read and written as the designer does
//! (`clarity_spec::report_design`), refused when the report would check
//! worse (`clarity_spec::reports::check`), and kept for Undo. Parts have no
//! names: one is named by its band and its place among the band's parts.

use std::path::{Path, PathBuf};

use clarity_agent::ToolDef;
use clarity_spec::report_design::{self, ReportBand, ReportDesign, ReportPart, PRINT_ORDER, REPEATED};
use clarity_spec::reports::{PARTS, REPORTS_DIR, REPORT_EXTENSION};
use clarity_spec::Severity;
use serde_json::{Value, json};

use super::{schema, tool};
use crate::{ProjectTools, Written};

/// The properties of a class the designer sets (ReportLook.svelte).
pub const CLASS_PROPERTIES: [&str; 7] = ["font-size", "font-weight", "font-style", "font-family", "color", "background-color", "text-align"];
/// A part's own words.
const PART_WORDS: [&str; 6] = ["key", "bind", "of", "source", "align", "class"];

pub fn definitions() -> Vec<ToolDef> {
    let text = |what: &str| json!({ "type": "string", "description": what });
    let report = || text("The report, by name (its file reports/<Name>.report).");
    let band = || json!({ "type": "string", "enum": PRINT_ORDER, "description": "The band, by kind." });
    let which = || json!({ "type": "integer", "minimum": 1, "description": "For a GroupHeader or GroupFooter: which one, counted from 1 in the order they print. Left out: the first." });
    let pair = |what: &str| json!({ "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2, "description": what });
    let part_words = |changing: bool| {
        let mut props = serde_json::Map::new();
        let none = if changing { " null takes it away." } else { "" };
        props.insert("key".into(), json!({ "type": ["string", "null"], "description": format!("A Text's translation key, like invoiceTitle.{none}") }));
        props.insert("bind".into(), json!({ "type": ["string", "null"], "description": format!("A Field's, Markdown's or Image's value, through the report's data, like order.number.{none}") }));
        props.insert("of".into(), json!({ "type": ["string", "null"], "description": format!("A Sum's number field of the detail's list, like line.total.{none}") }));
        props.insert("source".into(), json!({ "type": ["string", "null"], "description": format!("An Image's file among the resources.{none}") }));
        props.insert("align".into(), json!({ "type": ["string", "null"], "enum": ["left", "center", "right", null], "description": format!("How its text lines up.{none}") }));
        props.insert("class".into(), json!({ "type": ["string", "null"], "description": format!("Its class in the report's stylesheet (set_report_class), like title.{none}") }));
        props
    };
    let mut add_part = part_words(false);
    add_part.insert("report".into(), report());
    add_part.insert("band".into(), band());
    add_part.insert("which".into(), which());
    add_part.insert("kind".into(), json!({ "type": "string", "enum": PARTS, "description": "The part's kind." }));
    add_part.insert("at".into(), pair("Its place in the band, in millimetres from the band's top left: [x, y]."));
    add_part.insert("size".into(), pair("Its width and height in millimetres. Left out: the kind's starting size."));
    let mut change_part = part_words(true);
    change_part.insert("report".into(), report());
    change_part.insert("band".into(), band());
    change_part.insert("which".into(), which());
    change_part.insert("part".into(), json!({ "type": "integer", "minimum": 1, "description": "The part, by its place among the band's parts, counted from 1 (report_view lists them)." }));
    change_part.insert("at".into(), pair("Its new place in the band, in millimetres: [x, y]."));
    change_part.insert("size".into(), pair("Its new width and height in millimetres."));
    vec![
        tool("report_view", "A report as the Reports tab shows it: its paper, margin and headerFirst, the data it takes, its bands in the order they print with each part by its place, and the classes of its stylesheet. Without a report: the reports there are.", schema(json!({ "report": report() }), &[])),
        tool(
            "add_report",
            "Adds a report (reports/<Name>.report), as the Reports tab's New report: A4 paper, a report header, a page footer with the page's number, and the data it takes.",
            schema(
                json!({
                    "name": text("The new report's name, like Invoice."),
                    "data": { "type": "array", "items": { "type": "object", "properties": { "name": text("What the report calls it, like order."), "type": text("Its entity, like Order.") }, "required": ["name", "type"] }, "description": "The records it takes, each a name and an entity." }
                }),
                &["name"],
            ),
        ),
        tool(
            "change_report",
            "Changes a report's paper, margin or headerFirst; what is not given stays.",
            schema(
                json!({
                    "report": report(),
                    "paper": { "type": "string", "description": format!("One of {}, with `landscape` after it to turn it, like \"A4 landscape\".", report_design::PAPERS.join(", ")) },
                    "margin": text("The page margin, like 20mm."),
                    "headerFirst": { "type": "boolean", "description": "Whether the report header prints before the page header on the first page (true, the start) or after it." }
                }),
                &["report"],
            ),
        ),
        tool(
            "add_band",
            "Adds a band to a report. A report has one of each band, but several group headers and footers (one of each per group, added after the ones there).",
            schema(
                json!({ "report": report(), "band": band(), "height": { "type": "number", "description": "Its height in millimetres. Left out: 10." }, "by": text("A GroupHeader's or GroupFooter's field it groups by, like line.category."), "list": text("A Detail's list it prints a row of, like order.lines.") }),
                &["report", "band"],
            ),
        ),
        tool(
            "change_band",
            "Changes a band's height, a group's `by` or a detail's `list`; what is not given stays.",
            schema(json!({ "report": report(), "band": band(), "which": which(), "height": { "type": "number", "description": "Its height in millimetres." }, "by": text("A group's field."), "list": text("A Detail's list.") }), &["report", "band"]),
        ),
        tool("remove_band", "Removes a band of a report, with its parts.", schema(json!({ "report": report(), "band": band(), "which": which() }), &["report", "band"])),
        tool("add_report_part", "Adds a part to a band of a report, at a place, with the kind's starting words and the ones given.", schema(Value::Object(add_part), &["report", "band", "kind", "at"])),
        tool("change_report_part", "Changes a part of a band: its place, size or words; what is not given stays.", schema(Value::Object(change_part), &["report", "band", "part"])),
        tool(
            "remove_report_part",
            "Removes a part of a band.",
            schema(json!({ "report": report(), "band": band(), "which": which(), "part": { "type": "integer", "minimum": 1, "description": "The part, by its place among the band's parts, counted from 1." } }), &["report", "band", "part"]),
        ),
        tool(
            "set_report_class",
            "Sets one property of a class in a report's stylesheet (`.title { font-size: 18pt }`), as the Reports tab's look does; an empty value takes it away. A report without a stylesheet gets reports/<Name>.css.",
            schema(
                json!({
                    "report": report(),
                    "class": text("The class, without its dot, like title."),
                    "property": { "type": "string", "enum": CLASS_PROPERTIES },
                    "value": text("Its value, like 18pt, bold, italic, serif, #1d2330, center; empty to take it away.")
                }),
                &["report", "class", "property", "value"],
            ),
        ),
    ]
}

pub fn call(tools: &mut ProjectTools, name: &str, input: &Value) -> Option<Result<String, String>> {
    let tool = match name {
        "report_view" => view,
        "add_report" => add_report,
        "change_report" => change_report,
        "add_band" => add_band,
        "change_band" => change_band,
        "remove_band" => remove_band,
        "add_report_part" => add_part,
        "change_report_part" => change_part,
        "remove_report_part" => remove_part,
        "set_report_class" => set_class,
        _ => return None,
    };
    Some(tool(tools, input))
}

fn text(input: &Value, key: &str) -> String {
    input[key].as_str().unwrap_or_default().trim().to_string()
}

fn needed(input: &Value, key: &str) -> Result<String, String> {
    let value = text(input, key);
    if value.is_empty() { Err(format!("give {key}")) } else { Ok(value) }
}

fn folder(tools: &ProjectTools) -> Result<PathBuf, String> {
    let (_, dir) = tools.open()?;
    Ok(dir.join(REPORTS_DIR))
}

fn report_file(tools: &ProjectTools, name: &str) -> Result<PathBuf, String> {
    let file = folder(tools)?.join(format!("{name}.{REPORT_EXTENSION}"));
    if file.is_file() { Ok(file) } else { Err(format!("there is no report {name}: {}", names(tools)?)) }
}

/// The reports there are, as a sentence.
fn names(tools: &ProjectTools) -> Result<String, String> {
    let (_, dir) = tools.open()?;
    let found: Vec<String> = clarity_spec::reports::report_files(dir).map_err(|e| e.to_string())?.into_iter().map(|f| f.name).collect();
    Ok(if found.is_empty() { "the project has no reports; add_report makes one".to_string() } else { format!("the reports are {}", found.join(", ")) })
}

fn opened(tools: &ProjectTools, input: &Value) -> Result<(String, PathBuf, ReportDesign), String> {
    let name = needed(input, "report")?;
    let file = report_file(tools, &name)?;
    let text = std::fs::read_to_string(&file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let design = report_design::read(&text).map_err(|why| format!("{} cannot be read: {why}", file.display()))?;
    Ok((name, file, design))
}

/// The report's errors as the checker says them.
fn errors_in(dir: &Path, file: &Path) -> Result<Vec<String>, String> {
    let project = clarity_project::load_modules(dir).map_err(|e| format!("cannot read the project: {e}"))?;
    let path = file.display().to_string();
    Ok(project.diagnostics.iter().filter(|d| d.severity == Severity::Error && d.path == path).map(|d| d.message.clone()).collect())
}

/// Writes the files of one change (`(file, text)`), refused and taken back
/// when the report `file` would check worse; kept for Undo.
fn saved(tools: &mut ProjectTools, report: &Path, files: Vec<(PathBuf, String)>, done: String) -> Result<String, String> {
    let (_, dir) = tools.open()?.clone();
    let before_errors = if report.is_file() { errors_in(&dir, report)? } else { Vec::new() };
    let kept: Vec<(PathBuf, Option<String>)> = files.iter().map(|(f, _)| (f.clone(), std::fs::read_to_string(f).ok())).collect();
    for (file, text) in &files {
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))?;
        }
        crate::files::write_atomically(file, text)?;
    }
    let after_errors = errors_in(&dir, report)?;
    let gained: Vec<&String> = after_errors.iter().filter(|e| !before_errors.contains(e)).collect();
    if !gained.is_empty() {
        for (file, before) in &kept {
            match before {
                Some(text) => crate::files::write_atomically(file, text)?,
                None => {
                    let _ = std::fs::remove_file(file);
                }
            }
        }
        return Err(format!("refused, nothing written: the report would gain {}", gained.iter().map(|e| e.as_str()).collect::<Vec<_>>().join("; ")));
    }
    for ((file, text), (_, before)) in files.into_iter().zip(kept) {
        tools.written.push(Written { file, before, after: Some(text) });
    }
    Ok(format!("{done}. {}", tools.problems()?))
}

fn save(tools: &mut ProjectTools, file: &Path, design: &ReportDesign, done: String) -> Result<String, String> {
    saved(tools, file, vec![(file.to_path_buf(), report_design::write(design))], done)
}

fn band_kind(input: &Value) -> Result<String, String> {
    let band = needed(input, "band")?;
    if PRINT_ORDER.contains(&band.as_str()) { Ok(band) } else { Err(format!("a report has no band {band}: one of {}", PRINT_ORDER.join(", "))) }
}

/// The place in `design.bands` of the band asked for (`band`, `which`).
fn band_at(design: &ReportDesign, input: &Value) -> Result<usize, String> {
    let kind = band_kind(input)?;
    let which = input["which"].as_u64().unwrap_or(1).max(1) as usize;
    let of_kind: Vec<usize> = design.bands.iter().enumerate().filter(|(_, b)| b.kind == kind).map(|(i, _)| i).collect();
    match of_kind.get(which - 1) {
        Some(at) => Ok(*at),
        None if of_kind.is_empty() => Err(format!("the report has no {kind}: add_band adds it")),
        None => Err(format!("the report has {} {kind}s, not {which}", of_kind.len())),
    }
}

fn band_name(band: &ReportBand, which: usize) -> String {
    if REPEATED.contains(&band.kind.as_str()) { format!("{} {which}", band.kind) } else { band.kind.clone() }
}

fn pair(input: &Value, key: &str) -> Result<Option<(f64, f64)>, String> {
    match &input[key] {
        Value::Null => Ok(None),
        Value::Array(items) if items.len() == 2 => match (items[0].as_f64(), items[1].as_f64()) {
            (Some(a), Some(b)) if a >= 0.0 && b >= 0.0 => Ok(Some((report_design::snapped(a), report_design::snapped(b)))),
            _ => Err(format!("{key} is two numbers of millimetres, 0 or more")),
        },
        _ => Err(format!("{key} is two numbers: [x, y] or [width, height]")),
    }
}

fn set_words(attrs: &mut Vec<(String, String)>, input: &Value, words: &[&str]) {
    for word in words {
        match &input[*word] {
            Value::String(value) => report_design::set_word(attrs, word, value.trim()),
            Value::Null if input.get(*word).is_some() => report_design::set_word(attrs, word, ""),
            _ => {}
        }
    }
}

fn view(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    if text(input, "report").is_empty() {
        return Ok(format!("{}.", names(tools)?));
    }
    let (name, file, design) = opened(tools, input)?;
    let attrs = &design.attrs;
    let get = |k: &str| report_design::word(attrs, k).unwrap_or_default().to_string();
    let mut out = format!("Report {name} ({}): paper {}, margin {}, headerFirst {}", file.display(), or(get("paper"), "A4"), or(get("margin"), "20mm"), or(get("headerFirst"), "true"));
    out.push_str(&format!(", stylesheet {}.\n", or(get("stylesheet"), "none")));
    let data: Vec<String> = design.data.iter().map(|(n, t)| format!("{n}: {t}")).collect();
    out.push_str(&format!("Data: {}.\n", if data.is_empty() { "none".to_string() } else { data.join(", ") }));
    let mut ordered: Vec<&ReportBand> = design.bands.iter().collect();
    ordered.sort_by_key(|b| PRINT_ORDER.iter().position(|k| *k == b.kind));
    let mut seen: Vec<String> = Vec::new();
    for band in ordered {
        seen.push(band.kind.clone());
        let which = seen.iter().filter(|k| **k == band.kind).count();
        let words: String = band.attrs.iter().map(|(k, v)| format!(", {k} {v}")).collect();
        out.push_str(&format!("\n{} (height {}mm{words}):\n", band_name(band, which), report_design::written(band.height)));
        if band.parts.is_empty() {
            out.push_str("  no parts\n");
        }
        for (i, p) in band.parts.iter().enumerate() {
            let words: String = p.attrs.iter().map(|(k, v)| format!(", {k} {}", if v.is_empty() { "(empty)" } else { v })).collect();
            out.push_str(&format!(
                "  part {}: {} at {},{} size {},{}{words}\n",
                i + 1,
                p.kind,
                report_design::written(p.at.0),
                report_design::written(p.at.1),
                report_design::written(p.size.0),
                report_design::written(p.size.1)
            ));
        }
    }
    let sheet = report_design::word(attrs, "stylesheet").and_then(|s| std::fs::read_to_string(file.with_file_name(s)).ok());
    let classes = sheet.as_deref().map(report_design::classes).unwrap_or_default();
    out.push_str("\nClasses:");
    if classes.is_empty() {
        out.push_str(" none (set_report_class makes them)");
    }
    for (class, decls) in classes {
        out.push_str(&format!("\n  .{class} {{ {} }}", decls.iter().map(|(k, v)| format!("{k}: {v};")).collect::<Vec<_>>().join(" ")));
    }
    out.push('\n');
    Ok(out)
}

fn or(value: String, fallback: &str) -> String {
    if value.is_empty() { fallback.to_string() } else { value }
}

fn add_report(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let name = needed(input, "name")?;
    if !name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!("'{name}' is no report's name: letters and digits, starting with a capital, like Invoice"));
    }
    let file = folder(tools)?.join(format!("{name}.{REPORT_EXTENSION}"));
    if file.exists() {
        return Err(format!("there is a report {name} already"));
    }
    let mut design = report_design::read(&clarity_spec::reports::starter(&name))?;
    if let Some(items) = input["data"].as_array() {
        for item in items {
            let (data, ty) = (text(item, "name"), text(item, "type"));
            if data.is_empty() || ty.is_empty() {
                return Err("each data has a name and a type (an entity)".into());
            }
            design.data.push((data, ty));
        }
    }
    save(tools, &file, &design, format!("Added report {name} ({})", file.display()))
}

fn change_report(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (name, file, mut design) = opened(tools, input)?;
    if let Some(paper) = input["paper"].as_str() {
        let words: Vec<&str> = paper.split_whitespace().collect();
        let known = words.iter().all(|w| report_design::PAPERS.contains(w) || ["landscape", "portrait"].contains(w)) && words.iter().filter(|w| report_design::PAPERS.contains(w)).count() == 1;
        if !known {
            return Err(format!("'{paper}' is no paper: one of {}, with landscape after it to turn it", report_design::PAPERS.join(", ")));
        }
        report_design::set_word(&mut design.attrs, "paper", &words.join(" "));
    }
    if let Some(margin) = input["margin"].as_str() {
        if report_design::mm(Some(margin), -1.0) < 0.0 {
            return Err(format!("'{margin}' is no length: like 20mm, 2cm, 1in or 12pt"));
        }
        report_design::set_word(&mut design.attrs, "margin", margin.trim());
    }
    if let Some(first) = input["headerFirst"].as_bool() {
        report_design::set_word(&mut design.attrs, "headerFirst", if first { "true" } else { "false" });
    }
    save(tools, &file, &design, format!("Changed report {name}"))
}

fn band_words(band: &mut ReportBand, input: &Value) -> Result<(), String> {
    if let Some(height) = input["height"].as_f64() {
        if height <= 0.0 {
            return Err("height is a number of millimetres above 0".into());
        }
        band.height = report_design::snapped(height);
    }
    let grouped = REPEATED.contains(&band.kind.as_str());
    if input["by"].is_string() && !grouped {
        return Err(format!("a {} groups by nothing: `by` is a GroupHeader's or GroupFooter's", band.kind));
    }
    if input["list"].is_string() && band.kind != "Detail" {
        return Err(format!("a {} has no list: `list` is a Detail's", band.kind));
    }
    set_words(&mut band.attrs, input, &["by", "list"]);
    Ok(())
}

fn add_band(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (name, file, mut design) = opened(tools, input)?;
    let kind = band_kind(input)?;
    if !REPEATED.contains(&kind.as_str()) && design.bands.iter().any(|b| b.kind == kind) {
        return Err(format!("the report has a {kind} already: change_band changes it"));
    }
    let mut band = ReportBand { kind: kind.clone(), height: 10.0, attrs: Vec::new(), parts: Vec::new() };
    band_words(&mut band, input)?;
    design.bands.push(band);
    save(tools, &file, &design, format!("Added a {kind} to report {name}"))
}

fn change_band(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (name, file, mut design) = opened(tools, input)?;
    let at = band_at(&design, input)?;
    band_words(&mut design.bands[at], input)?;
    let kind = design.bands[at].kind.clone();
    save(tools, &file, &design, format!("Changed the {kind} of report {name}"))
}

fn remove_band(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (name, file, mut design) = opened(tools, input)?;
    let at = band_at(&design, input)?;
    let band = design.bands.remove(at);
    save(tools, &file, &design, format!("Removed the {} of report {name}, with its {} parts", band.kind, band.parts.len()))
}

fn add_part(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (name, file, mut design) = opened(tools, input)?;
    let at = band_at(&design, input)?;
    let kind = needed(input, "kind")?;
    let place = pair(input, "at")?.ok_or("give at: the part's place in the band, [x, y] in millimetres")?;
    let mut part: ReportPart = report_design::new_part(&kind, place).ok_or_else(|| format!("a band has no part {kind}: one of {}", PARTS.join(", ")))?;
    if let Some(size) = pair(input, "size")? {
        part.size = size;
    }
    set_words(&mut part.attrs, input, &PART_WORDS);
    let band = &mut design.bands[at];
    band.parts.push(part);
    let place = band.parts.len();
    let band_kind = band.kind.clone();
    save(tools, &file, &design, format!("Added a {kind} to the {band_kind} of report {name}, part {place}"))
}

fn part_at(design: &ReportDesign, input: &Value) -> Result<(usize, usize), String> {
    let band = band_at(design, input)?;
    let part = input["part"].as_u64().ok_or("give part: its place among the band's parts, counted from 1")? as usize;
    let count = design.bands[band].parts.len();
    if part == 0 || part > count {
        return Err(format!("the {} has {count} parts, not part {part}", design.bands[band].kind));
    }
    Ok((band, part - 1))
}

fn change_part(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (name, file, mut design) = opened(tools, input)?;
    let (band, at) = part_at(&design, input)?;
    let part = &mut design.bands[band].parts[at];
    if let Some(place) = pair(input, "at")? {
        part.at = place;
    }
    if let Some(size) = pair(input, "size")? {
        part.size = size;
    }
    set_words(&mut part.attrs, input, &PART_WORDS);
    let kind = part.kind.clone();
    save(tools, &file, &design, format!("Changed {kind} part {} of report {name}", at + 1))
}

fn remove_part(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (name, file, mut design) = opened(tools, input)?;
    let (band, at) = part_at(&design, input)?;
    let part = design.bands[band].parts.remove(at);
    save(tools, &file, &design, format!("Removed {} part {} of report {name}", part.kind, at + 1))
}

fn set_class(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (name, file, mut design) = opened(tools, input)?;
    let class = needed(input, "class")?.trim_start_matches('.').to_string();
    if !class.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) || !class.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(format!("'{class}' is no class name: letters, digits, - and _, starting with a letter"));
    }
    let property = needed(input, "property")?;
    if !CLASS_PROPERTIES.contains(&property.as_str()) {
        return Err(format!("a class here sets one of {}, not {property}", CLASS_PROPERTIES.join(", ")));
    }
    let value = text(input, "value");
    let mut files = Vec::new();
    let sheet_name = match report_design::word(&design.attrs, "stylesheet") {
        Some(sheet) => sheet.to_string(),
        None => {
            let sheet = format!("{name}.css");
            report_design::set_word(&mut design.attrs, "stylesheet", &sheet);
            files.push((file.clone(), report_design::write(&design)));
            sheet
        }
    };
    let sheet = file.with_file_name(&sheet_name);
    let css = std::fs::read_to_string(&sheet).unwrap_or_default();
    // The stylesheet goes first, so the report naming it finds it there.
    files.insert(0, (sheet.clone(), report_design::with_declaration(&css, &format!(".{class}"), &property, &value)));
    let done = if value.is_empty() { format!("Took {property} away from .{class} in {sheet_name}") } else { format!("Set .{class} {{ {property}: {value} }} in {sheet_name}") };
    saved(tools, &file, files, done)
}

#[cfg(test)]
mod tests {
    use clarity_agent::Tools;
    use serde_json::json;

    use crate::ProjectTools;

    fn project(name: &str) -> (std::path::PathBuf, std::path::PathBuf, ProjectTools) {
        let root = std::env::temp_dir().join(format!("clarity-report-tools-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("shop"), clarity_project::starter("Shop", "shop").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        let mut tools = ProjectTools::new(Some((file, dir.clone())), root.clone());
        let model = "module shop\n\nentity Order\n    number: text(20)\n    lines: list<Line>, owned\n\nentity Line\n    order: ref Order\n    product: text(80)\n    total: money\n";
        let wrote = tools.call("write_file", &json!({"path": "shop.clarity", "text": model}));
        assert!(!wrote.is_error, "{}", wrote.text);
        (root, dir, tools)
    }

    fn ok(tools: &mut ProjectTools, name: &str, input: serde_json::Value) -> String {
        let said = tools.call(name, &input);
        assert!(!said.is_error, "{name}: {}", said.text);
        said.text
    }

    #[test]
    fn an_agent_makes_an_invoice_as_the_reports_tab_does() {
        let (root, dir, mut tools) = project("invoice");
        assert!(ok(&mut tools, "report_view", json!({})).contains("no reports"));
        ok(&mut tools, "add_report", json!({"name": "Invoice", "data": [{"name": "order", "type": "Order"}]}));
        let file = dir.join("reports/Invoice.report");
        assert!(file.is_file());
        ok(&mut tools, "change_report", json!({"report": "Invoice", "paper": "A4 landscape", "margin": "15mm", "headerFirst": false}));
        ok(&mut tools, "add_band", json!({"report": "Invoice", "band": "Detail", "height": 6, "list": "order.lines"}));
        ok(&mut tools, "set_report_class", json!({"report": "Invoice", "class": "title", "property": "font-size", "value": "18pt"}));
        ok(&mut tools, "set_report_class", json!({"report": "Invoice", "class": "title", "property": "font-weight", "value": "bold"}));
        ok(&mut tools, "add_report_part", json!({"report": "Invoice", "band": "ReportHeader", "kind": "Field", "at": [0, 0], "bind": "order.number", "class": "title"}));
        ok(&mut tools, "add_report_part", json!({"report": "Invoice", "band": "Detail", "kind": "Field", "at": [0, 0], "size": [80, 6], "bind": "line.product"}));
        ok(&mut tools, "add_report_part", json!({"report": "Invoice", "band": "Detail", "kind": "Field", "at": [150, 0], "size": [30, 6], "bind": "line.total"}));
        ok(&mut tools, "change_report_part", json!({"report": "Invoice", "band": "Detail", "part": 2, "align": "right"}));
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains(r#"<Report name="Invoice" paper="A4 landscape" margin="15mm" headerFirst="false" stylesheet="Invoice.css">"#), "{text}");
        assert!(text.contains(r#"<Detail height="6mm" list="order.lines">"#) && text.contains(r#"<Field at="150,0" size="30,6" bind="line.total" align="right" />"#), "{text}");
        assert_eq!(std::fs::read_to_string(dir.join("reports/Invoice.css")).unwrap(), ".title { font-size: 18pt; font-weight: bold; }\n");
        let view = ok(&mut tools, "report_view", json!({"report": "Invoice"}));
        assert!(view.contains("paper A4 landscape") && view.contains("Data: order: Order.") && view.contains("part 2: Field at 150,0 size 30,6, bind line.total, align right"), "{view}");
        assert!(view.contains(".title { font-size: 18pt; font-weight: bold; }"), "{view}");
        // The report checks clean, as the designer would have saved it.
        let project = clarity_project::load_modules(&dir).unwrap();
        let report_errors: Vec<_> = project.diagnostics.iter().filter(|d| d.path.ends_with("Invoice.report")).collect();
        assert!(report_errors.is_empty(), "{report_errors:#?}");
        ok(&mut tools, "remove_report_part", json!({"report": "Invoice", "band": "Detail", "part": 1}));
        ok(&mut tools, "remove_band", json!({"report": "Invoice", "band": "Detail"}));
        assert!(!std::fs::read_to_string(&file).unwrap().contains("<Detail"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_change_that_breaks_the_report_is_refused_and_nothing_is_written() {
        let (root, dir, mut tools) = project("refused");
        ok(&mut tools, "add_report", json!({"name": "Invoice", "data": [{"name": "order", "type": "Order"}]}));
        let file = dir.join("reports/Invoice.report");
        let before = std::fs::read_to_string(&file).unwrap();
        let wrong = tools.call("add_report_part", &json!({"report": "Invoice", "band": "ReportHeader", "kind": "Field", "at": [0, 0], "bind": "order.nothing"}));
        assert!(wrong.is_error && wrong.text.contains("refused"), "{}", wrong.text);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), before, "nothing written");
        let twice = tools.call("add_band", &json!({"report": "Invoice", "band": "ReportHeader"}));
        assert!(twice.is_error && twice.text.contains("already"), "{}", twice.text);
        let by = tools.call("add_band", &json!({"report": "Invoice", "band": "Detail", "by": "line.product"}));
        assert!(by.is_error && by.text.contains("GroupHeader"), "{}", by.text);
        let kind = tools.call("add_report_part", &json!({"report": "Invoice", "band": "ReportHeader", "kind": "Chart", "at": [0, 0]}));
        assert!(kind.is_error && kind.text.contains("PageNumber"), "{}", kind.text);
        let property = tools.call("set_report_class", &json!({"report": "Invoice", "class": "title", "property": "margin", "value": "2mm"}));
        assert!(property.is_error && property.text.contains("font-size"), "{}", property.text);
        let none = tools.call("report_view", &json!({"report": "Bill"}));
        assert!(none.is_error && none.text.contains("Invoice"), "{}", none.text);
        let written = tools.call("write_file", &json!({"path": "reports/Invoice.report", "text": "<Report />"}));
        assert!(written.is_error && written.text.contains("add_report_part"), "write_file leaves reports to their tools: {}", written.text);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn groups_are_named_by_their_place() {
        let (root, dir, mut tools) = project("groups");
        ok(&mut tools, "add_report", json!({"name": "Sales", "data": [{"name": "order", "type": "Order"}]}));
        ok(&mut tools, "add_band", json!({"report": "Sales", "band": "Detail", "list": "order.lines"}));
        ok(&mut tools, "add_band", json!({"report": "Sales", "band": "GroupHeader", "by": "line.product"}));
        ok(&mut tools, "add_band", json!({"report": "Sales", "band": "GroupHeader", "by": "line.total"}));
        ok(&mut tools, "change_band", json!({"report": "Sales", "band": "GroupHeader", "which": 2, "height": 12}));
        let text = std::fs::read_to_string(dir.join("reports/Sales.report")).unwrap();
        assert!(text.contains(r#"<GroupHeader height="10mm" by="line.product" />"#) && text.contains(r#"<GroupHeader height="12mm" by="line.total" />"#), "{text}");
        let third = tools.call("change_band", &json!({"report": "Sales", "band": "GroupHeader", "which": 3, "height": 5}));
        assert!(third.is_error && third.text.contains("2 GroupHeaders"), "{}", third.text);
        let _ = std::fs::remove_dir_all(root);
    }
}
