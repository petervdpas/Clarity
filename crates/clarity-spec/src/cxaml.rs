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
    Command, CommandKind, DataKind, Footer, Frame, Menu, Page, PageKind, Ribbon, RibbonGroup, RibbonTab, Sidebar, Toolbar, TreeLevel, TreeSelect, Widget, WidgetPage, WindowSize,
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
    for data in page.data.iter().filter(|d| !d.made) {
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
        let _ = writeln!(body, "  <Data{attributes} />");
    }
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
fn page_element_name(kind: WidgetKind) -> &'static str {
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
    let mut reading = Reading { path, diagnostics: Vec::new() };
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

struct Reading<'a> {
    path: &'a str,
    diagnostics: Vec<Diagnostic>,
}

/// A line of the page written in Clarity, read by Clarity's own parser, and
/// the element it came from, whose span its diagnostics and spans take.
struct Line {
    text: String,
    span: Span,
}

impl Reading<'_> {
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
        // What Clarity's parser reads: the data, the tells, what a dialog gives and the page's handlers.
        let gives = root.get("gives").map(|g| format!(" gives {g}")).unwrap_or_default();
        let mut lines = vec![Line { text: format!("page {}: {}{gives}", name.value, kind_word(kind)), span: root.span }];
        for a in root.attributes.iter().filter(|a| a.name.starts_with("on") && a.name.len() > 2) {
            lines.push(Line { text: format!("    on {}: {}", &a.name[2..], a.value), span: a.span });
        }
        for child in &root.children {
            match child.name.as_str() {
                "Data" => {
                    let default = child.get("default").map(|d| format!(", default {d}")).unwrap_or_default();
                    lines.push(Line { text: format!("    data {}: {}{default}", child.get("name").unwrap_or(""), child.get("type").unwrap_or("")), span: child.span });
                }
                "Tells" => lines.push(Line { text: format!("    tells {}", child.get("message").unwrap_or("")), span: child.span }),
                _ => {}
            }
        }
        let parsed = self.parsed(&lines)?;
        let mut page = parsed.pages.into_iter().next()?;
        page.span = root.span;
        page.name.span = name.span;
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

    /// The page of `lines`, read by Clarity's parser; its spans and diagnostics are put on the elements.
    fn parsed(&mut self, lines: &[Line]) -> Option<crate::model::SpecFile> {
        // A module line first, which every file of Clarity starts with; the page's lines from line 2.
        let text: String = std::iter::once(format!("module {SYNTHETIC_MODULE}\n")).chain(lines.iter().map(|l| l.text.clone() + "\n")).collect();
        let (mut file, diagnostics) = crate::parser::parse(&text, self.path);
        let at = |span: Span| lines.get(span.line.saturating_sub(2) as usize).map_or(lines[0].span, |l| l.span);
        for mut d in diagnostics {
            d.span = at(d.span);
            self.diagnostics.push(d);
        }
        for page in &mut file.pages {
            for data in &mut page.data {
                data.span = at(data.span);
                data.name.span = data.span;
            }
            for tell in &mut page.tells {
                tell.span = at(tell.span);
                tell.name.span = tell.span;
            }
        }
        Some(file)
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
            match handler_of(&a.name[2..], &a.value) {
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
const SYNTHETIC_MODULE: &str = "cxaml";

/// The page kind as Clarity writes it: `window`, `view`.
fn kind_word(kind: PageKind) -> &'static str {
    match kind {
        PageKind::Window => "window",
        PageKind::Dialog => "dialog",
        PageKind::View => "view",
        PageKind::Splash => "splash",
        PageKind::Toolbox => "toolbox",
    }
}

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
fn handler_of(event: &str, call: &str) -> Result<crate::model::Handler, String> {
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
    let args = if args.trim().is_empty() {
        Vec::new()
    } else {
        // The values as Clarity reads them, through a line of its own.
        let (file, diagnostics) = crate::parser::parse(&format!("module {SYNTHETIC_MODULE}\npage P: window\n    button at 1,1 span 1 on click: h({args})\n"), "");
        if let Some(d) = diagnostics.first() {
            return Err(d.message.clone());
        }
        file.pages.first().and_then(|p| p.widgets.first()).and_then(|w| w.props.handlers.first()).map(|h| h.args.clone()).unwrap_or_default()
    };
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
    let (page, mut diagnostics) = read_page(text, path);
    let Some(page) = page else { return (None, diagnostics) };
    let Some(owner) = modules.iter().find(|f| f.module.as_ref().is_some_and(|m| m.name == module)) else {
        let mut d = Diagnostic::error(format!("class=\"{module}.{}\" names the module '{module}', which the project has none of", page.name.name), page.span);
        d.path = path.to_string();
        diagnostics.push(d);
        return (None, diagnostics);
    };
    let (mut file, _) = crate::parser::parse(&format!("module {module}\n"), path);
    file.version = crate::content_version(text);
    file.uses = owner.uses.clone();
    file.source = owner.source.clone();
    file.pages.push(page);
    (Some(file), diagnostics)
}

/// The files a project's `.clarity` files become with their pages moved
/// into `.cxaml` (docs/CXAML-SOURCE-PLAN.md): each page block, with the
/// comment above it, leaves its file for `design/<Page>.cxaml`.
/// Gives the changed `.clarity` files and the new `.cxaml` files, or what
/// stops a file moving (a file that does not parse).
pub fn moved_out(files: &[(String, String)]) -> Result<Vec<(String, String)>, String> {
    moved_out_only(files, None)
}

/// `moved_out` for the pages named in `only` (all when none): a window the
/// designer adds goes into its own file, the rest of its module as it was.
pub fn moved_out_only(files: &[(String, String)], only: Option<&[String]>) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for (path, text) in files {
        let (file, diagnostics) = crate::parser::parse(text, path);
        if let Some(d) = diagnostics.iter().find(|d| d.severity == crate::diagnostic::Severity::Error) {
            return Err(format!("{path}:{}: {}", d.span.line, d.message));
        }
        let moving: Vec<&Page> = file.pages.iter().filter(|p| only.is_none_or(|names| names.contains(&p.name.name))).collect();
        if moving.is_empty() {
            continue;
        }
        let module = file.module.as_ref().map(|m| m.name.clone()).ok_or_else(|| format!("{path} declares no module"))?;
        let folder = design_folder(std::path::Path::new(path).parent().unwrap_or(std::path::Path::new("")));
        let lines: Vec<&str> = text.split('\n').collect();
        let mut gone = vec![false; lines.len()];
        for page in moving {
            let first = page.span.line as usize - 1;
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
            let mut markup = page_source(page, &module);
            if !comment.is_empty() {
                let note = format!("<!-- {} -->\n", comment.join("\n     ").replace("--", "- -"));
                let at = markup.find('\n').map_or(0, |n| n + 1);
                markup.insert_str(at, &note);
            }
            out.push((folder.join(file_name(&page.name.name)).display().to_string(), markup));
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

/// The project with each window's and view's own file (`.cxaml`) folded
/// into its module's file of Clarity: one file per module, as the
/// generators read a module, which need no places in files.
pub fn folded(project: &crate::Project) -> crate::Project {
    let mut out = project.clone();
    let (pages, mut files): (Vec<crate::model::SpecFile>, Vec<crate::model::SpecFile>) = std::mem::take(&mut out.files).into_iter().partition(|f| is_page_file(&f.path));
    for page_file in pages {
        let module = page_file.module.as_ref().map(|m| m.name.clone());
        match files.iter_mut().find(|f| f.module.as_ref().map(|m| m.name.clone()) == module) {
            Some(owner) => owner.pages.extend(page_file.pages),
            None => files.push(page_file),
        }
    }
    out.files = files;
    out
}

// ---------------------------------------------------------------------------
// A page as Clarity writes it

/// A page as the block Clarity writes for it (`page Board: window` and its
/// lines). The designer's edits are made on this text, as they are on a
/// page in a file of Clarity, and the page goes back into its `.cxaml`.
pub fn page_block(page: &Page) -> String {
    use crate::lexer::text_token;
    let mut out = String::new();
    let gives = page.gives.as_ref().map(|g| format!(" gives {}", type_text(g))).unwrap_or_default();
    let _ = writeln!(out, "page {}: {}{gives}", page.name.name, kind_word(page.kind));
    if let Some(title) = &page.title {
        let _ = writeln!(out, "    title {}", text_token(title));
    }
    if let Some(size) = page.size {
        let _ = writeln!(out, "    size {} x {}", size.width, size.height);
    }
    if let Some(style) = &page.style {
        let _ = writeln!(out, "    style {style}");
    }
    if page.columns != crate::model::DEFAULT_PAGE_COLUMNS || page.cell.is_some() {
        let rows = page.cell.map(|c| format!(" rows {c}")).unwrap_or_default();
        let _ = writeln!(out, "    grid {} columns{rows}", page.columns);
    }
    for data in page.data.iter().filter(|d| !d.made) {
        let of = match &data.of {
            DataKind::Record { entity } => entity.name.clone(),
            DataKind::List { entity } => format!("list<{}>", entity.name),
            DataKind::Value { ty, default } => type_text(ty) + &default.as_ref().map(|d| format!(", default {}", widgets::literal_text(d))).unwrap_or_default(),
        };
        let _ = writeln!(out, "    data {}: {of}", data.name.name);
    }
    for tell in &page.tells {
        let params: Vec<String> = tell.params.iter().map(|p| format!("{}: {}", p.name.name, type_text(&p.ty))).collect();
        let params = if params.is_empty() { String::new() } else { format!("({})", params.join(", ")) };
        let _ = writeln!(out, "    tells {}{params}", tell.name.name);
    }
    for h in &page.handlers {
        let _ = writeln!(out, "    on {}: {}", h.event.keyword(), widgets::handler_call(h));
    }
    let frame = &page.frame;
    if let Some(menu) = &frame.menu {
        menu_block(menu, 1, &mut out);
    }
    if let Some(toolbar) = &frame.toolbar {
        let _ = writeln!(out, "    toolbar{}", toolbar.name.as_ref().map(|n| format!(" name {n}")).unwrap_or_default());
        commands_block(&toolbar.items, 2, &mut out);
    }
    if let Some(ribbon) = &frame.ribbon {
        let mut line = "    ribbon".to_string();
        if let Some(name) = &ribbon.name {
            let _ = write!(line, " name {name}");
        }
        if ribbon.collapsible {
            line.push_str(" collapsible");
        }
        if ribbon.collapsed {
            line.push_str(" collapsed");
        }
        let _ = writeln!(out, "{line}");
        for tab in &ribbon.tabs {
            let _ = writeln!(out, "        tab {}{}", text_token(&tab.title), tab.name.as_ref().map(|n| format!(" name {n}")).unwrap_or_default());
            for group in &tab.groups {
                let _ = writeln!(out, "            group {}{}", text_token(&group.title), group.name.as_ref().map(|n| format!(" name {n}")).unwrap_or_default());
                commands_block(&group.items, 4, &mut out);
            }
        }
    }
    for sidebar in &frame.sidebars {
        let _ = writeln!(out, "    {}", crate::frame::sidebar_line(&sidebar.props));
        for widget in &sidebar.widgets {
            widget_block(widget, 2, &mut out);
        }
    }
    if let Some(footer) = &frame.footer {
        let _ = writeln!(out, "    footer");
        if let Some(status) = &footer.status {
            let _ = writeln!(out, "        status {}", text_token(status));
        }
        for widget in &footer.widgets {
            widget_block(widget, 2, &mut out);
        }
    }
    for widget in &page.widgets {
        widget_block(widget, 1, &mut out);
    }
    out
}

fn menu_block(menu: &Menu, depth: usize, out: &mut String) {
    let _ = writeln!(out, "{}menu{}", "    ".repeat(depth), menu.name.as_ref().map(|n| format!(" name {n}")).unwrap_or_default());
    commands_block(&menu.items, depth + 1, out);
}

fn commands_block(items: &[Command], depth: usize, out: &mut String) {
    for item in items {
        let _ = writeln!(out, "{}{}", "    ".repeat(depth), crate::frame::command_line(&item.props));
        commands_block(&item.items, depth + 1, out);
    }
}

fn widget_block(widget: &Widget, depth: usize, out: &mut String) {
    use crate::lexer::text_token;
    let pad = "    ".repeat(depth);
    let _ = writeln!(out, "{pad}{}", widgets::widget_line(&widget.props));
    if let Some(menu) = &widget.menu {
        menu_block(menu, depth + 1, out);
    }
    for level in &widget.levels {
        level_block(level, depth + 1, out);
    }
    for select in &widget.selects {
        let _ = writeln!(out, "{pad}    on select {}: {}", select.entity, select.handler);
    }
    for child in &widget.children {
        widget_block(child, depth + 1, out);
    }
    let keyword = match widget.props.kind {
        WidgetKind::Steps => "step",
        WidgetKind::Accordion => "section",
        WidgetKind::Splitter => "pane",
        WidgetKind::Content => "view",
        _ => "tab",
    };
    for page in &widget.pages {
        let mut line = format!("{pad}    {keyword}");
        if let Some(title) = &page.title {
            let _ = write!(line, " {}", text_token(title));
        }
        if let Some(color) = &page.color {
            let shown = if color.starts_with('#') { crate::lexer::quote(color) } else { color.clone() };
            let _ = write!(line, " color {shown}");
        }
        if let Some(view) = &page.view {
            let _ = if keyword == "view" { write!(line, " {view}") } else { write!(line, " view {view}") };
        }
        for tell in &page.tells {
            let _ = write!(line, " on {}: {}", tell.tell, tell.handler);
        }
        if keyword == "pane" {
            if let Some(columns) = page.columns {
                let _ = write!(line, " span {columns}");
            }
        }
        let _ = writeln!(out, "{line}");
        for child in &page.children {
            widget_block(child, depth + 2, out);
        }
    }
}

fn level_block(level: &TreeLevel, depth: usize, out: &mut String) {
    use crate::lexer::text_token;
    let pad = "    ".repeat(depth);
    let mut line = match (&level.group, &level.entity) {
        (Some(group), _) => format!("{pad}group {}", text_token(group)),
        (None, entity) => format!("{pad}level {}", entity.as_deref().unwrap_or("")),
    };
    if let Some(under) = &level.under {
        let _ = write!(line, " under {under}");
    }
    if let Some(show) = &level.show {
        let shown = if widgets::is_identifier(show) { show.clone() } else { text_token(show) };
        let _ = write!(line, " show {shown}");
    }
    if let Some(order) = &level.order {
        let _ = write!(line, " order by {order}{}", if level.descending { " desc" } else { "" });
    }
    let _ = writeln!(out, "{line}");
    for inner in &level.levels {
        level_block(inner, depth + 1, out);
    }
}

// ---------------------------------------------------------------------------
// The designer's edits

/// Where each thing of a page is, in one order for every page: the page,
/// its data and tells, its frame, then each part with what it holds. Two
/// pages of the same shape (one read from `.cxaml`, one from its text of
/// Clarity) give lists that match place by place.
fn places(page: &Page) -> Vec<u32> {
    let mut out = vec![page.span.start];
    out.extend(page.data.iter().filter(|d| !d.made).map(|d| d.span.start));
    out.extend(page.tells.iter().map(|t| t.span.start));
    fn commands(items: &[Command], out: &mut Vec<u32>) {
        for c in items {
            out.push(c.span.start);
            commands(&c.items, out);
        }
    }
    fn parts(widgets: &[Widget], out: &mut Vec<u32>) {
        for w in widgets {
            out.push(w.span.start);
            if let Some(menu) = &w.menu {
                out.push(menu.span.start);
                commands(&menu.items, out);
            }
            fn levels(all: &[TreeLevel], out: &mut Vec<u32>) {
                for l in all {
                    out.push(l.span.start);
                    levels(&l.levels, out);
                }
            }
            levels(&w.levels, out);
            out.extend(w.selects.iter().map(|s| s.span.start));
            parts(&w.children, out);
            for p in &w.pages {
                out.push(p.span.start);
                parts(&p.children, out);
            }
        }
    }
    let frame = &page.frame;
    if let Some(menu) = &frame.menu {
        out.push(menu.span.start);
        commands(&menu.items, &mut out);
    }
    if let Some(toolbar) = &frame.toolbar {
        out.push(toolbar.span.start);
        commands(&toolbar.items, &mut out);
    }
    if let Some(ribbon) = &frame.ribbon {
        out.push(ribbon.span.start);
        for tab in &ribbon.tabs {
            out.push(tab.span.start);
            for group in &tab.groups {
                out.push(group.span.start);
                commands(&group.items, &mut out);
            }
        }
    }
    for sidebar in &frame.sidebars {
        out.push(sidebar.span.start);
        parts(&sidebar.widgets, &mut out);
    }
    if let Some(footer) = &frame.footer {
        out.push(footer.span.start);
        parts(&footer.widgets, &mut out);
    }
    parts(&page.widgets, &mut out);
    out
}

/// One page read from text of Clarity (a module line, then its block).
fn block_page(text: &str) -> Result<crate::model::SpecFile, crate::edit::EditError> {
    let (file, diagnostics) = crate::parser::parse(text, "");
    if let Some(d) = diagnostics.iter().find(|d| d.severity == crate::diagnostic::Severity::Error) {
        return Err(crate::edit::EditError(d.message.clone()));
    }
    Ok(file)
}

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

/// An edit of the designer on a window's or view's `.cxaml`: made on the
/// page's text of Clarity, as on a page in a file of Clarity, and the page
/// written back as markup, in the module its root names;
/// `project`, when given, lets edits build on the rest (templates, keys).
pub fn apply(project: Option<&crate::Project>, markup: &str, edit: &crate::edit::Edit) -> crate::edit::EditResult<crate::edit::EditOutcome> {
    use crate::edit::EditError;
    let module = class_of(markup).map(|(m, _)| m).unwrap_or_default();
    let (page, diagnostics) = read_page(markup, "");
    let page = page.ok_or_else(|| EditError(diagnostics.first().map_or("the window's file cannot be read".to_string(), |d| d.message.clone())))?;
    let text = format!("module {module}\n{}", page_block(&page));
    let block = block_page(&text)?;
    let shown = block.pages.first().ok_or_else(|| EditError("the window's file holds no page".into()))?;
    let (from, to) = (places(&page), places(shown));
    let at = |item: u32| from.iter().position(|p| *p == item).and_then(|i| to.get(i).copied()).unwrap_or(item);
    let edit = crate::edit::with_items(edit, &at)?;
    let outcome = match project {
        Some(project) => crate::edit::apply_in(project, &text, &edit)?,
        None => crate::edit::apply(&text, &edit)?,
    };
    let edited = block_page(&outcome.text)?;
    let others = edited.entities.len() + edited.enums.len() + edited.records.len() + edited.apps.len() + edited.code.len() + edited.functions.len();
    if edited.pages.len() != 1 || others > 0 {
        return Err(EditError("this change belongs in the module's file of Clarity, not in a window's .cxaml".into()));
    }
    let new_page = &edited.pages[0];
    let written = with_comments(markup, page_source(new_page, &module));
    // The focus: the same thing in the markup as written.
    let focus = match outcome.focus {
        Some(focus) => {
            let (read, _) = read_page(&written, "");
            let (block_places, markup_places) = (places(new_page), read.map(|p| places(&p)).unwrap_or_default());
            let index = block_places.iter().enumerate().filter(|(_, p)| **p <= focus).max_by_key(|(_, p)| **p).map(|(i, _)| i);
            index.and_then(|i| markup_places.get(i).copied())
        }
        None => None,
    };
    Ok(crate::edit::EditOutcome { text: written, focus, keys: outcome.keys })
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

/// How many bits of a designer's item are the place in a file: the parts of
/// a view drawn in a window's content, from the view's own file, are told
/// apart from the window's by the file's number above them
/// (`design_item`). Only the designer's layer and its edits see these numbers.
pub const DESIGN_PLACE_BITS: u32 = 24;

/// The designer's item for `start` in the project's file number `file`,
/// for a view's parts drawn in a window of another file.
pub fn design_item(file: usize, start: u32) -> u32 {
    ((file as u32 + 1) << DESIGN_PLACE_BITS) | start
}

/// The parts of a view, drawn in another file's window: each place moved
/// to its designer's item in file number `file`.
pub fn design_items(widgets: &mut [Widget], file: usize) {
    fn commands(items: &mut [Command], file: usize) {
        for c in items {
            c.span.start = design_item(file, c.span.start);
            commands(&mut c.items, file);
        }
    }
    for w in widgets {
        w.span.start = design_item(file, w.span.start);
        if let Some(menu) = &mut w.menu {
            menu.span.start = design_item(file, menu.span.start);
            commands(&mut menu.items, file);
        }
        design_items(&mut w.children, file);
        for page in &mut w.pages {
            page.span.start = design_item(file, page.span.start);
            design_items(&mut page.children, file);
        }
    }
}

/// `source` with the comments `markup` had before its root element.
fn with_comments(markup: &str, source: String) -> String {
    let comments = leading_comments(markup);
    match source.find('\n') {
        Some(n) if !comments.is_empty() => format!("{}{}{}", &source[..=n], comments, &source[n + 1..]),
        _ => source,
    }
}

/// A `.cxaml` written again from its page's text of Clarity (`block`, a
/// module line and the page), keeping the comments it had.
pub fn rewritten(markup: &str, block: &str) -> crate::edit::EditResult<String> {
    let file = block_page(block)?;
    let page = file.pages.first().ok_or_else(|| crate::edit::EditError("the window's file holds no page".into()))?;
    let module = class_of(markup).map(|(m, _)| m).unwrap_or_default();
    Ok(with_comments(markup, page_source(page, &module)))
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

/// A module's file of Clarity with its windows and views from `design/`
/// written in as page blocks: the module as one text, as it was before its
/// pages had files of their own.
pub fn module_text_with_pages(module_file: &std::path::Path) -> std::io::Result<String> {
    let mut text = std::fs::read_to_string(module_file)?;
    let module = module_name(&text).unwrap_or("").to_string();
    let dir = module_file.parent().unwrap_or(std::path::Path::new(""));
    for path in page_files(dir, &[], &[])? {
        let markup = std::fs::read_to_string(&path)?;
        if class_of(&markup).is_some_and(|(m, _)| m == module) {
            if let (Some(page), _) = read_page(&markup, &path.display().to_string()) {
                text.push('\n');
                text.push_str(&page_block(&page));
            }
        }
    }
    Ok(text)
}

/// What the Problems list says of a page still written in a module's file:
/// the start of each such message, which its fix (Move pages into design)
/// is found by.
pub const PAGE_LEFT_IN_MODULE: &str = "is written in this module's file; a window or view has a file of its own";

/// A page block left in a module's file of Clarity, each an error: a
/// window or view has its own `.cxaml` in `design/`, where Move pages into
/// design, in Problems (or `clarity convert-pages`), puts it.
pub fn pages_left_in_modules(project: &crate::Project) -> Vec<Diagnostic> {
    project
        .files
        .iter()
        .filter(|f| !is_page_file(&f.path))
        .flat_map(|f| {
            f.pages.iter().map(move |p| {
                Diagnostic::error(format!("{} {PAGE_LEFT_IN_MODULE}, design/{}: Move pages into design, in Problems, moves it", p.name.name, file_name(&p.name.name)), p.name.span).in_file(&f.path)
            })
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

/// Make texts keys over a project whose windows and views are in `.cxaml`
/// (`pages`, each a path and its markup) beside its modules: each page joins
/// as its text of Clarity and comes back as markup, its texts made keys as
/// the modules' are. The result lists the modules and then the pages, each
/// by its own path.
pub fn make_keys_with_pages(modules: &[(&str, &str)], pages: &[(&str, &str)], translations: &[(&str, &str)], default: &str) -> crate::keyed::Keyed {
    // Each page as a module of its own text, under a name of its own so it is read as Clarity.
    let blocks: Vec<(String, String)> = pages
        .iter()
        .filter_map(|(path, markup)| {
            let module = class_of(markup).map(|(m, _)| m).unwrap_or_default();
            let (page, _) = read_page(markup, path);
            page.map(|p| (format!("{path}.{}", crate::SPEC_EXTENSION), format!("module {module}\n{}", page_block(&p))))
        })
        .collect();
    let mut all: Vec<(&str, &str)> = modules.to_vec();
    all.extend(blocks.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    let mut keyed = crate::keyed::make_keys(&all, translations, default);
    for (path, text) in &mut keyed.modules {
        let Some(original) = path.strip_suffix(&format!(".{}", crate::SPEC_EXTENSION)).filter(|p| is_page_file(p)) else { continue };
        let Some((_, markup)) = pages.iter().find(|(p, _)| *p == original) else { continue };
        if let Ok(written) = rewritten(markup, text) {
            *text = written;
        }
        *path = original.to_string();
    }
    keyed
}
