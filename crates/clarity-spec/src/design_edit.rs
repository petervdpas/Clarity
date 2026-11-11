//! The designer's edits made on what a design file holds, a window's or
//! view's page (or the app), instead of on text: the `.cxaml` is read, the
//! model changed and the file written again (`cxaml::apply`). An edit names
//! a thing by where it starts in the file it was read from, which is where
//! the model read from that file has it; nothing is translated.
//!
//! What an edit makes new gets a place of its own above every place in a
//! file (`NEW`), so the edit can say what it produced; `cxaml::apply` finds
//! it again in the file as written by its key.

use crate::edit::{Edit, EditError, EditResult, FrameItem};
use crate::frame::{self, RegionKind};
use crate::model::{
    App, Command, CommandKind, CommandProps, DataDecl, DataKind, Footer, Frame, Handler, Ident, Menu, Page, Project, Ribbon, RibbonGroup, RibbonTab, Side, Sidebar, Toolbar,
    Widget, WidgetPage, WidgetProps, WindowSize,
};
use crate::span::Span;
use crate::widgets::{self, ContainerKind, Event, WidgetKind};

/// What a design file holds: a window's or view's page, or the app.
#[derive(Debug, Clone, PartialEq)]
pub enum Designed {
    Page(Page),
    App(App),
}

impl Designed {
    fn frame(&mut self) -> &mut Frame {
        match self {
            Designed::Page(page) => &mut page.frame,
            Designed::App(app) => &mut app.frame,
        }
    }

    fn frame_ref(&self) -> &Frame {
        match self {
            Designed::Page(page) => &page.frame,
            Designed::App(app) => &app.frame,
        }
    }

    /// Where the page or app starts in its file.
    pub fn start(&self) -> u32 {
        match self {
            Designed::Page(page) => page.span.start,
            Designed::App(app) => app.span.start,
        }
    }

    /// The parts on the page's own grid; an app has none.
    fn widgets(&mut self) -> Option<&mut Vec<Widget>> {
        match self {
            Designed::Page(page) => Some(&mut page.widgets),
            Designed::App(_) => None,
        }
    }

    fn data(&mut self) -> &mut Vec<DataDecl> {
        match self {
            Designed::Page(page) => &mut page.data,
            Designed::App(app) => &mut app.data,
        }
    }

    fn handlers(&mut self) -> &mut Vec<Handler> {
        match self {
            Designed::Page(page) => &mut page.handlers,
            Designed::App(app) => &mut app.handlers,
        }
    }

    fn page(&mut self, what: &str) -> EditResult<&mut Page> {
        match self {
            Designed::Page(page) => Ok(page),
            Designed::App(_) => fail(format!("the app has no {what}; a window has")),
        }
    }

    fn app(&mut self) -> EditResult<&mut App> {
        match self {
            Designed::App(app) => Ok(app),
            Designed::Page(_) => fail("only the app has a main window and a splash"),
        }
    }

    /// Every list of parts: the page's grid, its sidebars and its footer.
    fn all_widgets(&mut self) -> Vec<&mut Vec<Widget>> {
        let (widgets, frame) = match self {
            Designed::Page(page) => (Some(&mut page.widgets), &mut page.frame),
            Designed::App(app) => (None, &mut app.frame),
        };
        let mut out: Vec<&mut Vec<Widget>> = widgets.into_iter().collect();
        out.extend(frame.sidebars.iter_mut().map(|s| &mut s.widgets));
        if let Some(footer) = &mut frame.footer {
            out.push(&mut footer.widgets);
        }
        out
    }
}

fn fail<T>(message: impl Into<String>) -> EditResult<T> {
    Err(EditError(message.into()))
}

/// Where the things an edit makes start: above every place in a file.
pub const NEW: u32 = 0x8000_0000;

/// Whether `edit` is made on a design file: what a window, view or app holds.
/// Code, entities, enums and new windows belong in a module's file.
pub fn is_design_edit(edit: &Edit) -> bool {
    use Edit::*;
    match edit {
        Batch { edits } => edits.iter().all(is_design_edit),
        Remove { .. } | SetTitle { .. } | SetPaneSpan { .. } | SetPageView { .. } | SetPageColor { .. } | PlaceWidget { .. } | CreateWidget { .. } | UpdateWidget { .. } | AddTab { .. } | AddMenu { .. }
        | AddData { .. } | SetGrid { .. } | SetSize { .. } | SetPageStyle { .. } | SetPageAllow { .. } | PlaceField { .. } | NameParts | SetMain { .. } | SetSplash { .. } | SetOwnerEvent { .. } | AddRegion { .. }
        | UpdateSidebar { .. } | UpdateSidebars | UpdateRibbon { .. } | SetName { .. } | AddFrameItem { .. } | UpdateCommand { .. } | MoveSibling { .. } => true,
        _ => false,
    }
}

/// An edit of a design file split in two: what the design file takes, and
/// what the module's file takes (the handlers a wiring writes, code).
pub fn split(edit: &Edit) -> (Option<Edit>, Option<Edit>) {
    let one = |edits: Vec<Edit>| match edits.len() {
        0 => None,
        1 => edits.into_iter().next(),
        _ => Some(Edit::Batch { edits }),
    };
    match edit {
        Edit::Batch { edits } => {
            let (design, code): (Vec<Edit>, Vec<Edit>) = edits.iter().cloned().partition(is_design_edit);
            (one(design), one(code))
        }
        edit if is_design_edit(edit) => (Some(edit.clone()), None),
        edit => (None, Some(edit.clone())),
    }
}

/// Makes `edit` on `designed`; `project`, when given, lets it build on the
/// rest (a field's type). Gives where what it produced or changed starts:
/// a place it had, or one of `NEW`.
pub fn apply(designed: &mut Designed, edit: &Edit, project: Option<&Project>) -> EditResult<Option<u32>> {
    Editor { next: NEW, project }.apply(designed, edit)
}

struct Editor<'p> {
    next: u32,
    project: Option<&'p Project>,
}

// ---------------------------------------------------------------------------
// Finding things by where they start

fn find_widget(widgets: &mut [Widget], at: u32) -> Option<&mut Widget> {
    for w in widgets {
        if w.span.start == at {
            return Some(w);
        }
        if let Some(found) = find_widget(&mut w.children, at) {
            return Some(found);
        }
        for page in &mut w.pages {
            if let Some(found) = find_widget(&mut page.children, at) {
                return Some(found);
            }
        }
    }
    None
}

