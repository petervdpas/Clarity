//! The app and every window's frame, events and handlers: the app itself,
//! its main window and splash, menus, toolbars, ribbons, sidebars and the
//! footer, their items, the events of windows, the app and parts, and the
//! code of handlers. See mod.rs for the structural tools as a whole.

use std::path::{Path, PathBuf};

use clarity_agent::ToolDef;
use clarity_spec::Project;
use clarity_spec::edit::{Edit, FrameItem};
use clarity_spec::frame::{BUILT_IN_COMMANDS, RegionKind};
use clarity_spec::model::{Handler, PageKind, Side, TellHandler};
use clarity_spec::widgets::{self, Event};
use serde_json::{Value, json};

use super::places::{self, FrameOwner};
use super::{schema, tool};
use crate::ProjectTools;

/// What names a window, view or the app in every tool here.
const OWNER: &str = "A window's or view's name, like Main, or app for the app.";

const REGIONS: [&str; 5] = ["menu", "toolbar", "ribbon", "sidebar", "footer"];
const SIDES: [&str; 2] = ["left", "right"];
const ITEMS: [&str; 5] = ["item", "button", "separator", "tab", "group"];
const OWNER_EVENTS: [&str; 5] = ["open", "close", "return", "start", "unconnected"];

fn events() -> Vec<&'static str> {
    widgets::ALL_EVENTS.iter().map(|e| e.keyword()).collect()
}

pub fn definitions() -> Vec<ToolDef> {
    let owner = json!({ "type": "string", "description": OWNER });
    vec![
        tool(
            "add_app",
            "Adds the app to a module: the application, which has the main window, the splash and the frame every window shares. A project has one app.",
            schema(
                json!({
                    "name": { "type": "string", "description": "The app's name, like Shop." },
                    "module": { "type": "string", "description": "The module it goes in; needed only when the project has several." }
                }),
                &["name"],
            ),
        ),
        tool(
            "set_main_window",
            "Sets the window the app shows at start.",
            schema(json!({ "window": { "type": "string", "description": "A window's name, like Main." } }), &["window"]),
        ),
        tool(
            "set_splash",
            "Sets the splash page the app shows while it starts, and how long it shows at least; without a page, the app has no splash.",
            schema(
                json!({
                    "page": { "type": "string", "description": "A splash page's name (a page of kind splash); leave it out to take the splash away." },
                    "seconds": { "type": "number", "description": "How long it shows at least, 0.1 to 30; leave it out to close it as soon as the main window shows." }
                }),
                &[],
            ),
        ),
        tool(
            "add_region",
            "Docks a region with starter content to a window or the app: a menu, toolbar, ribbon, footer, or a sidebar on a side. A window's region replaces the app's of that kind. Tells the new region's name.",
            schema(
                json!({
                    "window": owner,
                    "region": { "type": "string", "enum": REGIONS },
                    "side": { "type": "string", "enum": SIDES, "description": "Sidebars only; left when left out." }
                }),
                &["window", "region"],
            ),
        ),
        tool(
            "add_command",
            "Adds an item at the end of a menu, toolbar, ribbon, ribbon tab or group, or under a menu item (a submenu): an item in a menu, a button in a toolbar or ribbon group, a separator in either, a tab in the ribbon, a group in a ribbon tab. Tells the new item's name.",
            schema(
                json!({
                    "window": owner,
                    "in": { "type": "string", "description": "The name of the menu, toolbar, ribbon, ribbon tab, ribbon group or menu item it goes in, like mainMenu." },
                    "kind": { "type": "string", "enum": ITEMS },
                    "caption": { "type": "string", "description": "What it says; a starter caption when left out." }
                }),
                &["window", "in", "kind"],
            ),
        ),
        tool(
            "change_command",
            "Changes a menu item or a toolbar or ribbon button: what it says, its name, icon and shortcut, and what a click does: open a window (or a built-in target such as quit), or run a handler, which is written in the code behind when it is new. What is left out stays; an empty text takes it away.",
            schema(
                json!({
                    "window": owner,
                    "command": { "type": "string", "description": "The item's or button's name, like openItem." },
                    "caption": { "type": "string" },
                    "name": { "type": "string", "description": "A new name, camelCase." },
                    "icon": { "type": "string" },
                    "shortcut": { "type": "string", "description": "Like Ctrl+O." },
                    "large": { "type": "boolean", "description": "A large ribbon button." },
                    "opens": { "type": "string", "description": "The window a click opens, or a built-in target: quit, close, language, theme, keys, undo, redo, back, forward." },
                    "with": { "type": "array", "items": { "type": "string" }, "description": "The data of this window the opened one is filled with, each by the same name." },
                    "new_window": { "type": "boolean", "description": "The window opens in a desktop window of its own." },
                    "runs": { "type": "string", "description": "The handler a click runs, like openClicked." }
                }),
                &["window", "command"],
            ),
        ),
        tool(
            "change_sidebar",
            "Changes a sidebar: its side, name, width in pixels (100 to 1600), how narrow and wide the person may drag it, and whether it resizes, folds, starts folded or fills the window. What is left out stays; null takes min or max away.",
            schema(
                json!({
                    "window": owner,
                    "sidebar": { "type": "string", "description": "The sidebar's name, like leftSidebar." },
                    "side": { "type": "string", "enum": SIDES },
                    "name": { "type": "string" },
                    "width": { "type": "integer" },
                    "min": { "type": ["integer", "null"] },
                    "max": { "type": ["integer", "null"] },
                    "resizable": { "type": "boolean" },
                    "collapsible": { "type": "boolean" },
                    "collapsed": { "type": "boolean" },
                    "maximizable": { "type": "boolean" },
                    "columns": { "type": "integer", "description": "Columns of its own grid." }
                }),
                &["window", "sidebar"],
            ),
        ),
        tool(
            "change_ribbon",
            "Changes the ribbon of a window or the app: its name, whether the person folds it to its tabs, and whether it starts folded. What is left out stays.",
            schema(
                json!({
                    "window": owner,
                    "name": { "type": "string" },
                    "collapsible": { "type": "boolean" },
                    "collapsed": { "type": "boolean" }
                }),
                &["window"],
            ),
        ),
        tool(
            "rename_frame_part",
            "Renames the menu, the toolbar, or a ribbon tab or group of a window or the app (a menu item or button is renamed with change_command).",
            schema(
                json!({
                    "window": owner,
                    "part": { "type": "string", "description": "Its name now, like mainMenu." },
                    "name": { "type": "string", "description": "Its new name, camelCase." }
                }),
                &["window", "part", "name"],
            ),
        ),
        tool(
            "set_frame_title",
            "Sets what a ribbon tab or group says, or the footer's status text.",
            schema(
                json!({
                    "window": owner,
                    "part": { "type": "string", "description": "The ribbon tab's or group's name, or footer for the footer's status." },
                    "title": { "type": "string" }
                }),
                &["window", "part", "title"],
            ),
        ),
        tool(
            "move_frame_part",
            "Moves a menu item, button, ribbon tab, ribbon group or sidebar one place up or down among its own kind.",
            schema(
                json!({
                    "window": owner,
                    "part": { "type": "string", "description": "Its name." },
                    "direction": { "type": "string", "enum": ["up", "down"] }
                }),
                &["window", "part", "direction"],
            ),
        ),
        tool(
            "remove_frame_part",
            "Removes a menu item or button, a ribbon tab or group, a sidebar, or a whole region (the menu, toolbar or ribbon by its name, footer for the footer) of a window or the app.",
            schema(json!({ "window": owner, "part": { "type": "string", "description": "Its name, or footer." } }), &["window", "part"]),
        ),
        tool(
            "add_context_menu",
            "Gives a part a menu with one item: its context menu, or a split button's dropdown. Tells the item's name; add_command adds more under it.",
            schema(json!({ "window": owner, "part": { "type": "string", "description": "The part's name." } }), &["window", "part"]),
        ),
        tool(
            "set_event",
            "Wires one of a window's own events (open, close, return) or the app's (start, unconnected) to a handler, written in the code behind when it is new; without a handler the event is unwired.",
            schema(
                json!({
                    "window": owner,
                    "event": { "type": "string", "enum": OWNER_EVENTS },
                    "handler": { "type": "string", "description": "Like windowOpened; leave it out to unwire." }
                }),
                &["window", "event"],
            ),
        ),
        tool(
            "wire_event",
            "Wires an event of a part (or a tell of a custom part, or of a view shown by the window's content) to a handler, written in the code behind when it is new; without a handler it is unwired. A click that runs a handler opens no window.",
            schema(
                json!({
                    "window": owner,
                    "part": { "type": "string", "description": "The part's name, like saveButton." },
                    "view": { "type": "string", "description": "Instead of part: a view the window's content shows, like ReviewView; give its tell." },
                    "event": { "type": "string", "enum": events(), "description": "One of the part's events; the parts tool lists each kind's." },
                    "tell": { "type": "string", "description": "Instead of event: a tell of a custom part, like scored, or of the view." },
                    "handler": { "type": "string", "description": "Like saveClicked; leave it out to unwire." }
                }),
                &["window"],
            ),
        ),
        tool(
            "set_handler_code",
            "Replaces the code of a handler in the code behind of a window, view or the app: its statements, without the handler line. The handler must exist; set_event and wire_event make one.",
            schema(
                json!({
                    "window": owner,
                    "handler": { "type": "string" },
                    "code": { "type": "string", "description": "Its statements, one per line, indented as among themselves." }
                }),
                &["window", "handler", "code"],
            ),
        ),
        tool(
            "name_parts",
            "Gives every part, region, menu item and button that has no name the first free one of its kind, in one window or view, or in every one.",
            schema(json!({ "window": { "type": "string", "description": "A window, view or app; every one when left out." } }), &[]),
        ),
        tool(
            "camel_names",
            "Renames every part and handler that is not camelCase to camelCase, wherever it is used, in one window or view, or in every one.",
            schema(json!({ "window": { "type": "string", "description": "A window, view or app; every one when left out." } }), &[]),
        ),
    ]
}

