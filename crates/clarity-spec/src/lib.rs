//! Clarity spec language: lexer, parser, checker and the neutral model.

pub mod check;
pub mod code;
pub mod diagnostic;
pub mod dictionary;
pub mod edit;
pub mod formats;
pub mod engines;
pub mod images;
pub mod keyed;
pub mod themes;
pub mod translations;
pub mod frame;
pub mod layout;
pub mod lexer;
pub mod migrate;
pub mod model;
pub mod names;
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
    diagnostics.extend(check::check(&files));
    diagnostics.extend(files.iter().flat_map(themes::check_decls));
    Project {
        files,
        diagnostics,
        migrations: None,
        sources: Vec::new(),
        locale: model::Locale::default(),
        themes: Vec::new(),
        images: None,
        engines: None,
        stores: None,
        translations: BTreeMap::new(),
    }
}

/// Window data read as a record (`data pending: Operation`) whose type is
/// an enum, and no entity, becomes a plain value: the parser reads one file
/// and cannot tell.
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
}

impl SourceRef {
    pub fn sql(name: &str) -> SourceRef {
        SourceRef {
            name: name.to_string(),
            sql: true,
            external: false,
            readonly: false,
        }
    }

    pub fn web(name: &str) -> SourceRef {
        SourceRef {
            name: name.to_string(),
            sql: false,
            external: true,
            readonly: false,
        }
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
    if let Some(found) = images::image_files(dir)? {
        images::attach(&mut project, found);
    }
    // A declared engine needs its file, so the folder is read even when it is missing.
    engines::attach(&mut project, engines::engine_files(dir)?.unwrap_or_default());
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

pub fn spec_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == SPEC_EXTENSION))
        .collect();
    paths.sort();
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
}

impl Preview {
    pub fn new(project: Project) -> Self {
        let samples = sample::sample_data(&project);
        let displays = sample::display_values(&project);
        Self {
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
