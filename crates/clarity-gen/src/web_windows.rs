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

use std::fmt::Write as _;

use clarity_spec::model::{App, Command, CommandKind, Footer, Frame, Menu, Page, PageKind, Project, Ribbon, Side, Sidebar, Toolbar, Widget, WidgetPage};
use clarity_spec::widgets::{self, WidgetKind};
use serde_json::Value;

use crate::OutFile;

/// Where the window files go in a generated project.
pub const WINDOWS_FOLDER: &str = "ui/windows";

/// Where the app files go: each app's frame (menu, toolbar, ribbon, sidebars, footer).
pub const APPS_FOLDER: &str = "ui/apps";

/// A field of a part's props and the word its attribute is written with,
/// where Clarity writes it differently. Every other field keeps its name.
const WORDS: [(&str, &str); 4] = [
    ("shortcut", "key"),
    ("tabOrder", "taborder"),
    // A tree's `children` field, which is no child element.
    ("children", "childList"),
    ("onClick", "opens"),
];

/// Fields written in their own form, not compared with the defaults:
/// the kind (the element), the place (`at`, `span`, `rows`), the handlers
/// (`onclick`), and what the project works out itself and no screen reads
/// (`made`). Where a part's edits are undone (`undoScope`), which the project
/// also works out, is written, for a screen that reads the markup alone.
const OWN_FORM: [&str; 8] = ["kind", "name", "col", "row", "width", "height", "handlers", "made"];

thread_local! {
    /// Set while the IDE's designer has markup written (`designed`).
    static DESIGNED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The page each tabs, steps or accordion part shows in the designer, by the part's item, from 0.
    static SHOWING: std::cell::RefCell<std::collections::BTreeMap<u32, usize>> = std::cell::RefCell::new(std::collections::BTreeMap::new());
}

/// Writes markup as the IDE's designer draws it (DESIGN 8.1): each window,
/// part, page and menu item with `designItem`, where it is written in the
/// project, so the designer's layer knows each box Clarity.UI draws. A
/// program's own markup never has it.
pub fn designed<R>(write: impl FnOnce() -> R) -> R {
    designed_showing(&std::collections::BTreeMap::new(), write)
}

/// `designed`, each container in `showing` (by its item) on the page the
/// designer switched to (from 0), as `designShown` (from 1).
pub fn designed_showing<R>(showing: &std::collections::BTreeMap<u32, usize>, write: impl FnOnce() -> R) -> R {
    let before = DESIGNED.with(|d| d.replace(true));
    let shown_before = SHOWING.with(|s| s.replace(showing.clone()));
    let written = write();
    DESIGNED.with(|d| d.set(before));
    SHOWING.with(|s| *s.borrow_mut() = shown_before);
    written
}

/// ` designItem="start"` while the designer's markup is written, else nothing.
fn design_item(span: &clarity_spec::Span) -> String {
    if DESIGNED.with(|d| d.get()) && span.start != SHOWN_ONLY { format!(" designItem=\"{}\"", span.start) } else { String::new() }
}

/// The start of a part's span that the designer shows but does not design
/// there: a view's parts drawn in a window's content, designed in the view.
pub const SHOWN_ONLY: u32 = u32::MAX;

