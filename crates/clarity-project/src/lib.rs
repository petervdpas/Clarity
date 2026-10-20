//! The project file, `<name>.clarity`: what the project is transpiled into
//! (backend, UI, targets), where its data lives (data sources) and where it
//! is built. The modules sit in the folder of the same name next to it. See
//! docs/DESIGN.md sections 1.2 and 1.3.

pub mod engines;
mod file;

use std::path::{Path, PathBuf};

use clarity_spec::widgets::is_identifier;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use file::{parse, write};

/// The extension of project files, the same as module files.
pub const PROJECT_EXTENSION: &str = "clarity";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Backend {
    Rust,
    Go,
    Csharp,
    C,
    Cpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Ui {
    Web,
    Avalonia,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Target {
    Desktop,
    Web,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum SourceKind {
    Sqlite,
    Postgres,
    Mysql,
    Mssql,
    Rest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum AuthKind {
    Bearer,
    ApiKey,
    Basic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Stack {
    pub backend: Backend,
    pub ui: Ui,
    pub targets: Vec<Target>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Auth {
    pub kind: AuthKind,
    /// Where the secret is kept: `env:NAME` or `keychain:NAME`.
    pub secret: String,
}

/// A folder that keeps the pictures of image fields with `store <name>` as
/// files, such as a folder of photos a catalog shows. Where it is differs
/// per installation, like a data source: a folder, or `env:NAME`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FileStore {
    pub name: String,
    pub location: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DataSource {
    pub name: String,
    pub kind: SourceKind,
    /// A file path for SQLite, a connection for the SQL servers, a base URL
    /// for web APIs.
    pub url: String,
    #[serde(default)]
    pub auth: Option<Auth>,
    /// A database Clarity does not own: it reads and writes its records,
    /// but never changes its schema.
    #[serde(default)]
    pub external: bool,
    /// Records are read, never saved or deleted.
    #[serde(default)]
    pub readonly: bool,
}

/// The settings of a project, as the Project tab shows them. The first data
/// source is the default one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Settings {
    /// The project's name, a Clarity name such as `Shop`; also the program's.
    pub name: String,
    /// The name people see, when it differs from the one derived from `name`.
    pub title: Option<String>,
    pub stack: Stack,
    pub datasources: Vec<DataSource>,
    /// The language the project's texts are written in, as a language tag
    /// (`en`, `nl-NL`). Formats follow the locale of the person using the
    /// application; this one where the system gives none.
    #[serde(default)]
    pub language: Option<String>,
    /// The currency of `money` fields, as its ISO 4217 code (`EUR`).
    #[serde(default)]
    pub currency: Option<String>,
    /// The themes the application offers, by name (`theme Shop Night`), the
    /// first being the one it starts with; without any,
    /// it looks as Clarity does.
    #[serde(default)]
    pub themes: Vec<String>,
    /// The file stores of image fields kept as files, by name.
    #[serde(default)]
    pub filestores: Vec<FileStore>,
    /// Where the application is generated and built: a folder relative to
    /// the project's folder, or an absolute one.
    pub out: String,
}

/// The output folder when the project file does not name one.
pub const DEFAULT_OUT: &str = "out";

pub const BACKENDS: [Backend; 5] = [Backend::Rust, Backend::Go, Backend::Csharp, Backend::C, Backend::Cpp];
pub const UIS: [Ui; 2] = [Ui::Web, Ui::Avalonia];
pub const TARGETS: [Target; 2] = [Target::Desktop, Target::Web];
pub const SOURCE_KINDS: [SourceKind; 5] =
    [SourceKind::Sqlite, SourceKind::Postgres, SourceKind::Mysql, SourceKind::Mssql, SourceKind::Rest];
pub const AUTH_KINDS: [AuthKind; 3] = [AuthKind::Bearer, AuthKind::ApiKey, AuthKind::Basic];

// The words of the project file.

pub fn backend_word(backend: Backend) -> &'static str {
    match backend {
        Backend::Rust => "rust",
        Backend::Go => "go",
        Backend::Csharp => "csharp",
        Backend::C => "c",
        Backend::Cpp => "cpp",
    }
}

pub fn ui_word(ui: Ui) -> &'static str {
    match ui {
        Ui::Web => "web",
        Ui::Avalonia => "avalonia",
    }
}

pub fn target_word(target: Target) -> &'static str {
    match target {
        Target::Desktop => "desktop",
        Target::Web => "web",
    }
}

pub fn source_word(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Sqlite => "sqlite",
        SourceKind::Postgres => "postgres",
        SourceKind::Mysql => "mysql",
        SourceKind::Mssql => "mssql",
        SourceKind::Rest => "rest",
    }
}

pub fn auth_word(kind: AuthKind) -> &'static str {
    match kind {
        AuthKind::Bearer => "bearer",
        AuthKind::ApiKey => "api_key",
        AuthKind::Basic => "basic",
    }
}

