//! The icons a Clarity application can show, by name: the Lucide set kept
//! in `icons/lucide.json` (icons/README.md). A button, a menu item or a
//! toolbar button shows one with `icon "save"`, and the `icon` part shows
//! one on its own. The renderer gets only the icons a project uses.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::model::{Command, Frame, Project, Widget};

static ICONS: LazyLock<BTreeMap<String, String>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../icons/lucide.json")).expect("icons/lucide.json is a map of names to paths"));

/// Lucide's license, which goes with every application that shows its icons.
pub const LICENSE: &str = include_str!("../icons/LICENSE");

/// How many icons Clarity knows.
pub fn count() -> usize {
    ICONS.len()
}

/// The icon's drawing: one SVG path in a 24 by 24 box, drawn with a 2 pixel round line.
pub fn path(name: &str) -> Option<&'static str> {
    ICONS.get(name).map(String::as_str)
}

/// Every icon's name, in order.
pub fn names() -> impl Iterator<Item = &'static str> {
    ICONS.keys().map(String::as_str)
}

/// The known icon nearest to a name that is not one, when one is near.
pub fn suggestion(name: &str) -> Option<&'static str> {
    let most = (name.chars().count() / 3).max(2);
    names().map(|n| (distance(name, n), n)).filter(|(d, _)| *d <= most).min_by_key(|(d, n)| (*d, n.len())).map(|(_, n)| n)
}

/// What the checker says of a name that is no icon.
pub fn unknown(name: &str) -> String {
    match suggestion(name) {
        Some(near) => format!("unknown icon '{name}'; did you mean '{near}'?"),
        None => format!("unknown icon '{name}'; the icons are Lucide's, by name, such as 'save' or 'trash'"),
    }
}

/// The edits that turn `a` into `b`: a letter added, taken away or changed.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let here = row[j + 1];
            row[j + 1] = (row[j + 1] + 1).min(row[j] + 1).min(previous + usize::from(ca != *cb));
            previous = here;
        }
    }
    row[b.len()]
}

/// The icons the project shows, with their drawings, for the renderer.
pub fn used(project: &Project) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut add = |name: &str| {
        if let Some(path) = path(name) {
            found.insert(name.to_string(), path.to_string());
        }
    };
    fn commands(list: &[Command], add: &mut dyn FnMut(&str)) {
        for c in list {
            if let Some(icon) = &c.props.icon {
                add(icon);
            }
            commands(&c.items, add);
        }
    }
    fn widgets(list: &[Widget], add: &mut dyn FnMut(&str)) {
        for w in list {
            if let Some(icon) = &w.props.icon {
                add(icon);
            }
            if let Some(menu) = &w.menu {
                commands(&menu.items, add);
            }
            widgets(&w.children, add);
            for page in &w.pages {
                widgets(&page.children, add);
            }
        }
    }
    fn frame(f: &Frame, add: &mut dyn FnMut(&str)) {
        if let Some(menu) = &f.menu {
            commands(&menu.items, add);
        }
        if let Some(toolbar) = &f.toolbar {
            commands(&toolbar.items, add);
        }
        if let Some(ribbon) = &f.ribbon {
            for group in ribbon.tabs.iter().flat_map(|t| &t.groups) {
                commands(&group.items, add);
            }
        }
        for sidebar in &f.sidebars {
            widgets(&sidebar.widgets, add);
        }
        if let Some(footer) = &f.footer {
            widgets(&footer.widgets, add);
        }
    }
    for page in project.pages() {
        widgets(&page.widgets, &mut add);
        frame(&page.frame, &mut add);
    }
    for app in project.apps() {
        frame(&app.frame, &mut add);
    }
    found
}
