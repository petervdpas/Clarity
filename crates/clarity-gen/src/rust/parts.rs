//! Custom parts written in Rust (DESIGN 5.19 and 6.5): a part is a crate of
//! the project (`crate "parts/Arena"`), which implements the trait Clarity
//! writes from the declaration into its `src/clarity.rs`. That file is all
//! the crate needs of Clarity: the trait, the window it draws in (with the raw
//! handle wgpu and Bevy take) and its typed tells. The program's glue makes
//! each kind of part for the runtime's part host and passes it its values,
//! hears and tells.

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

/// The `Told` a tell's value becomes.
fn told(ty: &TypeRef, name: &str) -> String {
    match ty.kind {
        TypeRefKind::Integer | TypeRefKind::Duration => format!("Told::Integer({name})"),
        TypeRefKind::Bool => format!("Told::Bool({name})"),
        TypeRefKind::Decimal | TypeRefKind::Money => format!("Told::Text({name}.to_string())"),
        TypeRefKind::Date | TypeRefKind::DateTime => format!("{name}.map_or(Told::Empty, |v| Told::Text(v.to_string()))"),
        _ => format!("Told::Text({name}.to_string())"),
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

/// `parts/Arena/src/clarity.rs`: what the crate implements and calls.
pub fn interface(part: &PartDecl) -> String {
    let n = &part.name.name;
    let mut out = format!(
        "//! Custom part {n} (DESIGN 5.19): the trait the crate's `{n}` implements, the
//! window it draws in and the tells it calls. Written by Clarity from the
//! part's declaration, and written again when the declaration changes; the
//! rest of the crate is yours.
//!
//! Every method is called on the part host's thread, one at a time; the part
//! is made there and lives there.

#![allow(dead_code)]

use std::ffi::c_void;

/// The native window the part draws in: on Linux an X11 window, on the
/// part host's own X11 connection. Its raw handle is what wgpu, Bevy and
/// softbuffer take.
#[derive(Debug, Clone, Copy)]
pub struct Window {{
    pub xid: u64,
    pub display: *mut c_void,
    pub screen: i32,
}}

impl raw_window_handle::HasWindowHandle for Window {{
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {{
        let handle = raw_window_handle::XlibWindowHandle::new(self.xid as std::ffi::c_ulong);
        // The window lives as long as the part: the host ends the part first.
        Ok(unsafe {{ raw_window_handle::WindowHandle::borrow_raw(handle.into()) }})
    }}
}}

impl raw_window_handle::HasDisplayHandle for Window {{
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {{
        let handle = raw_window_handle::XlibDisplayHandle::new(std::ptr::NonNull::new(self.display), self.screen);
        Ok(unsafe {{ raw_window_handle::DisplayHandle::borrow_raw(handle.into()) }})
    }}
}}

/// A value a tell gives.
#[derive(Debug, Clone, PartialEq)]
pub enum Told {{
    Integer(i64),
    Bool(bool),
    Text(String),
    Empty,
}}

/// What part {n} does.
pub trait {n}Part {{
    /// Makes the part in `window`, `width` by `height` pixels.
    fn create(window: Window, width: u32, height: u32, host: {n}Host) -> Self
    where
        Self: Sized;
    /// The window changed size.
    fn resize(&mut self, width: u32, height: u32) {{
        let _ = (width, height);
    }}
    /// About 60 times a second while the part shows: draw, and read input.
    fn frame(&mut self) {{}}
    /// Whether the person can use it, as code sets `enabled`.
    fn set_enabled(&mut self, enabled: bool) {{
        let _ = enabled;
    }}
"
    );
    if let Some(binds) = &part.binds {
        let _ = writeln!(out, "    /// The value it shows, from what it binds.\n    fn set_value(&mut self, value: {});", rust_type(binds));
    }
    for property in &part.properties {
        let p = snake(&property.name.name);
        let _ = writeln!(out, "    /// Property {}, as the window or code sets it.\n    fn set_{p}(&mut self, {p}: {});", property.name.name, rust_type(&property.ty));
    }
    for hear in &part.hears {
        let _ = writeln!(out, "    /// What code tells it: {}.\n    fn {}(&mut self{});", hear.name.name, snake(&hear.name.name), params(&hear.params, false));
    }
    let _ = write!(
        out,
        "}}\n\n/// Where part {n} tells what happened.\npub struct {n}Host {{\n    tell: Box<dyn Fn(&str, Vec<Told>)>,\n}}\n\nimpl {n}Host {{\n    /// Made by the program for each part it shows.\n    pub fn new(tell: Box<dyn Fn(&str, Vec<Told>)>) -> Self {{\n        {n}Host {{ tell }}\n    }}\n"
    );
    if let Some(binds) = &part.binds {
        let _ = write!(out, "\n    /// The person changed the value: what it binds takes it, and `on change` runs.\n    pub fn changed(&self, value: {}) {{\n        (self.tell)(\"change\", vec![{}]);\n    }}\n", given_type(binds), told(binds, "value"));
    }
    for tell in &part.tells {
        let values: Vec<String> = tell.params.iter().map(|p| told(&p.ty, &snake(&p.name.name))).collect();
        let _ = write!(out, "\n    /// Tells {}: its handler runs with these values.\n    pub fn {}(&self{}) {{\n        (self.tell)({:?}, vec![{}]);\n    }}\n", tell.name.name, snake(&tell.name.name), params(&tell.params, true), tell.name.name, values.join(", "));
    }
    out.push_str("}\n");
    out
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

/// What declaring a Rust part writes into its folder: the crate, which
/// builds and keeps the part's values, drawing nothing yet.
pub fn starter(part: &PartDecl) -> Vec<(String, String)> {
    let n = &part.name.name;
    let package = snake(n);
    let mut deps = String::from("# The window's raw handle, which wgpu and Bevy take.\nraw-window-handle = \"0.6\"\n");
    if uses_decimal(part) {
        deps.push_str("rust_decimal = \"1\"\n");
    }
    if uses_dates(part) {
        deps.push_str("chrono = \"0.4\"\n");
    }
    let cargo = format!(
        "# The crate that draws custom part {n}. Clarity builds it into the program.\n[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n{deps}# To draw with Bevy, add it here and run its App without a window loop,\n# calling app.update() in frame().\n"
    );
    let field = |name: &str, ty: &TypeRef| format!("    {}: {},", snake(name), rust_type(ty));
    let mut fields = vec!["    host: {n}Host,".replace("{n}", n), "    window: Window,".into(), "    width: u32,".into(), "    height: u32,".into(), "    enabled: bool,".into()];
    if let Some(binds) = &part.binds {
        fields.push(field("value", binds));
    }
    for property in &part.properties {
        fields.push(field(&property.name.name, &property.ty));
    }
    let mut inits = vec!["host".to_string(), "window".into(), "width".into(), "height".into(), "enabled: true".into()];
    if part.binds.is_some() {
        inits.push("value: Default::default()".into());
    }
    for property in &part.properties {
        inits.push(format!("{}: Default::default()", snake(&property.name.name)));
    }
    let mut lib = format!(
        "//! Custom part {n}: draws in the native window Clarity gives it. Keep the
//! methods of `{n}Part` (src/clarity.rs), which follow the part's declaration.

mod clarity;
pub use clarity::*;

pub struct {n} {{
{}
}}

impl {n}Part for {n} {{
    fn create(window: Window, width: u32, height: u32, host: {n}Host) -> Self {{
        {n} {{ {} }}
    }}

    fn resize(&mut self, width: u32, height: u32) {{
        self.width = width;
        self.height = height;
    }}

    fn frame(&mut self) {{
        // Draw here, about 60 times a second, and read the mouse and keys.
        let _ = (&self.host, &self.window);
    }}

    fn set_enabled(&mut self, enabled: bool) {{
        self.enabled = enabled;
    }}
",
        fields.join("\n"),
        inits.join(", ")
    );
    if let Some(binds) = &part.binds {
        let _ = write!(lib, "\n    fn set_value(&mut self, value: {}) {{\n        self.value = value;\n    }}\n", rust_type(binds));
    }
    for property in &part.properties {
        let p = snake(&property.name.name);
        let _ = write!(lib, "\n    fn set_{p}(&mut self, {p}: {}) {{\n        self.{p} = {p};\n    }}\n", rust_type(&property.ty));
    }
    for hear in &part.hears {
        let names: Vec<String> = hear.params.iter().map(|p| snake(&p.name.name)).collect();
        let quiet = if names.is_empty() { String::new() } else { format!("\n        let _ = ({});", names.join(", ")) };
        let _ = write!(lib, "\n    fn {}(&mut self{}) {{{quiet}\n    }}\n", snake(&hear.name.name), params(&hear.params, false));
    }
    lib.push_str("}\n");
    vec![("Cargo.toml".into(), cargo), ("src/lib.rs".into(), lib), ("src/clarity.rs".into(), interface(part))]
}

/// The JSON a `Told` value crosses as.
const TOLD_JSON: &str = "fn told_json(told: Told) -> clarity_runtime::parts::Value {\n    use clarity_runtime::parts::Value;\n    match told {\n        Told::Integer(v) => Value::from(v),\n        Told::Bool(v) => Value::from(v),\n        Told::Text(v) => Value::from(v),\n        Told::Empty => Value::Null,\n    }\n}\n";

/// `src/parts.rs` of the program: each part made for the part host, with
/// its values, hears and tells passed to and from its crate.
pub fn glue(project: &Project) -> String {
    let mut out = String::from("// The program's custom parts written in Rust (DESIGN 5.19), for the part host.\n// Generated by Clarity.\n\nuse clarity_runtime::parts::{self, Host, Part, Value};\n");
    let mut registers = String::new();
    for part in crate_parts(project) {
        let n = &part.name.name;
        let package = package_of(project, n).replace('-', "_");
        let glue = format!("{n}Glue");
        let mut sets = String::new();
        if let Some(binds) = &part.binds {
            let _ = writeln!(sets, "            \"value\" => self.0.set_value(parts::read::<{}>(Some(value))),", rust_type(binds));
        }
        for property in &part.properties {
            let _ = writeln!(sets, "            {:?} => self.0.set_{}(parts::read::<{}>(Some(value))),", property.name.name, snake(&property.name.name), rust_type(&property.ty));
        }
        let mut hears = String::new();
        for hear in &part.hears {
            let args: Vec<String> = hear.params.iter().enumerate().map(|(i, p)| format!("parts::read::<{}>(values.get({i}))", rust_type(&p.ty))).collect();
            let _ = writeln!(hears, "            {:?} => self.0.{}({}),", hear.name.name, snake(&hear.name.name), args.join(", "));
        }
        let _ = write!(
            out,
            "
mod {package}_glue {{
    use super::*;
    use {package}::{{{n}, {n}Host, {n}Part, Told}};

    {TOLD_JSON}
    pub struct {glue}(pub {n});

    impl Part for {glue} {{
        fn resize(&mut self, width: u32, height: u32) {{
            self.0.resize(width, height);
        }}

        fn frame(&mut self) {{
            self.0.frame();
        }}

        fn set(&mut self, name: &str, value: &Value) {{
            match name {{
                \"enabled\" => self.0.set_enabled(value.as_bool().unwrap_or(true)),
{sets}                _ => {{}}
            }}
        }}

        #[allow(unused_variables)]
        fn hear(&mut self, name: &str, values: &[Value]) {{
            match name {{
{hears}                _ => {{}}
            }}
        }}
    }}

    pub fn make(window: parts::Window, width: u32, height: u32, host: Host) -> Box<dyn Part> {{
        let window = {package}::Window {{ xid: window.xid, display: window.display, screen: window.screen }};
        let tell = {n}Host::new(Box::new(move |tell, values| host.tell(tell, values.into_iter().map(told_json).collect())));
        Box::new({glue}(<{n} as {n}Part>::create(window, width, height, tell)))
    }}
}}
"
        );
        let _ = writeln!(registers, "    parts::register({n:?}, {package}_glue::make);");
    }
    let _ = write!(out, "\n/// Tells the part host about every kind of part the program has.\npub fn register() {{\n{registers}}}\n");
    out
}