/// The settings of a new project: Rust with the web renderer on the
/// desktop, and one SQLite database.
pub fn starter(name: &str, module: &str) -> Settings {
    Settings {
        name: name.to_string(),
        title: None,
        stack: Stack {
            backend: Backend::Rust,
            ui: Ui::Web,
            targets: vec![Target::Desktop],
        },
        datasources: vec![DataSource {
            name: "main".to_string(),
            kind: SourceKind::Sqlite,
            url: format!("data/{module}.db"),
            auth: None,
            external: false,
            readonly: false,
        }],
        language: None,
        currency: None,
        themes: Vec::new(),
        filestores: Vec::new(),
        out: DEFAULT_OUT.to_string(),
    }
}

/// The name people see: the title, or one derived from the name.
pub fn title(settings: &Settings) -> String {
    settings
        .title
        .clone()
        .unwrap_or_else(|| clarity_spec::model::default_label(&settings.name))
}

/// The folder next to a project file that holds its modules:
/// `shop.clarity` has `shop/`.
pub fn module_dir(file: &Path) -> PathBuf {
    file.with_extension("")
}

/// The output folder of the project file `file`: relative to its folder,
/// or an absolute path as it is.
pub fn out_dir(file: &Path, settings: &Settings) -> PathBuf {
    module_dir(file).join(settings.out.trim())
}

/// A path picked in a file dialog, as the project file stores it: relative
/// to the project's folder when it is inside it (so the project can move),
/// the full path otherwise.
pub fn project_path(file: &Path, picked: &Path) -> String {
    match picked.strip_prefix(module_dir(file)) {
        Ok(inside) if !inside.as_os_str().is_empty() => inside
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/"),
        _ => picked.display().to_string(),
    }
}

/// What a project file holds, for the Project tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ProjectFile {
    /// None when the file cannot be read.
    pub settings: Option<Settings>,
    /// The version of the file text, to refuse saving over a newer file.
    pub version: String,
    /// The name people see (see `title`); empty when the file cannot be read.
    pub title: String,
    pub problems: Vec<String>,
    pub catalog: Catalog,
}

pub fn load(path: &Path) -> ProjectFile {
    let file = |settings: Option<Settings>, version, problems| ProjectFile {
        title: settings.as_ref().map(title).unwrap_or_default(),
        settings,
        version,
        problems,
        catalog: catalog(),
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => return file(None, String::new(), vec![format!("cannot read {}: {err}", path.display())]),
    };
    let version = clarity_spec::content_version(&text);
    match parse(&text) {
        Ok(settings) => {
            let problems = validate(&settings);
            file(Some(settings), version, problems)
        }
        Err(err) => file(None, version, vec![err]),
    }
}

pub use clarity_spec::formats::language_problem;

/// Why `code` is not a currency code: three uppercase letters (`EUR`, `USD`).
pub fn currency_problem(code: &str) -> Option<String> {
    let fits = code.len() == 3 && code.chars().all(|c| c.is_ascii_uppercase());
    (!fits).then(|| format!("'{code}' is not a currency code; write its ISO code, like EUR or USD"))
}

