//! Windows and views as `.cxaml` (docs/CXAML-SOURCE-PLAN.md): the markup
//! Clarity.UI reads on every stack, and the source a page is designed in.
//! `page_source` writes a page as its `.cxaml` file, `read_page` reads one
//! back into the same page, each element's place in the file being its span.
//!
//! A part writes only what differs from its kind's defaults (`new_props`),
//! in Clarity's words (`at`, `span`, `rows`, `key`, `on<event>`). Every value
//! is text: text as it is, a number or true and false as written, a list of
//! names as Clarity writes it (`title, kind`), anything else as JSON.

use std::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::code::{TypeRef, TypeRefKind};
use crate::diagnostic::Diagnostic;
use crate::model::{
    App, Command, CommandKind, DataDecl, DataKind, Footer, Frame, Ident, Menu, Page, PageKind, Ribbon, RibbonGroup, RibbonTab, Sidebar, Toolbar, TreeLevel, TreeSelect, Widget, WidgetPage, WindowSize,
};
use crate::span::Span;
use crate::widgets::{self, WidgetKind};
use crate::xml::{self, escaped, Element};

/// A field of a part's props and the word its attribute is written with,
/// where Clarity writes it differently. Every other field keeps its name.
const WORDS: [(&str, &str); 4] = [
    ("shortcut", "key"),
    ("tabOrder", "taborder"),
    // A tree's `children` field, which is no child element.
    ("children", "childList"),
    ("onClick", "opens"),
];

/// A command's fields written in other words.
const COMMAND_WORDS: [(&str, &str); 3] = [("shortcut", "key"), ("onClick", "opens"), ("handler", "onclick")];

/// Fields written in their own form, not compared with the defaults: the
/// kind (the element), the place (`at`, `span`, `rows`), the handlers
/// (`onclick`), and what the project works out itself (`made`).
const OWN_FORM: [&str; 8] = ["kind", "name", "col", "row", "width", "height", "handlers", "made"];

/// What the markup carries beyond the parts, by who writes it: a program's
/// own markup marks nothing; the designer's marks each box with where it is
/// written (`designItem`) and the page a container shows (`designShown`).
pub struct Marks<'a> {
    /// ` designItem="start"` for a span, or nothing.
    pub item: &'a dyn Fn(&Span) -> String,
    /// ` designShown="n"` for a container's span, or nothing.
    pub shown: &'a dyn Fn(&Span) -> String,
    /// The source form (a window's or view's own file): a content names its
    /// views, a tree keeps its levels.
    pub source: bool,
}

impl Marks<'_> {
    /// No marks: a program's own markup.
    pub const PLAIN: Marks<'static> = Marks { item: &|_| String::new(), shown: &|_| String::new(), source: false };
    /// The source form, without marks.
    pub const SOURCE: Marks<'static> = Marks { item: &|_| String::new(), shown: &|_| String::new(), source: true };
}

/// The root element's name for a page kind: `Window`, `Dialog`, `View`, `Splash`, `Toolbox`.
pub fn root_name(kind: PageKind) -> &'static str {
    match kind {
        PageKind::Window => "Window",
        PageKind::Dialog => "Dialog",
        PageKind::View => "View",
        PageKind::Splash => "Splash",
        PageKind::Toolbox => "Toolbox",
    }
}

fn kind_of_root(name: &str) -> Option<PageKind> {
    [PageKind::Window, PageKind::Dialog, PageKind::View, PageKind::Splash, PageKind::Toolbox].into_iter().find(|k| root_name(*k) == name)
}

/// A page as its own `.cxaml` file: the root names its kind and, as
/// Avalonia's `x:Class` does, its module and name (`class="desk.Desk"`: the
/// page whose code is the block `code Desk` of module `desk`); then its
/// title, size, grid, style, what a dialog gives and its handlers; inside,
/// its data, what it tells, its frame and its parts.
pub fn page_source(page: &Page, module: &str) -> String {
    let name = root_name(page.kind);
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{name}{}>\n{}</{name}>\n", page_root(page, module), page_body(page, &Marks::SOURCE))
}

/// The attributes of a page's root: its class, title, size, grid, style,
/// what a dialog gives and its handlers.
pub fn page_root(page: &Page, module: &str) -> String {
    let class = if module.is_empty() { page.name.name.clone() } else { format!("{module}.{}", page.name.name) };
    let mut root = attribute("class", &Value::String(class));
    if let Some(title) = &page.title {
        root.push_str(&attribute("title", &Value::String(title.clone())));
    }
    if let Some(size) = page.size {
        root.push_str(&attribute("width", &Value::from(size.width)));
        root.push_str(&attribute("height", &Value::from(size.height)));
    }
    if page.columns != crate::model::DEFAULT_PAGE_COLUMNS {
        root.push_str(&attribute("columns", &Value::from(page.columns)));
    }
    if let Some(cell) = page.cell {
        root.push_str(&attribute("cell", &Value::from(cell)));
    }
    if let Some(style) = &page.style {
        root.push_str(&attribute("style", &Value::String(style.clone())));
    }
    if let Some(gives) = &page.gives {
        root.push_str(&attribute("gives", &Value::String(type_text(gives))));
    }
    for h in &page.handlers {
        root.push_str(&attribute(&format!("on{}", h.event.keyword()), &Value::String(widgets::handler_call(h))));
    }
    root
}

/// What a page's root holds: its data, what it tells, its frame and its
/// parts, with the marks of who writes it (a program's own markup, the
/// designer's, or the source).
pub fn page_body(page: &Page, marks: &Marks) -> String {
    let mut body = String::new();
    data_elements(&page.data, &mut body);
    for tell in &page.tells {
        let params: Vec<String> = tell.params.iter().map(|p| format!("{}: {}", p.name.name, type_text(&p.ty))).collect();
        let message = if params.is_empty() { tell.name.name.clone() } else { format!("{}({})", tell.name.name, params.join(", ")) };
        let _ = writeln!(body, "  <Tells{} />", attribute("message", &Value::String(message)));
    }
    frame_elements(&page.frame, 1, &mut body, marks);
    for widget in &page.widgets {
        part(widget, 1, &mut body, marks);
    }
    body
}

/// A `<Data>` element for each data item a page or the app declares (not
/// those the project made itself).
fn data_elements(data: &[DataDecl], out: &mut String) {
    for data in data.iter().filter(|d| !d.made) {
        let mut attributes = attribute("name", &Value::String(data.name.name.clone()));
        let (ty, default) = match &data.of {
            DataKind::Record { entity } => (entity.name.clone(), None),
            DataKind::List { entity } => (format!("list<{}>", entity.name), None),
            DataKind::Value { ty, default } => (type_text(ty), default.as_ref().map(widgets::literal_text)),
        };
        attributes.push_str(&attribute("type", &Value::String(ty)));
        if let Some(default) = default {
            attributes.push_str(&attribute("default", &Value::String(default)));
        }
        let _ = writeln!(out, "  <Data{attributes} />");
    }
}

/// The root element of the app's file.
pub const APP_ROOT: &str = "App";

/// The app as its own `.cxaml` file, `design/<App>.cxaml`: the root names
/// its module and name (`class="shop.Shop"`; its code is the block
/// `code Shop` of module `shop`), then its title, main window, splash and
/// handlers; inside, its data and its frame.
pub fn app_source(app: &App, module: &str) -> String {
    let class = if module.is_empty() { app.name.name.clone() } else { format!("{module}.{}", app.name.name) };
    let mut root = attribute("class", &Value::String(class));
    if let Some(title) = &app.title {
        root.push_str(&attribute("title", &Value::String(title.clone())));
    }
    if let Some(main) = &app.main {
        root.push_str(&attribute("main", &Value::String(main.name.clone())));
    }
    if let Some(splash) = &app.splash {
        root.push_str(&attribute("splash", &Value::String(splash.name.clone())));
    }
    if let Some(ms) = app.splash_for {
        root.push_str(&attribute("splashFor", &Value::String(seconds_text(ms))));
    }
    for h in &app.handlers {
        root.push_str(&attribute(&format!("on{}", h.event.keyword()), &Value::String(widgets::handler_call(h))));
    }
    let mut body = String::new();
    data_elements(&app.data, &mut body);
    frame_elements(&app.frame, 1, &mut body, &Marks::SOURCE);
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{APP_ROOT}{root}>\n{body}</{APP_ROOT}>\n")
}

