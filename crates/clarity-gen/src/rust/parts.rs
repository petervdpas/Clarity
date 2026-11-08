//! Custom parts written in Rust (DESIGN 5.19 and 7.9): a part is a crate of
//! the project (`crate "parts/Arena"`), which implements the trait Clarity
//! writes from the declaration into its `src/clarity.rs`. That file is all
//! the crate needs of Clarity: the trait and its typed tells, over
//! Clarity.UI's custom part. The program's glue registers each kind of part
//! with Clarity.UI by its name.

use std::fmt::Write;

use clarity_spec::Project;
use clarity_spec::code::{Param, PartDecl, PartLibrary, TypeRef, TypeRefKind};

/// The project's parts written in Rust.
pub fn crate_parts(project: &Project) -> Vec<&PartDecl> {
    project.files.iter().flat_map(|f| &f.parts).filter(|p| matches!(p.library, Some(PartLibrary::Crate { .. }))).collect()
}

/// A value of `ty` as the part's Rust takes it.
fn rust_type(ty: &TypeRef) -> &'static str {
    match ty.kind {
        TypeRefKind::Integer | TypeRefKind::Duration => "i64",
        TypeRefKind::Bool => "bool",
        TypeRefKind::Decimal | TypeRefKind::Money => "rust_decimal::Decimal",
        TypeRefKind::Date => "Option<chrono::NaiveDate>",
        TypeRefKind::DateTime => "Option<chrono::NaiveDateTime>",
        _ => "String",
    }
}

/// A tell's value as the part gives it: text by reference.
fn given_type(ty: &TypeRef) -> &'static str {
    match rust_type(ty) {
        "String" => "&str",
        other => other,
    }
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn params(list: &[Param], given: bool) -> String {
    list.iter().map(|p| format!(", {}: {}", snake(&p.name.name), if given { given_type(&p.ty) } else { rust_type(&p.ty) })).collect()
}

fn uses_decimal(part: &PartDecl) -> bool {
    types(part).any(|t| matches!(t.kind, TypeRefKind::Decimal | TypeRefKind::Money))
}

fn uses_dates(part: &PartDecl) -> bool {
    types(part).any(|t| matches!(t.kind, TypeRefKind::Date | TypeRefKind::DateTime))
}

fn types(part: &PartDecl) -> impl Iterator<Item = &TypeRef> {
    part.binds
        .iter()
        .chain(part.properties.iter().map(|p| &p.ty))
        .chain(part.tells.iter().chain(&part.hears).flat_map(|m| m.params.iter().map(|p| &p.ty)))
}

/// The crate's package name, as its Cargo.toml gives it.
pub fn package_of(project: &Project, part: &str) -> String {
    project
        .part_files
        .iter()
        .flatten()
        .find(|f| f.part == part && f.path == "Cargo.toml")
        .and_then(|f| {
            String::from_utf8_lossy(&f.bytes).lines().find_map(|l| {
                let (key, value) = l.split_once('=')?;
                (key.trim() == "name").then(|| value.trim().trim_matches('"').to_string())
            })
        })
        .unwrap_or_else(|| snake(part))
}

/// The crate of Clarity's that a part may name, with the version the program is built on.
fn clarity_crates() -> [(&'static str, &'static str); 1] {
    [("clarity-runtime", super::runtime_version())]
}

/// The version requirement of `krate` in a Cargo.toml line, if the line names it with one.
fn requirement<'a>(line: &'a str, krate: &str) -> Option<&'a str> {
    let (key, value) = line.split_once('=')?;
    if key.trim() != krate {
        return None;
    }
    let value = value.trim();
    // `x = "0.2.0"` or `x = { version = "0.2.0", ... }`; a path or git one is the person's own.
    if value.starts_with('{') { value.split("version").nth(1)?.split('"').nth(1) } else { Some(value.trim_matches('"')) }
}

/// `manifest` with its line of Clarity's runtime naming the version the
/// program is built on, where the line names one Cargo would not share with
/// it; none when nothing needs a change.
pub fn with_runtime_version(manifest: &str) -> Option<String> {
    let mut changed = false;
    let lines: Vec<String> = manifest
        .lines()
        .map(|line| {
            for (krate, version) in clarity_crates() {
                if let Some(wanted) = requirement(line, krate) {
                    if !ui_accepts(wanted, version) {
                        changed = true;
                        return line.replacen(&format!("\"{wanted}\""), &format!("\"{version}\""), 1);
                    }
                }
            }
            line.to_string()
        })
        .collect();
    changed.then(|| {
        let mut text = lines.join("\n");
        if manifest.ends_with('\n') {
            text.push('\n');
        }
        text
    })
}

