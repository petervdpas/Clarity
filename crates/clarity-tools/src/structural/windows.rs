//! Windows, views and their parts, changed as the page designer changes
//! them: each tool names the page and its parts by name and makes one
//! designer edit on the page's `.cxaml`. See mod.rs for the whole set.

use std::path::{Path, PathBuf};

use clarity_agent::ToolDef;
use clarity_spec::edit::Edit;
use clarity_spec::model::{PageKind, WidgetProps};
use clarity_spec::templates::PageTemplate;
use serde_json::{Map, Value, json};

use super::places::{self, PageAt};
use super::{schema, tool};
use crate::ProjectTools;

/// The templates a new window starts from.
const TEMPLATES: [&str; 6] = ["blank", "form", "browse", "settings", "splash", "toolbox"];

/// Properties a part has that are worked out, never set.
const WORKED_OUT: [&str; 6] = ["kind", "made", "undoScope", "grows", "description", "graphModel"];

fn page_prop() -> Value {
    json!({ "type": "string", "description": "The window or view, by its name (Main, CustomerView)" })
}

fn part_prop(what: &str) -> Value {
    json!({ "type": "string", "description": what })
}

fn in_props() -> Value {
    json!({
        "in": { "type": "string", "description": "Where it goes: a container part by name (panel, form, tabs, splitter...), a sidebar by name, or footer; the page's own grid when left out" },
        "tab": { "type": "integer", "minimum": 1, "description": "With `in` a tabs, steps, accordion or splitter: its tab, step, section or pane, from 1" }
    })
}

fn set_prop() -> Value {
    json!({
        "type": "object",
        "description": "Properties of the part to set, by their names in edits (camelCase): caption, bind, placeholder, tooltip, style, readonly, onClick, columns, edit, show, icon, shortcut, min, max, step, foldable, ... A key not of a part is refused with the keys there are; null clears one.",
        "properties": {
            "caption": { "type": "string" },
            "bind": { "type": "string", "description": "data or data.field of the page" },
            "placeholder": { "type": "string" },
            "tooltip": { "type": "string" },
            "style": { "type": "string" },
            "readonly": { "type": "boolean" },
            "onClick": { "type": "string", "description": "The window a click opens" },
            "columns": { "type": "array", "items": { "type": "string" } },
            "col": { "type": "integer", "minimum": 1 },
            "row": { "type": "integer", "minimum": 1 },
            "width": { "type": "integer", "minimum": 1 },
            "height": { "type": "integer", "minimum": 1 }
        },
        "additionalProperties": true
    })
}