pub fn call(tools: &mut ProjectTools, name: &str, input: &Value) -> Option<Result<String, String>> {
    let result = match name {
        "add_app" => add_app(tools, input),
        "set_main_window" => set_main_window(tools, input),
        "set_splash" => set_splash(tools, input),
        "add_region" => add_region(tools, input),
        "add_command" => add_command(tools, input),
        "change_command" => change_command(tools, input),
        "change_sidebar" => change_sidebar(tools, input),
        "change_ribbon" => change_ribbon(tools, input),
        "rename_frame_part" => rename_frame_part(tools, input),
        "set_frame_title" => set_frame_title(tools, input),
        "move_frame_part" => move_frame_part(tools, input),
        "remove_frame_part" => remove_frame_part(tools, input),
        "add_context_menu" => add_context_menu(tools, input),
        "set_event" => set_event(tools, input),
        "wire_event" => wire_event(tools, input),
        "set_handler_code" => set_handler_code(tools, input),
        "name_parts" => housekeeping(tools, input, Edit::NameParts, "Named the parts"),
        "camel_names" => housekeeping(tools, input, Edit::CamelNames, "Made the names camelCase"),
        _ => return None,
    };
    Some(result)
}

// ---------------------------------------------------------------------------
// Reading the input

/// A text given, trimmed; none when left out. An empty text says "take it away".
fn text(input: &Value, key: &str) -> Option<String> {
    input.get(key).and_then(Value::as_str).map(|s| s.trim().to_string())
}

/// A text that must be given and not be empty.
fn required(input: &Value, key: &str) -> Result<String, String> {
    text(input, key).filter(|s| !s.is_empty()).ok_or_else(|| format!("give {key}"))
}

fn flag(input: &Value, key: &str) -> Option<bool> {
    input.get(key).and_then(Value::as_bool)
}

fn number(input: &Value, key: &str) -> Result<Option<u32>, String> {
    match input.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_u64().and_then(|n| u32::try_from(n).ok()).map(Some).ok_or_else(|| format!("{key} is a whole number")),
    }
}

/// The value of `key`, one of `allowed`, as the edit's type.
fn one_of<T: serde::de::DeserializeOwned>(input: &Value, key: &str, allowed: &[&str]) -> Result<T, String> {
    let given = required(input, key)?;
    if !allowed.contains(&given.as_str()) {
        return Err(format!("{key} is one of {}, not {given}", crate::listed(allowed)));
    }
    serde_json::from_value(json!(given)).map_err(|err| err.to_string())
}

// ---------------------------------------------------------------------------
// The project and the edit

fn loaded(tools: &ProjectTools) -> Result<(PathBuf, Project), String> {
    let (_, dir) = tools.open()?.clone();
    let project = places::model_project(&dir)?;
    Ok((dir, project))
}

/// Makes `edit` on `file` as the designer does, and tells `what` with what it changed.
fn apply(tools: &mut ProjectTools, what: &str, file: &Path, edit: &Edit) -> Result<String, String> {
    let (_, dir) = tools.open()?.clone();
    let applied = crate::edits::apply(&dir, file, edit)?;
    tools.applied(what, applied)
}

