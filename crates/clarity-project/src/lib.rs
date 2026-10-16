//! The project file, `clarity.toml`: what the project is transpiled into
//! (backend, UI, targets) and where its data lives (data sources). See
//! docs/DESIGN.md sections 1.2 and 1.3.
//!
//! Reading is strict: unknown keys and values are errors, so a typo never
//! silently picks a default. Writing changes only the values that differ,
//! so comments and layout written by hand stay where they are.

use std::path::Path;

use clarity_spec::widgets::is_identifier;
use serde::{Deserialize, Serialize};
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value, value};
use ts_rs::TS;

pub const PROJECT_FILE: &str = "clarity.toml";

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
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct Stack {
    pub backend: Backend,
    pub ui: Ui,
    pub targets: Vec<Target>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct Auth {
    pub kind: AuthKind,
    /// Where the secret is kept: `env:NAME` or `keychain:NAME`.
    pub secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
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
    pub name: String,
    pub stack: Stack,
    pub datasources: Vec<DataSource>,
}

/// The file's layout, which differs from `Settings` in where the name sits.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileForm {
    project: ProjectSection,
    stack: Stack,
    #[serde(default)]
    datasource: Vec<DataSource>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectSection {
    name: String,
}

pub fn parse(text: &str) -> Result<Settings, String> {
    let form: FileForm = toml_edit::de::from_str(text).map_err(|err| format!("{PROJECT_FILE}: {}", err.message()))?;
    Ok(Settings {
        name: form.project.name,
        stack: form.stack,
        datasources: form.datasource,
    })
}

/// The settings of a new project: Rust with the web renderer on the
/// desktop, and one SQLite database.
pub fn starter(name: &str, module: &str) -> Settings {
    Settings {
        name: name.to_string(),
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
    }
}

/// Everything wrong with the settings, in reading order. Empty when they
/// can be saved.
pub fn validate(settings: &Settings) -> Vec<String> {
    let mut problems = Vec::new();
    if settings.name.trim().is_empty() {
        problems.push("the project needs a name".to_string());
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
            problems.push(format!("the target {} is listed twice", target_id(*target)));
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

/// The project file text for `settings`. With `existing`, only the values
/// that changed are rewritten and everything else is kept as it was.
pub fn write(existing: Option<&str>, settings: &Settings) -> Result<String, String> {
    let mut doc: DocumentMut = match existing {
        Some(text) => text.parse().map_err(|err| format!("{PROJECT_FILE}: {err}"))?,
        None => DocumentMut::new(),
    };
    let project = table(doc.as_table_mut(), "project")?;
    set(project, "name", settings.name.as_str().into());

    let stack = table(doc.as_table_mut(), "stack")?;
    set(stack, "backend", backend_id(settings.stack.backend).into());
    set(stack, "ui", ui_id(settings.stack.ui).into());
    set(stack, "targets", Value::Array(settings.stack.targets.iter().map(|t| target_id(*t)).collect::<Array>()));

    // Data sources are rebuilt in their new order; a source that keeps its
    // name keeps its table, with the comments in it.
    let old: Vec<Table> = match doc.get("datasource") {
        Some(Item::ArrayOfTables(sources)) => sources.iter().cloned().collect(),
        Some(_) => return Err(format!("{PROJECT_FILE}: datasource must be a list of [[datasource]] tables")),
        None => Vec::new(),
    };
    let mut sources = ArrayOfTables::new();
    for source in &settings.datasources {
        let mut t = old
            .iter()
            .find(|t| t.get("name").and_then(Item::as_str) == Some(source.name.as_str()))
            .cloned()
            .unwrap_or_default();
        // Written after the other tables, in this order.
        t.set_position(None);
        set(&mut t, "name", source.name.as_str().into());
        set(&mut t, "kind", source_id(source.kind).into());
        set(&mut t, "url", source.url.as_str().into());
        match &source.auth {
            Some(auth) => {
                let mut inline = InlineTable::new();
                inline.insert("kind", auth_id(auth.kind).into());
                inline.insert("secret", auth.secret.as_str().into());
                set(&mut t, "auth", Value::InlineTable(inline));
            }
            None => {
                t.remove("auth");
            }
        }
        sources.push(t);
    }
    if sources.is_empty() {
        doc.remove("datasource");
    } else {
        doc.insert("datasource", Item::ArrayOfTables(sources));
    }
    Ok(doc.to_string())
}

/// The table under `key`, created when it is missing.
fn table<'d>(root: &'d mut Table, key: &str) -> Result<&'d mut Table, String> {
    if !root.contains_key(key) {
        root.insert(key, Item::Table(Table::new()));
    }
    root.get_mut(key)
        .and_then(Item::as_table_mut)
        .ok_or_else(|| format!("{PROJECT_FILE}: [{key}] must be a table"))
}

/// Sets `key` unless it already holds an equal value, keeping its comments.
fn set(table: &mut Table, key: &str, new: Value) {
    let same = table
        .get(key)
        .and_then(Item::as_value)
        .is_some_and(|old| same_value(old, &new));
    if !same {
        table.insert(key, value(new));
    }
}

fn same_value(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(a), Value::String(b)) => a.value() == b.value(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| same_value(a, b))
        }
        (Value::InlineTable(a), Value::InlineTable(b)) => {
            a.len() == b.len()
                && a.iter().all(|(k, v)| b.get(k).is_some_and(|w| same_value(v, w)))
        }
        _ => false,
    }
}