/// A tab, step, section, pane or line of a content, with the kind of the part that holds it.
fn find_inner(widgets: &mut [Widget], at: u32) -> Option<(WidgetKind, &mut WidgetPage)> {
    for w in widgets {
        let kind = w.props.kind;
        if let Some(index) = w.pages.iter().position(|p| p.span.start == at) {
            return Some((kind, &mut w.pages[index]));
        }
        if let Some(found) = find_inner(&mut w.children, at) {
            return Some(found);
        }
        for page in &mut w.pages {
            if let Some(found) = find_inner(&mut page.children, at) {
                return Some(found);
            }
        }
    }
    None
}

/// The parts of the container that starts at `at`: the page, a panel, a tab or pane, a sidebar, the footer.
fn container_of(designed: &mut Designed, at: u32) -> Option<&mut Vec<Widget>> {
    if designed.start() == at {
        return designed.widgets();
    }
    fn inside(widgets: &mut [Widget], at: u32) -> Option<&mut Vec<Widget>> {
        for w in widgets {
            if w.span.start == at && widgets::container(w.props.kind) == Some(ContainerKind::Plain) {
                return Some(&mut w.children);
            }
            if let Some(found) = inside(&mut w.children, at) {
                return Some(found);
            }
            for page in &mut w.pages {
                if page.span.start == at {
                    return Some(&mut page.children);
                }
                if let Some(found) = inside(&mut page.children, at) {
                    return Some(found);
                }
            }
        }
        None
    }
    let frame = designed.frame_ref();
    let sidebar = frame.sidebars.iter().position(|s| s.span.start == at);
    let footer = frame.footer.as_ref().is_some_and(|f| f.span.start == at);
    if let Some(index) = sidebar {
        return Some(&mut designed.frame().sidebars[index].widgets);
    }
    if footer {
        return designed.frame().footer.as_mut().map(|f| &mut f.widgets);
    }
    designed.all_widgets().into_iter().find_map(|list| inside(list, at))
}

fn remove_widget(widgets: &mut Vec<Widget>, at: u32) -> Option<Widget> {
    if let Some(index) = widgets.iter().position(|w| w.span.start == at) {
        return Some(widgets.remove(index));
    }
    for w in widgets {
        if let Some(found) = remove_widget(&mut w.children, at) {
            return Some(found);
        }
        for page in &mut w.pages {
            if let Some(found) = remove_widget(&mut page.children, at) {
                return Some(found);
            }
        }
    }
    None
}

fn remove_inner(widgets: &mut [Widget], at: u32) -> bool {
    for w in widgets {
        if let Some(index) = w.pages.iter().position(|p| p.span.start == at) {
            w.pages.remove(index);
            return true;
        }
        if remove_inner(&mut w.children, at) || w.pages.iter_mut().any(|p| remove_inner(&mut p.children, at)) {
            return true;
        }
    }
    false
}

/// Every list of menu items and buttons: the frame's and the parts' menus.
fn command_lists(designed: &mut Designed) -> Vec<&mut Vec<Command>> {
    fn of_widgets<'a>(widgets: &'a mut [Widget], out: &mut Vec<&'a mut Vec<Command>>) {
        for w in widgets {
            if let Some(menu) = &mut w.menu {
                out.push(&mut menu.items);
            }
            of_widgets(&mut w.children, out);
            for page in &mut w.pages {
                of_widgets(&mut page.children, out);
            }
        }
    }
    let (widgets, frame) = match designed {
        Designed::Page(page) => (Some(&mut page.widgets), &mut page.frame),
        Designed::App(app) => (None, &mut app.frame),
    };
    let mut out = Vec::new();
    if let Some(menu) = &mut frame.menu {
        out.push(&mut menu.items);
    }
    if let Some(toolbar) = &mut frame.toolbar {
        out.push(&mut toolbar.items);
    }
    if let Some(ribbon) = &mut frame.ribbon {
        for tab in &mut ribbon.tabs {
            for group in &mut tab.groups {
                out.push(&mut group.items);
            }
        }
    }
    for sidebar in &mut frame.sidebars {
        of_widgets(&mut sidebar.widgets, &mut out);
    }
    if let Some(footer) = &mut frame.footer {
        of_widgets(&mut footer.widgets, &mut out);
    }
    if let Some(widgets) = widgets {
        of_widgets(widgets, &mut out);
    }
    out
}

fn find_command(items: &mut [Command], at: u32) -> Option<&mut Command> {
    for c in items {
        if c.span.start == at {
            return Some(c);
        }
        if let Some(found) = find_command(&mut c.items, at) {
            return Some(found);
        }
    }
    None
}

/// The list a command is in, and its index there.
fn command_siblings(items: &mut Vec<Command>, at: u32) -> Option<(&mut Vec<Command>, usize)> {
    if let Some(index) = items.iter().position(|c| c.span.start == at) {
        return Some((items, index));
    }
    for c in items {
        if let Some(found) = command_siblings(&mut c.items, at) {
            return Some(found);
        }
    }
    None
}

/// Every name given in the window or app: parts, menus, items, regions, ribbon tabs and groups.
fn used_names(designed: &Designed) -> Vec<String> {
    fn commands(items: &[Command], out: &mut Vec<String>) {
        for c in items {
            out.extend(c.props.name.clone());
            commands(&c.items, out);
        }
    }
    fn parts(widgets: &[Widget], out: &mut Vec<String>) {
        for w in widgets {
            out.extend(w.props.name.clone());
            if let Some(menu) = &w.menu {
                out.extend(menu.name.clone());
                commands(&menu.items, out);
            }
            parts(&w.children, out);
            for page in &w.pages {
                parts(&page.children, out);
            }
        }
    }
    let mut out = Vec::new();
    let frame = designed.frame_ref();
    if let Some(menu) = &frame.menu {
        out.extend(menu.name.clone());
        commands(&menu.items, &mut out);
    }
    if let Some(toolbar) = &frame.toolbar {
        out.extend(toolbar.name.clone());
        commands(&toolbar.items, &mut out);
    }
    if let Some(ribbon) = &frame.ribbon {
        out.extend(ribbon.name.clone());
        for tab in &ribbon.tabs {
            out.extend(tab.name.clone());
            for group in &tab.groups {
                out.extend(group.name.clone());
                commands(&group.items, &mut out);
            }
        }
    }
    for sidebar in &frame.sidebars {
        out.extend(sidebar.props.name.clone());
        parts(&sidebar.widgets, &mut out);
    }
    if let Some(footer) = &frame.footer {
        parts(&footer.widgets, &mut out);
    }
    if let Designed::Page(page) = designed {
        parts(&page.widgets, &mut out);
    }
    out
}