/// Whether Cargo takes `version` for the requirement `wanted`, as a caret
/// requirement does: the same first number that is not 0 (0.1 and 0.2 differ).
pub fn ui_accepts(wanted: &str, version: &str) -> bool {
    let wanted = wanted.trim().trim_start_matches(['^', '=', '~']).trim();
    let numbers = |v: &str| v.split('.').map(|n| n.trim().parse::<u64>().ok()).collect::<Option<Vec<_>>>();
    let (Some(w), Some(v)) = (numbers(wanted), numbers(version)) else { return true };
    let significant = w.iter().position(|n| *n != 0).unwrap_or(w.len().saturating_sub(1));
    w.iter().zip(&v).take(significant + 1).all(|(a, b)| a == b) && v >= w
}

/// The parts whose Cargo.toml names clarity-runtime at a version the program
/// cannot share (Cargo would build two), or Clarity.UI's crate or Skia, which
/// a part no longer takes: it draws on the canvas of its `src/clarity.rs`.
pub fn ui_mismatches(project: &Project) -> Vec<String> {
    let mut found = Vec::new();
    for p in crate_parts(project) {
        let n = &p.name.name;
        let Some(manifest) = project.part_files.iter().flatten().find(|f| &f.part == n && f.path == "Cargo.toml") else { continue };
        let text = String::from_utf8_lossy(&manifest.bytes);
        for line in text.lines() {
            for (krate, version) in clarity_crates() {
                if let Some(wanted) = requirement(line, krate).filter(|w| !ui_accepts(w, version)) {
                    found.push(format!("custom part {n} names {krate} {wanted} in its Cargo.toml, but the program is built on {krate} {version}: write {krate} = \"{version}\" there"));
                }
            }
            for krate in ["clarity-ui", "skia-safe"] {
                if line.split_once('=').is_some_and(|(key, _)| key.trim() == krate) {
                    found.push(format!("custom part {n} names {krate} in its Cargo.toml, which a part no longer takes: it draws on the Canvas of src/clarity.rs, through Clarity.UI's library; remove the line"));
                }
            }
        }
    }
    found
}

/// The methods of trait `<Name>Part` in `src/clarity.rs`: each name, its
/// line, and whether the part must write it (it has no body there).
fn trait_methods(part: &PartDecl) -> Vec<(String, String, bool)> {
    let text = ui_interface(part);
    let head = format!("pub trait {}Part", part.name.name);
    let Some(start) = text.find(&head) else { return Vec::new() };
    let body = &text[start..];
    let body = &body[..body.find("\n}\n").unwrap_or(body.len())];
    body.lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("fn ")?;
            let name = crate::part_code::first_word(rest).to_string();
            Some((name, line.trim_end_matches([';', '{']).trim().to_string(), line.ends_with(';')))
        })
        .collect()
}

/// What Clarity no longer has that a Rust part's code may still use: the
/// name, and what takes its place.
fn removed(part: &PartDecl) -> Vec<(String, String)> {
    let n = &part.name.name;
    vec![(format!("{n}Host"), format!("the part tells its window through cx: cx.changed(..) or a tell of {n}Tells"))]
}