/// As `apply`, telling the name of what the edit made: `what` says it with `{}`.
fn apply_naming(tools: &mut ProjectTools, what: &str, owner: &str, file: &Path, edit: &Edit) -> Result<String, String> {
    let (_, dir) = tools.open()?.clone();
    let applied = crate::edits::apply(&dir, file, edit)?;
    let name = applied.focus.and_then(|at| {
        let project = places::model_project(&dir).ok()?;
        let owner = places::frame_owner(&project, &dir, owner).ok()?;
        named_at(&owner, at)
    });
    let said = match name {
        Some(name) => what.replace("{}", &name),
        None => what.replace(" {}", ""),
    };
    tools.applied(&said, applied)
}

/// The name of the region, command, ribbon tab or group that starts at `at`.
fn named_at(owner: &FrameOwner<'_>, at: u32) -> Option<String> {
    let frame = owner.frame();
    fn commands(list: &[clarity_spec::model::Command], at: u32) -> Option<String> {
        list.iter().find_map(|c| if c.span.start == at { c.props.name.clone() } else { commands(&c.items, at) })
    }
    fn parts(list: &[clarity_spec::model::Widget], at: u32) -> Option<String> {
        list.iter().find_map(|w| w.menu.as_ref().and_then(|m| commands(&m.items, at)).or_else(|| parts(&w.children, at)).or_else(|| w.pages.iter().find_map(|p| parts(&p.children, at))))
    }
    if let Some(menu) = &frame.menu {
        if menu.span.start == at {
            return menu.name.clone();
        }
        if let Some(found) = commands(&menu.items, at) {
            return Some(found);
        }
    }
    if let Some(toolbar) = &frame.toolbar {
        if toolbar.span.start == at {
            return toolbar.name.clone();
        }
        if let Some(found) = commands(&toolbar.items, at) {
            return Some(found);
        }
    }
    if let Some(ribbon) = &frame.ribbon {
        if ribbon.span.start == at {
            return ribbon.name.clone();
        }
        for tab in &ribbon.tabs {
            if tab.span.start == at {
                return tab.name.clone();
            }
            for group in &tab.groups {
                if group.span.start == at {
                    return group.name.clone();
                }
                if let Some(found) = commands(&group.items, at) {
                    return Some(found);
                }
            }
        }
    }
    if let Some(sidebar) = frame.sidebars.iter().find(|s| s.span.start == at) {
        return sidebar.props.name.clone();
    }
    if frame.footer.as_ref().is_some_and(|f| f.span.start == at) {
        return Some("footer".into());
    }
    let mut lists: Vec<&[clarity_spec::model::Widget]> = owner.page.map(|p| p.widgets.as_slice()).into_iter().collect();
    lists.extend(frame.sidebars.iter().map(|s| s.widgets.as_slice()));
    lists.extend(frame.footer.iter().map(|f| f.widgets.as_slice()));
    lists.into_iter().find_map(|list| parts(list, at))
}

// ---------------------------------------------------------------------------
// The app

fn add_app(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (_, project) = loaded(tools)?;
    let name = required(input, "name")?;
    let file = match text(input, "module").filter(|m| !m.is_empty()) {
        Some(module) => places::module_file(&project, &module)?,
        None => match places::module_names(&project).as_slice() {
            [one] => places::module_file(&project, one)?,
            names => return Err(format!("say which module the app goes in: {}", crate::listed(names))),
        },
    };
    apply(tools, &format!("Added the app {name}"), &file, &Edit::AddApp { name })
}

fn set_main_window(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let window = required(input, "window")?;
    let app = places::frame_owner(&project, &dir, "app")?;
    let windows = places::page_names_of(&project, &[PageKind::Window]);
    if !windows.contains(&window.as_str()) {
        return Err(format!("there is no window {window}; the windows are {}", crate::listed(&windows)));
    }
    let edit = Edit::SetMain { app: app.start, main: window.clone() };
    apply(tools, &format!("Set the main window to {window}"), &app.file.clone(), &edit)
}

fn set_splash(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let app = places::frame_owner(&project, &dir, "app")?;
    let page = text(input, "page").unwrap_or_default();
    let ms = match input.get("seconds").and_then(Value::as_f64) {
        Some(seconds) => Some((seconds * 1000.0).round() as u32),
        None => None,
    };
    let splashes = places::page_names_of(&project, &[PageKind::Splash]);
    if !page.is_empty() && !splashes.contains(&page.as_str()) {
        return Err(if splashes.is_empty() {
            format!("there is no splash page {page}; the project has no splash pages yet (a page of kind splash)")
        } else {
            format!("there is no splash page {page}; the splash pages are {}", crate::listed(&splashes))
        });
    }
    let what = if page.is_empty() { "Took the splash away".to_string() } else { format!("Set the splash to {page}") };
    apply(tools, &what, &app.file.clone(), &Edit::SetSplash { app: app.start, splash: page, ms })
}

// ---------------------------------------------------------------------------
// Regions and their items

fn add_region(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    let region: RegionKind = one_of(input, "region", &REGIONS)?;
    let side: Option<Side> = match text(input, "side").filter(|s| !s.is_empty()) {
        Some(_) => Some(one_of(input, "side", &SIDES)?),
        None if region == RegionKind::Sidebar => Some(Side::Left),
        None => None,
    };
    let what = format!("Added a {} {{}} to {}", clarity_spec::frame::keyword(region), owner.name);
    // Its starter items get names, as every part has one.
    let edit = Edit::Batch { edits: vec![Edit::AddRegion { owner: owner.start, region, side }, Edit::NameParts] };
    let (name, file) = (owner.name.clone(), owner.file.clone());
    apply_naming(tools, &what, &name, &file, &edit)
}

fn add_command(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    let parent_name = required(input, "in")?;
    let parent = places::frame_parent(&owner, &parent_name)?;
    let item: FrameItem = one_of(input, "kind", &ITEMS)?;
    let caption = text(input, "caption").filter(|c| !c.is_empty());
    let kind = required(input, "kind")?;
    let article = if kind.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
    let what = format!("Added {article} {kind} {{}} to {parent_name} in {}", owner.name);
    let edit = Edit::AddFrameItem { parent, item, caption };
    let (name, file) = (owner.name.clone(), owner.file.clone());
    apply_naming(tools, &what, &name, &file, &edit)
}