/// The windows and dialogs of the project as markup, one file each.
pub fn window_files(project: &Project) -> Vec<OutFile> {
    project
        .files
        .iter()
        .flat_map(|f| &f.pages)
        .filter(|p| p.kind != PageKind::View)
        .map(|page| OutFile { path: format!("{WINDOWS_FOLDER}/{}.cxaml", page.name.name), text: window_markup_rows(page, project.row_height), bytes: None })
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
    frame_elements(&app.frame, 1, &mut body);
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

/// A frame's regions: its menu bar, toolbar, ribbon, sidebars and footer.
fn frame_elements(frame: &Frame, depth: usize, out: &mut String) {
    if let Some(menu) = &frame.menu {
        menu_element(menu, depth, out);
    }
    if let Some(toolbar) = &frame.toolbar {
        toolbar_element(toolbar, depth, out);
    }
    if let Some(ribbon) = &frame.ribbon {
        ribbon_element(ribbon, depth, out);
    }
    for sidebar in &frame.sidebars {
        sidebar_element(sidebar, depth, out);
    }
    if let Some(footer) = &frame.footer {
        footer_element(footer, depth, out);
    }
}

fn toolbar_element(toolbar: &Toolbar, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let name = toolbar.name.as_ref().map(|n| attribute("name", &Value::String(n.clone()))).unwrap_or_default();
    let _ = writeln!(out, "{pad}<Toolbar{name}>");
    for item in &toolbar.items {
        command_element(item, depth + 1, out);
    }
    let _ = writeln!(out, "{pad}</Toolbar>");
}

/// A ribbon: its tabs, each with its groups of items.
fn ribbon_element(ribbon: &Ribbon, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let mut attributes = ribbon.name.as_ref().map(|n| attribute("name", &Value::String(n.clone()))).unwrap_or_default();
    for (word, on) in [("collapsible", ribbon.collapsible), ("collapsed", ribbon.collapsed)] {
        if on {
            attributes.push_str(&attribute(word, &Value::Bool(true)));
        }
    }
    let _ = writeln!(out, "{pad}<Ribbon{attributes}>");
    for tab in &ribbon.tabs {
        let name = tab.name.as_ref().map(|n| attribute("name", &Value::String(n.clone()))).unwrap_or_default();
        let _ = writeln!(out, "{pad}  <Tab{}{name}>", attribute("title", &Value::String(tab.title.clone())));
        for group in &tab.groups {
            let name = group.name.as_ref().map(|n| attribute("name", &Value::String(n.clone()))).unwrap_or_default();
            let _ = writeln!(out, "{pad}    <Group{}{name}>", attribute("title", &Value::String(group.title.clone())));
            for item in &group.items {
                command_element(item, depth + 3, out);
            }
            let _ = writeln!(out, "{pad}    </Group>");
        }
        let _ = writeln!(out, "{pad}  </Tab>");
    }
    let _ = writeln!(out, "{pad}</Ribbon>");
}

/// A sidebar: what differs from a new sidebar on its side, and its parts.
fn sidebar_element(sidebar: &Sidebar, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let defaults = serde_json::to_value(clarity_spec::frame::new_sidebar(sidebar.props.side, None)).unwrap_or(Value::Null);
    let mut attributes = String::new();
    for (field, value) in in_order(&serde_json::to_string(&sidebar.props).unwrap_or_default()) {
        // Its side always, for the reader to start from that side's sidebar.
        if field != "side" && defaults.get(&field) == Some(&value) {
            continue;
        }
        attributes.push_str(&attribute(&field, &value));
    }
    let _ = writeln!(out, "{pad}<Sidebar{attributes}>");
    for widget in &sidebar.widgets {
        part(widget, depth + 1, out);
    }
    let _ = writeln!(out, "{pad}</Sidebar>");
}

/// The footer: its status, its columns and its parts.
fn footer_element(footer: &Footer, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let mut attributes = footer.status.as_ref().map(|s| attribute("status", &Value::String(s.clone()))).unwrap_or_default();
    attributes.push_str(&attribute("columns", &Value::from(footer.columns)));
    if footer.widgets.is_empty() {
        let _ = writeln!(out, "{pad}<Footer{attributes} />");
        return;
    }
    let _ = writeln!(out, "{pad}<Footer{attributes}>");
    for widget in &footer.widgets {
        part(widget, depth + 1, out);
    }
    let _ = writeln!(out, "{pad}</Footer>");
}

/// One window as markup: `<Window name="Main">` with its own frame, if it
/// has one, and an element per part. A sound and a timer are written too,
/// though they have no place on the screen: a screen that reads only the
/// markup (Clarity.UI) runs them from here, and its grid leaves them out. A grid of other than twelve columns
/// says so on the root: `<Window name="Main" columns="6">`.
pub fn window_markup(page: &Page) -> String {
    window_markup_rows(page, clarity_spec::layout::ROW_HEIGHT)
}

/// The project's row height on a window's root, where it is not the
/// standard one: ` rowHeight="30"`.
pub fn row_height_attribute(row_height: u32) -> String {
    if row_height == clarity_spec::layout::ROW_HEIGHT { String::new() } else { attribute("rowHeight", &Value::from(row_height)) }
}

/// What a window says of its desktop window, as an Avalonia `Window` says it
/// in its XAML. A splash page: its size, no system decorations (Avalonia's
/// `SystemDecorations="None"`), centred on the screen (`WindowStartupLocation`),
/// fixed (`CanResize`), above the others (`Topmost`) and not in the taskbar
/// (`ShowInTaskbar`), and it carries none of the app's frame (`frame="none"`:
/// no menu, toolbar or ribbon, as an Avalonia window has only what its XAML
/// holds). Other windows declare nothing and keep the system's window.
fn window_look(page: &Page) -> String {
    if page.kind != PageKind::Splash {
        return String::new();
    }
    let (width, height) = page.size.map_or(clarity_spec::layout::SPLASH_SIZE, |s| (s.width, s.height));
    format!(" width=\"{width}\" height=\"{height}\" decorations=\"none\" startup=\"centerScreen\" resizable=\"false\" topmost=\"true\" taskbar=\"false\" frame=\"none\"")
}

/// `window_markup` of a project whose rows are `row_height` high, which
/// the root says when it is not the standard: `<Window name="Main" rowHeight="30">`.
pub fn window_markup_rows(page: &Page, row_height: u32) -> String {
    let columns = if page.columns == clarity_spec::model::DEFAULT_PAGE_COLUMNS { String::new() } else { attribute("columns", &Value::from(page.columns)) };
    let style = page.style.as_deref().map(|s| attribute("style", &Value::from(s))).unwrap_or_default();
    // A page's own exact grid (`grid 56 columns rows 10`) says its cells; else the project's rows.
    let rows = match page.cell {
        Some(cell) => attribute("cell", &Value::from(cell)),
        None => row_height_attribute(row_height),
    };
    let columns = columns + &rows + &style + &window_look(page);
    let mut body = String::new();
    frame_elements(&page.frame, 1, &mut body);
    for widget in &page.widgets {
        part(widget, 1, &mut body);
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- Generated by Clarity from the window {name}. Do not edit: change the window in Clarity instead. -->\n<Window name=\"{name}\"{columns}{item}>\n{body}</Window>\n",
        name = escaped(&page.name.name),
        item = design_item(&page.span)
    )
}

/// A part's kind as the model names it: `treeTable`, `custom`.
fn kind_name(kind: WidgetKind) -> String {
    serde_json::to_value(kind).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

/// A part's element name: its kind in Pascal case, `treeTable` as `TreeTable`.
pub fn element_name(kind: WidgetKind) -> String {
    let word = kind_name(kind);
    let mut chars = word.chars();
    chars.next().map(|c| c.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
}

fn part(widget: &Widget, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let p = &widget.props;
    let element = element_name(p.kind);
    let attributes = attributes(widget);
    let inner = !widget.children.is_empty() || !widget.pages.is_empty() || widget.menu.is_some();
    if !inner {
        let _ = writeln!(out, "{pad}<{element}{attributes} />");
        return;
    }
    let _ = writeln!(out, "{pad}<{element}{attributes}>");
    if let Some(menu) = &widget.menu {
        menu_element(menu, depth + 1, out);
    }
    for child in &widget.children {
        part(child, depth + 1, out);
    }
    for page in &widget.pages {
        page_element(p.kind, page, depth + 1, out);
    }
    let _ = writeln!(out, "{pad}</{element}>");
}

/// A part's menu (DESIGN 5.16): `<Menu>` with an `<Item>` per command.
fn menu_element(menu: &Menu, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let name = menu.name.as_ref().map(|n| attribute("name", &Value::String(n.clone()))).unwrap_or_default();
    let _ = writeln!(out, "{pad}<Menu{name}>");
    for item in &menu.items {
        command_element(item, depth + 1, out);
    }
    let _ = writeln!(out, "{pad}</Menu>");
}

/// A command of a menu: what differs from a new item, in Clarity's words,
/// and the items of a submenu inside it.
fn command_element(command: &Command, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let defaults = serde_json::to_value(clarity_spec::frame::new_command(CommandKind::Item, None)).unwrap_or(Value::Null);
    let mut attributes = String::new();
    for (field, value) in in_order(&serde_json::to_string(&command.props).unwrap_or_default()) {
        if defaults.get(&field) == Some(&value) {
            continue;
        }
        let word = match field.as_str() {
            "shortcut" => "key",
            "onClick" => "opens",
            "handler" => "onclick",
            other => other,
        };
        attributes.push_str(&attribute(word, &value));
    }
    attributes.push_str(&design_item(&command.span));
    if command.items.is_empty() {
        let _ = writeln!(out, "{pad}<Item{attributes} />");
        return;
    }
    let _ = writeln!(out, "{pad}<Item{attributes}>");
    for item in &command.items {
        command_element(item, depth + 1, out);
    }
    let _ = writeln!(out, "{pad}</Item>");
}

/// A tab, step, section or pane, with the parts on it.
fn page_element(kind: WidgetKind, page: &WidgetPage, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let element = match kind {
        WidgetKind::Steps => "Step",
        WidgetKind::Accordion => "Section",
        WidgetKind::Splitter => "Pane",
        _ => "Tab",
    };
    let mut attributes = String::new();
    if let Some(title) = &page.title {
        attributes.push_str(&attribute("title", &Value::String(title.clone())));
    }
    // A section's header in a color of its own.
    if let Some(color) = &page.color {
        attributes.push_str(&attribute("color", &Value::String(color.clone())));
    }
    // A splitter's pane starts as wide as its span; a tab, step or section showing a view keeps the view's columns.
    if let Some(columns) = page.columns {
        let name = if kind == WidgetKind::Splitter { "span" } else { "columns" };
        attributes.push_str(&attribute(name, &Value::from(columns)));
    }
    attributes.push_str(&design_item(&page.span));
    if page.children.is_empty() {
        let _ = writeln!(out, "{pad}<{element}{attributes} />");
        return;
    }
    let _ = writeln!(out, "{pad}<{element}{attributes}>");
    for child in &page.children {
        part(child, depth + 1, out);
    }
    let _ = writeln!(out, "{pad}</{element}>");
}

/// The attributes of a part: its name and place first, then what differs
/// from its kind's defaults, then its handlers.
fn attributes(widget: &Widget) -> String {
    let p = &widget.props;
    let mut out = String::new();
    if let Some(name) = &p.name {
        out.push_str(&attribute("name", &Value::String(name.clone())));
    }
    let _ = write!(out, " at=\"{},{}\"", p.col, p.row);
    out.push_str(&attribute("span", &Value::from(p.width)));
    if p.height > 1 {
        out.push_str(&attribute("rows", &Value::from(p.height)));
    }
    let own = in_order(&serde_json::to_string(p).unwrap_or_default());
    let defaults = serde_json::to_value(widgets::new_props(p.kind, p.col, p.row)).unwrap_or(Value::Null);
    // A field its kind gives a value and this part has none of: listed in `none`, which no text can be taken for.
    let mut none = Vec::new();
    if let Value::Object(defaults) = defaults {
        for (field, value) in &own {
            if OWN_FORM.contains(&field.as_str()) || defaults.get(field) == Some(value) {
                continue;
            }
            let word = WORDS.iter().find(|(f, _)| f == field).map_or(field.as_str(), |(_, w)| w);
            if value.is_null() {
                none.push(word.to_string());
            } else {
                out.push_str(&attribute(word, value));
            }
        }
    }
    if !none.is_empty() {
        out.push_str(&attribute("none", &Value::String(none.join(" "))));
    }
    for h in &p.handlers {
        out.push_str(&attribute(&format!("on{}", h.event.keyword()), &Value::String(widgets::handler_call(h))));
    }
    out.push_str(&design_item(&widget.span));
    if let Some(page) = DESIGNED.with(|d| d.get()).then(|| SHOWING.with(|s| s.borrow().get(&widget.span.start).copied())).flatten() {
        let _ = write!(out, " designShown=\"{}\"", page + 1);
    }
    out
}

/// The fields of a serialized object in the order they were written, which
/// is the order of the props struct and of a part's line in Clarity.
fn in_order(json: &str) -> Vec<(String, Value)> {
    struct Fields;
    impl<'de> serde::de::Visitor<'de> for Fields {
        type Value = Vec<(String, Value)>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an object")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut fields = Vec::new();
            while let Some(entry) = map.next_entry::<String, Value>()? {
                fields.push(entry);
            }
            Ok(fields)
        }
    }
    let mut reader = serde_json::Deserializer::from_str(json);
    serde::Deserializer::deserialize_map(&mut reader, Fields).unwrap_or_default()
}

/// ` word="value"`: text as it is, a list of names as Clarity writes it
/// (`title, kind`), anything else as JSON, escaped for XML.
fn attribute(word: &str, value: &Value) -> String {
    let text = match value {
        Value::String(s) => s.clone(),
        Value::Array(items) if !items.is_empty() && items.iter().all(|i| i.as_str().is_some_and(|s| !s.contains(','))) => {
            items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")
        }
        other => other.to_string(),
    };
    format!(" {word}=\"{}\"", escaped(&text))
}

/// Text as an XML attribute holds it: the characters XML gives a meaning, and
/// line ends and tabs, which an attribute would otherwise read as spaces.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            c => out.push(c),
        }
    }
    out
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