/// `base`, or `base2`, `base3`... whichever is not in `used`.
fn free_in(used: &[String], base: &str) -> String {
    if used.iter().any(|u| u == base) { (2..).map(|n| format!("{base}{n}")).find(|n| !used.contains(n)).expect("an unused name exists") } else { base.to_string() }
}

/// The first `{base}{n}` not in `used`.
fn numbered_in(used: &[String], base: &str) -> String {
    (1..).map(|n| format!("{base}{n}")).find(|n| !used.contains(n)).expect("an unused name exists")
}

/// A ribbon tab's or group's name after its title and kind: `homeTab`, `clipboardGroup`.
fn ribbon_part_base(title: &str, kind: &str) -> String {
    let title = crate::names::camel_case(title.trim_start_matches('@'));
    if title.is_empty() { kind.to_lowercase() } else { format!("{title}{kind}") }
}

/// Inserts a part among `widgets` in reading order: by row, then column.
fn insert_widget(widgets: &mut Vec<Widget>, widget: Widget) {
    let at = (widget.props.row, widget.props.col);
    let index = widgets.iter().position(|w| (w.props.row, w.props.col) > at && !is_off_grid(w)).unwrap_or(widgets.len());
    widgets.insert(index, widget);
}

/// A popover, slide-out, sound or timer: no place on the grid.
fn is_off_grid(w: &Widget) -> bool {
    matches!(w.props.kind, WidgetKind::Popover | WidgetKind::Slideout | WidgetKind::Sound | WidgetKind::Timer)
}

/// Whether the part `w` or something in it starts at `at`.
fn holds(w: &Widget, at: u32) -> bool {
    w.span.start == at || w.children.iter().any(|c| holds(c, at)) || w.pages.iter().any(|p| p.span.start == at || p.children.iter().any(|c| holds(c, at)))
}

