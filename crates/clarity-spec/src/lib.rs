//! Clarity spec language: lexer, parser, checker and the neutral model.

pub mod check;
pub mod colors;
pub mod code;
pub mod diagnostic;
pub mod diagrams;
pub mod dictionary;
pub mod edit;
pub mod formats;
pub mod engines;
pub mod resources;
pub mod sound;
pub mod keyed;
pub mod themes;
pub mod translations;
pub mod frame;
pub mod highlight;
pub mod icons;
pub mod layout;
pub mod lexer;
pub mod migrate;
pub mod model;
pub mod names;
pub mod native;
pub mod parser;
pub mod rename;
pub mod sample;
pub mod source;
pub mod span;
pub mod templates;
pub mod widgets;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

pub use diagnostic::{Diagnostic, Severity};
pub use model::Project;
pub use span::Span;

pub const SPEC_EXTENSION: &str = "clarity";

/// A short, stable hash of a source text (64-bit FNV-1a, as hex).
pub fn content_version(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Parses and checks a set of in-memory sources given as (path, text).
pub fn load_sources<'a>(sources: impl IntoIterator<Item = (&'a str, &'a str)>) -> Project {
    let mut files = Vec::new();
    let mut diagnostics = Vec::new();
    for (path, text) in sources {
        let (file, file_diagnostics) = parser::parse(text, path);
        files.push(file);
        diagnostics.extend(file_diagnostics);
    }
    resolve_data(&mut files);
    give_settings(&mut files);
    make_form_labels(&mut files);
    make_hears(&mut files);
    diagnostics.extend(check::check(&files));
    diagnostics.extend(files.iter().flat_map(themes::check_decls));
    Project {
        files,
        diagnostics,
        migrations: None,
        sources: Vec::new(),
        locale: model::Locale::default(),
        themes: Vec::new(),
        resources: None,
        diagrams: Vec::new(),
        resource_keys: Default::default(),
        engines: None,
        part_files: None,
        stores: None,
        translations: BTreeMap::new(),
    }
}

/// A form's label column (`labels 3`): a label part for each input in it,
/// in the first columns of the input's row, showing the field's label and
/// named after the input (`nameBoxLabel`), as the designer would write one.
/// A checkbox, a switch and a toggle label themselves.
fn make_form_labels(files: &mut [model::SpecFile]) {
    use widgets::WidgetKind;
    let entities: BTreeMap<String, model::Entity> = files.iter().flat_map(|f| &f.entities).map(|e| (e.name.name.clone(), e.clone())).collect();
    fn walk(widgets: &mut [model::Widget], data: &[model::DataDecl], entities: &BTreeMap<String, model::Entity>) {
        for widget in widgets.iter_mut() {
            if widget.props.kind == WidgetKind::Form
                && let Some(columns) = widget.props.labels
            {
                let entity = widget
                    .props
                    .bind
                    .as_deref()
                    .and_then(|b| data.iter().find(|d| d.name.name == b))
                    .and_then(|d| d.entity())
                    .and_then(|e| entities.get(&e.name));
                let mut made = Vec::new();
                for child in &widget.children {
                    let p = &child.props;
                    let labels_itself = matches!(p.kind, WidgetKind::Checkbox | WidgetKind::Switch | WidgetKind::Toggle);
                    if !widgets::info(p.kind).input || labels_itself || p.made {
                        continue;
                    }
                    let (Some(name), Some(field)) = (&p.name, p.bind.as_deref().and_then(|b| b.split_once('.')).and_then(|(_, f)| entity.and_then(|e| e.field(f)))) else { continue };
                    let mut label = widgets::new_props(WidgetKind::Label, 1, p.row);
                    label.width = columns;
                    label.caption = Some(field.modifiers.label.clone().unwrap_or_else(|| model::default_label(&field.name.name)));
                    label.name = Some(format!("{name}Label"));
                    label.for_part = Some(name.clone());
                    label.made = true;
                    made.push(model::Widget { props: label, children: Vec::new(), pages: Vec::new(), menu: None, span: child.span });
                }
                made.append(&mut widget.children);
                widget.children = made;
            }
            walk(&mut widget.children, data, entities);
            for page in &mut widget.pages {
                walk(&mut page.children, data, entities);
            }
        }
    }
    for file in files.iter_mut() {
        for page in &mut file.pages {
            walk(&mut page.widgets, &page.data, &entities);
        }
    }
}

