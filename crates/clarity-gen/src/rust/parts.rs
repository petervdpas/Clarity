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

// -- On Clarity.UI (DESIGN 7.9) ---------------------------------------------
//
// A part of a program on Clarity.UI is a control drawn on Clarity.UI's own
// surface, not a native window: its crate's type implements the typed trait
// written into its `src/clarity.rs`, which wraps it as a `CustomPart` of
// clarity-ui; the program registers it by its name.

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

/// `parts/Gauge/src/clarity.rs` of a part on Clarity.UI: the typed trait the
/// crate's `Gauge` implements, the part's words to its window, and the
/// wrapper that makes it a control of clarity-ui.
pub fn ui_interface(part: &PartDecl) -> String {
    let n = &part.name.name;
    let mut shows = String::new();
    let mut sets = String::new();
    if let Some(binds) = &part.binds {
        let _ = write!(shows, "\n    /// The value it shows, from what it binds; given when its window starts and when it changed.\n    fn show_value(&mut self, value: {}, cx: &mut CustomCx);\n", rust_type(binds));
        let _ = writeln!(sets, "            \"value\" => self.0.show_value({}(value), cx),", ui_reader(binds));
    }
    for property in &part.properties {
        let p = snake(&property.name.name);
        let _ = write!(shows, "\n    /// Property {}, as the window or code sets it.\n    fn show_{p}(&mut self, {p}: {}, cx: &mut CustomCx);\n", property.name.name, rust_type(&property.ty));
        let _ = writeln!(sets, "            {:?} => self.0.show_{p}({}(value), cx),", property.name.name, ui_reader(&property.ty));
    }
    let mut hears = String::new();
    for hear in &part.hears {
        let _ = write!(shows, "\n    /// What code tells it: {}.\n    fn on_{}(&mut self{}, cx: &mut CustomCx);\n", hear.name.name, snake(&hear.name.name), ui_params(&hear.params, false));
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
        format!("\n/// What part {n} says to its window, from any of its methods: `cx.changed(..)` and one per tell.\npub trait {n}Tells {{\n{tells_trait}}}\n\nimpl {n}Tells for CustomCx<'_> {{\n{tells_impl}}}\n")
    };
    let switch = |on: &str, unused: &str, cases: &str| if cases.is_empty() { format!("        let _ = ({unused}, cx);\n") } else { format!("        match {on} {{\n{cases}            _ => {{}}\n        }}\n") };
    format!(
        "//! Custom part {n} (DESIGN 5.19, 7.9): a control of Clarity.UI. The trait
//! the crate's `{n}` implements, what it says to its window, and the wrapper
//! that makes it a part of clarity-ui. Written by Clarity from the part's
//! declaration, and written again when the declaration changes; the rest of
//! the crate is yours.
//!
//! Every method is called on the screen's thread, one at a time; the part
//! is made when its window opens and lives as long as its window.

#![allow(dead_code)]

pub use clarity_ui::parts::custom::{{CustomCx, CustomPart}};
pub use clarity_ui::{{KeyInput, Modifiers, Painter, PointerInput, Rect}};
/// Skia, for drawing on the whole canvas (`painter.canvas`), at the version Clarity.UI is built on.
pub use clarity_ui::skia_safe;
use serde_json::Value;

/// What part {n} does: it draws itself in its place and shows what the
/// window gives it; the input methods it does not write do nothing.
pub trait {n}Part: Default + 'static {{
    /// Draws itself in `area`, its place in the window, with the theme's look (`painter.look`).
    fn draw(&self, painter: &Painter, area: Rect);
{shows}
    /// Whether it takes the focus: pressed, or reached with Tab.
    fn focusable(&self) -> bool {{
        false
    }}

    /// The main button went down on it.
    fn pointer_pressed(&mut self, at: PointerInput, cx: &mut CustomCx) {{
        let _ = (at, cx);
    }}

    /// The pointer moved over it, or anywhere while it holds the pointer.
    fn pointer_moved(&mut self, at: PointerInput, cx: &mut CustomCx) {{
        let _ = (at, cx);
    }}

    /// The main button went up after it was pressed on it.
    fn pointer_released(&mut self, at: PointerInput, cx: &mut CustomCx) {{
        let _ = (at, cx);
    }}

    /// Pressed and released with the main button.
    fn click(&mut self, cx: &mut CustomCx) {{
        let _ = cx;
    }}

    /// The wheel turned over it with keys `held`: true when it took the turn.
    fn wheel(&mut self, dx: f64, dy: f64, held: Modifiers, cx: &mut CustomCx) -> bool {{
        let _ = (dx, dy, held, cx);
        false
    }}

    /// A key while it has the focus: true when it took the key.
    fn key(&mut self, key: KeyInput, cx: &mut CustomCx) -> bool {{
        let _ = (key, cx);
        false
    }}

    /// Text typed while it has the focus.
    fn text(&mut self, text: &str, cx: &mut CustomCx) {{
        let _ = (text, cx);
    }}

    /// It got or lost the focus.
    fn focus_changed(&mut self, focused: bool, cx: &mut CustomCx) {{
        let _ = (focused, cx);
    }}

    /// The time it asked for came (`cx.wake(at)`).
    fn wake(&mut self, cx: &mut CustomCx) {{
        let _ = cx;
    }}

    /// Its window opened: called once, after the values of its line, what
    /// code set and what it binds; it may start a timer or an animation here.
    fn start(&mut self, cx: &mut CustomCx) {{
        let _ = cx;
    }}

    /// Its window left the screen: it stops what it started.
    fn stop(&mut self) {{}}

    /// What it says about the place at a point when the pointer rests there.
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {{
        let _ = (x, y);
        None
    }}
}}
{tells}
/// Part {n} as Clarity.UI holds it: the window's words in, typed.
struct Held<P>(P);

