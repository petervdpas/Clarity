//! The commands a person can give keys in Clarity's keys window (DESIGN
//! 5.28), when the project file says `keys editable`: the app's frame once,
//! under Everywhere, then each window's own frame commands and named buttons,
//! with the keys the design gives them. A command is known by its name, a
//! window's own by `Window.name`, as the person's keys are kept.

use serde::Serialize;
use ts_rs::TS;

use crate::model::{Command, CommandKind, Frame, Project, Widget};
use crate::widgets::{Event, WidgetKind};

/// The commands active in one place: the app's frame (`id` empty, no
/// title) or a window of its own (`id` its name, `title` its title as written).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KeyLayer {
    pub id: String,
    pub title: Option<String>,
    pub commands: Vec<KeyCommand>,
}

/// A command a key can run: its id as the person's keys keep it, its part
/// name, its caption as written, and the keys the design gives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KeyCommand {
    pub id: String,
    pub name: String,
    pub caption: Option<String>,
    pub keys: Vec<String>,
}

/// Every place with commands a key can run, the app's frame first, then the
/// windows in the order they are written; a place without one is left out.
pub fn catalog(project: &Project) -> Vec<KeyLayer> {
    let mut layers = Vec::new();
    let mut app = Vec::new();
    for a in project.files.iter().flat_map(|f| &f.apps) {
        frame_commands(&a.frame, "", &mut app);
    }
    if !app.is_empty() {
        layers.push(KeyLayer { id: String::new(), title: None, commands: app });
    }
    for page in project.pages() {
        let prefix = format!("{}.", page.name.name);
        let mut commands = Vec::new();
        frame_commands(&page.frame, &prefix, &mut commands);
        buttons(&page.widgets, &prefix, &mut commands);
        for sidebar in &page.frame.sidebars {
            buttons(&sidebar.widgets, &prefix, &mut commands);
        }
        if let Some(footer) = &page.frame.footer {
            buttons(&footer.widgets, &prefix, &mut commands);
        }
        if !commands.is_empty() {
            layers.push(KeyLayer { id: page.name.name.clone(), title: page.title.clone(), commands });
        }
    }
    layers
}

/// The menu items and toolbar and ribbon buttons that do something; an item
/// with a submenu only opens it.
fn frame_commands(frame: &Frame, prefix: &str, out: &mut Vec<KeyCommand>) {
    fn walk(items: &[Command], prefix: &str, out: &mut Vec<KeyCommand>) {
        for c in items {
            let p = &c.props;
            if p.kind != CommandKind::Separator
                && (p.on_click.is_some() || p.handler.is_some())
                && let Some(name) = &p.name
            {
                out.push(KeyCommand { id: format!("{prefix}{name}"), name: name.clone(), caption: p.caption.clone(), keys: p.shortcut.iter().cloned().collect() });
            }
            walk(&c.items, prefix, out);
        }
    }
    if let Some(menu) = &frame.menu {
        walk(&menu.items, prefix, out);
    }
    if let Some(toolbar) = &frame.toolbar {
        walk(&toolbar.items, prefix, out);
    }
    if let Some(ribbon) = &frame.ribbon {
        for group in ribbon.tabs.iter().flat_map(|t| &t.groups) {
            walk(&group.items, prefix, out);
        }
    }
}

/// The named buttons of a window that do something, also inside containers.
fn buttons(widgets: &[Widget], prefix: &str, out: &mut Vec<KeyCommand>) {
    for w in widgets {
        let p = &w.props;
        let does = p.on_click.is_some() || p.handlers.iter().any(|h| h.event == Event::Click);
        if p.kind == WidgetKind::Button
            && does
            && let Some(name) = &p.name
        {
            out.push(KeyCommand { id: format!("{prefix}{name}"), name: name.clone(), caption: p.caption.clone(), keys: p.shortcut.iter().cloned().collect() });
        }
        buttons(&w.children, prefix, out);
        for page in &w.pages {
            buttons(&page.children, prefix, out);
        }
    }
}
