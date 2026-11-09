//! The windows of a program as markup (DESIGN 7.8, 7.9): one XML file per
//! window, `ui/windows/Main.cxaml`, each part an element with its options as
//! attributes, written in Clarity's own words, as C# writes each window as
//! AXAML. Clarity.UI reads them when the program starts, on the desktop and
//! in the browser, so building a program needs no tools of the web's own.
//!
//! A part writes only what differs from its kind's defaults (the catalog's
//! `new_props`, which Clarity.UI reads from `catalog/part-defaults.json` in
//! runtimes/rust-ui), so a window reads like its Clarity. Every value is
//! text: text as it is, a number or true and false as written, a list or a
//! record as JSON; Clarity.UI reads each by its field's type
//! (`catalog/part-fields.json`).


use clarity_spec::cxaml::{self, attribute, kind_name, Marks};
use clarity_spec::model::{App, CommandKind, Page, PageKind, Project, Side};
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::xml::escaped;
use serde_json::Value;

use crate::OutFile;

/// Where the window files go in a generated project.
pub const WINDOWS_FOLDER: &str = "ui/windows";

/// Where the app files go: each app's frame (menu, toolbar, ribbon, sidebars, footer).
pub const APPS_FOLDER: &str = "ui/apps";

thread_local! {
    /// Set while the IDE's designer has markup written (`designed`).
    static DESIGNED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The key of each thing the designer's markup marks, by its number
    /// (`clarity_spec::design_keys::numbered`).
    static KEYS: std::cell::RefCell<std::collections::BTreeMap<u32, String>> = std::cell::RefCell::new(std::collections::BTreeMap::new());
    /// The page each tabs, steps or accordion part shows in the designer, by the part's key, from 0.
    static SHOWING: std::cell::RefCell<std::collections::BTreeMap<String, usize>> = std::cell::RefCell::new(std::collections::BTreeMap::new());
}

/// Writes markup as the IDE's designer draws it (DESIGN 8.1): each window,
/// part, page and menu item with `designItem`, its key (its file and name,
/// `design/Main.cxaml#Main/saveButton`), so the designer's layer knows each
/// box Clarity.UI draws. `keys` gives the key of each number the project's
/// things carry (`design_keys::numbered`); each container in `showing` (by
/// its key) shows the page the designer switched to (from 0), as
/// `designShown` (from 1). A program's own markup never has them.
pub fn designed_showing<R>(keys: &std::collections::BTreeMap<u32, String>, showing: &std::collections::BTreeMap<String, usize>, write: impl FnOnce() -> R) -> R {
    let before = DESIGNED.with(|d| d.replace(true));
    let keys_before = KEYS.with(|k| k.replace(keys.clone()));
    let shown_before = SHOWING.with(|s| s.replace(showing.clone()));
    let written = write();
    DESIGNED.with(|d| d.set(before));
    KEYS.with(|k| *k.borrow_mut() = keys_before);
    SHOWING.with(|s| *s.borrow_mut() = shown_before);
    written
}

/// The key of the thing numbered by `span`, while the designer's markup is written.
fn key_of(span: &clarity_spec::Span) -> Option<String> {
    DESIGNED.with(|d| d.get()).then(|| KEYS.with(|k| k.borrow().get(&span.start).cloned())).flatten()
}

/// ` designItem="key"` while the designer's markup is written, else nothing.
fn design_item(span: &clarity_spec::Span) -> String {
    key_of(span).map(|key| format!(" designItem=\"{}\"", cxaml_escaped(&key))).unwrap_or_default()
}

/// ` designShown="n"` for a container the designer switched to another page.
fn design_shown(span: &clarity_spec::Span) -> String {
    match key_of(span).and_then(|key| SHOWING.with(|s| s.borrow().get(&key).copied())) {
        Some(page) => format!(" designShown=\"{}\"", page + 1),
        None => String::new(),
    }
}

/// A key as an attribute's value.
fn cxaml_escaped(text: &str) -> String {
    text.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;")
}

/// The marks a program's markup or the designer's carries (`designed`).
fn marks() -> Marks<'static> {
    Marks { item: &design_item, shown: &design_shown, source: false }
}

