//! Semantic checks across all spec files of a project: name resolution,
//! module visibility, modifier validity, and windows with their widgets
//! and frame regions.

use std::collections::{HashMap, HashSet};

use crate::diagnostic::Diagnostic;
use crate::frame;
use crate::model::{
    App, Command, CommandKind, DataDecl, Entity, Field, FieldType, Frame, Ident, Project,
    Literal, MAX_PAGE_COLUMNS, Page, PageKind, SpecFile, Widget,
    WidgetProps,
};
use crate::span::Span;
use crate::widgets::{self, BindTarget, ContainerKind, WidgetKind};

/// Data declarations of the page being checked: name -> (entity, is list).
/// The entity is `None` when it did not resolve; that error is reported once.
struct PageContext<'a> {
    data: HashMap<&'a str, (Option<&'a Entity>, bool)>,
}

fn overlaps(a: &WidgetProps, b: &WidgetProps) -> bool {
    a.col < b.col + b.width
        && b.col < a.col + a.width
        && a.row < b.row + b.height
        && b.row < a.row + a.height
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DeclKind {
    Enum,
    Entity,
    Page,
    App,
}

impl DeclKind {
    fn noun(self) -> &'static str {
        match self {
            Self::Enum => "enum",
            Self::Entity => "entity",
            Self::Page => "page",
            Self::App => "app",
        }
    }
}

/// Where a declaration lives.
struct Decl<'a> {
    kind: DeclKind,
    module: &'a str,
    file: usize,
    index: usize,
}

struct Checker<'a> {
    files: &'a [SpecFile],
    /// Module name -> file index.
    modules: HashMap<&'a str, usize>,
    /// Declaration name -> all declarations with that name.
    decls: HashMap<&'a str, Vec<Decl<'a>>>,
    diagnostics: Vec<Diagnostic>,
}

pub fn check(files: &[SpecFile]) -> Vec<Diagnostic> {
    let mut checker = Checker {
        files,
        modules: HashMap::new(),
        decls: HashMap::new(),
        diagnostics: Vec::new(),
    };
    checker.collect();
    checker.check_single_app();
    for (file_index, file) in files.iter().enumerate() {
        checker.check_file(file_index, file);
    }
    checker.diagnostics.extend(crate::code::check::check(files));
    checker.diagnostics
}

impl<'a> Checker<'a> {
    fn error(&mut self, file: usize, message: impl Into<String>, span: Span) {
        let path = &self.files[file].path;
        self.diagnostics
            .push(Diagnostic::error(message, span).in_file(path));
    }

    fn warning(&mut self, file: usize, message: impl Into<String>, span: Span) {
        let path = &self.files[file].path;
        self.diagnostics
            .push(Diagnostic::warning(message, span).in_file(path));
    }