fn change_command(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    let name = required(input, "command")?;
    let command = places::frame_command(&owner, &name)?;
    let mut props = command.props.clone();
    let cleared = |v: String| (!v.is_empty()).then_some(v);
    if let Some(caption) = text(input, "caption") {
        props.caption = cleared(caption);
    }
    if let Some(new_name) = text(input, "name") {
        props.name = Some(new_name).filter(|n| !n.is_empty()).or(props.name);
    }
    if let Some(icon) = text(input, "icon") {
        props.icon = cleared(icon);
    }
    if let Some(shortcut) = text(input, "shortcut") {
        props.shortcut = cleared(shortcut);
    }
    if let Some(large) = flag(input, "large") {
        props.large = large;
    }
    let opens = text(input, "opens");
    let runs = text(input, "runs");
    if opens.as_deref().is_some_and(|o| !o.is_empty()) && runs.as_deref().is_some_and(|r| !r.is_empty()) {
        return Err("a click opens a window or runs a handler, not both".into());
    }
    if let Some(opens) = opens {
        if opens.is_empty() {
            props.on_click = None;
            props.open_with.clear();
            props.new_window = false;
        } else {
            let pages: Vec<&str> = project.pages().map(|p| p.name.name.as_str()).collect();
            if !pages.contains(&opens.as_str()) && !BUILT_IN_COMMANDS.contains(&opens.as_str()) {
                return Err(format!("{opens} is no window and no built-in target; the windows are {} and the built-in targets {}", crate::listed(&pages), crate::listed(BUILT_IN_COMMANDS)));
            }
            props.on_click = Some(opens);
            props.handler = None;
        }
    }
    if let Some(with) = input.get("with").and_then(Value::as_array) {
        props.open_with = with.iter().filter_map(Value::as_str).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    }
    if let Some(new_window) = flag(input, "new_window") {
        props.new_window = new_window;
    }
    let mut handler = None;
    if let Some(runs) = runs {
        if runs.is_empty() {
            props.handler = None;
        } else {
            props.handler = Some(runs.clone());
            props.on_click = None;
            props.open_with.clear();
            props.new_window = false;
            handler = Some(runs);
        }
    }
    let update = Edit::UpdateCommand { item: command.span.start, props };
    let edit = match handler {
        Some(handler) => Edit::Batch { edits: vec![update, Edit::AddHandler { owner: owner.name.clone(), name: handler }] },
        None => update,
    };
    apply(tools, &format!("Changed {name} in {}", owner.name), &owner.file.clone(), &edit)
}

fn change_sidebar(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    let name = required(input, "sidebar")?;
    let sidebar = places::frame_sidebar(&owner, &name)?;
    let mut props = sidebar.props.clone();
    if text(input, "side").is_some() {
        props.side = one_of(input, "side", &SIDES)?;
    }
    if let Some(new_name) = text(input, "name").filter(|n| !n.is_empty()) {
        props.name = Some(new_name);
    }
    if let Some(width) = number(input, "width")? {
        props.width = width;
    }
    if input.get("min").is_some() {
        props.min = number(input, "min")?;
    }
    if input.get("max").is_some() {
        props.max = number(input, "max")?;
    }
    if let Some(columns) = number(input, "columns")? {
        props.columns = columns;
    }
    for (key, slot) in [("resizable", &mut props.resizable), ("collapsible", &mut props.collapsible), ("collapsed", &mut props.collapsed), ("maximizable", &mut props.maximizable)] {
        if let Some(on) = flag(input, key) {
            *slot = on;
        }
    }
    let edit = Edit::UpdateSidebar { item: sidebar.span.start, props };
    apply(tools, &format!("Changed {name} in {}", owner.name), &owner.file.clone(), &edit)
}

fn change_ribbon(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    let Some(ribbon) = &owner.frame().ribbon else {
        return Err(format!("{} has no ribbon; add one with add_region", owner.name));
    };
    let name = text(input, "name").filter(|n| !n.is_empty()).or(ribbon.name.clone());
    let collapsible = flag(input, "collapsible").unwrap_or(ribbon.collapsible);
    let collapsed = flag(input, "collapsed").unwrap_or(ribbon.collapsed);
    let edit = Edit::UpdateRibbon { item: ribbon.span.start, name, collapsible, collapsed };
    apply(tools, &format!("Changed the ribbon of {}", owner.name), &owner.file.clone(), &edit)
}

fn rename_frame_part(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    let part = required(input, "part")?;
    let name = required(input, "name")?;
    let item = places::frame_named(&owner, &part)?;
    apply(tools, &format!("Renamed {part} to {name} in {}", owner.name), &owner.file.clone(), &Edit::SetName { item, name: name.clone() })
}

fn set_frame_title(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    let part = required(input, "part")?;
    let title = required(input, "title")?;
    let target = if part == "footer" {
        let footer = owner.frame().footer.as_ref().ok_or_else(|| format!("{} has no footer; add one with add_region", owner.name))?;
        footer.status_span.map(|s| s.start).ok_or_else(|| format!("the footer of {} has no status text", owner.name))?
    } else {
        places::ribbon_part(&owner, &part)?
    };
    apply(tools, &format!("Set the title of {part} in {} to {title}", owner.name), &owner.file.clone(), &Edit::SetTitle { target, title: title.clone() })
}

fn move_frame_part(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    let part = required(input, "part")?;
    let direction = required(input, "direction")?;
    let delta = match direction.as_str() {
        "up" => -1,
        "down" => 1,
        other => return Err(format!("direction is up or down, not {other}")),
    };
    let item = places::frame_command(&owner, &part)
        .map(|c| c.span.start)
        .or_else(|_| places::ribbon_part(&owner, &part))
        .or_else(|_| places::frame_sidebar(&owner, &part).map(|s| s.span.start))
        .map_err(|_| {
            let mut names = places::command_names(&owner);
            for tab in owner.frame().ribbon.iter().flat_map(|r| &r.tabs) {
                names.extend(tab.name.as_deref());
                names.extend(tab.groups.iter().filter_map(|g| g.name.as_deref()));
            }
            names.extend(owner.frame().sidebars.iter().filter_map(|s| s.props.name.as_deref()));
            format!("there is no menu item, button, ribbon tab, group or sidebar {part} in {}; there are {}", owner.name, crate::listed(&names))
        })?;
    apply(tools, &format!("Moved {part} {direction} in {}", owner.name), &owner.file.clone(), &Edit::MoveSibling { item, delta })
}

fn remove_frame_part(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    let part = required(input, "part")?;
    let frame = owner.frame();
    let region = |name: &Option<String>, start: u32| (name.as_deref() == Some(part.as_str())).then_some(start);
    let item = if part == "footer" {
        frame.footer.as_ref().map(|f| f.span.start).ok_or_else(|| format!("{} has no footer", owner.name))?
    } else if let Some(at) = frame.menu.as_ref().and_then(|m| region(&m.name, m.span.start)).or_else(|| frame.toolbar.as_ref().and_then(|t| region(&t.name, t.span.start))).or_else(|| frame.ribbon.as_ref().and_then(|r| region(&r.name, r.span.start))) {
        at
    } else {
        places::frame_command(&owner, &part)
            .map(|c| c.span.start)
            .or_else(|_| places::ribbon_part(&owner, &part))
            .or_else(|_| places::frame_sidebar(&owner, &part).map(|s| s.span.start))
            .map_err(|_| {
                let mut names: Vec<&str> = Vec::new();
                names.extend(frame.menu.iter().filter_map(|m| m.name.as_deref()));
                names.extend(frame.toolbar.iter().filter_map(|t| t.name.as_deref()));
                names.extend(frame.ribbon.iter().filter_map(|r| r.name.as_deref()));
                names.extend(places::command_names(&owner));
                for tab in frame.ribbon.iter().flat_map(|r| &r.tabs) {
                    names.extend(tab.name.as_deref());
                    names.extend(tab.groups.iter().filter_map(|g| g.name.as_deref()));
                }
                names.extend(frame.sidebars.iter().filter_map(|s| s.props.name.as_deref()));
                if frame.footer.is_some() {
                    names.push("footer");
                }
                format!("there is no frame part {part} in {}; there are {}", owner.name, crate::listed(&names))
            })?
    };
    apply(tools, &format!("Removed {part} from {}", owner.name), &owner.file.clone(), &Edit::Remove { item })
}