/// `arena.restart(2)` reads as a shape drawn on a canvas; where `arena` is
/// a custom part of the window, it is a hear of that part (DESIGN 5.19).
fn make_hears(files: &mut [model::SpecFile]) {
    use code::{Stmt, StmtKind};
    fn custom_parts(widgets: &[model::Widget], found: &mut Vec<String>) {
        for widget in widgets {
            // A custom part and a sound hear what code tells them (DESIGN 5.19, 5.22).
            if matches!(widget.props.kind, widgets::WidgetKind::Custom | widgets::WidgetKind::Sound)
                && let Some(name) = &widget.props.name
            {
                found.push(name.clone());
            }
            custom_parts(&widget.children, found);
            for page in &widget.pages {
                custom_parts(&page.children, found);
            }
        }
    }
    fn walk(body: &mut [Stmt], parts: &[String]) {
        for stmt in body.iter_mut() {
            match &mut stmt.kind {
                StmtKind::Draw { canvas, shape, args } if parts.contains(&canvas.name) => {
                    stmt.kind = StmtKind::Hear { part: canvas.clone(), hear: shape.clone(), args: std::mem::take(args) };
                }
                StmtKind::If { branches, otherwise } => {
                    for branch in branches {
                        walk(&mut branch.body, parts);
                    }
                    if let Some(body) = otherwise {
                        walk(body, parts);
                    }
                }
                StmtKind::For { body, .. } => walk(body, parts),
                _ => {}
            }
        }
    }
    for file in files.iter_mut() {
        for block in &mut file.code {
            let mut parts = Vec::new();
            for page in file.pages.iter().filter(|p| p.name.name == block.name.name) {
                custom_parts(&page.widgets, &mut parts);
                for sidebar in &page.frame.sidebars {
                    custom_parts(&sidebar.widgets, &mut parts);
                }
            }
            if parts.is_empty() {
                continue;
            }
            for handler in &mut block.handlers {
                walk(&mut handler.body, &parts);
            }
        }
    }
}

/// With a `settings` block, every window and the app has the settings as
/// data of its own, `settings`, without a `data` line (DESIGN 5.17).
fn give_settings(files: &mut [model::SpecFile]) {
    use parser::{SETTINGS_DATA, SETTINGS_ENTITY};
    let Some(span) = files.iter().flat_map(|f| &f.entities).find(|e| e.settings).map(|e| e.span) else { return };
    let decl = model::DataDecl {
        name: model::Ident { name: SETTINGS_DATA.to_string(), span },
        of: model::DataKind::Record { entity: model::Ident { name: SETTINGS_ENTITY.to_string(), span } },
        made: true,
        span,
    };
    for file in files.iter_mut() {
        let data = file.pages.iter_mut().map(|p| &mut p.data).chain(file.apps.iter_mut().map(|a| &mut a.data));
        for list in data {
            if !list.iter().any(|d| d.name.name == SETTINGS_DATA) {
                list.push(decl.clone());
            }
        }
    }
}

/// Window data read as a record (`data pending: Operation`) whose type is
/// an enum, and no entity, becomes a plain value, and a field `list<Genre>`
/// of an enum a list of values: the parser reads one file and cannot tell.
fn resolve_data(files: &mut [model::SpecFile]) {
    use model::DataKind;
    let enums: std::collections::HashSet<String> = files.iter().flat_map(|f| &f.enums).map(|e| e.name.name.clone()).collect();
    let entities: std::collections::HashSet<String> = files.iter().flat_map(|f| &f.entities).map(|e| e.name.name.clone()).collect();
    for file in files.iter_mut() {
        let data = file.pages.iter_mut().flat_map(|p| &mut p.data).chain(file.apps.iter_mut().flat_map(|a| &mut a.data));
        for decl in data {
            if let DataKind::Record { entity } = &decl.of
                && enums.contains(&entity.name)
                && !entities.contains(&entity.name)
            {
                let ty = code::TypeRef { kind: code::TypeRefKind::Named { name: entity.name.clone() }, span: entity.span };
                decl.of = DataKind::Value { ty, default: None };
            }
        }
        // A field `list<Genre>` of an enum is a list of values (U1c), and a
        // field named after an entity holds a record of it (DESIGN 5.25).
        for field in file.entities.iter_mut().flat_map(|e| &mut e.fields) {
            if let model::FieldType::Enum { name } = &field.ty
                && entities.contains(&name.name)
                && !enums.contains(&name.name)
            {
                field.ty = model::FieldType::Record { entity: name.clone() };
            }
            if let model::FieldType::List { item } = &field.ty
                && enums.contains(&item.name)
                && !entities.contains(&item.name)
            {
                field.ty = model::FieldType::Values { item: model::ListItem::Enum { name: item.clone() } };
            }
        }
    }
}