/// What a Rust part's own code lacks of `src/clarity.rs`, or still uses that
/// Clarity no longer has, read from its code without building it.
pub fn code_mismatches(project: &Project) -> Vec<String> {
    use crate::part_code::{Lang, after, before, close, find_word, first_word, sources, uses_name};
    let mut found = Vec::new();
    for p in crate_parts(project) {
        let n = &p.name.name;
        let code = sources(project, p, Lang::Rust, |path| path.ends_with(".rs") && path != "src/clarity.rs");
        if code.is_empty() {
            continue;
        }
        let methods = trait_methods(p);
        let trait_name = format!("{n}Part");
        // Each `impl <Name>Part for T { .. }`: the file, where, and the fns it writes.
        let mut impls = Vec::new();
        for source in &code {
            for at in find_word(&source.code, &trait_name) {
                // `impl`, `impl<T>` or `impl clarity::` before the name.
                let mut b = before(&source.code, at);
                while let Some(rest) = b.strip_suffix("::") {
                    b = rest.trim_end().trim_end_matches(crate::part_code::is_ident).trim_end();
                }
                let is_impl = b.ends_with("impl") || b.rfind("impl<").is_some_and(|k| b.ends_with('>') && !b[k..].contains([';', '{', '}']));
                if !is_impl || first_word(after(&source.code, at + trait_name.len())) != "for" {
                    continue;
                }
                let Some(open) = source.code[at..].find('{').map(|k| at + k) else { continue };
                let Some(end) = close(&source.code, open) else { continue };
                let block = &source.code[open..end];
                // The block's own fns, not those inside a method.
                let depth = |k: usize| block[..k].matches('{').count() - block[..k].matches('}').count();
                let fns: Vec<(String, usize)> = find_word(block, "fn").into_iter().filter(|&k| depth(k) == 1).map(|k| (first_word(after(block, k + 2)).to_string(), open + k)).filter(|(name, _)| !name.is_empty()).collect();
                impls.push((source, at, fns));
            }
        }
        if impls.is_empty() {
            found.push(format!("custom part {n}: no type of its crate implements {trait_name} (src/clarity.rs): write impl {trait_name} for {n} {{ .. }} in src/lib.rs"));
        }
        for (source, at, fns) in &impls {
            for (name, line, required) in &methods {
                if *required && !fns.iter().any(|(f, _)| f == name) {
                    found.push(format!("custom part {n}: {} impl {trait_name} lacks {name}: write {line} {{ .. }}", source.at(*at)));
                }
            }
            for (name, k) in fns {
                if !methods.iter().any(|(m, _, _)| m == name) {
                    found.push(format!("custom part {n}: {} fn {name} is not a method of {trait_name} (src/clarity.rs) any more: remove it or move it to another impl", source.at(*k)));
                }
            }
        }
        for (name, instead) in removed(p) {
            if let Some((k, at)) = uses_name(&code, &name) {
                found.push(format!("custom part {n}: {} uses {name}, which Clarity no longer has: {instead}", code[k].at(at)));
            }
        }
        // Clarity.UI's crate is not the part's: it draws through the library.
        for source in &code {
            if let Some(at) = find_word(&source.code, "clarity_ui").into_iter().find(|&at| after(&source.code, at + "clarity_ui".len()).starts_with("::")) {
                found.push(format!("custom part {n}: {} uses clarity_ui, which a part no longer takes: draw on the Canvas and act through the Cx of src/clarity.rs", source.at(at)));
            }
        }
        // The runtime's part host of native windows is gone.
        for source in &code {
            if let Some(at) = find_word(&source.code, "clarity_runtime").into_iter().find(|&at| after(&source.code, at + "clarity_runtime".len()).strip_prefix("::").is_some_and(|r| first_word(r.trim_start()) == "parts")) {
                found.push(format!("custom part {n}: {} uses clarity_runtime::parts, which Clarity no longer has: implement {trait_name} of src/clarity.rs, a control of Clarity.UI", source.at(at)));
            }
        }
    }
    found
}

// -- On Clarity.UI (DESIGN 7.9) ---------------------------------------------
//
// A part is a control drawn on Clarity.UI's surface, as Go, C and C++ parts
// are: its crate's type implements the typed trait of its `src/clarity.rs`,
// which declares the canvas it draws on and what it does through Clarity.UI
// as plain Rust traits, so the crate needs nothing of Clarity's. The
// program's glue (`src/parts.rs`) makes those of the runtime's
// `native_ui` and registers each part with the library by its name.

/// How a value of `ty` is read from what the window holds.
fn ui_reader(ty: &TypeRef) -> &'static str {
    match ty.kind {
        TypeRefKind::Integer | TypeRefKind::Duration => "integer",
        TypeRefKind::Bool => "boolean",
        TypeRefKind::Decimal | TypeRefKind::Money => "decimal",
        TypeRefKind::Date => "date",
        TypeRefKind::DateTime => "date_time",
        _ => "text",
    }
}

/// A told value `name` of `ty` as the window takes it.
fn ui_value(ty: &TypeRef, name: &str) -> String {
    match ty.kind {
        TypeRefKind::Decimal | TypeRefKind::Money => format!("Value::from({name}.to_string())"),
        TypeRefKind::Date => format!("{name}.map_or(Value::Null, |v| Value::from(v.to_string()))"),
        TypeRefKind::DateTime => format!("{name}.map_or(Value::Null, |v| Value::from(v.format(\"%Y-%m-%dT%H:%M:%S\").to_string()))"),
        _ => format!("Value::from({name})"),
    }
}

/// The readers of values the interface uses, each a small function.
fn ui_readers(part: &PartDecl) -> String {
    let mut out = String::from(
        "/// A whole number, from a number or the text of one.\nfn integer(value: &Value) -> i64 {\n    value.as_i64().or_else(|| value.as_f64().map(|v| v as i64)).or_else(|| value.as_str().and_then(|v| v.trim().parse().ok())).unwrap_or_default()\n}\n\n/// True or false; anything else is false.\nfn boolean(value: &Value) -> bool {\n    value.as_bool().unwrap_or(false)\n}\n\n/// Text: a number or a truth as written; nothing is empty.\nfn text(value: &Value) -> String {\n    match value {\n        Value::String(v) => v.clone(),\n        Value::Null => String::new(),\n        other => other.to_string(),\n    }\n}\n",
    );
    if uses_decimal(part) {
        out.push_str("\n/// A decimal, from its text or a number.\nfn decimal(value: &Value) -> rust_decimal::Decimal {\n    text(value).trim().parse().unwrap_or_default()\n}\n");
    }
    if uses_dates(part) {
        out.push_str("\n/// A day, from its ISO text (`2026-10-20`); none for anything else.\nfn date(value: &Value) -> Option<chrono::NaiveDate> {\n    value.as_str().and_then(|v| chrono::NaiveDate::parse_from_str(v.get(..10)?, \"%Y-%m-%d\").ok())\n}\n\n/// A moment, from its ISO text (`2026-10-20T09:30:00`); none for anything else.\nfn date_time(value: &Value) -> Option<chrono::NaiveDateTime> {\n    let v = value.as_str()?.replace(' ', \"T\");\n    chrono::NaiveDateTime::parse_from_str(&v, \"%Y-%m-%dT%H:%M:%S\").or_else(|_| chrono::NaiveDateTime::parse_from_str(&v, \"%Y-%m-%dT%H:%M\")).ok()\n}\n");
    }
    out
}

