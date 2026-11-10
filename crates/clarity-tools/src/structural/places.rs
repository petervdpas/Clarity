//! Names to places: the structural tools name an entity, a window or a part
//! by its name, and a designer edit wants where it starts in its file. Each
//! function finds the file and the place in the loaded project, or says
//! which names there are when the name is not one of them (no guessing).

// -- the data model (structural::model) -----------------------------------

/// The checked project in `dir`, as the model tools read it.
pub fn model_project(dir: &std::path::Path) -> Result<clarity_spec::Project, String> {
    clarity_project::load_modules(dir).map_err(|err| format!("cannot read the project: {err}"))
}

/// `what` is not one of `names`: said with the names there are.
fn unknown_in_model(what: &str, kinds: &str, names: &[&str]) -> String {
    if names.is_empty() {
        format!("there is no {what}; there are no {kinds} yet")
    } else {
        format!("there is no {what}; the {kinds} are {}", crate::listed(names))
    }
}

/// The modules of the project: the `.clarity` files that declare one.
fn model_modules(project: &clarity_spec::Project) -> impl Iterator<Item = (&str, &str)> {
    project
        .files
        .iter()
        .filter(|f| std::path::Path::new(&f.path).extension().is_some_and(|e| e == clarity_spec::SPEC_EXTENSION))
        .filter_map(|f| f.module.as_ref().map(|m| (m.name.as_str(), f.path.as_str())))
}

/// The names of the project's modules.
pub fn module_names(project: &clarity_spec::Project) -> Vec<&str> {
    let mut names: Vec<&str> = model_modules(project).map(|(name, _)| name).collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// The file of the module `name`.
pub fn module_file(project: &clarity_spec::Project, name: &str) -> Result<std::path::PathBuf, String> {
    let files: Vec<&str> = model_modules(project).filter(|(m, _)| *m == name).map(|(_, path)| path).collect();
    match files.as_slice() {
        [file] => Ok(std::path::PathBuf::from(file)),
        [] => Err(unknown_in_model(&format!("module {name}"), "modules", &module_names(project))),
        _ => Err(format!("the module {name} is in several files ({}); keep it in one", crate::listed(&files))),
    }
}

/// The entities of the project (not its records or settings), with their files.
fn model_entities(project: &clarity_spec::Project) -> impl Iterator<Item = (&std::path::Path, &clarity_spec::model::Entity)> {
    project.files.iter().flat_map(|f| f.entities.iter().filter(|e| !e.settings && !e.record).map(move |e| (std::path::Path::new(&f.path), e)))
}

/// The names of the project's entities.
pub fn entity_names(project: &clarity_spec::Project) -> Vec<&str> {
    model_entities(project).map(|(_, e)| e.name.name.as_str()).collect()
}

/// The names of the project's records (DESIGN 5.12).
pub fn record_names(project: &clarity_spec::Project) -> Vec<&str> {
    project.files.iter().flat_map(|f| f.records.iter().map(|r| r.name.name.as_str())).collect()
}

/// The entity `name` and its file.
pub fn entity<'a>(project: &'a clarity_spec::Project, name: &str) -> Result<(std::path::PathBuf, &'a clarity_spec::model::Entity), String> {
    model_entities(project)
        .find(|(_, e)| e.name.name == name)
        .map(|(file, e)| (file.to_path_buf(), e))
        .ok_or_else(|| unknown_in_model(&format!("entity {name}"), "entities", &entity_names(project)))
}

/// The field `name` of `entity`.
pub fn field<'a>(entity: &'a clarity_spec::model::Entity, name: &str) -> Result<&'a clarity_spec::model::Field, String> {
    entity.field(name).ok_or_else(|| {
        let names: Vec<&str> = entity.fields.iter().map(|f| f.name.name.as_str()).collect();
        unknown_in_model(&format!("field {name} in {}", entity.name.name), &format!("fields of {}", entity.name.name), &names)
    })
}