/// A data source of the project, as the checker needs to know it. The
/// project file declares them; `clarity-project` hands them in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRef {
    pub name: String,
    /// A SQL database, whose records Clarity reads and writes itself.
    pub sql: bool,
    /// A database Clarity does not own: no migration steps, and its
    /// entities map onto existing tables.
    pub external: bool,
    /// Records are read, never saved or deleted.
    pub readonly: bool,
    /// A web API's address, which `http ... from source` calls paths of.
    pub address: String,
    /// A web API's auth: its kind as the project file writes it (`bearer`,
    /// `api-key`, `basic`) and where its secret is (`env:NAME`).
    pub auth: Option<(String, String)>,
}

impl SourceRef {
    pub fn sql(name: &str) -> SourceRef {
        SourceRef {
            name: name.to_string(),
            sql: true,
            external: false,
            readonly: false,
            address: String::new(),
            auth: None,
        }
    }

    pub fn web(name: &str) -> SourceRef {
        SourceRef {
            name: name.to_string(),
            sql: false,
            external: true,
            readonly: false,
            address: String::new(),
            auth: None,
        }
    }

    /// A web API at `address`, signed with `auth` (kind, where the secret is).
    pub fn at(mut self, address: &str, auth: Option<(&str, &str)>) -> SourceRef {
        self.address = address.to_string();
        self.auth = auth.map(|(kind, secret)| (kind.to_string(), secret.to_string()));
        self
    }

    pub fn external(mut self) -> SourceRef {
        self.external = true;
        self
    }

    pub fn readonly(mut self) -> SourceRef {
        self.readonly = true;
        self
    }
}

/// Gives the project the file stores of its settings, by name, and checks
/// that every image field with a `store` names one of them.
pub fn attach_stores(project: &mut Project, stores: Vec<String>) {
    let mut found = Vec::new();
    for file in &project.files {
        for entity in &file.entities {
            for field in &entity.fields {
                let Some(store) = &field.modifiers.store else { continue };
                if stores.contains(store) {
                    continue;
                }
                let have = if stores.is_empty() { "add it there".to_string() } else { format!("they have {}", stores.join(", ")) };
                found.push(
                    diagnostic::Diagnostic::error(
                        format!("'{}' is kept in file store '{store}', which the project settings do not have; {have}", field.name.name),
                        field.span,
                    )
                    .in_file(&file.path),
                );
            }
        }
    }
    for (path, store) in code::check::code_stores(project) {
        if !stores.contains(&store.name) {
            let have = if stores.is_empty() { "add one in the project settings".to_string() } else { format!("the project settings have {}", stores.join(", ")) };
            found.push(diagnostic::Diagnostic::error(format!("there is no file store '{}'; {have}", store.name), store.span).in_file(&path));
        }
    }
    project.diagnostics.extend(found);
    project.stores = Some(stores);
}

/// The data source of a project loaded without its project file.
pub const DEFAULT_SOURCE: &str = "main";

/// Like `load_sources`, with the migration steps given as (path, text) in
/// order, checked against the dictionary. A step belongs to the data
/// source its folder is named after (`migrations/main/0001.clarity`), or to
/// the first source when its path has no folder.
pub fn load_with_migrations<'a>(
    sources: impl IntoIterator<Item = (&'a str, &'a str)>,
    migrations: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Project {
    load_in(sources, migrations, &[])
}

/// Like `load_with_migrations`, for a project with these data sources.
/// Without any, the project has one SQL source called `main`.
pub fn load_in<'a>(
    files: impl IntoIterator<Item = (&'a str, &'a str)>,
    migrations: impl IntoIterator<Item = (&'a str, &'a str)>,
    sources: &[SourceRef],
) -> Project {
    let mut project = load_sources(files);
    project.sources = if sources.is_empty() { vec![SourceRef::sql(DEFAULT_SOURCE)] } else { sources.to_vec() };
    let mut steps: BTreeMap<String, Vec<migrate::Migration>> = BTreeMap::new();
    for (path, text) in migrations {
        let (migration, diagnostics) = migrate::parse(text, path);
        project.diagnostics.extend(diagnostics);
        let source = Path::new(path)
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| project.default_source().to_string());
        steps.entry(source).or_default().push(migration);
    }
    project.migrations = Some(steps);
    check::check_sources(&mut project);
    code::check::check_readonly(&mut project);
    code::check::check_query_sources(&mut project);
    code::check::check_web_sources(&mut project);
    migrate::check(&mut project);
    project
}