/// The parameters of a hear or tell with the part's context last.
fn ui_params(list: &[Param], given: bool) -> String {
    list.iter().map(|p| format!(", {}: {}", snake(&p.name.name), if given { given_type(&p.ty) } else { rust_type(&p.ty) })).collect()
}

/// The canvas and the context of `src/clarity.rs`, the same for every part:
/// `(trait, [(method line, call on the runtime's own)])`, which the
/// interface declares and the program's glue forwards.
const CANVAS: &[(&str, &str, &str)] = &[
    ("", "fn fill(&mut self, x: f64, y: f64, width: f64, height: f64, color: u32, radius: f64)", "fill(x, y, width, height, color, radius)"),
    ("", "fn stroke(&mut self, x: f64, y: f64, width: f64, height: f64, color: u32, line_width: f64, radius: f64)", "stroke(x, y, width, height, color, line_width, radius)"),
    ("", "fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, color: u32, line_width: f64)", "line(x1, y1, x2, y2, color, line_width)"),
    ("", "fn oval(&mut self, x: f64, y: f64, width: f64, height: f64, color: u32, filled: bool, line_width: f64)", "oval(x, y, width, height, color, filled, line_width)"),
    ("An SVG path's data (\"M0 0 L10 10 Z\"), filled or stroked.", "fn path(&mut self, svg: &str, color: u32, filled: bool, line_width: f64)", "path(svg, color, filled, line_width)"),
    ("Text in a box: align 0 left, 1 center, 2 right; size 0 is the look's text size.", "fn text(&mut self, text: &str, x: f64, y: f64, width: f64, height: f64, color: u32, size: f64, bold: bool, align: i32)", "text(text, x, y, width, height, color, size, bold, align)"),
    ("How wide text is at a size (0: the look's text size).", "fn measure(&mut self, text: &str, size: f64, bold: bool) -> f64", "measure(text, size, bold)"),
    ("RGBA pixels (unpremultiplied, rows of `stride` bytes) drawn into a box.", "fn pixels(&mut self, rgba: &[u8], width: u32, height: u32, stride: usize, x: f64, y: f64, box_width: f64, box_height: f64)", "pixels(rgba, width, height, stride, x, y, box_width, box_height)"),
    ("Clips what follows to a box until `restore`.", "fn clip(&mut self, x: f64, y: f64, width: f64, height: f64)", "clip(x, y, width, height)"),
    ("", "fn restore(&mut self)", "restore()"),
    ("A color of the look by its role (\"accent\", \"text\", \"border\", \"error\").", "fn look_color(&mut self, role: &str) -> u32", "look_color(role)"),
];

const CX: &[(&str, &str, &str)] = &[
    ("Its place in the window: x, y, width and height.", "fn bounds(&self) -> (f64, f64, f64, f64)", "bounds()"),
    ("", "fn enabled(&self) -> bool", "enabled()"),
    ("Has it drawn again.", "fn invalidate(&mut self)", "invalidate()"),
    ("Has its `wake` called once after `ms` milliseconds.", "fn wake_in(&mut self, ms: f64)", "wake_in(ms)"),
    ("Has it drawn every frame while on.", "fn animate(&mut self, on: bool)", "animate(on)"),
    ("", "fn focus(&mut self)", "focus()"),
    ("The person changed the value it binds (see `changed` of its tells).", "fn change(&mut self, value: Value)", "change(&value)"),
    ("It tells `tell` with its values (see its tells).", "fn tell(&mut self, tell: &str, values: Vec<Value>)", "tell(tell, &values)"),
    ("A text of the program's translations by its key, in the language shown; `english` when there is none.", "fn translate(&self, key: &str, english: &str) -> String", "translate(key, english)"),
];