/// The names of the project's enums.
pub fn enum_names(project: &clarity_spec::Project) -> Vec<&str> {
    project.files.iter().flat_map(|f| f.enums.iter().map(|e| e.name.name.as_str())).collect()
}

/// The enum `name` and its file.
pub fn enum_decl<'a>(project: &'a clarity_spec::Project, name: &str) -> Result<(std::path::PathBuf, &'a clarity_spec::model::EnumDecl), String> {
    project
        .files
        .iter()
        .find_map(|f| f.enums.iter().find(|e| e.name.name == name).map(|e| (std::path::PathBuf::from(&f.path), e)))
        .ok_or_else(|| unknown_in_model(&format!("enum {name}"), "enums", &enum_names(project)))
}

/// The value `name` of `decl`.
pub fn variant<'a>(decl: &'a clarity_spec::model::EnumDecl, name: &str) -> Result<&'a clarity_spec::model::EnumVariant, String> {
    decl.variants.iter().find(|v| v.name.name == name).ok_or_else(|| {
        let names: Vec<&str> = decl.variants.iter().map(|v| v.name.name.as_str()).collect();
        unknown_in_model(&format!("value {name} in {}", decl.name.name), &format!("values of {}", decl.name.name), &names)
    })
}

// -- windows, views and their parts (structural::windows) -----------------

/// `what` is not one of `names`: said with the names there are.
fn unknown_on_page(what: &str, kinds: &str, names: &[String]) -> String {
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    unknown_in_model(what, kinds, &names)
}

/// A window or view (any page: a dialog, splash or toolbox too), read from its `.cxaml`.
pub struct PageAt {
    pub file: std::path::PathBuf,
    pub page: clarity_spec::model::Page,
}

/// The `.cxaml` files of the project's design folder, with their markup.
fn design_files(dir: &std::path::Path) -> Vec<(std::path::PathBuf, String)> {
    let mut files: Vec<(std::path::PathBuf, String)> = std::fs::read_dir(clarity_spec::cxaml::design_folder(dir))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| clarity_spec::cxaml::is_page_file(&n.to_string_lossy())))
        .filter_map(|p| std::fs::read_to_string(&p).ok().map(|text| (p, text)))
        .collect();
    files.sort();
    files
}

/// The names of the project's windows and views, sorted.
pub fn page_names(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = design_files(dir)
        .into_iter()
        .filter(|(_, markup)| !clarity_spec::cxaml::is_app_markup(markup))
        .filter_map(|(_, markup)| clarity_spec::cxaml::class_of(&markup).map(|(_, name)| name))
        .collect();
    names.sort();
    names
}

/// The window or view `name` and its file.
pub fn page(dir: &std::path::Path, name: &str) -> Result<PageAt, String> {
    let found = design_files(dir)
        .into_iter()
        .filter(|(_, markup)| !clarity_spec::cxaml::is_app_markup(markup))
        .find(|(_, markup)| clarity_spec::cxaml::class_of(markup).is_some_and(|(_, n)| n == name));
    let Some((file, markup)) = found else {
        return Err(unknown_on_page(&format!("window or view {name}"), "windows and views", &page_names(dir)));
    };
    let shown = format!("design/{}", file.file_name().unwrap_or_default().to_string_lossy());
    match clarity_spec::cxaml::read_designed(&markup, &shown)? {
        clarity_spec::design_edit::Designed::Page(page) => Ok(PageAt { file, page }),
        clarity_spec::design_edit::Designed::App(_) => Err(format!("{name} is the app, not a window or view")),
    }
}

