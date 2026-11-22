//! Semantic checks across all spec files of a project: name resolution,
//! module visibility, modifier validity, and windows with their widgets
//! and frame regions.

use std::collections::{HashMap, HashSet};

use crate::diagnostic::Diagnostic;
use crate::frame;
use crate::model::{
    ChartMeasure,
    App, Command, CommandKind, DataDecl, DataKind, Entity, Field, FieldType, Frame, Ident, Project,
    Literal, MAX_PAGE_COLUMNS, Page, PageKind, SpecFile, Widget,
    WidgetProps,
};
use crate::span::Span;
use crate::widgets::{self, BindTarget, ContainerKind, WidgetKind};

mod data;
mod parts;
mod settings;
mod signin;
mod toolboxes;
mod views;
mod window_frame;

/// Data declarations of the page being checked: name -> (entity, is list).
/// The entity is `None` when it did not resolve; that error is reported once.
struct PageContext<'a> {
    data: HashMap<&'a str, Held<'a>>,
    /// The popovers and slide-outs of the window, by name: a click opens them as it opens a window.
    popovers: HashSet<&'a str>,
}

/// The slide-outs beside a part inside other parts (`beside chapterTable`).
fn slideouts_beside<'w>(widgets: &'w [Widget], out: &mut Vec<&'w Widget>) {
    for w in widgets {
        for inner in w.children.iter().chain(w.pages.iter().flat_map(|p| &p.children)) {
            if inner.props.kind == WidgetKind::Slideout && inner.props.beside.is_some() {
                out.push(inner);
            }
        }
        slideouts_beside(&w.children, out);
        for page in &w.pages {
            slideouts_beside(&page.children, out);
        }
    }
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
        // A list of values (manual/parts.data.md); a list of records is not a plain value.
        T::List { item } => FieldType::Values { item: crate::model::ListItem::of_word(item)? },
        T::File | T::Bytes => return None,
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
    Record,
    Page,
    App,
    Part,
    Library,
}

impl DeclKind {
    fn noun(self) -> &'static str {
        match self {
            Self::Enum => "enum",
            Self::Entity => "entity",
            Self::Record => "record",
            Self::Page => "page",
            Self::App => "app",
            Self::Part => "part",
            Self::Library => "library",
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
    /// The popovers and slide-outs of the window being checked: a menu item's click opens them too.
    overlays: HashSet<&'a str>,
    /// The project file's `signin` block (docs/ledger/signin.md); none without it.
    signin: Option<&'a crate::signin::Signin>,
    /// The page being checked, for the parts a `signin` button reads.
    page: Option<&'a Page>,
}

/// The fewest characters a hashed field keeps: an Argon2id hash as text is
/// about a hundred.
pub const HASH_LENGTH: u32 = 128;

pub fn check<'a>(files: &'a [SpecFile], signin: Option<&'a crate::signin::Signin>) -> Vec<Diagnostic> {
    let mut checker = Checker {
        files,
        modules: HashMap::new(),
        decls: HashMap::new(),
        diagnostics: Vec::new(),
        overlays: HashSet::new(),
        signin,
        page: None,
    };
    checker.collect();
    checker.check_single_app();
    checker.check_settings();
    checker.diagnostics.extend(check_toasts(files));
    for (file_index, file) in files.iter().enumerate() {
        checker.check_file(file_index, file);
    }
    checker.check_screen_engines_run();
    checker.diagnostics.extend(crate::code::check::check(files));
    checker.diagnostics
}