/// Everything wrong with the settings, in reading order. Empty when they
/// can be saved.
pub fn validate(settings: &Settings) -> Vec<String> {
    let mut problems = Vec::new();
    if !is_identifier(&settings.name) {
        problems.push(format!(
            "'{}' is not a valid project name; use a name like MyShop, and a title for the text people see",
            settings.name
        ));
    }
    // Generation removes what it no longer produces, so the output folder
    // can never be the project folder itself.
    let out = settings.out.trim().trim_end_matches(['/', '\\']);
    if out.is_empty() || out == "." {
        problems.push("the output folder must be a folder of its own, like out".to_string());
    }
    problems.extend(settings.language.as_deref().and_then(language_problem));
    problems.extend(settings.currency.as_deref().and_then(currency_problem));
    let stack = &settings.stack;
    let backend = backend_info(stack.backend);
    if !backend.uis.contains(&stack.ui) {
        problems.push(format!(
            "{} is a UI for the {} backend, not for {}",
            ui_label(stack.ui),
            ui_backend_label(stack.ui),
            backend.label
        ));
    }
    if stack.targets.is_empty() {
        problems.push("the project needs at least one target".to_string());
    }
    for (i, target) in stack.targets.iter().enumerate() {
        if stack.targets[..i].contains(target) {
            problems.push(format!("the target {} is listed twice", target_word(*target)));
        }
    }
    for (i, source) in settings.datasources.iter().enumerate() {
        if !is_identifier(&source.name) {
            problems.push(format!("'{}' is not a valid data source name", source.name));
        } else if settings.datasources[..i].iter().any(|s| s.name == source.name) {
            problems.push(format!("there are two data sources named '{}'", source.name));
        }
        if let Some(problem) = source_problem(source) {
            problems.push(format!("data source '{}': {problem}", source.name));
        }
        if source.kind == SourceKind::Rest && source.external {
            problems.push(format!(
                "data source '{}': a web API is always external; leave out 'external'",
                source.name
            ));
        }
    }
    for (i, store) in settings.filestores.iter().enumerate() {
        if !is_identifier(&store.name) {
            problems.push(format!("'{}' is not a valid file store name", store.name));
        } else if settings.filestores[..i].iter().any(|s| s.name == store.name) {
            problems.push(format!("there are two file stores named '{}'", store.name));
        }
        if store.location.trim().is_empty() {
            problems.push(format!("file store '{}' needs a folder", store.name));
        }
    }
    problems
}

fn is_reference(text: &str) -> bool {
    ["env:", "keychain:"]
        .iter()
        .any(|prefix| text.strip_prefix(prefix).is_some_and(|rest| !rest.trim().is_empty()))
}

/// True when a connection URL (`scheme://user:password@host`) or a
/// connection string (`...;Password=...`) carries a password.
fn has_password(url: &str) -> bool {
    if let Some((_, rest)) = url.split_once("://") {
        let authority = rest.split('/').next().unwrap_or("");
        if let Some((user, _)) = authority.rsplit_once('@') {
            return user.contains(':');
        }
    }
    url.split(';').any(|part| {
        let key = part.split('=').next().unwrap_or("").trim().to_ascii_lowercase();
        part.contains('=') && (key == "password" || key == "pwd")
    })
}

fn source_problem(source: &DataSource) -> Option<String> {
    let url = source.url.trim();
    if url.is_empty() {
        return Some("the connection is empty".to_string());
    }
    if source.auth.is_some() && source.kind != SourceKind::Rest {
        return Some("only web API sources have auth".to_string());
    }
    match source.kind {
        SourceKind::Sqlite if is_reference(url) || url.contains("://") => {
            Some("a SQLite source is a file path, like data/shop.db".to_string())
        }
        SourceKind::Rest if !(url.starts_with("http://") || url.starts_with("https://")) => {
            Some("the base URL must start with http:// or https://".to_string())
        }
        _ if has_password(url) => Some(
            "the connection contains a password; put it in an environment variable (env:NAME) or the keychain (keychain:NAME)"
                .to_string(),
        ),
        _ => match &source.auth {
            Some(auth) if !is_reference(auth.secret.trim()) => Some(
                "the secret must be a reference (env:NAME or keychain:NAME), not the secret itself".to_string(),
            ),
            _ => None,
        },
    }
}