fn add_context_menu(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    let part = required(input, "part")?;
    let widget = places::frame_part(&owner, &part)?;
    let what = format!("Gave {part} a menu with the item {{}}");
    let edit = Edit::AddMenu { item: widget.span.start };
    let (name, file) = (owner.name.clone(), owner.file.clone());
    apply_naming(tools, &what, &name, &file, &edit)
}

// ---------------------------------------------------------------------------
// Events and handlers

fn set_event(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    let event: Event = one_of(input, "event", &OWNER_EVENTS)?;
    let allowed: &[Event] = if owner.app.is_some() { &widgets::APP_EVENTS } else { &widgets::WINDOW_EVENTS };
    if !allowed.contains(&event) {
        let names: Vec<&str> = allowed.iter().map(|e| e.keyword()).collect();
        return Err(format!("{} has the events {}, not {}", owner.name, crate::listed(&names), event.keyword()));
    }
    let handler = text(input, "handler").filter(|h| !h.is_empty());
    let wire = Edit::SetOwnerEvent { owner: owner.start, event, handler: handler.clone() };
    let (edit, what) = match &handler {
        Some(h) => (Edit::Batch { edits: vec![wire, Edit::AddHandler { owner: owner.name.clone(), name: h.clone() }] }, format!("Wired {} of {} to {h}", event.keyword(), owner.name)),
        None => (wire, format!("Unwired {} of {}", event.keyword(), owner.name)),
    };
    apply(tools, &what, &owner.file.clone(), &edit)
}

fn wire_event(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    if let Some(view) = text(input, "view").filter(|v| !v.is_empty()) {
        return wire_view_tell(tools, &project, &owner, &view, input);
    }
    let part = required(input, "part")?;
    let widget = places::frame_part(&owner, &part)?;
    let handler = text(input, "handler").filter(|h| !h.is_empty());
    let mut props = widget.props.clone();
    let tell = text(input, "tell").filter(|t| !t.is_empty());
    let given_event = text(input, "event").filter(|e| !e.is_empty());
    let (written, said) = match (tell, given_event) {
        (Some(_), Some(_)) => return Err("give event or tell, not both".into()),
        (None, None) => return Err("give event, or tell for a custom part's tell".into()),
        (Some(tell), None) => {
            let declared = props.part.as_deref().and_then(|p| project.files.iter().flat_map(|f| &f.parts).find(|d| d.name.name == p));
            let Some(declared) = declared else { return Err(format!("{part} is no custom part, so it has no tells; give event")) };
            let Some(message) = declared.tells.iter().find(|t| t.name.name == tell) else {
                let names: Vec<&str> = declared.tells.iter().map(|t| t.name.name.as_str()).collect();
                return Err(if names.is_empty() { format!("{} tells nothing", declared.name.name) } else { format!("{} has no tell {tell}; it tells {}", declared.name.name, crate::listed(&names)) });
            };
            let params = message.params.iter().map(|p| format!("{}: {}", p.name.name, clarity_spec::cxaml::type_text(&p.ty))).collect::<Vec<_>>().join(", ");
            props.tells.retain(|t| t.tell != tell);
            if let Some(h) = &handler {
                props.tells.push(TellHandler { tell: tell.clone(), handler: h.clone() });
            }
            (handler.clone().map(|h| Edit::AddHandlerWith { owner: owner.name.clone(), name: h, params }), tell)
        }
        (None, Some(_)) => {
            let event: Event = one_of(input, "event", &events())?;
            let allowed = widgets::info(props.kind).events;
            if !allowed.contains(&event) {
                let names: Vec<&str> = allowed.iter().map(|e| e.keyword()).collect();
                let kind = widgets::keyword(props.kind);
                return Err(if names.is_empty() { format!("a {kind} has no events") } else { format!("a {kind} has the events {}, not {}", crate::listed(&names), event.keyword()) });
            }
            props.handlers.retain(|h| h.event != event);
            if let Some(h) = &handler {
                props.handlers.push(Handler { event, handler: h.clone(), args: Vec::new() });
                // A click runs a handler or opens a window, not both.
                if event == Event::Click {
                    props.on_click = None;
                }
            }
            props.handlers.sort_by_key(|h| allowed.iter().position(|e| *e == h.event));
            (handler.clone().map(|h| Edit::AddHandler { owner: owner.name.clone(), name: h }), event.keyword().to_string())
        }
    };
    let update = Edit::UpdateWidget { item: widget.span.start, container: None, props };
    let edit = match written {
        Some(write) => Edit::Batch { edits: vec![update, write] },
        None => update,
    };
    let what = match &handler {
        Some(h) => format!("Wired {said} of {part} to {h}"),
        None => format!("Unwired {said} of {part}"),
    };
    apply(tools, &what, &owner.file.clone(), &edit)
}

/// A tell of `view` wired on the line of the window's content that shows
/// it, with a handler that takes the tell's values; unwired without a handler.
fn wire_view_tell(tools: &mut ProjectTools, project: &clarity_spec::Project, owner: &places::FrameOwner, view: &str, input: &Value) -> Result<String, String> {
    if text(input, "event").is_some_and(|e| !e.is_empty()) {
        return Err(format!("a view's line hears its tells, not events; give tell, one that {view} tells"));
    }
    let line = places::view_line(owner, view)?;
    let tell = text(input, "tell").filter(|t| !t.is_empty()).ok_or_else(|| format!("give tell, one that {view} tells"))?;
    let shown = project.pages().find(|p| p.name.name == view).ok_or_else(|| format!("there is no view {view}"))?;
    let Some(message) = shown.tells.iter().find(|t| t.name.name == tell) else {
        let names: Vec<&str> = shown.tells.iter().map(|t| t.name.name.as_str()).collect();
        return Err(if names.is_empty() { format!("{view} tells nothing; add_tell declares a tell") } else { format!("{view} has no tell {tell}; it tells {}", crate::listed(&names)) });
    };
    let params = message.params.iter().map(|p| format!("{}: {}", p.name.name, clarity_spec::cxaml::type_text(&p.ty))).collect::<Vec<_>>().join(", ");
    let handler = text(input, "handler").filter(|h| !h.is_empty());
    let wire = Edit::SetViewTell { target: line.span.start, tell: tell.clone(), handler: handler.clone() };
    let (edit, what) = match &handler {
        Some(h) => (Edit::Batch { edits: vec![wire, Edit::AddHandlerWith { owner: owner.name.clone(), name: h.clone(), params }] }, format!("Wired {tell} of {view} to {h}")),
        None => (wire, format!("Unwired {tell} of {view}")),
    };
    apply(tools, &what, &owner.file.clone(), &edit)
}