    fn module_name(&self, file: usize) -> &'a str {
        self.files[file]
            .module
            .as_ref()
            .map(|m| m.name.as_str())
            .unwrap_or("")
    }

    // -- collection -------------------------------------------------------

    fn collect(&mut self) {
        let files = self.files;
        for (file_index, file) in files.iter().enumerate() {
            if let Some(module) = &file.module {
                if let Some(&other) = self.modules.get(module.name.as_str()) {
                    let other_path = files[other].path.clone();
                    self.error(
                        file_index,
                        format!(
                            "module '{}' is already declared in {other_path}",
                            module.name
                        ),
                        module.span,
                    );
                } else {
                    self.modules.insert(&module.name, file_index);
                }
            }

            let module = self.module_name(file_index);
            let names = file
                .enums
                .iter()
                .enumerate()
                .map(|(i, e)| (DeclKind::Enum, i, &e.name))
                .chain(
                    file.entities
                        .iter()
                        .enumerate()
                        .map(|(i, e)| (DeclKind::Entity, i, &e.name)),
                )
                .chain(
                    file.pages
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (DeclKind::Page, i, &p.name)),
                )
                .chain(
                    file.apps
                        .iter()
                        .enumerate()
                        .map(|(i, a)| (DeclKind::App, i, &a.name)),
                );
            for (kind, index, name) in names {
                let existing = self.decls.entry(&name.name).or_default();
                if existing.iter().any(|d| d.module == module) {
                    let message =
                        format!("'{}' is already declared in module '{module}'", name.name);
                    self.error(file_index, message, name.span);
                    continue;
                }
                existing.push(Decl {
                    kind,
                    module,
                    file: file_index,
                    index,
                });
            }
        }
    }

    // -- resolution -------------------------------------------------------

    /// Modules whose declarations are visible from a file: its own and the
    /// ones it uses.
    fn visible_modules(&self, file: usize) -> HashSet<&'a str> {
        let mut visible: HashSet<&str> = self.files[file]
            .uses
            .iter()
            .map(|u| u.name.as_str())
            .collect();
        visible.insert(self.module_name(file));
        visible
    }

    /// Resolves a name to a declaration of the expected kind, reporting
    /// unknown, ambiguous, invisible and wrongly-kinded names.
    fn resolve(&mut self, file: usize, name: &Ident, kind: DeclKind) -> Option<(usize, usize)> {
        let visible = self.visible_modules(file);
        let candidates: Vec<(DeclKind, &str, usize, usize)> = self
            .decls
            .get(name.name.as_str())
            .map(|ds| {
                ds.iter()
                    .map(|d| (d.kind, d.module, d.file, d.index))
                    .collect()
            })
            .unwrap_or_default();

        let in_scope: Vec<_> = candidates
            .iter()
            .filter(|(_, module, _, _)| visible.contains(module))
            .collect();

        match in_scope.as_slice() {
            [] => {
                let message = match candidates.first() {
                    Some((_, module, _, _)) => format!(
                        "'{}' is declared in module '{module}', add 'use {module}'",
                        name.name
                    ),
                    None => format!("unknown {} '{}'", kind.noun(), name.name),
                };
                self.error(file, message, name.span);
                None
            }
            [(found, module, decl_file, index)] => {
                if *found != kind {
                    let message = format!(
                        "'{}' is {} {} (in module '{module}'), expected {} {}",
                        name.name,
                        article(found.noun()),
                        found.noun(),
                        article(kind.noun()),
                        kind.noun()
                    );
                    self.error(file, message, name.span);
                    return None;
                }
                Some((*decl_file, *index))
            }
            many => {
                let modules: Vec<_> = many.iter().map(|(_, m, _, _)| *m).collect();
                let message = format!(
                    "'{}' is ambiguous, it is declared in modules {}",
                    name.name,
                    modules.join(", ")
                );
                self.error(file, message, name.span);
                None
            }
        }
    }

    // -- per file ---------------------------------------------------------

    fn check_file(&mut self, file_index: usize, file: &'a SpecFile) {
        for used in &file.uses {
            if !self.modules.contains_key(used.name.as_str()) {
                self.error(
                    file_index,
                    format!("unknown module '{}'", used.name),
                    used.span,
                );
            }
            if Some(&used.name) == file.module.as_ref().map(|m| &m.name) {
                self.error(file_index, "a module cannot use itself", used.span);
            }
        }

        for decl in &file.enums {
            let mut seen = HashSet::new();
            for variant in &decl.variants {
                if !seen.insert(variant.name.name.as_str()) {
                    self.error(
                        file_index,
                        format!("duplicate variant '{}'", variant.name.name),
                        variant.name.span,
                    );
                }
            }
        }

        for entity in &file.entities {
            let mut seen = HashSet::new();
            for field in &entity.fields {
                if !seen.insert(field.name.name.as_str()) {
                    self.error(
                        file_index,
                        format!("duplicate field '{}'", field.name.name),
                        field.name.span,
                    );
                }
                self.check_field(file_index, field);
            }
        }

        for page in &file.pages {
            self.check_page(file_index, page);
        }

        for app in &file.apps {
            self.check_app(file_index, app);
        }
    }

    // -- pages ------------------------------------------------------------

    fn check_page(&mut self, file: usize, page: &'a Page) {
        if page.columns == 0 || page.columns > MAX_PAGE_COLUMNS {
            self.error(
                file,
                format!("a page grid has 1 to {MAX_PAGE_COLUMNS} columns"),
                page.name.span,
            );
            return;
        }

        let context = self.data_context(file, &page.data);
        self.check_widgets(file, &context, &page.widgets, page.columns);
        if page.kind != PageKind::Window && !page.frame.is_empty() {
            let kind = if page.kind == PageKind::Dialog {
                "dialog"
            } else {
                "view"
            };
            self.error(
                file,
                format!("only windows have menus, toolbars, ribbons, sidebars and footers; a {kind} has none"),
                page.name.span,
            );
        }
        self.check_frame(file, &context, &page.frame);
        self.check_names(file, &page.widgets, &page.frame);
        self.check_forms(file, &page.widgets, None);
        for sidebar in &page.frame.sidebars {
            self.check_forms(file, &sidebar.widgets, None);
        }
    }

    fn data_context(&mut self, file: usize, decls: &'a [DataDecl]) -> PageContext<'a> {
        let mut data: HashMap<&str, (Option<&'a Entity>, bool)> = HashMap::new();
        for decl in decls {
            let entity = self
                .resolve(file, &decl.entity, DeclKind::Entity)
                .map(|(f, i)| &self.files[f].entities[i]);
            if data.insert(&decl.name.name, (entity, decl.list)).is_some() {
                self.error(
                    file,
                    format!("duplicate data '{}'", decl.name.name),
                    decl.name.span,
                );
            }
        }
        PageContext { data }
    }

    // -- apps and frames --------------------------------------------------

    fn check_single_app(&mut self) {
        let files = self.files;
        let mut first: Option<&str> = None;
        for (file, f) in files.iter().enumerate() {
            for app in &f.apps {
                match first {
                    None => first = Some(&app.name.name),
                    Some(name) => self.error(
                        file,
                        format!("a project has one app; '{name}' is already declared"),
                        app.name.span,
                    ),
                }
            }
        }
    }

    fn check_app(&mut self, file: usize, app: &'a App) {
        match &app.main {
            None => self.warning(
                file,
                format!("app '{}' has no 'main' window yet", app.name.name),
                app.name.span,
            ),
            Some(main) => self.check_main(file, main),
        }
        let context = self.data_context(file, &app.data);
        self.check_frame(file, &context, &app.frame);
        self.check_names(file, &[], &app.frame);
    }

    /// The main window is a page of kind window.
    fn check_main(&mut self, file: usize, main: &Ident) {
        let visible = self.visible_modules(file);
        let found = self.decls.get(main.name.as_str()).and_then(|ds| {
            ds.iter()
                .find(|d| {
                    visible.contains(d.module)
                        && d.kind == DeclKind::Page
                })
                .map(|d| (d.kind, d.file, d.index))
        });
        match found {
            None => self.error(
                file,
                format!("'{}' is not a window", main.name),
                main.span,
            ),
            Some((DeclKind::Page, f, i)) if self.files[f].pages[i].kind != PageKind::Window => self
                .error(
                    file,
                    format!(
                        "the main window '{}' must be a page of kind window",
                        main.name
                    ),
                    main.span,
                ),
            Some(_) => {}
        }
    }

    /// Part names are unique in a window, its sidebars and its footer
    /// included: the code behind refers to parts by name.
    fn check_names(&mut self, file: usize, widgets: &[Widget], frame: &Frame) {
        fn collect<'w>(widgets: &'w [Widget], out: &mut Vec<&'w Widget>) {
            for w in widgets {
                out.push(w);
                collect(&w.children, out);
                for inner in &w.pages {
                    collect(&inner.children, out);
                }
            }
        }
        let mut all = Vec::new();
        collect(widgets, &mut all);
        for sidebar in &frame.sidebars {
            collect(&sidebar.widgets, &mut all);
        }
        if let Some(footer) = &frame.footer {
            collect(&footer.widgets, &mut all);
        }
        fn commands<'c>(items: &'c [Command], out: &mut Vec<(&'c str, Span)>) {
            for c in items {
                if let Some(name) = &c.props.name {
                    out.push((name, c.span));
                }
                commands(&c.items, out);
            }
        }
        let mut named: Vec<(&str, Span)> = all
            .iter()
            .filter_map(|w| w.props.name.as_deref().map(|n| (n, w.span)))
            .collect();
        if let Some(menu) = &frame.menu {
            commands(&menu.items, &mut named);
        }
        if let Some(toolbar) = &frame.toolbar {
            commands(&toolbar.items, &mut named);
        }
        if let Some(ribbon) = &frame.ribbon {
            for group in ribbon.tabs.iter().flat_map(|t| &t.groups) {
                commands(&group.items, &mut named);
            }
        }
        let mut seen = HashSet::new();
        for (name, span) in named {
            if !seen.insert(name) {
                self.error(file, format!("duplicate part name '{name}'"), span);
            }
        }

        // A label or help text belongs to an input of the same window.
        for widget in &all {
            let p = &widget.props;
            let Some(target) = &p.for_part else { continue };
            let at = format!("{} at {},{}", widgets::keyword(p.kind), p.col, p.row);
            match all.iter().find(|w| w.props.name.as_deref() == Some(target.as_str())) {
                None => self.error(
                    file,
                    format!("{at}: there is no part '{target}' in this window"),
                    widget.span,
                ),
                Some(other) if !widgets::info(other.props.kind).input => self.error(
                    file,
                    format!(
                        "{at}: '{target}' is a {}, not an input",
                        widgets::keyword(other.props.kind)
                    ),
                    widget.span,
                ),
                Some(_) => {}
            }
        }

        // Enter clicks the default button and Escape the cancel button, so
        // a window has at most one of each.
        let defaults: Vec<&&Widget> = all.iter().filter(|w| w.props.default_button).collect();
        if let Some(extra) = defaults.get(1) {
            self.error(file, "more than one default button in this window", extra.span);
        }
        let cancels: Vec<&&Widget> = all.iter().filter(|w| w.props.cancel).collect();
        if let Some(extra) = cancels.get(1) {
            self.error(file, "more than one cancel button in this window", extra.span);
        }
    }

    /// A form holds the inputs of the one record it binds, and no other
    /// form.
    fn check_forms(&mut self, file: usize, widgets: &[Widget], form: Option<&Widget>) {
        for widget in widgets {
            let p = &widget.props;
            let at = format!("{} at {},{}", widgets::keyword(p.kind), p.col, p.row);
            let mut inner = form;
            if p.kind == WidgetKind::Form {
                if form.is_some() {
                    self.error(file, format!("{at}: a form cannot hold another form"), widget.span);
                }
                inner = Some(widget);
            } else if let (Some(form), Some(bind)) = (form, &p.bind)
                && widgets::info(p.kind).input
            {
                let data = bind.split('.').next().unwrap_or_default();
                let held = form.props.bind.as_deref().unwrap_or_default();
                let held = held.split('.').next().unwrap_or_default();
                if data != held {
                    let which = match &form.props.name {
                        Some(name) => format!("'{name}'"),
                        None => format!("at {},{}", form.props.col, form.props.row),
                    };
                    self.error(
                        file,
                        format!("{at}: binds to '{data}', but its form {which} holds '{held}'"),
                        widget.span,
                    );
                }
            }
            self.check_forms(file, &widget.children, inner);
            for page in &widget.pages {
                self.check_forms(file, &page.children, inner);
            }
        }
    }

    fn check_frame(&mut self, file: usize, context: &PageContext<'a>, frame: &'a Frame) {
        if let Some(menu) = &frame.menu {
            if menu.items.is_empty() {
                self.warning(file, "the menu has no items yet", menu.span);
            }
            self.check_commands(file, &menu.items);
        }
        if let Some(toolbar) = &frame.toolbar {
            if toolbar.items.is_empty() {
                self.warning(file, "the toolbar has no buttons yet", toolbar.span);
            }
            self.check_commands(file, &toolbar.items);
        }
        if let Some(ribbon) = &frame.ribbon {
            if ribbon.tabs.is_empty() {
                self.warning(file, "the ribbon has no tabs yet", ribbon.span);
            }
            for tab in &ribbon.tabs {
                for group in &tab.groups {
                    self.check_commands(file, &group.items);
                }
            }
        }
        for sidebar in &frame.sidebars {
            if sidebar.width == 0 || sidebar.width > MAX_PAGE_COLUMNS {
                self.error(
                    file,
                    format!("a sidebar is 1 to {MAX_PAGE_COLUMNS} columns wide"),
                    sidebar.span,
                );
            } else {
                self.check_widgets(file, context, &sidebar.widgets, sidebar.width);
            }
        }
        if let Some(footer) = &frame.footer {
            self.check_widgets(file, context, &footer.widgets, footer.columns);
            for widget in &footer.widgets {
                let p = &widget.props;
                if p.row != 1 || p.height != 1 {
                    self.error(
                        file,
                        format!(
                            "{} at {},{}: a footer has one row",
                            widgets::keyword(p.kind),
                            p.col,
                            p.row
                        ),
                        widget.span,
                    );
                }
            }
        }
    }

    fn check_commands(&mut self, file: usize, commands: &[Command]) {
        for command in commands {
            let p = &command.props;
            if p.kind == CommandKind::Item && !command.items.is_empty() && p.on_click.is_some() {
                self.error(
                    file,
                    "an item with a submenu opens the submenu; remove its 'on click'",
                    command.span,
                );
            }
            if let Some(target) = &p.on_click
                && p.new_window
                && frame::BUILT_IN_COMMANDS.contains(&target.as_str())
            {
                self.error(file, "only a window opens in a new window", command.span);
            }
            if let Some(target) = &p.on_click
                && !frame::BUILT_IN_COMMANDS.contains(&target.as_str())
            {
                self.check_click_target(file, target, command.span);
            }
            self.check_commands(file, &command.items);
        }
    }

    fn check_widgets(
        &mut self,
        file: usize,
        page: &PageContext<'a>,
        widgets: &[Widget],
        columns: u32,
    ) {
        for (index, widget) in widgets.iter().enumerate() {
            let p = &widget.props;
            let name = widgets::keyword(p.kind);
            if p.col == 0 || p.row == 0 || p.width == 0 || p.height == 0 {
                self.error(
                    file,
                    format!(
                        "{name} at {},{}: position and size start at 1",
                        p.col, p.row
                    ),
                    widget.span,
                );
            } else if p.col + p.width - 1 > columns {
                self.error(
                    file,
                    format!(
                        "{name} at {},{} span {} runs past column {columns}",
                        p.col, p.row, p.width
                    ),
                    widget.span,
                );
            }
            for other in &widgets[..index] {
                if overlaps(&other.props, p) {
                    self.error(
                        file,
                        format!(
                            "{name} at {},{} overlaps the {} at {},{}",
                            p.col,
                            p.row,
                            widgets::keyword(other.props.kind),
                            other.props.col,
                            other.props.row
                        ),
                        widget.span,
                    );
                }
            }
            self.check_binding(file, page, widget);
            if let Some(target) = &p.on_click {
                if p.new_window && frame::BUILT_IN_COMMANDS.contains(&target.as_str()) {
                    self.error(file, "only a window opens in a new window", widget.span);
                }
                self.check_click_target(file, target, widget.span);
            }

            match widgets::container(p.kind) {
                None => {}
                Some(ContainerKind::Plain) => {
                    self.check_widgets(file, page, &widget.children, p.width)
                }
                Some(container) => {
                    match container {
                        ContainerKind::Tabs if widget.pages.is_empty() => {
                            self.error(file, "tabs need at least one 'tab'", widget.span);
                        }
                        ContainerKind::Splitter if widget.pages.len() != 2 => {
                            self.error(
                                file,
                                "a splitter has exactly two 'pane' blocks",
                                widget.span,
                            );
                        }
                        _ => {}
                    }
                    for inner in &widget.pages {
                        self.check_widgets(file, page, &inner.children, p.width);
                    }
                }
            }

            if p.kind == WidgetKind::Canvas && p.renderer.is_none() {
                self.warning(
                    file,
                    format!("canvas at {},{} has no renderer yet", p.col, p.row),
                    widget.span,
                );
            }
            if p.kind == WidgetKind::Image && p.source.is_none() {
                self.warning(
                    file,
                    format!("image at {},{} has no source yet", p.col, p.row),
                    widget.span,
                );
            }
        }
    }

    fn check_binding(&mut self, file: usize, page: &PageContext<'a>, widget: &Widget) {
        let p = &widget.props;
        let name = widgets::keyword(p.kind);
        let info = widgets::info(p.kind);
        let at = format!("{name} at {},{}", p.col, p.row);

        let Some(bind) = &p.bind else {
            if !p.columns.is_empty() || !p.show.is_empty() {
                self.error(
                    file,
                    format!("{at} lists fields but is not bound to data"),
                    widget.span,
                );
            }
            return;
        };

        let (data_name, field_name) = match bind.split_once('.') {
            Some((d, f)) => (d, Some(f)),
            None => (bind.as_str(), None),
        };
        let Some(&(entity, list)) = page.data.get(data_name) else {
            self.error(
                file,
                format!("{at}: unknown data '{data_name}'"),
                widget.span,
            );
            return;
        };
        let Some(entity) = entity else { return };

        match info.bind {
            BindTarget::Nothing => {}
            BindTarget::Record => {
                if list || field_name.is_some() {
                    self.error(
                        file,
                        format!("{at}: a {name} binds to one record, as in 'bind customer'"),
                        widget.span,
                    );
                }
            }
            BindTarget::Field => {
                if list {
                    self.error(
                        file,
                        format!("{at}: '{data_name}' is a list; a {name} binds to a field of a single record"),
                        widget.span,
                    );
                    return;
                }
                let Some(field_name) = field_name else {
                    self.error(
                        file,
                        format!("{at}: bind a field, as in '{data_name}.name'"),
                        widget.span,
                    );
                    return;
                };
                match entity.field(field_name) {
                    None => self.error(
                        file,
                        format!("{at}: '{}' has no field '{field_name}'", entity.name.name),
                        widget.span,
                    ),
                    Some(field) if !widgets::accepts(p.kind, &field.ty) => self.error(
                        file,
                        format!(
                            "{at}: '{field_name}' is {}; a {name} shows {}",
                            widgets::field_type_tag(&field.ty),
                            widgets::accepted_types(p.kind).join(", ")
                        ),
                        widget.span,
                    ),
                    Some(_) => {}
                }
            }
            BindTarget::Collection => {
                if !list || field_name.is_some() {
                    self.error(
                        file,
                        format!("{at}: a {name} binds to a list, as in 'data orders: list<Order>'"),
                        widget.span,
                    );
                    return;
                }
                for field in p.columns.iter().chain(&p.show) {
                    match entity.field(field) {
                        None => self.error(
                            file,
                            format!("{at}: '{}' has no field '{field}'", entity.name.name),
                            widget.span,
                        ),
                        Some(f) if matches!(f.ty, FieldType::List { .. }) => self.error(
                            file,
                            format!("{at}: list field '{field}' cannot be shown in a {name}"),
                            widget.span,
                        ),
                        Some(_) => {}
                    }
                }
            }
        }
    }

    /// A click opens a window. Unknown names are warnings: they become
    /// event handlers once code behind is part of the language.
    fn check_click_target(&mut self, file: usize, target: &str, span: Span) {
        let visible = self.visible_modules(file);
        let kinds: Vec<DeclKind> = self
            .decls
            .get(target)
            .map(|ds| {
                ds.iter()
                    .filter(|d| visible.contains(d.module))
                    .map(|d| d.kind)
                    .collect()
            })
            .unwrap_or_default();
        if kinds.is_empty() {
            self.warning(
                file,
                format!("'{target}' is not a window; handlers come with code behind"),
                span,
            );
        } else if !kinds.contains(&DeclKind::Page) {
            self.error(
                file,
                format!(
                    "'{target}' is {} {}, a click opens a window",
                    article(kinds[0].noun()),
                    kinds[0].noun()
                ),
                span,
            );
        }
    }

    fn check_field(&mut self, file: usize, field: &Field) {
        let m = &field.modifiers;
        match &field.ty {
            FieldType::Text {
                max_length: Some(0),
            } => {
                self.error(file, "text length must be at least 1", field.span);
            }
            FieldType::Decimal { precision, scale } if *precision == 0 || scale > precision => {
                self.error(
                    file,
                    "decimal precision must be at least 1 and not less than the scale",
                    field.span,
                );
            }
            FieldType::Enum { name } => {
                self.resolve(file, name, DeclKind::Enum);
            }
            FieldType::Ref { entity } | FieldType::List { item: entity } => {
                self.resolve(file, entity, DeclKind::Entity);
            }
            _ => {}
        }

        let name = &field.name.name;
        if m.owned && !matches!(field.ty, FieldType::List { .. }) {
            self.error(
                file,
                format!("'owned' only applies to lists ('{name}')"),
                field.span,
            );
        }
        if m.pattern.is_some() && !field.ty.is_textual() {
            self.error(
                file,
                format!("'pattern' only applies to text fields ('{name}')"),
                field.span,
            );
        }
        if let Some(problem) = m.format.as_deref().and_then(|f| crate::formats::problem(&field.ty, name, f)) {
            self.error(file, problem, field.span);
        }
        if m.computed && m.default.is_some() {
            self.error(
                file,
                format!("a computed field cannot have a default ('{name}')"),
                field.span,
            );
        }
        if m.computed && m.required {
            self.warning(
                file,
                format!("'required' has no effect on a computed field ('{name}')"),
                field.span,
            );
        }
        for (what, literal) in [("min", &m.min), ("max", &m.max)] {
            if let Some(literal) = literal
                && !self.bound_fits(&field.ty, literal)
            {
                self.error(
                    file,
                    format!("'{what}' needs a number for numeric fields or a date string for dates ('{name}')"),
                    field.span,
                );
            }
        }
        if let (Some(Literal::Number(min)), Some(Literal::Number(max))) = (&m.min, &m.max)
            && let (Ok(min), Ok(max)) = (min.parse::<f64>(), max.parse::<f64>())
            && min > max
        {
            self.error(
                file,
                format!("'min' is greater than 'max' ('{name}')"),
                field.span,
            );
        }
        if let Some(default) = &m.default {
            self.check_default(file, field, default);
        }
    }

    fn bound_fits(&self, ty: &FieldType, literal: &Literal) -> bool {
        match ty {
            t if t.is_numeric() => matches!(literal, Literal::Number(_)),
            FieldType::Date | FieldType::DateTime => matches!(literal, Literal::Str(_)),
            _ => false,
        }
    }

    fn check_default(&mut self, file: usize, field: &Field, default: &Literal) {
        let name = &field.name.name;
        let ok = match (&field.ty, default) {
            (FieldType::Bool, Literal::Bool(_)) => true,
            (t, Literal::Number(_)) if t.is_numeric() => true,
            (t, Literal::Str(_)) if t.is_textual() => true,
            (FieldType::Date | FieldType::DateTime, Literal::Str(_)) => true,
            (FieldType::Enum { name: enum_name }, Literal::Ident(variant)) => {
                if let Some(decl) = self.enum_by_name(file, &enum_name.name)
                    && !decl.variants.iter().any(|v| &v.name.name == variant)
                {
                    self.error(
                        file,
                        format!(
                            "'{variant}' is not a variant of '{}' ('{name}')",
                            enum_name.name
                        ),
                        field.span,
                    );
                }
                true
            }
            _ => false,
        };
        if !ok {
            self.error(
                file,
                format!("default value does not match the field type ('{name}')"),
                field.span,
            );
        }
    }

    fn enum_by_name(&self, file: usize, name: &str) -> Option<&'a crate::model::EnumDecl> {
        let visible = self.visible_modules(file);
        let decls = self.decls.get(name)?;
        let found: Vec<_> = decls
            .iter()
            .filter(|d| d.kind == DeclKind::Enum && visible.contains(d.module))
            .collect();
        match found.as_slice() {
            [d] => Some(&self.files[d.file].enums[d.index]),
            _ => None,
        }
    }

}

