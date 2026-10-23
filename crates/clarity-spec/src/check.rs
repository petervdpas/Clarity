//! Semantic checks across all spec files of a project: name resolution,
//! module visibility, modifier validity, and windows with their widgets
//! and frame regions.

use std::collections::{HashMap, HashSet};

use crate::diagnostic::Diagnostic;
use crate::frame;
use crate::model::{
    App, Command, CommandKind, DataDecl, DataKind, Entity, Field, FieldType, Frame, Ident, Project,
    Literal, MAX_PAGE_COLUMNS, Page, PageKind, SpecFile, Widget,
    WidgetProps,
};
use crate::span::Span;
use crate::widgets::{self, BindTarget, ContainerKind, WidgetKind};

/// Data declarations of the page being checked: name -> (entity, is list).
/// The entity is `None` when it did not resolve; that error is reported once.
struct PageContext<'a> {
    data: HashMap<&'a str, Held<'a>>,
    /// The popovers of the window, by name: a click opens them as it opens a window.
    popovers: HashSet<&'a str>,
}

/// What a window's data holds, as its parts bind it. An entity that does
/// not resolve is None, reported once where it is declared.
#[derive(Clone)]
enum Held<'a> {
    Record(Option<&'a Entity>),
    List(Option<&'a Entity>),
    /// A plain value, with the field type a part shows it as.
    Value(FieldType),
}

/// The field type a part shows a plain value of `ty` as, for the types a
/// part accepts; None for a type no part shows (a list).
fn shown_as(ty: &crate::code::TypeRefKind) -> Option<FieldType> {
    use crate::code::TypeRefKind as T;
    Some(match ty {
        T::Text => FieldType::Text { max_length: None },
        T::Integer => FieldType::Integer,
        // A plain decimal keeps the digits it has; only the kind matters here.
        T::Decimal => FieldType::Decimal { precision: 0, scale: 0 },
        T::Money => FieldType::Money,
        T::Bool => FieldType::Bool,
        T::Date => FieldType::Date,
        T::DateTime => FieldType::DateTime,
        T::Duration => FieldType::Duration,
        T::Named { name } => FieldType::Enum { name: Ident { name: name.clone(), span: crate::span::Span::default() } },
        T::File | T::Bytes | T::List { .. } => return None,
    })
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
        if let Some(size) = page.size
            && !crate::layout::window_size_fits(size.width, size.height)
        {
            self.error(file, crate::layout::window_size_problem(), page.name.span);
        }
        if page.columns == 0 || page.columns > MAX_PAGE_COLUMNS {
            self.error(
                file,
                format!("a page grid has 1 to {MAX_PAGE_COLUMNS} columns"),
                page.name.span,
            );
            return;
        }

        let mut context = self.data_context(file, &page.data);
        context.popovers = page.widgets.iter().filter(|w| w.props.kind == WidgetKind::Popover).filter_map(|w| w.props.name.as_deref()).collect();
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
        if let Some(gives) = &page.gives
            && page.kind != PageKind::Dialog
        {
            self.error(file, format!("only a dialog gives a value; make {} a dialog, or take 'gives' away", page.name.name), gives.span);
        }
        self.check_frame(file, &context, &page.frame);
        self.check_names(file, &page.widgets, &page.frame);
        self.check_commands_and_data(file, page);
        self.check_forms(file, &page.widgets, None);
        for sidebar in &page.frame.sidebars {
            self.check_forms(file, &sidebar.widgets, None);
        }
    }

    fn data_context(&mut self, file: usize, decls: &'a [DataDecl]) -> PageContext<'a> {
        let mut data: HashMap<&str, Held<'a>> = HashMap::new();
        for decl in decls {
            let held = match &decl.of {
                DataKind::Record { entity } | DataKind::List { entity } => {
                    let found = if self.decls.contains_key(entity.name.as_str()) {
                        self.resolve(file, entity, DeclKind::Entity).map(|(f, i)| &self.files[f].entities[i])
                    } else {
                        self.error(file, format!("unknown entity or enum '{}'", entity.name), entity.span);
                        None
                    };
                    if decl.is_list() { Held::List(found) } else { Held::Record(found) }
                }
                DataKind::Value { ty, default } => {
                    if let crate::code::TypeRefKind::Named { name } = &ty.kind {
                        let ident = Ident { name: name.clone(), span: ty.span };
                        let is_entity = self.decls.get(name.as_str()).is_some_and(|ds| ds.iter().any(|d| d.kind == DeclKind::Entity));
                        if is_entity && default.is_some() {
                            self.error(
                                file,
                                format!("'{}' is a record, which starts new; only a plain value has a default", decl.name.name),
                                ty.span,
                            );
                        } else if self.decls.contains_key(name.as_str()) {
                            self.resolve(file, &ident, DeclKind::Enum);
                        } else {
                            self.error(file, format!("unknown entity or enum '{name}'"), ty.span);
                        }
                    }
                    match shown_as(&ty.kind) {
                        Some(shown) => Held::Value(shown),
                        None => continue,
                    }
                }
            };
            if data.insert(&decl.name.name, held).is_some() {
                self.error(
                    file,
                    format!("duplicate data '{}'", decl.name.name),
                    decl.name.span,
                );
            }
        }
        PageContext { data, popovers: HashSet::new() }
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
    /// A sidebar's name, its width and limits, and what the person may do with it.
    fn check_sidebar(&mut self, file: usize, sidebar: &crate::model::Sidebar) {
        let p = &sidebar.props;
        let side = crate::frame::side_keyword(p.side);
        let shown = p.name.clone().unwrap_or_else(|| side.to_string());
        let mut problems = Vec::new();
        match p.name.as_deref() {
            None => problems.push(format!("sidebar {side} has no name; every part has one (Name them, in Problems, names them all)")),
            Some(name) if !crate::names::is_camel_case(name) => problems.push(crate::names::camel_case_problem("part", name)),
            Some(_) => {}
        }
        let (least, most) = crate::model::SIDEBAR_WIDTHS;
        for width in [Some(p.width), p.min, p.max].into_iter().flatten() {
            if !(least..=most).contains(&width) {
                problems.push(format!("a sidebar is {least} to {most} pixels wide"));
                break;
            }
        }
        if (p.min.is_some() || p.max.is_some()) && !p.resizable {
            problems.push(format!("sidebar {shown}: 'min' and 'max' are for a resizable sidebar; add 'resizable'"));
        }
        if let Some(min) = p.min.filter(|m| *m > p.width) {
            problems.push(format!("sidebar {shown}: min {min} is more than its width {}", p.width));
        }
        if let Some(max) = p.max.filter(|m| *m < p.width) {
            problems.push(format!("sidebar {shown}: max {max} is less than its width {}", p.width));
        }
        if p.collapsed && !p.collapsible {
            problems.push(format!("sidebar {shown}: 'collapsed' needs 'collapsible': write 'collapsible collapsed'"));
        }
        for problem in problems {
            self.error(file, problem, sidebar.span);
        }
    }

    /// The menu items and buttons a window shows (its own, and the app's) are
    /// members of it beside its data, so they cannot share a name with it.
    fn check_commands_and_data(&mut self, file: usize, page: &'a Page) {
        fn named<'c>(items: &'c [Command], out: &mut Vec<(&'c str, Span)>) {
            for c in items {
                if let Some(name) = &c.props.name {
                    out.push((name, c.span));
                }
                named(&c.items, out);
            }
        }
        let mut commands: Vec<(&str, Span, Option<usize>)> = Vec::new();
        let mut collect = |frame: &'a Frame, from: Option<usize>| {
            let mut found = Vec::new();
            if let Some(menu) = &frame.menu {
                named(&menu.items, &mut found);
            }
            if let Some(toolbar) = &frame.toolbar {
                named(&toolbar.items, &mut found);
            }
            if let Some(ribbon) = &frame.ribbon {
                for tab in &ribbon.tabs {
                    for group in &tab.groups {
                        named(&group.items, &mut found);
                    }
                }
            }
            commands.extend(found.into_iter().map(|(n, s)| (n, s, from)));
        };
        collect(&page.frame, None);
        if page.kind == PageKind::Window {
            for (index, f) in self.files.iter().enumerate() {
                for app in &f.apps {
                    collect(&app.frame, Some(index));
                }
            }
        }
        for (name, span, from) in commands {
            if page.data.iter().any(|d| d.name.name == name) {
                // Reported where the item is written.
                self.error(
                    from.unwrap_or(file),
                    format!("item or button '{name}' has the name of data of {}; give one of them another name", page.name.name),
                    span,
                );
            }
        }
    }

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
        for menu in all.iter().filter_map(|w| w.menu.as_ref()) {
            named.extend(menu.name.as_deref().map(|n| (n, menu.span)));
            commands(&menu.items, &mut named);
        }
        if let Some(menu) = &frame.menu {
            named.extend(menu.name.as_deref().map(|n| (n, menu.span)));
            commands(&menu.items, &mut named);
        }
        if let Some(toolbar) = &frame.toolbar {
            named.extend(toolbar.name.as_deref().map(|n| (n, toolbar.span)));
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
            let found = all.iter().find(|w| w.props.name.as_deref() == Some(target.as_str()));
            match found {
                None => self.error(
                    file,
                    format!("{at}: there is no part '{target}' in this window"),
                    widget.span,
                ),
                // A popover opens next to any part.
                Some(_) if p.kind == WidgetKind::Popover => {}
                Some(other) if !widgets::info(other.props.kind).input => self.error(
                    file,
                    format!(
                        "{at}: '{target}' is {} {}, not an input",
                        article(widgets::keyword(other.props.kind)),
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

    /// A menu and a toolbar are parts, with camelCase names.
    fn check_region_name(&mut self, file: usize, name: &Option<String>, what: &str, span: Span) {
        match name.as_deref() {
            None => self.error(file, format!("{what} has no name; every part has one (Name them, in Problems, names them all)"), span),
            Some(n) if !crate::names::is_camel_case(n) => self.error(file, crate::names::camel_case_problem("part", n), span),
            Some(_) => {}
        }
    }

    fn check_frame(&mut self, file: usize, context: &PageContext<'a>, frame: &'a Frame) {
        if let Some(menu) = &frame.menu {
            if menu.items.is_empty() {
                self.warning(file, "the menu has no items yet", menu.span);
            }
            self.check_region_name(file, &menu.name, "the menu", menu.span);
            self.check_commands(file, &menu.items);
        }
        if let Some(toolbar) = &frame.toolbar {
            if toolbar.items.is_empty() {
                self.warning(file, "the toolbar has no buttons yet", toolbar.span);
            }
            self.check_region_name(file, &toolbar.name, "the toolbar", toolbar.span);
            self.check_commands(file, &toolbar.items);
        }
        if let Some(ribbon) = &frame.ribbon {
            if ribbon.tabs.is_empty() {
                self.warning(file, "the ribbon has no tabs yet", ribbon.span);
            }
            // The ribbon, its tabs and its groups are parts, with names.
            let named = |name: &Option<String>, what: String| match name.as_deref() {
                None => Some(format!("{what} has no name; every part has one (Name them, in Problems, names them all)")),
                Some(n) if !crate::names::is_camel_case(n) => Some(crate::names::camel_case_problem("part", n)),
                Some(_) => None,
            };
            let mut problems = Vec::new();
            problems.extend(named(&ribbon.name, "the ribbon".to_string()).map(|p| (p, ribbon.span)));
            if ribbon.collapsed && !ribbon.collapsible {
                problems.push(("the ribbon: 'collapsed' needs 'collapsible': write 'collapsible collapsed'".to_string(), ribbon.span));
            }
            for tab in &ribbon.tabs {
                problems.extend(named(&tab.name, format!("tab \"{}\"", tab.title)).map(|p| (p, tab.span)));
                for group in &tab.groups {
                    problems.extend(named(&group.name, format!("group \"{}\"", group.title)).map(|p| (p, group.span)));
                }
            }
            for (problem, span) in problems {
                self.error(file, problem, span);
            }
            for tab in &ribbon.tabs {
                for group in &tab.groups {
                    self.check_commands(file, &group.items);
                }
            }
        }
        for sidebar in &frame.sidebars {
            self.check_sidebar(file, sidebar);
            if sidebar.props.columns == 0 || sidebar.props.columns > MAX_PAGE_COLUMNS {
                self.error(file, format!("a sidebar's grid has 1 to {MAX_PAGE_COLUMNS} columns"), sidebar.span);
            } else {
                self.check_widgets(file, context, &sidebar.widgets, sidebar.props.columns);
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
            // Every menu item and button has a name, in camelCase, as a part has.
            if p.kind != CommandKind::Separator {
                match p.name.as_deref() {
                    None => self.error(
                        file,
                        format!(
                            "{} \"{}\" has no name; every menu item and button has one (Name them, in Problems, names them all)",
                            frame::command_keyword(p.kind),
                            p.caption.as_deref().unwrap_or_default()
                        ),
                        command.span,
                    ),
                    Some(name) if !crate::names::is_camel_case(name) => {
                        self.error(file, crate::names::camel_case_problem("item or button", name), command.span)
                    }
                    Some(_) => {}
                }
            }
            if let Some(icon) = p.icon.as_deref().filter(|i| crate::icons::path(i).is_none()) {
                self.error(file, crate::icons::unknown(icon), command.span);
            }
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

    /// The rules of a separator, a link and a progress bar (U1b).
    fn check_small_part(&mut self, file: usize, widget: &Widget) {
        let p = &widget.props;
        match p.kind {
            WidgetKind::Form if p.labels.is_some() => {
                let columns = p.labels.unwrap_or_default();
                if columns == 0 || columns >= p.width {
                    self.error(
                        file,
                        format!("a form's label column leaves room for its inputs: 'labels' is 1 to {} here", p.width.saturating_sub(1)),
                        widget.span,
                    );
                }
                let labelled: Vec<&str> = widget.children.iter().filter(|w| w.props.made).filter_map(|w| w.props.for_part.as_deref()).collect();
                for inner in widget.children.iter().filter(|w| !w.props.made) {
                    let q = &inner.props;
                    if let Some(name) = q.name.as_deref().filter(|n| labelled.contains(n))
                        && q.col <= columns
                    {
                        self.error(file, format!("'{name}' starts at column {} or later: its label takes the columns before", columns + 1), inner.span);
                    }
                    if q.kind == WidgetKind::Label
                        && let Some(target) = q.for_part.as_deref().filter(|t| labelled.contains(t))
                    {
                        self.error(file, format!("the form labels '{target}' itself; remove this label"), inner.span);
                    }
                }
            }
            WidgetKind::Buttons => {
                for inner in &widget.children {
                    if !matches!(inner.props.kind, WidgetKind::Button | WidgetKind::Toggle) {
                        self.error(file, "a button group holds buttons and toggles", inner.span);
                    } else if inner.props.row != 1 || inner.props.height != 1 {
                        self.error(file, "the buttons of a group stand in one row: at row 1, one row high", inner.span);
                    }
                }
            }
            WidgetKind::Button if p.split => {
                if widget.menu.is_none() {
                    self.error(file, "a split button opens its menu: write a 'menu' block under it", widget.span);
                }
                let clicks = p.on_click.is_some() || p.handlers.iter().any(|h| h.event == widgets::Event::Click);
                if !clicks {
                    self.error(file, "a split button runs its click too: wire 'on click'", widget.span);
                }
            }
            WidgetKind::Separator if p.caption.is_some() => {
                self.error(file, "a separator has no caption: it is only a line", widget.span);
            }
            WidgetKind::Link => {
                let clicks = p.on_click.is_some() || p.handlers.iter().any(|h| h.event == widgets::Event::Click);
                let ways = [p.to.is_some(), p.bind.is_some(), clicks].iter().filter(|w| **w).count();
                if ways == 0 {
                    self.error(file, "a link opens an address ('to'), what it binds or runs a handler ('on click')", widget.span);
                } else if ways > 1 {
                    self.error(file, "a link opens an address, what it binds or runs a handler: one of them", widget.span);
                }
                if let Some(to) = p.to.as_deref().filter(|t| !is_address(t)) {
                    self.error(file, format!("'{to}' is no address; write it whole, as in \"https://{to}\""), widget.span);
                }
            }
            WidgetKind::Progress => {
                if p.bind.is_none() {
                    self.error(file, "a progress bar shows a number: bind one, as in 'bind done'", widget.span);
                }
                if p.max.as_deref().is_some_and(|m| m.parse::<f64>().map_or(true, |n| n <= 0.0)) {
                    self.error(file, "'max' is a number above 0: the number the bar is full at", widget.span);
                }
            }
            _ => {}
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
            if let Some(icon) = p.icon.as_deref().filter(|i| crate::icons::path(i).is_none()) {
                self.error(file, crate::icons::unknown(icon), widget.span);
            }
            self.check_small_part(file, widget);
            // A popover is outside the grid: only its own parts are placed, on its own grid.
            let popover = p.kind == WidgetKind::Popover;
            if popover && p.for_part.is_none() {
                self.error(file, "a popover opens next to a part: write 'for' and the part's name", widget.span);
            }
            let inside = widget.children.iter().chain(widget.pages.iter().flat_map(|page| &page.children));
            for nested in inside.filter(|w| w.props.kind == WidgetKind::Popover) {
                self.error(file, "a popover stands on the window, not in another part", nested.span);
            }
            if let Some(menu) = &widget.menu {
                // A context menu is named only when code hides or disables it as a whole.
                if let Some(menu_name) = menu.name.as_deref().filter(|n| !crate::names::is_camel_case(n)) {
                    self.error(file, crate::names::camel_case_problem("context menu", menu_name), menu.span);
                }
                if menu.items.is_empty() {
                    self.warning(file, "the context menu has no items yet", menu.span);
                }
                self.check_commands(file, &menu.items);
            }
            if !popover && (p.col == 0 || p.row == 0 || p.width == 0 || p.height == 0) {
                self.error(
                    file,
                    format!(
                        "{name} at {},{}: position and size start at 1",
                        p.col, p.row
                    ),
                    widget.span,
                );
            } else if !popover && p.col + p.width - 1 > columns {
                self.error(
                    file,
                    format!(
                        "{name} at {},{} span {} runs past column {columns}",
                        p.col, p.row, p.width
                    ),
                    widget.span,
                );
            }
            for other in widgets[..index].iter().filter(|o| !popover && o.props.kind != WidgetKind::Popover) {
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
            // Every part has a name, which code and its events go by (as in an IDE).
            if let Some(part) = p.name.as_deref().filter(|n| !crate::names::is_camel_case(n)) {
                self.error(file, crate::names::camel_case_problem("part", part), widget.span);
            }
            if p.name.is_none() {
                self.error(
                    file,
                    format!("{name} at {},{} has no name; every part has one (Name them, in Problems, names them all)", p.col, p.row),
                    widget.span,
                );
            }
            self.check_binding(file, page, widget);
            if let Some(target) = &p.on_click {
                if p.new_window && frame::BUILT_IN_COMMANDS.contains(&target.as_str()) {
                    self.error(file, "only a window opens in a new window", widget.span);
                }
                if frame::BUTTON_COMMANDS.contains(&target.as_str()) {
                    // A button closes its window or quits like a menu item.
                } else if frame::BUILT_IN_COMMANDS.contains(&target.as_str()) {
                    self.error(file, format!("'{target}' picks in a menu or a toolbar; a button can close its window or quit"), widget.span);
                } else if page.popovers.contains(target.as_str()) {
                    // A click opens a popover of its window next to its part.
                } else {
                    self.check_click_target(file, target, widget.span);
                }
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
                        ContainerKind::Accordion if widget.pages.is_empty() => {
                            self.error(file, "an accordion needs at least one 'section'", widget.span);
                        }
                        ContainerKind::Steps if widget.pages.len() < 2 => {
                            self.error(file, "steps need at least two 'step' blocks: a wizard of one step is a panel", widget.span);
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

            let draws = p.handlers.iter().any(|h| h.event == widgets::Event::Draw);
            if p.kind == WidgetKind::Canvas && p.renderer.is_none() && !draws {
                self.warning(
                    file,
                    format!("canvas at {},{} draws nothing yet; wire 'on draw: handler'", p.col, p.row),
                    widget.span,
                );
            }
            if p.kind == WidgetKind::Image && p.source.is_none() && p.bind.is_none() && p.caption.is_none() {
                self.warning(
                    file,
                    format!("image at {},{} has no source yet", p.col, p.row),
                    widget.span,
                );
            }
        }
    }

    /// The rules of a slider's or a spin box's range (U1c): its lowest value
    /// below its highest, a step above 0, whole for a field of whole numbers.
    /// `field` is the bound field's name, type and modifiers, when it binds one.
    fn check_range(&mut self, file: usize, widget: &Widget, field: Option<(&str, &FieldType, &crate::model::Modifiers)>) {
        let p = &widget.props;
        if !matches!(p.kind, WidgetKind::Slider | WidgetKind::Spinbox) {
            return;
        }
        let range = widgets::number_range(p, field.map(|(_, _, m)| m));
        let number = |n: &Option<String>| n.as_deref().and_then(|n| n.parse::<f64>().ok());
        if let (Some(min), Some(max)) = (number(&range.min), number(&range.max))
            && min >= max
        {
            let (low, high) = (range.min.as_deref().unwrap_or_default(), range.max.as_deref().unwrap_or_default());
            self.error(file, format!("'min' is below 'max': {low} is not below {high}"), widget.span);
        }
        let step = range.step.parse::<f64>().unwrap_or(0.0);
        if step <= 0.0 {
            self.error(file, "'step' is a number above 0", widget.span);
        } else if let Some((name, FieldType::Integer, _)) = field
            && step.fract() != 0.0
        {
            self.error(file, format!("'{name}' holds whole numbers: the step is a whole number"), widget.span);
        }
    }

    fn check_binding(&mut self, file: usize, page: &PageContext<'a>, widget: &Widget) {
        let p = &widget.props;
        let name = widgets::keyword(p.kind);
        let info = widgets::info(p.kind);
        let at = format!("{name} at {},{}", p.col, p.row);

        // A markdown or an image shows one thing: a key, a file of the resources (source), or what it binds.
        if matches!(p.kind, WidgetKind::Markdown | WidgetKind::Image)
            && [p.caption.is_some(), p.source.is_some(), p.bind.is_some()].iter().filter(|given| **given).count() > 1
        {
            self.error(
                file,
                format!("a {name} shows a key, a file of the resources (source) or what it binds: one of them"),
                widget.span,
            );
        }

        let Some(bind) = &p.bind else {
            self.check_range(file, widget, None);
            if !p.columns.is_empty() || !p.edit.is_empty() || !p.show.is_empty() {
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
        let Some(held) = page.data.get(data_name).cloned() else {
            self.error(
                file,
                format!("{at}: unknown data '{data_name}'"),
                widget.span,
            );
            return;
        };
        let (entity, list) = match held {
            Held::Record(entity) => (entity, false),
            Held::List(entity) => (entity, true),
            Held::Value(shown) => {
                let problem = match info.bind {
                    _ if field_name.is_some() => Some(format!("'{data_name}' is a plain value; bind it as '{data_name}'")),
                    BindTarget::Nothing => None,
                    BindTarget::Record => Some(format!("a {name} binds to one record, as in 'bind customer'")),
                    BindTarget::Collection => Some(format!("a {name} binds to a list, as in 'data orders: list<Order>'")),
                    BindTarget::Field if !widgets::accepts(p.kind, &shown) => Some(format!(
                        "'{data_name}' is {}; a {name} shows {}",
                        widgets::field_type_tag(&shown),
                        widgets::accepted_types(p.kind).join(", ")
                    )),
                    BindTarget::Field => None,
                };
                if let Some(problem) = problem {
                    self.error(file, format!("{at}: {problem}"), widget.span);
                } else {
                    self.check_range(file, widget, None);
                }
                return;
            }
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
                    Some(field) => self.check_range(file, widget, Some((field_name, &field.ty, &field.modifiers))),
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
                        Some(f) if f.ty == FieldType::Image => self.error(
                            file,
                            format!("{at}: '{field}' is a picture; show it in an image part"),
                            widget.span,
                        ),
                        Some(_) => {}
                    }
                }
                if let Some(order) = p.order.as_ref().filter(|o| !p.columns.contains(&o.field)) {
                    self.error(file, format!("{at}: '{}' is not a column of the table; order by one it shows", order.field), widget.span);
                }
                self.check_edits(file, &at, entity, widget);
            }
        }
    }

    /// The edited columns of a table: shown, of a type a cell can edit,
    /// and saved by an `on change` handler.
    fn check_edits(&mut self, file: usize, at: &str, entity: &Entity, widget: &Widget) {
        let p = &widget.props;
        if p.edit.is_empty() {
            return;
        }
        for field in &p.edit {
            if !p.columns.contains(field) {
                self.error(file, format!("{at}: '{field}' is edited but is not one of the columns"), widget.span);
                continue;
            }
            let Some(f) = entity.field(field) else { continue };
            if p.switches.contains(field) && f.ty != FieldType::Bool {
                self.error(
                    file,
                    format!("{at}: '{field}' is {}; only a bool column can be a switch", widgets::field_type_tag(&f.ty)),
                    widget.span,
                );
            }
            if !widgets::cell_edits(&f.ty) {
                self.error(
                    file,
                    format!(
                        "{at}: '{field}' is {}; a table edits text, numbers, bool, dates, enum and ref columns",
                        widgets::field_type_tag(&f.ty)
                    ),
                    widget.span,
                );
            }
        }
        if !p.handlers.iter().any(|h| h.event == widgets::Event::Change) {
            self.warning(file, format!("{at} edits its rows but has no 'on change' handler to save them"), widget.span);
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
        if m.store.is_some() && field.ty != FieldType::Image {
            self.error(file, format!("only an image field has a store ('{name}')"), field.span);
        }
        if field.ty == FieldType::Image {
            if m.unique {
                self.error(file, format!("a picture cannot be unique ('{name}')"), field.span);
            }
            if m.default.is_some() {
                self.error(file, format!("a picture has no default; it starts empty ('{name}')"), field.span);
            }
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
        if let Some(default) = m.default.as_ref().filter(|_| field.ty != FieldType::Image) {
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
pub(crate) fn listed(names: &[&str]) -> String {
    match names {
        [] => "none".to_string(),
        [only] => (*only).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Whether a link's `to` is a whole address: a scheme and what follows it.
fn is_address(to: &str) -> bool {
    ["https://", "http://", "mailto:", "tel:"].iter().any(|scheme| to.starts_with(scheme) && to.len() > scheme.len())
}