/// Milliseconds as seconds, as a person writes them: `2`, `1.5`, `0.25`.
fn seconds_text(ms: u32) -> String {
    let text = format!("{:.3}", f64::from(ms) / 1000.0);
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A type as Clarity writes it: `integer`, `Customer`, `list<Order>`.
pub fn type_text(ty: &TypeRef) -> String {
    match &ty.kind {
        TypeRefKind::Text => "text".into(),
        TypeRefKind::Integer => "integer".into(),
        TypeRefKind::Decimal => "decimal".into(),
        TypeRefKind::Money => "money".into(),
        TypeRefKind::Bool => "bool".into(),
        TypeRefKind::Date => "date".into(),
        TypeRefKind::DateTime => "datetime".into(),
        TypeRefKind::Duration => "duration".into(),
        TypeRefKind::File => "file".into(),
        TypeRefKind::Bytes => "bytes".into(),
        TypeRefKind::Named { name } => name.clone(),
        TypeRefKind::List { item } => format!("list<{item}>"),
    }
}

/// A frame's regions: its menu bar, toolbar, ribbon, sidebars and footer.
pub fn frame_elements(frame: &Frame, depth: usize, out: &mut String, marks: &Marks) {
    if let Some(menu) = &frame.menu {
        menu_element(menu, depth, out, marks);
    }
    if let Some(toolbar) = &frame.toolbar {
        toolbar_element(toolbar, depth, out, marks);
    }
    if let Some(ribbon) = &frame.ribbon {
        ribbon_element(ribbon, depth, out, marks);
    }
    for sidebar in &frame.sidebars {
        sidebar_element(sidebar, depth, out, marks);
    }
    if let Some(footer) = &frame.footer {
        footer_element(footer, depth, out, marks);
    }
}

fn name_attribute(name: &Option<String>) -> String {
    name.as_ref().map(|n| attribute("name", &Value::String(n.clone()))).unwrap_or_default()
}

fn toolbar_element(toolbar: &Toolbar, depth: usize, out: &mut String, marks: &Marks) {
    let pad = "  ".repeat(depth);
    let _ = writeln!(out, "{pad}<Toolbar{}>", name_attribute(&toolbar.name));
    for item in &toolbar.items {
        command_element(item, depth + 1, out, marks);
    }
    let _ = writeln!(out, "{pad}</Toolbar>");
}

/// A ribbon: its tabs, each with its groups of items.
fn ribbon_element(ribbon: &Ribbon, depth: usize, out: &mut String, marks: &Marks) {
    let pad = "  ".repeat(depth);
    let mut attributes = name_attribute(&ribbon.name);
    for (word, on) in [("collapsible", ribbon.collapsible), ("collapsed", ribbon.collapsed)] {
        if on {
            attributes.push_str(&attribute(word, &Value::Bool(true)));
        }
    }
    let _ = writeln!(out, "{pad}<Ribbon{attributes}>");
    for tab in &ribbon.tabs {
        let _ = writeln!(out, "{pad}  <Tab{}{}>", attribute("title", &Value::String(tab.title.clone())), name_attribute(&tab.name));
        for group in &tab.groups {
            let _ = writeln!(out, "{pad}    <Group{}{}>", attribute("title", &Value::String(group.title.clone())), name_attribute(&group.name));
            for item in &group.items {
                command_element(item, depth + 3, out, marks);
            }
            let _ = writeln!(out, "{pad}    </Group>");
        }
        let _ = writeln!(out, "{pad}  </Tab>");
    }
    let _ = writeln!(out, "{pad}</Ribbon>");
}

/// A sidebar: what differs from a new sidebar on its side, and its parts.
fn sidebar_element(sidebar: &Sidebar, depth: usize, out: &mut String, marks: &Marks) {
    let pad = "  ".repeat(depth);
    let defaults = serde_json::to_value(crate::frame::new_sidebar(sidebar.props.side, None)).unwrap_or(Value::Null);
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
        part(widget, depth + 1, out, marks);
    }
    let _ = writeln!(out, "{pad}</Sidebar>");
}

/// The footer: its status, its columns and its parts.
fn footer_element(footer: &Footer, depth: usize, out: &mut String, marks: &Marks) {
    let pad = "  ".repeat(depth);
    let mut attributes = footer.status.as_ref().map(|s| attribute("status", &Value::String(s.clone()))).unwrap_or_default();
    attributes.push_str(&attribute("columns", &Value::from(footer.columns)));
    if footer.widgets.is_empty() {
        let _ = writeln!(out, "{pad}<Footer{attributes} />");
        return;
    }
    let _ = writeln!(out, "{pad}<Footer{attributes}>");
    for widget in &footer.widgets {
        part(widget, depth + 1, out, marks);
    }
    let _ = writeln!(out, "{pad}</Footer>");
}

