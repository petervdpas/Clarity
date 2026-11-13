//! The Clarity app backend. It owns parsing, checking, sample data, edits
//! and their history; the frontend only renders the `Preview` it receives.

mod browser_keys;
mod fail_safe;
mod build;
mod logging;
mod links;
mod foundry;
mod diagrams;
mod resources;
mod engines;
mod environment;
mod libraries;
mod folders;
mod system;
mod parts;
mod external;
mod mappings;
mod apis;
mod themes;
mod translations;
mod vault;
mod acp;
mod browse;
mod agent;
mod manual;
mod migrations;
mod splash;
mod dev_rebuild;
mod backups;
mod resource_urls;
mod own_build;

use crate::fail_safe::Held;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;

use clarity_project::{Catalog, ProjectFile, Settings, Stack};
// Writing files and new projects, shared with the MCP server (crates/clarity-tools).
use clarity_tools::files::{FileChange, is_spec_file, write_atomically};
use clarity_tools::new_project::write_new_project;
use clarity_spec::Preview;
use clarity_spec::edit::Edit;
use clarity_spec::rename::RenameKind;
use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

/// Event sent to the frontend whenever a watched spec file changes.
const PREVIEW_UPDATED: &str = "preview-updated";

/// Event sent to the frontend whenever the project file changes.
const PROJECT_UPDATED: &str = "project-updated";

/// Editors often write a file in several steps; changes arriving within
/// this window are handled as one reload.
const DEBOUNCE: Duration = Duration::from_millis(150);

/// The open project: its file, the folder with its modules next to it, and
/// the watcher. Dropping the watcher stops watching and ends the reload
/// thread.
struct OpenProject {
    file: PathBuf,
    dir: PathBuf,
    watcher: notify::RecommendedWatcher,
}

#[derive(Default)]
struct WatchState {
    open: Mutex<Option<OpenProject>>,
}

/// One applied change: an edit of one file, or a rename across several.
/// It is undone and redone as a whole.
struct Change(Vec<FileChange>);

/// Edits that can be undone and redone, newest last.
#[derive(Default)]
struct History {
    undo: Vec<Change>,
    redo: Vec<Change>,
}

/// How many edits can be undone.
const HISTORY_LIMIT: usize = 200;

#[derive(Default)]
struct HistoryState(Mutex<History>);

/// The result of a designer edit: the reloaded project and the offset of
/// the element the edit produced or moved, so the designer can select it.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct EditApplied {
    pub preview: Preview,
    pub focus: Option<u32>,
}

fn load(dir: &Path) -> Result<Preview, String> {
    let project = clarity_project::load_modules(dir)
        .map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    Ok(Preview::new(project))
}

/// True for events that change a module of `dir` or the project file.
/// Access events are ignored: reloading reads the files, and reacting to
/// those reads would reload in an endless loop.
fn changes_project(event: &notify::Event, file: &Path, dir: &Path) -> bool {
    use notify::EventKind;
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) && event.paths.iter().any(|p| {
        p == file
            || *p == dir.join(clarity_spec::README_FILE)
            || (is_spec_file(p) && p.parent() == Some(dir))
            // A window's or view's own file, in its module's folder.
            || (clarity_spec::cxaml::is_page_file(&p.to_string_lossy()) && clarity_spec::cxaml::project_dir_of(p) == Some(dir))
            // Anything in a folder of the project, and the folder itself.
            || clarity_spec::PROJECT_FOLDERS.iter().any(|f| p.starts_with(dir.join(f)))
    })
}

/// Watches the module folder, and the folder holding the project file (a
/// save replaces the file, so the file itself cannot be watched).
fn watch(file: &Path, dir: &Path, app: AppHandle) -> Result<notify::RecommendedWatcher, String> {
    let (tx, rx) = mpsc::channel::<()>();
    let (watched_file, watched_dir) = (file.to_path_buf(), dir.to_path_buf());
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event
            && changes_project(&event, &watched_file, &watched_dir)
        {
            let _ = tx.send(());
        }
    })
    .map_err(|err| format!("cannot watch {}: {err}", dir.display()))?;
    for folder in [dir, file.parent().unwrap_or(dir)] {
        watcher
            .watch(folder, RecursiveMode::NonRecursive)
            .map_err(|err| format!("cannot watch {}: {err}", folder.display()))?;
    }
    // Every folder of the project it has (clarity_spec::PROJECT_FOLDERS), with what lies in its folders.
    for name in clarity_spec::PROJECT_FOLDERS {
        let folder = dir.join(name);
        if folder.is_dir() {
            watcher
                .watch(&folder, RecursiveMode::Recursive)
                .map_err(|err| format!("cannot watch {}: {err}", folder.display()))?;
        }
    }

    let (file, dir) = (file.to_path_buf(), dir.to_path_buf());
    std::thread::spawn(move || {
        // Ends when the watcher (and with it the sender) is dropped.
        while rx.recv().is_ok() {
            while rx.recv_timeout(DEBOUNCE).is_ok() {}
            // A new revision of a web API's description, or other picks, write its module again;
            // writing it is a change too, after which nothing differs.
            let _ = clarity_project::refresh_apis(&file);
            if let Ok(preview) = load(&dir) {
                let _ = app.emit(PREVIEW_UPDATED, preview);
            }
            let _ = app.emit(PROJECT_UPDATED, clarity_project::load(&file));
        }
    });

    Ok(watcher)
}

