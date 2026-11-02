//! The rules of a frame every screen keeps alike: the tab a ribbon shows
//! (conformance/ribbons.json), the width a sidebar is dragged to and whether
//! it is folded or fills the window (conformance/sidebars.json), and what of
//! a frame's commands shows, takes clicks and answers keys, as their part
//! state says (conformance/overlays.json `item`, DESIGN 5.28).

use std::rc::Rc;

use crate::markup::CommandSpec;
use crate::parts::PartHost;

use super::spec::{ShownFrame, SidebarSpec};

/// The narrowest and widest a sidebar may be (clarity_spec::model::SIDEBAR_WIDTHS).
pub const NARROWEST: f64 = 100.0;
pub const WIDEST: f64 = 1600.0;

/// The width of a folded sidebar: room for its button.
pub const STRIP_WIDTH: f64 = 28.0;

/// The tab a ribbon shows (from 1) of tabs that are visible or not, when
/// `asked` is wanted: it, when it is there and visible, else the first
/// visible one; none when no tab is visible.
pub fn shown_tab(visible: &[bool], asked: i64) -> Option<usize> {
    if asked >= 1 && (asked as usize) <= visible.len() && visible[asked as usize - 1] {
        return Some(asked as usize);
    }
    visible.iter().position(|v| *v).map(|i| i + 1)
}

/// The width a drag to `dragged` pixels gives: within the limits, in whole pixels.
pub fn sidebar_width(dragged: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    dragged.max(min.unwrap_or(NARROWEST)).min(max.unwrap_or(WIDEST)).round()
}

/// Whether a sidebar is folded and fills the window now: as its part state
/// says (a handler or the person set it), else as designed; only what it allows.
pub fn sidebar_now(sidebar: &SidebarSpec, collapsed: Option<bool>, maximized: Option<bool>) -> (bool, bool) {
    (sidebar.collapsible && collapsed.unwrap_or(sidebar.collapsed), sidebar.maximizable && maximized.unwrap_or(false))
}

/// Whether a menu item is a line between items.
pub fn is_separator(item: &CommandSpec) -> bool {
    item.text("kind") == Some("separator")
}

/// Whether a named region or command shows: while its state is not hidden.
pub fn shows(host: &dyn PartHost, name: Option<&str>) -> bool {
    name.is_none() || host.state(name).and_then(|s| s.visible) != Some(false)
}

/// Whether a named region or command takes clicks: while its state is not disabled.
pub fn takes(host: &dyn PartHost, name: Option<&str>) -> bool {
    name.is_none() || host.state(name).and_then(|s| s.enabled) != Some(false)
}

/// Whether a named menu item shows a check mark: its state is checked.
pub fn checked(host: &dyn PartHost, name: Option<&str>) -> bool {
    name.is_some() && host.state(name).and_then(|s| s.checked) == Some(true)
}

/// The built-in click target a command picks in place, `language` or `theme`; none for any other.
pub fn picker_of(item: &CommandSpec) -> Option<&'static str> {
    if item.text("handler").is_some_and(|h| !h.is_empty()) {
        return None;
    }
    match item.text("onClick") {
        Some("language") => Some("language"),
        Some("theme") => Some("theme"),
        _ => None,
    }
}

/// The keys a command answers: the person's where they set their own (a
/// command of the app's frame by its name, a window's own by `Window.name`
/// of `window`), else its designed one.
pub fn keys_of(host: &dyn PartHost, window: Option<&str>, name: Option<&str>, app: bool, designed: Option<&str>) -> Vec<String> {
    let designed = designed.filter(|d| !d.is_empty());
    let id = match (name, app, window) {
        (Some(name), true, _) => Some(name.to_string()),
        // A window's own command is known by `Window.name`: both are the project's own names.
        (Some(name), false, Some(window)) => Some(format!("{window}.{name}")),
        _ => None,
    };
    match id {
        Some(id) => host.keys_of(&id, designed),
        None => designed.map(|d| vec![d.to_string()]).unwrap_or_default(),
    }
}