/// A part's kind as the model names it: `treeTable`, `custom`.
pub fn kind_name(kind: WidgetKind) -> String {
    serde_json::to_value(kind).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

/// A part's element name: its kind in Pascal case, `treeTable` as `TreeTable`.
pub fn element_name(kind: WidgetKind) -> String {
    let word = kind_name(kind);
    let mut chars = word.chars();
    chars.next().map(|c| c.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
}

/// The kind an element names, the inverse of `element_name`.
fn kind_of_element(name: &str) -> Option<WidgetKind> {
    let mut chars = name.chars();
    let word = chars.next().map(|c| c.to_ascii_lowercase().to_string() + chars.as_str())?;
    serde_json::from_value(Value::String(word)).ok()
}

/// A part as an element, with its menu, its parts and its pages inside.
pub fn part(widget: &Widget, depth: usize, out: &mut String, marks: &Marks) {
    let pad = "  ".repeat(depth);
    let p = &widget.props;
    let element = element_name(p.kind);
    let attributes = attributes(widget, marks);
    let levels = marks.source && (!widget.levels.is_empty() || !widget.selects.is_empty());
    let inner = !widget.children.is_empty() || !widget.pages.is_empty() || widget.menu.is_some() || levels;
    if !inner {
        let _ = writeln!(out, "{pad}<{element}{attributes} />");
        return;
    }
    let _ = writeln!(out, "{pad}<{element}{attributes}>");
    if let Some(menu) = &widget.menu {
        menu_element(menu, depth + 1, out, marks);
    }
    for child in &widget.children {
        part(child, depth + 1, out, marks);
    }
    for page in &widget.pages {
        page_element(p.kind, page, depth + 1, out, marks);
    }
    if levels {
        for level in &widget.levels {
            level_element(level, depth + 1, out);
        }
        for select in &widget.selects {
            let _ = writeln!(out, "{pad}  <Select{}{} />", attribute("entity", &Value::String(select.entity.clone())), attribute("handler", &Value::String(select.handler.clone())));
        }
    }
    let _ = writeln!(out, "{pad}</{element}>");
}

/// A level of a tree of several kinds of records, or a group heading, with the levels inside.
fn level_element(level: &TreeLevel, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let mut attributes = String::new();
    for (word, value) in [("entity", &level.entity), ("group", &level.group), ("under", &level.under), ("show", &level.show), ("order", &level.order)] {
        if let Some(value) = value {
            attributes.push_str(&attribute(word, &Value::String(value.clone())));
        }
    }
    if level.descending {
        attributes.push_str(&attribute("descending", &Value::Bool(true)));
    }
    if level.levels.is_empty() {
        let _ = writeln!(out, "{pad}<Level{attributes} />");
        return;
    }
    let _ = writeln!(out, "{pad}<Level{attributes}>");
    for inner in &level.levels {
        level_element(inner, depth + 1, out);
    }
    let _ = writeln!(out, "{pad}</Level>");
}

/// A part's menu (DESIGN 5.16): `<Menu>` with an `<Item>` per command.
pub fn menu_element(menu: &Menu, depth: usize, out: &mut String, marks: &Marks) {
    let pad = "  ".repeat(depth);
    let _ = writeln!(out, "{pad}<Menu{}>", name_attribute(&menu.name));
    for item in &menu.items {
        command_element(item, depth + 1, out, marks);
    }
    let _ = writeln!(out, "{pad}</Menu>");
}

/// A command of a menu: what differs from a new item, in Clarity's words,
/// and the items of a submenu inside it.
fn command_element(command: &Command, depth: usize, out: &mut String, marks: &Marks) {
    let pad = "  ".repeat(depth);
    let defaults = serde_json::to_value(crate::frame::new_command(CommandKind::Item, None)).unwrap_or(Value::Null);
    let mut attributes = String::new();
    for (field, value) in in_order(&serde_json::to_string(&command.props).unwrap_or_default()) {
        if defaults.get(&field) == Some(&value) {
            continue;
        }
        let word = COMMAND_WORDS.iter().find(|(f, _)| *f == field).map_or(field.as_str(), |(_, w)| w);
        attributes.push_str(&attribute(word, &value));
    }
    attributes.push_str(&(marks.item)(&command.span));
    if command.items.is_empty() {
        let _ = writeln!(out, "{pad}<Item{attributes} />");
        return;
    }
    let _ = writeln!(out, "{pad}<Item{attributes}>");
    for item in &command.items {
        command_element(item, depth + 1, out, marks);
    }
    let _ = writeln!(out, "{pad}</Item>");
}

/// The element of a container's page: a tab, step, section or pane; a line
/// of a window's content is `<View name="X">` in the source.
pub fn page_element_name(kind: WidgetKind) -> &'static str {
    match kind {
        WidgetKind::Steps => "Step",
        WidgetKind::Accordion => "Section",
        WidgetKind::Splitter => "Pane",
        // A line of a window's content is the view it shows, by name; a
        // program's markup holds the view's parts inside it.
        WidgetKind::Content => "View",
        _ => "Tab",
    }
}

/// A tab, step, section or pane, with the parts on it; in the source, the
/// view it shows and what it does with the view's tells.
fn page_element(kind: WidgetKind, page: &WidgetPage, depth: usize, out: &mut String, marks: &Marks) {
    let pad = "  ".repeat(depth);
    let element = page_element_name(kind);
    let mut attributes = String::new();
    if kind == WidgetKind::Content {
        if let Some(view) = &page.view {
            attributes.push_str(&attribute("name", &Value::String(view.clone())));
        }
    }
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
    if marks.source {
        if kind != WidgetKind::Content {
            if let Some(view) = &page.view {
                attributes.push_str(&attribute("view", &Value::String(view.clone())));
            }
        }
        for tell in &page.tells {
            attributes.push_str(&attribute(&format!("on{}", tell.tell), &Value::String(tell.handler.clone())));
        }
    }
    attributes.push_str(&(marks.item)(&page.span));
    if page.children.is_empty() {
        let _ = writeln!(out, "{pad}<{element}{attributes} />");
        return;
    }
    let _ = writeln!(out, "{pad}<{element}{attributes}>");
    for child in &page.children {
        part(child, depth + 1, out, marks);
    }
    let _ = writeln!(out, "{pad}</{element}>");
}

/// The attributes of a part: its name and place first, then what differs
/// from its kind's defaults, then its handlers.
fn attributes(widget: &Widget, marks: &Marks) -> String {
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
    out.push_str(&(marks.item)(&widget.span));
    out.push_str(&(marks.shown)(&widget.span));
    out
}

/// The fields of a serialized object in the order they were written, which
/// is the order of the props struct and of a part's line in Clarity.
pub fn in_order(json: &str) -> Vec<(String, Value)> {
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
pub fn attribute(word: &str, value: &Value) -> String {
    let text = match value {
        Value::String(s) => s.clone(),
        Value::Array(items) if !items.is_empty() && items.iter().all(|i| i.as_str().is_some_and(|s| !s.contains(','))) => {
            items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")
        }
        other => other.to_string(),
    };
    format!(" {word}=\"{}\"", escaped(&text))
}

// ---------------------------------------------------------------------------
// Reading

/// The page in a `.cxaml` file, or what is wrong with it, where; `path` is
/// the file's, for the diagnostics. Each part's span is its element's place
/// in the file.
pub fn read_page(text: &str, path: &str) -> (Option<Page>, Vec<Diagnostic>) {
    let mut reading = Reading { diagnostics: Vec::new() };
    let page = match xml::read(text) {
        Ok(root) => reading.page(&root),
        Err(d) => {
            reading.diagnostics.push(d);
            None
        }
    };
    for d in &mut reading.diagnostics {
        d.path = path.to_string();
    }
    (page, reading.diagnostics)
}

/// The app in its `.cxaml` file (`<App class="shop.Shop">`), or what is
/// wrong with it, where; `path` is the file's, for the diagnostics.
pub fn read_app(text: &str, path: &str) -> (Option<App>, Vec<Diagnostic>) {
    let mut reading = Reading { diagnostics: Vec::new() };
    let app = match xml::read(text) {
        Ok(root) => reading.app(&root),
        Err(d) => {
            reading.diagnostics.push(d);
            None
        }
    };
    for d in &mut reading.diagnostics {
        d.path = path.to_string();
    }
    (app, reading.diagnostics)
}

/// Whether a `.cxaml` file is the app's: its root is `<App>`.
pub fn is_app_markup(markup: &str) -> bool {
    xml::read(markup).is_ok_and(|root| root.name == APP_ROOT)
}

struct Reading {
    diagnostics: Vec<Diagnostic>,
}

impl Reading {
    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }

    fn page(&mut self, root: &Element) -> Option<Page> {
        let Some(kind) = kind_of_root(&root.name) else {
            self.error(format!("a page's file starts with <Window>, <Dialog>, <View>, <Splash> or <Toolbox>, not <{}>", root.name), root.span);
            return None;
        };
        let Some(class) = root.attribute("class") else {
            self.error(format!("<{}> names its module and page, as in class=\"desk.Desk\"", root.name), root.span);
            return None;
        };
        let Some((_, page_name)) = class.value.rsplit_once('.').filter(|(m, n)| widgets::is_identifier(m) && widgets::is_identifier(n)) else {
            self.error(format!("class=\"{}\" is a module and a page's name, as in class=\"desk.Desk\"", class.value), class.span);
            return None;
        };
        let name = crate::xml::Attribute { name: "name".into(), value: page_name.to_string(), span: class.span };
        let name = &name;
        // What a dialog gives, its handlers, its data and tells: each value read where it is, by Clarity's own rules.
        let gives = root.attribute("gives").and_then(|a| self.read(crate::parser::type_value(&a.value, a.span)));
        let handlers = self.owner_events(root, &widgets::WINDOW_EVENTS, |e| format!("a window has the events open, close and return, not {e}"));
        let data = self.data(root);
        let tells = root.children.iter().filter(|c| c.name == "Tells").filter_map(|c| self.tells(c)).collect();
        let mut page = Page {
            name: Ident { name: name.value.clone(), span: name.span },
            kind,
            title: None,
            size: None,
            style: None,
            data,
            columns: crate::model::DEFAULT_PAGE_COLUMNS,
            cell: None,
            widgets: Vec::new(),
            frame: Frame::default(),
            handlers,
            gives,
            tells,
            span: root.span,
        };
        page.title = root.get("title").map(str::to_string);
        page.size = match (root.get("width"), root.get("height")) {
            (Some(w), Some(h)) => match (w.parse(), h.parse()) {
                (Ok(width), Ok(height)) => Some(WindowSize { width, height }),
                _ => {
                    self.error("a page's width and height are whole numbers", root.span);
                    None
                }
            },
            (None, None) => None,
            _ => {
                self.error("a page's size needs both width and height", root.span);
                None
            }
        };
        page.columns = self.number(root, "columns").unwrap_or(crate::model::DEFAULT_PAGE_COLUMNS);
        page.cell = self.number(root, "cell");
        page.style = root.get("style").map(str::to_string);
        for child in &root.children {
            match child.name.as_str() {
                "Data" | "Tells" => {}
                "Menu" => page.frame.menu = Some(self.menu(child)),
                "Toolbar" => page.frame.toolbar = Some(Toolbar { name: child.get("name").map(str::to_string), items: self.commands(&child.children), span: child.span }),
                "Ribbon" => page.frame.ribbon = Some(self.ribbon(child)),
                "Sidebar" => {
                    if let Some(sidebar) = self.sidebar(child) {
                        page.frame.sidebars.push(sidebar);
                    }
                }
                "Footer" => page.frame.footer = Some(self.footer(child)),
                _ => {
                    if let Some(widget) = self.part(child) {
                        page.widgets.push(widget);
                    }
                }
            }
        }
        Some(page)
    }

    fn app(&mut self, root: &Element) -> Option<App> {
        if root.name != APP_ROOT {
            self.error(format!("the app's file starts with <App>, not <{}>", root.name), root.span);
            return None;
        }
        let Some(class) = root.attribute("class") else {
            self.error("<App> names its module and the app, as in class=\"shop.Shop\"", root.span);
            return None;
        };
        let Some((_, app_name)) = class.value.rsplit_once('.').filter(|(m, n)| widgets::is_identifier(m) && widgets::is_identifier(n)) else {
            self.error(format!("class=\"{}\" is a module and the app's name, as in class=\"shop.Shop\"", class.value), class.span);
            return None;
        };
        const ROOT_WORDS: [&str; 5] = ["class", "title", "main", "splash", "splashFor"];
        for a in root.attributes.iter().filter(|a| !ROOT_WORDS.contains(&a.name.as_str()) && !(a.name.starts_with("on") && a.name.len() > 2)) {
            self.error(format!("<App> has no '{}'", a.name), a.span);
        }
        // The app's handlers and data: each value read where it is, by Clarity's own rules.
        let handlers = self.owner_events(root, &widgets::APP_EVENTS, |e| format!("the app has the event start, not {e}"));
        let data = self.data(root);
        let mut app = App {
            name: Ident { name: app_name.to_string(), span: class.span },
            title: None,
            main: None,
            splash: None,
            splash_for: None,
            data,
            frame: Frame::default(),
            handlers,
            span: root.span,
        };
        app.title = root.get("title").map(str::to_string);
        let ident = |a: &xml::Attribute| Ident { name: a.value.clone(), span: a.span };
        app.main = root.attribute("main").map(ident);
        app.splash = root.attribute("splash").map(ident);
        if let Some(a) = root.attribute("splashFor") {
            match a.value.parse::<f64>() {
                Ok(seconds) if (0.1..=30.0).contains(&seconds) => app.splash_for = Some((seconds * 1000.0).round() as u32),
                _ => self.error(format!("splashFor is how many seconds the splash shows at least, from 0.1 to 30, not '{}'", a.value), a.span),
            }
        }
        for child in &root.children {
            match child.name.as_str() {
                "Data" => {}
                "Menu" => app.frame.menu = Some(self.menu(child)),
                "Toolbar" => app.frame.toolbar = Some(Toolbar { name: child.get("name").map(str::to_string), items: self.commands(&child.children), span: child.span }),
                "Ribbon" => app.frame.ribbon = Some(self.ribbon(child)),
                "Sidebar" => {
                    if let Some(sidebar) = self.sidebar(child) {
                        app.frame.sidebars.push(sidebar);
                    }
                }
                "Footer" => app.frame.footer = Some(self.footer(child)),
                other => self.error(format!("<{other}> is not in an app: it holds <Data>, <Menu>, <Toolbar>, <Ribbon>, <Sidebar> and <Footer>"), child.span),
            }
        }
        Some(app)
    }

    fn number(&mut self, element: &Element, name: &str) -> Option<u32> {
        let a = element.attribute(name)?;
        match a.value.parse() {
            Ok(n) => Some(n),
            Err(_) => {
                self.error(format!("'{name}' is a whole number, not '{}'", a.value), a.span);
                None
            }
        }
    }

    /// What a reading of a value gives; its error is the file's.
    fn read<T>(&mut self, result: Result<T, Diagnostic>) -> Option<T> {
        result.map_err(|d| self.diagnostics.push(d)).ok()
    }

    /// The owner's own events (`onopen="windowOpened"`), each among `allowed`.
    fn owner_events(&mut self, root: &Element, allowed: &[widgets::Event], refused: impl Fn(&str) -> String) -> Vec<crate::model::Handler> {
        let mut out = Vec::new();
        for a in root.attributes.iter().filter(|a| a.name.starts_with("on") && a.name.len() > 2) {
            let word = &a.name[2..];
            let Some(event) = widgets::Event::by_keyword(word).filter(|e| allowed.contains(e)) else {
                self.error(refused(word), a.span);
                continue;
            };
            if !widgets::is_identifier(&a.value) {
                self.error(format!("'{}' is not a handler's name", a.value), a.span);
                continue;
            }
            out.push(crate::model::Handler { event, handler: a.value.clone(), args: Vec::new() });
        }
        out
    }

    /// The owner's data: `<Data name="customer" type="Customer" />`, each where it is.
    fn data(&mut self, root: &Element) -> Vec<DataDecl> {
        let mut out = Vec::new();
        for child in root.children.iter().filter(|c| c.name == "Data") {
            let (Some(name), Some(ty)) = (child.attribute("name"), child.attribute("type")) else {
                self.error("<Data> has a name and a type, as in <Data name=\"customer\" type=\"Customer\" />", child.span);
                continue;
            };
            if !widgets::is_identifier(&name.value) {
                self.error(format!("'{}' is not a data name", name.value), name.span);
                continue;
            }
            let ident = Ident { name: name.value.clone(), span: child.span };
            let default = child.attribute("default").map(|d| (d.value.as_str(), d.span));
            if let Some(decl) = self.read(crate::parser::data_value(ident, (&ty.value, ty.span), default, child.span)) {
                out.push(decl);
            }
        }
        out
    }

    /// What a view tells its window: `<Tells message="saved(name: text)" />`.
    fn tells(&mut self, child: &Element) -> Option<crate::code::PartMessage> {
        let Some(message) = child.attribute("message") else {
            self.error("<Tells> names a message, as in <Tells message=\"saved\" />", child.span);
            return None;
        };
        let mut tell = self.read(crate::parser::tells_value(&message.value, message.span, child.span))?;
        tell.name.span = child.span;
        Some(tell)
    }

    fn menu(&mut self, element: &Element) -> Menu {
        Menu { name: element.get("name").map(str::to_string), items: self.commands(&element.children), span: element.span }
    }

    fn commands(&mut self, elements: &[Element]) -> Vec<Command> {
        elements.iter().filter_map(|e| self.command(e)).collect()
    }

    fn command(&mut self, element: &Element) -> Option<Command> {
        if element.name != "Item" {
            self.error(format!("a menu holds <Item>s, not <{}>", element.name), element.span);
            return None;
        }
        let defaults = crate::frame::new_command(CommandKind::Item, None);
        let props = self.props(element, &defaults, &COMMAND_WORDS, &[])?;
        Some(Command { props, items: self.commands(&element.children), span: element.span })
    }

    fn ribbon(&mut self, element: &Element) -> Ribbon {
        let mut tabs = Vec::new();
        for tab in &element.children {
            let mut groups = Vec::new();
            for group in &tab.children {
                groups.push(RibbonGroup { title: group.get("title").unwrap_or("").to_string(), name: group.get("name").map(str::to_string), items: self.commands(&group.children), span: group.span });
            }
            tabs.push(RibbonTab { title: tab.get("title").unwrap_or("").to_string(), name: tab.get("name").map(str::to_string), groups, span: tab.span });
        }
        Ribbon { name: element.get("name").map(str::to_string), collapsible: element.get("collapsible") == Some("true"), collapsed: element.get("collapsed") == Some("true"), tabs, span: element.span }
    }

    fn sidebar(&mut self, element: &Element) -> Option<Sidebar> {
        let side = match element.get("side") {
            Some("right") => crate::model::Side::Right,
            _ => crate::model::Side::Left,
        };
        let props = self.props(element, &crate::frame::new_sidebar(side, None), &[], &[])?;
        let widgets = element.children.iter().filter_map(|c| self.part(c)).collect();
        Some(Sidebar { props, widgets, span: element.span })
    }

    fn footer(&mut self, element: &Element) -> Footer {
        let widgets = element.children.iter().filter_map(|c| self.part(c)).collect();
        let status = element.attribute("status");
        Footer { status: status.map(|s| s.value.clone()), status_span: status.map(|s| s.span), columns: self.number(element, "columns").unwrap_or(12), widgets, span: element.span }
    }

    /// `defaults` with each attribute of `element` read into its field, by
    /// the field's type: as text, else as JSON, else as a list of names;
    /// `none` lists fields that have no value. `skip` names attributes read elsewhere.
    fn props<T: serde::Serialize + DeserializeOwned>(&mut self, element: &Element, defaults: &T, words: &[(&str, &str)], skip: &[&str]) -> Option<T> {
        let Ok(Value::Object(mut fields)) = serde_json::to_value(defaults) else { return None };
        for a in &element.attributes {
            if skip.contains(&a.name.as_str()) || a.name.starts_with("design") {
                continue;
            }
            if a.name == "none" {
                for word in a.value.split_whitespace() {
                    let field = words.iter().find(|(_, w)| *w == word).map_or(word, |(f, _)| f);
                    fields.insert(field.to_string(), Value::Null);
                }
                continue;
            }
            let field = words.iter().find(|(_, w)| *w == a.name).map_or(a.name.as_str(), |(f, _)| f).to_string();
            if !fields.contains_key(&field) {
                self.error(format!("<{}> has no '{}'", element.name, a.name), a.span);
                continue;
            }
            match accepted::<T>(&fields, &field, &a.value) {
                Some(value) => {
                    fields.insert(field, value);
                }
                None => self.error(format!("'{}' on <{}> cannot be '{}'", a.name, element.name, a.value), a.span),
            }
        }
        serde_json::from_value(Value::Object(fields)).ok()
    }

    fn part(&mut self, element: &Element) -> Option<Widget> {
        let Some(kind) = kind_of_element(&element.name) else {
            self.error(format!("<{}> is not a part Clarity knows", element.name), element.span);
            return None;
        };
        let (col, row) = match element.attribute("at") {
            Some(at) => match at.value.split_once(',').map(|(c, r)| (c.trim().parse::<u32>(), r.trim().parse::<u32>())) {
                Some((Ok(c), Ok(r))) => (c, r),
                _ => {
                    self.error(format!("'at' is a column and a row, as in at=\"1,2\", not '{}'", at.value), at.span);
                    (1, 1)
                }
            },
            None => (1, 1),
        };
        let skip = ["at", "span", "rows"];
        let handlers: Vec<&xml::Attribute> = element.attributes.iter().filter(|a| a.name.starts_with("on") && a.name.len() > 2 && a.name != "onClick").collect();
        let mut skipped: Vec<&str> = skip.to_vec();
        skipped.extend(handlers.iter().map(|a| a.name.as_str()));
        let mut props = self.props(element, &widgets::new_props(kind, col, row), &WORDS, &skipped)?;
        props.col = col;
        props.row = row;
        props.width = self.number(element, "span").unwrap_or(1);
        props.height = self.number(element, "rows").unwrap_or(1);
        for a in handlers {
            match handler_of(&a.name[2..], &a.value, a.span) {
                Ok(h) => props.handlers.push(h),
                Err(message) => self.error(message, a.span),
            }
        }
        let mut widget = Widget { props, children: Vec::new(), pages: Vec::new(), menu: None, levels: Vec::new(), selects: Vec::new(), span: element.span };
        for child in &element.children {
            match child.name.as_str() {
                "Menu" => widget.menu = Some(self.menu(child)),
                "Tab" | "Step" | "Section" | "Pane" | "View" => {
                    let page = self.widget_page(kind, child);
                    widget.pages.push(page);
                }
                "Level" => widget.levels.push(self.level(child)),
                "Select" => widget.selects.push(TreeSelect { entity: child.get("entity").unwrap_or("").to_string(), handler: child.get("handler").unwrap_or("").to_string(), span: child.span }),
                _ => {
                    if let Some(part) = self.part(child) {
                        widget.children.push(part);
                    }
                }
            }
        }
        Some(widget)
    }

    fn widget_page(&mut self, kind: WidgetKind, element: &Element) -> WidgetPage {
        let content = kind == WidgetKind::Content;
        let columns_word = if kind == WidgetKind::Splitter { "span" } else { "columns" };
        let reserved = ["name", "title", "color", "span", "columns", "view"];
        let tells = element
            .attributes
            .iter()
            .filter(|a| a.name.starts_with("on") && a.name.len() > 2 && !reserved.contains(&a.name.as_str()))
            .map(|a| crate::model::TellHandler { tell: a.name[2..].to_string(), handler: a.value.clone() })
            .collect();
        let view = if content { element.get("name") } else { element.get("view") };
        WidgetPage {
            title: element.get("title").map(str::to_string),
            columns: self.number(element, columns_word),
            view: view.map(str::to_string),
            tells,
            color: element.get("color").map(str::to_string),
            children: element.children.iter().filter_map(|c| self.part(c)).collect(),
            span: element.span,
        }
    }

    fn level(&mut self, element: &Element) -> TreeLevel {
        let text = |word: &str| element.get(word).map(str::to_string);
        TreeLevel {
            entity: text("entity"),
            under: text("under"),
            show: text("show"),
            group: text("group"),
            order: text("order"),
            descending: element.get("descending") == Some("true"),
            levels: element.children.iter().map(|c| self.level(c)).collect(),
            span: element.span,
        }
    }
}

/// The module of the lines the reader has Clarity's parser read.

/// The value `text` gives `field` of a `T`: as text, else as JSON, else as
/// a list of names (`title, kind`); the first a `T` takes.
fn accepted<T: DeserializeOwned>(fields: &Map<String, Value>, field: &str, text: &str) -> Option<Value> {
    let mut candidates = vec![Value::String(text.to_string())];
    if let Ok(json) = serde_json::from_str::<Value>(text) {
        candidates.push(json);
    }
    candidates.push(Value::Array(text.split(',').map(|s| Value::String(s.trim().to_string())).filter(|v| v.as_str() != Some("")).collect()));
    candidates.into_iter().find(|value| {
        let mut tried = fields.clone();
        tried.insert(field.to_string(), value.clone());
        serde_json::from_value::<T>(Value::Object(tried)).is_ok()
    })
}

/// `digit(7)` on `click`: the handler and the values it is run with.
fn handler_of(event: &str, call: &str, at: Span) -> Result<crate::model::Handler, String> {
    let event_kind = widgets::Event::by_keyword(event).ok_or_else(|| format!("'{event}' is not an event a part has"))?;
    let (name, args) = match call.find('(') {
        Some(open) => {
            let close = call.rfind(')').filter(|c| *c > open).ok_or_else(|| format!("'{call}' opens ( without closing it"))?;
            (call[..open].trim(), &call[open + 1..close])
        }
        None => (call.trim(), ""),
    };
    if !widgets::is_identifier(name) {
        return Err(format!("'{name}' is not a handler's name"));
    }
    // The values as Clarity reads them, where they are in the markup.
    let offset = call.find('(').map_or(0, |open| open as u32 + 1);
    let args = crate::parser::literals_value(args, Span { start: at.start + offset, col: at.col + offset, ..at }).map_err(|d| d.message)?;
    Ok(crate::model::Handler { event: event_kind, handler: name.to_string(), args })
}

/// A whole file's name for the page `name`: `Board.cxaml`.
pub fn file_name(name: &str) -> String {
    format!("{name}.{EXTENSION}")
}

/// The extension of a window's or view's file.
pub const EXTENSION: &str = "cxaml";

// ---------------------------------------------------------------------------
// Files

/// Whether `path` is a window's or view's file.
pub fn is_page_file(path: &str) -> bool {
    std::path::Path::new(path).extension().and_then(|e| e.to_str()) == Some(EXTENSION)
}

/// The `.cxaml` files of the project in `dir`: those in its `design`
/// folder, sorted, with the ones in `replaced` not written yet.
pub fn page_files(dir: &std::path::Path, _modules: &[std::path::PathBuf], replaced: &[(&std::path::Path, &str)]) -> std::io::Result<Vec<std::path::PathBuf>> {
    let folder = design_folder(dir);
    let mut out = Vec::new();
    if folder.is_dir() {
        for entry in std::fs::read_dir(&folder)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) == Some(EXTENSION) {
                out.push(path);
            }
        }
    }
    for (path, _) in replaced {
        if path.parent() == Some(folder.as_path()) && is_page_file(&path.to_string_lossy()) && !out.iter().any(|p| p == path) {
            out.push(path.to_path_buf());
        }
    }
    // A file replaced by nothing is left out: a window, view or app being removed.
    out.retain(|p| !replaced.iter().any(|(path, text)| path == p && text.is_empty()));
    out.sort();
    Ok(out)
}