impl<'a> Checker<'a> {
    /// An engine beside the screen that no canvas runs never runs.
    fn check_screen_engines_run(&mut self) {
        let mut run: Vec<String> = Vec::new();
        for file in self.files {
            for page in &file.pages {
                crate::resources::each_widget(&page.widgets, &mut |w| run.extend(w.props.runs.clone()));
            }
        }
        for (index, file) in self.files.iter().enumerate() {
            for engine in file.engines.iter().filter(|e| e.beside && !run.contains(&e.name.name)) {
                self.warning(index, format!("engine {} runs beside the screen, but no canvas runs it", engine.name.name), engine.span);
            }
        }
    }

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
            // A window's or view's own file (`.cxaml`) is part of its folder's module, not a module of its own.
            if let Some(module) = file.module.as_ref().filter(|_| !crate::cxaml::is_page_file(&file.path)) {
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
                    // The settings are found as every window's `settings`, not by name.
                    file.entities
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| !e.settings)
                        .map(|(i, e)| (DeclKind::Entity, i, &e.name)),
                )
                .chain(file.records.iter().enumerate().map(|(i, r)| (DeclKind::Record, i, &r.name)))
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
                )
                .chain(file.parts.iter().enumerate().map(|(i, p)| (DeclKind::Part, i, &p.name)))
                .chain(file.libraries.iter().enumerate().map(|(i, l)| (DeclKind::Library, i, &l.name)));
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
                if let Some(problem) = crate::names::name_problem(if entity.settings { "setting" } else { "field" }, &field.name.name) {
                    self.error(file_index, problem, field.name.span);
                }
                self.check_field(file_index, field);
                self.check_hashed(file_index, !entity.settings, field);
            }
        }

        // A record holds values: no table, no references to stored records, no files.
        for record in &file.records {
            let mut seen = HashSet::new();
            for field in &record.fields {
                if !seen.insert(field.name.name.as_str()) {
                    self.error(file_index, format!("duplicate field '{}'", field.name.name), field.name.span);
                }
                if let Some(problem) = crate::names::name_problem("field", &field.name.name) {
                    self.error(file_index, problem, field.name.span);
                }
                if matches!(field.ty, FieldType::Image | FieldType::Blob) {
                    self.error(file_index, format!("record {} has field {}: a record's fields are values or references to stored records, not pictures or files", record.name.name, field.name.name), field.name.span);
                    continue;
                }
                // A record refers to a stored record by its id, never to another record.
                if let FieldType::Ref { entity } = &field.ty
                    && self.decls.get(entity.name.as_str()).is_some_and(|ds| ds.iter().any(|d| d.kind == DeclKind::Record))
                {
                    self.error(file_index, format!("record {} has field {}: a reference names a stored entity; {} is a record, which is never stored", record.name.name, field.name.name, entity.name), field.name.span);
                    continue;
                }
                self.check_field(file_index, field);
                self.check_hashed(file_index, false, field);
            }
            if record.table.is_some() || record.key.is_some() {
                self.error(file_index, format!("record {} belongs to no table, so it has no 'table' or 'key'", record.name.name), record.name.span);
            }
        }

        for page in &file.pages {
            self.check_page(file_index, page);
            self.overlays.clear();
        }

        for app in &file.apps {
            self.check_app(file_index, app);
        }
    }

    // -- pages ------------------------------------------------------------

    fn check_page(&mut self, file: usize, page: &'a Page) {
        self.page = Some(page);
        if page.kind == PageKind::Signin {
            self.check_signin_page(file, page);
        }
        if page.kind == PageKind::Splash {
            // Optional, but a splash page is there to be shown: one no app names is said.
            let shown = self.files.iter().flat_map(|f| &f.apps).any(|a| a.splash.as_ref().is_some_and(|s| s.name == page.name.name));
            if !shown {
                self.warning(file, format!("{} is a splash page no app shows: write splash {} in the app", page.name.name, page.name.name), page.name.span);
            }
            // The application shows and closes it: it has no open, close or return of its own.
            if let Some(handler) = page.handlers.first() {
                self.error(file, format!("a splash page shows while the application starts and closes when the main window shows; it has no 'on {}' of its own", handler.event.keyword()), page.name.span);
            }
        }
        if let Some(size) = page.size
            && !crate::layout::window_size_fits(size.width, size.height)
        {
            self.error(file, crate::layout::window_size_problem(), page.name.span);
        }
        if let Some(cell) = page.cell
            && !(crate::layout::CELL_HEIGHTS.0..=crate::layout::CELL_HEIGHTS.1).contains(&cell)
        {
            let (least, most) = crate::layout::CELL_HEIGHTS;
            self.error(file, format!("a page's rows are {least} to {most} pixels high"), page.name.span);
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
        context.popovers = page.widgets.iter().filter(|w| matches!(w.props.kind, WidgetKind::Popover | WidgetKind::Slideout)).filter_map(|w| w.props.name.as_deref()).collect();
        // A slide-out beside a part stands in that part's container, at any depth.
        let mut beside = Vec::new();
        slideouts_beside(&page.widgets, &mut beside);
        context.popovers.extend(beside.iter().filter_map(|w| w.props.name.as_deref()));
        self.overlays = context.popovers.clone();
        self.check_widgets(file, &context, &page.widgets, page.columns);
        self.check_beside(file, &page.widgets);
        self.check_drops(file, &page.widgets);
        self.check_tools(file, &page.widgets, page.kind == PageKind::Toolbox, false);
        if page.kind != PageKind::Window && !page.frame.is_empty() {
            let kind = match page.kind {
                PageKind::Dialog => "dialog",
                PageKind::Signin => "sign-in window",
                _ => "view",
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
        self.check_views(file, page);
        self.check_page_keys(file, page);
        self.check_commands_and_data(file, page);
        self.check_forms(file, &page.widgets, None);
        for sidebar in &page.frame.sidebars {
            self.check_forms(file, &sidebar.widgets, None);
        }
        self.check_signin_guided(file, page);
    }

    // -- apps and frames --------------------------------------------------

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
            // Entities in a web API (manual/code.web-apis.md): its actions read and send them.
            Some(_) => continue,
        };
        found.push(Diagnostic::error(message, source.span).in_file(&file.path));
    }
    for file in &project.files {
        for entity in &file.entities {
            found.extend(check_mapping(entity).into_iter().map(|d| d.in_file(&file.path)));
        }
    }
    found.extend(check_source_mappings(project));
    project.diagnostics.extend(found);
}