fn set_handler_code(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let owner = places::frame_owner(&project, &dir, &required(input, "window")?)?;
    let handler = required(input, "handler")?;
    let code = input.get("code").and_then(Value::as_str).ok_or("give code")?;
    let names = places::handler_names(&project, &owner.name);
    if !names.contains(&handler.as_str()) {
        return Err(if names.is_empty() {
            format!("{} has no handlers yet; set_event and wire_event make one", owner.name)
        } else {
            format!("{} has no handler {handler}; its handlers are {}", owner.name, crate::listed(&names))
        });
    }
    // The code is in the module's file of a window's or view's .cxaml.
    let module = if clarity_spec::cxaml::is_page_file(&owner.file.to_string_lossy()) {
        clarity_spec::cxaml::module_file(&owner.file).ok_or_else(|| format!("{} is in no module's folder", owner.file.display()))?
    } else {
        owner.file.clone()
    };
    let source = std::fs::read_to_string(&module).map_err(|err| format!("cannot read {}: {err}", module.display()))?;
    let current = clarity_spec::code::edit::code_text(&source, &owner.name);
    let whole = with_body(&current, &handler, code).ok_or_else(|| format!("the handler {handler} of {} is not where its code is", owner.name))?;
    let edit = Edit::SetCode { owner: owner.name.clone(), text: whole };
    apply(tools, &format!("Wrote the code of {handler} in {}", owner.name), &owner.file.clone(), &edit)
}

/// `code` (a code block's text, as `code_text` gives it) with the
/// statements of handler `name` replaced by `body`; none when it has no
/// such handler.
fn with_body(code: &str, name: &str, body: &str) -> Option<String> {
    let lines: Vec<&str> = code.lines().collect();
    let is_head = |line: &str| line.strip_prefix("handler ").is_some_and(|rest| rest.split(|c: char| !(c.is_alphanumeric() || c == '_')).next() == Some(name));
    let head = lines.iter().position(|l| is_head(l))?;
    let end = lines[head + 1..].iter().position(|l| !l.trim().is_empty() && !l.starts_with(char::is_whitespace)).map_or(lines.len(), |i| head + 1 + i);
    // The blank lines before the next declaration stay between them.
    let mut kept = end;
    while kept > head + 1 && lines[kept - 1].trim().is_empty() {
        kept -= 1;
    }
    let body_lines: Vec<&str> = body.trim_end().lines().collect();
    let indent = body_lines.iter().filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    let mut out: Vec<String> = lines[..=head].iter().map(|l| l.to_string()).collect();
    let mut leading = true;
    for line in body_lines {
        if line.trim().is_empty() {
            if !leading {
                out.push(String::new());
            }
            continue;
        }
        leading = false;
        out.push(format!("    {}", &line[indent.min(line.len() - line.trim_start().len())..]));
    }
    out.extend(lines[kept..end].iter().map(|l| l.to_string()));
    if end < lines.len() && kept == end {
        // The next declaration keeps a blank line before it.
        out.push(String::new());
    }
    out.extend(lines[end..].iter().map(|l| l.to_string()));
    Some(out.iter().map(|l| format!("{l}\n")).collect())
}