/// The module and page a `.cxaml` names on its root (`class="desk.Desk"`).
pub fn class_of(markup: &str) -> Option<(String, String)> {
    let root = xml::read(markup).ok()?;
    let (module, name) = root.get("class")?.rsplit_once('.')?;
    Some((module.to_string(), name.to_string()))
}

/// The module a file of Clarity declares on its first line of code.
fn module_name(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with("//"))?.strip_prefix("module ")?.split_whitespace().next()
}

/// The file of the model a `.cxaml` file is: its page, in the module its
/// root names (`class="desk.Desk"`), using what that module uses.
pub fn page_file(path: &str, text: &str, modules: &[crate::model::SpecFile]) -> (Option<crate::model::SpecFile>, Vec<Diagnostic>) {
    let module = class_of(text).map(|(m, _)| m).unwrap_or_default();
    // The app's file holds the app; any other, a page.
    let (page, app, mut diagnostics) = if is_app_markup(text) {
        let (app, d) = read_app(text, path);
        (None, app, d)
    } else {
        let (page, d) = read_page(text, path);
        (page, None, d)
    };
    let (name, span) = match (&page, &app) {
        (Some(p), _) => (p.name.name.clone(), p.span),
        (None, Some(a)) => (a.name.name.clone(), a.span),
        (None, None) => return (None, diagnostics),
    };
    let Some(owner) = modules.iter().find(|f| f.module.as_ref().is_some_and(|m| m.name == module)) else {
        let mut d = Diagnostic::error(format!("class=\"{module}.{name}\" names the module '{module}', which the project has none of"), span);
        d.path = path.to_string();
        diagnostics.push(d);
        return (None, diagnostics);
    };
    let (mut file, _) = crate::parser::parse(&format!("module {module}\n"), path);
    file.version = crate::content_version(text);
    file.uses = owner.uses.clone();
    file.source = owner.source.clone();
    file.pages.extend(page);
    file.apps.extend(app);
    (Some(file), diagnostics)
}