/// Each toast: its name once in the project, a text, a lifespan Clarity
/// can show and an icon it knows.
fn check_toasts(files: &[SpecFile]) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for file in files {
        for toast in &file.toasts {
            let name = &toast.name.name;
            if !seen.insert(name.clone()) {
                found.push(Diagnostic::error(format!("toast {name} is declared twice; give each toast a name of its own"), toast.name.span).in_file(&file.path));
            }
            if toast.text.is_none() {
                found.push(Diagnostic::error(format!("toast {name} has no text; give it one, as in text @{name}Toast"), toast.name.span).in_file(&file.path));
            }
            if let Some(ms) = toast.lifespan.filter(|ms| !crate::code::TOAST_LIFESPANS.contains(ms)) {
                let (least, most) = (crate::code::TOAST_LIFESPANS.start(), crate::code::TOAST_LIFESPANS.end());
                found.push(Diagnostic::error(format!("toast {name} shows from {least} to {most} milliseconds, not a lifespan of {ms}"), toast.span).in_file(&file.path));
            }
            if let Some(icon) = toast.icon.as_deref().filter(|i| crate::icons::path(i).is_none()) {
                found.push(Diagnostic::error(crate::icons::unknown(icon), toast.span).in_file(&file.path));
            }
        }
    }
    found
}