/// NameParts or CamelNames on one window, view or the app, or on every one.
fn housekeeping(tools: &mut ProjectTools, input: &Value, edit: Edit, what: &str) -> Result<String, String> {
    let (dir, project) = loaded(tools)?;
    let files: Vec<PathBuf> = match text(input, "window").filter(|w| !w.is_empty()) {
        Some(window) => vec![places::frame_owner(&project, &dir, &window)?.file],
        None => {
            let mut files: Vec<PathBuf> = Vec::new();
            for file in project.files.iter().filter(|f| !f.pages.is_empty() || !f.apps.is_empty()) {
                let path = Path::new(&file.path);
                let path = if path.is_absolute() { path.to_path_buf() } else { dir.join(path) };
                if !files.contains(&path) {
                    files.push(path);
                }
            }
            files
        }
    };
    let mut all = crate::edits::Applied::default();
    for file in &files {
        match crate::edits::apply(&dir, file, &edit) {
            Ok(applied) => {
                for change in applied.changes {
                    // A module written by several files' edits is one change, before the first to after the last.
                    match all.changes.iter_mut().find(|c| c.file == change.file) {
                        Some(kept) => kept.after = change.after,
                        None => all.changes.push(change),
                    }
                }
            }
            Err(err) => {
                let shown = clarity_project::slashed(file.strip_prefix(&dir).unwrap_or(file));
                let done = tools.applied(what, all)?;
                return Err(format!("{shown}: {err}. Before it: {done}"));
            }
        }
    }
    tools.applied(what, all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clarity_agent::Tools;

    /// A new project in a scratch folder: the app Todo, its window Main and the view MainView.
    fn project(name: &str) -> (PathBuf, PathBuf, PathBuf, ProjectTools) {
        let root = std::env::temp_dir().join(format!("clarity-frame-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        let tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        (root, file, dir, tools)
    }

    fn ok(tools: &mut ProjectTools, name: &str, input: Value) -> String {
        let result = tools.call(name, &input);
        assert!(!result.is_error, "{name} {input}: {}", result.text);
        result.text
    }

    fn refused(tools: &mut ProjectTools, name: &str, input: Value) -> String {
        let result = tools.call(name, &input);
        assert!(result.is_error, "{name} {input} was not refused: {}", result.text);
        result.text
    }

    fn read(dir: &Path, path: &str) -> String {
        std::fs::read_to_string(dir.join(path)).unwrap()
    }

    /// A page of the module, written as its own file: a root of `kind` (Window, Splash).
    fn add_page(dir: &Path, kind: &str, name: &str) {
        std::fs::write(dir.join(format!("design/{name}.cxaml")), format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{kind} class=\"todo.{name}\">\n</{kind}>\n")).unwrap();
    }

    /// A view says what it tells; the window that shows it hears a tell on
    /// the view's line, with a handler that takes the tell's values.
    #[test]
    fn a_view_tells_and_its_window_hears_it() {
        let (root, _, dir, mut tools) = project("tells");
        assert!(refused(&mut tools, "add_tell", json!({"page": "Main", "message": "saved"})).contains("only a view tells"));
        ok(&mut tools, "add_tell", json!({"page": "MainView", "message": "saved(name: text)"}));
        assert!(read(&dir, "design/MainView.cxaml").contains("<Tells message=\"saved(name: text)\" />"), "{}", read(&dir, "design/MainView.cxaml"));

        let wired = ok(&mut tools, "wire_event", json!({"window": "Main", "view": "MainView", "tell": "saved", "handler": "mainSaved"}));
        assert!(wired.contains("Wired saved of MainView to mainSaved"), "{wired}");
        assert!(read(&dir, "design/Main.cxaml").contains("<View name=\"MainView\" onsaved=\"mainSaved\" />"), "{}", read(&dir, "design/Main.cxaml"));
        assert!(read(&dir, "todo.clarity").contains("handler mainSaved(name: text)"), "{}", read(&dir, "todo.clarity"));
        assert!(refused(&mut tools, "wire_event", json!({"window": "Main", "view": "MainView", "tell": "closed", "handler": "x"})).contains("MainView has no tell closed; it tells saved"));
        assert!(refused(&mut tools, "wire_event", json!({"window": "Main", "view": "OtherView", "tell": "saved", "handler": "x"})).contains("Main shows no view OtherView; it shows MainView"));
        assert!(refused(&mut tools, "wire_event", json!({"window": "Main", "view": "MainView", "event": "click", "handler": "x"})).contains("tell"));

        ok(&mut tools, "wire_event", json!({"window": "Main", "view": "MainView", "tell": "saved"}));
        assert!(read(&dir, "design/Main.cxaml").contains("<View name=\"MainView\" />"), "{}", read(&dir, "design/Main.cxaml"));
        ok(&mut tools, "remove_tell", json!({"page": "MainView", "tell": "saved"}));
        assert!(!read(&dir, "design/MainView.cxaml").contains("<Tells"), "{}", read(&dir, "design/MainView.cxaml"));
        assert!(refused(&mut tools, "remove_tell", json!({"page": "MainView", "tell": "saved"})).contains("tells no saved"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_app_s_main_window_and_splash() {
        let (root, _, dir, mut tools) = project("app");
        let wrong = refused(&mut tools, "set_main_window", json!({"window": "MainView"}));
        assert!(wrong.contains("there is no window MainView; the windows are Main"), "{wrong}");
        add_page(&dir, "Window", "Second");
        let said = ok(&mut tools, "set_main_window", json!({"window": "Second"}));
        assert!(said.contains("Set the main window to Second"), "{said}");
        assert!(read(&dir, "design/Todo.cxaml").contains("main=\"Second\""), "{}", read(&dir, "design/Todo.cxaml"));

        let none = refused(&mut tools, "set_splash", json!({"page": "Main"}));
        assert!(none.contains("no splash pages yet"), "{none}");
        add_page(&dir, "Splash", "Welcome");
        ok(&mut tools, "set_splash", json!({"page": "Welcome", "seconds": 2}));
        assert!(read(&dir, "design/Todo.cxaml").contains("Welcome"), "{}", read(&dir, "design/Todo.cxaml"));
        ok(&mut tools, "set_splash", json!({}));
        assert!(!read(&dir, "design/Todo.cxaml").contains("Welcome"), "{}", read(&dir, "design/Todo.cxaml"));

        // One app per name: an unknown owner is refused with the names there are.
        let unknown = refused(&mut tools, "add_region", json!({"window": "Nope", "region": "menu"}));
        assert!(unknown.contains("there is no window, view or app Nope") && unknown.contains("Main") && unknown.contains("app (the app Todo)"), "{unknown}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn regions_and_their_items() {
        let (root, _, dir, mut tools) = project("regions");
        let said = ok(&mut tools, "add_region", json!({"window": "app", "region": "menu"}));
        assert!(said.contains("Added a menu mainMenu to Todo"), "{said}");
        let bad = refused(&mut tools, "add_region", json!({"window": "Main", "region": "statusbar"}));
        assert!(bad.contains("region is one of menu, toolbar, ribbon, sidebar and footer"), "{bad}");

        let said = ok(&mut tools, "add_command", json!({"window": "app", "in": "mainMenu", "kind": "item", "caption": "Open"}));
        let open = said.split_whitespace().nth(3).unwrap().to_string();
        assert!(said.starts_with("Added an item ") && said.contains("to mainMenu in Todo"), "{said}");
        let nowhere = refused(&mut tools, "add_command", json!({"window": "app", "in": "fileMenu", "kind": "item"}));
        assert!(nowhere.contains("there is no menu, toolbar, ribbon, ribbon tab or group, or menu item fileMenu in Todo") && nowhere.contains("mainMenu"), "{nowhere}");

        ok(&mut tools, "change_command", json!({"window": "app", "command": open, "shortcut": "Ctrl+O", "opens": "Main"}));
        let app = read(&dir, "design/Todo.cxaml");
        assert!(app.contains("Ctrl+O") && app.contains("Main"), "{app}");
        let unknown = refused(&mut tools, "change_command", json!({"window": "app", "command": "saveItem", "caption": "Save"}));
        assert!(unknown.contains("there is no menu item or button saveItem in Todo") && unknown.contains(&open), "{unknown}");
        let target = refused(&mut tools, "change_command", json!({"window": "app", "command": open, "opens": "Elsewhere"}));
        assert!(target.contains("Elsewhere is no window") && target.contains("quit"), "{target}");
        ok(&mut tools, "change_command", json!({"window": "app", "command": open, "runs": "openClicked"}));
        assert!(read(&dir, "todo.clarity").contains("handler openClicked"), "{}", read(&dir, "todo.clarity"));
        assert!(read(&dir, "design/Todo.cxaml").contains("openClicked"));

        ok(&mut tools, "add_region", json!({"window": "Main", "region": "sidebar", "side": "right"}));
        ok(&mut tools, "change_sidebar", json!({"window": "Main", "sidebar": "rightSidebar", "width": 320, "resizable": true}));
        let main = read(&dir, "design/Main.cxaml");
        assert!(main.contains("320") && main.contains("rightSidebar"), "{main}");
        let sidebar = refused(&mut tools, "change_sidebar", json!({"window": "Main", "sidebar": "leftSidebar", "width": 300}));
        assert!(sidebar.contains("there is no sidebar leftSidebar in Main; the sidebars of Main are rightSidebar"), "{sidebar}");

        let none = refused(&mut tools, "change_ribbon", json!({"window": "Main", "collapsible": true}));
        assert!(none.contains("Main has no ribbon"), "{none}");
        ok(&mut tools, "add_region", json!({"window": "Main", "region": "ribbon"}));
        let before = read(&dir, "design/Main.cxaml");
        ok(&mut tools, "change_ribbon", json!({"window": "Main", "collapsible": true}));
        assert_ne!(before, read(&dir, "design/Main.cxaml"));
        let said = ok(&mut tools, "add_command", json!({"window": "Main", "in": "homeTab", "kind": "group", "caption": "More"}));
        assert!(said.contains("Added a group moreGroup to homeTab in Main"), "{said}");
        ok(&mut tools, "set_frame_title", json!({"window": "Main", "part": "moreGroup", "title": "Extra"}));
        ok(&mut tools, "rename_frame_part", json!({"window": "Main", "part": "moreGroup", "name": "extraGroup"}));
        assert!(read(&dir, "design/Main.cxaml").contains("extraGroup"));
        ok(&mut tools, "move_frame_part", json!({"window": "Main", "part": "extraGroup", "direction": "up"}));
        ok(&mut tools, "remove_frame_part", json!({"window": "Main", "part": "extraGroup"}));
        assert!(!read(&dir, "design/Main.cxaml").contains("extraGroup"));
        ok(&mut tools, "remove_frame_part", json!({"window": "Main", "part": "rightSidebar"}));
        assert!(!read(&dir, "design/Main.cxaml").contains("rightSidebar"));
        let gone = refused(&mut tools, "remove_frame_part", json!({"window": "Main", "part": "rightSidebar"}));
        assert!(gone.contains("there is no frame part rightSidebar in Main; there are mainRibbon"), "{gone}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn events_handlers_and_their_code() {
        let (root, _, dir, mut tools) = project("events");
        ok(&mut tools, "set_event", json!({"window": "app", "event": "start", "handler": "appStarted"}));
        assert!(read(&dir, "todo.clarity").contains("handler appStarted"), "{}", read(&dir, "todo.clarity"));
        assert!(read(&dir, "design/Todo.cxaml").contains("appStarted"));
        let wrong = refused(&mut tools, "set_event", json!({"window": "Main", "event": "start", "handler": "x"}));
        assert!(wrong.contains("Main has the events open, close and return, not start"), "{wrong}");

        // A button on the view, as the designer places one.
        let project = places::model_project(&dir).unwrap();
        let view = places::frame_owner(&project, &dir, "MainView").unwrap();
        let place = Edit::CreateWidget { container: view.start, kind: clarity_spec::widgets::WidgetKind::Button, col: 1, row: 1, width: None, caption: Some("Save".into()), bind: None, part: None, height: None, input: None, strategy: None, shows: None };
        crate::edits::apply(&dir, &view.file, &place).unwrap();
        let project = places::model_project(&dir).unwrap();
        let view = places::frame_owner(&project, &dir, "MainView").unwrap();
        let button = view.page.unwrap().widgets.iter().find(|w| w.props.kind == clarity_spec::widgets::WidgetKind::Button).and_then(|w| w.props.name.clone()).unwrap();

        ok(&mut tools, "wire_event", json!({"window": "MainView", "part": button, "event": "click", "handler": "saveClicked"}));
        assert!(read(&dir, "todo.clarity").contains("handler saveClicked"), "{}", read(&dir, "todo.clarity"));
        assert!(read(&dir, "design/MainView.cxaml").contains("saveClicked"));
        let part = refused(&mut tools, "wire_event", json!({"window": "MainView", "part": "cancelButton", "event": "click", "handler": "x"}));
        assert!(part.contains("there is no part cancelButton in MainView") && part.contains(&button), "{part}");
        let event = refused(&mut tools, "wire_event", json!({"window": "MainView", "part": button, "event": "tick", "handler": "x"}));
        assert!(event.contains("a button has the events") && event.contains("not tick"), "{event}");

        ok(&mut tools, "set_handler_code", json!({"window": "MainView", "handler": "saveClicked", "code": "let saved = 1\nlet again = saved + 1"}));
        let module = read(&dir, "todo.clarity");
        assert!(module.contains("    handler saveClicked\n        let saved = 1\n        let again = saved + 1\n"), "{module}");
        let unknown = refused(&mut tools, "set_handler_code", json!({"window": "MainView", "handler": "loadAll", "code": "let a = 1"}));
        assert!(unknown.contains("MainView has no handler loadAll; its handlers are saveClicked"), "{unknown}");

        let said = ok(&mut tools, "add_context_menu", json!({"window": "MainView", "part": button}));
        assert!(said.contains("menu with the item firstItem"), "{said}");
        ok(&mut tools, "add_command", json!({"window": "MainView", "in": "firstItem", "kind": "item", "caption": "Deeper"}));
        ok(&mut tools, "change_command", json!({"window": "MainView", "command": "firstItem", "caption": "Actions"}));

        ok(&mut tools, "wire_event", json!({"window": "MainView", "part": button, "event": "click"}));
        assert!(!read(&dir, "design/MainView.cxaml").contains("saveClicked"));
        ok(&mut tools, "name_parts", json!({}));
        ok(&mut tools, "camel_names", json!({"window": "MainView"}));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn an_app_is_added_to_the_module() {
        let (root, _, dir, mut tools) = project("add-app");
        std::fs::remove_file(dir.join("design/Todo.cxaml")).unwrap();
        let none = refused(&mut tools, "set_main_window", json!({"window": "Main"}));
        assert!(none.contains("the project has no app; add one with add_app"), "{none}");
        let module = refused(&mut tools, "add_app", json!({"name": "Todo", "module": "shop"}));
        assert!(module.contains("there is no module shop; the modules are todo"), "{module}");
        ok(&mut tools, "add_app", json!({"name": "Todo"}));
        ok(&mut tools, "set_main_window", json!({"window": "Main"}));
        assert!(read(&dir, "design/Todo.cxaml").contains("main=\"Main\""), "{}", read(&dir, "design/Todo.cxaml"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_handler_s_body_is_replaced_and_the_rest_kept() {
        let code = "handler a\n    one()\n\nhandler b(x: integer)\n    two()\n    three()\n\nhandler c\n    four()\n";
        assert_eq!(with_body(code, "b", "  five()\n    six()").unwrap(), "handler a\n    one()\n\nhandler b(x: integer)\n    five()\n      six()\n\nhandler c\n    four()\n");
        assert_eq!(with_body(code, "c", "seven()").unwrap(), "handler a\n    one()\n\nhandler b(x: integer)\n    two()\n    three()\n\nhandler c\n    seven()\n");
        assert_eq!(with_body("handler a\nhandler b\n    x()\n", "a", "y()").unwrap(), "handler a\n    y()\n\nhandler b\n    x()\n");
        assert!(with_body(code, "d", "x()").is_none());
    }
}