/// Opens a project file and starts watching it and its modules, replacing
/// any previous project.
#[tauri::command]
fn open_project(
    path: String,
    app: AppHandle,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<Preview, String> {
    open_file(&path, app, &state, &history)
}

/// Reads the open project from disk again and checks it, keeping the undo
/// history (opening the project again would start a new one).
#[tauri::command]
fn check_project(state: State<WatchState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    load(&dir)
}

/// The project file at `path`, with the folder of its modules.
fn project_paths(path: &str) -> Result<(PathBuf, PathBuf), String> {
    let file = clarity_project::plain(PathBuf::from(path).canonicalize().map_err(|err| format!("cannot open {path}: {err}"))?);
    if file.is_dir() || !is_spec_file(&file) {
        return Err(format!(
            "{} is not a project file; open the .clarity file next to the project's folder",
            file.display()
        ));
    }
    let dir = clarity_project::module_dir(&file);
    if !dir.is_dir() {
        return Err(format!("the folder {} with the project's modules is missing", dir.display()));
    }
    Ok((file, dir))
}

fn open_file(path: &str, app: AppHandle, state: &WatchState, history: &HistoryState) -> Result<Preview, String> {
    let (file, dir) = project_paths(path)?;
    // The modules of the web APIs, as their descriptions and picks are now (DESIGN 5.23).
    let _ = clarity_project::refresh_apis(&file);
    let preview = load(&dir)?;
    let watcher = watch(&file, &dir, app)?;
    // `clarity mcp` works on the project open here (docs/MCP-PLAN.md).
    if let Err(err) = clarity_project::ide::set_open_project(&file) {
        logging::line(&err);
    }
    // The file browser's Recent projects.
    if let Err(err) = clarity_project::recent::remember_project(&file) {
        logging::line(&err);
    }
    *state.open.held().map_err(|_| "watch state is poisoned")? = Some(OpenProject {
        file,
        dir,
        watcher,
    });
    // Undo works within one project.
    *history.0.held().map_err(|_| "history is poisoned")? = History::default();
    Ok(preview)
}

/// Applies a designer edit to one spec file of the open directory.
/// `version` is the file version the designer saw; if the file changed
/// since, the edit is refused instead of overwriting someone else's work.
#[tauri::command]
fn apply_edit(
    path: String,
    version: String,
    edit: Edit,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<EditApplied, String> {
    let (dir, file) = spec_file_of(&state, &path)?;
    let source =
        std::fs::read_to_string(&file).map_err(|err| format!("cannot read {path}: {err}"))?;
    if clarity_spec::content_version(&source) != version {
        return Err(format!(
            "{} changed on disk since the designer loaded it; the view has been refreshed, try again",
            file.file_name().unwrap_or_default().to_string_lossy()
        ));
    }

    // The designer's edit, made as every agent's tools make it (clarity_tools::edits): checked, written, one Undo.
    let applied = clarity_tools::edits::apply(&dir, &file, &edit)?;
    let focus = applied.focus;
    if !applied.changes.is_empty() {
        record(&history, applied.changes)?;
    }
    Ok(EditApplied {
        preview: load(&dir)?,
        focus,
    })
}

/// Moves every page still written in a module's file into its own `.cxaml`
/// in `design/`: Move pages into design, under
/// Problems, as `clarity convert-pages` does. One change for Undo.
#[tauri::command]
fn move_pages_out(state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let dir = open_dir_of(&state)?;
    let read_err = |err: std::io::Error| format!("cannot read {}: {err}", dir.display());
    let mut files = Vec::new();
    for path in clarity_spec::spec_files(&dir).map_err(read_err)? {
        let text = std::fs::read_to_string(&path).map_err(read_err)?;
        files.push((path.display().to_string(), text));
    }
    let moved = clarity_spec::cxaml::moved_out(&files)?;
    let mut changes = Vec::new();
    for (path, after) in moved {
        let file = PathBuf::from(&path);
        let before = files.iter().find(|(p, _)| *p == path).map(|(_, t)| t.clone());
        if before.is_none() && file.exists() {
            return Err(format!("{} is there already; nothing was moved", file.display()));
        }
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
        }
        changes.push(FileChange { file, before, after: Some(after) });
    }
    for change in &changes {
        change.make()?;
    }
    record(&history, changes)?;
    load(&dir)
}

/// The code behind of a window or the app, as the code panel shows it.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct CodeView {
    pub text: String,
    pub problems: Vec<clarity_spec::Diagnostic>,
}

/// The code of `owner` in the spec file at `path`, with its problems.
#[tauri::command]
fn code_text(path: String, owner: String, state: State<WatchState>) -> Result<CodeView, String> {
    let (dir, file) = spec_file_of(&state, &path)?;
    // The code of a window's .cxaml is in its module's file.
    let file = code_file_of(file)?;
    let source = std::fs::read_to_string(&file).map_err(|err| format!("cannot read {path}: {err}"))?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    Ok(CodeView {
        problems: clarity_spec::code::edit::problems_in_code(&source, &owner, &file.display().to_string(), &project.diagnostics),
        text: clarity_spec::code::edit::code_text(&source, &owner),
    })
}

/// The problems `text` would have as the code of `owner`, without saving
/// it. The code panel calls this while the developer types.
#[tauri::command]
fn check_code(
    path: String,
    owner: String,
    text: String,
    state: State<WatchState>,
) -> Result<Vec<clarity_spec::Diagnostic>, String> {
    let (dir, file) = spec_file_of(&state, &path)?;
    // The code of a window's .cxaml is in its module's file.
    let file = code_file_of(file)?;
    let source = std::fs::read_to_string(&file).map_err(|err| format!("cannot read {path}: {err}"))?;
    let after = clarity_spec::code::edit::set_code(&source, &owner, &text).map_err(|err| err.to_string())?;
    let project = clarity_project::load_modules_with(&dir, &file, &after)
        .map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    Ok(clarity_spec::code::edit::problems_in_code(&after, &owner, &file.display().to_string(), &project.diagnostics))
}