/// The folder of migration steps in a project's folder.
pub const MIGRATIONS_DIR: &str = "migrations";

/// The step files of the project in `dir`, as (path, text): per data
/// source a folder, `migrations/main/0001.clarity`, in order.
pub fn migration_sources(dir: &Path) -> std::io::Result<Vec<(String, String)>> {
    let folder = dir.join(MIGRATIONS_DIR);
    if !folder.is_dir() {
        return Ok(Vec::new());
    }
    let mut folders: Vec<PathBuf> = std::fs::read_dir(&folder)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    let mut steps = Vec::new();
    for source in folders {
        for path in spec_files(&source)? {
            steps.push((path.display().to_string(), std::fs::read_to_string(&path)?));
        }
    }
    Ok(steps)
}

/// Loads every `.clarity` file in `dir` (not recursive), sorted by name,
/// with its migration steps, for a project with these data sources.
pub fn load_dir_in(dir: &Path, sources: &[SourceRef]) -> std::io::Result<Project> {
    load_dir_with_in(dir, None, sources)
}

/// Like `load_dir_in`, but with `text` in place of the file at `path`. Used
/// to check an edit against the whole project before it is written.
pub fn load_dir_with_in(dir: &Path, replaced: Option<(&Path, &str)>, sources: &[SourceRef]) -> std::io::Result<Project> {
    load_dir_with_files_in(dir, replaced.as_slice(), sources)
}

/// Like `load_dir_in`, with `replaced` (a module, a language file, one not
/// written yet) in place of what the folder holds. Used to check an edit
/// that writes several files against the whole project first.
pub fn load_dir_with_files_in(dir: &Path, replaced: &[(&Path, &str)], sources: &[SourceRef]) -> std::io::Result<Project> {
    let mut files = Vec::new();
    for p in spec_files(dir)? {
        let content = match replaced.iter().find(|(path, _)| *path == p) {
            Some((_, text)) => text.to_string(),
            None => std::fs::read_to_string(&p)?,
        };
        files.push((p.display().to_string(), content));
    }
    let steps = migration_sources(dir)?;
    let mut project = load_in(
        files.iter().map(|(p, t)| (p.as_str(), t.as_str())),
        steps.iter().map(|(p, t)| (p.as_str(), t.as_str())),
        sources,
    );
    let translations = translation_sources(dir, replaced)?;
    translations::attach(&mut project, translations.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    // A folder without resources has none: what a window shows of them is checked all the same.
    resources::attach(&mut project, resources::resource_files(dir)?.unwrap_or_default());
    // A declared engine needs its file, so the folder is read even when it is missing.
    engines::attach(&mut project, engines::engine_files(dir)?.unwrap_or_default());
    // A declared native part needs its folder, so a missing one is reported.
    let part_files = native::native_files(dir, &project)?;
    native::attach(&mut project, part_files);
    project.diagrams = diagrams::diagram_files(dir)?;
    Ok(project)
}

/// The translation files of the project in `dir`, as (path, text): one
/// per language, `translations/nl.clarity`. `replaced` stands in for the
/// file it names, as it does for the modules.
pub fn translation_sources(dir: &Path, replaced: &[(&Path, &str)]) -> std::io::Result<Vec<(String, String)>> {
    let folder = dir.join(translations::TRANSLATIONS_DIR);
    let mut paths = if folder.is_dir() { spec_files(&folder)? } else { Vec::new() };
    // A language file about to be written, which is not on disk yet.
    for (path, _) in replaced {
        if path.parent() == Some(folder.as_path()) && !paths.iter().any(|p| p == path) {
            paths.push(path.to_path_buf());
        }
    }
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let text = match replaced.iter().find(|(path, _)| *path == p) {
                Some((_, text)) => text.to_string(),
                None => std::fs::read_to_string(&p)?,
            };
            Ok((p.display().to_string(), text))
        })
        .collect()
}

/// `load_dir_in` for a project whose only data source is `main`.
pub fn load_dir(dir: &Path) -> std::io::Result<Project> {
    load_dir_in(dir, &[])
}