impl<P: {n}Part> CustomPart for Held<P> {{
    fn set(&mut self, name: &str, value: &Value, cx: &mut CustomCx) {{
        if name == \"enabled\" {{
            // The window draws it again; `cx.enabled()` says which it is.
            cx.invalidate();
            return;
        }}
{set_switch}    }}

    fn hear(&mut self, hear: &str, values: &[Value], cx: &mut CustomCx) {{
        // What it hears may take no values.
        let _ = values;
{hear_switch}    }}

    fn draw(&self, painter: &Painter, area: Rect) {{
        self.0.draw(painter, area);
    }}

    fn focusable(&self) -> bool {{
        self.0.focusable()
    }}

    fn pointer_pressed(&mut self, at: PointerInput, cx: &mut CustomCx) {{
        self.0.pointer_pressed(at, cx);
    }}

    fn pointer_moved(&mut self, at: PointerInput, cx: &mut CustomCx) {{
        self.0.pointer_moved(at, cx);
    }}

    fn pointer_released(&mut self, at: PointerInput, cx: &mut CustomCx) {{
        self.0.pointer_released(at, cx);
    }}

    fn click(&mut self, cx: &mut CustomCx) {{
        self.0.click(cx);
    }}

    fn wheel(&mut self, dx: f64, dy: f64, held: Modifiers, cx: &mut CustomCx) -> bool {{
        self.0.wheel(dx, dy, held, cx)
    }}

    fn key(&mut self, key: KeyInput, cx: &mut CustomCx) -> bool {{
        self.0.key(key, cx)
    }}

    fn text(&mut self, text: &str, cx: &mut CustomCx) {{
        self.0.text(text, cx);
    }}

    fn focus_changed(&mut self, focused: bool, cx: &mut CustomCx) {{
        self.0.focus_changed(focused, cx);
    }}

    fn wake(&mut self, cx: &mut CustomCx) {{
        self.0.wake(cx);
    }}

    fn start(&mut self, cx: &mut CustomCx) {{
        self.0.start(cx);
    }}

    fn stop(&mut self) {{
        self.0.stop();
    }}

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {{
        self.0.tooltip_at(x, y)
    }}
}}

/// A new part {n}, as the program registers it with Clarity.UI.
pub fn make() -> Box<dyn CustomPart> {{
    Box::new(Held(crate::{n}::default()))
}}

{readers}",
        set_switch = switch("name", "value", &sets),
        hear_switch = switch("hear", "hear, values", &hears),
        readers = ui_readers(part),
    )
}

/// What declaring a Rust part writes into its folder on Clarity.UI: a crate
/// on clarity-ui whose type draws its value in a frame until it draws the
/// library's work.
pub fn ui_starter(part: &PartDecl) -> Vec<(String, String)> {
    let n = &part.name.name;
    let package = snake(n);
    let mut deps = format!("clarity-ui = \"{}\"\nserde_json = \"1\"\n", super::ui_version());
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
    fn draw(&self, painter: &Painter, area: Rect) {{
        let look = painter.look;
        painter.stroke(area, look.color(\"border\"), look.border(), look.radius(\"input\"));
        painter.text(&self.shown, area.inset(8.0), look.color(\"text\"), look.size(\"text\"), \"text\", false, clarity_ui::TextAlign::Left);
    }}
"
    );
    if let Some(binds) = &part.binds {
        let _ = write!(lib, "\n    fn show_value(&mut self, value: {}, cx: &mut CustomCx) {{\n        self.shown = {};\n        cx.invalidate();\n    }}\n", rust_type(binds), shown(binds));
    }
    for property in &part.properties {
        let p = snake(&property.name.name);
        let _ = write!(lib, "\n    fn show_{p}(&mut self, {p}: {}, cx: &mut CustomCx) {{\n        let _ = ({p}, cx);\n    }}\n", rust_type(&property.ty));
    }
    for hear in &part.hears {
        let mut names: Vec<String> = hear.params.iter().map(|p| snake(&p.name.name)).collect();
        names.push("cx".into());
        let _ = write!(lib, "\n    fn on_{}(&mut self{}, cx: &mut CustomCx) {{\n        let _ = ({});\n    }}\n", snake(&hear.name.name), ui_params(&hear.params, false), names.join(", "));
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

/// `src/parts.rs` of a program on Clarity.UI: each part registered with
/// clarity-ui by its declaration's name.
pub fn ui_glue(project: &Project) -> String {
    let mut registers = String::new();
    for part in crate_parts(project) {
        let n = &part.name.name;
        let package = package_of(project, n).replace('-', "_");
        let _ = writeln!(registers, "    clarity_ui::parts::custom::register({n:?}, {package}::make);");
    }
    format!("// The program's custom parts written in Rust, controls of Clarity.UI (DESIGN 7.9).\n// Generated by Clarity.\n\n/// Tells Clarity.UI about every kind of part the program has, by its name.\npub fn register() {{\n{registers}}}\n")
}