/// The methods every part may write, beside drawing and its window's words:
/// (what it is for, line, body of the default, call in the wrapper).
const INPUT: &[(&str, &str, &str, &str)] = &[
    ("Whether it takes the focus: pressed, or reached with Tab.", "fn focusable(&self) -> bool", "false", "focusable()"),
    ("The main button went down on it.", "fn pointer_pressed(&mut self, at: Pointer, cx: &mut dyn Cx)", "let _ = (at, cx);", "pointer_pressed(at, cx)"),
    ("The pointer moved over it, or anywhere while it holds the pointer.", "fn pointer_moved(&mut self, at: Pointer, cx: &mut dyn Cx)", "let _ = (at, cx);", "pointer_moved(at, cx)"),
    ("The main button went up after it was pressed on it.", "fn pointer_released(&mut self, at: Pointer, cx: &mut dyn Cx)", "let _ = (at, cx);", "pointer_released(at, cx)"),
    ("Pressed and released with the main button.", "fn click(&mut self, cx: &mut dyn Cx)", "let _ = cx;", "click(cx)"),
    ("The wheel turned over it with keys held (`SHIFT` and the others): true when it took the turn.", "fn wheel(&mut self, dx: f64, dy: f64, modifiers: u32, cx: &mut dyn Cx) -> bool", "let _ = (dx, dy, modifiers, cx);\n        false", "wheel(dx, dy, modifiers, cx)"),
    ("A key by its name (\"Enter\", \"Space\", \"Left\", \"A\", \"+\") while it has the focus: true when it took the key.", "fn key(&mut self, name: &str, modifiers: u32, cx: &mut dyn Cx) -> bool", "let _ = (name, modifiers, cx);\n        false", "key(name, modifiers, cx)"),
    ("Text typed while it has the focus.", "fn text(&mut self, text: &str, cx: &mut dyn Cx)", "let _ = (text, cx);", "text(text, cx)"),
    ("It got or lost the focus.", "fn focus_changed(&mut self, focused: bool, cx: &mut dyn Cx)", "let _ = (focused, cx);", "focus_changed(focused, cx)"),
    ("The time it asked for came (`cx.wake_in(ms)`).", "fn wake(&mut self, cx: &mut dyn Cx)", "let _ = cx;", "wake(cx)"),
    ("Its window opened: called once, after the values of its line, what\n    /// code set and what it binds; it may start a timer or an animation here.", "fn start(&mut self, cx: &mut dyn Cx)", "let _ = cx;", "start(cx)"),
    ("Its window left the screen: it stops what it started.", "fn stop(&mut self)", "", "stop()"),
    ("What it says about the place at a point when the pointer rests there.", "fn tooltip_at(&self, x: f64, y: f64) -> Option<String>", "let _ = (x, y);\n        None", "tooltip_at(x, y)"),
];

fn doc(what: &str) -> String {
    if what.is_empty() { String::new() } else { format!("    /// {what}\n") }
}