/// Every part of the page with a name: on its grid, in its containers, its sidebars and footer.
pub fn page_parts(page: &clarity_spec::model::Page) -> Vec<&clarity_spec::model::Widget> {
    fn walk<'a>(widgets: &'a [clarity_spec::model::Widget], out: &mut Vec<&'a clarity_spec::model::Widget>) {
        for w in widgets {
            out.push(w);
            walk(&w.children, out);
            for inner in &w.pages {
                walk(&inner.children, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&page.widgets, &mut out);
    for sidebar in &page.frame.sidebars {
        walk(&sidebar.widgets, &mut out);
    }
    if let Some(footer) = &page.frame.footer {
        walk(&footer.widgets, &mut out);
    }
    out
}

/// The names of the page's parts, in the order they are written.
pub fn part_names(page: &clarity_spec::model::Page) -> Vec<String> {
    page_parts(page).into_iter().filter_map(|w| w.props.name.clone()).collect()
}

/// The part `name` of the page.
pub fn part<'a>(page: &'a clarity_spec::model::Page, name: &str) -> Result<&'a clarity_spec::model::Widget, String> {
    page_parts(page)
        .into_iter()
        .find(|w| w.props.name.as_deref() == Some(name))
        .ok_or_else(|| unknown_on_page(&format!("part {name} in {}", page.name.name), &format!("parts of {}", page.name.name), &part_names(page)))
}

/// The tab, step, section, pane or content line `number` (from 1) of the part `widget`.
pub fn tab(widget: &clarity_spec::model::Widget, number: u32) -> Result<&clarity_spec::model::WidgetPage, String> {
    let name = widget.props.name.clone().unwrap_or_default();
    if widget.pages.is_empty() {
        return Err(format!("{name} is a {} and has no tabs, steps, sections or panes", clarity_spec::widgets::keyword(widget.props.kind)));
    }
    let index = (number as usize).checked_sub(1).filter(|i| *i < widget.pages.len());
    index.map(|i| &widget.pages[i]).ok_or_else(|| {
        let listed: Vec<String> = widget
            .pages
            .iter()
            .enumerate()
            .map(|(i, p)| match (&p.title, &p.view) {
                (Some(title), _) => format!("{} \"{title}\"", i + 1),
                (None, Some(view)) => format!("{} view {view}", i + 1),
                (None, None) => (i + 1).to_string(),
            })
            .collect();
        format!("{name} has no tab {number}; it has {}: {}", widget.pages.len(), listed.join(", "))
    })
}

/// Where parts are put: the place a designer edit names the container by,
/// and the columns of its grid.
pub struct ContainerAt {
    pub at: u32,
    pub columns: u32,
}

/// The container `inside` names on the page: none or the page's own name
/// for its grid, a container part (with `tab`, from 1, for tabs, steps,
/// sections and panes), a sidebar by its name, or `footer`.
pub fn container(page: &clarity_spec::model::Page, inside: Option<&str>, tab_number: Option<u32>) -> Result<ContainerAt, String> {
    let inside = inside.filter(|n| *n != page.name.name);
    let Some(name) = inside else {
        if tab_number.is_some() {
            return Err("a tab is one of a part's: give the part as `in` too".into());
        }
        return Ok(ContainerAt { at: page.span.start, columns: page.columns });
    };
    let alone = |at: u32, columns: u32| match tab_number {
        Some(_) => Err(format!("{name} has no tabs; leave tab out")),
        None => Ok(ContainerAt { at, columns }),
    };
    if let Some(sidebar) = page.frame.sidebars.iter().find(|s| s.props.name.as_deref() == Some(name)) {
        return alone(sidebar.span.start, sidebar.props.columns);
    }
    let found = page_parts(page).into_iter().find(|w| w.props.name.as_deref() == Some(name));
    if found.is_none()
        && name == "footer"
        && let Some(footer) = &page.frame.footer
    {
        return alone(footer.span.start, footer.columns);
    }
    let Some(widget) = found else {
        let mut names: Vec<String> = page_parts(page).into_iter().filter(|w| clarity_spec::widgets::container(w.props.kind).is_some()).filter_map(|w| w.props.name.clone()).collect();
        names.extend(page.frame.sidebars.iter().filter_map(|s| s.props.name.clone()));
        if page.frame.footer.is_some() {
            names.push("footer".into());
        }
        names.insert(0, page.name.name.clone());
        return Err(unknown_on_page(&format!("container {name} in {}", page.name.name), &format!("places for parts in {} (the page itself by its name)", page.name.name), &names));
    };
    let width = widget.props.width;
    match clarity_spec::widgets::container(widget.props.kind) {
        Some(clarity_spec::widgets::ContainerKind::Plain) => alone(widget.span.start, width),
        Some(clarity_spec::widgets::ContainerKind::Content) => Err(format!("{name} is a content: it shows views, and their parts are put in the view")),
        Some(_) => {
            let Some(number) = tab_number else {
                return Err(format!("{name} is a {}: give tab, the number of its tab, step, section or pane (from 1)", clarity_spec::widgets::keyword(widget.props.kind)));
            };
            let inner = tab(widget, number)?;
            if inner.view.is_some() {
                return Err(format!("tab {number} of {name} shows a view; put the part in that view"));
            }
            let columns = match widget.props.kind {
                clarity_spec::widgets::WidgetKind::Splitter => {
                    let first = widget.pages.first().and_then(|p| p.columns);
                    match (number, first) {
                        (1, Some(first)) => first,
                        (_, Some(first)) => width.saturating_sub(first).max(1),
                        _ => width,
                    }
                }
                _ => width,
            };
            Ok(ContainerAt { at: inner.span.start, columns })
        }
        None => Err(format!("{name} is a {}, which holds no parts", clarity_spec::widgets::keyword(widget.props.kind))),
    }
}

