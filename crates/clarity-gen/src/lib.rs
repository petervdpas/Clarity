//! Transpiles a checked Clarity project into a real application for the
//! stack its project file names (docs/DESIGN.md section 1.2). Generation is
//! deterministic: the same project always gives the same files, byte for
//! byte, so `out/` can be compared, cached and regenerated at any time.

pub mod agent_tools;
pub mod tools;
pub mod clarity_ui;
mod c;
pub mod calls;
mod consts;
pub mod libraries;
pub mod parts;
mod part_code;
mod commands;
mod csharp;
mod ddl;
pub(crate) mod go;
mod cpp;
mod shape;
mod signin;
pub use signin::carries_auth;
use clarity_spec::code::print;
mod query;
mod readme;
mod rust;
mod schema;
pub mod web;
pub mod web_windows;
pub mod windows_resources;
pub mod web_browser;
pub mod preview;

pub use csharp::runtime_version as csharp_runtime_version;
pub use go::runtime_version as go_runtime_version;
pub use rust::runtime_version as rust_runtime_version;
pub use rust::local_patch as rust_local_patch;
pub use rust::migration_sql;
pub use web::{APP_CLASS, base_css, theme_css};

use clarity_project::{Backend, Settings};
use clarity_spec::Project;

use ddl::Sql;

/// The databases of an application and the ones Clarity owns.
type Databases = (Vec<(String, Sql, String)>, Vec<(String, Sql)>);

/// The database of every data source that holds entities, with its kind and
/// where it is, and the ones Clarity owns (and so migrates); external ones
/// are opened as they are. `backend` names the backend in messages.
fn databases(project: &Project, settings: &Settings) -> Result<Databases, Vec<String>> {
    // A project with no entities stores nothing, so it needs no data source.
    if settings.datasources.is_empty() && project.entities().next().is_some() {
        return Err(vec!["the project needs a data source to store its records".to_string()]);
    }
    let mut databases: Vec<(String, Sql, String)> = Vec::new();
    for source in &settings.datasources {
        if project.entities_in(&source.name).next().is_none() {
            continue;
        }
        // A web API keeps its own records (DESIGN 5.23): its entities are not stored.
        // A file source is opened from its file, not connected to (`file_sources`).
        if source.kind == clarity_project::SourceKind::File {
            continue;
        }
        let Some(sql) = Sql::of(source.kind) else { continue };
        if source.url.starts_with("keychain:") {
            return Err(vec![format!("data source '{}': {}", source.name, clarity_project::keychain_problem(&source.url))]);
        }
        databases.push((source.name.clone(), sql, source.url.clone()));
    }
    let owned = databases
        .iter()
        .filter(|(name, _, _)| settings.datasources.iter().any(|s| s.name == *name && !s.external))
        .map(|(name, sql, _)| (name.clone(), *sql))
        .collect();
    Ok((databases, owned))
}

/// A data source that is a file in a format (docs/FORMATS-PLAN.md), as
/// every runtime opens it: read into one in-memory SQLite table of its
/// entity, written back after each change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileSource {
    pub name: String,
    pub format: String,
    pub path: String,
    /// The format's options as a JSON object, or empty.
    pub options: String,
    pub table: String,
    pub key: String,
    /// The columns the entity maps with their kinds, as JSON.
    pub fields: String,
    pub readonly: bool,
}

/// The project's data sources that are files, each with the one entity it holds.
pub(crate) fn file_sources(project: &Project, settings: &clarity_project::Settings) -> Result<Vec<FileSource>, Vec<String>> {
    let mut out = Vec::new();
    for source in &settings.datasources {
        let Some(file) = &source.file else { continue };
        // `using OrderFeed`: the mapping says the format, the entity and the options (docs/FORMATS-PLAN.md, step 4).
        let mapping = match &file.mapping {
            Some(name) => match project.mapping(name) {
                Some(mapping) => Some(mapping),
                None => return Err(vec![format!("data source '{}' is laid over by mapping {name}, which no module declares", source.name)]),
            },
            None => None,
        };
        let format = mapping.map_or(file.format.as_str(), |m| m.format.name.as_str());
        let entities: Vec<&clarity_spec::model::Entity> = project.entities_in(&source.name).filter(|e| mapping.is_none_or(|m| m.entity.name == e.name.name)).collect();
        let entity = match entities.as_slice() {
            [] => continue,
            [one] => *one,
            _ => return Err(vec![format!("data source '{}' is one {format} file, one table: it holds the records of one entity, not {}", source.name, entities.len())]),
        };
        if clarity_formats::format(format).is_none() {
            return Err(vec![format!("data source '{}': there is no format '{format}'", source.name)]);
        }
        let inside = mapping.map(|m| m.fields.as_slice()).unwrap_or_default();
        let fields = file_fields(project, entity, inside);
        out.push(FileSource {
            name: source.name.clone(),
            format: format.to_string(),
            path: source.url.clone(),
            options: mapping.map_or_else(|| clarity_project::file_options_json(file), |m| shape::mapping_options(&by_columns(project, m))),
            table: entity.table_name(),
            key: entity.key_column().to_string(),
            fields: fields.to_string(),
            readonly: source.readonly,
        });
    }
    Ok(out)
}

/// The columns `entity` keeps in a file source with their kinds, as
/// Clarity.Formats takes them: each stored field by its column, and each
/// field of records inside (`lines of Line` in the mapping) as its records
/// kind, with the table and key the runtime keeps those records in.
fn file_fields(project: &Project, entity: &clarity_spec::model::Entity, inside: &[clarity_spec::code::MappedField]) -> serde_json::Value {
    let kind = shape::write_kind(project, &clarity_spec::code::check::TypeInfo::Record(entity.name.name.clone())).unwrap_or_default();
    let mut fields: Vec<serde_json::Value> = serde_json::from_str::<serde_json::Value>(&kind)
        .ok()
        .and_then(|k| k.get("fields").and_then(|f| f.as_array()).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter(|f| !matches!(f.get("kind").and_then(|k| k.as_str()), Some("other" | "record" | "records")))
        .map(|mut f| {
            let column = f.get("name").and_then(|n| n.as_str()).map(|n| entity.column_of(n)).unwrap_or_default();
            f["name"] = serde_json::Value::String(column);
            f
        })
        .collect();
    for mapped in inside {
        let Some(nested) = &mapped.inside else { continue };
        let Some(inner) = project.entity(&nested.entity.name) else { continue };
        let many = !matches!(entity.field(&mapped.field.name).map(|f| &f.ty), Some(clarity_spec::model::FieldType::Record { .. }));
        fields.push(serde_json::json!({
            "name": mapped.field.name,
            "kind": if many { "records" } else { "record" },
            "entity": inner.name.name,
            "fields": file_fields(project, inner, &nested.fields),
            "base": {},
            "table": inner.table_name(),
            "key": inner.key_column(),
        }));
    }
    serde_json::Value::Array(fields)
}

/// A mapping as the columns it lays a file over: each field's line and
/// `order is ".."` by the field's column, as the runtime's table names it;
/// a field of records inside keeps its name, which the records field has.
fn by_columns(project: &Project, mapping: &clarity_spec::code::MappingDecl) -> clarity_spec::code::MappingDecl {
    fn rename(project: &Project, entity: Option<&clarity_spec::model::Entity>, fields: &mut [clarity_spec::code::MappedField]) {
        for mapped in fields {
            match &mut mapped.inside {
                Some(nested) => {
                    let inner = project.entity(&nested.entity.name);
                    if let (Some(parent), Some(inner)) = (&mut nested.parent, inner) {
                        parent.field.name = inner.column_of(&parent.field.name);
                    }
                    rename(project, inner, &mut nested.fields);
                }
                None => {
                    if let Some(entity) = entity {
                        mapped.field.name = entity.column_of(&mapped.field.name);
                    }
                }
            }
        }
    }
    let mut out = mapping.clone();
    rename(project, project.entity(&mapping.entity.name), &mut out.fields);
    out
}

/// Whether the program carries Clarity.Formats: its code reads or writes a
/// format, or a data source is a file in one.
pub fn carries_formats(project: &Project, settings: &clarity_project::Settings) -> bool {
    project.uses_formats() || settings.datasources.iter().any(|s| s.file.is_some() && project.entities_in(&s.name).next().is_some())
}

/// Every file store an image field keeps its pictures in must be one of the
/// project settings, which say where it is.
fn check_stores(project: &Project, settings: &Settings) -> Result<(), Vec<String>> {
    let missing: Vec<String> = project
        .entities()
        .flat_map(|e| e.fields.iter().map(move |f| (e, f)))
        .filter_map(|(e, f)| {
            let store = f.modifiers.store.as_ref()?;
            (!settings.filestores.iter().any(|s| &s.name == store)).then(|| {
                format!("{}.{} is kept in file store '{store}', which the project settings do not have", e.name.name, f.name.name)
            })
        })
        .collect();
    if missing.is_empty() { Ok(()) } else { Err(missing) }
}

/// The window the application opens with: the app's main window, or the
/// first window of the project.
pub(crate) fn main_window(project: &Project) -> Option<String> {
    project
        .apps()
        .find_map(|a| a.main.as_ref().map(|m| m.name.clone()))
        .or_else(|| project.pages().next().map(|p| p.name.name.clone()))
}

/// The size the window `name` opens at, in pixels: its own (`size 320 x
/// 460`), or the standard size every stack shares.
pub(crate) fn window_size(project: &Project, name: &str) -> (u32, u32) {
    let page = project.pages().find(|p| p.name.name == name);
    if let Some(size) = page.and_then(|p| p.size) {
        return (size.width, size.height);
    }
    // A toolbox is drawn as large as its tools, not as a window.
    if let Some(page) = page.filter(|p| p.kind == clarity_spec::model::PageKind::Toolbox) {
        let rows = page.widgets.iter().map(|w| w.props.row + w.props.height.max(1) - 1).max().unwrap_or(1);
        return clarity_spec::layout::toolbox_size(page.columns, rows);
    }
    (clarity_spec::layout::WINDOW_WIDTH, clarity_spec::layout::WINDOW_HEIGHT)
}

/// Every engine the project declares (DESIGN 5.11).
/// What a `TypeRef` names, as a checker type.
/// What cascades from `entity` (`cascade`): each child entity, the module
/// file it is in, and the column of its field that refers to `entity`.
pub(crate) fn cascades<'p>(project: &'p Project, entity: &str) -> Vec<(&'p clarity_spec::model::Entity, &'p clarity_spec::model::SpecFile, String)> {
    project
        .cascades(entity)
        .into_iter()
        .filter_map(|(child, field)| {
            let file = project.files.iter().find(|f| f.entities.iter().any(|e| e.name.name == child.name.name))?;
            Some((child, file, child.column_of(&field.name.name).to_string()))
        })
        .collect()
}