/// Every tool of windows, views and parts.
pub fn definitions() -> Vec<ToolDef> {
    let kinds = places::part_kinds();
    let mut add_part = json!({
        "page": page_prop(),
        "kind": { "type": "string", "enum": kinds, "description": "The part's kind; the tool `parts` describes each" },
        "name": { "type": "string", "description": "Its name, camelCase (emailBox); one after its kind when left out" },
        "col": { "type": "integer", "minimum": 1, "description": "Its first column on the grid, from 1" },
        "row": { "type": "integer", "minimum": 1, "description": "Its first row, from 1" },
        "width": { "type": "integer", "minimum": 1, "description": "Columns it spans; its kind's own when left out" },
        "height": { "type": "integer", "minimum": 1, "description": "Rows it spans; its kind's own when left out" },
        "caption": { "type": "string", "description": "What it says" },
        "bind": { "type": "string", "description": "The data it shows: data or data.field of the page" },
        "part": { "type": "string", "description": "With kind custom: the custom part's declared name" },
        "set": set_prop()
    });
    add_part.as_object_mut().unwrap().extend(in_props().as_object().unwrap().clone());
    let mut place_field = json!({
        "page": page_prop(),
        "field": { "type": "string", "description": "data.field of the page's data (customer.email)" },
        "col": { "type": "integer", "minimum": 1 },
        "row": { "type": "integer", "minimum": 1 }
    });
    place_field.as_object_mut().unwrap().extend(in_props().as_object().unwrap().clone());
    let mut move_part = json!({
        "page": page_prop(),
        "part": part_prop("The part, by its name"),
        "col": { "type": "integer", "minimum": 1, "description": "Its new first column; as it is when left out" },
        "row": { "type": "integer", "minimum": 1, "description": "Its new first row; as it is when left out" }
    });
    move_part.as_object_mut().unwrap().extend(in_props().as_object().unwrap().clone());
    move_part["in"]["description"] = json!("The container it moves into: a container part, a sidebar, footer, or the page by its own name for its grid; where it is when left out");
    vec![
        tool(
            "add_window",
            "Adds a window to the project, as New window does: its .cxaml in design/ and the view its content shows (Blank, Form, Browse), from a template. A dialog or view: add_view.",
            schema(
                json!({
                    "name": { "type": "string", "description": "The window's name, capitalized (Orders)" },
                    "template": { "type": "string", "enum": TEMPLATES, "description": "blank: an empty view in its content; form and browse: of an entity; settings: a dialog of the project's settings; splash; toolbox" },
                    "entity": { "type": "string", "description": "With form and browse: the entity" },
                    "module": { "type": "string", "description": "The module it belongs to; the app's when left out" }
                }),
                &["name", "template"],
            ),
        ),
        tool(
            "add_view",
            "Adds an empty view (which a window's content, a tab or a slide-out shows) or dialog: its .cxaml in design/.",
            schema(
                json!({
                    "name": { "type": "string", "description": "Its name, capitalized (CustomerView)" },
                    "kind": { "type": "string", "enum": ["view", "dialog"], "description": "view when left out" },
                    "module": { "type": "string", "description": "The module it belongs to; the app's when left out" }
                }),
                &["name"],
            ),
        ),
        tool("remove_page", "Removes a window or view: its .cxaml and its code.", schema(json!({ "page": page_prop() }), &["page"])),
        tool(
            "change_page",
            "Changes a window's or view's own settings: its title, grid, size and style. Give only what changes.",
            schema(
                json!({
                    "page": page_prop(),
                    "title": { "type": "string", "description": "Its title; empty for none" },
                    "columns": { "type": "integer", "minimum": 1, "maximum": 96, "description": "Columns of its grid" },
                    "rows": { "type": "integer", "minimum": 0, "description": "The height of a row of its own exact grid in pixels; 0 for the project's rows" },
                    "width": { "type": "integer", "minimum": 1, "description": "The width its window opens at, in pixels, with height" },
                    "height": { "type": "integer", "minimum": 1, "description": "The height its window opens at, in pixels, with width" },
                    "standardSize": { "type": "boolean", "description": "true: it opens at the standard size" },
                    "style": { "type": "string", "description": "A style of the theme for the window itself; empty for none" }
                }),
                &["page"],
            ),
        ),
        tool("add_part", "Places a new part on a window or view, as dropping it from the Parts palette does. A window shows its parts in a view through its content: put parts in the view.", schema(add_part, &["page", "kind", "col", "row"])),
        tool("place_field", "Places a field of the page's data, as dropping it from the data does: its input, with a label and help text when there is room. A list goes on as a table with add_part and bind.", schema(place_field, &["page", "field", "col", "row"])),
        tool("change_part", "Changes a part's properties, as the Properties panel does; the rest stays as it is.", schema(json!({ "page": page_prop(), "part": part_prop("The part, by its name"), "set": set_prop() }), &["page", "part", "set"])),
        tool("rename_part", "Gives a part another name. The page's code that says the old name is not changed: change it too.", schema(json!({ "page": page_prop(), "part": part_prop("The part, by its name"), "to": { "type": "string", "description": "The new name, camelCase" } }), &["page", "part", "to"])),
        tool("move_part", "Moves a part to another cell, or into another container of the same page, as dragging it does.", schema(move_part, &["page", "part"])),
        tool("remove_part", "Removes a part, with what it holds.", schema(json!({ "page": page_prop(), "part": part_prop("The part, by its name") }), &["page", "part"])),
        tool(
            "add_tab",
            "Adds a tab to a tabs part, a step to steps, a section to an accordion, or a view line to a content.",
            schema(json!({ "page": page_prop(), "part": part_prop("The tabs, steps, accordion or content, by its name"), "title": { "type": "string", "description": "Its title; for a content, the view it shows" } }), &["page", "part", "title"]),
        ),
        tool(
            "change_tab",
            "Changes a tab, step, section or pane of a part: its title, the view it shows, a section's color, a splitter's first pane's size (columns side by side, rows stacked). Give only what changes.",
            schema(
                json!({
                    "page": page_prop(),
                    "part": part_prop("The tabs, steps, accordion, splitter or content, by its name"),
                    "tab": { "type": "integer", "minimum": 1, "description": "Which, from 1" },
                    "title": { "type": "string" },
                    "view": { "type": "string", "description": "The view it shows; empty for parts of its own" },
                    "color": { "type": "string", "description": "An accordion section's header color; empty for the theme's" },
                    "paneColumns": { "type": "integer", "minimum": 0, "description": "A splitter's first pane: the columns it starts with side by side, the rows stacked; 0 for panes alike" }
                }),
                &["page", "part", "tab"],
            ),
        ),
        tool("remove_tab", "Removes a tab, step, section or content line of a part, with what it holds.", schema(json!({ "page": page_prop(), "part": part_prop("The part, by its name"), "tab": { "type": "integer", "minimum": 1 } }), &["page", "part", "tab"])),
        tool(
            "add_data",
            "Declares data on a window or view, which its parts bind to: a record of an entity or a record type, a list of them, or a value (text, integer, decimal, money, bool, date, datetime, duration, or an enum).",
            schema(
                json!({
                    "page": page_prop(),
                    "name": { "type": "string", "description": "The data's name, camelCase (customer)" },
                    "entity": { "type": "string", "description": "What it holds: an entity, a record type, an enum or a plain type (text, bool, ...)" },
                    "list": { "type": "boolean", "description": "true for a list of them; only records come as a list" }
                }),
                &["page", "name", "entity"],
            ),
        ),
        tool("remove_data", "Removes data declared on a window or view.", schema(json!({ "page": page_prop(), "name": { "type": "string" } }), &["page", "name"])),
        tool(
            "add_tell",
            "Declares what a view tells the window it shows in, as in saved or saved(name: text); its code says tell saved(name), and the window hears it with wire_event on the view. A tell of the same name is replaced.",
            schema(json!({ "page": page_prop(), "message": { "type": "string", "description": "The tell and its values, like saved(name: text)" } }), &["page", "message"]),
        ),
        tool("remove_tell", "Removes a tell a view declares.", schema(json!({ "page": page_prop(), "tell": { "type": "string" } }), &["page", "tell"])),
        tool(
            "set_part_grows",
            "Whether a custom part grows with its window (its declaration's grows line).",
            schema(json!({ "part": { "type": "string", "description": "The custom part's declared name" }, "grows": { "type": "boolean" } }), &["part", "grows"]),
        ),
    ]
}

