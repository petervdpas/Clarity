//! The Clarity app backend. It owns parsing, checking, sample data, edits
//! and their history; the frontend only renders the `Preview` it receives.

mod build;

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
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

/// Event sent to the frontend whenever a watched spec file changes.
const PREVIEW_UPDATED: &str = "preview-updated";

/// Event sent to the frontend whenever the project file changes.
const PROJECT_UPDATED: &str = "project-updated";

/// Editors often write a file in several steps; changes arriving within
/// this window are handled as one reload.
const DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Default)]
struct WatchState {
    /// The open spec directory and its watcher. Dropping the watcher stops
    /// watching and ends the reload thread.
    open: Mutex<Option<(PathBuf, notify::RecommendedWatcher)>>,
}

/// One file's text before and after a change. A file that did not exist
/// before has no text, and undoing its change removes it.
#[derive(Debug)]
struct FileChange {
    file: PathBuf,
    before: Option<String>,
    after: String,
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
    let project = clarity_spec::load_dir(dir)
        .map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    Ok(Preview::new(project))
}

fn is_spec_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == clarity_spec::SPEC_EXTENSION)
}

fn is_project_file(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == clarity_project::PROJECT_FILE)
}

/// True for events that change a spec file or the project file. Access
/// events are ignored: reloading reads the files, and reacting to those
/// reads would reload in an endless loop.
fn changes_project(event: &notify::Event) -> bool {
    use notify::EventKind;
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) && event.paths.iter().any(|p| is_spec_file(p) || is_project_file(p))
}

fn watch(dir: &Path, app: AppHandle) -> Result<notify::RecommendedWatcher, String> {
    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event
            && changes_project(&event)
        {
            let _ = tx.send(());
        }
    })
    .map_err(|err| format!("cannot watch {}: {err}", dir.display()))?;
    watcher
        .watch(dir, RecursiveMode::NonRecursive)
        .map_err(|err| format!("cannot watch {}: {err}", dir.display()))?;

    let dir = dir.to_path_buf();
    std::thread::spawn(move || {
        // Ends when the watcher (and with it the sender) is dropped.
        while rx.recv().is_ok() {
            while rx.recv_timeout(DEBOUNCE).is_ok() {}
            if let Ok(preview) = load(&dir) {
                let _ = app.emit(PREVIEW_UPDATED, preview);
            }
            let _ = app.emit(PROJECT_UPDATED, clarity_project::load(&dir));
        }
    });

    Ok(watcher)
}