/// The files a project's `.clarity` files become with their pages and app
/// moved into `.cxaml` (docs/CXAML-SOURCE-PLAN.md): each page block and the
/// app block, with the comment above it, leaves its file for
/// `design/<Page>.cxaml` or `design/<App>.cxaml`.
/// Gives the changed `.clarity` files and the new `.cxaml` files, or what
/// stops a file moving (a file that does not parse).
pub fn moved_out(files: &[(String, String)]) -> Result<Vec<(String, String)>, String> {
    moved_out_only(files, None)
}

/// `moved_out` for the pages and app named in `only` (all when none): a
/// window the designer adds goes into its own file, the rest of its module
/// as it was.
pub fn moved_out_only(files: &[(String, String)], only: Option<&[String]>) -> Result<Vec<(String, String)>, String> {
    moved(files, only, true)
}

/// `moved_out` of what the parser read, also from a module it found errors
/// in: for tests that start from text written the old way, wrong on purpose
/// (`clarity_spec::converted`). A person's files are moved only when they
/// read clean (`moved_out`).
pub fn moved_out_as_read(files: &[(String, String)]) -> Result<Vec<(String, String)>, String> {
    moved(files, None, false)
}

/// Whether `line` is in the block of Clarity that starts at line `start`
/// (both from 1): up to the next line written at the left edge.
fn in_block(text: &str, start: u32, line: u32) -> bool {
    if line < start {
        return false;
    }
    let lines: Vec<&str> = text.lines().collect();
    let end = (start as usize..lines.len()).find(|&i| {
        let l = lines[i];
        !l.trim().is_empty() && !l.starts_with(char::is_whitespace) && !l.trim_start().starts_with("//")
    });
    end.is_none_or(|end| (line as usize) <= end)
}