/// The required fields a save of `entity` checks (conformance/required.json):
/// each stored field with `required`, as its column (a file's content
/// column) and its label as written (`@key` or the text). None for an
/// entity no database keeps (settings, a web API's records).
pub(crate) fn required_fields(project: &Project, entity: &clarity_spec::model::Entity) -> Vec<(String, String)> {
    if entity.settings || project.in_web_api(&entity.name.name) {
        return Vec::new();
    }
    entity
        .fields
        .iter()
        .filter(|f| f.modifiers.required && !f.modifiers.computed && !f.ty.is_held())
        .map(|f| (entity.column_of(&f.name.name).to_string(), f.label.clone()))
        .collect()
}

/// The file stores whose folder is a folder setting (`filestore photos
/// "setting:photosFolder"`): the store, the setting and the setting's
/// default ("" without one). Each runtime reads the setting at every use.
pub(crate) fn folder_stores(project: &Project, settings: &clarity_project::Settings) -> Vec<(String, String, String)> {
    let default_of = |setting: &str| {
        project
            .entities()
            .filter(|e| e.settings)
            .flat_map(|e| &e.fields)
            .find(|f| f.name.name == setting)
            .and_then(|f| match &f.modifiers.default {
                Some(clarity_spec::model::Literal::Str(text)) => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default()
    };
    settings
        .filestores
        .iter()
        .filter_map(|s| s.location.strip_prefix("setting:").map(|setting| (s.name.clone(), setting.to_string(), default_of(setting))))
        .collect()
}

/// The file stores at a folder or `env:NAME`, found once when the program starts.
pub(crate) fn fixed_stores(settings: &clarity_project::Settings) -> Vec<&clarity_project::FileStore> {
    settings.filestores.iter().filter(|s| !s.location.starts_with("setting:")).collect()
}

/// Another record's picture or file copied in code (`album.cover =
/// other.cover`): the record it comes from, which field, and where both
/// fields are kept. Every stack's runtime copies it alike: a picture in hand
/// as it is, a stored one read first, a file of a store linked only in the
/// same store.
pub(crate) struct Copied<'e> {
    /// The record the value is a field of.
    pub object: &'e clarity_spec::code::Expr,
    pub entity: String,
    pub field: String,
    pub column: String,
    /// The value's file store, none for the database.
    pub from: Option<String>,
    /// The target field's file store.
    pub to: Option<String>,
    /// A `file` field's file, rather than a picture.
    pub file: bool,
}

/// A record's picture given to a picture function (`resize(book.cover, ...)`):
/// read first, as a copy into the database is, so the function has its bytes.
pub(crate) fn read_picture<'e>(
    project: &Project,
    type_of: impl Fn(&clarity_spec::code::Expr) -> clarity_spec::code::check::TypeInfo,
    value: &'e clarity_spec::code::Expr,
) -> Option<Copied<'e>> {
    use clarity_spec::code::ExprKind;
    use clarity_spec::code::check::TypeInfo;
    if type_of(value) != TypeInfo::Image {
        return None;
    }
    let ExprKind::Member { object, member } = &value.kind else { return None };
    let TypeInfo::Record(entity) = type_of(object) else { return None };
    let field = member.name.clone();
    let declared = project.entity(&entity);
    let from = declared.and_then(|e| e.field(&field)).and_then(|f| f.modifiers.store.clone());
    let column = declared.map_or_else(|| field.clone(), |e| e.column_of(&field).to_string());
    Some(Copied { object: object.as_ref(), entity, field, column, from, to: None, file: false })
}

/// Whether the project's code calls a picture function, so the program carries Clarity.Image.
pub(crate) fn uses_picture_functions(project: &Project) -> bool {
    project.calls_any(&|name| clarity_spec::code::check::picture_function(name).is_some() && built_in(project, name).is_some())
}

/// The folder setting an assignment sets, `settings.ledgerFolder = x`: the
/// running handler holds it (`hold_setting`), so a file store whose folder it
/// is reads it at once, before the settings are kept (conformance/settings.json, `now`).
pub(crate) fn held_setting<'p>(
    project: &'p Project,
    type_of: impl Fn(&clarity_spec::code::Expr) -> clarity_spec::code::check::TypeInfo,
    target: &clarity_spec::code::Expr,
) -> Option<&'p str> {
    let clarity_spec::code::ExprKind::Member { object, member } = &target.kind else { return None };
    let clarity_spec::code::check::TypeInfo::Record(entity) = type_of(object) else { return None };
    let entity = project.entity(&entity).filter(|e| e.settings)?;
    entity.field(&member.name).filter(|f| f.ty == clarity_spec::model::FieldType::Folder).map(|f| f.name.name.as_str())
}

/// Whether the running handler holds the settings: a file store reads a
/// folder setting, so the settings the call began with come first.
pub(crate) fn holds_settings(project: &Project) -> bool {
    !project.folder_settings().is_empty()
}

pub(crate) fn copied<'e>(
    project: &Project,
    type_of: impl Fn(&clarity_spec::code::Expr) -> clarity_spec::code::check::TypeInfo,
    target: &clarity_spec::code::Expr,
    value: &'e clarity_spec::code::Expr,
) -> Option<Copied<'e>> {
    use clarity_spec::code::ExprKind;
    use clarity_spec::code::check::TypeInfo;
    let kind = type_of(value);
    if !matches!(kind, TypeInfo::Image | TypeInfo::RecordFile) {
        return None;
    }
    let field_of = |expr: &'e clarity_spec::code::Expr| -> Option<(&'e clarity_spec::code::Expr, String, String)> {
        let ExprKind::Member { object, member } = &expr.kind else { return None };
        let TypeInfo::Record(owner) = type_of(object) else { return None };
        Some((object.as_ref(), owner, member.name.clone()))
    };
    let store = |owner: &str, field: &str| project.entity(owner).and_then(|e| e.field(field)).and_then(|f| f.modifiers.store.clone());
    let (object, entity, field) = field_of(value)?;
    let ExprKind::Member { object: target_object, member: target_field } = &target.kind else { return None };
    let to = match type_of(target_object) {
        TypeInfo::Record(owner) => store(&owner, &target_field.name),
        _ => None,
    };
    let column = project.entity(&entity).map_or_else(|| field.clone(), |e| e.column_of(&field).to_string());
    Some(Copied { object, from: store(&entity, &field), to, column, entity, field, file: kind == TypeInfo::RecordFile })
}

pub(crate) fn type_ref_info(project: &Project, ty: &clarity_spec::code::TypeRef) -> clarity_spec::code::check::TypeInfo {
    use clarity_spec::code::TypeRefKind;
    use clarity_spec::code::check::{TypeInfo, ValueItem};
    match &ty.kind {
        TypeRefKind::Text => TypeInfo::Text,
        TypeRefKind::Integer => TypeInfo::Integer,
        TypeRefKind::Duration => TypeInfo::Duration,
        TypeRefKind::Decimal => TypeInfo::Decimal,
        TypeRefKind::Money => TypeInfo::Money,
        TypeRefKind::Bool => TypeInfo::Bool,
        TypeRefKind::Date => TypeInfo::Date,
        TypeRefKind::DateTime => TypeInfo::DateTime,
        TypeRefKind::File => TypeInfo::File,
        TypeRefKind::Bytes => TypeInfo::Bytes,
        TypeRefKind::List { item } if item == "file" => TypeInfo::Files,
        TypeRefKind::List { item } if ValueItem::of_word(item).is_some() => TypeInfo::Values(ValueItem::of_word(item).expect("a value item")),
        TypeRefKind::Named { name } if project.entity(name).is_some() => TypeInfo::Record(name.clone()),
        TypeRefKind::Named { name } => TypeInfo::Enum(name.clone()),
        TypeRefKind::List { item } => TypeInfo::List(item.clone()),
    }
}

/// The engines the backend loads from WebAssembly files (`engine X from
/// x.wasm`). An engine beside the screen is not one: the renderer runs it.
pub(crate) fn engines(project: &Project) -> Vec<&clarity_spec::code::EngineDecl> {
    project.files.iter().flat_map(|f| &f.engines).filter(|e| e.body.is_none() && !e.beside).collect()
}

/// The engines written in Clarity, each with the file that declares it,
/// which generation compiles to WebAssembly first (`compile_engines`).
pub(crate) fn written_engines(project: &Project) -> Vec<(&clarity_spec::model::SpecFile, &clarity_spec::code::EngineDecl, &clarity_spec::code::EngineBody)> {
    project
        .files
        .iter()
        .flat_map(|f| f.engines.iter().filter_map(move |e| e.body.as_ref().map(|b| (f, e, b))))
        .collect()
}

/// The files of the project's engines, in `folder` of the output; the
/// checker has made sure each declared one is there.
pub(crate) fn engine_files(project: &Project, folder: &str) -> Vec<OutFile> {
    let found = project.engines.as_deref().unwrap_or_default();
    engines(project)
        .into_iter()
        .filter_map(|e| found.iter().find(|f| f.name == e.file))
        .map(|f| OutFile { path: format!("{folder}/{}", f.name), text: String::new(), bytes: Some(std::borrow::Cow::Owned(f.bytes.clone())) })
        .collect()
}

/// The engines beside the screen (DESIGN 5.12), which the renderer runs:
/// the web stacks carry them in the model, C# in the application.
pub(crate) fn screen_engines(project: &Project) -> Vec<&clarity_spec::code::EngineDecl> {
    project.files.iter().flat_map(|f| &f.engines).filter(|e| e.body.is_none() && e.beside).collect()
}

/// The files of the engines beside the screen, in `folder` of the output.
pub(crate) fn screen_engine_files(project: &Project, folder: &str) -> Vec<OutFile> {
    let found = project.engines.as_deref().unwrap_or_default();
    screen_engines(project)
        .into_iter()
        .filter_map(|e| found.iter().find(|f| f.name == e.file))
        .map(|f| OutFile { path: format!("{folder}/{}", f.name), text: String::new(), bytes: Some(std::borrow::Cow::Owned(f.bytes.clone())) })
        .collect()
}