/// The tool `name`, when it is one of this file's: its answer.
pub fn call(tools: &mut ProjectTools, name: &str, input: &Value) -> Option<Result<String, String>> {
    let result = match name {
        "add_window" => add_window(tools, input),
        "add_view" => add_view(tools, input),
        "remove_page" => remove_page(tools, input),
        "change_page" => change_page(tools, input),
        "add_part" => add_part(tools, input),
        "place_field" => place_field(tools, input),
        "change_part" => change_part(tools, input),
        "rename_part" => rename_part(tools, input),
        "move_part" => move_part(tools, input),
        "remove_part" => remove_part(tools, input),
        "add_tab" => add_tab(tools, input),
        "change_tab" => change_tab(tools, input),
        "remove_tab" => remove_tab(tools, input),
        "add_data" => add_data(tools, input),
        "remove_data" => remove_data(tools, input),
        "add_tell" => add_tell(tools, input),
        "remove_tell" => remove_tell(tools, input),
        "set_part_grows" => set_part_grows(tools, input),
        _ => return None,
    };
    Some(result)
}

// ---------------------------------------------------------------------------
// Reading the input

/// A text given, trimmed; none when left out or empty.
fn text(input: &Value, key: &str) -> Option<String> {
    input.get(key).and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty()).map(str::to_string)
}

/// A text given, possibly empty; none when left out.
fn given(input: &Value, key: &str) -> Option<String> {
    input.get(key).and_then(Value::as_str).map(|t| t.trim().to_string())
}

fn required(input: &Value, key: &str) -> Result<String, String> {
    text(input, key).ok_or_else(|| format!("give {key}"))
}

/// A whole number given; refused when it is not one.
fn number(input: &Value, key: &str) -> Result<Option<u32>, String> {
    match input.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_u64().and_then(|n| u32::try_from(n).ok()).map(Some).ok_or_else(|| format!("{key} is a whole number, not {v}")),
    }
}

fn flag(input: &Value, key: &str) -> Result<Option<bool>, String> {
    match input.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(v) => Err(format!("{key} is true or false, not {v}")),
    }
}

fn dir_of(tools: &ProjectTools) -> Result<PathBuf, String> {
    Ok(tools.open()?.1.clone())
}

/// The page the input names.
fn page_of(tools: &ProjectTools, input: &Value) -> Result<(PathBuf, PageAt), String> {
    let dir = dir_of(tools)?;
    let page = places::page(&dir, &required(input, "page")?)?;
    Ok((dir, page))
}

/// Makes `edit` on `file` and says `what` it did.
fn make(tools: &mut ProjectTools, dir: &Path, file: &Path, edit: &Edit, what: &str) -> Result<String, String> {
    let applied = crate::edits::apply(dir, file, edit)?;
    tools.applied(what, applied)
}

/// The name of the part that starts at `focus` in the page file, after an edit.
fn part_named_at(file: &Path, focus: Option<u32>) -> Option<String> {
    let markup = std::fs::read_to_string(file).ok()?;
    let page = match clarity_spec::cxaml::read_designed(&markup, "").ok()? {
        clarity_spec::design_edit::Designed::Page(page) => page,
        clarity_spec::design_edit::Designed::App(_) => return None,
    };
    let focus = focus?;
    places::page_parts(&page).into_iter().find(|w| w.span.start == focus).and_then(|w| w.props.name.clone())
}

/// `props` with the properties of `set` (camelCase, as in edits) put over them.
fn merged(props: &WidgetProps, set: &Value) -> Result<WidgetProps, String> {
    let Some(set) = set.as_object() else {
        return Err("set is an object of the part's properties, like {\"caption\": \"Email\"}".into());
    };
    let Value::Object(mut current) = serde_json::to_value(props).map_err(|err| err.to_string())? else {
        return Err("a part's properties cannot be read".into());
    };
    let keys: Vec<String> = current.keys().filter(|k| !WORKED_OUT.contains(&k.as_str())).cloned().collect();
    for (key, value) in set {
        if WORKED_OUT.contains(&key.as_str()) {
            return Err(format!("{key} is not set on a part: {}", if key == "kind" { "remove it and add one of the other kind" } else { "Clarity works it out" }));
        }
        if !current.contains_key(key) {
            return Err(format!("a part has no property '{key}'; the properties are {}", keys.join(", ")));
        }
        current.insert(key.clone(), value.clone());
    }
    serde_json::from_value(Value::Object(current)).map_err(|err| format!("a property's value is not one it takes: {err}"))
}

fn kind_word(props: &WidgetProps) -> &'static str {
    clarity_spec::widgets::keyword(props.kind)
}

// ---------------------------------------------------------------------------
// Windows and views

fn add_window(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let dir = dir_of(tools)?;
    let name = required(input, "name")?;
    let template_word = required(input, "template")?;
    let entity = || required(input, "entity").map_err(|_| format!("a {template_word} window is of an entity: give entity"));
    let template = match template_word.as_str() {
        "blank" => PageTemplate::Blank,
        "form" => PageTemplate::Form { entity: entity()? },
        "browse" => PageTemplate::Browse { entity: entity()? },
        "settings" => PageTemplate::Settings,
        "splash" => PageTemplate::Splash,
        "toolbox" => PageTemplate::Toolbox,
        other => return Err(format!("there is no template '{other}'; the templates are {}", TEMPLATES.join(", "))),
    };
    let module = places::module_for_page(&dir, text(input, "module").as_deref())?;
    make(tools, &dir, &module, &Edit::AddWindow { name: name.clone(), template }, &format!("Added window {name} ({template_word})"))
}

