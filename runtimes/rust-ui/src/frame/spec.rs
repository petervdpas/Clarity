//! The frame of a window as its markup writes it (DESIGN 7.8): the menu
//! bar, toolbar, ribbon, sidebars and footer of the window's own `.cxaml`
//! and of its app's (`apps/Shop.cxaml`). A window shows its own region where
//! it has one and the app's where it has none, as every screen does.

use serde_json::Value;

use crate::markup::{self, CommandSpec, MenuSpec, PartSpec};
use crate::xml::XmlElement;

/// A toolbar: its name, by which code hides and disables it, and its buttons.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolbarSpec {
    pub name: Option<String>,
    pub items: Vec<CommandSpec>,
}

/// A group of a ribbon's tab: its name, its title under it and its buttons.
#[derive(Debug, Clone, PartialEq)]
pub struct RibbonGroupSpec {
    pub name: Option<String>,
    pub title: String,
    pub items: Vec<CommandSpec>,
}

/// A tab of a ribbon: its name, its title and its groups.
#[derive(Debug, Clone, PartialEq)]
pub struct RibbonTabSpec {
    pub name: Option<String>,
    pub title: String,
    pub groups: Vec<RibbonGroupSpec>,
}

/// A ribbon: its name, whether the person may fold it and whether it starts folded, and its tabs.
#[derive(Debug, Clone, PartialEq)]
pub struct RibbonSpec {
    pub name: Option<String>,
    pub collapsible: bool,
    pub collapsed: bool,
    pub tabs: Vec<RibbonTabSpec>,
}

/// A sidebar on the left or right of the window: its width and the limits
/// a drag keeps it in, what the person may do with it (resize, fold, let it
/// fill the window), whether it starts folded, and its parts on a grid of its columns.
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarSpec {
    pub side: String,
    pub name: Option<String>,
    pub width: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub resizable: bool,
    pub collapsible: bool,
    pub collapsed: bool,
    pub maximizable: bool,
    pub columns: i64,
    pub parts: Vec<PartSpec>,
    /// What it is in the design (`designItem`): the designer finds its grid by it.
    pub item: Option<String>,
}

impl SidebarSpec {
    /// Whether it is docked to the left edge; any side but `right` is.
    pub fn left(&self) -> bool {
        self.side != "right"
    }
}

/// The columns of a footer's grid that says none (clarity_spec::model::FOOTER_COLUMNS).
pub const FOOTER_COLUMNS: i64 = 12;

/// The footer: its status text at the left and its parts on one row of its columns.
#[derive(Debug, Clone, PartialEq)]
pub struct FooterSpec {
    pub status: Option<String>,
    pub columns: i64,
    pub parts: Vec<PartSpec>,
    /// What it is in the design (`designItem`): the designer finds its grid by it.
    pub item: Option<String>,
}

/// The regions of one frame, a window's own or its app's; each is none where the frame has none.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FrameSpec {
    pub menu: Option<MenuSpec>,
    pub toolbar: Option<ToolbarSpec>,
    pub ribbon: Option<RibbonSpec>,
    pub left: Option<SidebarSpec>,
    pub right: Option<SidebarSpec>,
    pub footer: Option<FooterSpec>,
}

impl FrameSpec {
    /// A frame of the regions its markup writes: `Menu`, `Toolbar`,
    /// `Ribbon`, `Sidebar` and `Footer`; other elements are no region, and a
    /// part the catalog does not have in a sidebar or footer is left out.
    pub fn read(regions: &[XmlElement]) -> FrameSpec {
        let mut frame = FrameSpec::default();
        for region in regions {
            match region.name.as_str() {
                "Menu" => frame.menu = Some(markup::menu_of(region)),
                "Toolbar" => frame.toolbar = Some(ToolbarSpec { name: region.attribute("name").map(str::to_string), items: region.children.iter().map(markup::command_of).collect() }),
                "Ribbon" => frame.ribbon = Some(ribbon_of(region)),
                "Sidebar" => {
                    let sidebar = sidebar_of(region);
                    if sidebar.left() {
                        frame.left = Some(sidebar);
                    } else {
                        frame.right = Some(sidebar);
                    }
                }
                "Footer" => {
                    let columns = region.attribute("columns").and_then(|c| c.trim().parse::<f64>().ok()).filter(|c| *c >= 1.0).map(|c| c as i64).unwrap_or(FOOTER_COLUMNS);
                    frame.footer = Some(FooterSpec { status: region.attribute("status").map(str::to_string), columns, parts: parts_of(region), item: region.attribute(markup::DESIGN_ITEM).map(str::to_string) });
                }
                _ => {}
            }
        }
        frame
    }
}