/// The data `name` declared on the page.
pub fn page_data<'a>(page: &'a clarity_spec::model::Page, name: &str) -> Result<&'a clarity_spec::model::DataDecl, String> {
    page.data.iter().find(|d| d.name.name == name && !d.made).ok_or_else(|| {
        let names: Vec<String> = page.data.iter().filter(|d| !d.made).map(|d| d.name.name.clone()).collect();
        unknown_on_page(&format!("data {name} on {}", page.name.name), &format!("data of {}", page.name.name), &names)
    })
}

/// The module file a new window or view goes beside: `module` when given,
/// else the app's module, else the project's one module.
pub fn module_for_page(dir: &std::path::Path, module: Option<&str>) -> Result<std::path::PathBuf, String> {
    let project = model_project(dir)?;
    if let Some(module) = module {
        return module_file(&project, module);
    }
    let app_module = design_files(dir).into_iter().find(|(_, m)| clarity_spec::cxaml::is_app_markup(m)).and_then(|(_, m)| clarity_spec::cxaml::class_of(&m)).map(|(module, _)| module);
    if let Some(module) = app_module {
        return module_file(&project, &module);
    }
    match module_names(&project).as_slice() {
        [one] => module_file(&project, one),
        names => Err(format!("give module, the one the window goes in: {}", crate::listed(names))),
    }
}

/// The custom part declared as `name` (DESIGN 5.19): its module's file and where it starts.
pub fn part_decl(dir: &std::path::Path, name: &str) -> Result<(std::path::PathBuf, u32), String> {
    let project = model_project(dir)?;
    let found = project.files.iter().find_map(|f| f.parts.iter().find(|d| d.name.name == name).map(|d| (std::path::PathBuf::from(&f.path), d.span.start)));
    found.ok_or_else(|| {
        let names: Vec<&str> = project.files.iter().flat_map(|f| f.parts.iter().map(|d| d.name.name.as_str())).collect();
        unknown_in_model(&format!("custom part {name}"), "custom parts", &names)
    })
}

/// The part kinds a tool takes, as their names in edits (`textbox`, `treeTable`).
pub fn part_kinds() -> Vec<String> {
    clarity_spec::widgets::catalog().into_iter().filter_map(|info| serde_json::to_value(info.kind).ok().and_then(|v| v.as_str().map(str::to_string))).collect()
}