/// Loads a spec directory and starts watching it, replacing any previous watch.
#[tauri::command]
fn open_spec_dir(
    path: String,
    app: AppHandle,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<Preview, String> {
    open_dir(&path, app, &state, &history)
}

fn open_dir(path: &str, app: AppHandle, state: &WatchState, history: &HistoryState) -> Result<Preview, String> {
    let dir = PathBuf::from(path)
        .canonicalize()
        .map_err(|err| format!("cannot open {path}: {err}"))?;
    if !dir.is_dir() {
        return Err(format!("{} is not a directory", dir.display()));
    }
    let preview = load(&dir)?;
    let watcher = watch(&dir, app)?;
    *state.open.lock().map_err(|_| "watch state is poisoned")? = Some((dir, watcher));
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
    let before = clarity_spec::load_dir(&dir).map_err(read_err)?;
    let outcome = edit::apply_in(&before, &source, &edit).map_err(|err| err.to_string())?;
    if outcome.text != source {
        // The checker decides what is valid: an edit that adds an error
        // anywhere in the project (an overlap, a wrong binding) is refused.
        let after = clarity_spec::load_dir_with(&dir, &file, &outcome.text).map_err(read_err)?;
        if let Some(error) = clarity_spec::new_error(&before, &after) {
            return Err(error.message.clone());
        }
        write_atomically(&file, &outcome.text)?;
        record(&history, vec![FileChange {
            file: file.clone(),
            before: Some(source),
            after: outcome.text,
        }])?;
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
    let project = clarity_spec::load_dir(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
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
    let project = clarity_spec::load_dir_with(&dir, &file, &after)
        .map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    Ok(clarity_spec::code::edit::problems_in_code(&after, &owner, &file.display().to_string(), &project.diagnostics))
}

/// The text of the declaration at `item` (a window, the app, a function,
/// ...) in the spec file at `path`, with its problems.
#[tauri::command]
fn decl_text(path: String, item: u32, state: State<WatchState>) -> Result<CodeView, String> {
    let (dir, file) = spec_file_of(&state, &path)?;
    let source = std::fs::read_to_string(&file).map_err(|err| format!("cannot read {path}: {err}"))?;
    let project = clarity_spec::load_dir(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
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

#[tauri::command]
fn try_decl(path: String, item: u32, text: String, state: State<WatchState>) -> Result<DeclTrial, String> {
    let (dir, file) = spec_file_of(&state, &path)?;
    let source = std::fs::read_to_string(&file).map_err(|err| format!("cannot read {path}: {err}"))?;
    let after = clarity_spec::source::replace_decl(&source, item, &text).map_err(|err| err.to_string())?;
    let project = clarity_spec::load_dir_with(&dir, &file, &after)
        .map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    Ok(DeclTrial {
        problems: clarity_spec::source::problems_in_decl(&after, item, &file.display().to_string(), &project.diagnostics),
        preview: Preview::new(project),
    })
}

/// Generates the open project into `out/`, builds it and, with `run`,
/// starts it. Progress arrives as `build-line` and `build-done` events.
#[tauri::command]
fn build_app(release: bool, run: bool, app: AppHandle, state: State<WatchState>) -> Result<(), String> {
    build::start(app, open_dir_of(&state)?, release, run)
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
    let before = clarity_spec::load_dir(&dir).map_err(read_err)?;
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
            after: r.text,
        });
    }
    record(&history, changes)?;
    load(&dir)
}

fn open_dir_of(state: &WatchState) -> Result<PathBuf, String> {
    state
        .open
        .lock()
        .map_err(|_| "watch state is poisoned")?
        .as_ref()
        .map(|(dir, _)| dir.clone())
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
        Replay::Undo => (Some(c.after.clone()), c.before.clone()),
        Replay::Redo => (c.before.clone(), Some(c.after.clone())),
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

/// The project file of the open project, for the Project tab.
#[tauri::command]
fn project_file(state: State<WatchState>) -> Result<ProjectFile, String> {
    Ok(clarity_project::load(&open_dir_of(&state)?))
}

/// What a project can choose: backends, UIs, targets and data sources.
#[tauri::command]
fn stack_catalog() -> Catalog {
    clarity_project::catalog()
}

/// Writes the starter project file into the open project when it has
/// none, named after the folder like a new project.
#[tauri::command]
fn create_project_file(state: State<WatchState>, history: State<HistoryState>) -> Result<ProjectFile, String> {
    let dir = open_dir_of(&state)?;
    let change = starter_change(&dir)?;
    write_atomically(&change.file, &change.after)?;
    record(&history, vec![change])?;
    Ok(clarity_project::load(&dir))
}

fn starter_change(dir: &Path) -> Result<FileChange, String> {
    let file = dir.join(clarity_project::PROJECT_FILE);
    if file.exists() {
        return Err(format!("the project already has a {}", clarity_project::PROJECT_FILE));
    }
    let (module, app_name) = project_names(&folder_name(dir));
    let settings = clarity_project::starter(&clarity_spec::model::default_label(&app_name), &module);
    Ok(FileChange {
        file,
        before: None,
        after: clarity_project::write(None, &settings)?,
    })
}

fn folder_name(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "app".to_string())
}

/// Saves the project settings. `version` is the file version the Project
/// tab saw (empty when there was no file); settings with problems are
/// refused, with the first one.
#[tauri::command]
fn save_settings(
    version: String,
    settings: Settings,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<ProjectFile, String> {
    let dir = open_dir_of(&state)?;
    let change = settings_change(&dir, &version, &settings)?;
    if let Some(change) = change {
        write_atomically(&change.file, &change.after)?;
        record(&history, vec![change])?;
    }
    Ok(clarity_project::load(&dir))
}

/// The change that saving `settings` makes to the project file, or None
/// when the file already says the same.
fn settings_change(dir: &Path, version: &str, settings: &Settings) -> Result<Option<FileChange>, String> {
    if let Some(problem) = clarity_project::validate(settings).into_iter().next() {
        return Err(problem);
    }
    let file = dir.join(clarity_project::PROJECT_FILE);
    let before = match std::fs::read_to_string(&file) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(format!("cannot read {}: {err}", file.display())),
    };
    let current = before.as_deref().map(clarity_spec::content_version).unwrap_or_default();
    if current != version {
        return Err(format!(
            "{} changed on disk since it was loaded; the view has been refreshed, try again",
            clarity_project::PROJECT_FILE
        ));
    }
    let after = clarity_project::write(before.as_deref(), settings)?;
    if before.as_deref() == Some(after.as_str()) {
        return Ok(None);
    }
    Ok(Some(FileChange {
        file,
        before,
        after,
    }))
}

/// Creates a project in a new or empty folder with the chosen stack: the
/// project file and one spec file with an app and its main window. Then
/// opens it.
#[tauri::command]
fn new_project(
    path: String,
    stack: Stack,
    app: AppHandle,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<Preview, String> {
    write_new_project(Path::new(&path), stack)?;
    open_dir(&path, app, &state, &history)
}

/// What a new project keeps out of version control: the generated application.
const GITIGNORE: &str = "# Generated by Clarity, and the build tools' output; rebuilt from the .clarity files.\nout/\n";

/// Writes the files of a new project into `dir`, which must not hold a
/// project yet.
fn write_new_project(dir: &Path, stack: Stack) -> Result<PathBuf, String> {
    let shown = dir.display();
    std::fs::create_dir_all(dir).map_err(|err| format!("cannot create {shown}: {err}"))?;
    let specs = clarity_spec::spec_files(dir).map_err(|err| format!("cannot read {shown}: {err}"))?;
    if !specs.is_empty() || dir.join(clarity_project::PROJECT_FILE).exists() {
        return Err(format!("{shown} already holds a Clarity project; open it instead"));
    }
    let (module, app_name) = project_names(&folder_name(dir));
    let title = clarity_spec::model::default_label(&app_name);
    let settings = Settings {
        stack,
        ..clarity_project::starter(&title, &module)
    };
    // Check before writing anything, so a refused stack leaves no files.
    if let Some(problem) = clarity_project::validate(&settings).into_iter().next() {
        return Err(problem);
    }
    let project = clarity_project::write(None, &settings)?;
    let text = format!(
        "module {module}\n\napp {app_name}\n    title \"{title}\"\n    main Main\n\npage Main: window\n    title \"Main\"\n"
    );
    let file = dir.join(format!("{module}.clarity"));
    std::fs::write(dir.join(clarity_project::PROJECT_FILE), project)
        .map_err(|err| format!("cannot write the project: {err}"))?;
    std::fs::write(&file, text).map_err(|err| format!("cannot write the project: {err}"))?;
    std::fs::write(dir.join(".gitignore"), GITIGNORE).map_err(|err| format!("cannot write the project: {err}"))?;
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

/// The spec directory given on the command line or in CLARITY_SPEC_DIR.
#[tauri::command]
fn initial_spec_dir() -> Option<String> {
    let dir = std::env::args()
        .nth(1)
        .filter(|arg| !arg.starts_with('-'))
        .or_else(|| std::env::var("CLARITY_SPEC_DIR").ok())?;
    let start = std::env::var_os("INIT_CWD").map(PathBuf::from);
    Some(resolve_dir(&dir, start.as_deref()))
}

/// A relative directory is taken from where the command was typed: npm
/// passes that directory in INIT_CWD, while the app itself runs elsewhere.
fn resolve_dir(dir: &str, start: Option<&Path>) -> String {
    let path = Path::new(dir);
    match start {
        Some(start) if path.is_relative() => start.join(path).display().to_string(),
        _ => dir.to_string(),
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(WatchState::default())
        .manage(HistoryState::default())
        .manage(build::BuildState::default())
        .invoke_handler(tauri::generate_handler![
            open_spec_dir,
            initial_spec_dir,
            apply_edit,
            undo_edit,
            redo_edit,
            history_counts,
            new_project,
            project_file,
            save_settings,
            stack_catalog,
            create_project_file,
            code_text,
            check_code,
            decl_text,
            try_decl,
            build_app,
            stop_app,
            rename
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
    fn a_new_project_checks_clean_and_is_not_written_twice() {
        let dir = std::env::temp_dir().join(format!("clarity-new-{}", std::process::id())).join("my-shop");
        let _ = std::fs::remove_dir_all(&dir);
        let file = write_new_project(&dir, csharp()).unwrap();
        assert_eq!(file.file_name().unwrap(), "my_shop.clarity");
        let project = clarity_spec::load_dir(&dir).unwrap();
        assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
        assert_eq!(project.apps().next().unwrap().name.name, "MyShop");
        let settings = clarity_project::load(&dir);
        assert!(settings.problems.is_empty(), "{:?}", settings.problems);
        let settings = settings.settings.unwrap();
        assert_eq!(settings.name, "My shop");
        assert_eq!(settings.stack, csharp());
        assert_eq!(settings.datasources[0].url, "data/my_shop.db");
        assert!(std::fs::read_to_string(dir.join(".gitignore")).unwrap().contains("out/"));
        assert!(write_new_project(&dir, csharp()).unwrap_err().contains("already holds"));
        std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
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
        let dir = std::env::temp_dir().join(format!("clarity-refused-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let stack = Stack {
            backend: Backend::Go,
            ..csharp()
        };
        assert!(write_new_project(&dir, stack).unwrap_err().contains("Avalonia"));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn saving_settings_checks_them_and_the_file_version() {
        let dir = std::env::temp_dir().join(format!("clarity-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut settings = clarity_project::starter("Shop", "shop");

        // No file yet: the version is empty, and saving creates the file.
        let change = settings_change(&dir, "", &settings).unwrap().unwrap();
        assert_eq!(change.before, None);
        std::fs::write(&change.file, &change.after).unwrap();
        let version = clarity_project::load(&dir).version;

        assert!(settings_change(&dir, &version, &settings).unwrap().is_none(), "nothing changed");
        assert!(settings_change(&dir, "stale", &settings).unwrap_err().contains("changed on disk"));

        settings.stack.ui = Ui::Avalonia;
        assert!(settings_change(&dir, &version, &settings).unwrap_err().contains("C#"));

        settings.stack = csharp();
        let change = settings_change(&dir, &version, &settings).unwrap().unwrap();
        assert!(change.after.contains("backend = \"csharp\""));
        assert!(change.before.is_some());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn creates_the_starter_project_file_only_when_missing() {
        let dir = std::env::temp_dir().join(format!("clarity-starter-{}", std::process::id())).join("order-desk");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let change = starter_change(&dir).unwrap();
        let settings = clarity_project::parse(&change.after).unwrap();
        assert_eq!(settings.name, "Order desk");
        assert_eq!(settings.datasources[0].url, "data/order_desk.db");
        std::fs::write(&change.file, &change.after).unwrap();
        assert!(starter_change(&dir).unwrap_err().contains("already has"));
        std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
    }

    #[test]
    fn names_a_project_after_its_folder() {
        assert_eq!(project_names("my-shop"), ("my_shop".into(), "MyShop".into()));
        assert_eq!(project_names("Shop"), ("shop".into(), "Shop".into()));
        assert_eq!(project_names("2026 plan"), ("app_2026_plan".into(), "App2026Plan".into()));
        assert_eq!(project_names("--"), ("app".into(), "App".into()));
    }

    #[test]
    fn relative_spec_dirs_start_where_the_command_was_typed() {
        let start = Path::new("/home/me/project");
        assert_eq!(resolve_dir("examples/shop/spec", Some(start)), "/home/me/project/examples/shop/spec");
        assert_eq!(resolve_dir("/tmp/spec", Some(start)), "/tmp/spec");
        assert_eq!(resolve_dir("spec", None), "spec");
    }

    fn event(kind: EventKind, path: &str) -> Event {
        Event::new(kind).add_path(PathBuf::from(path))
    }

    #[test]
    fn reacts_to_changes_of_spec_files() {
        assert!(changes_project(&event(
            EventKind::Modify(ModifyKind::Any),
            "/s/a.clarity"
        )));
        assert!(changes_project(&event(
            EventKind::Create(CreateKind::File),
            "/s/a.clarity"
        )));
        assert!(changes_project(&event(
            EventKind::Modify(ModifyKind::Name(RenameMode::To)),
            "/s/a.clarity"
        )));
    }

    #[test]
    fn ignores_reads_and_other_files() {
        assert!(!changes_project(&event(
            EventKind::Access(AccessKind::Any),
            "/s/a.clarity"
        )));
        assert!(!changes_project(&event(
            EventKind::Modify(ModifyKind::Any),
            "/s/a.clarity.tmp"
        )));
        assert!(!changes_project(&event(
            EventKind::Modify(ModifyKind::Any),
            "/s/notes.txt"
        )));
    }
}