/// A part's element name: its kind in Pascal case, `treeTable` as `TreeTable`.
pub fn element_name(kind: WidgetKind) -> String {
    cxaml::element_name(kind)
}

/// The windows and dialogs of the project as markup, one file each.
pub fn window_files(project: &Project) -> Vec<OutFile> {
    project
        .files
        .iter()
        .flat_map(|f| f.pages.iter().map(move |p| (f.module.as_ref().map_or("", |m| m.name.as_str()), p)))
        .filter(|(_, p)| p.kind != PageKind::View)
        .map(|(module, page)| OutFile { path: format!("{WINDOWS_FOLDER}/{}.cxaml", page.name.name), text: window_markup_in(page, module, project.row_height), bytes: None })
        .collect()
}

/// The apps of the project, each its frame as markup, one file each.
pub fn app_files(project: &Project) -> Vec<OutFile> {
    project
        .files
        .iter()
        .flat_map(|f| &f.apps)
        .map(|app| OutFile { path: format!("{APPS_FOLDER}/{}.cxaml", app.name.name), text: app_markup(app), bytes: None })
        .collect()
}

/// An app as markup: `<App name="Shop">` with its frame; an app with a
/// splash names it and its seconds, as an Avalonia app starts its splash
/// window: `<App name="Shop" splash="Welcome" splashFor="2">`.
pub fn app_markup(app: &App) -> String {
    let mut body = String::new();
    cxaml::frame_elements(&app.frame, 1, &mut body, &marks());
    let splash = match (&app.splash, app.splash_for) {
        (Some(page), Some(ms)) => format!(" splash=\"{}\" splashFor=\"{}\"", escaped(&page.name), seconds(ms)),
        (Some(page), None) => format!(" splash=\"{}\"", escaped(&page.name)),
        (None, _) => String::new(),
    };
    // The title a window without its own shows: a key or a text.
    let title = app.title.as_deref().map(|t| format!(" title=\"{}\"", escaped(t))).unwrap_or_default();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- Generated by Clarity from the app {name}. Do not edit: change the app in Clarity instead. -->\n<App name=\"{name}\"{title}{splash}>\n{body}</App>\n",
        name = escaped(&app.name.name)
    )
}

/// Milliseconds as seconds are written: 2, 1.5, 0.25.
fn seconds(ms: u32) -> String {
    format!("{}.{:03}", ms / 1000, ms % 1000).trim_end_matches('0').trim_end_matches('.').to_string()
}

/// One window as markup, in the source's own dialect (`.cxaml`, DESIGN
/// 7.8): its page as its source writes it, with what the transpiler works
/// out filled in, as Avalonia compiles its `.axaml`: each content's views and
/// each canvas's toolbox with their parts, a tree's levels as its rows, form
/// labels, undo scopes. The root also says what only the program needs: a
/// project's own row height, a splash's window.
pub fn window_markup(page: &Page) -> String {
    window_markup_rows(page, clarity_spec::layout::ROW_HEIGHT)
}

/// The project's row height on a window's root, where it is not the
/// standard one: ` rowHeight="30"`.
pub fn row_height_attribute(row_height: u32) -> String {
    if row_height == clarity_spec::layout::ROW_HEIGHT { String::new() } else { attribute("rowHeight", &Value::from(row_height)) }
}

/// What a window says of its desktop window, as an Avalonia `Window` says it
/// in its XAML. A splash page: its size (when the page names none), no
/// system decorations, centred on the screen, fixed, above the others and not
/// in the taskbar, and none of the app's frame. Other windows declare nothing
/// and keep the system's window.
fn window_look(page: &Page) -> String {
    if page.kind != PageKind::Splash {
        return String::new();
    }
    let size = if page.size.is_some() {
        String::new()
    } else {
        let (width, height) = clarity_spec::layout::SPLASH_SIZE;
        format!(" width=\"{width}\" height=\"{height}\"")
    };
    format!("{size} decorations=\"none\" startup=\"centerScreen\" resizable=\"false\" topmost=\"true\" taskbar=\"false\" frame=\"none\"")
}