/// What a toolbar's or ribbon's button says when the pointer rests on it: its caption and its keys.
pub fn tip_of(caption: &str, keys: &[String]) -> Option<String> {
    match (caption.is_empty(), keys.is_empty()) {
        (true, true) => None,
        (false, true) => Some(caption.to_string()),
        (true, false) => Some(keys.join(", ")),
        (false, false) => Some(format!("{caption} ({})", keys.join(", "))),
    }
}

/// The frame's commands a key may run, in the order they are written: the
/// menu, the toolbar, the ribbon (whichever tab is open), while they and
/// what holds them show and take clicks; each with a key of its design or
/// a name the person's keys know it by, and whether it is of the app's
/// frame (its region is inherited).
pub fn keyed(frame: &ShownFrame, host: &dyn PartHost) -> Vec<(CommandSpec, bool)> {
    fn walk(items: &[CommandSpec], app: bool, host: &dyn PartHost, out: &mut Vec<(CommandSpec, bool)>) {
        for item in items {
            let name = item.text("name");
            if is_separator(item) || !shows(host, name) || !takes(host, name) {
                continue;
            }
            if item.text("shortcut").is_some_and(|s| !s.is_empty()) || (name.is_some() && item.items.is_empty()) {
                out.push((item.clone(), app));
            }
            walk(&item.items, app, host, out);
        }
    }
    let usable = |name: Option<&str>| shows(host, name) && takes(host, name);
    let mut out = Vec::new();
    if let Some(menu) = frame.menu.as_ref().filter(|m| usable(m.value.name.as_deref())) {
        walk(&menu.value.items, menu.inherited, host, &mut out);
    }
    if let Some(toolbar) = frame.toolbar.as_ref().filter(|t| usable(t.value.name.as_deref())) {
        walk(&toolbar.value.items, toolbar.inherited, host, &mut out);
    }
    if let Some(ribbon) = frame.ribbon.as_ref().filter(|r| usable(r.value.name.as_deref())) {
        for tab in ribbon.value.tabs.iter().filter(|t| shows(host, t.name.as_deref())) {
            for group in tab.groups.iter().filter(|g| shows(host, g.name.as_deref())) {
                walk(&group.items, ribbon.inherited, host, &mut out);
            }
        }
    }
    out
}

/// A choice of a built-in picker: its value, its label and whether it is the one shown.
#[derive(Debug, Clone, PartialEq)]
pub struct PickerChoice {
    pub value: String,
    pub label: String,
    pub shown: bool,
}

/// The choices of a built-in picker, `language` or `theme`, and what
/// choosing one does. Set in place, no window opens for them.
#[derive(Clone)]
pub struct FramePicker {
    pub choices: Rc<dyn Fn() -> Vec<PickerChoice>>,
    pub choose: Rc<dyn Fn(&str)>,
}

impl FramePicker {
    pub fn new(choices: impl Fn() -> Vec<PickerChoice> + 'static, choose: impl Fn(&str) + 'static) -> FramePicker {
        FramePicker { choices: Rc::new(choices), choose: Rc::new(choose) }
    }

    /// The languages the program offers, each by its own name; choosing one
    /// shows the texts in it (the windows are made again in it).
    pub fn language() -> FramePicker {
        FramePicker::new(
            || crate::choices::languages().into_iter().map(|o| PickerChoice { value: o.name, label: o.label, shown: o.current }).collect(),
            crate::choices::choose_language,
        )
    }

    /// The themes the program offers, each labelled in the language shown;
    /// choosing one shows it (the screen follows it by itself).
    pub fn theme() -> FramePicker {
        FramePicker::new(
            || crate::choices::themes().into_iter().map(|o| PickerChoice { value: o.name, label: o.label, shown: o.current }).collect(),
            crate::choices::choose_theme,
        )
    }
}