/// What the project folder's `clarity.toml` holds, for the Project tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ProjectFile {
    /// None when the file is missing or cannot be read.
    pub settings: Option<Settings>,
    /// The version of the file text, to refuse saving over a newer file.
    pub version: String,
    pub problems: Vec<String>,
    pub catalog: Catalog,
}

pub fn load(dir: &Path) -> ProjectFile {
    let file = |settings, version, problems| ProjectFile {
        settings,
        version,
        problems,
        catalog: catalog(),
    };
    let text = match std::fs::read_to_string(dir.join(PROJECT_FILE)) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return file(
                None,
                String::new(),
                vec![format!("the project has no {PROJECT_FILE}; create it on the Project tab")],
            );
        }
        Err(err) => return file(None, String::new(), vec![format!("cannot read {PROJECT_FILE}: {err}")]),
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

const BACKENDS: [Backend; 5] = [Backend::Rust, Backend::Go, Backend::Csharp, Backend::C, Backend::Cpp];

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

// The words used in the file, matching the serde names above.

fn backend_id(backend: Backend) -> &'static str {
    match backend {
        Backend::Rust => "rust",
        Backend::Go => "go",
        Backend::Csharp => "csharp",
        Backend::C => "c",
        Backend::Cpp => "cpp",
    }
}

fn ui_id(ui: Ui) -> &'static str {
    match ui {
        Ui::Web => "web",
        Ui::Avalonia => "avalonia",
    }
}

fn target_id(target: Target) -> &'static str {
    match target {
        Target::Desktop => "desktop",
        Target::Web => "web",
    }
}

fn source_id(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Sqlite => "sqlite",
        SourceKind::Postgres => "postgres",
        SourceKind::Mysql => "mysql",
        SourceKind::Mssql => "mssql",
        SourceKind::Rest => "rest",
    }
}

fn auth_id(kind: AuthKind) -> &'static str {
    match kind {
        AuthKind::Bearer => "bearer",
        AuthKind::ApiKey => "api-key",
        AuthKind::Basic => "basic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The word serde writes for a unit variant.
    fn word<T: Serialize>(v: T) -> String {
        #[derive(Serialize)]
        struct Wrap<T> {
            v: T,
        }
        let text = toml_edit::ser::to_string(&Wrap { v }).unwrap();
        text.trim().trim_start_matches("v = ").trim_matches('"').to_string()
    }

    /// The words written by hand above must match the ones serde reads.
    #[test]
    fn file_words_match_serde() {
        for b in BACKENDS {
            assert_eq!(word(b), backend_id(b));
        }
        for u in [Ui::Web, Ui::Avalonia] {
            assert_eq!(word(u), ui_id(u));
        }
        for t in [Target::Desktop, Target::Web] {
            assert_eq!(word(t), target_id(t));
        }
        for k in [SourceKind::Sqlite, SourceKind::Postgres, SourceKind::Mysql, SourceKind::Mssql, SourceKind::Rest] {
            assert_eq!(word(k), source_id(k));
        }
        for a in [AuthKind::Bearer, AuthKind::ApiKey, AuthKind::Basic] {
            assert_eq!(word(a), auth_id(a));
        }
    }
}
