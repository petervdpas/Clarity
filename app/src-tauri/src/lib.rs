//! The Clarity app backend. It owns parsing, checking, sample data, edits
//! and their history; the frontend only renders the `Preview` it receives.

mod build;
mod links;
mod diagrams;
mod resources;
mod engines;
mod parts;
mod external;
mod themes;
mod translations;
mod vault;
mod agent;
mod manual;
mod migrations;
mod pictures;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;

use clarity_project::{Catalog, ProjectFile, Settings, Stack};
use clarity_spec::Preview;
use clarity_spec::edit::{self, Edit};
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

/// One file's text before and after a change. A file that did not exist
/// before has no text, and undoing its change removes it.
#[derive(Debug)]
struct FileChange {
    file: PathBuf,
    /// The text before; None when the change made the file.
    before: Option<String>,
    /// The text after; None when the change took the file away.
    after: Option<String>,
}

impl FileChange {
    /// Makes the change: writes the text after, or takes the file away.
    fn make(&self) -> Result<(), String> {
        match &self.after {
            Some(text) => write_atomically(&self.file, text),
            None => std::fs::remove_file(&self.file).map_err(|err| format!("cannot remove {}: {err}", self.file.display())),
        }
    }
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

fn is_spec_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == clarity_spec::SPEC_EXTENSION)
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
        let folder = p.parent();
        let resources = dir.join(clarity_spec::resources::RESOURCES_DIR);
        let engines = dir.join(clarity_spec::engines::ENGINES_DIR);
        let diagrams = dir.join(clarity_spec::diagrams::DIAGRAMS_DIR);
        p == file
            || (is_spec_file(p) && (folder == Some(dir) || folder == Some(&dir.join(clarity_spec::MIGRATIONS_DIR))))
            || *p == resources
            || folder == Some(&resources)
            || *p == engines
            || folder == Some(&engines)
            || *p == diagrams
            || folder == Some(&diagrams)
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
    // The migration steps, the resources, the engines and the diagrams too, once the project has them.
    let steps = dir.join(clarity_spec::MIGRATIONS_DIR);
    let resources = dir.join(clarity_spec::resources::RESOURCES_DIR);
    let engines = dir.join(clarity_spec::engines::ENGINES_DIR);
    let diagrams = dir.join(clarity_spec::diagrams::DIAGRAMS_DIR);
    let mut folders = vec![dir, file.parent().unwrap_or(dir)];
    for extra in [&steps, &resources, &engines, &diagrams] {
        if extra.is_dir() {
            folders.push(extra);
        }
    }
    for folder in folders {
        watcher
            .watch(folder, RecursiveMode::NonRecursive)
            .map_err(|err| format!("cannot watch {}: {err}", folder.display()))?;
    }

    let (file, dir) = (file.to_path_buf(), dir.to_path_buf());
    std::thread::spawn(move || {
        // Ends when the watcher (and with it the sender) is dropped.
        while rx.recv().is_ok() {
            while rx.recv_timeout(DEBOUNCE).is_ok() {}
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
    let file = PathBuf::from(path)
        .canonicalize()
        .map_err(|err| format!("cannot open {path}: {err}"))?;
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
    let preview = load(&dir)?;
    let watcher = watch(&file, &dir, app)?;
    *state.open.lock().map_err(|_| "watch state is poisoned")? = Some(OpenProject {
        file,
        dir,
        watcher,
    });
    // Undo works within one project.
    *history.0.lock().map_err(|_| "history is poisoned")? = History::default();
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

    let read_err = |err: std::io::Error| format!("cannot read {}: {err}", dir.display());
    let before = clarity_project::load_modules(&dir).map_err(read_err)?;
    let outcome = edit::apply_in(&before, &source, &edit).map_err(|err| err.to_string())?;
    if outcome.text != source {
        // A text the edit wrote became a key: every language file gets it,
        // the default language with its text.
        let default = clarity_spec::translations::default_language(&before);
        let languages = clarity_spec::keyed::plan_keys(&dir, &default, &outcome.keys).map_err(read_err)?;
        let mut replaced: Vec<(&Path, &str)> = vec![(file.as_path(), outcome.text.as_str())];
        replaced.extend(languages.iter().map(|(path, _, after)| (path.as_path(), after.as_str())));
        // The checker decides what is valid: an edit that adds an error
        // anywhere in the project (an overlap, a wrong binding) is refused.
        let after = clarity_project::load_modules_with_files(&dir, &replaced).map_err(read_err)?;
        if let Some(error) = clarity_spec::new_error(&before, &after) {
            return Err(error.message.clone());
        }
        write_atomically(&file, &outcome.text)?;
        let mut changes = vec![FileChange {
            file: file.clone(),
            before: Some(source),
            after: Some(outcome.text),
        }];
        for (path, before_text, after_text) in languages {
            if let Some(folder) = path.parent() {
                std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
            }
            write_atomically(&path, &after_text)?;
            changes.push(FileChange { file: path, before: before_text, after: Some(after_text) });
        }
        record(&history, changes)?;
    }
    Ok(EditApplied {
        preview: load(&dir)?,
        focus: outcome.focus,
    })
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
fn highlight(text: String) -> Vec<clarity_spec::highlight::Piece> {
    clarity_spec::highlight::highlight(&text)
}

#[tauri::command]
fn try_decl(path: String, item: u32, text: String, state: State<WatchState>) -> Result<DeclTrial, String> {
    let (dir, file) = spec_file_of(&state, &path)?;
    let source = std::fs::read_to_string(&file).map_err(|err| format!("cannot read {path}: {err}"))?;
    let after = clarity_spec::source::replace_decl(&source, item, &text).map_err(|err| err.to_string())?;
    let project = clarity_project::load_modules_with(&dir, &file, &after)
        .map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
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

/// The open directory and the canonical path of one of its spec files.
fn spec_file_of(state: &WatchState, path: &str) -> Result<(PathBuf, PathBuf), String> {
    let dir = open_dir_of(state)?;
    let file = PathBuf::from(path)
        .canonicalize()
        .map_err(|err| format!("cannot open {path}: {err}"))?;
    if file.parent() != Some(dir.as_path()) || !is_spec_file(&file) {
        return Err(format!("{path} is not a spec file of the open directory"));
    }
    Ok((dir, file))
}

/// Remembers an applied change for Undo; a new change clears Redo.
fn record(history: &HistoryState, files: Vec<FileChange>) -> Result<(), String> {
    let mut history = history.0.lock().map_err(|_| "history is poisoned")?;
    history.redo.clear();
    history.undo.push(Change(files));
    if history.undo.len() > HISTORY_LIMIT {
        history.undo.remove(0);
    }
    Ok(())
}

/// Renames an entity or enum and every reference to it, in all files.
#[tauri::command]
fn rename(
    kind: RenameKind,
    name: String,
    new_name: String,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<Preview, String> {
    let dir = open_dir_of(&state)?;
    let read_err = |err: std::io::Error| format!("cannot read {}: {err}", dir.display());
    let before = clarity_project::load_modules(&dir).map_err(read_err)?;
    let source = |path: &str| std::fs::read_to_string(path).ok();
    let renamed = clarity_spec::rename::rename(&before, source, kind, &name, new_name.trim())?;
    if renamed.is_empty() {
        return load(&dir);
    }
    // Check the whole project with every renamed file in place.
    let mut sources = Vec::new();
    for path in clarity_spec::spec_files(&dir).map_err(read_err)? {
        let shown = path.display().to_string();
        let text = match renamed.iter().find(|r| r.path == shown) {
            Some(r) => r.text.clone(),
            None => std::fs::read_to_string(&path).map_err(read_err)?,
        };
        sources.push((shown, text));
    }
    let after = clarity_spec::load_sources(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    if let Some(error) = clarity_spec::new_error(&before, &after) {
        return Err(error.message.clone());
    }
    let mut changes = Vec::new();
    for r in renamed {
        let file = PathBuf::from(&r.path);
        let old = std::fs::read_to_string(&file).map_err(read_err)?;
        write_atomically(&file, &r.text)?;
        changes.push(FileChange {
            file,
            before: Some(old),
            after: Some(r.text),
        });
    }
    record(&history, changes)?;
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
        .lock()
        .map_err(|_| "watch state is poisoned")?
        .as_ref()
        .map(|o| (o.file.clone(), o.dir.clone()))
        .ok_or_else(|| "no project is open".to_string())
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
    let mut history = history.0.lock().map_err(|_| "history is poisoned")?;
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
    let history = history.0.lock().map_err(|_| "history is poisoned")?;
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

/// What a new project keeps out of version control: the generated application.
const GITIGNORE: &str = "# Generated by Clarity, and the build tools' output; rebuilt from the .clarity files.\nout/\n# The application's records while you develop it.\ndata/\n";

/// Writes a new project: the project file (with `.clarity` added when it is
/// missing) and its folder, which must not exist yet or be empty. Returns
/// the project file.
fn write_new_project(path: &Path, stack: Stack) -> Result<PathBuf, String> {
    let file = if is_spec_file(path) { path.to_path_buf() } else { path.with_extension(clarity_spec::SPEC_EXTENSION) };
    let dir = clarity_project::module_dir(&file);
    if file.exists() {
        return Err(format!("{} already exists; open it instead", file.display()));
    }
    let occupied = std::fs::read_dir(&dir).map(|mut entries| entries.next().is_some()).unwrap_or(false);
    if occupied {
        return Err(format!("the folder {} is not empty; choose another name", dir.display()));
    }
    let stem = file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let (module, app_name) = project_names(&stem);
    let settings = Settings {
        stack,
        ..clarity_project::starter(&app_name, &module)
    };
    // Check before writing anything, so a refused stack leaves no files.
    if let Some(problem) = clarity_project::validate(&settings).into_iter().next() {
        return Err(problem);
    }
    let project = clarity_project::write(None, &settings)?;
    let title = clarity_project::title(&settings);
    let module_text = format!(
        "module {module}\n\napp {app_name}\n    title \"{title}\"\n    main Main\n\npage Main: window\n    title \"Main\"\n"
    );
    // Its texts by key, with the default language's file giving their values.
    let language = settings.language.clone().unwrap_or_else(|| "en".to_string());
    let module_file = format!("{module}.clarity");
    let keyed = clarity_spec::keyed::make_keys(&[(module_file.as_str(), module_text.as_str())], &[], &language);
    let failed = |err: std::io::Error| format!("cannot write the project: {err}");
    std::fs::create_dir_all(dir.join(clarity_spec::translations::TRANSLATIONS_DIR)).map_err(failed)?;
    std::fs::write(dir.join(&module_file), &keyed.modules[0].1).map_err(failed)?;
    std::fs::write(dir.join(clarity_spec::translations::TRANSLATIONS_DIR).join(format!("{language}.clarity")), keyed.language_file(&language)).map_err(failed)?;
    std::fs::write(dir.join(".gitignore"), GITIGNORE).map_err(failed)?;
    std::fs::write(&file, project).map_err(failed)?;
    Ok(file)
}

/// A module name (`my_shop`) and an app name (`MyShop`) from a folder name.
fn project_names(folder: &str) -> (String, String) {
    let words: Vec<String> = folder
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    let words = if words.is_empty() { vec!["app".to_string()] } else { words };
    let mut module = words.join("_");
    if module.starts_with(|c: char| c.is_ascii_digit()) {
        module.insert_str(0, "app_");
    }
    let app = words
        .iter()
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map(|c| c.to_uppercase().chain(chars).collect::<String>()).unwrap_or_default()
        })
        .collect::<String>();
    let app = if app.starts_with(|c: char| c.is_ascii_digit()) { format!("App{app}") } else { app };
    (module, app)
}

/// Writes through a temporary file and a rename, so readers (including the
/// watcher and other editors) never see a half-written spec.
fn write_atomically(file: &Path, text: &str) -> Result<(), String> {
    let mut tmp = file.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, text).map_err(|err| format!("cannot write {}: {err}", tmp.display()))?;
    std::fs::rename(&tmp, file).map_err(|err| {
        let _ = std::fs::remove_file(&tmp);
        format!("cannot replace {}: {err}", file.display())
    })
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
    tauri::Builder::default()
        .setup(|app| {
            let info = app.package_info();
            if let Some(window) = app.get_webview_window("main") {
                let dev_release = cfg!(debug_assertions).then_some(env!("CLARITY_DEV_RELEASE"));
                window.set_title(&window_title(&info.name, &info.version.to_string(), dev_release))?;
                // The manual's window closes with Clarity's.
                let app = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Destroyed = event
                        && let Some(manual) = app.get_webview_window("manual")
                    {
                        manual.close().ok();
                    }
                });
            }
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .manage(WatchState::default())
        .manage(HistoryState::default())
        .manage(build::BuildState::default())
        .manage(vault::VaultState::default())
        .manage(agent::AgentState::default())
        .invoke_handler(tauri::generate_handler![
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
            type_line,
            stop_app,
            engines::engine_info,
            engines::engine_source_text,
            parts::part_files,
            parts::part_file_text,
            parts::save_part_file,
            parts::write_part_folders,
            engines::save_engine_source,
            engines::new_engine_source,
            engines::add_engine_stubs,
            engines::build_engine,
            rename,
            migrations::pending_migration,
            migrations::create_migration,
            external::source_tables,
            external::import_tables,
            external::take_ownership,
            translations::translation_table,
            translations::set_translation,
            translations::set_translation_resource,
            translations::rename_key,
            translations::make_texts_keys,
            translations::add_language,
            themes::theme_styles,
            themes::theme_starter,
            pictures::project_picture,
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
            agent::agent_stop,
            agent::agent_answer,
            agent::agent_forget,
            links::open_link,
            resources::resource_kinds,
            resources::add_resource,
            resources::replace_resource_file,
            resources::save_markdown,
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

    #[test]
    fn a_new_project_is_a_file_with_its_folder() {
        let base = std::env::temp_dir().join(format!("clarity-new-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        // The extension is added when the dialog leaves it out.
        let file = write_new_project(&base.join("my-shop"), csharp()).unwrap();
        assert_eq!(file, base.join("my-shop.clarity"));
        let dir = base.join("my-shop");
        assert!(dir.join("my_shop.clarity").exists());
        let project = clarity_project::load_modules(&dir).unwrap();
        assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
        assert_eq!(project.apps().next().unwrap().name.name, "MyShop");
        let loaded = clarity_project::load(&file);
        assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
        let settings = loaded.settings.unwrap();
        assert_eq!(settings.name, "MyShop");
        assert_eq!(clarity_project::title(&settings), "My shop");
        assert_eq!(settings.stack, csharp());
        assert_eq!(settings.datasources[0].url, "data/my_shop.db");
        assert!(std::fs::read_to_string(dir.join(".gitignore")).unwrap().contains("out/"));
        assert!(write_new_project(&file, csharp()).unwrap_err().contains("already exists"));

        // A folder that already holds something is not taken over.
        std::fs::create_dir_all(base.join("taken")).unwrap();
        std::fs::write(base.join("taken/notes.txt"), "mine").unwrap();
        assert!(write_new_project(&base.join("taken.clarity"), csharp()).unwrap_err().contains("not empty"));
        std::fs::remove_dir_all(&base).unwrap();
    }

    fn csharp() -> Stack {
        Stack {
            backend: Backend::Csharp,
            ui: Ui::Avalonia,
            targets: vec![Target::Desktop],
        }
    }

    #[test]
    fn a_refused_stack_writes_no_files() {
        let base = std::env::temp_dir().join(format!("clarity-refused-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let stack = Stack {
            backend: Backend::Go,
            ..csharp()
        };
        assert!(write_new_project(&base.join("shop.clarity"), stack).unwrap_err().contains("Avalonia"));
        assert_eq!(std::fs::read_dir(&base).unwrap().count(), 0);
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn opening_needs_a_project_file_with_its_folder() {
        let base = std::env::temp_dir().join(format!("clarity-open-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let file = write_new_project(&base.join("shop.clarity"), csharp()).unwrap();
        let (found, dir) = project_paths(&file.display().to_string()).unwrap();
        assert_eq!((found, dir), (file.canonicalize().unwrap(), base.canonicalize().unwrap().join("shop")));
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

        settings.stack.ui = Ui::Avalonia;
        assert!(settings_change(&file, &version, &settings).unwrap_err().contains("C#"));

        settings.stack = csharp();
        let change = settings_change(&file, &version, &settings).unwrap().unwrap();
        let after = change.after.as_deref().unwrap_or_default();
        assert!(after.contains("    stack csharp avalonia desktop\n"), "{after}");
        assert!(change.before.is_some());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn names_a_project_after_its_file() {
        assert_eq!(project_names("my-shop"), ("my_shop".into(), "MyShop".into()));
        assert_eq!(project_names("Shop"), ("shop".into(), "Shop".into()));
        assert_eq!(project_names("2026 plan"), ("app_2026_plan".into(), "App2026Plan".into()));
        assert_eq!(project_names("--"), ("app".into(), "App".into()));
    }

    #[test]
    fn relative_paths_start_where_the_command_was_typed() {
        let start = Path::new("/home/me/project");
        assert_eq!(resolve_dir("examples/shop.clarity", Some(start)), "/home/me/project/examples/shop.clarity");
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