fn add_view(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let dir = dir_of(tools)?;
    let name = required(input, "name")?;
    let edit = match text(input, "kind").as_deref() {
        None | Some("view") => Edit::AddWindow { name: name.clone(), template: PageTemplate::View },
        Some("dialog") => Edit::AddPage { name: name.clone(), kind: PageKind::Dialog },
        Some(other) => return Err(format!("kind is view or dialog, not '{other}'; a window: add_window")),
    };
    let word = if matches!(edit, Edit::AddPage { .. }) { "dialog" } else { "view" };
    let module = places::module_for_page(&dir, text(input, "module").as_deref())?;
    make(tools, &dir, &module, &edit, &format!("Added {word} {name}"))
}

fn remove_page(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let name = at.page.name.name.clone();
    make(tools, &dir, &at.file, &Edit::Remove { item: at.page.span.start }, &format!("Removed {name}"))
}

fn change_page(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let page = &at.page;
    let start = page.span.start;
    let mut edits = Vec::new();
    let mut said = Vec::new();
    if let Some(title) = given(input, "title") {
        said.push(if title.is_empty() { "no title".to_string() } else { format!("title \"{title}\"") });
        edits.push(Edit::SetTitle { target: start, title });
    }
    let columns = number(input, "columns")?;
    let rows = number(input, "rows")?;
    if columns.is_some() || rows.is_some() {
        let columns = columns.unwrap_or(page.columns);
        let rows = match rows {
            Some(0) => None,
            Some(rows) => Some(rows),
            None => page.cell,
        };
        said.push(format!("a grid of {columns} columns"));
        edits.push(Edit::SetGrid { page: start, columns, rows });
    }
    let (width, height) = (number(input, "width")?, number(input, "height")?);
    if flag(input, "standardSize")? == Some(true) {
        if width.is_some() || height.is_some() {
            return Err("give a width and height, or standardSize, not both".into());
        }
        said.push("the standard size".into());
        edits.push(Edit::SetSize { page: start, width: None, height: None });
    } else if width.is_some() || height.is_some() {
        let (Some(width), Some(height)) = (width, height) else { return Err("a size is a width and a height: give both".into()) };
        said.push(format!("a size of {width} x {height}"));
        edits.push(Edit::SetSize { page: start, width: Some(width), height: Some(height) });
    }
    if let Some(style) = given(input, "style") {
        said.push(if style.is_empty() { "no style".to_string() } else { format!("style {style}") });
        edits.push(Edit::SetPageStyle { page: start, style });
    }
    let edit = match edits.len() {
        0 => return Err("give what changes: title, columns, rows, width and height, standardSize or style".into()),
        1 => edits.remove(0),
        _ => Edit::Batch { edits },
    };
    let name = page.name.name.clone();
    make(tools, &dir, &at.file, &edit, &format!("Gave {name} {}", said.join(", ")))
}

// ---------------------------------------------------------------------------
// Parts

fn add_part(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let kind = places::part_kind(&required(input, "kind")?)?;
    let container = places::container(&at.page, text(input, "in").as_deref(), number(input, "tab")?)?;
    let (col, row) = (number(input, "col")?.ok_or("give col, from 1")?, number(input, "row")?.ok_or("give row, from 1")?);
    let mut props = clarity_spec::widgets::new_props(kind, col, row);
    if let Some(width) = number(input, "width")? {
        props.width = width;
    } else {
        // As the designer does: no wider than the room to the container's edge.
        props.width = props.width.min(container.columns.saturating_sub(col) + 1).max(1);
    }
    if let Some(height) = number(input, "height")? {
        props.height = height;
    }
    if let Some(caption) = given(input, "caption") {
        props.caption = (!caption.is_empty()).then_some(caption);
    }
    props.bind = text(input, "bind");
    props.part = text(input, "part");
    if kind == clarity_spec::widgets::WidgetKind::Custom && props.part.is_none() {
        return Err("a custom part names its declaration: give part".into());
    }
    if let Some(name) = text(input, "name") {
        if places::part_names(&at.page).contains(&name) {
            return Err(format!("{} has a part {name} already", at.page.name.name));
        }
        props.name = Some(name);
    }
    if let Some(set) = input.get("set").filter(|s| !s.is_null()) {
        props = merged(&props, set)?;
    }
    let word = kind_word(&props);
    let page = at.page.name.name.clone();
    let applied = crate::edits::apply(&dir, &at.file, &Edit::PlaceWidget { container: container.at, props })?;
    let name = part_named_at(&at.file, applied.focus).unwrap_or_default();
    tools.applied(&format!("Added {word} {name} to {page}"), applied)
}

fn place_field(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let field = required(input, "field")?;
    let Some((data, _)) = field.split_once('.') else {
        return Err(format!("'{field}' is not a field: give data.field; a list goes on with add_part, kind table, bind {field}"));
    };
    places::page_data(&at.page, data)?;
    let container = places::container(&at.page, text(input, "in").as_deref(), number(input, "tab")?)?;
    let (col, row) = (number(input, "col")?.ok_or("give col, from 1")?, number(input, "row")?.ok_or("give row, from 1")?);
    let room = container.columns.saturating_sub(col) + 1;
    let page = at.page.name.name.clone();
    let applied = crate::edits::apply(&dir, &at.file, &Edit::PlaceField { container: container.at, col, row, room, bind: field.clone() })?;
    let name = part_named_at(&at.file, applied.focus).unwrap_or_default();
    tools.applied(&format!("Placed {field} on {page} as {name}"), applied)
}

