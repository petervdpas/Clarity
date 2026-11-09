//! What the designer calls each thing it shows: its file and name,
//! `design/Main.cxaml#Main/saveButton`, instead of where it is written. The
//! designer's layer, its picks and its edits go by these keys, so a key stays
//! when the text around it changes and says which file an edit changes.
//!
//! A key is the file, `#`, then a path that starts with the window's, view's
//! or app's name. A thing with a name (a part, menu, item, toolbar, ribbon
//! tab or group, sidebar) is the name under it: `Main/saveButton`, wherever
//! it stands, as names are unique in a window. A thing without one is its
//! element and number among the same elements under what holds it:
//! `Main/mainTabs/Tab[2]`. Data and tells go by their names in brackets,
//! `Main/Data[orders]`, as a part may have the same name. A footer's status
//! is `Main/Footer[1]/Status`.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::model::{App, Command, Frame, Page, Project, TreeLevel, Widget};
use crate::span::Span;

/// Each thing of `page`, written in the file `file`, with its key, by where it starts.
pub fn page_keys(file: &str, page: &Page) -> Vec<(u32, String)> {
    let mut walk = Walk::new(file, &page.name.name);
    let root = walk.root.clone();
    walk.out.push((page.span.start, walk.full(&root)));
    for data in page.data.iter().filter(|d| !d.made) {
        walk.bracketed(&root, "Data", &data.name.name, data.span.start);
    }
    for tells in &page.tells {
        walk.bracketed(&root, "Tells", &tells.name.name, tells.span.start);
    }
    walk.frame(&page.frame);
    walk.parts(&root, &page.widgets);
    walk.out
}

/// Each thing of `app`, written in the file `file`, with its key, by where it starts.
pub fn app_keys(file: &str, app: &App) -> Vec<(u32, String)> {
    let mut walk = Walk::new(file, &app.name.name);
    let root = walk.root.clone();
    walk.out.push((app.span.start, walk.full(&root)));
    for data in app.data.iter().filter(|d| !d.made) {
        walk.bracketed(&root, "Data", &data.name.name, data.span.start);
    }
    walk.frame(&app.frame);
    walk.out
}

/// The key of each thing of every window, view and app of the project, by
/// its file's path and where it starts in that file.
/// A key names the file by its path in the project's folder (`design/Main.cxaml`).
pub fn project_keys(project: &Project) -> BTreeMap<String, BTreeMap<u32, String>> {
    let folder = project_folder(project);
    let mut out: BTreeMap<String, BTreeMap<u32, String>> = BTreeMap::new();
    for file in &project.files {
        let name = in_folder(&file.path, folder.as_deref());
        let keys = out.entry(file.path.clone()).or_default();
        for page in &file.pages {
            keys.extend(page_keys(&name, page));
        }
        for app in &file.apps {
            keys.extend(app_keys(&name, app));
        }
    }
    out
}

/// The project's folder: the one its modules are in, above `design/`.
fn project_folder(project: &Project) -> Option<std::path::PathBuf> {
    project.files.iter().find_map(|file| {
        let path = std::path::Path::new(&file.path);
        let folder = if crate::cxaml::is_page_file(&file.path) { crate::cxaml::project_dir_of(path) } else { path.parent() };
        folder.map(std::path::Path::to_path_buf)
    })
}

/// `path` from the project's folder, with `/` between folders on every system.
fn in_folder(path: &str, folder: Option<&std::path::Path>) -> String {
    let path = std::path::Path::new(path);
    let inner = folder.and_then(|f| path.strip_prefix(f).ok()).unwrap_or(path);
    inner.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

/// A key's file and path: `design/Main.cxaml#Main/saveButton` is
/// `design/Main.cxaml` and `Main/saveButton`.
pub fn split(key: &str) -> Option<(&str, &str)> {
    key.split_once('#')
}

/// Where the thing `key` names starts in `page`, written in `file`; none
/// when the key is of another file or names nothing of the page.
pub fn place_of(file: &str, page: &Page, key: &str) -> Option<u32> {
    if split(key)?.0 != file {
        return None;
    }
    page_keys(file, page).into_iter().find(|(_, k)| k == key).map(|(at, _)| at)
}

struct Walk<'a> {
    file: &'a str,
    root: String,
    out: Vec<(u32, String)>,
    /// The names given out, so a second thing of the same name goes by its place.
    taken: HashSet<String>,
}

