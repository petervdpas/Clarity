//! The frame catalog: the regions that dock around a window's grid (menu,
//! toolbar, ribbon, sidebar, footer), with the edges they dock to. The
//! parser, the edit engine and the designer's Parts panel read it, like the
//! widget catalog in `widgets`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::lexer::quote;
use crate::model::{CommandKind, CommandProps, Side};
use crate::widgets::is_identifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RegionKind {
    Menu,
    Toolbar,
    Ribbon,
    Sidebar,
    Footer,
}

/// A window edge a region docks to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Dock {
    Top,
    Left,
    Right,
    Bottom,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RegionInfo {
    pub region: RegionKind,
    /// The keyword in spec files.
    pub keyword: String,
    /// The name shown in the Parts panel.
    pub name: String,
    pub docks: Vec<Dock>,
    /// What the region holds, for the Parts panel.
    pub holds: String,
}

struct Entry {
    region: RegionKind,
    keyword: &'static str,
    name: &'static str,
    docks: &'static [Dock],
    holds: &'static str,
}

/// In the order regions are written in a page or app, and stacked on the
/// top edge.
const ENTRIES: &[Entry] = &[
    Entry {
        region: RegionKind::Menu,
        keyword: "menu",
        name: "Menu",
        docks: &[Dock::Top],
        holds: "Menu items, submenus and separators",
    },
    Entry {
        region: RegionKind::Toolbar,
        keyword: "toolbar",
        name: "Toolbar",
        docks: &[Dock::Top],
        holds: "A row of buttons and separators",
    },
    Entry {
        region: RegionKind::Ribbon,
        keyword: "ribbon",
        name: "Ribbon",
        docks: &[Dock::Top],
        holds: "Tabs with groups of large and small buttons",
    },
    Entry {
        region: RegionKind::Sidebar,
        keyword: "sidebar",
        name: "Sidebar",
        docks: &[Dock::Left, Dock::Right],
        holds: "Widgets on its own snap grid",
    },
    Entry {
        region: RegionKind::Footer,
        keyword: "footer",
        name: "Footer",
        docks: &[Dock::Bottom],
        holds: "A status text and widgets on one row",
    },
];

fn entry(region: RegionKind) -> &'static Entry {
    ENTRIES
        .iter()
        .find(|e| e.region == region)
        .expect("every region kind has a catalog entry")
}

/// The full frame catalog, in Parts panel order.
pub fn catalog() -> Vec<RegionInfo> {
    ENTRIES
        .iter()
        .map(|e| RegionInfo {
            region: e.region,
            keyword: e.keyword.to_string(),
            name: e.name.to_string(),
            docks: e.docks.to_vec(),
            holds: e.holds.to_string(),
        })
        .collect()
}

pub fn by_keyword(keyword: &str) -> Option<RegionKind> {
    ENTRIES
        .iter()
        .find(|e| e.keyword == keyword)
        .map(|e| e.region)
}

pub fn keyword(region: RegionKind) -> &'static str {
    entry(region).keyword
}

/// Position of a region in the canonical order.
pub fn order(region: RegionKind) -> usize {
    ENTRIES
        .iter()
        .position(|e| e.region == region)
        .expect("every region kind has a catalog entry")
}

pub fn side_keyword(side: Side) -> &'static str {
    match side {
        Side::Left => "left",
        Side::Right => "right",
    }
}

/// Click targets that are not windows.
pub const BUILT_IN_COMMANDS: &[&str] = &["quit", "close", "language", "theme"];

/// The built-in click targets a button in a window can have too; picking
/// the language or theme is for menus and toolbars.
pub const BUTTON_COMMANDS: &[&str] = &["quit", "close"];

pub fn command_keyword(kind: CommandKind) -> &'static str {
    match kind {
        CommandKind::Item => "item",
        CommandKind::Button => "button",
        CommandKind::Separator => "separator",
    }
}

pub fn command_by_keyword(keyword: &str) -> Option<CommandKind> {
    match keyword {
        "item" => Some(CommandKind::Item),
        "button" => Some(CommandKind::Button),
        "separator" => Some(CommandKind::Separator),
        _ => None,
    }
}

pub fn new_command(kind: CommandKind, caption: Option<String>) -> CommandProps {
    CommandProps {
        kind,
        caption,
        name: None,
        icon: None,
        shortcut: None,
        large: false,
        on_click: None,
        new_window: false,
        handler: None,
    }
}

/// Writes command properties as one canonical spec line.
pub fn command_line(props: &CommandProps) -> String {
    let mut parts = vec![command_keyword(props.kind).to_string()];
    if let Some(caption) = &props.caption {
        parts.push(quote(caption));
    }
    if let Some(name) = &props.name {
        parts.push(format!("name {name}"));
    }
    if let Some(icon) = &props.icon {
        parts.push(format!("icon {}", quote(icon)));
    }
    if let Some(shortcut) = &props.shortcut {
        parts.push(format!("key {}", quote(shortcut)));
    }
    if props.large {
        parts.push("large".to_string());
    }
    if let Some(handler) = &props.handler {
        parts.push(format!("on click: {handler}"));
    }
    if let Some(target) = &props.on_click {
        parts.push(format!("on click -> {target}"));
        if props.new_window {
            parts.push("in new window".to_string());
        }
    }
    parts.join(" ")
}