// The catalog: what the Project tab offers, so the frontend never keeps its
// own list of backends, toolkits or databases.

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct BackendInfo {
    pub id: Backend,
    pub label: &'static str,
    /// The UIs this backend can render with; the first is the usual one.
    pub uis: Vec<Ui>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Choice<T> {
    pub id: T,
    pub label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SourceInfo {
    pub id: SourceKind,
    pub label: &'static str,
    /// What the `url` field means for this kind, and an example of it.
    pub url_label: &'static str,
    pub url_example: &'static str,
    pub has_auth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Catalog {
    /// The stack a new project starts with.
    pub starter: Stack,
    pub backends: Vec<BackendInfo>,
    pub uis: Vec<Choice<Ui>>,
    pub targets: Vec<Choice<Target>>,
    pub sources: Vec<SourceInfo>,
    pub auths: Vec<Choice<AuthKind>>,
}


fn backend_info(backend: Backend) -> BackendInfo {
    let (label, uis) = match backend {
        Backend::Rust => ("Rust", vec![Ui::Web]),
        Backend::Go => ("Go", vec![Ui::Web]),
        Backend::Csharp => ("C#", vec![Ui::Web, Ui::Avalonia]),
        Backend::C => ("C", vec![Ui::Web]),
        Backend::Cpp => ("C++", vec![Ui::Web]),
    };
    BackendInfo { id: backend, label, uis }
}

fn ui_label(ui: Ui) -> &'static str {
    match ui {
        Ui::Web => "Web renderer",
        Ui::Avalonia => "Avalonia",
    }
}

/// The backend a native toolkit belongs to, for messages.
fn ui_backend_label(ui: Ui) -> &'static str {
    match ui {
        Ui::Web => "any",
        Ui::Avalonia => "C#",
    }
}

pub fn catalog() -> Catalog {
    let source = |id, label, url_label, url_example, has_auth| SourceInfo {
        id,
        label,
        url_label,
        url_example,
        has_auth,
    };
    Catalog {
        starter: starter("", "").stack,
        backends: BACKENDS.into_iter().map(backend_info).collect(),
        uis: [Ui::Web, Ui::Avalonia].into_iter().map(|id| Choice { id, label: ui_label(id) }).collect(),
        targets: vec![
            Choice { id: Target::Desktop, label: "Desktop" },
            Choice { id: Target::Web, label: "Web" },
        ],
        sources: vec![
            source(SourceKind::Sqlite, "SQLite", "File", "data/app.db", false),
            source(SourceKind::Postgres, "PostgreSQL", "Connection", "env:DATABASE_URL", false),
            source(SourceKind::Mysql, "MySQL", "Connection", "env:DATABASE_URL", false),
            source(SourceKind::Mssql, "SQL Server", "Connection", "env:DATABASE_URL", false),
            source(SourceKind::Rest, "Web API (REST)", "Base URL", "https://api.example.com/v1", true),
        ],
        auths: vec![
            Choice { id: AuthKind::Bearer, label: "Bearer token" },
            Choice { id: AuthKind::ApiKey, label: "API key" },
            Choice { id: AuthKind::Basic, label: "User and password" },
        ],
    }
}


/// The data sources of `settings`, as the checker needs them.
pub fn source_refs(settings: &Settings) -> Vec<clarity_spec::SourceRef> {
    settings
        .datasources
        .iter()
        .map(|s| match s.kind {
            SourceKind::Rest => clarity_spec::SourceRef::web(&s.name),
            _ => clarity_spec::SourceRef {
                external: s.external,
                readonly: s.readonly,
                ..clarity_spec::SourceRef::sql(&s.name)
            },
        })
        .collect()
}

/// The data sources of the project whose modules are in `dir`, read from
/// the project file next to it; none when it cannot be read.
pub fn sources_of(dir: &Path) -> Vec<clarity_spec::SourceRef> {
    let file = dir.with_extension(clarity_spec::SPEC_EXTENSION);
    load(&file).settings.as_ref().map(source_refs).unwrap_or_default()
}

/// Loads the modules in `dir` with their migration steps, checked against
/// the project's data sources and the tables of its external databases.
pub fn load_modules(dir: &Path) -> std::io::Result<clarity_spec::Project> {
    let mut project = clarity_spec::load_dir_in(dir, &sources_of(dir))?;
    check_external(dir, &mut project);
    hand_over(dir, &mut project);
    Ok(project)
}

/// Like `load_modules`, with `text` in place of the file at `path`.
pub fn load_modules_with(dir: &Path, path: &Path, text: &str) -> std::io::Result<clarity_spec::Project> {
    let mut project = clarity_spec::load_dir_with_in(dir, Some((path, text)), &sources_of(dir))?;
    check_external(dir, &mut project);
    hand_over(dir, &mut project);
    Ok(project)
}

/// The file of an SQLite data source: a relative location is in the
/// project's folder `dir`, like the output folder.
pub fn sqlite_path(dir: &Path, source: &DataSource) -> PathBuf {
    let path = Path::new(&source.url);
    let joined = if path.is_absolute() { path.to_path_buf() } else { dir.join(path) };
    std::path::absolute(&joined).unwrap_or(joined)
}

/// Hands the project file's language, currency and theme to the project,
/// for showing values and for its look; a part's `style` is checked
/// against that theme.
fn hand_over(dir: &Path, project: &mut clarity_spec::Project) {
    let file = dir.with_extension(clarity_spec::SPEC_EXTENSION);
    let settings = load(&file).settings;
    let themes = settings.as_ref().map(|s| s.themes.clone()).unwrap_or_default();
    if let Some(settings) = &settings {
        clarity_spec::attach_stores(project, settings.filestores.iter().map(|s| s.name.clone()).collect());
    }
    if let Some(settings) = settings {
        project.locale = clarity_spec::model::Locale {
            language: settings.language,
            currency: settings.currency,
        };
    }
    clarity_spec::themes::use_themes(project, &themes.iter().map(String::as_str).collect::<Vec<_>>());
}

/// Compares the entities of each external SQLite source with its tables.
/// A database that cannot be read is a warning: its tables are not checked.
fn check_external(dir: &Path, project: &mut clarity_spec::Project) {
    let file = dir.with_extension(clarity_spec::SPEC_EXTENSION);
    let Some(settings) = load(&file).settings else { return };
    for source in settings.datasources.iter().filter(|s| s.external && s.kind == SourceKind::Sqlite) {
        if project.entities_in(&source.name).next().is_none() {
            continue;
        }
        match clarity_db::read_sqlite(&sqlite_path(dir, source)) {
            Ok(schema) => {
                let found = clarity_db::drift(project, &source.name, &schema);
                project.diagnostics.extend(found);
            }
            Err(reason) => project.diagnostics.push(
                clarity_spec::Diagnostic::warning(
                    format!("data source {}: {reason}; its tables were not checked", source.name),
                    clarity_spec::Span::default(),
                )
                .in_file(&file.display().to_string()),
            ),
        }
    }
}

/// The settings a generated application is built with. A relative SQLite
/// location or file store is in the project's folder `dir`, for Clarity and
/// for the application alike, so it becomes the full path from there: the
/// same place whether a database is owned or external, and whichever folder
/// the application runs in.
pub fn build_settings(dir: &Path, settings: &Settings) -> Settings {
    let mut built = settings.clone();
    for source in built.datasources.iter_mut().filter(|s| s.kind == SourceKind::Sqlite) {
        source.url = sqlite_path(dir, source).display().to_string();
    }
    for store in built.filestores.iter_mut().filter(|s| !s.location.starts_with("env:")) {
        let path = Path::new(&store.location);
        let joined = if path.is_absolute() { path.to_path_buf() } else { dir.join(path) };
        store.location = std::path::absolute(&joined).unwrap_or(joined).display().to_string();
    }
    built
}

/// A table of an external database, for the Import list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SourceTable {
    pub name: String,
    pub columns: usize,
    /// Why the table cannot be imported, if it cannot.
    #[serde(rename = "notImportable")]
    #[ts(rename = "notImportable")]
    pub not_importable: Option<String>,
}