fn article(noun: &str) -> &'static str {
    if noun.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    }
}

/// `source` lines name a SQL data source of the project.
pub(crate) fn check_sources(project: &mut Project) {
    let names: Vec<&str> = project.sources.iter().map(|s| s.name.as_str()).collect();
    let mut found = Vec::new();
    for file in &project.files {
        let Some(source) = &file.source else { continue };
        let message = match project.sources.iter().find(|s| s.name == source.name) {
            None => format!("unknown data source '{}'; the project has {}", source.name, listed(&names)),
            Some(s) if !s.sql => format!("'{}' is a web API; entities cannot live in a web API yet", source.name),
            Some(_) => continue,
        };
        found.push(Diagnostic::error(message, source.span).in_file(&file.path));
    }
    for file in &project.files {
        for entity in &file.entities {
            found.extend(check_mapping(entity).into_iter().map(|d| d.in_file(&file.path)));
        }
    }
    project.diagnostics.extend(found);
}

/// `table`, `key` and `column` map an entity onto a table, one column per
/// field.
fn check_mapping(entity: &Entity) -> Vec<Diagnostic> {
    let name = &entity.name.name;
    let mut found = Vec::new();
    let stored: Vec<&Field> = entity.fields.iter().filter(|f| !matches!(f.ty, FieldType::List { .. })).collect();
    for (i, field) in stored.iter().enumerate() {
        let column = entity.column_of(&field.name.name);
        if column == entity.key_column() {
            found.push(Diagnostic::error(
                format!("{name}: field '{}' uses column '{column}', which is the key", field.name.name),
                field.span,
            ));
        }
        if let Some(other) = stored[..i].iter().find(|o| entity.column_of(&o.name.name) == column) {
            found.push(Diagnostic::error(
                format!("{name}: fields '{}' and '{}' both use column '{column}'", other.name.name, field.name.name),
                field.span,
            ));
        }
    }
    found
}

/// `a`, `a and b`, `a, b and c`.
fn listed(names: &[&str]) -> String {
    match names {
        [] => "none".to_string(),
        [only] => (*only).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