fn moved(files: &[(String, String)], only: Option<&[String]>, clean: bool) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for (path, text) in files {
        let (file, diagnostics) = crate::parser::parse(text, path);
        let error = diagnostics.iter().find(|d| d.severity == crate::diagnostic::Severity::Error);
        if clean && let Some(d) = error {
            return Err(format!("{path}:{}: {}", d.span.line, d.message));
        }
        // What the parser read of a page with an error in it is not the page: it stays as written.
        let starts: Vec<u32> = file.pages.iter().map(|p| p.span.line).chain(file.apps.iter().map(|a| a.span.line)).collect();
        if diagnostics.iter().any(|d| d.severity == crate::diagnostic::Severity::Error && starts.iter().any(|&start| in_block(text, start, d.span.line))) {
            continue;
        }
        let pages: Vec<&Page> = file.pages.iter().filter(|p| only.is_none_or(|names| names.contains(&p.name.name))).collect();
        let apps: Vec<&App> = file.apps.iter().filter(|a| only.is_none_or(|names| names.contains(&a.name.name))).collect();
        if pages.is_empty() && apps.is_empty() {
            continue;
        }
        let module = file.module.as_ref().map(|m| m.name.clone()).ok_or_else(|| format!("{path} declares no module"))?;
        // Each block that moves: the line it starts on, its name and its markup.
        let moving: Vec<(u32, &str, String)> = pages
            .iter()
            .map(|p| (p.span.line, p.name.name.as_str(), page_source(p, &module)))
            .chain(apps.iter().map(|a| (a.span.line, a.name.name.as_str(), app_source(a, &module))))
            .collect();
        if let Some(app) = apps.iter().find(|a| file.pages.iter().any(|p| p.name.name == a.name.name)) {
            return Err(format!("{path}: the app and a window are both called {}, and each needs design/{}; rename one", app.name.name, file_name(&app.name.name)));
        }
        let folder = design_folder(std::path::Path::new(path).parent().unwrap_or(std::path::Path::new("")));
        let lines: Vec<&str> = text.split('\n').collect();
        let mut gone = vec![false; lines.len()];
        for (line, name, mut markup) in moving {
            let first = line as usize - 1;
            // The block: the page's line and every line after it until the next line at the left edge.
            let mut last = first;
            for (i, line) in lines.iter().enumerate().skip(first + 1) {
                if !line.trim().is_empty() && !line.starts_with(char::is_whitespace) {
                    break;
                }
                if !line.trim().is_empty() {
                    last = i;
                }
            }
            // The comment right above it says what the page is: it goes along.
            let mut top = first;
            while top > 0 && lines[top - 1].trim_start().starts_with("//") && !lines[top - 1].starts_with(char::is_whitespace) {
                top -= 1;
            }
            let comment: Vec<&str> = lines[top..first].iter().map(|l| l.trim_start().trim_start_matches("//").trim()).collect();
            for flag in &mut gone[top..=last] {
                *flag = true;
            }
            if !comment.is_empty() {
                let note = format!("<!-- {} -->\n", comment.join("\n     ").replace("--", "- -"));
                let at = markup.find('\n').map_or(0, |n| n + 1);
                markup.insert_str(at, &note);
            }
            out.push((folder.join(file_name(name)).display().to_string(), markup));
        }
        // What is left, with no more than one blank line in a row.
        let mut kept = String::new();
        let mut blank = 0;
        for (line, gone) in lines.iter().zip(&gone) {
            if *gone {
                continue;
            }
            if line.trim().is_empty() {
                blank += 1;
                if blank > 1 {
                    continue;
                }
            } else {
                blank = 0;
            }
            kept.push_str(line);
            kept.push('\n');
        }
        let kept = kept.trim_end().to_string() + "\n";
        out.push((path.clone(), kept));
    }
    Ok(out)
}