/// The part kind `name`, one of `part_kinds`.
pub fn part_kind(name: &str) -> Result<clarity_spec::widgets::WidgetKind, String> {
    let kinds = part_kinds();
    if !kinds.iter().any(|k| k == name) {
        return Err(format!("there is no part kind '{name}'; the kinds are {}", kinds.join(", ")));
    }
    serde_json::from_value(serde_json::Value::String(name.to_string())).map_err(|err| err.to_string())
}

// -- the app and the frames (structural::frame) ---------------------------

/// A window, view or the app found by name: its design file, where it
/// starts there, and what it holds.
pub struct FrameOwner<'a> {
    pub file: std::path::PathBuf,
    pub start: u32,
    /// Its name, which its code block goes by.
    pub name: String,
    pub page: Option<&'a clarity_spec::model::Page>,
    pub app: Option<&'a clarity_spec::model::App>,
}

impl<'a> FrameOwner<'a> {
    pub fn frame(&self) -> &'a clarity_spec::model::Frame {
        match (self.page, self.app) {
            (Some(page), _) => &page.frame,
            (None, Some(app)) => &app.frame,
            (None, None) => unreachable!("an owner is a page or the app"),
        }
    }

    /// Every list of parts it has: its grid, its sidebars and its footer.
    fn part_lists(&self) -> Vec<&'a [clarity_spec::model::Widget]> {
        let frame = self.frame();
        let mut lists: Vec<&'a [clarity_spec::model::Widget]> = self.page.map(|p| p.widgets.as_slice()).into_iter().collect();
        lists.extend(frame.sidebars.iter().map(|s| s.widgets.as_slice()));
        lists.extend(frame.footer.iter().map(|f| f.widgets.as_slice()));
        lists
    }
}

/// A design file's path, which the project may give relative to `dir`.
fn frame_file(dir: &std::path::Path, path: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(path);
    if path.is_absolute() { path.to_path_buf() } else { dir.join(path) }
}

/// The names of the project's windows and views, and its app, as an owner is named.
pub fn owner_names(project: &clarity_spec::Project) -> Vec<String> {
    let mut names: Vec<String> = project.pages().map(|p| p.name.name.clone()).collect();
    names.extend(project.apps().map(|a| format!("app (the app {})", a.name.name)));
    names
}

/// The window, view or app `name`: `app` or the app's own name for the app.
pub fn frame_owner<'a>(project: &'a clarity_spec::Project, dir: &std::path::Path, name: &str) -> Result<FrameOwner<'a>, String> {
    for file in &project.files {
        for app in &file.apps {
            if name == "app" || app.name.name == name {
                return Ok(FrameOwner { file: frame_file(dir, &file.path), start: app.span.start, name: app.name.name.clone(), page: None, app: Some(app) });
            }
        }
        for page in &file.pages {
            if page.name.name == name {
                return Ok(FrameOwner { file: frame_file(dir, &file.path), start: page.span.start, name: page.name.name.clone(), page: Some(page), app: None });
            }
        }
    }
    let names = owner_names(project);
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    Err(if name == "app" { format!("the project has no app; add one with add_app. Its windows and views are {}", crate::listed(&names)) } else { unknown_in_model(&format!("window, view or app {name}"), "windows, views and app", &names) })
}

/// The names of the project's pages of `kinds`.
pub fn page_names_of<'a>(project: &'a clarity_spec::Project, kinds: &[clarity_spec::model::PageKind]) -> Vec<&'a str> {
    project.pages().filter(|p| kinds.contains(&p.kind)).map(|p| p.name.name.as_str()).collect()
}