/// Checks command properties, from the parser or from the designer.
pub fn validate_command(props: &CommandProps) -> Result<(), String> {
    let name = command_keyword(props.kind);
    match props.kind {
        CommandKind::Separator => {
            let extra = props.caption.is_some()
                || props.handler.is_some()
                || props.icon.is_some()
                || props.shortcut.is_some()
                || props.large
                || props.on_click.is_some();
            if extra {
                return Err("a separator has no caption, icon or click target".to_string());
            }
        }
        CommandKind::Item => {
            if props.caption.is_none() {
                return Err("a menu item needs a caption, as in 'item \"Open\"'".to_string());
            }
            if props.large {
                return Err("'large' applies to ribbon buttons".to_string());
            }
            if let Some(shortcut) = &props.shortcut {
                crate::widgets::check_shortcut(shortcut)?;
            }
        }
        CommandKind::Button => {
            if props.caption.is_none() && props.icon.is_none() {
                return Err("a button needs a caption or an icon".to_string());
            }
            if props.shortcut.is_some() {
                return Err("'key' applies to menu items".to_string());
            }
        }
    }
    if let Some(caption) = &props.caption
        && caption.trim().is_empty()
    {
        return Err(format!("a {name} caption cannot be empty"));
    }
    if let Some(target) = &props.on_click
        && !is_identifier(target)
    {
        return Err(format!("'{target}' is not a valid click target"));
    }
    if props.on_click.is_some() && props.handler.is_some() {
        return Err("'click' is wired twice".to_string());
    }
    for (what, value) in [("part name", &props.name), ("handler name", &props.handler)] {
        if let Some(value) = value
            && !is_identifier(value)
        {
            return Err(format!("'{value}' is not a valid {what}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_region_has_one_entry_and_a_unique_keyword() {
        for e in ENTRIES {
            assert_eq!(by_keyword(e.keyword), Some(e.region));
        }
        assert_eq!(catalog().len(), 5);
    }

    #[test]
    fn writes_commands_in_canonical_order() {
        let mut props = new_command(CommandKind::Button, Some("New".to_string()));
        props.on_click = Some("CustomerCard".to_string());
        props.large = true;
        props.icon = Some("add".to_string());
        assert_eq!(
            command_line(&props),
            "button \"New\" icon \"add\" large on click -> CustomerCard"
        );
        assert_eq!(
            command_line(&new_command(CommandKind::Separator, None)),
            "separator"
        );
    }

    #[test]
    fn rejects_misplaced_properties() {
        let mut item = new_command(CommandKind::Item, Some("Open".to_string()));
        item.large = true;
        assert!(validate_command(&item).is_err());
        let separator = new_command(CommandKind::Separator, Some("x".to_string()));
        assert!(validate_command(&separator).is_err());
        assert!(validate_command(&new_command(CommandKind::Item, None)).is_err());
    }
}

/// The regions a window shows: its own, and the app's where it has none of
/// that kind (for sidebars, of that side). Dialogs show no frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShownFrame<'a> {
    pub menu: Option<&'a crate::model::Menu>,
    pub toolbar: Option<&'a crate::model::Toolbar>,
    pub ribbon: Option<&'a crate::model::Ribbon>,
    pub left: Option<&'a crate::model::Sidebar>,
    pub right: Option<&'a crate::model::Sidebar>,
    pub footer: Option<&'a crate::model::Footer>,
}

/// The frame `page` shows in `app`. The web renderer has the same rule
/// (`shownFrame` in app/src/lib/frame.ts).
pub fn shown<'a>(app: Option<&'a crate::model::App>, page: &'a crate::model::Page) -> ShownFrame<'a> {
    if page.kind == crate::model::PageKind::Dialog {
        return ShownFrame::default();
    }
    let own = &page.frame;
    let theirs = app.map(|a| &a.frame);
    let side = |frame: Option<&'a crate::model::Frame>, side: Side| frame.and_then(|f| f.sidebars.iter().find(|s| s.side == side));
    ShownFrame {
        menu: own.menu.as_ref().or(theirs.and_then(|f| f.menu.as_ref())),
        toolbar: own.toolbar.as_ref().or(theirs.and_then(|f| f.toolbar.as_ref())),
        ribbon: own.ribbon.as_ref().or(theirs.and_then(|f| f.ribbon.as_ref())),
        left: side(Some(own), Side::Left).or(side(theirs, Side::Left)),
        right: side(Some(own), Side::Right).or(side(theirs, Side::Right)),
        footer: own.footer.as_ref().or(theirs.and_then(|f| f.footer.as_ref())),
    }
}

/// Whether any click target of the project opens `in new window`: only then
/// does an application make a second desktop window.
pub fn opens_new_windows(project: &crate::model::Project) -> bool {
    fn widgets(list: &[crate::model::Widget]) -> bool {
        list.iter().any(|w| w.props.new_window || widgets(&w.children) || w.pages.iter().any(|p| widgets(&p.children)))
    }
    fn commands(list: &[crate::model::Command]) -> bool {
        list.iter().any(|c| c.props.new_window || commands(&c.items))
    }
    let frame = |f: &crate::model::Frame| {
        f.menu.as_ref().is_some_and(|m| commands(&m.items))
            || f.toolbar.as_ref().is_some_and(|t| commands(&t.items))
            || f.ribbon.as_ref().is_some_and(|r| r.tabs.iter().flat_map(|t| &t.groups).any(|g| commands(&g.items)))
            || f.sidebars.iter().any(|s| widgets(&s.widgets))
            || f.footer.as_ref().is_some_and(|ft| widgets(&ft.widgets))
    };
    project.pages().any(|p| widgets(&p.widgets) || frame(&p.frame)) || project.apps().any(|a| frame(&a.frame))
}