/// `load_dir_with_in` for a project whose only data source is `main`.
pub fn load_dir_with(dir: &Path, path: &Path, text: &str) -> std::io::Result<Project> {
    load_dir_with_in(dir, Some((path, text)), &[])
}

/// The folder of a project's web API descriptions, beside its modules, and
/// of the modules Clarity writes from them (DESIGN 5.23).
pub const APIS_DIR: &str = "apis";

/// The module files of the project folder `dir`: its own, then those Clarity
/// writes into its `apis` folder.
pub fn spec_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let modules = |dir: &Path| -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == SPEC_EXTENSION))
            .collect();
        paths.sort();
        paths
    };
    std::fs::read_dir(dir)?;
    let mut paths = modules(dir);
    paths.extend(modules(&dir.join(APIS_DIR)));
    Ok(paths)
}

/// The first error in `after` that `before` does not have. Positions shift
/// with edits, so errors are compared by file and message.
pub fn new_error<'a>(before: &Project, after: &'a Project) -> Option<&'a Diagnostic> {
    let known: Vec<(&str, &str)> = before
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| (d.path.as_str(), d.message.as_str()))
        .collect();
    after.diagnostics.iter().find(|d| {
        d.severity == Severity::Error && !known.contains(&(d.path.as_str(), d.message.as_str()))
    })
}

/// Everything a preview needs: the checked model, sample rows per entity,
/// the display text of each sample row and the widget catalog.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Preview {
    pub project: Project,
    pub samples: BTreeMap<String, Vec<sample::Row>>,
    pub displays: BTreeMap<String, Vec<String>>,
    pub catalog: Vec<widgets::WidgetInfo>,
    /// The frame regions the Parts panel offers.
    pub frame: Vec<frame::RegionInfo>,
    /// Click targets that are not windows (`quit`, `close`).
    pub commands: Vec<String>,
    /// The events a window has, and the events the app has.
    #[serde(rename = "windowEvents")]
    #[ts(rename = "windowEvents")]
    pub window_events: Vec<widgets::Event>,
    #[serde(rename = "appEvents")]
    #[ts(rename = "appEvents")]
    pub app_events: Vec<widgets::Event>,
    /// The named formats of each field type, by the type's kind.
    pub formats: BTreeMap<&'static str, &'static [formats::FormatInfo]>,
    /// Clarity's own theme, which every theme starts from.
    #[serde(rename = "baseTheme")]
    #[ts(rename = "baseTheme")]
    pub base_theme: model::ThemeDecl,
    /// The icons the project shows, by name, each an SVG path in a 24 by 24 box.
    pub icons: BTreeMap<String, String>,
    /// Every icon's name, for the designer to offer.
    #[serde(rename = "iconNames")]
    #[ts(rename = "iconNames")]
    pub icon_names: Vec<String>,
}

impl Preview {
    pub fn new(project: Project) -> Self {
        let samples = sample::sample_data(&project);
        let displays = sample::display_values(&project);
        let icons = icons::used(&project);
        Self {
            icons,
            icon_names: icons::names().map(str::to_string).collect(),
            project,
            samples,
            displays,
            catalog: widgets::catalog(),
            frame: frame::catalog(),
            commands: frame::BUILT_IN_COMMANDS
                .iter()
                .map(|c| c.to_string())
                .collect(),
            window_events: widgets::WINDOW_EVENTS.to_vec(),
            app_events: widgets::APP_EVENTS.to_vec(),
            formats: formats::catalog(),
            base_theme: themes::built_in(),
        }
    }
}

/// Writes `ops` as the next step of data source `source`, in the
/// `migrations` folder of the project's module folder `dir`, and returns
/// its path.
pub fn write_step(dir: &Path, source: &str, title: &str, ops: &[migrate::Op]) -> std::io::Result<PathBuf> {
    if ops.is_empty() {
        return Err(std::io::Error::other("there are no changes to migrate"));
    }
    write_step_text(dir, source, &migrate::step_text(title, ops))
}

/// Writes `text` as the next step file of data source `source`.
pub fn write_step_text(dir: &Path, source: &str, text: &str) -> std::io::Result<PathBuf> {
    let folder = dir.join(MIGRATIONS_DIR).join(source);
    std::fs::create_dir_all(&folder)?;
    let existing: Vec<String> = spec_files(&folder)?
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    let path = folder.join(migrate::next_file_name(&existing));
    std::fs::write(&path, text)?;
    Ok(path)
}