/// Every menu item, button and separator of a frame and of its owner's
/// parts' menus, depth first.
fn frame_commands<'a>(owner: &FrameOwner<'a>) -> Vec<&'a clarity_spec::model::Command> {
    fn walk<'a>(list: &'a [clarity_spec::model::Command], out: &mut Vec<&'a clarity_spec::model::Command>) {
        for c in list {
            out.push(c);
            walk(&c.items, out);
        }
    }
    fn parts<'a>(list: &'a [clarity_spec::model::Widget], out: &mut Vec<&'a clarity_spec::model::Command>) {
        for w in list {
            if let Some(menu) = &w.menu {
                walk(&menu.items, out);
            }
            parts(&w.children, out);
            for page in &w.pages {
                parts(&page.children, out);
            }
        }
    }
    let frame = owner.frame();
    let mut out = Vec::new();
    if let Some(menu) = &frame.menu {
        walk(&menu.items, &mut out);
    }
    if let Some(toolbar) = &frame.toolbar {
        walk(&toolbar.items, &mut out);
    }
    for group in frame.ribbon.iter().flat_map(|r| &r.tabs).flat_map(|t| &t.groups) {
        walk(&group.items, &mut out);
    }
    for list in owner.part_lists() {
        parts(list, &mut out);
    }
    out
}

/// The menu item or button `name` of the owner's frame or its parts' menus.
pub fn frame_command<'a>(owner: &FrameOwner<'a>, name: &str) -> Result<&'a clarity_spec::model::Command, String> {
    let commands = frame_commands(owner);
    if let Some(found) = commands.iter().find(|c| c.props.name.as_deref() == Some(name)) {
        return Ok(found);
    }
    let names: Vec<&str> = commands.iter().filter_map(|c| c.props.name.as_deref()).collect();
    let unnamed = commands.iter().any(|c| c.props.name.is_none() && c.props.kind != clarity_spec::model::CommandKind::Separator);
    let hint = if unnamed { "; some have no name yet, which name_parts gives them" } else { "" };
    Err(format!("{}{hint}", unknown_in_model(&format!("menu item or button {name} in {}", owner.name), &format!("menu items and buttons of {}", owner.name), &names)))
}

/// What holds commands, ribbon tabs or groups in the owner's frame, by name:
/// the menu, the toolbar, the ribbon, a ribbon tab or group, a menu item.
pub fn frame_parent(owner: &FrameOwner<'_>, name: &str) -> Result<u32, String> {
    let frame = owner.frame();
    let mut named: Vec<(String, u32)> = Vec::new();
    if let Some(menu) = &frame.menu {
        named.extend(menu.name.clone().map(|n| (n, menu.span.start)));
    }
    if let Some(toolbar) = &frame.toolbar {
        named.extend(toolbar.name.clone().map(|n| (n, toolbar.span.start)));
    }
    if let Some(ribbon) = &frame.ribbon {
        named.extend(ribbon.name.clone().map(|n| (n, ribbon.span.start)));
        for tab in &ribbon.tabs {
            named.extend(tab.name.clone().map(|n| (n, tab.span.start)));
            for group in &tab.groups {
                named.extend(group.name.clone().map(|n| (n, group.span.start)));
            }
        }
    }
    for c in frame_commands(owner) {
        if c.props.kind == clarity_spec::model::CommandKind::Item {
            named.extend(c.props.name.clone().map(|n| (n, c.span.start)));
        }
    }
    if let Some((_, at)) = named.iter().find(|(n, _)| n == name) {
        return Ok(*at);
    }
    let names: Vec<&str> = named.iter().map(|(n, _)| n.as_str()).collect();
    Err(unknown_in_model(&format!("menu, toolbar, ribbon, ribbon tab or group, or menu item {name} in {}", owner.name), &format!("ones of {}", owner.name), &names))
}