fn change_part(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let name = required(input, "part")?;
    let widget = places::part(&at.page, &name)?;
    let set = input.get("set").cloned().unwrap_or(Value::Null);
    if set.as_object().is_some_and(Map::is_empty) {
        return Err("give in set what changes".into());
    }
    let props = merged(&widget.props, &set)?;
    let keys: Vec<&str> = set.as_object().map(|o| o.keys().map(String::as_str).collect()).unwrap_or_default();
    let what = format!("Changed {} of {name} in {}", keys.join(", "), at.page.name.name);
    make(tools, &dir, &at.file, &Edit::UpdateWidget { item: widget.span.start, container: None, props }, &what)
}

fn rename_part(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let name = required(input, "part")?;
    let to = required(input, "to")?;
    let widget = places::part(&at.page, &name)?;
    if !clarity_spec::names::is_camel_case(&to) {
        return Err(clarity_spec::names::camel_case_problem("part", &to));
    }
    if to != name && places::part_names(&at.page).contains(&to) {
        return Err(format!("{} has a part {to} already", at.page.name.name));
    }
    let mut props = widget.props.clone();
    props.name = Some(to.clone());
    let what = format!("Renamed {name} to {to} in {}", at.page.name.name);
    make(tools, &dir, &at.file, &Edit::UpdateWidget { item: widget.span.start, container: None, props }, &what)
}

fn move_part(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let name = required(input, "part")?;
    let widget = places::part(&at.page, &name)?;
    let inside = text(input, "in");
    let tab = number(input, "tab")?;
    let container = match (&inside, tab) {
        (None, None) => None,
        (inside, tab) => Some(places::container(&at.page, inside.as_deref(), tab)?.at),
    };
    let mut props = widget.props.clone();
    props.col = number(input, "col")?.unwrap_or(props.col);
    props.row = number(input, "row")?.unwrap_or(props.row);
    if container.is_none() && props == widget.props {
        return Err("give where it goes: col and row, or in (and tab)".into());
    }
    let what = format!("Moved {name} to {}, {}{}", props.col, props.row, inside.map(|i| format!(" in {i}")).unwrap_or_default());
    make(tools, &dir, &at.file, &Edit::UpdateWidget { item: widget.span.start, container, props }, &what)
}

fn remove_part(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let name = required(input, "part")?;
    let item = places::part(&at.page, &name)?.span.start;
    make(tools, &dir, &at.file, &Edit::Remove { item }, &format!("Removed {name} from {}", at.page.name.name))
}

// ---------------------------------------------------------------------------
// Tabs, steps, sections and panes

fn add_tab(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let name = required(input, "part")?;
    let title = required(input, "title")?;
    let item = places::part(&at.page, &name)?.span.start;
    make(tools, &dir, &at.file, &Edit::AddTab { item, title: title.clone() }, &format!("Added \"{title}\" to {name} in {}", at.page.name.name))
}

fn change_tab(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let name = required(input, "part")?;
    let number_of = number(input, "tab")?.ok_or("give tab, from 1")?;
    let widget = places::part(&at.page, &name)?;
    let target = places::tab(widget, number_of)?.span.start;
    let mut edits = Vec::new();
    if let Some(title) = given(input, "title") {
        edits.push(Edit::SetTitle { target, title });
    }
    if let Some(view) = given(input, "view") {
        edits.push(Edit::SetPageView { target, view: (!view.is_empty()).then_some(view) });
    }
    if let Some(color) = given(input, "color") {
        edits.push(Edit::SetPageColor { target, color: (!color.is_empty()).then_some(color) });
    }
    if let Some(columns) = number(input, "paneColumns")? {
        edits.push(Edit::SetPaneSpan { target, columns: (columns > 0).then_some(columns) });
    }
    let edit = match edits.len() {
        0 => return Err("give what changes: title, view, color or paneColumns".into()),
        1 => edits.remove(0),
        _ => Edit::Batch { edits },
    };
    make(tools, &dir, &at.file, &edit, &format!("Changed tab {number_of} of {name} in {}", at.page.name.name))
}

fn remove_tab(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let name = required(input, "part")?;
    let number_of = number(input, "tab")?.ok_or("give tab, from 1")?;
    let item = places::tab(places::part(&at.page, &name)?, number_of)?.span.start;
    make(tools, &dir, &at.file, &Edit::Remove { item }, &format!("Removed tab {number_of} of {name} in {}", at.page.name.name))
}

// ---------------------------------------------------------------------------
// Data and custom parts

/// The plain types window data can hold, as the designer offers them (`PLAIN_TYPES` in runtimes/web/src/screen/windowData.ts).
const PLAIN_DATA_TYPES: [&str; 8] = ["text", "integer", "decimal", "money", "bool", "date", "datetime", "duration"];