impl<'a> Walk<'a> {
    fn new(file: &'a str, root: &str) -> Self {
        Walk { file, root: root.to_string(), out: Vec::new(), taken: HashSet::new() }
    }

    fn full(&self, path: &str) -> String {
        format!("{}#{path}", self.file)
    }

    /// The path of a thing under `parent`: its name where it has one not
    /// given out yet, else its element and number. Gives the path.
    fn add(&mut self, parent: &str, element: &str, number: usize, name: Option<&str>, at: u32) -> String {
        let path = match name.filter(|n| !n.is_empty()) {
            Some(name) if self.taken.insert(name.to_string()) => format!("{}/{name}", self.root),
            _ => format!("{parent}/{element}[{number}]"),
        };
        self.out.push((at, self.full(&path)));
        path
    }

    fn bracketed(&mut self, parent: &str, element: &str, name: &str, at: u32) {
        let path = format!("{parent}/{element}[{name}]");
        self.out.push((at, self.full(&path)));
    }

    fn frame(&mut self, frame: &Frame) {
        let root = self.root.clone();
        if let Some(menu) = &frame.menu {
            let path = self.add(&root, "Menu", 1, menu.name.as_deref(), menu.span.start);
            self.commands(&path, &menu.items);
        }
        if let Some(toolbar) = &frame.toolbar {
            let path = self.add(&root, "Toolbar", 1, toolbar.name.as_deref(), toolbar.span.start);
            self.commands(&path, &toolbar.items);
        }
        if let Some(ribbon) = &frame.ribbon {
            let path = self.add(&root, "Ribbon", 1, ribbon.name.as_deref(), ribbon.span.start);
            for (i, tab) in ribbon.tabs.iter().enumerate() {
                let tab_path = self.add(&path, "Tab", i + 1, tab.name.as_deref(), tab.span.start);
                for (j, group) in tab.groups.iter().enumerate() {
                    let group_path = self.add(&tab_path, "Group", j + 1, group.name.as_deref(), group.span.start);
                    self.commands(&group_path, &group.items);
                }
            }
        }
        for (i, sidebar) in frame.sidebars.iter().enumerate() {
            let path = self.add(&root, "Sidebar", i + 1, sidebar.props.name.as_deref(), sidebar.span.start);
            self.parts(&path, &sidebar.widgets);
        }
        if let Some(footer) = &frame.footer {
            let path = self.add(&root, "Footer", 1, None, footer.span.start);
            if let Some(status) = footer.status_span {
                self.out.push((status.start, self.full(&format!("{path}/Status"))));
            }
            self.parts(&path, &footer.widgets);
        }
    }

    fn commands(&mut self, parent: &str, items: &[Command]) {
        for (i, command) in items.iter().enumerate() {
            let path = self.add(parent, "Item", i + 1, command.props.name.as_deref(), command.span.start);
            self.commands(&path, &command.items);
        }
    }

    fn levels(&mut self, parent: &str, levels: &[TreeLevel]) {
        for (i, level) in levels.iter().enumerate() {
            let path = self.add(parent, "Level", i + 1, None, level.span.start);
            self.levels(&path, &level.levels);
        }
    }