/// An import that was written: the module file and the tables left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDone {
    pub path: PathBuf,
    pub text: String,
    pub skipped: Vec<(String, String)>,
}

/// The live schema of external data source `source` of the project whose
/// modules are in `dir`.
fn external_schema(dir: &Path, source: &str) -> Result<clarity_db::LiveSchema, String> {
    let file = dir.with_extension(clarity_spec::SPEC_EXTENSION);
    let settings = load(&file).settings.ok_or_else(|| format!("{} cannot be read", file.display()))?;
    let found = settings
        .datasources
        .iter()
        .find(|s| s.name == source)
        .ok_or_else(|| format!("the project has no data source {source}"))?;
    if !found.external {
        return Err(format!("data source {source} is not external; Clarity makes its tables from the dictionary"));
    }
    if found.kind != SourceKind::Sqlite {
        return Err(format!("data source {source} is {}; importing is for SQLite only for now", source_word(found.kind)));
    }
    clarity_db::read_sqlite(&sqlite_path(dir, found))
}

/// The tables of external data source `source`, and which can be imported.
pub fn source_tables(dir: &Path, source: &str) -> Result<Vec<SourceTable>, String> {
    let schema = external_schema(dir, source)?;
    Ok(schema
        .tables
        .iter()
        .map(|t| SourceTable {
            name: t.name.clone(),
            columns: t.columns.len(),
            not_importable: clarity_db::not_importable(t),
        })
        .collect())
}

