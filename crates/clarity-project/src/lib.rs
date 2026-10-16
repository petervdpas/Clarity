//! The project file, `<name>.clarity`: what the project is transpiled into
//! (backend, UI, targets), where its data lives (data sources) and where it
//! is built. The modules sit in the folder of the same name next to it. See
//! docs/DESIGN.md sections 1.2 and 1.3.

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
        }],
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