/// The project with each window's, view's and the app's own file (`.cxaml`) folded
/// into its module's file of Clarity: one file per module, as the
/// generators read a module, which need no places in files.
pub fn folded(project: &crate::Project) -> crate::Project {
    let mut out = project.clone();
    let (pages, mut files): (Vec<crate::model::SpecFile>, Vec<crate::model::SpecFile>) = std::mem::take(&mut out.files).into_iter().partition(|f| is_page_file(&f.path));
    for page_file in pages {
        let module = page_file.module.as_ref().map(|m| m.name.clone());
        match files.iter_mut().find(|f| f.module.as_ref().map(|m| m.name.clone()) == module) {
            Some(owner) => {
                owner.pages.extend(page_file.pages);
                owner.apps.extend(page_file.apps);
            }
            None => files.push(page_file),
        }
    }
    out.files = files;
    out
}

// ---------------------------------------------------------------------------
// The designer's edits

/// The comments before a markup's root element, kept when the page is written again.
fn leading_comments(markup: &str) -> String {
    let start = markup.find("?>").map_or(0, |n| n + 2);
    let root = markup[start..].find(|c: char| c == '<').map(|n| start + n);
    let mut out = String::new();
    let mut rest = &markup[start..];
    while let Some(open) = rest.find("<!--") {
        if root.is_some_and(|r| markup.len() - rest.len() + open > r) && !rest[..open].trim().is_empty() {
            break;
        }
        let Some(close) = rest[open..].find("-->") else { break };
        out.push_str(&rest[open..open + close + 3]);
        out.push('\n');
        rest = &rest[open + close + 3..];
        if !rest.trim_start().starts_with("<!--") {
            break;
        }
    }
    out
}

/// An edit of the designer on a window's, view's or app's `.cxaml`: the
/// file is read, the edit made on what it holds (`design_edit`) and the file
/// written again, keeping the comments before its root; `project`, when
/// given, lets edits build on the rest (a field's type) and makes the texts
/// they write keys. The focus is where the thing the edit produced or
/// changed starts in the file as written.
/// What a design file holds, read from its markup: a window's or view's
/// page, or the app.
/// A file read with an error is refused: what the reader passed over would
/// be lost if the file were written again.
pub fn read_designed(markup: &str, path: &str) -> Result<crate::design_edit::Designed, String> {
    use crate::design_edit::Designed;
    let (designed, diagnostics) = if is_app_markup(markup) {
        let (app, diagnostics) = read_app(markup, path);
        (app.map(Designed::App), diagnostics)
    } else {
        let (page, diagnostics) = read_page(markup, path);
        (page.map(Designed::Page), diagnostics)
    };
    match (designed, diagnostics.iter().find(|d| d.severity == crate::diagnostic::Severity::Error)) {
        (Some(designed), None) => Ok(designed),
        (_, Some(d)) => Err(format!("{path}:{}: {}; fix the file first", d.span.line, d.message)),
        (None, None) => Err(format!("{path} cannot be read")),
    }
}

/// The design file `markup` written again with what it holds now, keeping
/// the comments before its root.
pub fn written_again(markup: &str, designed: &crate::design_edit::Designed) -> String {
    let module = class_of(markup).map(|(m, _)| m).unwrap_or_default();
    let source = match designed {
        crate::design_edit::Designed::Page(page) => page_source(page, &module),
        crate::design_edit::Designed::App(app) => app_source(app, &module),
    };
    with_comments(markup, source)
}

pub fn apply(project: Option<&crate::Project>, markup: &str, edit: &crate::edit::Edit) -> crate::edit::EditResult<crate::edit::EditOutcome> {
    use crate::design_edit::Designed;
    use crate::edit::EditError;
    let read = |markup: &str| -> crate::edit::EditResult<Designed> { read_designed(markup, "").map_err(EditError) };
    let keys_of = |designed: &Designed| match designed {
        Designed::Page(page) => crate::design_keys::page_keys("", page),
        Designed::App(app) => crate::design_keys::app_keys("", app),
    };
    let mut designed = read(markup)?;
    let focus = crate::design_edit::apply(&mut designed, edit, project)?;
    // What the edit produced, by its key, to find again in the file as written.
    let focus_key = focus.and_then(|at| keys_of(&designed).into_iter().find(|(start, _)| *start == at).map(|(_, key)| key));
    let mut written = written_again(markup, &designed);
    // A text people read that the edit wrote (a caption typed for a new part) becomes a key, beside the project's.
    let mut keys = Vec::new();
    if let Some(project) = project
        && written != markup
    {
        let default = crate::translations::default_language(project);
        let existing = project.translations.get(&default).cloned().unwrap_or_default();
        let mut keyed = read(&written)?;
        let made = crate::keyed::key_designed(&mut keyed, &default, &existing);
        if !made.is_empty() {
            written = written_again(&written, &keyed);
            keys = made;
        }
    }
    let focus = match focus_key {
        Some(key) => keys_of(&read(&written)?).into_iter().find(|(_, k)| *k == key).map(|(start, _)| start),
        None => None,
    };
    Ok(crate::edit::EditOutcome { text: written, focus, keys })
}