/// `parts/Gauge/src/clarity.rs` of a part: the canvas it draws on, what it
/// does through Clarity.UI, the typed trait the crate's `Gauge` implements,
/// its words to its window, and what the program's glue holds of it.
pub fn ui_interface(part: &PartDecl) -> String {
    let n = &part.name.name;
    let mut shows = String::new();
    let mut sets = String::new();
    if let Some(binds) = &part.binds {
        let _ = write!(shows, "\n    /// The value it shows, from what it binds; given when its window starts and when it changed.\n    fn show_value(&mut self, value: {}, cx: &mut dyn Cx);\n", rust_type(binds));
        let _ = writeln!(sets, "            \"value\" => self.0.show_value({}(value), cx),", ui_reader(binds));
    }
    for property in &part.properties {
        let p = snake(&property.name.name);
        let _ = write!(shows, "\n    /// Property {}, as the window or code sets it.\n    fn show_{p}(&mut self, {p}: {}, cx: &mut dyn Cx);\n", property.name.name, rust_type(&property.ty));
        let _ = writeln!(sets, "            {:?} => self.0.show_{p}({}(value), cx),", property.name.name, ui_reader(&property.ty));
    }
    let mut hears = String::new();
    for hear in &part.hears {
        let _ = write!(shows, "\n    /// What code tells it: {}.\n    fn on_{}(&mut self{}, cx: &mut dyn Cx);\n", hear.name.name, snake(&hear.name.name), ui_params(&hear.params, false));
        let args: Vec<String> = hear.params.iter().enumerate().map(|(i, p)| format!("{}(values.get({i}).unwrap_or(&Value::Null))", ui_reader(&p.ty))).collect();
        let args = args.iter().map(|a| format!("{a}, ")).collect::<String>();
        let _ = writeln!(hears, "            {:?} => self.0.on_{}({args}cx),", hear.name.name, snake(&hear.name.name));
    }
    let mut tells_trait = String::new();
    let mut tells_impl = String::new();
    if let Some(binds) = &part.binds {
        let _ = write!(tells_trait, "    /// The person changed the value: what the part binds takes it (one edit, which Ctrl+Z takes back) and its change handler runs.\n    fn changed(&mut self, value: {});\n", given_type(binds));
        let _ = write!(tells_impl, "    fn changed(&mut self, value: {}) {{\n        self.change({});\n    }}\n", given_type(binds), ui_value(binds, "value"));
    }
    for tell in &part.tells {
        let t = snake(&tell.name.name);
        let values: Vec<String> = tell.params.iter().map(|p| ui_value(&p.ty, &snake(&p.name.name))).collect();
        if !tells_trait.is_empty() {
            tells_trait.push('\n');
            tells_impl.push('\n');
        }
        let _ = write!(tells_trait, "    /// Tells {}: the handler wired to it runs with these values.\n    fn {t}(&mut self{});\n", tell.name.name, params(&tell.params, true));
        let _ = write!(tells_impl, "    fn {t}(&mut self{}) {{\n        self.tell({:?}, vec![{}]);\n    }}\n", params(&tell.params, true), tell.name.name, values.join(", "));
    }
    let tells = if tells_trait.is_empty() {
        String::new()
    } else {
        format!("\n/// What part {n} says to its window, from any of its methods: `cx.changed(..)` and one per tell.\npub trait {n}Tells {{\n{tells_trait}}}\n\nimpl<C: Cx + ?Sized> {n}Tells for C {{\n{tells_impl}}}\n")
    };
    let switch = |on: &str, unused: &str, cases: &str| if cases.is_empty() { format!("        let _ = ({unused}, cx);\n") } else { format!("        match {on} {{\n{cases}            _ => {{}}\n        }}\n") };
    let declare = |list: &[(&str, &str, &str)]| list.iter().map(|(what, line, _)| format!("{}    {line};\n", doc(what))).collect::<Vec<_>>().join("\n");
    let mut input = String::new();
    let mut custom = String::new();
    let mut forwards = String::new();
    for (what, line, body, call) in INPUT {
        let body = if body.is_empty() { "{}".to_string() } else { format!("{{\n        {body}\n    }}") };
        let _ = write!(input, "\n{}    {line} {body}\n", doc(what));
        let _ = writeln!(custom, "    {line};");
        let _ = write!(forwards, "\n    {line} {{\n        self.0.{call}\n    }}\n");
    }
    format!(
        "//! Custom part {n} (DESIGN 5.19, 7.9): a control of Clarity.UI. The canvas
//! it draws on, what it does through Clarity.UI, the trait the crate's `{n}`
//! implements and what it says to its window, in plain Rust: the crate needs
//! nothing of Clarity's. Written by Clarity from the part's declaration, and
//! written again when the declaration changes; the rest of the crate is yours.
//!
//! Every method is called on the screen's thread, one at a time; the part
//! is made when its window opens and lives as long as its window.

#![allow(dead_code)]

pub use serde_json::Value;

/// What `draw` draws on, in the screen's units. Colors are 0xAARRGGBB.
pub trait Canvas {{
{canvas}}}

/// What the part does through Clarity.UI, valid during the call it is given to.
pub trait Cx {{
{cx}}}

/// Keys held, as bits of `modifiers`.
pub const SHIFT: u32 = 1;
pub const CONTROL: u32 = 2;
pub const ALT: u32 = 4;
pub const META: u32 = 8;

/// Where the pointer is, in the screen's units, with the keys held and the clicks counted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pointer {{
    pub x: f64,
    pub y: f64,
    pub modifiers: u32,
    pub clicks: u32,
}}

/// What part {n} does: it draws itself in its place and shows what the
/// window gives it; the input methods it does not write do nothing.
pub trait {n}Part: Default + 'static {{
    /// Draws itself in its place, with the look's colors (`canvas.look_color`).
    fn draw(&self, canvas: &mut dyn Canvas, x: f64, y: f64, width: f64, height: f64);
{shows}{input}}}
{tells}
/// A part as the program's glue holds it, whatever its kind.
pub trait Custom {{
    fn set(&mut self, name: &str, value: &Value, cx: &mut dyn Cx);
    fn hear(&mut self, hear: &str, values: &[Value], cx: &mut dyn Cx);
    fn draw(&self, canvas: &mut dyn Canvas, x: f64, y: f64, width: f64, height: f64);
{custom}}}

/// Part {n} as the glue holds it: the window's words in, typed.
struct Held<P>(P);

impl<P: {n}Part> Custom for Held<P> {{
    fn set(&mut self, name: &str, value: &Value, cx: &mut dyn Cx) {{
        if name == \"enabled\" {{
            // The window draws it again; `cx.enabled()` says which it is.
            cx.invalidate();
            return;
        }}
{set_switch}    }}

    fn hear(&mut self, hear: &str, values: &[Value], cx: &mut dyn Cx) {{
        // What it hears may take no values.
        let _ = values;
{hear_switch}    }}

    fn draw(&self, canvas: &mut dyn Canvas, x: f64, y: f64, width: f64, height: f64) {{
        self.0.draw(canvas, x, y, width, height);
    }}
{forwards}}}