/// Writes `tables` of external data source `source` as module `module`,
/// in a new file next to the other modules.
pub fn import_tables(dir: &Path, source: &str, module: &str, tables: &[String]) -> Result<ImportDone, String> {
    if !is_identifier(module) {
        return Err(format!("'{module}' is not a valid module name"));
    }
    let path = dir.join(format!("{module}.{}", clarity_spec::SPEC_EXTENSION));
    if path.exists() {
        return Err(format!("there is already a module file {}; choose another module name", path.display()));
    }
    let schema = external_schema(dir, source)?;
    let project = load_modules(dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    if project.files.iter().any(|f| f.module.as_ref().is_some_and(|m| m.name == module)) {
        return Err(format!("there is already a module {module}; choose another name"));
    }
    let taken: Vec<String> = project.entities().map(|e| e.name.name.clone()).collect();
    let imported = clarity_db::import(&schema, tables, module, source, &taken);
    std::fs::write(&path, &imported.text).map_err(|err| format!("cannot write {}: {err}", path.display()))?;
    Ok(ImportDone {
        path,
        text: imported.text,
        skipped: imported.skipped,
    })
}

/// What taking ownership wrote: the baseline step, and the project file
/// before and after, so it can be undone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owned {
    pub step: PathBuf,
    pub step_text: String,
    pub project_file: PathBuf,
    pub before: String,
    pub after: String,
}

/// Makes external data source `source` one Clarity owns: its entities as
/// they are become a baseline step, and the source loses `external`. From
/// then on the dictionary drives its tables through ordinary steps.
pub fn take_ownership(dir: &Path, source: &str) -> Result<Owned, String> {
    external_schema(dir, source)?;
    let project = load_modules(dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    if let Some(problem) = project.diagnostics.iter().find(|d| d.severity == clarity_spec::Severity::Error) {
        return Err(format!("fix the problems first: {}", problem.message));
    }
    let ops = clarity_spec::migrate::pending(&project, source, &[]);
    if ops.is_empty() {
        return Err(format!("data source {source} has no entities to take over; import its tables first"));
    }
    let file = dir.with_extension(clarity_spec::SPEC_EXTENSION);
    let before = std::fs::read_to_string(&file).map_err(|err| format!("cannot read {}: {err}", file.display()))?;
    let mut settings = parse(&before)?;
    for s in settings.datasources.iter_mut().filter(|s| s.name == source) {
        s.external = false;
    }
    let after = write(Some(&before), &settings)?;
    let step_text = clarity_spec::migrate::step_text_baseline("Taken over", &ops);
    let step = clarity_spec::write_step_text(dir, source, &step_text).map_err(|err| format!("cannot write the step: {err}"))?;
    std::fs::write(&file, &after).map_err(|err| format!("cannot write {}: {err}", file.display()))?;
    Ok(Owned {
        step,
        step_text,
        project_file: file,
        before,
        after,
    })
}