/// The menu, toolbar, ribbon tab or ribbon group `name` of the owner's frame.
pub fn frame_named(owner: &FrameOwner<'_>, name: &str) -> Result<u32, String> {
    let frame = owner.frame();
    let mut named: Vec<(&str, u32)> = Vec::new();
    named.extend(frame.menu.iter().filter_map(|m| m.name.as_deref().map(|n| (n, m.span.start))));
    named.extend(frame.toolbar.iter().filter_map(|t| t.name.as_deref().map(|n| (n, t.span.start))));
    for tab in frame.ribbon.iter().flat_map(|r| &r.tabs) {
        named.extend(tab.name.as_deref().map(|n| (n, tab.span.start)));
        named.extend(tab.groups.iter().filter_map(|g| g.name.as_deref().map(|n| (n, g.span.start))));
    }
    named.iter().find(|(n, _)| *n == name).map(|(_, at)| *at).ok_or_else(|| {
        let names: Vec<&str> = named.iter().map(|(n, _)| *n).collect();
        unknown_in_model(&format!("menu, toolbar, or ribbon tab or group {name} in {}", owner.name), &format!("ones of {}", owner.name), &names)
    })
}

/// The ribbon tab or group `name` of the owner's frame.
pub fn ribbon_part(owner: &FrameOwner<'_>, name: &str) -> Result<u32, String> {
    let mut named: Vec<(&str, u32)> = Vec::new();
    for tab in owner.frame().ribbon.iter().flat_map(|r| &r.tabs) {
        named.extend(tab.name.as_deref().map(|n| (n, tab.span.start)));
        named.extend(tab.groups.iter().filter_map(|g| g.name.as_deref().map(|n| (n, g.span.start))));
    }
    named.iter().find(|(n, _)| *n == name).map(|(_, at)| *at).ok_or_else(|| {
        let names: Vec<&str> = named.iter().map(|(n, _)| *n).collect();
        unknown_in_model(&format!("ribbon tab or group {name} in {}", owner.name), &format!("ribbon tabs and groups of {}", owner.name), &names)
    })
}

/// The sidebar `name` of the owner's frame.
pub fn frame_sidebar<'a>(owner: &FrameOwner<'a>, name: &str) -> Result<&'a clarity_spec::model::Sidebar, String> {
    let sidebars = &owner.frame().sidebars;
    sidebars.iter().find(|s| s.props.name.as_deref() == Some(name)).ok_or_else(|| {
        let names: Vec<&str> = sidebars.iter().filter_map(|s| s.props.name.as_deref()).collect();
        unknown_in_model(&format!("sidebar {name} in {}", owner.name), &format!("sidebars of {}", owner.name), &names)
    })
}

/// The part `name` of the owner: on its grid, in a container, a sidebar or the footer.
pub fn frame_part<'a>(owner: &FrameOwner<'a>, name: &str) -> Result<&'a clarity_spec::model::Widget, String> {
    fn walk<'a>(list: &'a [clarity_spec::model::Widget], out: &mut Vec<&'a clarity_spec::model::Widget>) {
        for w in list {
            out.push(w);
            walk(&w.children, out);
            for page in &w.pages {
                walk(&page.children, out);
            }
        }
    }
    let mut parts = Vec::new();
    for list in owner.part_lists() {
        walk(list, &mut parts);
    }
    if let Some(found) = parts.iter().find(|w| w.props.name.as_deref() == Some(name)) {
        return Ok(found);
    }
    let names: Vec<&str> = parts.iter().filter_map(|w| w.props.name.as_deref()).collect();
    let unnamed = parts.iter().any(|w| w.props.name.is_none());
    let hint = if unnamed { "; some have no name yet, which name_parts gives them" } else { "" };
    Err(format!("{}{hint}", unknown_in_model(&format!("part {name} in {}", owner.name), &format!("parts of {}", owner.name), &names)))
}

/// The handlers in the code behind of the window, view or app `owner`.
pub fn handler_names<'a>(project: &'a clarity_spec::Project, owner: &str) -> Vec<&'a str> {
    project.files.iter().flat_map(|f| &f.code).filter(|b| b.name.name == owner).flat_map(|b| &b.handlers).map(|h| h.name.name.as_str()).collect()
}

/// The names of the owner's menu items and buttons, its parts' menus' too.
pub fn command_names<'a>(owner: &FrameOwner<'a>) -> Vec<&'a str> {
    frame_commands(owner).into_iter().filter_map(|c| c.props.name.as_deref()).collect()
}