impl Editor<'_> {
    fn fresh(&mut self) -> Span {
        let start = self.next;
        self.next += 1;
        Span { start, end: start, line: 0, col: 0 }
    }

    fn inner(&mut self, title: Option<String>) -> WidgetPage {
        WidgetPage { title, columns: None, view: None, tells: Vec::new(), color: None, children: Vec::new(), span: self.fresh() }
    }

    fn command(&mut self, props: CommandProps) -> Command {
        Command { props, items: Vec::new(), span: self.fresh() }
    }

    /// A new part, with the pages its kind starts with.
    fn widget(&mut self, props: WidgetProps) -> Widget {
        let pages = match widgets::container(props.kind) {
            Some(ContainerKind::Tabs) => vec![self.inner(Some("Tab 1".into()))],
            Some(ContainerKind::Steps) => (1..=2).map(|n| self.inner(Some(format!("Step {n}")))).collect(),
            Some(ContainerKind::Accordion) => (1..=2).map(|n| self.inner(Some(format!("Section {n}")))).collect(),
            Some(ContainerKind::Splitter) => (0..2).map(|_| self.inner(None)).collect(),
            _ => Vec::new(),
        };
        Widget { props, children: Vec::new(), pages, menu: None, levels: Vec::new(), selects: Vec::new(), span: self.fresh() }
    }

    fn apply(&mut self, designed: &mut Designed, edit: &Edit) -> EditResult<Option<u32>> {
        match edit {
            Edit::Batch { edits } => {
                let mut focus = None;
                for edit in edits {
                    focus = self.apply(designed, edit)?.or(focus);
                }
                Ok(focus)
            }
            Edit::Remove { item } => self.remove(designed, *item),
            Edit::SetTitle { target, title } => self.set_title(designed, *target, title),
            Edit::SetPaneSpan { target, columns } => {
                let widgets = designed.page("panes")?;
                match find_inner(&mut widgets.widgets, *target) {
                    Some((WidgetKind::Splitter, pane)) => pane.columns = *columns,
                    _ => return fail("there is no pane there"),
                }
                Ok(Some(*target))
            }
            Edit::SetPageView { target, view } => {
                let page = designed.page("views")?;
                let Some((kind, line)) = find_inner(&mut page.widgets, *target) else { return fail("there is no view line of content there") };
                if kind != WidgetKind::Content {
                    return fail("a view shows only in content; put a content in it");
                }
                let Some(view) = view.as_deref().map(str::trim).filter(|v| !v.is_empty()) else {
                    return fail("a view line of content names a view; remove the line instead");
                };
                line.view = Some(view.to_string());
                Ok(Some(*target))
            }
            Edit::SetPageColor { target, color } => {
                let page = designed.page("sections")?;
                let Some((kind, section)) = find_inner(&mut page.widgets, *target) else { return fail("there is no section there") };
                if kind != WidgetKind::Accordion {
                    return fail("only an accordion's section has a color");
                }
                section.color = color.as_deref().map(str::trim).filter(|c| !c.is_empty()).map(str::to_string);
                Ok(Some(*target))
            }
            Edit::PlaceWidget { container, props } => self.place_widget(designed, *container, props),
            Edit::CreateWidget { container, kind, col, row, width, caption, bind, part, height } => {
                let mut props = widgets::new_props(*kind, *col, *row);
                if let Some(width) = width {
                    props.width = *width;
                }
                if let Some(height) = height {
                    props.height = *height;
                }
                props.part = part.clone();
                if caption.is_some() {
                    props.caption = caption.clone();
                }
                props.bind = bind.clone();
                self.place_widget(designed, *container, &props)
            }
            Edit::UpdateWidget { item, container, props } => self.update_widget(designed, *item, *container, props),
            Edit::AddTab { item, title } => self.add_tab(designed, *item, title),
            Edit::AddMenu { item } => self.add_menu(designed, *item),
            Edit::AddData { page, name, entity, list } => {
                if *page != designed.start() {
                    return fail("data is declared on the window or the app");
                }
                if !widgets::is_identifier(name) {
                    return fail(format!("'{name}' is not a valid data name"));
                }
                if !widgets::is_identifier(entity) {
                    return fail(format!("'{entity}' is not a valid entity name"));
                }
                let entity = Ident { name: entity.clone(), span: Span::default() };
                let of = if *list { DataKind::List { entity } } else { DataKind::Record { entity } };
                let span = self.fresh();
                designed.data().push(DataDecl { name: Ident { name: name.clone(), span }, of, made: false, span });
                Ok(Some(span.start))
            }
            Edit::SetGrid { page: at, columns, rows } => {
                let page = designed.page("grid of its own")?;
                if page.span.start != *at {
                    return fail("only a window or view has a grid setting");
                }
                if *columns == 0 || *columns > crate::model::MAX_PAGE_COLUMNS {
                    return fail(format!("a page grid has 1 to {} columns", crate::model::MAX_PAGE_COLUMNS));
                }
                let (least, most) = crate::layout::CELL_HEIGHTS;
                if let Some(rows) = rows
                    && !(least..=most).contains(rows)
                {
                    return fail(format!("a page's rows are {least} to {most} pixels high"));
                }
                page.columns = *columns;
                page.cell = *rows;
                Ok(Some(*at))
            }
            Edit::SetSize { page: at, width, height } => {
                let page = designed.page("size")?;
                if page.span.start != *at {
                    return fail("only a window has a size");
                }
                page.size = match width.zip(*height) {
                    None => None,
                    Some((width, height)) if crate::layout::window_size_fits(width, height) => Some(WindowSize { width, height }),
                    Some(_) => return fail(crate::layout::window_size_problem()),
                };
                Ok(Some(*at))
            }
            Edit::SetPageStyle { page: at, style } => {
                let page = designed.page("style")?;
                if page.span.start != *at {
                    return fail("only a window has a style");
                }
                let style = style.trim();
                if !style.is_empty() && !widgets::is_identifier(style) {
                    return fail(format!("'{style}' is not a style name"));
                }
                page.style = (!style.is_empty()).then(|| style.to_string());
                Ok(Some(*at))
            }
            Edit::SetPageAllow { page: at, allow } => {
                let page = designed.page("allow")?;
                if page.span.start != *at {
                    return fail("only a window or view says who may open it");
                }
                let allow = allow.trim();
                page.allow = (!allow.is_empty()).then(|| allow.to_string());
                page.allow_condition = None;
                Ok(Some(*at))
            }
            Edit::PlaceField { container, col, row, room, bind } => self.place_field(designed, *container, *col, *row, *room, bind),
            Edit::NameParts => {
                self.name_parts(designed);
                Ok(None)
            }
            Edit::SetMain { app: at, main } => {
                let app = designed.app()?;
                if app.span.start != *at {
                    return fail("there is no app there");
                }
                let main = main.trim();
                if !main.is_empty() && !widgets::is_identifier(main) {
                    return fail(format!("'{main}' is not a valid window name"));
                }
                app.main = (!main.is_empty()).then(|| Ident { name: main.to_string(), span: Span::default() });
                Ok(Some(*at))
            }
            Edit::SetSplash { app: at, splash, ms } => {
                let app = designed.app()?;
                if app.span.start != *at {
                    return fail("there is no app there");
                }
                let splash = splash.trim();
                if !splash.is_empty() && !widgets::is_identifier(splash) {
                    return fail(format!("'{splash}' is not a valid page name"));
                }
                if let Some(ms) = ms
                    && !(100..=30_000).contains(ms)
                {
                    return fail("a splash shows from 0.1 to 30 seconds");
                }
                app.splash = (!splash.is_empty()).then(|| Ident { name: splash.to_string(), span: Span::default() });
                app.splash_for = if splash.is_empty() { None } else { *ms };
                Ok(Some(*at))
            }
            Edit::SetOwnerEvent { owner, event, handler } => self.set_owner_event(designed, *owner, *event, handler.as_deref()),
            Edit::AddRegion { owner, region, side } => self.add_region(designed, *owner, *region, *side),
            Edit::UpdateSidebar { item, props } => {
                let used = used_names(designed);
                let frame = designed.frame();
                let Some(index) = frame.sidebars.iter().position(|s| s.span.start == *item) else { return fail("there is no sidebar there") };
                if frame.sidebars.iter().enumerate().any(|(i, s)| i != index && s.props.side == props.side) {
                    return fail(format!("there is already a {} sidebar", frame::side_keyword(props.side)));
                }
                let mut props = props.clone();
                if props.name.is_none() {
                    props.name = Some(free_in(&used, &format!("{}Sidebar", frame::side_keyword(props.side))));
                }
                frame.sidebars[index].props = props;
                Ok(Some(*item))
            }
            Edit::UpdateSidebars => {
                let mut used = used_names(designed);
                for sidebar in &mut designed.frame().sidebars {
                    if sidebar.props.name.is_none() {
                        let name = free_in(&used, &format!("{}Sidebar", frame::side_keyword(sidebar.props.side)));
                        used.push(name.clone());
                        sidebar.props.name = Some(name);
                    }
                }
                Ok(None)
            }
            Edit::UpdateRibbon { item, name, collapsible, collapsed } => {
                let used = used_names(designed);
                let Some(ribbon) = designed.frame().ribbon.as_mut().filter(|r| r.span.start == *item) else { return fail("there is no ribbon there") };
                ribbon.name = Some(name.clone().unwrap_or_else(|| free_in(&used, "mainRibbon")));
                ribbon.collapsible = *collapsible;
                ribbon.collapsed = *collapsible && *collapsed;
                Ok(Some(*item))
            }
            Edit::SetName { item, name } => {
                if !crate::names::is_camel_case(name) {
                    return fail(crate::names::camel_case_problem("part", name));
                }
                let frame = designed.frame();
                let slot = if let Some(menu) = frame.menu.as_mut().filter(|m| m.span.start == *item) {
                    &mut menu.name
                } else if let Some(toolbar) = frame.toolbar.as_mut().filter(|t| t.span.start == *item) {
                    &mut toolbar.name
                } else if let Some(tab) = frame.ribbon.iter_mut().flat_map(|r| r.tabs.iter_mut()).find(|t| t.span.start == *item) {
                    &mut tab.name
                } else if let Some(group) = frame.ribbon.iter_mut().flat_map(|r| r.tabs.iter_mut()).flat_map(|t| t.groups.iter_mut()).find(|g| g.span.start == *item) {
                    &mut group.name
                } else {
                    return fail("there is no menu, toolbar, or ribbon tab or group there");
                };
                *slot = Some(name.clone());
                Ok(Some(*item))
            }
            Edit::AddFrameItem { parent, item, caption } => self.add_frame_item(designed, *parent, *item, caption.clone()),
            Edit::UpdateCommand { item, props } => {
                frame::validate_command(props).map_err(EditError)?;
                let used = used_names(designed);
                let mut props = props.clone();
                if props.name.is_none() && props.kind != CommandKind::Separator {
                    props.name = Some(free_in(&used, &crate::names::command_name(props.caption.as_deref(), frame::command_keyword(props.kind))));
                }
                let Some(command) = command_lists(designed).into_iter().find_map(|list| find_command(list, *item)) else { return fail("there is no menu item, button or separator there") };
                if command.props.kind != props.kind {
                    return fail(format!("a {} cannot be turned into a {}", frame::command_keyword(command.props.kind), frame::command_keyword(props.kind)));
                }
                command.props = props;
                Ok(Some(*item))
            }
            Edit::MoveSibling { item, delta } => self.move_sibling(designed, *item, *delta),
            _ => fail("this change belongs in the module's file of Clarity, not in a window's .cxaml"),
        }
    }

    fn remove(&mut self, designed: &mut Designed, item: u32) -> EditResult<Option<u32>> {
        if item == designed.start() {
            return fail("a window or view is removed with its file");
        }
        let data = designed.data();
        if let Some(index) = data.iter().position(|d| d.span.start == item && !d.made) {
            data.remove(index);
            return Ok(None);
        }
        for list in designed.all_widgets() {
            // A splitter has its two panes.
            if find_inner(list, item).is_some_and(|(kind, _)| kind == WidgetKind::Splitter) {
                return fail("a splitter has two panes; remove the splitter instead");
            }
            if remove_widget(list, item).is_some() || remove_inner(list, item) {
                return Ok(None);
            }
        }
        let frame = designed.frame();
        if frame.menu.as_ref().is_some_and(|m| m.span.start == item) {
            frame.menu = None;
            return Ok(None);
        }
        if frame.toolbar.as_ref().is_some_and(|t| t.span.start == item) {
            frame.toolbar = None;
            return Ok(None);
        }
        if frame.ribbon.as_ref().is_some_and(|r| r.span.start == item) {
            frame.ribbon = None;
            return Ok(None);
        }
        if frame.footer.as_ref().is_some_and(|f| f.span.start == item) {
            frame.footer = None;
            return Ok(None);
        }
        if let Some(index) = frame.sidebars.iter().position(|s| s.span.start == item) {
            frame.sidebars.remove(index);
            return Ok(None);
        }
        if let Some(ribbon) = &mut frame.ribbon {
            if let Some(index) = ribbon.tabs.iter().position(|t| t.span.start == item) {
                ribbon.tabs.remove(index);
                return Ok(None);
            }
            for tab in &mut ribbon.tabs {
                if let Some(index) = tab.groups.iter().position(|g| g.span.start == item) {
                    tab.groups.remove(index);
                    return Ok(None);
                }
            }
        }
        // A part's menu, or a menu item or button.
        fn menus(widgets: &mut [Widget], item: u32) -> bool {
            widgets.iter_mut().any(|w| {
                if w.menu.as_ref().is_some_and(|m| m.span.start == item) {
                    w.menu = None;
                    return true;
                }
                menus(&mut w.children, item) || w.pages.iter_mut().any(|p| menus(&mut p.children, item))
            })
        }
        if designed.all_widgets().into_iter().any(|list| menus(list, item)) {
            return Ok(None);
        }
        for list in command_lists(designed) {
            if let Some((siblings, index)) = command_siblings(list, item) {
                siblings.remove(index);
                return Ok(None);
            }
        }
        fail("there is no window, part, tab, region, command or data declaration there")
    }

    fn set_title(&mut self, designed: &mut Designed, target: u32, title: &str) -> EditResult<Option<u32>> {
        let title = title.trim();
        if target == designed.start() {
            let slot = match designed {
                Designed::Page(page) => &mut page.title,
                Designed::App(app) => &mut app.title,
            };
            *slot = (!title.is_empty()).then(|| title.to_string());
            return Ok(Some(target));
        }
        if title.is_empty() {
            return fail("a tab or status needs a title");
        }
        for list in designed.all_widgets() {
            if let Some((kind, inner)) = find_inner(list, target) {
                // A line of a window's content is the view it shows, by name.
                if kind == WidgetKind::Content {
                    return fail("a line of a content shows a view; it has no title");
                }
                if kind == WidgetKind::Splitter {
                    return fail("a splitter's pane has no title");
                }
                inner.title = Some(title.to_string());
                return Ok(Some(target));
            }
        }
        let frame = designed.frame();
        if let Some(footer) = frame.footer.as_mut().filter(|f| f.status_span.is_some_and(|s| s.start == target)) {
            footer.status = Some(title.to_string());
            return Ok(Some(target));
        }
        for tab in frame.ribbon.iter_mut().flat_map(|r| r.tabs.iter_mut()) {
            if tab.span.start == target {
                tab.title = title.to_string();
                return Ok(Some(target));
            }
            if let Some(group) = tab.groups.iter_mut().find(|g| g.span.start == target) {
                group.title = title.to_string();
                return Ok(Some(target));
            }
        }
        fail("there is no page, app, tab or status there")
    }

    fn place_widget(&mut self, designed: &mut Designed, container: u32, props: &WidgetProps) -> EditResult<Option<u32>> {
        widgets::validate_props(props).map_err(EditError)?;
        // A new part gets a name, after what it shows when it says, else like `button3`, so events can be wired.
        let mut props = props.clone();
        if props.name.is_none() {
            let used = used_names(designed);
            props.name = Some(match crate::names::suggested_name(&props) {
                Some(base) => free_in(&used, &base),
                None => numbered_in(&used, &widgets::name_base(&props)),
            });
        }
        let widget = self.widget(props);
        let at = widget.span.start;
        let Some(list) = container_of(designed, container) else { return fail("there is no page, panel, tab or pane there") };
        insert_widget(list, widget);
        Ok(Some(at))
    }

    fn update_widget(&mut self, designed: &mut Designed, item: u32, container: Option<u32>, props: &WidgetProps) -> EditResult<Option<u32>> {
        widgets::validate_props(props).map_err(EditError)?;
        let used = used_names(designed);
        let current = designed.all_widgets().into_iter().find_map(|list| find_widget(list, item).map(|w| w.clone()));
        let Some(current) = current else { return fail("there is no part there") };
        // Every part has a name: one written without gets one now, like a new part.
        let mut props = props.clone();
        if props.name.is_none() {
            props.name = Some(numbered_in(&used, &widgets::name_base(&props)));
        } else if let Some(better) = better_name(designed, self.project, &used, &props) {
            // A numbered part takes a name after its purpose once it knows it.
            props.name = Some(better);
        }
        if widgets::written_word(&current.props) != widgets::written_word(&props) {
            return fail(format!("a {} cannot be turned into a {}", widgets::written_word(&current.props), widgets::written_word(&props)));
        }
        match container {
            None => {
                for list in designed.all_widgets() {
                    if let Some(w) = find_widget(list, item) {
                        w.props = props;
                        break;
                    }
                }
                // Reading order again, as a part moved on its grid.
                sort_holding(designed, item);
            }
            Some(target) => {
                if holds(&current, target) {
                    return fail("a widget cannot be moved into itself");
                }
                if container_of(designed, target).is_none() {
                    return fail("there is no page, panel, tab or pane there");
                }
                let mut moved = designed.all_widgets().into_iter().find_map(|list| remove_widget(list, item)).expect("the part found above");
                moved.props = props;
                let list = container_of(designed, target).expect("the container found above");
                insert_widget(list, moved);
            }
        }
        Ok(Some(item))
    }

    fn add_tab(&mut self, designed: &mut Designed, item: u32, title: &str) -> EditResult<Option<u32>> {
        let title = title.trim();
        let mut inner = self.inner(None);
        let at = inner.span.start;
        let Some(widget) = designed.all_widgets().into_iter().find_map(|list| find_widget(list, item)) else { return fail("there is no tabs or steps part there") };
        let container = widgets::container(widget.props.kind);
        if !matches!(container, Some(ContainerKind::Tabs | ContainerKind::Steps | ContainerKind::Accordion | ContainerKind::Content)) {
            return fail("there is no tabs or steps part there");
        }
        if title.is_empty() {
            return fail("a tab or step needs a title");
        }
        // A window's content gets a view by its name.
        if container == Some(ContainerKind::Content) {
            if !widgets::is_identifier(title) {
                return fail(format!("'{title}' is not the name of a view"));
            }
            inner.view = Some(title.to_string());
        } else {
            inner.title = Some(title.to_string());
        }
        widget.pages.push(inner);
        Ok(Some(at))
    }

    fn add_menu(&mut self, designed: &mut Designed, item: u32) -> EditResult<Option<u32>> {
        let used = used_names(designed);
        let name = std::iter::once("firstItem".to_string()).chain((2..).map(|n| format!("firstItem{n}"))).find(|n| !used.contains(n)).expect("an unused name exists");
        let mut props = frame::new_command(CommandKind::Item, Some("First item".into()));
        props.name = Some(name);
        let first = self.command(props);
        let at = first.span.start;
        let menu = Menu { name: None, items: vec![first], span: self.fresh() };
        let Some(widget) = designed.all_widgets().into_iter().find_map(|list| find_widget(list, item)) else { return fail("there is no part there") };
        if widget.menu.is_some() {
            return fail("the part has a menu already");
        }
        widget.menu = Some(menu);
        Ok(Some(at))
    }

    /// A field of the window's data with the parts that go with it (see
    /// `templates::field_parts`); the input is named after the field.
    fn place_field(&mut self, designed: &mut Designed, container: u32, col: u32, row: u32, room: u32, bind: &str) -> EditResult<Option<u32>> {
        let Some(project) = self.project else { return fail("this edit needs the whole project") };
        let Some((data, field_name)) = bind.split_once('.') else { return fail(format!("'{bind}' is not a field, use data.field")) };
        let entity = designed.data().iter().find(|d| d.name.name == data).and_then(|d| match &d.of {
            DataKind::Record { entity } | DataKind::List { entity } => Some(entity.name.clone()),
            DataKind::Value { .. } => None,
        });
        let Some(entity) = entity else { return fail(format!("the window has no data '{data}'")) };
        let field = project.entity(&entity).and_then(|e| e.field(field_name)).ok_or_else(|| EditError(format!("'{entity}' has no field '{field_name}'")))?;
        let base = crate::names::camel_case(field_name);
        let used = used_names(designed);
        let input = if used.contains(&base) { numbered_in(&used, &base) } else { base };
        let parts = crate::templates::field_parts(field, data, &input, col, row, room.max(1));
        let mut focus = None;
        for props in &parts {
            let at = self.place_widget(designed, container, props)?;
            focus = focus.or(at);
        }
        Ok(focus)
    }

    /// A name for each part, region, item and ribbon tab or group without one.
    fn name_parts(&mut self, designed: &mut Designed) {
        let mut used = used_names(designed);
        fn give(used: &mut Vec<String>, name: String) -> String {
            used.push(name.clone());
            name
        }
        /// `base` or the first free numbered one, given out.
        fn take(used: &mut Vec<String>, base: &str) -> String {
            let name = free_in(used, base);
            give(used, name)
        }
        fn commands(items: &mut [Command], used: &mut Vec<String>) {
            for c in items {
                if c.props.name.is_none() && c.props.kind != CommandKind::Separator {
                    let base = crate::names::command_name(c.props.caption.as_deref(), frame::command_keyword(c.props.kind));
                    c.props.name = Some(give(used, free_in(used, &base)));
                }
                commands(&mut c.items, used);
            }
        }
        fn parts(widgets: &mut [Widget], used: &mut Vec<String>) {
            for w in widgets {
                if w.props.name.is_none() {
                    let name = match crate::names::suggested_name(&w.props) {
                        Some(base) => free_in(used, &base),
                        None => numbered_in(used, &widgets::name_base(&w.props)),
                    };
                    w.props.name = Some(give(used, name));
                }
                if let Some(menu) = &mut w.menu {
                    commands(&mut menu.items, used);
                }
                parts(&mut w.children, used);
                for page in &mut w.pages {
                    parts(&mut page.children, used);
                }
            }
        }
        let frame = designed.frame();
        if let Some(menu) = &mut frame.menu {
            if menu.name.is_none() {
                menu.name = Some(take(&mut used, "mainMenu"));
            }
            commands(&mut menu.items, &mut used);
        }
        if let Some(toolbar) = &mut frame.toolbar {
            if toolbar.name.is_none() {
                toolbar.name = Some(take(&mut used, "mainToolbar"));
            }
            commands(&mut toolbar.items, &mut used);
        }
        if let Some(ribbon) = &mut frame.ribbon {
            if ribbon.name.is_none() {
                ribbon.name = Some(take(&mut used, "mainRibbon"));
            }
            for tab in &mut ribbon.tabs {
                if tab.name.is_none() {
                    tab.name = Some(take(&mut used, &ribbon_part_base(&tab.title, "Tab")));
                }
                for group in &mut tab.groups {
                    if group.name.is_none() {
                        group.name = Some(take(&mut used, &ribbon_part_base(&group.title, "Group")));
                    }
                    commands(&mut group.items, &mut used);
                }
            }
        }
        for sidebar in &mut frame.sidebars {
            if sidebar.props.name.is_none() {
                sidebar.props.name = Some(take(&mut used, &format!("{}Sidebar", frame::side_keyword(sidebar.props.side))));
            }
            parts(&mut sidebar.widgets, &mut used);
        }
        if let Some(footer) = &mut frame.footer {
            parts(&mut footer.widgets, &mut used);
        }
        if let Some(widgets) = designed.widgets() {
            parts(widgets, &mut used);
        }
    }

    fn set_owner_event(&mut self, designed: &mut Designed, owner: u32, event: Event, handler: Option<&str>) -> EditResult<Option<u32>> {
        if owner != designed.start() {
            return fail("there is no window or app there");
        }
        let app = matches!(designed, Designed::App(_));
        let allowed: &[Event] = if app { &widgets::APP_EVENTS } else { &widgets::WINDOW_EVENTS };
        if !allowed.contains(&event) {
            return fail(if app {
                format!("the app has the event start, not {}", event.keyword())
            } else {
                format!("a window has the events open, close and return, not {}", event.keyword())
            });
        }
        let handler = handler.map(str::trim).filter(|h| !h.is_empty());
        if let Some(h) = handler
            && !widgets::is_identifier(h)
        {
            return fail(format!("'{h}' is not a valid handler name"));
        }
        let handlers = designed.handlers();
        let existing = handlers.iter().position(|h| h.event == event);
        match (existing, handler) {
            (Some(index), None) => {
                handlers.remove(index);
            }
            (Some(index), Some(h)) => handlers[index].handler = h.to_string(),
            (None, Some(h)) => handlers.push(Handler { event, handler: h.to_string(), args: Vec::new() }),
            (None, None) => {}
        }
        Ok(Some(owner))
    }

    fn add_region(&mut self, designed: &mut Designed, owner: u32, region: RegionKind, side: Option<Side>) -> EditResult<Option<u32>> {
        if owner != designed.start() {
            return fail("there is no window or app there");
        }
        if region != RegionKind::Sidebar && side.is_some() {
            return fail("only a sidebar has a side");
        }
        let used = used_names(designed);
        let frame_now = designed.frame_ref();
        let taken = match region {
            RegionKind::Menu => frame_now.menu.is_some(),
            RegionKind::Toolbar => frame_now.toolbar.is_some(),
            RegionKind::Ribbon => frame_now.ribbon.is_some(),
            RegionKind::Footer => frame_now.footer.is_some(),
            RegionKind::Sidebar => frame_now.sidebars.iter().any(|s| s.props.side == side.unwrap_or(Side::Left)),
        };
        if taken {
            let what = match region {
                RegionKind::Sidebar => format!("a {} sidebar", frame::side_keyword(side.unwrap_or(Side::Left))),
                other => format!("a {}", frame::keyword(other)),
            };
            return fail(format!("this window already has {what}"));
        }
        // Starter content, so the new region shows up and is ready to edit; every
        // starter command named, as the checker asks of every command.
        let span = self.fresh();
        let named = |kind: CommandKind, caption: &str| {
            let mut props = frame::new_command(kind, Some(caption.into()));
            props.name = Some(free_in(&used, &crate::names::command_name(props.caption.as_deref(), frame::command_keyword(props.kind))));
            props
        };
        match region {
            RegionKind::Menu => {
                let mut file = self.command(named(CommandKind::Item, "File"));
                let mut quit = named(CommandKind::Item, "Quit");
                quit.on_click = Some("quit".into());
                file.items.push(self.command(quit));
                designed.frame().menu = Some(Menu { name: Some(free_in(&used, "mainMenu")), items: vec![file], span });
            }
            RegionKind::Toolbar => {
                let button = self.command(named(CommandKind::Button, "Button"));
                designed.frame().toolbar = Some(Toolbar { name: Some(free_in(&used, "mainToolbar")), items: vec![button], span });
            }
            RegionKind::Ribbon => {
                let button = self.command(named(CommandKind::Button, "Button"));
                let group = RibbonGroup { title: "Actions".into(), name: Some(free_in(&used, "actionsGroup")), items: vec![button], span: self.fresh() };
                let tab = RibbonTab { title: "Home".into(), name: Some(free_in(&used, "homeTab")), groups: vec![group], span: self.fresh() };
                designed.frame().ribbon = Some(Ribbon { name: Some(free_in(&used, "mainRibbon")), collapsible: false, collapsed: false, tabs: vec![tab], span });
            }
            RegionKind::Footer => {
                let status = self.fresh();
                designed.frame().footer = Some(Footer { status: Some("Ready".into()), status_span: Some(status), columns: 12, widgets: Vec::new(), span });
            }
            RegionKind::Sidebar => {
                let side = side.unwrap_or(Side::Left);
                let props = frame::new_sidebar(side, Some(free_in(&used, &format!("{}Sidebar", frame::side_keyword(side)))));
                designed.frame().sidebars.push(Sidebar { props, widgets: Vec::new(), span });
            }
        }
        Ok(Some(span.start))
    }

    fn add_frame_item(&mut self, designed: &mut Designed, parent: u32, item: FrameItem, caption: Option<String>) -> EditResult<Option<u32>> {
        let caption = caption.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
        let used = used_names(designed);
        let command_props = |kind: CommandKind, caption: Option<String>| -> EditResult<CommandProps> {
            let caption = match kind {
                CommandKind::Separator => None,
                CommandKind::Item => caption.or(Some("Item".into())),
                CommandKind::Button => caption.or(Some("Button".into())),
            };
            let mut props = frame::new_command(kind, caption);
            // A new item or button is named after its caption, like a new part.
            if kind != CommandKind::Separator {
                props.name = Some(free_in(&used, &crate::names::command_name(props.caption.as_deref(), frame::command_keyword(kind))));
            }
            frame::validate_command(&props).map_err(EditError)?;
            Ok(props)
        };
        let kind_of = |item: FrameItem, buttons: bool| match (item, buttons) {
            (FrameItem::Separator, _) => Some(CommandKind::Separator),
            (FrameItem::Item, false) => Some(CommandKind::Item),
            (FrameItem::Button, true) => Some(CommandKind::Button),
            _ => None,
        };
        let frame_now = designed.frame_ref();
        let in_menu = frame_now.menu.as_ref().is_some_and(|m| m.span.start == parent);
        let in_toolbar = frame_now.toolbar.as_ref().is_some_and(|t| t.span.start == parent);
        let in_ribbon = frame_now.ribbon.as_ref().is_some_and(|r| r.span.start == parent);
        let in_tab = frame_now.ribbon.iter().flat_map(|r| &r.tabs).any(|t| t.span.start == parent);
        let in_group = frame_now.ribbon.iter().flat_map(|r| &r.tabs).flat_map(|t| &t.groups).any(|g| g.span.start == parent);
        let frame = designed.frame();
        if in_menu || in_toolbar || in_group {
            let Some(kind) = kind_of(item, !in_menu) else { return fail("that cannot go there") };
            let command = self.command(command_props(kind, caption)?);
            let at = command.span.start;
            let items = if in_menu {
                &mut frame.menu.as_mut().expect("the menu found above").items
            } else if in_toolbar {
                &mut frame.toolbar.as_mut().expect("the toolbar found above").items
            } else {
                &mut frame.ribbon.iter_mut().flat_map(|r| r.tabs.iter_mut()).flat_map(|t| t.groups.iter_mut()).find(|g| g.span.start == parent).expect("the group found above").items
            };
            items.push(command);
            return Ok(Some(at));
        }
        if in_ribbon {
            if item != FrameItem::Tab {
                return fail("that cannot go there");
            }
            let title = caption.unwrap_or_else(|| "Tab".into());
            let tab = RibbonTab { name: Some(free_in(&used, &ribbon_part_base(&title, "Tab"))), title, groups: Vec::new(), span: self.fresh() };
            let at = tab.span.start;
            frame.ribbon.as_mut().expect("the ribbon found above").tabs.push(tab);
            return Ok(Some(at));
        }
        if in_tab {
            if item != FrameItem::Group {
                return fail("that cannot go there");
            }
            let title = caption.unwrap_or_else(|| "Group".into());
            let group = RibbonGroup { name: Some(free_in(&used, &ribbon_part_base(&title, "Group"))), title, items: Vec::new(), span: self.fresh() };
            let at = group.span.start;
            frame.ribbon.iter_mut().flat_map(|r| r.tabs.iter_mut()).find(|t| t.span.start == parent).expect("the tab found above").groups.push(group);
            return Ok(Some(at));
        }
        // A submenu item of a menu item, in the frame's menu or a part's.
        let Some(kind) = kind_of(item, false) else { return fail("that cannot go there") };
        let props = command_props(kind, caption)?;
        let span = self.fresh();
        for list in command_lists(designed) {
            if let Some(command) = find_command(list, parent) {
                if command.props.kind != CommandKind::Item {
                    return fail("only a menu item can hold a submenu");
                }
                command.items.push(Command { props, items: Vec::new(), span });
                return Ok(Some(span.start));
            }
        }
        fail("there is no menu, toolbar, ribbon or part of one there")
    }

    fn move_sibling(&mut self, designed: &mut Designed, item: u32, delta: i32) -> EditResult<Option<u32>> {
        if delta != -1 && delta != 1 {
            return fail("move by one place at a time");
        }
        fn swap<T>(list: &mut [T], index: usize, delta: i32) {
            let other = index as i64 + delta as i64;
            if other >= 0 && (other as usize) < list.len() {
                list.swap(index, other as usize);
            }
        }
        for list in command_lists(designed) {
            if let Some((siblings, index)) = command_siblings(list, item) {
                swap(siblings, index, delta);
                return Ok(Some(item));
            }
        }
        let frame = designed.frame();
        if let Some(index) = frame.sidebars.iter().position(|s| s.span.start == item) {
            swap(&mut frame.sidebars, index, delta);
            return Ok(Some(item));
        }
        if let Some(ribbon) = &mut frame.ribbon {
            if let Some(index) = ribbon.tabs.iter().position(|t| t.span.start == item) {
                swap(&mut ribbon.tabs, index, delta);
                return Ok(Some(item));
            }
            for tab in &mut ribbon.tabs {
                if let Some(index) = tab.groups.iter().position(|g| g.span.start == item) {
                    swap(&mut tab.groups, index, delta);
                    return Ok(Some(item));
                }
            }
        }
        // A region has its place in the frame by its kind: moving it changes nothing.
        let regions = [frame.menu.as_ref().map(|m| m.span.start), frame.toolbar.as_ref().map(|t| t.span.start), frame.ribbon.as_ref().map(|r| r.span.start), frame.footer.as_ref().map(|f| f.span.start)];
        if regions.contains(&Some(item)) {
            return Ok(Some(item));
        }
        fail("there is no command, ribbon tab, ribbon group or region there")
    }
}