fn parts_of(region: &XmlElement) -> Vec<PartSpec> {
    region.children.iter().filter_map(|c| markup::part_of(c).ok()).collect()
}

fn ribbon_of(ribbon: &XmlElement) -> RibbonSpec {
    let flag = |name: &str| ribbon.attribute(name) == Some("true");
    let named = |e: &XmlElement| e.attribute("name").map(str::to_string);
    let titled = |e: &XmlElement| e.attribute("title").unwrap_or("").to_string();
    RibbonSpec {
        name: named(ribbon),
        collapsible: flag("collapsible"),
        collapsed: flag("collapsed"),
        tabs: ribbon
            .children
            .iter()
            .filter(|t| t.name == "Tab")
            .map(|tab| RibbonTabSpec {
                name: named(tab),
                title: titled(tab),
                groups: tab
                    .children
                    .iter()
                    .filter(|g| g.name == "Group")
                    .map(|group| RibbonGroupSpec { name: named(group), title: titled(group), items: group.children.iter().map(markup::command_of).collect() })
                    .collect(),
            })
            .collect(),
    }
}

/// A sidebar: what its element writes over the catalog's fields for its side (sidebar-defaults.json).
fn sidebar_of(sidebar: &XmlElement) -> SidebarSpec {
    let props = markup::sidebar_of(sidebar);
    let number = |name: &str| props.get(name).and_then(Value::as_f64);
    let flag = |name: &str| props.get(name).and_then(Value::as_bool).unwrap_or(false);
    SidebarSpec {
        side: props.get("side").and_then(Value::as_str).unwrap_or("left").to_string(),
        name: props.get("name").and_then(Value::as_str).map(str::to_string),
        width: number("width").unwrap_or(0.0),
        min: number("min"),
        max: number("max"),
        resizable: flag("resizable"),
        collapsible: flag("collapsible"),
        collapsed: flag("collapsed"),
        maximizable: flag("maximizable"),
        columns: (number("columns").unwrap_or(1.0) as i64).max(1),
        parts: parts_of(sidebar),
        item: sidebar.attribute(markup::DESIGN_ITEM).map(str::to_string),
    }
}

/// A region as a window shows it: its own, or the app's (`inherited`).
#[derive(Debug, Clone, PartialEq)]
pub struct Shown<T> {
    pub value: T,
    pub inherited: bool,
}

/// The regions a window shows: each of its own replaces the app's.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShownFrame {
    pub menu: Option<Shown<MenuSpec>>,
    pub toolbar: Option<Shown<ToolbarSpec>>,
    pub ribbon: Option<Shown<RibbonSpec>>,
    pub left: Option<Shown<SidebarSpec>>,
    pub right: Option<Shown<SidebarSpec>>,
    pub footer: Option<Shown<FooterSpec>>,
}

fn pick<T: Clone>(own: &Option<T>, app: Option<&Option<T>>) -> Option<Shown<T>> {
    match (own, app.and_then(Option::as_ref)) {
        (Some(own), _) => Some(Shown { value: own.clone(), inherited: false }),
        (None, Some(app)) => Some(Shown { value: app.clone(), inherited: true }),
        (None, None) => None,
    }
}

impl ShownFrame {
    /// The frame of a window with its own regions `own` in the app whose frame is `app` (none for a dialog, or a window without an app).
    pub fn of(app: Option<&FrameSpec>, own: &FrameSpec) -> ShownFrame {
        ShownFrame {
            menu: pick(&own.menu, app.map(|a| &a.menu)),
            toolbar: pick(&own.toolbar, app.map(|a| &a.toolbar)),
            ribbon: pick(&own.ribbon, app.map(|a| &a.ribbon)),
            left: pick(&own.left, app.map(|a| &a.left)),
            right: pick(&own.right, app.map(|a| &a.right)),
            footer: pick(&own.footer, app.map(|a| &a.footer)),
        }
    }

    /// Whether it shows no region at all.
    pub fn is_empty(&self) -> bool {
        self.menu.is_none() && self.toolbar.is_none() && self.ribbon.is_none() && self.left.is_none() && self.right.is_none() && self.footer.is_none()
    }
}