/// A new part {n}, as the program registers it with Clarity.UI.
pub fn make() -> Box<dyn Custom> {{
    Box::new(Held(crate::{n}::default()))
}}

{readers}",
        canvas = declare(CANVAS),
        cx = declare(CX),
        set_switch = switch("name", "value", &sets),
        hear_switch = switch("hear", "hear, values", &hears),
        readers = ui_readers(part),
    )
}

/// What declaring a Rust part writes into its folder: a crate whose type
/// draws its value in a frame until it draws the library's work.
pub fn ui_starter(part: &PartDecl) -> Vec<(String, String)> {
    let n = &part.name.name;
    let package = snake(n);
    let mut deps = String::from("serde_json = \"1\"\n");
    if uses_decimal(part) {
        deps.push_str("rust_decimal = \"1\"\n");
    }
    if uses_dates(part) {
        deps.push_str("chrono = \"0.4\"\n");
    }
    let cargo = format!("# The crate of custom part {n}, a control of Clarity.UI. Clarity builds it into the program.\n[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n{deps}");
    let tell = part.tells.first().map_or_else(|| "cx.changed(..)".to_string(), |t| format!("cx.{}(..)", snake(&t.name.name)));
    let mut lib = format!(
        "//! Custom part {n}: a control of Clarity.UI that shows the work of its
//! library in its place in the window. It draws itself (`draw`) and takes the
//! pointer and keys as any part does (`click`, `key`). Keep the methods of
//! `{n}Part` (src/clarity.rs), which follow the part's declaration. Call a
//! tell, such as {tell}, to run its handler.

mod clarity;
pub use clarity::*;

/// What it shows until it draws the library's work.
#[derive(Default)]
pub struct {n} {{
    shown: String,
}}

impl {n}Part for {n} {{
    fn draw(&self, canvas: &mut dyn Canvas, x: f64, y: f64, width: f64, height: f64) {{
        let (border, text) = (canvas.look_color(\"border\"), canvas.look_color(\"text\"));
        canvas.stroke(x, y, width, height, border, 1.0, 4.0);
        canvas.text(&self.shown, x + 8.0, y + 8.0, width - 16.0, height - 16.0, text, 0.0, false, 0);
    }}
"
    );
    if let Some(binds) = &part.binds {
        let _ = write!(lib, "\n    fn show_value(&mut self, value: {}, cx: &mut dyn Cx) {{\n        self.shown = {};\n        cx.invalidate();\n    }}\n", rust_type(binds), shown(binds));
    }
    for property in &part.properties {
        let p = snake(&property.name.name);
        let _ = write!(lib, "\n    fn show_{p}(&mut self, {p}: {}, cx: &mut dyn Cx) {{\n        let _ = ({p}, cx);\n    }}\n", rust_type(&property.ty));
    }
    for hear in &part.hears {
        let mut names: Vec<String> = hear.params.iter().map(|p| snake(&p.name.name)).collect();
        names.push("cx".into());
        let _ = write!(lib, "\n    fn on_{}(&mut self{}, cx: &mut dyn Cx) {{\n        let _ = ({});\n    }}\n", snake(&hear.name.name), ui_params(&hear.params, false), names.join(", "));
    }
    lib.push_str("}\n");
    vec![("Cargo.toml".into(), cargo), ("src/lib.rs".into(), lib), ("src/clarity.rs".into(), ui_interface(part))]
}

/// The text a starter shows of a value of `ty`.
fn shown(ty: &TypeRef) -> &'static str {
    match ty.kind {
        TypeRefKind::Date | TypeRefKind::DateTime => "value.map(|v| v.to_string()).unwrap_or_default()",
        _ => "value.to_string()",
    }
}

/// The glue's method forwarding `line` of a part's trait to the runtime's
/// own: the line with the runtime's types, and the call.
fn forward(line: &str, call: &str) -> String {
    format!("            {line} {{\n                self.0.{call}\n            }}\n")
}

/// `src/parts.rs` of a program: each part's crate made a custom part of the
/// runtime's `native_ui` and registered with Clarity.UI by its declaration's
/// name. `glue!` is the same for every part: its canvas and context are the
/// runtime's, forwarded.
pub fn ui_glue(project: &Project) -> String {
    let canvas: String = CANVAS.iter().map(|(_, line, call)| forward(line, call)).collect::<Vec<_>>().join("\n");
    let cx: String = CX.iter().map(|(_, line, call)| forward(line, call)).collect::<Vec<_>>().join("\n");
    let mut input = String::new();
    for (_, line, _, call) in INPUT {
        let line = line.replace("cx: &mut dyn Cx", "cx: &mut ui::Cx").replace("at: Pointer", "at: ui::Pointer");
        let call = call.replace("(at,", "(pointer(at),");
        let call = call.strip_suffix("cx)").map_or(call.clone(), |head| format!("{head}&mut Cx(cx))"));
        let _ = write!(input, "\n{}", forward(&line, &call));
    }
    let mut modules = String::new();
    let mut registers = String::new();
    for part in crate_parts(project) {
        let n = &part.name.name;
        let package = package_of(project, n).replace('-', "_");
        let module = format!("part_{}", snake(n));
        let _ = write!(modules, "\nmod {module} {{\n    glue!({package});\n}}\n");
        let _ = writeln!(registers, "        ({n:?}, {module}::make),");
    }
    format!(
        "// The program's custom parts written in Rust, controls of Clarity.UI (DESIGN 7.9).
// Generated by Clarity.

use clarity_runtime::native_ui as ui;

/// A part's crate as a custom part of the runtime: its canvas and context
/// are the runtime's, and what the library says reaches the part.
macro_rules! glue {{
    ($krate:ident) => {{
        use super::ui;
        use serde_json::Value;

        struct Canvas<'a>(&'a mut ui::Canvas);

        impl $krate::Canvas for Canvas<'_> {{
{canvas}        }}

        struct Cx<'a>(&'a mut ui::Cx);

        impl $krate::Cx for Cx<'_> {{
{cx}        }}

        fn pointer(at: ui::Pointer) -> $krate::Pointer {{
            $krate::Pointer {{ x: at.x, y: at.y, modifiers: at.modifiers, clicks: at.clicks }}
        }}

        struct Part(Box<dyn $krate::Custom>);

        impl ui::CustomPart for Part {{
            fn set(&mut self, name: &str, value: &Value, cx: &mut ui::Cx) {{
                self.0.set(name, value, &mut Cx(cx))
            }}

            fn hear(&mut self, hear: &str, values: &[Value], cx: &mut ui::Cx) {{
                self.0.hear(hear, values, &mut Cx(cx))
            }}

            fn draw(&self, canvas: &mut ui::Canvas, x: f64, y: f64, width: f64, height: f64) {{
                self.0.draw(&mut Canvas(canvas), x, y, width, height)
            }}
{input}        }}

        pub fn make() -> Box<dyn ui::CustomPart> {{
            Box::new(Part($krate::make()))
        }}
    }};
}}
{modules}
/// Tells Clarity.UI about every kind of part the program has, by its name.
pub fn register() {{
    let parts: &[(&str, fn() -> Box<dyn ui::CustomPart>)] = &[
{registers}    ];
    for (kind, make) in parts {{
        if let Err(why) = ui::register_part(kind, *make) {{
            eprintln!(\"{{why}}\");
        }}
    }}
}}
"
    )
}