/// How the type an `ask` asks for travels to the screen (DESIGN 5.14):
/// its word (text, integer, decimal, money, date, datetime, duration, enum,
/// record) and the enum or entity it names.
pub(crate) fn ask_type(project: &Project, ty: &clarity_spec::code::TypeRef) -> (&'static str, Option<String>) {
    use clarity_spec::code::check::TypeInfo;
    match type_ref_info(project, ty) {
        TypeInfo::Integer => ("integer", None),
        TypeInfo::Decimal => ("decimal", None),
        TypeInfo::Money => ("money", None),
        TypeInfo::Date => ("date", None),
        TypeInfo::DateTime => ("datetime", None),
        TypeInfo::Duration => ("duration", None),
        TypeInfo::Enum(name) => ("enum", Some(name)),
        TypeInfo::Record(name) => ("record", Some(name)),
        _ => ("text", None),
    }
}

/// The handlers of a window the answer to one of its questions runs: the
/// handlers after `then`, each once.
/// The handlers that run with values given from outside the code: the
/// handler an answer goes to (DESIGN 5.14), and a handler wired to a custom
/// part's tell (DESIGN 5.19).
pub(crate) fn given_values_handlers<'a>(page: Option<&clarity_spec::model::Page>, handlers: &[&'a clarity_spec::code::HandlerDecl]) -> Vec<&'a clarity_spec::code::HandlerDecl> {
    let mut found = answering_handlers(handlers);
    for name in parts::told_handlers(page) {
        if let Some(handler) = handlers.iter().find(|h| h.name.name == name)
            && !found.iter().any(|f| f.name.name == name)
        {
            found.push(handler);
        }
    }
    found
}

pub(crate) fn answering_handlers<'a>(handlers: &[&'a clarity_spec::code::HandlerDecl]) -> Vec<&'a clarity_spec::code::HandlerDecl> {
    use clarity_spec::code::{Stmt, StmtKind};
    fn targets(body: &[Stmt], found: &mut Vec<String>) {
        for stmt in body {
            match &stmt.kind {
                StmtKind::Ask { then, .. } if !found.contains(&then.name) => found.push(then.name.clone()),
                StmtKind::If { branches, otherwise } => {
                    for branch in branches {
                        targets(&branch.body, found);
                    }
                    if let Some(body) = otherwise {
                        targets(body, found);
                    }
                }
                StmtKind::For { body, .. } => targets(body, found),
                _ => {}
            }
        }
    }
    let mut names = Vec::new();
    for handler in handlers {
        targets(&handler.body, &mut names);
    }
    names.iter().filter_map(|n| handlers.iter().find(|h| h.name.name == *n).copied()).collect()
}

