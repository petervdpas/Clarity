//! Transpiles a checked Clarity project into a real application for the
//! stack its project file names (docs/DESIGN.md section 1.2). Generation is
//! deterministic: the same project always gives the same files, byte for
//! byte, so `out/` can be compared, cached and regenerated at any time.

pub mod tools;
mod c;
pub mod libraries;
pub mod parts;
mod commands;
mod csharp;
mod ddl;
pub(crate) mod go;
mod cpp;
mod print;
mod query;
mod readme;
mod rust;
mod schema;
pub mod web;

pub use csharp::{avalonia_theme, runtime_version as csharp_runtime_version};
pub use go::runtime_version as go_runtime_version;
pub use rust::runtime_version as rust_runtime_version;
pub use rust::migration_sql;
pub use web::{APP_CLASS, base_css, theme_css, theme_file};

use clarity_project::{Backend, Settings};
use clarity_spec::Project;

use ddl::Sql;

/// The databases of an application and the ones Clarity owns.
type Databases = (Vec<(String, Sql, String)>, Vec<(String, Sql)>);

/// The database of every data source that holds entities, with its kind and
/// where it is, and the ones Clarity owns (and so migrates); external ones
/// are opened as they are. `backend` names the backend in messages.
fn databases(project: &Project, settings: &Settings, backend: &str) -> Result<Databases, Vec<String>> {
    if settings.datasources.is_empty() {
        return Err(vec!["the project needs a data source to store its records".to_string()]);
    }
    let mut databases: Vec<(String, Sql, String)> = Vec::new();
    for source in &settings.datasources {
        if project.entities_in(&source.name).next().is_none() {
            continue;
        }
        let Some(sql) = Sql::of(source.kind) else {
            return Err(vec![format!(
                "data source '{}' is a web API; the {backend} backend stores records in SQL databases only for now",
                source.name
            )]);
        };
        if source.url.starts_with("keychain:") {
            return Err(vec![format!(
                "data source '{}': a generated application reads its connection from 'env:NAME' for now, not from the keychain",
                source.name
            )]);
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

/// The size the window `name` opens at, in pixels: its own (`size 320 x
/// 460`), or the standard size every stack shares.
pub(crate) fn window_size(project: &Project, name: &str) -> (u32, u32) {
    project
        .pages()
        .find(|p| p.name.name == name)
        .and_then(|p| p.size)
        .map_or((clarity_spec::layout::WINDOW_WIDTH, clarity_spec::layout::WINDOW_HEIGHT), |s| (s.width, s.height))
}

/// Every engine the project declares (DESIGN 5.11).
/// What a `TypeRef` names, as a checker type.
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

/// The engines loaded from WebAssembly files (`engine X from x.wasm`).
pub(crate) fn engines(project: &Project) -> Vec<&clarity_spec::code::EngineDecl> {
    project.files.iter().flat_map(|f| &f.engines).filter(|e| e.body.is_none()).collect()
}

/// The engines written in Clarity, each with the file that declares it:
/// every stack generates their code like that of functions.
pub(crate) fn written_engines(project: &Project) -> Vec<(&clarity_spec::model::SpecFile, &clarity_spec::code::EngineDecl, &clarity_spec::code::EngineBody)> {
    project
        .files
        .iter()
        .flat_map(|f| f.engines.iter().filter_map(move |e| e.body.as_ref().map(|b| (f, e, b))))
        .collect()
}

/// The engine `name`, loaded or written.
pub(crate) fn engine<'a>(project: &'a Project, name: &str) -> Option<&'a clarity_spec::code::EngineDecl> {
    project.files.iter().flat_map(|f| &f.engines).find(|e| e.name.name == name)
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
/// the web stacks through the runtime's `deselect`, on Avalonia because the
/// table is given new rows.
pub(crate) fn rows_shown(page: Option<&clarity_spec::model::Page>) -> std::collections::BTreeMap<String, Vec<String>> {
    use clarity_spec::model::Widget;
    use clarity_spec::widgets::WidgetKind;
    fn walk(widgets: &[Widget], out: &mut std::collections::BTreeMap<String, Vec<String>>) {
        for widget in widgets {
            let p = &widget.props;
            if let (WidgetKind::Table | WidgetKind::List | WidgetKind::Tree | WidgetKind::TreeTable, Some(name), Some(bind)) = (p.kind, &p.name, &p.bind)
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

pub fn generate(project: &Project, settings: &Settings) -> Result<Vec<OutFile>, Vec<String>> {
    if project.has_errors() {
        return Err(vec!["the project has errors; fix the errors before generating it".to_string()]);
    }
    check_stores(project, settings)?;
    // A custom part is written in the project's own language (DESIGN 5.19).
    let misfits: Vec<String> = clarity_project::target_problems(settings, project, "").into_iter().map(|d| d.message).filter(|m| m.starts_with("part ") || m.starts_with("library ")).collect();
    if !misfits.is_empty() {
        return Err(misfits);
    }
    let project = &with_resource_texts(project);
    let mut files = match settings.stack.backend {
        Backend::Rust => rust::generate(project, settings)?,
        Backend::Csharp if settings.stack.ui == clarity_project::Ui::Avalonia => csharp::generate(project, settings)?,
        Backend::Csharp => {
            return Err(vec!["C# with the web renderer is not generated yet; C# uses Avalonia".to_string()]);
        }
        Backend::Go => go::generate(project, settings)?,
        Backend::C => c::generate(project, settings)?,
        Backend::Cpp => cpp::generate(project, settings)?,
    };
    files.push(OutFile {
        path: "README.md".to_string(),
        text: readme::readme(settings),
        bytes: None,
    });
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

/// The programs of the project: the desktop program when a target shows
/// windows, and the command-line program with the `cli` target.
pub fn programs(project: &Project, settings: &Settings) -> Result<Vec<Program>, Vec<String>> {
    use clarity_project::Target;
    let mut programs = Vec::new();
    if settings.stack.has_ui() {
        programs.push(Program { target: Target::Desktop, files: generate(project, settings)? });
    }
    if settings.stack.has(Target::Cli) {
        programs.push(Program { target: Target::Cli, files: generate_cli(project, settings)? });
    }
    Ok(programs)
}

/// The files of the command-line program, sorted by path; or why it cannot
/// be generated.
pub fn generate_cli(project: &Project, settings: &Settings) -> Result<Vec<OutFile>, Vec<String>> {
    if project.has_errors() {
        return Err(vec!["the project has errors; fix the errors before generating it".to_string()]);
    }
    check_stores(project, settings)?;
    let project = &with_resource_texts(project);
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

/// Writes each program into its folder of `out` (`out/desktop`, `out/cli`),
/// as `write_out` writes one, and removes the folder of a target no longer
/// generated. Output from before, with the desktop program in `out` itself,
/// moves into `out/desktop`: its build tools' folders and data are kept
/// there, its other files go. Returns how many files changed.
pub fn write_programs(out: &std::path::Path, programs: &[Program], backend: Backend, keeps: &[String]) -> std::io::Result<usize> {
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
        let desktop = target_dir(out, clarity_project::Target::Desktop);
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
    let mut written = 0;
    for program in programs {
        written += write_out(&target_dir(out, program.target), &program.files, &kept)?;
    }
    for target in TARGETS {
        let dir = target_dir(out, target);
        if !programs.iter().any(|p| p.target == target) && dir.join(OUT_MARKER).exists() {
            std::fs::remove_dir_all(&dir)?;
        }
    }
    Ok(written)
}

/// Writes `files` into `dir`, which then holds exactly these files: files
/// that changed are rewritten, unchanged ones are left alone (so build
/// tools see no change), and files no longer generated are removed.
/// `keep` names top-level entries that are not generated, such as the
/// build tool's `target` folder.
pub fn write_out(dir: &std::path::Path, files: &[OutFile], keep: &[&str]) -> std::io::Result<usize> {
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
    let mut written = 0;
    for file in files {
        let path = dir.join(&file.path);
        let contents = file.bytes.as_deref().unwrap_or(file.text.as_bytes());
        if std::fs::read(&path).ok().as_deref() == Some(contents) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, contents)?;
        written += 1;
    }
    remove_stale(dir, dir, files, keep)?;
    Ok(written)
}

fn remove_stale(root: &std::path::Path, dir: &std::path::Path, files: &[OutFile], keep: &[&str]) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        if dir == root && keep.contains(&relative.as_str()) {
            continue;
        }
        if path.is_dir() {
            remove_stale(root, &path, files, keep)?;
            if std::fs::read_dir(&path)?.next().is_none() {
                std::fs::remove_dir(&path)?;
            }
        } else if relative != OUT_MARKER && !files.iter().any(|f| f.path == relative) {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// How the build tools of a stack build its generated project in `out/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolchain {
    /// The command, such as `cargo`.
    pub tool: &'static str,
    pub args: Vec<String>,
    /// Environment the tool needs for plain line output.
    pub env: Vec<(&'static str, &'static str)>,
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
                env: vec![("CARGO_TERM_PROGRESS_WHEN", "never")],
                program: exe(format!("target/{profile}/{}", rust::crate_name(settings))),
                missing: "is Rust installed?",
            })
        }
        Backend::Csharp if settings.stack.ui == clarity_project::Ui::Avalonia => {
            let configuration = if release { "Release" } else { "Debug" };
            Some(Toolchain {
                tool: "dotnet",
                // `-tl:off`: plain lines, not the terminal logger.
                args: ["build", "--nologo", "-tl:off", "-c", configuration].map(String::from).to_vec(),
                env: vec![("DOTNET_CLI_TELEMETRY_OPTOUT", "1"), ("DOTNET_NOLOGO", "1")],
                program: exe(format!("bin/{configuration}/{}/{}", csharp::TARGET_FRAMEWORK, csharp::project_name(settings))),
                missing: "is the .NET SDK installed?",
            })
        }
        Backend::Go => {
            // `go build -o` writes the name it is given and adds nothing of
            // its own, so the name it builds under is the name Run starts,
            // `.exe` on Windows and all.
            let program = exe(format!("bin/{}", go::module_name(settings)));
            let built = program.to_string_lossy().into_owned();
            // -v names each package as it is built: a first build compiles
            // WebKit's bindings for minutes, and is silent otherwise.
            let mut args = vec!["build".to_string(), "-v".to_string()];
            if release {
                // Wails' production build, without debug information.
                args.extend(["-tags", "production", "-trimpath", "-ldflags=-s -w"].map(String::from));
            }
            args.extend(["-o".to_string(), built, ".".to_string()]);
            Some(Toolchain {
                tool: "go",
                args,
                env: vec![],
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
                env: vec![("DOTNET_CLI_TELEMETRY_OPTOUT", "1"), ("DOTNET_NOLOGO", "1")],
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
        // cargo's `target` and lock file, Tauri's `gen`, and the patch that
        // builds on the runtime in Clarity's repository while working on it.
        Backend::Rust => &["target", "data", "gen", "Cargo.lock", ".cargo"],
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