/// `window_markup` of a project whose rows are `row_height` high, which the
/// root says when it is not the standard: `rowHeight="30"`; `module` names
/// the page's module in its class.
pub fn window_markup_rows(page: &Page, row_height: u32) -> String {
    window_markup_in(page, "", row_height)
}

/// `window_markup_rows` of a page of `module`.
pub fn window_markup_in(page: &Page, module: &str, row_height: u32) -> String {
    // A page's own exact grid (`grid 56 columns rows 10`) says its cells; else the project's rows.
    let rows = if page.cell.is_some() { String::new() } else { row_height_attribute(row_height) };
    let root = cxaml::page_root(page, module) + &rows + &window_look(page) + &design_item(&page.span);
    let name = cxaml::root_name(page.kind);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- Generated by Clarity from {kind} {page}: its .cxaml with what Clarity works out filled in. Do not edit: change {kind} in Clarity instead. -->\n<{name}{root}>\n{body}</{name}>\n",
        kind = if page.kind == PageKind::View { "the view" } else { "the window" },
        page = escaped(&page.name.name),
        body = cxaml::page_body(page, &marks())
    )
}

/// A new menu item's defaults as the screen's markup reader reads them (`command-defaults.json`).
pub fn command_defaults() -> String {
    serde_json::to_string_pretty(&clarity_spec::frame::new_command(CommandKind::Item, None)).unwrap_or_default() + "\n"
}

/// A new sidebar's defaults on each side, as the screen's markup reader reads them (`sidebar-defaults.json`).
pub fn sidebar_defaults() -> String {
    let mut all = serde_json::Map::new();
    for side in [Side::Left, Side::Right] {
        let props = clarity_spec::frame::new_sidebar(side, None);
        let name = serde_json::to_value(side).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
        all.insert(name, serde_json::to_value(props).unwrap_or(Value::Null));
    }
    serde_json::to_string_pretty(&Value::Object(all)).unwrap_or_default() + "\n"
}

/// Every kind's defaults as the screen's markup reader reads them (`part-defaults.json`).
pub fn part_defaults() -> String {
    let mut all = serde_json::Map::new();
    for info in widgets::catalog() {
        all.insert(kind_name(info.kind), serde_json::to_value(widgets::new_props(info.kind, 1, 1)).unwrap_or(Value::Null));
    }
    serde_json::to_string_pretty(&Value::Object(all)).unwrap_or_default() + "\n"
}

/// How the markup reader reads each field of a type, from the TypeScript
/// ts-rs writes for it (`WidgetProps.ts`): `text`, `number`, `bool`, or
/// `json` for a list, a record or a name of the model's own (an enum),
/// which is JSON when it reads as JSON and text otherwise.
pub fn field_kinds(typescript: &str) -> String {
    // The comments out, then the fields between the type's outer braces.
    let mut code = String::new();
    let mut rest = typescript;
    while let Some(start) = rest.find("/*") {
        code.push_str(&rest[..start]);
        rest = rest[start..].find("*/").map_or("", |end| &rest[start + end + 2..]);
    }
    code.push_str(rest);
    let code: String = code.lines().filter(|l| !l.trim_start().starts_with("//") && !l.trim_start().starts_with("import ")).collect::<Vec<_>>().join("\n");
    let body = code.find('{').and_then(|open| code.rfind('}').map(|close| &code[open + 1..close])).unwrap_or_default();
    let mut fields = serde_json::Map::new();
    let (mut depth, mut field) = (0, String::new());
    for c in body.chars().chain(std::iter::once(',')) {
        match c {
            '{' | '<' | '[' | '(' => depth += 1,
            '}' | '>' | ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                if let Some((name, kind)) = field.split_once(':') {
                    let kind = match kind.trim() {
                        "string" | "string | null" => "text",
                        // A list of names, written as Clarity writes it: `title, kind`.
                        "Array<string>" => "texts",
                        "number" | "number | null" | "bigint" | "bigint | null" => "number",
                        "boolean" => "bool",
                        _ => "json",
                    };
                    fields.insert(name.trim().trim_end_matches('?').to_string(), Value::from(kind));
                }
                field.clear();
                continue;
            }
            _ => {}
        }
        field.push(c);
    }
    serde_json::to_string_pretty(&Value::Object(fields)).unwrap_or_default() + "\n"
}