/// The text of the declaration at `item` (a window, the app, a function,
/// ...) in the spec file at `path`, with its problems.
#[tauri::command]
fn decl_text(path: String, item: u32, state: State<WatchState>) -> Result<CodeView, String> {
    let (dir, file) = spec_file_of(&state, &path)?;
    let source = std::fs::read_to_string(&file).map_err(|err| format!("cannot read {path}: {err}"))?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    // A window's or view's own file is its whole text, its markup.
    if is_page_file(&file) {
        let shown = file.display().to_string();
        return Ok(CodeView { text: source, problems: project.diagnostics.iter().filter(|d| d.path == shown).cloned().collect() });
    }
    Ok(CodeView {
        text: clarity_spec::source::decl_text(&source, item).map_err(|err| err.to_string())?,
        problems: clarity_spec::source::problems_in_decl(&source, item, &file.display().to_string(), &project.diagnostics),
    })
}

/// What a declaration would look like with `text`, without saving it: its
/// problems, and the whole project rendered with it, so the designer can
/// follow the text while it is typed.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct DeclTrial {
    pub problems: Vec<clarity_spec::Diagnostic>,
    pub preview: Preview,
}

/// Clarity text in colored pieces, as the lexer reads it, for the code editors.
#[tauri::command]
fn highlight(text: String, markup: Option<bool>) -> Vec<clarity_spec::highlight::Piece> {
    // A window's or view's .cxaml is colored as markup.
    if markup == Some(true) { clarity_spec::highlight::highlight_markup(&text) } else { clarity_spec::highlight::highlight(&text) }
}

#[tauri::command]
fn try_decl(path: String, item: u32, text: String, state: State<WatchState>, server: State<PreviewServer>) -> Result<DeclTrial, String> {
    let (dir, file) = spec_file_of(&state, &path)?;
    let source = std::fs::read_to_string(&file).map_err(|err| format!("cannot read {path}: {err}"))?;
    // A window's or view's own file is tried as a whole.
    let after = if is_page_file(&file) { text.clone() } else { clarity_spec::source::replace_decl(&source, item, &text).map_err(|err| err.to_string())? };
    let project = clarity_project::load_modules_with(&dir, &file, &after)
        .map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    // The design canvas draws the window as this text makes it (DESIGN 8.1).
    if let Some(server) = &server.0 {
        server.set_draft(Some(project.clone()));
    }
    Ok(DeclTrial {
        problems: clarity_spec::source::problems_in_decl(&after, item, &file.display().to_string(), &project.diagnostics),
        preview: Preview::new(project),
    })
}

/// Generates the open project into `out/`, builds it and, with `run`,
/// starts it; with a `command_line`, the command-line program with those
/// words. Progress arrives as `build-line` and `build-done` events.
#[tauri::command]
fn build_app(release: bool, run: bool, command_line: Option<String>, app: AppHandle, state: State<WatchState>) -> Result<(), String> {
    build::start(app, project_file_of(&state)?, release, run, command_line)
}

/// What Generate did: the output folder, and the files it wrote and removed there.
#[derive(serde::Serialize)]
struct GeneratedApp {
    out: String,
    #[serde(flatten)]
    written: clarity_gen::Written,
}

/// Transpiles the open project into its source tree in the output folder,
/// without building it; says what it wrote.
#[tauri::command]
fn generate_app(state: State<WatchState>) -> Result<GeneratedApp, String> {
    let file = project_file_of(&state)?;
    build::generate_source(&file).map(|source| GeneratedApp { out: source.out.display().to_string(), written: source.written })
}

/// Types a line to the command-line program Run started, as its input.
#[tauri::command]
fn type_line(line: String, app: AppHandle) -> Result<(), String> {
    build::type_line(&app, &line)
}

/// Ends the application Run started.
#[tauri::command]
fn stop_app(app: AppHandle) {
    build::stop(&app);
}

/// The last build, for a screen that reloaded while it ran: its lines and how it ended.
#[tauri::command]
fn build_snapshot(app: AppHandle) -> Option<build::BuildSnapshot> {
    build::snapshot(&app)
}

/// The application Run started, while it runs: where a browser opens it.
#[tauri::command]
fn app_state(app: AppHandle) -> Option<build::AppRunning> {
    build::running(&app)
}

/// The open directory and the canonical path of one of its spec files.
fn spec_file_of(state: &WatchState, path: &str) -> Result<(PathBuf, PathBuf), String> {
    let dir = open_dir_of(state)?;
    let file = clarity_project::plain(PathBuf::from(path).canonicalize().map_err(|err| format!("cannot open {path}: {err}"))?);
    let module_file = file.parent() == Some(dir.as_path()) && is_spec_file(&file);
    // A window's or view's own file, in its module's folder.
    let page_file = is_page_file(&file) && clarity_spec::cxaml::project_dir_of(&file) == Some(dir.as_path());
    if !module_file && !page_file {
        return Err(format!("{path} is not a spec file of the open directory"));
    }
    Ok((dir, file))
}

fn is_page_file(path: &Path) -> bool {
    clarity_spec::cxaml::is_page_file(&path.to_string_lossy())
}

/// The file whose code blocks a file's pages run: a window's `.cxaml`
/// runs its module's file of Clarity; a module's file its own.
fn code_file_of(file: PathBuf) -> Result<PathBuf, String> {
    if !is_page_file(&file) {
        return Ok(file);
    }
    clarity_spec::cxaml::module_file(&file).ok_or_else(|| format!("{} is in no module's folder", file.display()))
}

/// Remembers an applied change for Undo; a new change clears Redo.
fn record(history: &HistoryState, files: Vec<FileChange>) -> Result<(), String> {
    let mut history = history.0.held().map_err(|_| "history is poisoned")?;
    history.redo.clear();
    history.undo.push(Change(files));
    if history.undo.len() > HISTORY_LIMIT {
        history.undo.remove(0);
    }
    Ok(())
}