/// A file source laid over by a mapping (`using OrderFeed`): a module
/// declares the mapping, and its entity lives in that source. A missing
/// mapping is reported on the `source` line of each module in the source.
fn check_source_mappings(project: &Project) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for source in &project.sources {
        let Some(name) = &source.mapping else { continue };
        let Some(mapping) = project.mapping(name) else {
            for file in project.files.iter().filter(|f| f.source.as_ref().is_some_and(|s| s.name == source.name)) {
                let span = file.source.as_ref().map(|s| s.span).unwrap_or_default();
                found.push(Diagnostic::error(format!("data source {} is laid over by mapping {name}, which no module declares", source.name), span).in_file(&file.path));
            }
            continue;
        };
        if !project.entities_in(&source.name).any(|e| e.name.name == mapping.entity.name) {
            let file = project.files.iter().find(|f| f.mappings.iter().any(|m| std::ptr::eq(m, mapping)));
            found.push(
                Diagnostic::error(
                    format!("data source {} is laid over by mapping {name}, so {} lives in it: write 'source {}' in the module of {}", source.name, mapping.entity.name, source.name, mapping.entity.name),
                    mapping.name.span,
                )
                .in_file(file.map(|f| f.path.as_str()).unwrap_or_default()),
            );
        }
        // The records inside (`lines of Line`) are kept in the same file, so their entity lives in the source too.
        let file = project.files.iter().find(|f| f.mappings.iter().any(|m| std::ptr::eq(m, mapping)));
        // Each with the record around it, so a record made in code knows where in the file it goes.
        let mut inside: Vec<(&str, &crate::code::MappedField, &crate::code::MappedInside)> =
            mapping.fields.iter().filter_map(|f| f.inside.as_ref().map(|i| (mapping.entity.name.as_str(), f, i))).collect();
        while let Some((outer, field, nested)) = inside.pop() {
            inside.extend(nested.fields.iter().filter_map(|f| f.inside.as_ref().map(|i| (nested.entity.name.as_str(), f, i))));
            let entity = &nested.entity.name;
            if nested.parent.is_none() {
                found.push(
                    Diagnostic::error(
                        format!(
                            "data source {} keeps the {entity} records inside each {outer}, so mapping {name} names the {outer} a {entity} sits in: write '{} is \"..\"' under '{} of {entity}', with {}: ref {outer} on {entity}",
                            source.name,
                            lower_first(outer),
                            field.field.name,
                            lower_first(outer)
                        ),
                        nested.entity.span,
                    )
                    .in_file(file.map(|f| f.path.as_str()).unwrap_or_default()),
                );
            }
            if project.entity(entity).is_some() && !project.entities_in(&source.name).any(|e| e.name.name == *entity) {
                found.push(
                    Diagnostic::error(format!("data source {} is laid over by mapping {name}, so {entity} lives in it: write 'source {}' in the module of {entity}", source.name, source.name), nested.entity.span)
                        .in_file(file.map(|f| f.path.as_str()).unwrap_or_default()),
                );
            }
        }
    }
    found
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

/// What a command with a key does: a menu item and a button that open the
/// same window or run the same handler are one command (manual/app.keys.md).
#[derive(PartialEq)]
enum Does<'a> {
    Open(&'a str, bool),
    Run(&'a str, &'a [crate::model::Literal]),
    /// Nothing yet: only itself.
    Only(Span),
}

/// A command with a key, where it is written.
struct Keyed<'a> {
    key: &'a str,
    name: String,
    does: Does<'a>,
    file: usize,
    span: Span,
}

/// The keys of a frame's menu items and toolbar and ribbon buttons.
fn frame_keys<'a>(file: usize, menu: &'a Option<crate::model::Menu>, toolbar: &'a Option<crate::model::Toolbar>, ribbon: &'a Option<crate::model::Ribbon>, out: &mut Vec<Keyed<'a>>) {
    fn commands<'a>(file: usize, items: &'a [Command], out: &mut Vec<Keyed<'a>>) {
        for c in items {
            if let Some(key) = &c.props.shortcut {
                let does = match (&c.props.on_click, &c.props.handler) {
                    (Some(target), _) => Does::Open(target, c.props.new_window),
                    (None, Some(handler)) => Does::Run(handler, &[]),
                    (None, None) => Does::Only(c.span),
                };
                let name = c.props.name.clone().or_else(|| c.props.caption.clone()).unwrap_or_else(|| frame::command_keyword(c.props.kind).to_string());
                out.push(Keyed { key, name, does, file, span: c.span });
            }
            commands(file, &c.items, out);
        }
    }
    if let Some(menu) = menu {
        commands(file, &menu.items, out);
    }
    if let Some(toolbar) = toolbar {
        commands(file, &toolbar.items, out);
    }
    if let Some(ribbon) = ribbon {
        for group in ribbon.tabs.iter().flat_map(|t| &t.groups) {
            commands(file, &group.items, out);
        }
    }
}

/// How a page's kind reads in a message: `window Card`, `dialog Ask`.
fn page_kind_word(kind: PageKind) -> &'static str {
    kind.word()
}

/// A name with its first letter lower case: `Order` is `order`.
fn lower_first(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map(|c| c.to_lowercase().chain(chars).collect()).unwrap_or_default()
}