    fn parts(&mut self, parent: &str, widgets: &[Widget]) {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for widget in widgets {
            let element = crate::cxaml::element_name(widget.props.kind);
            let number = counts.entry(element.clone()).and_modify(|n| *n += 1).or_insert(1);
            let path = self.add(parent, &element, *number, widget.props.name.as_deref(), widget.span.start);
            if let Some(menu) = &widget.menu {
                let menu_path = self.add(&path, "Menu", 1, menu.name.as_deref(), menu.span.start);
                self.commands(&menu_path, &menu.items);
            }
            self.levels(&path, &widget.levels);
            for (i, select) in widget.selects.iter().enumerate() {
                self.add(&path, "Select", i + 1, None, select.span.start);
            }
            self.parts(&path, &widget.children);
            let page_element = crate::cxaml::page_element_name(widget.props.kind);
            for (i, page) in widget.pages.iter().enumerate() {
                let page_path = self.add(&path, page_element, i + 1, None, page.span.start);
                self.parts(&page_path, &page.children);
            }
        }
    }
}

/// The project as the designer draws it: each thing with a key gets a
/// number of its own in place of where it starts (the places of two files
/// overlap, and a window shows the parts of views from other files), and
/// the key of each number. Only the designer's markup reads these numbers.
pub fn numbered(project: &Project) -> (Project, BTreeMap<u32, String>) {
    let keys = project_keys(project);
    let mut out = project.clone();
    let mut by_number = BTreeMap::new();
    let mut next = 1u32;
    for file in &mut out.files {
        let Some(file_keys) = keys.get(&file.path) else { continue };
        let mut number = |span: &mut Span| {
            if let Some(key) = file_keys.get(&span.start) {
                by_number.insert(next, key.clone());
                *span = Span { start: next, end: next, ..*span };
                next += 1;
            }
        };
        for page in &mut file.pages {
            number(&mut page.span);
            page.data.iter_mut().filter(|d| !d.made).for_each(|d| number(&mut d.span));
            page.tells.iter_mut().for_each(|t| number(&mut t.span));
            frame_spans(&mut page.frame, &mut number);
            parts_spans(&mut page.widgets, &mut number);
        }
        for app in &mut file.apps {
            number(&mut app.span);
            app.data.iter_mut().filter(|d| !d.made).for_each(|d| number(&mut d.span));
            frame_spans(&mut app.frame, &mut number);
        }
    }
    (out, by_number)
}

fn frame_spans(frame: &mut Frame, f: &mut dyn FnMut(&mut Span)) {
    if let Some(menu) = &mut frame.menu {
        f(&mut menu.span);
        commands_spans(&mut menu.items, f);
    }
    if let Some(toolbar) = &mut frame.toolbar {
        f(&mut toolbar.span);
        commands_spans(&mut toolbar.items, f);
    }
    if let Some(ribbon) = &mut frame.ribbon {
        f(&mut ribbon.span);
        for tab in &mut ribbon.tabs {
            f(&mut tab.span);
            for group in &mut tab.groups {
                f(&mut group.span);
                commands_spans(&mut group.items, f);
            }
        }
    }
    for sidebar in &mut frame.sidebars {
        f(&mut sidebar.span);
        parts_spans(&mut sidebar.widgets, f);
    }
    if let Some(footer) = &mut frame.footer {
        f(&mut footer.span);
        if let Some(status) = &mut footer.status_span {
            f(status);
        }
        parts_spans(&mut footer.widgets, f);
    }
}

fn commands_spans(items: &mut [Command], f: &mut dyn FnMut(&mut Span)) {
    for command in items {
        f(&mut command.span);
        commands_spans(&mut command.items, f);
    }
}

fn levels_spans(levels: &mut [TreeLevel], f: &mut dyn FnMut(&mut Span)) {
    for level in levels {
        f(&mut level.span);
        levels_spans(&mut level.levels, f);
    }
}

fn parts_spans(widgets: &mut [Widget], f: &mut dyn FnMut(&mut Span)) {
    for widget in widgets {
        f(&mut widget.span);
        if let Some(menu) = &mut widget.menu {
            f(&mut menu.span);
            commands_spans(&mut menu.items, f);
        }
        levels_spans(&mut widget.levels, f);
        widget.selects.iter_mut().for_each(|s| f(&mut s.span));
        parts_spans(&mut widget.children, f);
        for page in &mut widget.pages {
            f(&mut page.span);
            parts_spans(&mut page.children, f);
        }
    }
}