/// The canvases of a window that a draw handler draws, by name, with the
/// handler: `("dial", "draw_dial")`. Each draw starts an empty drawing; the
/// backend runs them when the window opens and after every other handler.
pub(crate) fn canvas_draws(page: &clarity_spec::model::Page) -> Vec<(String, String)> {
    use clarity_spec::model::Widget;
    use clarity_spec::widgets::{Event, WidgetKind};
    fn walk(widgets: &[Widget], out: &mut Vec<(String, String)>) {
        for widget in widgets {
            let p = &widget.props;
            if p.kind == WidgetKind::Canvas
                && let (Some(name), Some(h)) = (&p.name, p.handlers.iter().find(|h| h.event == Event::Draw))
            {
                out.push((name.clone(), h.handler.clone()));
            }
            walk(&widget.children, out);
            for page in &widget.pages {
                walk(&page.children, out);
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

/// Each value of a shape as a draw statement gives it: its key on the
/// wire, what it is, and the argument written, or the value it has when
/// left out (conformance/drawing.json).
/// The tool a painting or vector canvas starts with (DESIGN 5.31): the
/// first of its `tools`, else the first of its mode's; empty for none.
/// A spin box's range as its line writes it, whole numbers: 0 and 2147483647 where it writes none.
pub(crate) fn spin_range(props: &clarity_spec::model::WidgetProps) -> (i64, i64) {
    let whole = |t: &Option<String>| t.as_deref().and_then(|t| t.trim().parse::<i64>().ok());
    (whole(&props.min).unwrap_or(0), whole(&props.max).unwrap_or(2_147_483_647))
}

pub(crate) fn canvas_tool(props: &clarity_spec::model::WidgetProps) -> String {
    let first = props.tools.first().map(String::as_str).or_else(|| props.mode.and_then(|m| m.tools().first().copied()));
    first.unwrap_or_default().to_string()
}

/// The type of a field a query compares: the entity's field, or its key
/// for `id`, which every record has (`where p.id != current.id`).
pub(crate) fn query_field_type(project: &clarity_spec::Project, entity: &str, field: &str) -> Option<clarity_spec::model::FieldType> {
    let entity = project.entity(entity)?;
    match entity.field(field) {
        Some(found) => Some(found.ty.clone()),
        None if field == clarity_spec::code::check::RECORD_ID => Some(clarity_spec::model::FieldType::Integer),
        None => None,
    }
}

/// The column a query compares: the field's, or the key column for `id`.
pub(crate) fn query_column(project: &clarity_spec::Project, entity: &str, field: &str) -> String {
    match project.entity(entity) {
        Some(e) if e.field(field).is_none() && field == clarity_spec::code::check::RECORD_ID => e.key_column().to_string(),
        Some(e) => e.column_of(field).to_string(),
        None => field.to_string(),
    }
}

pub(crate) fn shape_values<'a>(
    shape: &str,
    args: &'a [clarity_spec::code::Expr],
) -> Vec<(&'static str, clarity_spec::code::check::ShapeValue, Result<&'a clarity_spec::code::Expr, &'static str>)> {
    let Some(found) = clarity_spec::code::check::shape(shape) else { return Vec::new() };
    found
        .values
        .iter()
        .enumerate()
        .map(|(i, (key, kind, default))| (*key, *kind, args.get(i).ok_or(default.unwrap_or(""))))
        .collect()
}

/// The calls with values a window's parts wire, each once, in the order of
/// the design: `digit(7)`, `digit(8)`. A backend dispatches each by its
/// written form (widgets::handler_call), which the renderer sends, with
/// the values compiled in.
pub(crate) fn wired_calls(page: &clarity_spec::model::Page) -> Vec<&clarity_spec::model::Handler> {
    use clarity_spec::model::Widget;
    fn walk<'a>(widgets: &'a [Widget], out: &mut Vec<&'a clarity_spec::model::Handler>) {
        for widget in widgets {
            for h in widget.props.handlers.iter().filter(|h| !h.args.is_empty()) {
                if !out.iter().any(|o| o.handler == h.handler && o.args == h.args) {
                    out.push(h);
                }
            }
            walk(&widget.children, out);
            for page in &widget.pages {
                walk(&page.children, out);
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

/// The named tables and lists of a page that show the rows of its data, by
/// the data's name. When a handler replaces those rows (`reload`, or a new
/// list), they are other records, so these parts lose their selected row: on
/// the web stacks through the runtime's `deselect`, on C# because the
/// table is given new rows.
pub(crate) fn rows_shown(page: Option<&clarity_spec::model::Page>) -> std::collections::BTreeMap<String, Vec<String>> {
    use clarity_spec::model::Widget;
    use clarity_spec::widgets::WidgetKind;
    fn walk(widgets: &[Widget], out: &mut std::collections::BTreeMap<String, Vec<String>>) {
        for widget in widgets {
            let p = &widget.props;
            if let (WidgetKind::Table | WidgetKind::List | WidgetKind::Tree | WidgetKind::TreeTable | WidgetKind::Graph, Some(name), Some(bind)) = (p.kind, &p.name, &p.bind)
                && !bind.contains('.')
            {
                out.entry(bind.clone()).or_default().push(name.clone());
            }
            walk(&widget.children, out);
            for page in &widget.pages {
                walk(&page.children, out);
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    if let Some(page) = page {
        walk(&page.widgets, &mut out);
    }
    out
}

/// The image fields of every entity, with the column and store of each.
pub(crate) fn picture_fields(project: &Project) -> Vec<(&clarity_spec::model::Entity, &clarity_spec::model::Field)> {
    project
        .entities()
        .flat_map(|e| e.fields.iter().filter(|f| f.ty == clarity_spec::model::FieldType::Image).map(move |f| (e, f)))
        .collect()
}

/// The dropdowns with options of U1c, as the shells serve them: the text
/// fields a `typed` dropdown offers the values of, and for each entity a
/// `search` dropdown picks from, the fields its records show.
#[derive(Debug, Default)]
pub(crate) struct DropdownNeeds {
    /// `(entity, field)`, each once.
    pub typed: Vec<(String, String)>,
    /// Entity to the fields shown, each once, in the order first written.
    pub shown: std::collections::BTreeMap<String, Vec<String>>,
}

pub(crate) fn dropdown_needs(project: &Project) -> DropdownNeeds {
    use clarity_spec::model::{FieldType, Widget};
    use clarity_spec::widgets::WidgetKind;
    fn walk<'a>(widgets: &'a [Widget], found: &mut Vec<&'a Widget>) {
        for w in widgets {
            found.push(w);
            walk(&w.children, found);
            for page in &w.pages {
                walk(&page.children, found);
            }
        }
    }
    let mut needs = DropdownNeeds::default();
    for page in project.files.iter().flat_map(|f| &f.pages) {
        let mut parts = Vec::new();
        walk(&page.widgets, &mut parts);
        for p in parts.iter().map(|w| &w.props).filter(|p| p.kind == WidgetKind::Dropdown && (p.typed || p.search)) {
            let Some((data, field)) = p.bind.as_deref().and_then(|b| b.split_once('.')) else { continue };
            let Some(entity) = page.data.iter().find(|d| d.name.name == data).and_then(|d| d.entity()).and_then(|e| project.entity(&e.name)) else { continue };
            let Some(field) = entity.field(field) else { continue };
            if p.typed {
                let pair = (entity.name.name.clone(), field.name.name.clone());
                if !needs.typed.contains(&pair) {
                    needs.typed.push(pair);
                }
            }
            if let (true, FieldType::Ref { entity: target }) = (p.search, &field.ty) {
                let shown = needs.shown.entry(target.name.clone()).or_default();
                for f in &p.show {
                    if !shown.contains(f) {
                        shown.push(f.clone());
                    }
                }
            }
        }
    }
    needs
}

/// One generated file, by its path inside `out/`. Most are text; a few, like
/// icons, are bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutFile {
    pub path: String,
    pub text: String,
    /// The contents of a binary file; `text` is empty then.
    pub bytes: Option<std::borrow::Cow<'static, [u8]>>,
}

/// The files of the application, sorted by path; or why it cannot be
/// generated.
/// The project with each key that points at a resource made a text in every
/// language: the Markdown of its file, or the name of its picture. Every
/// stack shows a key's value through its texts, so this one way covers them.
/// The entities in an order where each comes after the records it holds
/// (DESIGN 5.25), as C and C++ declare a struct before one that holds it.
/// The checker keeps a record from holding itself.
pub(crate) fn holding_order(project: &Project) -> Vec<&clarity_spec::model::Entity> {
    fn visit<'p>(project: &'p Project, entity: &'p clarity_spec::model::Entity, out: &mut Vec<&'p clarity_spec::model::Entity>, busy: &mut Vec<String>) {
        if out.iter().any(|e| e.name.name == entity.name.name) || busy.contains(&entity.name.name) {
            return;
        }
        busy.push(entity.name.name.clone());
        for field in &entity.fields {
            if let clarity_spec::model::FieldType::Record { entity: held } = &field.ty
                && let Some(held) = project.entity(&held.name)
            {
                visit(project, held, out, busy);
            }
        }
        out.push(entity);
    }
    let mut out = Vec::new();
    for entity in project.entities() {
        visit(project, entity, &mut out, &mut Vec::new());
    }
    out
}

/// The name a program keeps the person's things under: the folder of its
/// settings, and its secrets in the system's protected store (DESIGN 5.17,
/// 5.27). The project's name in snake case (`ShopDesk` is `shop_desk`), the
/// same on every stack and for its command-line program too, so an
/// application built on another stack finds what this one kept
/// (conformance/secret-store.json). The executable, crate, module and
/// namespace keep their own names.
pub fn settings_program(settings: &Settings) -> String {
    clarity_spec::model::snake_case(&settings.name)
}

/// `project` with each web API that signs with a secret setting naming it as
/// `setting:<program>/<name>` (DESIGN 5.27): where the runtime finds it in
/// the system's protected store, under the program's own name, which each
/// stack's settings use too.
pub(crate) fn with_secret_settings(project: &Project, program: &str) -> Project {
    let mut project = project.clone();
    for source in &mut project.sources {
        if let Some((_, secret)) = &mut source.auth
            && let Some(name) = secret.strip_prefix("setting:").map(str::to_string)
        {
            *secret = format!("setting:{program}/{name}");
        }
    }
    project
}

/// The databases with each connection kept as a secret setting named as
/// `setting:<program>/<name>` (DESIGN 5.27), as `with_secret_settings` does
/// for a web API's key.
pub(crate) fn with_secret_locations(databases: Vec<(String, ddl::Sql, String)>, program: &str) -> Vec<(String, ddl::Sql, String)> {
    databases
        .into_iter()
        .map(|(name, sql, at)| match at.strip_prefix("setting:") {
            Some(setting) => (name, sql, format!("setting:{program}/{setting}")),
            None => (name, sql, at),
        })
        .collect()
}

/// What an application does when its databases do not open (DESIGN 5.27):
/// the app's `on unconnected` handler, by its app and handler names, and
/// the window that keeps a secret connection. None when it just stops.
pub(crate) struct Unconnected {
    pub window: Option<String>,
    /// The app and its handler, which takes why as its one value, or none.
    pub handler: Option<(String, String)>,
}

pub(crate) fn unconnected(project: &Project) -> Option<Unconnected> {
    let handler = project
        .files
        .iter()
        .flat_map(|f| &f.apps)
        .find_map(|a| a.handlers.iter().find(|h| h.event == clarity_spec::widgets::Event::Unconnected).map(|h| (a.name.name.clone(), h.handler.clone())));
    let window = project.connection_window().map(|p| p.name.name.clone());
    (handler.is_some() || window.is_some()).then_some(Unconnected { window, handler })
}

/// Whether the `on unconnected` handler `name` of `app` takes why, as text.
pub(crate) fn unconnected_takes_reason(project: &Project, app: &str, name: &str) -> bool {
    project.files.iter().flat_map(|f| &f.code).filter(|b| b.name.name == app).flat_map(|b| &b.handlers).any(|h| h.name.name == name && h.params.len() == 1)
}

/// The secret settings' names as a list a program hands its runtime (DESIGN 5.27).
pub(crate) fn secret_names(project: &Project) -> Vec<String> {
    project.secret_settings().into_iter().map(str::to_string).collect()
}

pub fn with_resource_texts(project: &Project) -> Project {
    let mut project = project.clone();
    let markdown = project.resources.as_ref().map(|r| r.markdown.clone()).unwrap_or_default();
    for (language, keys) in &project.resource_keys {
        let texts = project.translations.entry(language.clone()).or_default();
        for (key, file) in keys {
            texts.insert(key.clone(), markdown.get(file).cloned().unwrap_or_else(|| file.clone()));
        }
    }
    project
}

/// Engines written in Clarity become WebAssembly files (DESIGN 5.12, N5b):
/// each is compiled, or built for the interpreter when the project says
/// `engines interpreted`, as `<name>.wasm` among the engine files; its keys
/// go to its `says`, and from then on every stack takes it as any engine file.
fn compile_engines(mut project: Project, settings: &Settings) -> Result<Project, Vec<String>> {
    let mode = if settings.engines_interpreted { clarity_wasm::Mode::Interpreted } else { clarity_wasm::Mode::Compiled };
    let written: Vec<String> = written_engines(&project).iter().map(|(_, e, _)| e.name.name.clone()).collect();
    let mut compiled = Vec::new();
    let mut errors = Vec::new();
    for name in &written {
        match clarity_wasm::build(&project, name, mode) {
            Ok(bytes) => compiled.push((name.clone(), bytes)),
            Err(mut problems) => errors.append(&mut problems),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut files = project.engines.take().unwrap_or_default();
    for (name, bytes) in compiled {
        let file = format!("{}.wasm", name.to_lowercase());
        if files.iter().any(|f| f.name == file) {
            return Err(vec![format!("engine {name} is written in Clarity, so it becomes {file}, which the engines folder already has")]);
        }
        let decl = project.files.iter_mut().flat_map(|f| &mut f.engines).find(|e| e.name.name == name).expect("a written engine");
        if let Some(body) = decl.body.take() {
            for key in clarity_spec::translations::engine_keys(&body) {
                let said = format!("@{key}");
                if !decl.says.contains(&said) {
                    decl.says.push(said);
                }
            }
        }
        decl.file = file.clone();
        files.push(clarity_spec::engines::EngineFile { name: file, bytes });
    }
    project.engines = Some(files);
    Ok(project)
}

pub fn generate(project: &Project, settings: &Settings) -> Result<Vec<OutFile>, Vec<String>> {
    if project.has_errors() {
        return Err(vec!["the project has errors; fix the errors before generating it".to_string()]);
    }
    // The texts that show the app's version carry it, in every stack; the agent settings start as the project allows.
    let filled = agent_tools::with_agent_defaults(&clarity_spec::translations::with_app_slots(project), settings);
    let project = &filled;
    check_stores(project, settings)?;
    check_web_secrets(project)?;
    check_parts_in_browser(project, settings)?;
    // A custom part is written in the project's own language (DESIGN 5.19).
    let misfits: Vec<String> = clarity_project::target_problems(settings, project, "")
        .into_iter()
        .map(|d| d.message)
        .filter(|m| m.starts_with("part ") || m.starts_with("library ") || m.contains(" clicks to keys,"))
        .collect();
    if !misfits.is_empty() {
        return Err(misfits);
    }
    // Each view shows as parts of the window that shows it (clarity_spec::views).
    let mut project = compile_engines(with_resource_texts(&consts::inlined(&calls::filled(&clarity_spec::views::placed(project)))), settings)?;
    // The keys window and the person's keys, where the project file allows them (DESIGN 5.28).
    project.keys_editable = settings.keys_editable;
    // How high the windows' rows are, which their markup carries.
    project.row_height = clarity_project::row_height(settings);
    let project = &project;
    let mut files = match settings.stack.backend {
        Backend::Rust if settings.stack.ui == clarity_project::Ui::Clarity => rust::generate(project, settings)?,
        Backend::Rust => {
            return Err(vec!["Rust draws its windows with Clarity.UI only; write stack rust desktop".to_string()]);
        }
        Backend::Csharp if settings.stack.ui == clarity_project::Ui::Clarity => csharp::generate(project, settings)?,
        Backend::Csharp => {
            return Err(vec!["C# draws with Clarity.UI only".to_string()]);
        }
        Backend::Go => go::generate(project, settings)?,
        Backend::C => c::generate(project, settings)?,
        Backend::Cpp => cpp::generate(project, settings)?,
    };
    // The codec libraries the project adds, carried and added as the program starts (docs/VIDEO-PLAN.md).
    if settings.stack.has(clarity_project::Target::Desktop) {
        with_codecs(&mut files, crate::codec_libraries(project, settings)?, settings.stack.backend);
    }
    // The package carries the app's version, as each stack's build keeps it.
    if let Some(version) = &project.version {
        versioned(&mut files, version);
    }
    // A program that writes its own README (C#'s web program: a server and a
    // browser program) keeps it, with where its data is; the others get the
    // README of their stack.
    let general = readme::readme(settings, project);
    match files.iter_mut().find(|f| f.path == "README.md") {
        Some(own) => {
            if let Some(at) = general.find("\n## Data\n") {
                own.text.push_str(&general[at..]);
            }
        }
        None => files.push(OutFile { path: "README.md".to_string(), text: general, bytes: None }),
    }
    // The icons the application shows are Lucide's, whose license travels with them.
    if !clarity_spec::icons::used(project).is_empty() {
        files.push(OutFile {
            path: "THIRD_PARTY_NOTICES.md".to_string(),
            text: format!(
                "# Third-party notices\n\nThe icons this application shows are from [Lucide](https://lucide.dev), under this license:\n\n```text\n{}```\n",
                clarity_spec::icons::LICENSE
            ),
            bytes: None,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// One program of a project: what a target of its stack builds, written
/// into its own folder of `out/` (DESIGN 5.20).
#[derive(Debug, Clone)]
pub struct Program {
    pub target: clarity_project::Target,
    pub files: Vec<OutFile>,
}

/// The programs of the project: the program with windows when a target
/// shows them (the desktop program, or the web program when it is served
/// to a browser alone), and the command-line program with the `cli` target.
/// C# with both `desktop` and `web` gives both programs, each in its own
/// folder: its desktop program cannot serve a browser.
pub fn programs(project: &Project, settings: &Settings) -> Result<Vec<Program>, Vec<String>> {
    use clarity_project::Target;
    if let Some(why) = pictures_without_library(project, settings) {
        return Err(vec![why]);
    }
    let mut programs = Vec::new();
    if settings.stack.backend == Backend::Csharp && settings.stack.has(Target::Desktop) && settings.stack.has(Target::Web) {
        for target in [Target::Desktop, Target::Web] {
            let other = if target == Target::Desktop { Target::Web } else { Target::Desktop };
            let mut alone = settings.clone();
            alone.stack.targets.retain(|t| *t != other);
            programs.push(Program { target, files: generate(project, &alone)? });
        }
    } else if settings.stack.has_ui() {
        programs.push(Program { target: windowed_target(settings), files: generate(project, settings)? });
    }
    if settings.stack.has(Target::Cli) {
        programs.push(Program { target: Target::Cli, files: generate_cli(project, settings)? });
    }
    Ok(programs)
}

/// Why the project's picture functions cannot run in a program of its stack:
/// Go, C and C++ link Clarity.Image beside Clarity.UI's library, which only
/// their desktop program has (a desktop program also serves a browser).
pub(crate) fn pictures_without_library(project: &Project, settings: &Settings) -> Option<String> {
    use clarity_project::Target;
    let linked = matches!(settings.stack.backend, Backend::Go | Backend::C | Backend::Cpp);
    let alone = (settings.stack.has(Target::Web) && !settings.stack.has(Target::Desktop)) || settings.stack.has(Target::Cli);
    (linked && alone && uses_picture_functions(project)).then(|| {
        "the code calls picture functions, which a Go, C or C++ program runs on Clarity.Image beside Clarity.UI's library: only in a desktop program for now (stack ... desktop, which can also serve a browser)".to_string()
    })
}

/// The target of the program with windows, and its folder of `out/`: the
/// desktop's, or the web's when it is served to a browser alone.
pub fn windowed_target(settings: &Settings) -> clarity_project::Target {
    use clarity_project::Target;
    if settings.stack.has(Target::Desktop) { Target::Desktop } else { Target::Web }
}

/// The files of the command-line program, sorted by path; or why it cannot
/// be generated.
pub fn generate_cli(project: &Project, settings: &Settings) -> Result<Vec<OutFile>, Vec<String>> {
    if project.has_errors() {
        return Err(vec!["the project has errors; fix the errors before generating it".to_string()]);
    }
    check_stores(project, settings)?;
    check_web_secrets(project)?;
    let project = &agent_tools::with_agent_defaults(project, settings);
    // One file per module, its windows' own files (`.cxaml`) folded in, as the generators read a module.
    let project = &compile_engines(with_resource_texts(&consts::inlined(&calls::filled(&clarity_spec::cxaml::folded(project)))), settings)?;
    match settings.stack.backend {
        Backend::Rust => rust::generate_cli(project, settings),
        Backend::Go => go::generate_cli(project, settings),
        Backend::Csharp => csharp::generate_cli(project, settings),
        Backend::C => c::generate_cli(project, settings),
        Backend::Cpp => cpp::generate_cli(project, settings),
    }
}

/// Whether `dir` holds anything that is not Clarity's: its marker, the
/// folders the build tools make, and the ones the project keeps are its
/// own, so a folder holding only those is Clarity's to write in, as an
/// empty one is. A folder it cannot read holds nothing it has to spare.
fn holds_other_files(dir: &std::path::Path, mine: &[&str]) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    entries.flatten().any(|entry| {
        let name = entry.file_name().to_string_lossy().into_owned();
        name != OUT_MARKER && !mine.contains(&name.as_str())
    })
}

/// Writes each program into its folder of `out` (`out/desktop` or
/// `out/web`, and `out/cli`), as `write_out` writes one, and removes the
/// folder of a target no longer generated. Output from before, with the
/// program with windows in `out` itself, moves into its folder: its build
/// tools' folders and data are kept there, its other files go. Returns how
/// many files changed.
pub fn write_programs(out: &std::path::Path, programs: &[Program], backend: Backend, keeps: &[String]) -> std::io::Result<usize> {
    write_programs_listed(out, programs, backend, keeps).map(|done| done.written.len())
}

/// As `write_programs`, saying what it did: each path under `out`, with
/// the folder of its program (`desktop/Program.cs`).
pub fn write_programs_listed(out: &std::path::Path, programs: &[Program], backend: Backend, keeps: &[String]) -> std::io::Result<Written> {
    use clarity_project::{TARGETS, target_dir, target_word};
    let targets: Vec<&str> = TARGETS.iter().map(|t| target_word(*t)).collect();
    // The stack's build tools make their own folders; the project names any others it keeps.
    let kept: Vec<&str> = build_output(backend).iter().copied().chain(keeps.iter().map(String::as_str)).collect();
    let marked = out.join(OUT_MARKER).exists() || out.join("clarity-runtime/src/ui.rs").exists();
    let mine: Vec<&str> = targets.iter().copied().chain(kept.iter().copied()).collect();
    if !marked && holds_other_files(out, &mine) {
        return Err(std::io::Error::other(format!("{} holds files not made by Clarity; choose an empty or new output folder", out.display())));
    }
    std::fs::create_dir_all(out)?;
    for entry in std::fs::read_dir(out)? {
        let path = entry?.path();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if targets.contains(&name.as_str()) || name == OUT_MARKER {
            continue;
        }
        // The old place: what the build tools keep moves on, the rest is made again.
        let windowed = programs.iter().map(|p| p.target).find(|t| *t != clarity_project::Target::Cli).unwrap_or(clarity_project::Target::Desktop);
        let desktop = target_dir(out, windowed);
        if kept.contains(&name.as_str()) && !desktop.join(&name).exists() {
            std::fs::create_dir_all(&desktop)?;
            std::fs::write(desktop.join(OUT_MARKER), OUT_MARKER_TEXT)?;
            std::fs::rename(&path, desktop.join(&name))?;
        } else if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    std::fs::write(out.join(OUT_MARKER), OUT_MARKER_TEXT)?;
    let mut done = Written::default();
    for program in programs {
        let dir = target_dir(out, program.target);
        let mut here = Written::default();
        write_out_listed(&dir, &program.files, &kept, &mut here)?;
        let folder = clarity_project::slashed(dir.strip_prefix(out).unwrap_or(&dir));
        let under = |path: String| if folder.is_empty() { path } else { format!("{folder}/{path}") };
        done.written.extend(here.written.into_iter().map(under));
        done.removed.extend(here.removed.into_iter().map(under));
        // What Build and Run check they build: these files, as generated.
        std::fs::write(dir.join(GENERATED_STAMP), format!("{}\n", fingerprint(program)))?;
    }
    for target in TARGETS {
        let dir = target_dir(out, target);
        if !programs.iter().any(|p| p.target == target) && dir.join(OUT_MARKER).exists() {
            std::fs::remove_dir_all(&dir)?;
            done.removed.push(format!("{}/", clarity_project::slashed(dir.strip_prefix(out).unwrap_or(&dir))));
        }
    }
    Ok(done)
}

/// Writes `files` into `dir`, which then holds exactly these files: files
/// are all written again, each time, and files no longer generated are removed.
/// `keep` names top-level entries that are not generated, such as the
/// build tool's `target` folder.
pub fn write_out(dir: &std::path::Path, files: &[OutFile], keep: &[&str]) -> std::io::Result<usize> {
    let mut done = Written::default();
    write_out_listed(dir, files, keep, &mut done)?;
    Ok(done.written.len())
}

/// What writing the output did: every file written, and those removed, by
/// their path in the output folder.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Written {
    pub written: Vec<String>,
    pub removed: Vec<String>,
}

/// As `write_out`, adding what it did to `done`, its paths under `dir`.
fn write_out_listed(dir: &std::path::Path, files: &[OutFile], keep: &[&str], done: &mut Written) -> std::io::Result<()> {
    // Removing stale files is only safe in a folder Clarity made: refuse a
    // folder that holds anything without the marker.
    // Output from before the marker is recognized by the runtime Clarity copies in.
    let marked = dir.join(OUT_MARKER).exists() || dir.join("clarity-runtime/src/ui.rs").exists();
    if !marked && holds_other_files(dir, keep) {
        return Err(std::io::Error::other(format!(
            "{} holds files not made by Clarity; choose an empty or new output folder",
            dir.display()
        )));
    }
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(OUT_MARKER), OUT_MARKER_TEXT)?;
    for file in files {
        let path = dir.join(&file.path);
        let contents = file.bytes.as_deref().unwrap_or(file.text.as_bytes());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, contents)?;
        done.written.push(file.path.clone());
    }
    remove_stale_patch(dir, files, done)?;
    remove_stale(dir, dir, files, keep, done)
}

/// The Rust patch Clarity wrote (`.cargo/config.toml`, starting with its own
/// line) goes when it is no longer generated: nothing names the runtime's
/// folder any more (Clarity settings > Libraries reset, CLARITY_RUNTIME
/// unset), so the program builds on the published crates. One written by
/// hand stays, as the rest of `.cargo` does.
fn remove_stale_patch(dir: &std::path::Path, files: &[OutFile], done: &mut Written) -> std::io::Result<()> {
    const PATCH: &str = ".cargo/config.toml";
    let path = dir.join(PATCH);
    let ours = std::fs::read_to_string(&path).is_ok_and(|text| text.starts_with("# Written by Clarity because"));
    if ours && !files.iter().any(|f| f.path == PATCH) {
        std::fs::remove_file(&path)?;
        done.removed.push(PATCH.to_string());
    }
    Ok(())
}

fn remove_stale(root: &std::path::Path, dir: &std::path::Path, files: &[OutFile], keep: &[&str], done: &mut Written) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let relative = clarity_project::slashed(path.strip_prefix(root).unwrap_or(&path));
        if dir == root && keep.contains(&relative.as_str()) {
            continue;
        }
        if path.is_dir() {
            remove_stale(root, &path, files, keep, done)?;
            if std::fs::read_dir(&path)?.next().is_none() {
                std::fs::remove_dir(&path)?;
            }
        } else if relative != OUT_MARKER && relative != BUILT_STAMP && relative != GENERATED_STAMP && !files.iter().any(|f| f.path == relative) {
            std::fs::remove_file(&path)?;
            done.removed.push(relative);
        }
    }
    Ok(())
}

/// What a program's build was made from, written beside it, so Run starts
/// the program as built while nothing changed (every stack).
pub const BUILT_STAMP: &str = ".clarity-built";

/// The file in a program's folder that names the files Generate wrote there (their `fingerprint`).
pub const GENERATED_STAMP: &str = ".clarity-generated";

/// The fingerprint of the files Generate last wrote in `dir`; none when it wrote none there.
pub fn last_generated(dir: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(dir.join(GENERATED_STAMP)).ok().map(|s| s.trim().to_string())
}

/// A mark of a program's generated files, their paths and contents: the
/// same files give the same mark.
pub fn fingerprint(program: &Program) -> String {
    use std::hash::{Hash, Hasher};
    let mut files: Vec<&OutFile> = program.files.iter().collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for file in files {
        file.path.hash(&mut hasher);
        file.bytes.as_deref().unwrap_or(file.text.as_bytes()).hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

/// Records that the program in `dir` was built from `fingerprint`, as a
/// release program or (not `release`) a debug one.
pub fn record_build(dir: &std::path::Path, fingerprint: &str, release: bool) -> std::io::Result<()> {
    let kind = if release { "release" } else { "debug" };
    std::fs::write(dir.join(BUILT_STAMP), format!("{kind} {fingerprint}\n"))
}

/// Whether the last build in `dir` was made from `fingerprint`: `Some(true)`
/// for a release program, `Some(false)` for a debug one, `None` when it was
/// made from other files or nothing was built.
pub fn last_build(dir: &std::path::Path, fingerprint: &str) -> Option<bool> {
    let stamp = std::fs::read_to_string(dir.join(BUILT_STAMP)).ok()?;
    match stamp.trim().split_once(' ')? {
        ("release", made) if made == fingerprint => Some(true),
        ("debug", made) if made == fingerprint => Some(false),
        _ => None,
    }
}

/// How the build tools of a stack build its generated project in `out/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolchain {
    /// The command, such as `cargo`.
    pub tool: &'static str,
    pub args: Vec<String>,
    /// Environment the tool needs for plain line output.
    pub env: Vec<(&'static str, String)>,
    /// The program the build makes, relative to `out/`.
    pub program: std::path::PathBuf,
    /// What to say when the tool cannot be started.
    pub missing: &'static str,
}

/// The build of the project `settings` describes, as a debug or (with
/// `release`) a release program; `None` for a stack Clarity cannot generate yet.
pub fn toolchain(settings: &Settings, release: bool) -> Option<Toolchain> {
    let exe = |path: String| std::path::PathBuf::from(if cfg!(windows) { format!("{path}.exe") } else { path });
    match settings.stack.backend {
        Backend::Rust => {
            let mut args = vec!["build".to_string(), "--color".to_string(), "never".to_string()];
            if release {
                args.push("--release".to_string());
            }
            let profile = if release { "release" } else { "debug" };
            Some(Toolchain {
                tool: "cargo",
                args,
                // No progress bars, which only make sense in a terminal.
                env: vec![("CARGO_TERM_PROGRESS_WHEN", "never".into())],
                program: exe(format!("target/{profile}/{}", rust::crate_name(settings))),
                missing: "is Rust installed?",
            })
        }
        Backend::Csharp if settings.stack.ui == clarity_project::Ui::Clarity => {
            let configuration = if release { "Release" } else { "Debug" };
            // The web application is built as its server, which builds the application it serves (DESIGN 7.6).
            let web = windowed_target(settings) == clarity_project::Target::Web;
            let (project, folder, program) = if web {
                (vec!["Server".to_string()], "Server/", format!("{}.Server", csharp::project_name(settings)))
            } else {
                (vec![], "", csharp::project_name(settings))
            };
            Some(Toolchain {
                tool: "dotnet",
                // `-tl:off`: plain lines, not the terminal logger.
                args: ["build".to_string()].into_iter().chain(project).chain(["--nologo", "-tl:off", "-c", configuration].map(String::from)).collect(),
                env: vec![("DOTNET_CLI_TELEMETRY_OPTOUT", "1".into()), ("DOTNET_NOLOGO", "1".into())],
                program: exe(format!("{folder}bin/{configuration}/{}/{program}", csharp::TARGET_FRAMEWORK)),
                missing: "is the .NET SDK installed?",
            })
        }
        Backend::Go => {
            // `go build -o` writes the name it is given and adds nothing of
            // its own, so the name it builds under is the name Run starts,
            // `.exe` on Windows and all.
            let program = exe(format!("bin/{}", go::module_name(settings)));
            let built = program.to_string_lossy().into_owned();
            // -v names each package as it is built: a first build is silent otherwise.
            let mut args = vec!["build".to_string(), "-v".to_string()];
            if release {
                // Without debug information.
                args.extend(["-trimpath", "-ldflags=-s -w"].map(String::from));
            }
            args.extend(["-o".to_string(), built, ".".to_string()]);
            // The desktop program links Clarity.UI's library, from the folder CLARITY_UI names (DESIGN 7.9).
            let env = if settings.stack.has(clarity_project::Target::Desktop) { go_cgo_env(std::env::var_os("CLARITY_UI").as_deref().map(std::path::Path::new), std::env::consts::OS) } else { Vec::new() };
            Some(Toolchain {
                tool: "go",
                args,
                env,
                program,
                missing: "is Go installed?",
            })
        }
        Backend::C | Backend::Cpp => {
            // CMakePresets.json: one workflow configures and builds.
            let preset = if release { "release" } else { "debug" };
            let program = if settings.stack.backend == Backend::C { c::program_name(settings) } else { cpp::program_name(settings) };
            Some(Toolchain {
                tool: "cmake",
                args: ["--workflow", "--preset", preset].map(String::from).to_vec(),
                env: vec![],
                program: exe(format!("build/{preset}/{program}")),
                missing: "is CMake installed?",
            })
        }
        // C# with the web renderer is not generated.
        Backend::Csharp => None,
    }
}

/// What cgo needs to build a Go desktop program on Clarity.UI's library in
/// `clarity_ui` (a folder with include/clarity_ui.h and lib/), on system
/// `os`: the header's folder, the library's, and where the program finds
/// the shared library when it runs: beside it (where a shipped program has
/// it, as C's and C++'s builds copy it), else in that folder. Nothing when
/// the folder is not named: the build then says the header is missing.
pub fn go_cgo_env(clarity_ui: Option<&std::path::Path>, os: &str) -> Vec<(&'static str, String)> {
    let Some(folder) = clarity_ui else { return Vec::new() };
    let lib = folder.join("lib");
    let lib = lib.display();
    // Windows finds the DLL beside the program or on PATH; it has no rpath.
    let found = match os {
        "windows" => String::new(),
        "macos" => format!(" -Wl,-rpath,@executable_path -Wl,-rpath,{lib}"),
        _ => format!(" -Wl,-rpath,$ORIGIN -Wl,-rpath,{lib}"),
    };
    vec![("CGO_ENABLED", "1".into()), ("CGO_CFLAGS", format!("-I{}", folder.join("include").display())), ("CGO_LDFLAGS", format!("-L{lib}{found}"))]
}

/// The codec libraries a project with videos adds (docs/VIDEO-PLAN.md), for
/// this system: each library's file name (`libvp9.so`) and the file carrying
/// it in the program's `codecs` folder. The `from` of each is resolved by
/// `clarity_project::build_settings`; a library that is not there is an error.
pub(crate) fn codec_libraries(project: &Project, settings: &Settings) -> Result<Vec<(String, OutFile)>, Vec<String>> {
    if settings.codecs.is_empty() || !clarity_spec::resources::plays_videos(project) {
        return Ok(Vec::new());
    }
    let extension = match std::env::consts::OS {
        "windows" => "dll",
        "macos" => "dylib",
        _ => "so",
    };
    let mut found = Vec::new();
    let mut errors = Vec::new();
    for codec in &settings.codecs {
        let path = std::path::PathBuf::from(format!("{}.{extension}", codec.from));
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        match std::fs::read(&path) {
            Ok(bytes) => found.push((name.clone(), OutFile { path: format!("codecs/{name}"), text: String::new(), bytes: Some(bytes.into()) })),
            Err(_) => errors.push(format!("codec {}: there is no library at {} for this system", codec.name, path.display())),
        }
    }
    if errors.is_empty() { Ok(found) } else { Err(errors) }
}

/// The migration steps of data source `source` for SQLite, as the program
/// applies them when it starts: each step's title and statements.
pub fn sqlite_steps(project: &Project, source: &str) -> Result<Vec<(String, Vec<String>)>, Vec<String>> {
    schema::steps(project, source, Sql::Sqlite)
}

/// Whether the app's handlers carry the settings: the app has the project's
/// settings as its data, so a handler of the app (a menu item's) starts from
/// the settings of the window it runs from and hands them back in the reply,
/// as a window's own handler does. Every stack's dispatch follows this.
pub(crate) fn app_carries_settings(project: &Project, app: &str) -> bool {
    project.apps().any(|a| a.name.name == app && a.data.iter().any(|d| d.name.name == clarity_spec::parser::SETTINGS_DATA))
}

/// A built-in function of code (conformance/texts.json, conformance/zip.json)
/// by the name each runtime gives it, written out so every name is in source.
pub(crate) struct BuiltIn {
    pub name: &'static str,
    pub rust: &'static str,
    pub go: &'static str,
    pub csharp: &'static str,
    pub c: &'static str,
    pub cpp: &'static str,
}

pub(crate) const BUILT_INS: [BuiltIn; 19] = [
    BuiltIn { name: "words", rust: "clarity_runtime::texts::words", go: "clarity.TextWords", csharp: "Clarity.Runtime.Texts.Words", c: "clarity_text_words", cpp: "clarity::words" },
    BuiltIn { name: "split", rust: "clarity_runtime::texts::split", go: "clarity.TextSplit", csharp: "Clarity.Runtime.Texts.Split", c: "clarity_text_split", cpp: "clarity::split" },
    BuiltIn { name: "trim", rust: "clarity_runtime::texts::trim", go: "clarity.TextTrim", csharp: "Clarity.Runtime.Texts.Trim", c: "clarity_text_trim", cpp: "clarity::trim" },
    BuiltIn { name: "lower", rust: "clarity_runtime::texts::lower", go: "clarity.TextLower", csharp: "Clarity.Runtime.Texts.Lower", c: "clarity_text_lower", cpp: "clarity::lower" },
    BuiltIn { name: "upper", rust: "clarity_runtime::texts::upper", go: "clarity.TextUpper", csharp: "Clarity.Runtime.Texts.Upper", c: "clarity_text_upper", cpp: "clarity::upper" },
    BuiltIn { name: "replace", rust: "clarity_runtime::texts::replace", go: "clarity.TextReplace", csharp: "Clarity.Runtime.Texts.Replace", c: "clarity_text_replace", cpp: "clarity::replace" },
    BuiltIn { name: "startsWith", rust: "clarity_runtime::texts::starts_with", go: "clarity.TextStartsWith", csharp: "Clarity.Runtime.Texts.StartsWith", c: "clarity_text_starts_with", cpp: "clarity::starts_with" },
    BuiltIn { name: "endsWith", rust: "clarity_runtime::texts::ends_with", go: "clarity.TextEndsWith", csharp: "Clarity.Runtime.Texts.EndsWith", c: "clarity_text_ends_with", cpp: "clarity::ends_with" },
    BuiltIn { name: "slice", rust: "clarity_runtime::texts::slice", go: "clarity.TextSlice", csharp: "Clarity.Runtime.Texts.Slice", c: "clarity_text_slice", cpp: "clarity::slice" },
    BuiltIn { name: "indexOf", rust: "clarity_runtime::texts::index_of", go: "clarity.TextIndexOf", csharp: "Clarity.Runtime.Texts.IndexOf", c: "clarity_text_index_of", cpp: "clarity::index_of" },
    BuiltIn { name: "zip", rust: "clarity_runtime::zip::stored", go: "clarity.Zip", csharp: "Clarity.Runtime.Zip.Of", c: "clarity_zip", cpp: "clarity::zip" },
    // The picture functions (docs/IMAGE-PLAN.md), through Clarity.Image; Go's are in the program's pictures.go.
    BuiltIn { name: "resize", rust: "clarity_runtime::picture_functions::resize", go: "pictureResize", csharp: "Clarity.Image.PictureFunctions.Resize", c: "clarity_picture_resize", cpp: "clarity::picture_resize" },
    BuiltIn { name: "crop", rust: "clarity_runtime::picture_functions::crop", go: "pictureCrop", csharp: "Clarity.Image.PictureFunctions.Crop", c: "clarity_picture_crop", cpp: "clarity::picture_crop" },
    BuiltIn { name: "rotate", rust: "clarity_runtime::picture_functions::rotate", go: "pictureRotate", csharp: "Clarity.Image.PictureFunctions.Rotate", c: "clarity_picture_rotate", cpp: "clarity::picture_rotate" },
    BuiltIn { name: "flip", rust: "clarity_runtime::picture_functions::flip", go: "pictureFlip", csharp: "Clarity.Image.PictureFunctions.Flip", c: "clarity_picture_flip", cpp: "clarity::picture_flip" },
    BuiltIn { name: "blend", rust: "clarity_runtime::picture_functions::blend", go: "pictureBlend", csharp: "Clarity.Image.PictureFunctions.Blend", c: "clarity_picture_blend", cpp: "clarity::picture_blend" },
    BuiltIn { name: "adjust", rust: "clarity_runtime::picture_functions::adjust", go: "pictureAdjust", csharp: "Clarity.Image.PictureFunctions.Adjust", c: "clarity_picture_adjust", cpp: "clarity::picture_adjust" },
    BuiltIn { name: "pictureWidth", rust: "clarity_runtime::picture_functions::width", go: "pictureWidth", csharp: "Clarity.Image.PictureFunctions.Width", c: "clarity_picture_width", cpp: "clarity::picture_width" },
    BuiltIn { name: "pictureHeight", rust: "clarity_runtime::picture_functions::height", go: "pictureHeight", csharp: "Clarity.Image.PictureFunctions.Height", c: "clarity_picture_height", cpp: "clarity::picture_height" },
];

/// The built-in function `name`, unless the project has a function of that name.
pub(crate) fn built_in(project: &Project, name: &str) -> Option<&'static BuiltIn> {
    BUILT_INS.iter().find(|b| b.name == name).filter(|_| project.files.iter().flat_map(|f| &f.functions).all(|f| f.name.name != name))
}


/// Puts `added` after the first `marker` of the file `path`; nothing when either is missing.
fn insert_after(files: &mut [OutFile], path: &str, marker: &str, added: &str) {
    if let Some(file) = files.iter_mut().find(|f| f.path == path)
        && let Some(at) = file.text.find(marker)
    {
        file.text.insert_str(at + marker.len(), added);
    }
}

/// The codec libraries in the program's `codecs` folder, and each stack's
/// lines that add them to Clarity.Video as the program starts, after it is
/// handed to Clarity.UI. C and C++ copy the folder beside the program
/// (`clarity_use_video`), C# copies it to its output, a Go program has it
/// beside itself, and Clarity.Video also looks in `codecs` where a program runs.
fn with_codecs(files: &mut Vec<OutFile>, codecs: Vec<(String, OutFile)>, backend: Backend) {
    if codecs.is_empty() {
        return;
    }
    let names: Vec<String> = codecs.iter().map(|(name, _)| name.clone()).collect();
    files.extend(codecs.into_iter().map(|(_, file)| file));
    let quoted = |name: &str| serde_json::Value::String(name.to_string()).to_string();
    match backend {
        Backend::Rust => {
            // Clarity.Video's table, from its library beside Clarity.UI's.
            let lines: String = names
                .iter()
                .map(|n| format!("    if let Err(why) = clarity_runtime::native_ui::add_codec(unsafe {{ clarity_video_api() }}, {}) {{\n        eprintln!(\"{{why}}\");\n    }}\n", quoted(n)))
                .collect();
            let run = "    clarity_runtime::native_ui::run(app, FILES)";
            if let Some(main) = files.iter_mut().find(|f| f.path == "src/main.rs")
                && let Some(at) = main.text.find(run)
            {
                main.text.insert_str(at, &format!("    // The codec libraries the project adds (docs/VIDEO-PLAN.md).\n{lines}"));
            }
        }
        Backend::Csharp => {
            let lines: String = names.iter().map(|n| format!("        Clarity.Video.ClarityVideo.AddCodec({});\n", quoted(n))).collect();
            insert_after(files, "Program.cs", "Clarity.Video.ClarityVideo.Use();\n", &lines);
            if let Some(project) = files.iter_mut().find(|f| f.path.ends_with(".csproj") && !f.path.contains('/')) {
                project.text = project.text.replacen("</Project>", "  <!-- The codec libraries the project adds, beside the program (docs/VIDEO-PLAN.md). -->\n  <ItemGroup>\n    <None Include=\"codecs/**\" CopyToOutputDirectory=\"PreserveNewest\" />\n  </ItemGroup>\n</Project>", 1);
            }
        }
        Backend::Go => {
            let lines: String = names.iter().map(|n| format!("\tif C.add_codec(C.CString({q}), {len}) != C.CLARITY_VIDEO_OK {{\n\t\tfmt.Fprintln(os.Stderr, \"the codec library {n} could not be added\")\n\t}}\n", q = quoted(n), len = n.len())).collect();
            insert_after(files, "video.go", "#include \"clarity_ui.h\"\n", "\nstatic int add_codec(const char *file, size_t len) { return clarity_video_api()->add_codec(file, len); }\n");
            if let Some(video) = files.iter_mut().find(|f| f.path == "video.go")
                && let Some(at) = video.text.rfind("}\n")
            {
                video.text.insert_str(at, &format!("\t// The codec libraries the project adds (docs/VIDEO-PLAN.md).\n{lines}"));
            }
        }
        Backend::C | Backend::Cpp => {
            let (main, error) = if backend == Backend::C { ("src/main.c", "fputs(\"the codec library NAME could not be added\\n\", stderr);") } else { ("src/main.cpp", "std::cerr << \"the codec library NAME could not be added\\n\";") };
            let lines: String = names.iter().map(|n| format!("    if (clarity_video_api()->add_codec({}, {}) != CLARITY_VIDEO_OK) {}\n", quoted(n), n.len(), error.replace("NAME", n))).collect();
            if let Some(file) = files.iter_mut().find(|f| f.path == main)
                && let Some(at) = file.text.find("clarity_ui_use_video(clarity_video_api())")
                && let Some(end) = file.text[at..].find('\n')
            {
                file.text.insert_str(at + end + 1, &format!("    // The codec libraries the project adds (docs/VIDEO-PLAN.md).\n{lines}"));
            }
        }
    }
}

/// The crates of the base libraries every Go, C, C++ and Rust desktop program
/// links: Clarity.UI and Clarity.Audio, Clarity.Video, Clarity.Image,
/// Clarity.Formats, Clarity.Auth and Clarity.AppMcp beside it (a program of any
/// target whose code uses a format links Clarity.Formats too, one that signs
/// people in Clarity.Auth, a command-line program with commands for agents Clarity.AppMcp).
pub const BASE_LIBRARIES: [&str; 7] = ["clarity-ui-ffi", "clarity-audio-ffi", "clarity-video-ffi", "clarity-image-ffi", "clarity-formats-ffi", "clarity-auth-ffi", "clarity-app-mcp-ffi"];

/// Clarity's repository holding `from` or a folder above it; none outside one.
pub fn repository_of(from: &std::path::Path) -> Option<std::path::PathBuf> {
    from.ancestors().find(|d| d.join("runtimes/rust-ui-ffi/Cargo.toml").is_file()).map(std::path::Path::to_path_buf)
}

/// Rebuilds the base libraries from the source in Clarity's repository
/// `repo` (release builds into its target/release), so the programs Clarity
/// builds link this source's Clarity.UI: how a change of Clarity.UI is tried.
/// The folder they are in, as CLARITY_UI names it.
pub fn rebuild_base_libraries(repo: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let mut command = std::process::Command::new("cargo");
    command.current_dir(repo).args(["build", "--release"]);
    for library in BASE_LIBRARIES {
        command.args(["-p", library]);
    }
    let status = command.status().map_err(|e| format!("cannot run cargo: {e}"))?;
    if !status.success() {
        return Err("cargo could not build the base libraries; it says why above".to_string());
    }
    clarity_ui::repository_library(repo, std::env::consts::OS).ok_or_else(|| format!("the base libraries are not in {} after the build", repo.join("target/release").display()))
}

/// Whether the windowed build (not the command-line program) links
/// Clarity.UI's library from the folder CLARITY_UI names: a Go, C, C++ or
/// Rust desktop program (DESIGN 7.9). C# brings the library in its package.
/// Whether a build of the program of `target` needs Clarity.UI's folder
/// (CLARITY_UI): a Go, C, C++ or Rust desktop program draws with its library,
/// and a Go, C or C++ program of any target whose code reads or writes a
/// format links Clarity.Formats from beside it (docs/FORMATS-PLAN.md).
pub fn needs_library_folder(project: &Project, settings: &clarity_project::Settings, target: clarity_project::Target) -> bool {
    let draws = target == clarity_project::Target::Desktop && uses_clarity_ui(settings);
    let formats = carries_formats(project, settings) && matches!(settings.stack.backend, Backend::Go | Backend::C | Backend::Cpp);
    // A program that signs people in links Clarity.Auth from beside Clarity.UI's library.
    let auth = carries_auth(project) && matches!(settings.stack.backend, Backend::Go | Backend::C | Backend::Cpp);
    draws || formats || auth
}

pub fn uses_clarity_ui(settings: &clarity_project::Settings) -> bool {
    let stack = &settings.stack;
    let links = match stack.backend {
        Backend::Go | Backend::C | Backend::Cpp | Backend::Rust => true,
        _ => false,
    };
    links && stack.has(clarity_project::Target::Desktop)
}

/// `toolchain` building with Clarity.UI's library in `folder` (the one
/// CLARITY_UI names, or the one Clarity fetched): cgo's flags for Go, the
/// variable CLARITY_UI for CMake.
pub fn with_clarity_ui(mut toolchain: Toolchain, backend: Backend, folder: &std::path::Path) -> Toolchain {
    let env = match backend {
        Backend::Go => go_cgo_env(Some(folder), std::env::consts::OS),
        Backend::C | Backend::Cpp | Backend::Rust => vec![("CLARITY_UI", folder.display().to_string())],
        _ => Vec::new(),
    };
    toolchain.env.retain(|(name, _)| !env.iter().any(|(n, _)| n == name));
    toolchain.env.extend(env);
    toolchain
}

/// A command line typed in Clarity (Run on a command-line program) as the
/// words a shell gives the program: apart where there are spaces, together
/// within "double" or 'single' quotes.
pub fn command_words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word: Option<String> = None;
    let mut quote: Option<char> = None;
    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.get_or_insert_with(String::new).push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                word.get_or_insert_with(String::new);
            }
            (None, c) if c.is_whitespace() => words.extend(word.take()),
            (None, c) => word.get_or_insert_with(String::new).push(c),
        }
    }
    words.extend(word);
    words
}

/// The build of the command-line program (DESIGN 5.20) in `out/cli`, with
/// the stack's own tools; the program is named as it is typed.
pub fn cli_toolchain(settings: &Settings, release: bool) -> Option<Toolchain> {
    let program = commands::program_name(settings);
    let exe = |path: String| std::path::PathBuf::from(if cfg!(windows) { format!("{path}.exe") } else { path });
    let mut toolchain = match settings.stack.backend {
        // C# has no web renderer to need: its console program builds the same way on either UI.
        Backend::Csharp => {
            let configuration = if release { "Release" } else { "Debug" };
            Toolchain {
                tool: "dotnet",
                args: ["build", "--nologo", "-tl:off", "-c", configuration].map(String::from).to_vec(),
                env: vec![("DOTNET_CLI_TELEMETRY_OPTOUT", "1".into()), ("DOTNET_NOLOGO", "1".into())],
                program: exe(format!("bin/{configuration}/{}/{program}", csharp::TARGET_FRAMEWORK)),
                missing: "is the .NET SDK installed?",
            }
        }
        _ => toolchain(settings, release)?,
    };
    match settings.stack.backend {
        Backend::Rust => toolchain.program = exe(format!("target/{}/{program}", if release { "release" } else { "debug" })),
        Backend::Go => {
            // The name it builds under is the name Run starts; see `toolchain`.
            let into = exe(format!("bin/{program}"));
            let built = into.to_string_lossy().into_owned();
            let mut args = vec!["build".to_string(), "-v".to_string()];
            if release {
                args.extend(["-trimpath", "-ldflags=-s -w"].map(String::from));
            }
            args.extend(["-o".to_string(), built, ".".to_string()]);
            toolchain.args = args;
            toolchain.program = into;
        }
        Backend::C | Backend::Cpp => toolchain.program = exe(format!("build/{}/{program}", if release { "release" } else { "debug" })),
        Backend::Csharp => {}
    }
    Some(toolchain)
}

/// What the build tools of `backend` put in `out/`, which `write_out`
/// leaves alone, and the application's `data` (its database while it runs
/// from `out/`). Everything else in `out/` is the generated project, so
/// the build output of another stack goes when a project changes stack.
pub fn build_output(backend: Backend) -> &'static [&'static str] {
    match backend {
        // cargo's `target` and lock file, and the patch that builds on the
        // runtime in Clarity's repository while working on it.
        Backend::Rust => &["target", "data", "Cargo.lock", ".cargo"],
        // dotnet's `bin` and `obj`.
        Backend::Csharp => &["bin", "obj", "data"],
        // go build's program, and a workspace that builds the module on the
        // runtime in Clarity's repository while working on it.
        Backend::Go => &["bin", "data", "go.work", "go.work.sum"],
        // CMake's builds.
        Backend::C | Backend::Cpp => &["build", "data"],
    }
}

/// The file that marks a folder as Clarity's output. `write_out` only
/// removes files in folders that have it.
pub const OUT_MARKER: &str = ".clarity-out";

const OUT_MARKER_TEXT: &str =
    "This folder is generated by Clarity from a project's .clarity files. Clarity replaces its contents on every build.\n";

/// Where `http ... from source` calls (DESIGN 5.23): the web API's address,
/// and its auth as the project file writes it (kind, where the secret is).
pub(crate) fn web_source<'p>(project: &'p Project, name: &str) -> Option<(&'p str, Option<(&'p str, &'p str)>)> {
    let source = project.sources.iter().find(|s| s.name == name && !s.sql)?;
    Some((source.address.as_str(), source.auth.as_ref().map(|(kind, secret)| (kind.as_str(), secret.as_str()))))
}

/// How a web API data source pages, which `every page` follows (DESIGN 5.23).
pub(crate) fn web_paging<'p>(project: &'p Project, name: &str) -> Option<&'p clarity_spec::Paging> {
    project.sources.iter().find(|s| s.name == name && !s.sql)?.paging.as_ref()
}