#[cfg(test)]
mod tests {
    use super::{ui_accepts, with_runtime_version};

    #[test]
    fn a_parts_runtime_line_follows_the_program() {
        let runtime = super::super::runtime_version();
        assert_eq!(with_runtime_version("[dependencies]\nclarity-runtime = \"0.0.1\"\nbevy = \"0.16\"\n").unwrap(), format!("[dependencies]\nclarity-runtime = \"{runtime}\"\nbevy = \"0.16\"\n"));
        let table = "clarity-runtime = { version = \"0.0.1\", features = [] }";
        assert_eq!(with_runtime_version(table).unwrap(), format!("clarity-runtime = {{ version = \"{runtime}\", features = [] }}"));
        // Already right, or the person's own source: left alone.
        assert_eq!(with_runtime_version(&format!("clarity-runtime = \"{runtime}\"\n")), None);
        assert_eq!(with_runtime_version("clarity-runtime = { path = \"../rust\" }"), None);
    }

    #[test]
    fn a_part_takes_clarity_ui_as_cargo_does() {
        // 0.x: the minor number decides, as for Cargo's caret requirement.
        assert!(ui_accepts("0.2.0", "0.2.0"));
        assert!(ui_accepts("0.2", "0.2.3"));
        assert!(ui_accepts("^0.2.1", "0.2.4"));
        assert!(!ui_accepts("0.1.0", "0.2.0"));
        assert!(!ui_accepts("0.2.5", "0.2.0"));
        assert!(!ui_accepts("=0.3.0", "0.2.0"));
        // 1 and up: the major number decides.
        assert!(ui_accepts("1.0", "1.4.2"));
        assert!(!ui_accepts("1.0", "2.0.0"));
        // What it cannot read is left to Cargo.
        assert!(ui_accepts("*", "0.2.0"));
    }
}