/// The module's file of Clarity that a window's or view's `.cxaml` belongs
/// to: in its project's folder, the file that declares the module its root
/// names. Its code blocks hold the page's handlers.
pub fn module_file(page_file: &std::path::Path) -> Option<std::path::PathBuf> {
    let module = class_of(&std::fs::read_to_string(page_file).ok()?)?.0;
    let dir = project_dir_of(page_file)?;
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().and_then(|e| e.to_str()) == Some(crate::SPEC_EXTENSION)).collect();
    files.sort();
    files.into_iter().find(|p| std::fs::read_to_string(p).ok().as_deref().and_then(module_name) == Some(module.as_str()))
}

/// `source` with the comments `markup` had before its root element.
fn with_comments(markup: &str, source: String) -> String {
    let comments = leading_comments(markup);
    match source.find('\n') {
        Some(n) if !comments.is_empty() => format!("{}{}{}", &source[..=n], comments, &source[n + 1..]),
        _ => source,
    }
}

/// The folder of a project's windows and views, beside its modules, as an
/// Avalonia project keeps its views: each file names its module on its root.
pub const DESIGN_DIR: &str = "design";

/// The folder of the windows and views of the project in `dir`.
pub fn design_folder(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join(DESIGN_DIR)
}

/// The project folder a window's or view's file is in: above `design/`.
pub fn project_dir_of(page_file: &std::path::Path) -> Option<&std::path::Path> {
    let design = page_file.parent()?;
    (design.file_name()? == DESIGN_DIR).then(|| design.parent()).flatten()
}

/// What the Problems list says of a page still written in a module's file:
/// the start of each such message, which its fix (Move pages into design)
/// is found by.
pub const PAGE_LEFT_IN_MODULE: &str = "is written in this module's file; a window or view has a file of its own";

/// What the Problems list says of the app still written in a module's
/// file, which the same fix (Move pages into design) moves.
pub const APP_LEFT_IN_MODULE: &str = "is written in this module's file; the app has a file of its own";

/// A page or app block left in a module's file of Clarity, each an error: a
/// window or view has its own `.cxaml` in `design/`, where Move pages into
/// design, in Problems (or `clarity convert-pages`), puts it.
pub fn pages_left_in_modules(project: &crate::Project) -> Vec<Diagnostic> {
    project
        .files
        .iter()
        .filter(|f| !is_page_file(&f.path))
        .flat_map(|f| {
            let pages = f.pages.iter().map(move |p| {
                Diagnostic::error(format!("{} {PAGE_LEFT_IN_MODULE}, design/{}: Move pages into design, in Problems, moves it", p.name.name, file_name(&p.name.name)), p.name.span).in_file(&f.path)
            });
            let apps = f.apps.iter().map(move |a| {
                Diagnostic::error(format!("{} {APP_LEFT_IN_MODULE}, design/{}: Move pages into design, in Problems, moves it", a.name.name, file_name(&a.name.name)), a.name.span).in_file(&f.path)
            });
            pages.chain(apps).collect::<Vec<_>>()
        })
        .collect()
}

/// Writes a module's text at `path` as a project holds it: its pages each in
/// their own file in `design/` beside it, the rest in the module's file.
pub fn write_with_pages_moved_out(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let files = moved_out(&[(path.display().to_string(), text.to_string())]).map_err(std::io::Error::other)?;
    if files.is_empty() {
        return std::fs::write(path, text);
    }
    for (file, written) in files {
        let file = std::path::Path::new(&file);
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(file, written)?;
    }
    Ok(())
}

/// Make texts keys over a project whose windows, views and app are in
/// `.cxaml` (`pages`, each a path and its markup) beside its modules: each
/// page's and app's texts are made keys on its model, after the modules',
/// and its markup is written again. The result lists the modules and then
/// the pages, each by its own path; a page that cannot be read is left out.
pub fn make_keys_with_pages(modules: &[(&str, &str)], pages: &[(&str, &str)], translations: &[(&str, &str)], default: &str) -> crate::keyed::Keyed {
    let read: Vec<(&str, &str, crate::design_edit::Designed)> = pages.iter().filter_map(|(path, markup)| read_designed(markup, path).ok().map(|d| (*path, *markup, d))).collect();
    let (places, mut designed): (Vec<(&str, &str)>, Vec<crate::design_edit::Designed>) = read.into_iter().map(|(p, m, d)| ((p, m), d)).unzip();
    let mut keyed = crate::keyed::make_keys_designed(modules, &mut designed, translations, default, &std::collections::BTreeMap::new());
    for ((path, markup), designed) in places.iter().zip(&designed) {
        keyed.modules.push((path.to_string(), written_again(markup, designed)));
    }
    keyed
}


// ---------------------------------------------------------------------------
// New windows, views and the app

/// What an edit that adds a window, view, splash, toolbox or the app makes:
/// the module's text with the code they start with, each new file in
/// `design/` with its markup, and the keys their texts were made.
#[derive(Debug, Clone, PartialEq)]
pub struct Added {
    pub module: String,
    pub files: Vec<(std::path::PathBuf, String)>,
    pub keys: Vec<(String, String)>,
}

/// The files `edit` adds beside the module at `module_path` (its text
/// `module_text`), when it is an edit that adds a window (`AddWindow`), a
/// page (`AddPage`) or the app (`AddApp`); none for another edit.
pub fn added(project: &crate::Project, module_path: &std::path::Path, module_text: &str, edit: &crate::edit::Edit) -> Option<Result<Added, String>> {
    use crate::edit::Edit;
    let made = match edit {
        Edit::AddWindow { name, template } => crate::templates::new_window(project, name, template),
        Edit::AddPage { name, kind } => crate::templates::new_page(project, name, *kind),
        Edit::AddApp { name } => crate::templates::new_app(project, name),
        _ => return None,
    };
    Some(made.and_then(|made| {
        let module = module_name(module_text).ok_or_else(|| format!("{} names no module", module_path.display()))?;
        let folder = design_folder(module_path.parent().unwrap_or(std::path::Path::new("")));
        // Texts people read become keys, beside the project's and each other's.
        let default = crate::translations::default_language(project);
        let mut existing = project.translations.get(&default).cloned().unwrap_or_default();
        let mut keys = Vec::new();
        let mut files = Vec::new();
        for mut designed in made.designed {
            let made_keys = crate::keyed::key_designed(&mut designed, &default, &existing);
            existing.extend(made_keys.iter().cloned());
            keys.extend(made_keys);
            let (name, markup) = match &designed {
                crate::design_edit::Designed::Page(page) => (page.name.name.clone(), page_source(page, module)),
                crate::design_edit::Designed::App(app) => (app.name.name.clone(), app_source(app, module)),
            };
            let path = folder.join(file_name(&name));
            if path.exists() {
                return Err(format!("{} is there already", path.display()));
            }
            files.push((path, markup));
        }
        let mut text = module_text.to_string();
        for code in &made.code {
            text = crate::source::append_decl(&text, code).map_err(|e| e.0)?;
        }
        Ok(Added { module: text, files, keys })
    }))
}

/// What removing the window, view or app of the design file `markup` does
/// to its module's text (`module_text`): its code block goes with it. The
/// design file itself is deleted. None when `item` is not where the file's
/// window, view or app starts.
pub fn removed(markup: &str, item: u32, module_text: &str) -> Option<Result<String, String>> {
    let designed = read_designed(markup, "").ok()?;
    if designed.start() != item {
        return None;
    }
    let owner = class_of(markup).map(|(_, name)| name).unwrap_or_default();
    Some(crate::code::edit::set_code(module_text, &owner, "").map_err(|e| e.0))
}