/// The word of a paging kind as the runtimes name it: `page`, `offset`, `cursor`, `link`.
pub(crate) fn paging_word(kind: clarity_spec::PagingKind) -> &'static str {
    match kind {
        clarity_spec::PagingKind::Page => "page",
        clarity_spec::PagingKind::Offset => "offset",
        clarity_spec::PagingKind::Cursor => "cursor",
        clarity_spec::PagingKind::Link => "link",
    }
}

/// A custom part is a library of the stack's own language (DESIGN 6.5, 7.9),
/// which runs beside Clarity.UI on the desktop; in a browser Clarity.UI runs
/// alone, compiled once for every language, so a program served to a
/// browser alone has none (a .NET part too, since C#'s browser drawing went).
fn check_parts_in_browser(project: &Project, settings: &Settings) -> Result<(), Vec<String>> {
    use clarity_project::Target;
    if settings.stack.has(Target::Desktop) || !settings.stack.has(Target::Web) {
        return Ok(());
    }
    let found: Vec<String> = project
        .files
        .iter()
        .flat_map(|f| &f.parts)
        .filter(|p| p.library.is_some())
        .map(|p| format!("part {} draws in a window of the desktop, which a program served to a browser has not; add the desktop target, or leave the part out", p.name.name))
        .collect();
    if found.is_empty() { Ok(()) } else { Err(found) }
}