/// The list holding the part at `item`, put back in reading order.
fn sort_holding(designed: &mut Designed, item: u32) {
    fn sort(widgets: &mut Vec<Widget>, item: u32) -> bool {
        if let Some(index) = widgets.iter().position(|w| w.span.start == item) {
            let moved = widgets.remove(index);
            insert_widget(widgets, moved);
            return true;
        }
        widgets.iter_mut().any(|w| sort(&mut w.children, item) || w.pages.iter_mut().any(|p| sort(&mut p.children, item)))
    }
    for list in designed.all_widgets() {
        if sort(list, item) {
            return;
        }
    }
}

/// A name after its purpose for a part that still has its numbered name
/// (`label1`), when it knows its purpose and nothing else in its window
/// goes by the number.
fn better_name(designed: &Designed, project: Option<&Project>, used: &[String], props: &WidgetProps) -> Option<String> {
    let name = props.name.as_deref()?;
    let kind = widgets::name_base(props);
    let numbered = name.strip_prefix(kind.as_str()).is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    if !numbered || mentioned(designed, project, name) {
        return None;
    }
    // A name already numbered from what it would be named after stays.
    let base = crate::names::suggested_name(props)?;
    if name.strip_prefix(base.as_str()).is_some_and(|n| n.chars().all(|c| c.is_ascii_digit())) {
        return None;
    }
    Some(free_in(used, &base))
}

/// Whether anything of the window says `name` other than where a part is named it.
/// The window's code is in its module's file: with the project, its code says it too.
fn mentioned(designed: &Designed, project: Option<&Project>, name: &str) -> bool {
    let Designed::Page(page) = designed else { return false };
    let written = crate::cxaml::page_source(page, "");
    let quoted = format!("\"{name}\"");
    let named = format!("name={quoted}");
    let in_markup = written.matches(&quoted).count() > written.matches(&named).count();
    let in_code = project.is_some_and(|project| {
        project.files.iter().flat_map(|f| &f.code).filter(|b| b.name.name == page.name.name).flat_map(|b| &b.handlers).any(|h| crate::rename::said_in(&h.body, name))
    });
    in_markup || in_code
}