/// Renames an entity, an enum, a window or a view and every reference to
/// it, in all files; a window's or view's file takes the new name.
#[tauri::command]
fn rename(
    kind: RenameKind,
    name: String,
    new_name: String,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<Preview, String> {
    let dir = open_dir_of(&state)?;
    let before = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let at = |path: &str| if Path::new(path).is_absolute() { PathBuf::from(path) } else { dir.join(path) };
    let source = |path: &str| std::fs::read_to_string(at(path)).ok();
    let renamed = clarity_spec::rename::rename(&before, source, kind, &name, new_name.trim())?;
    write_renamed(&dir, &before, renamed, &history)
}

/// Renames field `name` of entity or record `entity` and every place that
/// names it, in all files: the entity editor's name cell asks for it.
#[tauri::command]
fn rename_field(entity: String, name: String, new_name: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let dir = open_dir_of(&state)?;
    let before = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let at = |path: &str| if Path::new(path).is_absolute() { PathBuf::from(path) } else { dir.join(path) };
    let source = |path: &str| std::fs::read_to_string(at(path)).ok();
    let renamed = clarity_spec::rename::rename_field(&before, source, &entity, &name, new_name.trim())?;
    write_renamed(&dir, &before, renamed, &history)
}

/// Writes a rename's files once the whole project checks with them in
/// place, as one change for Undo. A path is the project's (relative to
/// `dir`, such as `translations/en.clarity`) or absolute.
fn write_renamed(dir: &Path, before: &clarity_spec::Project, renamed: Vec<clarity_spec::rename::Renamed>, history: &HistoryState) -> Result<Preview, String> {
    let read_err = |err: std::io::Error| format!("cannot read {}: {err}", dir.display());
    if renamed.is_empty() {
        return load(dir);
    }
    let at = |path: &str| if Path::new(path).is_absolute() { PathBuf::from(path) } else { dir.join(path) };
    // Check the whole project with every renamed file in place.
    let mut sources = Vec::new();
    let mut paths = clarity_spec::spec_files(&dir).map_err(read_err)?;
    paths.extend(clarity_spec::cxaml::page_files(&dir, &paths.clone(), &[]).map_err(read_err)?);
    for path in paths {
        let shown = path.display().to_string();
        // A moved file (a renamed window's) is read under its new path only.
        if renamed.iter().any(|r| r.moved_from.as_deref().is_some_and(|was| at(was) == path)) {
            continue;
        }
        let text = match renamed.iter().find(|r| at(&r.path) == path) {
            Some(r) => r.text.clone(),
            None => std::fs::read_to_string(&path).map_err(read_err)?,
        };
        sources.push((shown, text));
    }
    for r in renamed.iter().filter(|r| r.moved_from.is_some()) {
        sources.push((at(&r.path).display().to_string(), r.text.clone()));
    }
    let after = clarity_spec::load_sources(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    if let Some(error) = clarity_spec::new_error(before, &after) {
        return Err(error.message.clone());
    }
    let mut changes = Vec::new();
    for r in renamed {
        let file = at(&r.path);
        if r.moved_from.is_some() && file.exists() {
            return Err(format!("{} is there already", file.display()));
        }
        let old = if r.moved_from.is_some() { None } else { Some(std::fs::read_to_string(&file).map_err(read_err)?) };
        write_atomically(&file, &r.text)?;
        changes.push(FileChange {
            file,
            before: old,
            after: Some(r.text),
        });
        // A window's file takes its new name: the old one goes, kept for Undo.
        if let Some(was) = r.moved_from {
            let was = at(&was);
            let old = std::fs::read_to_string(&was).map_err(read_err)?;
            std::fs::remove_file(&was).map_err(read_err)?;
            changes.push(FileChange { file: was, before: Some(old), after: None });
        }
    }
    record(history, changes)?;
    load(dir)
}

/// Makes a new module by hand: its file `name.clarity` beside the others,
/// with its `module` line, as one change for Undo.
#[tauri::command]
fn add_module(name: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let dir = open_dir_of(&state)?;
    let (file, text) = clarity_project::new_module_file(&dir, name.trim())?;
    let change = FileChange { file, before: None, after: Some(text) };
    change.make()?;
    record(&history, vec![change])?;
    load(&dir)
}

/// The folder with the open project's modules.
fn open_dir_of(state: &WatchState) -> Result<PathBuf, String> {
    open_project_of(state).map(|(_, dir)| dir)
}

/// The open project's file.
fn project_file_of(state: &WatchState) -> Result<PathBuf, String> {
    open_project_of(state).map(|(file, _)| file)
}

fn open_project_of(state: &WatchState) -> Result<(PathBuf, PathBuf), String> {
    state
        .open
        .held()
        .map_err(|_| "watch state is poisoned")?
        .as_ref()
        .map(|o| (o.file.clone(), o.dir.clone()))
        .ok_or_else(|| "no project is open".to_string())
}

/// The server the IDE's preview and design canvas draw from (DESIGN 8.1,
/// `clarity_gen::preview`): Clarity.UI's browser files, the open project's
/// windows and a program answering with sample data. None when it could not start.
struct PreviewServer(Option<clarity_gen::preview::Server>);

fn preview_server(app: AppHandle) -> PreviewServer {
    let loader: clarity_gen::preview::Loader = std::sync::Arc::new(move || {
        let (file, dir) = open_project_of(&app.state::<WatchState>())?;
        clarity_gen::preview::load(&file, &dir)
    });
    let repository = cfg!(debug_assertions).then(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")));
    let library: clarity_gen::preview::Library = std::sync::Arc::new(move || clarity_gen::web_browser::local_folder(repository.as_deref()));
    PreviewServer(clarity_gen::preview::Server::start(loader, library, &clarity_gen::preview::secret()).ok())
}

/// Where the preview server serves, ending in `/`.
/// Clarity's repository, for a Clarity run from its source; none in an installed one.
fn repository() -> Option<PathBuf> {
    cfg!(debug_assertions).then(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")))
}

/// How the browser files the design and preview draw with stand, and whether
/// this Clarity runs from its repository (and so can build them again).
#[derive(Serialize)]
struct BrowserFilesState {
    state: clarity_gen::clarity_ui::BrowserFiles,
    repository: bool,
}

/// How Clarity.UI's browser files stand: fine, missing or (in a Clarity run
/// from its repository) older than Clarity.UI's source. Asked when the IDE
/// starts and whenever a canvas cannot load them.
#[tauri::command]
fn browser_files() -> BrowserFilesState {
    let repo = repository();
    BrowserFilesState { state: clarity_gen::clarity_ui::browser_files(repo.as_deref()), repository: repo.is_some() }
}

/// Builds Clarity.UI's browser files again from the repository's source, in the Output panel.
#[tauri::command]
fn rebuild_browser_files(app: AppHandle) -> Result<(), String> {
    let repo = repository().ok_or("only a Clarity run from its repository builds Clarity.UI's browser files")?;
    let steps = vec![vec!["bash".to_string(), "runtimes/rust-ui-browser/build.sh".to_string()]];
    build::start_part(app, "Clarity.UI's browser files".to_string(), repo, steps, vec![("CARGO_BUILD_JOBS".to_string(), "2".to_string())])
}

#[tauri::command]
fn preview_server_address(server: State<PreviewServer>) -> Result<String, String> {
    server.0.as_ref().map(|s| s.address().to_string()).ok_or_else(|| "The preview's server could not start on this machine's loopback address.".to_string())
}

/// Which way a change is replayed.
#[derive(Clone, Copy)]
enum Replay {
    Undo,
    Redo,
}

/// Takes back the last edit, or applies the last undone edit again. A file
/// that changed since (in another editor) is left alone.
fn replay(state: &WatchState, history: &HistoryState, way: Replay) -> Result<Preview, String> {
    let dir = open_dir_of(state)?;
    let mut history = history.0.held().map_err(|_| "history is poisoned")?;
    let (from, to) = match way {
        Replay::Undo => (&mut history.undo, Replay::Redo),
        Replay::Redo => (&mut history.redo, Replay::Undo),
    };
    let change = from.pop().ok_or("nothing to undo or redo")?;
    let pick = |c: &FileChange| match way {
        Replay::Undo => (c.after.clone(), c.before.clone()),
        Replay::Redo => (c.before.clone(), c.after.clone()),
    };
    // Check every file first, so a change is replayed whole or not at all.
    for c in &change.0 {
        let current = std::fs::read_to_string(&c.file).ok();
        if current != pick(c).0 {
            let name = c.file.file_name().unwrap_or_default().to_string_lossy().to_string();
            from.push(change);
            return Err(format!("{name} changed outside the designer; it cannot be undone here"));
        }
    }
    for c in &change.0 {
        match pick(c).1 {
            Some(text) => write_atomically(&c.file, &text)?,
            None => std::fs::remove_file(&c.file)
                .map_err(|err| format!("cannot remove {}: {err}", c.file.display()))?,
        }
    }
    match to {
        Replay::Redo => history.redo.push(change),
        Replay::Undo => history.undo.push(change),
    }
    load(&dir)
}

#[tauri::command]
fn undo_edit(state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    replay(&state, &history, Replay::Undo)
}

#[tauri::command]
fn redo_edit(state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    replay(&state, &history, Replay::Redo)
}

/// How many edits can be undone and redone, for the ribbon buttons.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct HistoryCounts {
    pub undo: u32,
    pub redo: u32,
}

#[tauri::command]
fn history_counts(history: State<HistoryState>) -> Result<HistoryCounts, String> {
    let history = history.0.held().map_err(|_| "history is poisoned")?;
    Ok(HistoryCounts {
        undo: history.undo.len() as u32,
        redo: history.redo.len() as u32,
    })
}

/// The open project's file, for the Project tab.
#[tauri::command]
fn project_file(state: State<WatchState>) -> Result<ProjectFile, String> {
    Ok(clarity_project::load(&project_file_of(&state)?))
}

/// The folder with the open project's modules, where file dialogs start.
#[tauri::command]
fn project_folder(state: State<WatchState>) -> Result<String, String> {
    Ok(open_dir_of(&state)?.display().to_string())
}

/// A path picked in a file dialog as the project file stores it: relative
/// to the project's folder when it is inside it.
#[tauri::command]
fn project_path(picked: String, state: State<WatchState>) -> Result<String, String> {
    Ok(clarity_project::project_path(&project_file_of(&state)?, Path::new(&picked)))
}

/// What a project can choose: backends, UIs, targets and data sources.
#[tauri::command]
fn stack_catalog() -> Catalog {
    clarity_project::catalog()
}

/// The build tools of every stack on this machine. Looking runs the tools,
/// so it happens off the window's thread.
#[tauri::command]
async fn stack_tools() -> Result<Vec<build::StackTools>, String> {
    tauri::async_runtime::spawn_blocking(build::stack_tools).await.map_err(|err| err.to_string())
}

/// Saves the project settings. `version` is the file version the Project
/// tab saw; settings with problems are refused, with the first one.
#[tauri::command]
fn save_settings(
    version: String,
    settings: Settings,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<ProjectFile, String> {
    let file = project_file_of(&state)?;
    let change = settings_change(&file, &version, &settings)?;
    if let Some(change) = change {
        change.make()?;
        record(&history, vec![change])?;
    }
    Ok(clarity_project::load(&file))
}

/// The change that saving `settings` makes to the project file `file`, or
/// None when the file already says the same.
fn settings_change(file: &Path, version: &str, settings: &Settings) -> Result<Option<FileChange>, String> {
    if let Some(problem) = clarity_project::validate(settings).into_iter().next() {
        return Err(problem);
    }
    let shown = file.file_name().unwrap_or_default().to_string_lossy().to_string();
    let before = std::fs::read_to_string(file).map_err(|err| format!("cannot read {shown}: {err}"))?;
    if clarity_spec::content_version(&before) != version {
        return Err(format!("{shown} changed on disk since it was loaded; the view has been refreshed, try again"));
    }
    let after = clarity_project::write(Some(&before), settings)?;
    if before == after {
        return Ok(None);
    }
    Ok(Some(FileChange {
        file: file.to_path_buf(),
        before: Some(before),
        after: Some(after),
    }))
}

/// Creates a project with the chosen stack: the project file at `path`
/// (like `shop.clarity`) and the folder next to it with a first module, an
/// app and its main window. Then opens it.
#[tauri::command]
fn new_project(
    path: String,
    stack: Stack,
    app: AppHandle,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<Preview, String> {
    let file = write_new_project(Path::new(&path), stack)?;
    open_file(&file.display().to_string(), app, &state, &history)
}

/// The project file given on the command line or in CLARITY_PROJECT.
#[tauri::command]
fn initial_project() -> Option<String> {
    let dir = std::env::args()
        .nth(1)
        .filter(|arg| !arg.starts_with('-'))
        .or_else(|| std::env::var("CLARITY_PROJECT").ok())?;
    let start = std::env::var_os("INIT_CWD").map(PathBuf::from);
    Some(resolve_dir(&dir, start.as_deref()))
}

/// A relative path is taken from where the command was typed: npm
/// passes that directory in INIT_CWD, while the app itself runs elsewhere.
fn resolve_dir(dir: &str, start: Option<&Path>) -> String {
    let path = Path::new(dir);
    match start {
        Some(start) if path.is_relative() => start.join(path).display().to_string(),
        _ => dir.to_string(),
    }
}

/// Clarity's window title: its name and version. A release says the version
/// the release pipeline stamped into tauri.conf.json from its tag; a
/// development build (`dev_release`, the newest `v*` tag it builds on, from
/// build.rs) says that release with `-dev`, since tauri.conf.json holds only
/// a placeholder in the repository.
fn window_title(name: &str, version: &str, dev_release: Option<&str>) -> String {
    match dev_release.map(|t| t.trim().trim_start_matches('v')) {
        None => format!("{name} {version}"),
        Some("") => format!("{name} (development)"),
        Some(release) => format!("{name} {release}-dev"),
    }
}

pub fn run() {
    // First, before any thread: nothing the IDE starts sees its own build folder on the library path.
    own_build::leave_out_own_build();
    tauri::Builder::default()
        .setup(|app| {
            // The splash screen first, so something shows while the rest starts.
            let info = app.package_info();
            let dev_release = cfg!(debug_assertions).then_some(env!("CLARITY_DEV_RELEASE"));
            let title = window_title(&info.name, &info.version.to_string(), dev_release);
            // The version alone, as the title says it after the name (which may be more than one word).
            let version = title.strip_prefix(info.name.as_str()).unwrap_or(&title).trim().to_string();
            // Pieces ticked in Clarity settings > Rebuild (a Clarity run from its repository) build first,
            // the splash showing the build line for line; else the usual splash.
            app.manage(dev_rebuild::Building::default());
            if !dev_rebuild::at_start(app.handle(), &title, &version) {
                splash::open(app.handle(), &title, &version);
            }
            // The log first, when logging is on, so it holds the rest of the start.
            logging::begin();
            // Fail safe: a panic in a command shows in the IDE.
            fail_safe::report_panics(app.handle().clone());
            // The status bar's meters, and the System page while it is open.
            system::start(app.handle().clone());
            // Clarity.UI's library for Go, C and C++ builds (DESIGN 7.9): the folder the
            // installer carries; in a debug build, the one the repository built.
            let os = std::env::consts::OS;
            let carried = app.path().resource_dir().ok().map(|dir| dir.join("clarity-ui")).filter(|dir| clarity_gen::clarity_ui::complete(dir, os));
            let repository = || cfg!(debug_assertions).then(|| clarity_gen::clarity_ui::repository_library(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")), os)).flatten();
            if let Some(folder) = carried.or_else(repository) {
                clarity_gen::clarity_ui::set_bundled(folder);
            }
            // The folders chosen in Clarity settings > Libraries, which every build and the preview use first.
            clarity_gen::tools::environment::use_libraries(&clarity_gen::tools::environment::Environment::current());
            // Its browser files for web programs (round B): carried by the installer; in a debug build, the repository's.
            // Named whether or not they are there now: each use checks the folder again, so files deleted
            // or built again while the IDE runs count (browser_files).
            let carried = app.path().resource_dir().ok().map(|dir| dir.join("clarity-ui-browser"));
            let complete = carried.as_deref().is_some_and(clarity_gen::clarity_ui::browser_complete);
            let folder = match repository() {
                Some(repo) if !complete => Some(clarity_gen::clarity_ui::repository_browser_folder(&repo)),
                _ => carried,
            };
            if let Some(folder) = folder {
                clarity_gen::clarity_ui::set_bundled_browser(folder);
            }
            // The preview and the design canvas draw with the same browser files (DESIGN 8.1).
            splash::status(app.handle(), "Starting the preview");
            app.manage(preview_server(app.handle().clone()));
            splash::status(app.handle(), "Loading the designer");
            if let Some(window) = app.get_webview_window("main") {
                // F5, Ctrl+R, Ctrl+P and Ctrl+F are Clarity's, not the webview's.
                browser_keys::turn_off(&window);
                // Hidden until its page has drawn itself (main.ts); see show_later. The splash goes when it shows, or with it then.
                dev_rebuild::show_later(app.handle().clone(), window.clone());
                let later = app.handle().clone();
                std::thread::spawn(move || {
                    while dev_rebuild::building(&later) {
                        std::thread::sleep(std::time::Duration::from_millis(200));
                    }
                    std::thread::sleep(std::time::Duration::from_secs(4));
                    splash::done(&later);
                });
                window.set_title(&title)?;
                // The manual's window closes with Clarity's; its own close only hides it.
                // Its project closes with it too, so `clarity mcp` no longer works on it.
                let app = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Destroyed = event {
                        if let Some(manual) = app.get_webview_window("manual") {
                            manual.destroy().ok();
                        }
                        if let Ok(file) = project_file_of(&app.state::<WatchState>())
                            && let Err(err) = clarity_project::ide::close_project(&file)
                        {
                            logging::line(&err);
                        }
                    }
                });
            }
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .manage(splash::SplashState::default())
        .manage(WatchState::default())
        .manage(HistoryState::default())
        .manage(build::BuildState::default())
        .manage(vault::VaultState::default())
        .manage(agent::AgentState::default())
        .manage(acp::AcpState::default())
        .invoke_handler(tauri::generate_handler![
            logging::logging_view,
            logging::set_logging,
            logging::open_log_folder,
            logging::log_line,
            preview_server_address,
            browser_files,
            rebuild_browser_files,
            dev_rebuild::dev_rebuild,
            dev_rebuild::set_dev_rebuild,
            dev_rebuild::dev_restart,
            dev_rebuild::main_drawn,
            splash::splash_done,
            splash::splash_status,
            manual::manual_topics,
            manual::manual_page,
            manual::open_manual,
            open_project,
            check_project,
            initial_project,
            apply_edit,
            undo_edit,
            redo_edit,
            history_counts,
            new_project,
            project_file,
            save_settings,
            stack_catalog,
            stack_tools,
            project_folder,
            project_path,
            code_text,
            check_code,
            decl_text,
            try_decl,
            highlight,
            build_app,
            generate_app,
            type_line,
            stop_app,
            app_state,
            build_snapshot,
            engines::engine_info,
            engines::engine_source_text,
            parts::part_files,
            parts::part_file_text,
            parts::save_part_file,
            parts::new_part_file,
            parts::rename_part_file,
            parts::delete_part_file,
            parts::build_part,
            environment::environment_view,
            environment::choose_tool,
            libraries::libraries_view,
            libraries::choose_library,
            libraries::reset_libraries,
            folders::folders_view,
            folders::set_folder,
            folders::new_project_file,
            folders::new_project_path,
            system::machine_facts,
            system::watch_system,
            system::disk_space,
            parts::write_part_folders,
            engines::save_engine_source,
            engines::new_engine_source,
            engines::add_engine_stubs,
            engines::build_engine,
            rename,
            rename_field,
            add_module,
            migrations::pending_migration,
            migrations::create_migration,
            backups::list_backups,
            backups::take_backup,
            backups::restore_backup,
            backups::delete_backup,
            external::source_tables,
            apis::api_listing,
            apis::add_api_file,
            external::import_tables,
            external::take_ownership,
            mappings::list_mappings,
            mappings::read_mapping,
            mappings::save_mapping,
            mappings::preview_mapping,
            mappings::mapping_uses,
            mappings::delete_mapping,
            translations::translation_table,
            translations::set_translation,
            translations::set_translation_resource,
            translations::rename_key,
            translations::make_texts_keys,
            move_pages_out,
            translations::add_language,
            themes::theme_styles,
            themes::theme_starter,
            resource_urls::project_resource,
            vault::vault_info,
            vault::vault_create,
            vault::vault_unlock,
            vault::vault_lock,
            vault::vault_change_password,
            vault::vault_destroy,
            vault::vault_secrets,
            vault::vault_value,
            vault::vault_save_secret,
            vault::vault_delete_secret,
            vault::vault_categories,
            vault::vault_save_categories,
            vault::connections_list,
            vault::connection_save,
            vault::connection_remove,
            agent::agent_send,
            acp::outside_agents,
            acp::set_outside_agents,
            acp::agent_activity,
            acp::agent_sign_in,
            acp::agent_set_choice,
            browse::browse_list,
            browse::browse_places,
            browse::browse_remember,
            browse::recent_projects,
            agent::agent_stop,
            agent::agent_answer,
            agent::agent_forget,
            links::open_link,
            resources::resource_kinds,
            resources::add_resource,
            resources::replace_resource_file,
            resources::save_markdown,
            resources::save_readme,
            resources::guidance_sections,
            resources::save_guidance_section,
            diagrams::diagram_kinds,
            diagrams::check_diagram,
            diagrams::read_erd,
            diagrams::write_erd,
            diagrams::add_diagram,
            diagrams::save_diagram,
            diagrams::generate_diagram,
            diagrams::rename_diagram,
            diagrams::delete_diagram,
            diagrams::read_classes,
            diagrams::write_classes,
            diagrams::diagram_proposals,
            diagrams::apply_diagram
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Clarity");
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, CreateKind, ModifyKind, RenameMode};
    use notify::{Event, EventKind};
    use clarity_project::{Backend, Target, Ui};

    #[test]
    fn the_window_title_carries_the_version() {
        // A release says the version its tag stamped in.
        assert_eq!(window_title("Clarity", "0.3.0", None), "Clarity 0.3.0");
        // A development build says the newest release it builds on, marked as such.
        assert_eq!(window_title("Clarity", "0.1.0", Some("v0.3.0")), "Clarity 0.3.0-dev");
        // Without a release tag to go by, it says it is a development build.
        assert_eq!(window_title("Clarity", "0.1.0", Some("")), "Clarity (development)");
    }

    fn csharp() -> Stack {
        Stack {
            backend: Backend::Csharp,
            ui: Ui::Clarity,
            targets: vec![Target::Desktop],
        }
    }

    #[test]
    fn opening_needs_a_project_file_with_its_folder() {
        let base = std::env::temp_dir().join(format!("clarity-open-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let file = write_new_project(&base.join("shop.clarity"), csharp()).unwrap();
        let (found, dir) = project_paths(&file.display().to_string()).unwrap();
        // The canonical path, without the prefix Windows answers with.
        let canonical = |path: &Path| clarity_project::plain(path.canonicalize().unwrap());
        assert_eq!((found, dir), (canonical(&file), canonical(&base).join("shop")));
        assert!(project_paths(&base.join("shop").display().to_string()).unwrap_err().contains("not a project file"));
        std::fs::remove_dir_all(base.join("shop")).unwrap();
        assert!(project_paths(&file.display().to_string()).unwrap_err().contains("is missing"));
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn saving_settings_checks_them_and_the_file_version() {
        let base = std::env::temp_dir().join(format!("clarity-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let file = write_new_project(&base.join("shop.clarity"), clarity_project::starter("Shop", "shop").stack).unwrap();
        let mut settings = clarity_project::load(&file).settings.unwrap();
        let version = clarity_project::load(&file).version;

        assert!(settings_change(&file, &version, &settings).unwrap().is_none(), "nothing changed");
        assert!(settings_change(&file, "stale", &settings).unwrap_err().contains("changed on disk"));

        // Every language draws with Clarity.UI only: the shared web screen is refused.
        settings.stack.backend = Backend::Go;
        settings.stack.ui = Ui::Webview;
        assert!(settings_change(&file, &version, &settings).unwrap_err().contains("Go draws its windows with Clarity.UI"));

        settings.stack = csharp();
        let change = settings_change(&file, &version, &settings).unwrap().unwrap();
        let after = change.after.as_deref().unwrap_or_default();
        assert!(after.contains("    stack csharp desktop\n"), "{after}");
        assert!(change.before.is_some());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn relative_paths_start_where_the_command_was_typed() {
        let start = Path::new("/home/me/project");
        // The system's own separator joins the two, so the answer is joined as well.
        assert_eq!(resolve_dir("examples/shop.clarity", Some(start)), start.join("examples/shop.clarity").display().to_string());
        assert_eq!(resolve_dir("/tmp/shop.clarity", Some(start)), "/tmp/shop.clarity");
        assert_eq!(resolve_dir("shop.clarity", None), "shop.clarity");
    }

    fn event(kind: EventKind, path: &str) -> Event {
        Event::new(kind).add_path(PathBuf::from(path))
    }

    const FILE: &str = "/p/shop.clarity";
    const DIR: &str = "/p/shop";

    fn changes(kind: EventKind, path: &str) -> bool {
        changes_project(&event(kind, path), Path::new(FILE), Path::new(DIR))
    }

    #[test]
    fn reacts_to_changes_of_modules_and_the_project_file() {
        assert!(changes(EventKind::Modify(ModifyKind::Any), "/p/shop/a.clarity"));
        assert!(changes(EventKind::Create(CreateKind::File), "/p/shop/a.clarity"));
        assert!(changes(EventKind::Modify(ModifyKind::Name(RenameMode::To)), "/p/shop/a.clarity"));
        assert!(changes(EventKind::Modify(ModifyKind::Name(RenameMode::To)), FILE));
        // Migration steps belong to the project too.
        assert!(changes(EventKind::Create(CreateKind::File), "/p/shop/migrations/0002.clarity"));
    }

    /// A resource added, changed or removed, and the resources folder itself.
    #[test]
    fn reacts_to_changes_of_the_resources() {
        assert!(changes(EventKind::Create(CreateKind::File), "/p/shop/resources/logo.png"));
        assert!(changes(EventKind::Modify(ModifyKind::Any), "/p/shop/resources/rules.nl.md"));
        assert!(changes(EventKind::Remove(notify::event::RemoveKind::File), "/p/shop/resources/logo.png"));
        assert!(changes(EventKind::Create(CreateKind::Folder), "/p/shop/resources"));
        assert!(!changes(EventKind::Create(CreateKind::File), "/p/shop/notes.txt"));
    }

    /// An engine's file added, rebuilt or removed, and the engines folder itself.
    #[test]
    fn reacts_to_changes_of_the_engines() {
        assert!(changes(EventKind::Create(CreateKind::File), "/p/shop/engines/pricing.wasm"));
        assert!(changes(EventKind::Modify(ModifyKind::Any), "/p/shop/engines/pricing.wasm"));
        assert!(changes(EventKind::Remove(notify::event::RemoveKind::File), "/p/shop/engines/pricing.wasm"));
        assert!(changes(EventKind::Create(CreateKind::Folder), "/p/shop/engines"));
    }

    /// Every folder of a project Clarity reads counts, at any depth: the
    /// translations, the windows' own files, the guidance for agents (a
    /// translation added after a window once went unseen until the project opened again).
    #[test]
    fn reacts_to_changes_in_every_folder_of_the_project() {
        for path in ["/p/shop/translations/en.clarity", "/p/shop/design/Main.cxaml", "/p/shop/design/orders/Card.cxaml", "/p/shop/agent/guidance.md", "/p/shop/migrations/main/0002.clarity", "/p/shop/diagrams/model.mermaid", "/p/shop/apis/stock.json"] {
            assert!(changes(EventKind::Modify(ModifyKind::Any), path), "{path}");
        }
        for folder in clarity_spec::PROJECT_FOLDERS {
            assert!(changes(EventKind::Create(CreateKind::Folder), &format!("/p/shop/{folder}")), "{folder}");
        }
        // What the project writes is not the project.
        assert!(!changes(EventKind::Modify(ModifyKind::Any), "/p/shop/out/desktop/src/main.rs"));
        assert!(!changes(EventKind::Modify(ModifyKind::Any), "/p/shop/data/shop.db"));
    }

    #[test]
    fn ignores_reads_and_other_files() {
        assert!(!changes(EventKind::Access(AccessKind::Any), "/p/shop/a.clarity"));
        assert!(!changes(EventKind::Modify(ModifyKind::Any), "/p/shop/a.clarity.tmp"));
        assert!(!changes(EventKind::Modify(ModifyKind::Any), "/p/shop/notes.txt"));
        // Other projects next to this one, and generated files, are not this project.
        assert!(!changes(EventKind::Modify(ModifyKind::Any), "/p/other.clarity"));
        assert!(!changes(EventKind::Modify(ModifyKind::Any), "/p/shop/out/src/x.clarity"));
    }
}