/// Why a generated application cannot sign the calls of its web API sources:
/// a secret named as a keychain entry, which is reached through a setting.
fn check_web_secrets(project: &Project) -> Result<(), Vec<String>> {
    let found: Vec<String> = project
        .sources
        .iter()
        .filter_map(|s| s.auth.as_ref().filter(|(_, secret)| secret.starts_with("keychain:")).map(|(_, secret)| (s, secret)))
        .map(|(s, secret)| format!("data source '{}': {}", s.name, clarity_project::keychain_problem(secret)))
        .collect();
    if found.is_empty() { Ok(()) } else { Err(found) }
}

/// The program's build files with the app's version: Rust's `Cargo.toml`
/// (`version`), C#'s project (`<Version>`), and CMake's `project(... VERSION)`
/// for C and C++, which takes only its numbers (a pre-release stays out).
pub fn versioned(files: &mut [OutFile], version: &str) {
    let numbers = version.split('-').next().unwrap_or(version);
    for file in files.iter_mut() {
        if file.path == "Cargo.toml" {
            file.text = file.text.replacen("version = \"0.1.0\"\n", &format!("version = \"{version}\"\n"), 1);
        } else if file.path.ends_with(".csproj") && !file.path.contains('/') {
            file.text = file.text.replacen("    <OutputType>", &format!("    <Version>{version}</Version>\n    <OutputType>"), 1);
        } else if file.path == "CMakeLists.txt" {
            for language in [" LANGUAGES C)", " LANGUAGES CXX)"] {
                if let Some(at) = file.text.find(language)
                    && file.text[..at].rfind("project(").is_some_and(|start| !file.text[start..at].contains('\n'))
                {
                    file.text.insert_str(at, &format!(" VERSION {numbers}"));
                    break;
                }
            }
        }
    }
}