fn add_data(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let name = required(input, "name")?;
    let entity = required(input, "entity")?;
    let project = places::model_project(&dir)?;
    let list = flag(input, "list")?.unwrap_or(false);
    // What the designer's Data panel offers: a record of an entity or a record type, a list of them, or a value (a plain type or an enum).
    let records: Vec<&str> = places::entity_names(&project).into_iter().chain(places::record_names(&project)).collect();
    let values: Vec<&str> = PLAIN_DATA_TYPES.iter().copied().chain(places::enum_names(&project)).collect();
    if values.contains(&entity.as_str()) {
        if list {
            return Err(format!("{entity} is a value; only records come as a list: {}", crate::listed(&records)));
        }
    } else if !records.contains(&entity.as_str()) {
        let all: Vec<&str> = records.iter().chain(values.iter()).copied().collect();
        return Err(format!("there is no entity, record, enum or plain type {entity}; data can hold {}", crate::listed(&all)));
    }
    let of = if list { format!("list<{entity}>") } else { entity.clone() };
    let edit = Edit::AddData { page: at.page.span.start, name: name.clone(), entity, list };
    make(tools, &dir, &at.file, &edit, &format!("Declared data {name}: {of} on {}", at.page.name.name))
}

fn remove_data(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let name = required(input, "name")?;
    let item = places::page_data(&at.page, &name)?.span.start;
    make(tools, &dir, &at.file, &Edit::Remove { item }, &format!("Removed data {name} from {}", at.page.name.name)).map_err(|err| {
        if err.starts_with(&format!("unknown name '{name}'")) { format!("the code uses {name}, so it stays until that code changes: {err}") } else { err }
    })
}

fn add_tell(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let message = required(input, "message")?;
    let name: String = message.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    let edit = Edit::SetPageTell { page: at.page.span.start, name: name.clone(), message: Some(message.clone()) };
    make(tools, &dir, &at.file, &edit, &format!("{} tells {message}", at.page.name.name))
}

fn remove_tell(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, at) = page_of(tools, input)?;
    let tell = required(input, "tell")?;
    let edit = Edit::SetPageTell { page: at.page.span.start, name: tell.clone(), message: None };
    make(tools, &dir, &at.file, &edit, &format!("{} no longer tells {tell}", at.page.name.name))
}

fn set_part_grows(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let dir = dir_of(tools)?;
    let name = required(input, "part")?;
    let grows = flag(input, "grows")?.ok_or("give grows, true or false")?;
    let (file, item) = places::part_decl(&dir, &name)?;
    let what = if grows { format!("{name} grows with its window") } else { format!("{name} keeps its rows") };
    make(tools, &dir, &file, &Edit::SetPartGrows { item, grows }, &what)
}

#[cfg(test)]
mod tests {
    use clarity_agent::Tools;
    use serde_json::json;

    use crate::ProjectTools;

    /// A new project in a scratch folder (the Main window showing MainView), and its tools.
    fn project(name: &str) -> (std::path::PathBuf, std::path::PathBuf, ProjectTools) {
        let root = std::env::temp_dir().join(format!("clarity-windows-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        let tools = ProjectTools::new(Some((file, dir.clone())), root.clone());
        (root, dir, tools)
    }

    fn view(dir: &std::path::Path, name: &str) -> String {
        std::fs::read_to_string(dir.join("design").join(format!("{name}.cxaml"))).unwrap_or_default()
    }

    /// The project with an entity Task the pages can hold.
    fn with_task(tools: &mut ProjectTools) {
        let wrote = tools.call("write_file", &json!({"path": "todo.clarity", "text": "module todo\n\nentity Task\n    title: text(80)\n    done: bool\n"}));
        assert!(!wrote.is_error, "{}", wrote.text);
    }

    /// No edit leaves parts reaching past what holds them: a memo too tall for
    /// the window is cut to the rows the window holds from its content area's top.
    #[test]
    fn an_edit_cuts_a_part_to_fit_its_window() {
        let (root, dir, mut tools) = project("fit");
        let added = tools.call("add_part", &json!({"page": "MainView", "kind": "memo", "name": "notesBox", "col": 1, "row": 1, "width": 12, "height": 40}));
        assert!(!added.is_error, "{}", added.text);
        let text = view(&dir, "MainView");
        assert!(!text.contains("rows=\"40\""), "{text}");
        let file = dir.with_extension("clarity");
        let settings = clarity_project::load(&file).settings.unwrap();
        let project = clarity_project::load_modules(&dir).unwrap();
        let scrolls: Vec<String> = clarity_project::target_problems(&settings, &project, "").into_iter().map(|d| d.message).filter(|m| m.contains("scrolls") || m.contains("holds")).collect();
        assert!(scrolls.is_empty(), "{scrolls:?}\n{text}");
        // The cut is part of the edit: one Undo takes it back with it.
        assert!(tools.written.iter().any(|w| w.file.ends_with("design/MainView.cxaml")));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_part_is_added_changed_renamed_moved_and_removed_by_name() {
        let (root, dir, mut tools) = project("parts");
        let added = tools.call("add_part", &json!({"page": "MainView", "kind": "textbox", "name": "emailBox", "col": 1, "row": 2, "width": 6, "set": {"placeholder": "Email"}}));
        assert!(!added.is_error, "{}", added.text);
        assert!(added.text.contains("Added textbox emailBox to MainView"), "{}", added.text);
        let text = view(&dir, "MainView");
        assert!(text.contains("<Textbox name=\"emailBox\" at=\"1,2\" span=\"6\""), "{text}");
        assert!(tools.written.iter().any(|w| w.file.ends_with("design/MainView.cxaml")));

        let changed = tools.call("change_part", &json!({"page": "MainView", "part": "emailBox", "set": {"readonly": true, "tooltip": "Where we write"}}));
        assert!(!changed.is_error, "{}", changed.text);
        assert!(view(&dir, "MainView").contains("readonly=\"true\""), "{}", view(&dir, "MainView"));

        let moved = tools.call("move_part", &json!({"page": "MainView", "part": "emailBox", "col": 3, "row": 4}));
        assert!(!moved.is_error, "{}", moved.text);
        assert!(view(&dir, "MainView").contains("name=\"emailBox\" at=\"3,4\""), "{}", view(&dir, "MainView"));

        let renamed = tools.call("rename_part", &json!({"page": "MainView", "part": "emailBox", "to": "mailBox"}));
        assert!(!renamed.is_error, "{}", renamed.text);
        assert!(view(&dir, "MainView").contains("name=\"mailBox\""), "{}", view(&dir, "MainView"));

        let removed = tools.call("remove_part", &json!({"page": "MainView", "part": "mailBox"}));
        assert!(!removed.is_error, "{}", removed.text);
        assert!(!view(&dir, "MainView").contains("mailBox"), "{}", view(&dir, "MainView"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unknown_names_kinds_and_properties_are_refused_with_what_there_is() {
        let (root, dir, mut tools) = project("unknown");
        let page = tools.call("add_part", &json!({"page": "Mian", "kind": "button", "col": 1, "row": 1}));
        assert!(page.is_error && page.text.contains("Main and MainView"), "{}", page.text);
        assert!(!page.text.contains("did you mean"), "{}", page.text);

        let kind = tools.call("add_part", &json!({"page": "MainView", "kind": "knob", "col": 1, "row": 1}));
        assert!(kind.is_error && kind.text.contains("knob") && kind.text.contains("textbox"), "{}", kind.text);

        let part = tools.call("change_part", &json!({"page": "Main", "part": "nothing", "set": {"caption": "x"}}));
        assert!(part.is_error && part.text.contains("main"), "the content part is listed: {}", part.text);

        tools.call("add_part", &json!({"page": "MainView", "kind": "button", "name": "saveButton", "col": 1, "row": 1}));
        let property = tools.call("change_part", &json!({"page": "MainView", "part": "saveButton", "set": {"colour": "red"}}));
        assert!(property.is_error && property.text.contains("colour") && property.text.contains("caption"), "{}", property.text);
        let into = tools.call("add_part", &json!({"page": "MainView", "kind": "label", "in": "saveButton", "col": 1, "row": 1}));
        assert!(into.is_error && into.text.contains("holds no parts"), "{}", into.text);
        assert!(!view(&dir, "MainView").contains("label"), "nothing written: {}", view(&dir, "MainView"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn windows_and_views_are_added_changed_and_removed() {
        let (root, dir, mut tools) = project("windows");
        let added = tools.call("add_window", &json!({"name": "Orders", "template": "blank"}));
        assert!(!added.is_error, "{}", added.text);
        assert!(view(&dir, "Orders").contains("<Window class=\"todo.Orders\""), "{}", view(&dir, "Orders"));
        assert!(view(&dir, "OrdersView").contains("<View class=\"todo.OrdersView\""), "{}", view(&dir, "OrdersView"));

        let form = tools.call("add_window", &json!({"name": "TaskCard", "template": "form"}));
        assert!(form.is_error && form.text.contains("entity"), "{}", form.text);
        let template = tools.call("add_window", &json!({"name": "TaskCard", "template": "wizard"}));
        assert!(template.is_error && template.text.contains("blank, form, browse"), "{}", template.text);

        let dialog = tools.call("add_view", &json!({"name": "Ask", "kind": "dialog"}));
        assert!(!dialog.is_error && view(&dir, "Ask").contains("<Dialog class=\"todo.Ask\""), "{}", dialog.text);

        let changed = tools.call("change_page", &json!({"page": "Orders", "title": "All orders", "width": 800, "height": 600}));
        assert!(!changed.is_error, "{}", changed.text);
        let text = view(&dir, "Orders");
        assert!(text.contains("width=\"800\"") && text.contains("height=\"600\""), "{text}");
        let grid = tools.call("change_page", &json!({"page": "OrdersView", "columns": 24}));
        assert!(!grid.is_error && view(&dir, "OrdersView").contains("24"), "{}", grid.text);

        let removed = tools.call("remove_page", &json!({"page": "Ask"}));
        assert!(!removed.is_error, "{}", removed.text);
        assert!(!dir.join("design/Ask.cxaml").exists() || view(&dir, "Ask").is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn data_fields_and_tabs_go_on_by_name() {
        let (root, dir, mut tools) = project("data");
        with_task(&mut tools);
        let entity = tools.call("add_data", &json!({"page": "MainView", "name": "task", "entity": "Tsak"}));
        assert!(entity.is_error && entity.text.contains("Task"), "{}", entity.text);
        let data = tools.call("add_data", &json!({"page": "MainView", "name": "task", "entity": "Task"}));
        assert!(!data.is_error, "{}", data.text);
        assert!(view(&dir, "MainView").contains("<Data name=\"task\" type=\"Task\""), "{}", view(&dir, "MainView"));

        // What the designer's Data panel declares, the tool declares too: a list of records, plain values, an enum.
        let wrote = tools.call("write_file", &json!({"path": "todo.clarity", "text": "module todo\n\nentity Task\n    title: text(80)\n    done: bool\n\nenum Level\n    Low\n    High\n\nrecord Row\n    key: text\n    label: text\n"}));
        assert!(!wrote.is_error, "{}", wrote.text);
        let rows = tools.call("add_data", &json!({"page": "MainView", "name": "rows", "entity": "Row", "list": true}));
        assert!(!rows.is_error, "{}", rows.text);
        let note = tools.call("add_data", &json!({"page": "MainView", "name": "note", "entity": "text"}));
        assert!(!note.is_error, "{}", note.text);
        let level = tools.call("add_data", &json!({"page": "MainView", "name": "level", "entity": "Level"}));
        assert!(!level.is_error, "{}", level.text);
        let text = view(&dir, "MainView");
        assert!(text.contains("<Data name=\"rows\" type=\"list&lt;Row&gt;\"") && text.contains("<Data name=\"note\" type=\"text\"") && text.contains("<Data name=\"level\" type=\"Level\""), "{text}");
        let plain_list = tools.call("add_data", &json!({"page": "MainView", "name": "notes", "entity": "text", "list": true}));
        assert!(plain_list.is_error && plain_list.text.contains("list"), "{}", plain_list.text);
        let unknown = tools.call("add_data", &json!({"page": "MainView", "name": "thing", "entity": "Thing"}));
        assert!(unknown.is_error && unknown.text.contains("Row") && unknown.text.contains("text"), "{}", unknown.text);
        // Plain data is removed by name like any other.
        let flag = tools.call("add_data", &json!({"page": "MainView", "name": "reviewed", "entity": "bool"}));
        assert!(!flag.is_error, "{}", flag.text);
        let removed = tools.call("remove_data", &json!({"page": "MainView", "name": "reviewed"}));
        assert!(!removed.is_error, "{}", removed.text);
        assert!(!view(&dir, "MainView").contains("name=\"reviewed\""), "{}", view(&dir, "MainView"));
        let flag = tools.call("add_data", &json!({"page": "Main", "name": "reviewed", "entity": "bool"}));
        assert!(!flag.is_error, "{}", flag.text);
        let removed = tools.call("remove_data", &json!({"page": "Main", "name": "reviewed"}));
        assert!(!removed.is_error, "{}", removed.text);
        assert!(!view(&dir, "Main").contains("name=\"reviewed\""), "{}", view(&dir, "Main"));
        // Data the code still uses stays, and the answer says where the code uses it.
        let noted = tools.call("add_data", &json!({"page": "MainView", "name": "memo", "entity": "text"}));
        assert!(!noted.is_error, "{}", noted.text);
        let module = std::fs::read_to_string(dir.join("todo.clarity")).unwrap();
        std::fs::write(dir.join("todo.clarity"), format!("{module}\ncode MainView\n    handler memoShown\n        memo = \"x\"\n")).unwrap();
        let used = tools.call("remove_data", &json!({"page": "MainView", "name": "memo"}));
        assert!(used.is_error && used.text.contains("the code uses memo") && used.text.contains("todo.clarity:"), "{}", used.text);

        let placed = tools.call("place_field", &json!({"page": "MainView", "field": "task.title", "col": 1, "row": 1}));
        assert!(!placed.is_error, "{}", placed.text);
        assert!(view(&dir, "MainView").contains("bind=\"task.title\""), "{}", view(&dir, "MainView"));
        let wrong = tools.call("place_field", &json!({"page": "MainView", "field": "order.title", "col": 1, "row": 3}));
        assert!(wrong.is_error && wrong.text.contains("task"), "{}", wrong.text);

        let tabs = tools.call("add_part", &json!({"page": "MainView", "kind": "tabs", "name": "pages", "col": 1, "row": 4, "width": 12, "height": 6}));
        assert!(!tabs.is_error, "{}", tabs.text);
        let tab = tools.call("add_tab", &json!({"page": "MainView", "part": "pages", "title": "Notes"}));
        assert!(!tab.is_error, "{}", tab.text);
        let inside = tools.call("add_part", &json!({"page": "MainView", "kind": "label", "name": "notesLabel", "in": "pages", "tab": 2, "col": 1, "row": 1, "caption": "Notes"}));
        assert!(!inside.is_error, "{}", inside.text);
        let text = view(&dir, "MainView");
        let notes = text.find("notesLabel").unwrap();
        assert!(text[..notes].matches("<Tab title").count() == 2, "{text}");
        let missing = tools.call("add_part", &json!({"page": "MainView", "kind": "label", "in": "pages", "tab": 5, "col": 1, "row": 1}));
        assert!(missing.is_error && missing.text.contains("has 2"), "{}", missing.text);

        let retitled = tools.call("change_tab", &json!({"page": "MainView", "part": "pages", "tab": 2, "title": "Remarks"}));
        assert!(!retitled.is_error, "{}", retitled.text);
        let removed = tools.call("remove_tab", &json!({"page": "MainView", "part": "pages", "tab": 2}));
        assert!(!removed.is_error && !view(&dir, "MainView").contains("notesLabel"), "{}", removed.text);
        let gone = tools.call("remove_data", &json!({"page": "MainView", "name": "nothing"}));
        assert!(gone.is_error && gone.text.contains("task"), "{}", gone.text);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn every_tool_lists_its_static_values_as_enums() {
        let defs = super::definitions();
        let add_part = defs.iter().find(|d| d.name == "add_part").unwrap();
        let kinds = add_part.input_schema["properties"]["kind"]["enum"].as_array().unwrap();
        assert!(kinds.iter().any(|k| k == "textbox") && kinds.iter().any(|k| k == "table"));
        let add_window = defs.iter().find(|d| d.name == "add_window").unwrap();
        assert!(add_window.input_schema["properties"]["template"]["enum"].as_array().unwrap().iter().any(|t| t == "browse"));
    }
}
