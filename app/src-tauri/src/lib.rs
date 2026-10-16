//! The Clarity app backend. It owns parsing, checking, sample data, edits
//! and their history; the frontend only renders the `Preview` it receives.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;

use clarity_spec::Preview;
use clarity_spec::edit::{self, Edit};
use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

/// Event sent to the frontend whenever a watched spec file changes.
const PREVIEW_UPDATED: &str = "preview-updated";

/// Editors often write a file in several steps; changes arriving within
/// this window are handled as one reload.
const DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Default)]
struct WatchState {
    /// The open spec directory and its watcher. Dropping the watcher stops
    /// watching and ends the reload thread.
    open: Mutex<Option<(PathBuf, notify::RecommendedWatcher)>>,
}

/// One applied edit: the file text before and after it.
struct Change {
    file: PathBuf,
    before: String,
    after: String,
}

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

/// True for events that change a spec file. Access events are ignored:
/// reloading reads the files, and reacting to those reads would reload
/// in an endless loop.
fn changes_spec(event: &notify::Event) -> bool {
    use notify::EventKind;
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) && event.paths.iter().any(|p| is_spec_file(p))
}

fn watch(dir: &Path, app: AppHandle) -> Result<notify::RecommendedWatcher, String> {
    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event
            && changes_spec(&event)
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
    let dir = open_dir_of(&state)?;

    let file = PathBuf::from(&path)
        .canonicalize()
        .map_err(|err| format!("cannot open {path}: {err}"))?;
    if file.parent() != Some(dir.as_path()) || !is_spec_file(&file) {
        return Err(format!("{path} is not a spec file of the open directory"));
    }

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
        let mut history = history.0.lock().map_err(|_| "history is poisoned")?;
        history.redo.clear();
        history.undo.push(Change {
            file: file.clone(),
            before: source,
            after: outcome.text,
        });
        if history.undo.len() > HISTORY_LIMIT {
            history.undo.remove(0);
        }
    }
    Ok(EditApplied {
        preview: load(&dir)?,
        focus: outcome.focus,
    })
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
    let (expected, wanted) = match way {
        Replay::Undo => (&change.after, &change.before),
        Replay::Redo => (&change.before, &change.after),
    };
    let current = std::fs::read_to_string(&change.file)
        .map_err(|err| format!("cannot read {}: {err}", change.file.display()))?;
    if &current != expected {
        return Err(format!(
            "{} changed outside the designer; it cannot be undone here",
            change.file.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
    write_atomically(&change.file, wanted)?;
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

/// Creates a project in a new or empty folder: one file with an app and
/// its main window, then opens it.
#[tauri::command]
fn new_project(
    path: String,
    app: AppHandle,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<Preview, String> {
    write_new_project(Path::new(&path))?;
    open_dir(&path, app, &state, &history)
}

/// Writes the first file of a new project into `dir`, which must not hold
/// a project yet.
fn write_new_project(dir: &Path) -> Result<PathBuf, String> {
    let shown = dir.display();
    std::fs::create_dir_all(dir).map_err(|err| format!("cannot create {shown}: {err}"))?;
    let specs = clarity_spec::spec_files(dir).map_err(|err| format!("cannot read {shown}: {err}"))?;
    if !specs.is_empty() {
        return Err(format!("{shown} already holds a Clarity project; open it instead"));
    }
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "app".to_string());
    let (module, app_name) = project_names(&name);
    let text = format!(
        "module {module}\n\napp {app_name}\n    title \"{}\"\n    main Main\n\npage Main: window\n    title \"Main\"\n",
        clarity_spec::model::default_label(&app_name)
    );
    let file = dir.join(format!("{module}.clarity"));
    std::fs::write(&file, text).map_err(|err| format!("cannot write the project: {err}"))?;
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
    let tmp = file.with_extension("clarity.tmp");
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
        .invoke_handler(tauri::generate_handler![
            open_spec_dir,
            initial_spec_dir,
            apply_edit,
            undo_edit,
            redo_edit,
            history_counts,
            new_project
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Clarity");
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, CreateKind, ModifyKind, RenameMode};
    use notify::{Event, EventKind};

    #[test]
    fn a_new_project_checks_clean_and_is_not_written_twice() {
        let dir = std::env::temp_dir().join(format!("clarity-new-{}", std::process::id())).join("my-shop");
        let _ = std::fs::remove_dir_all(&dir);
        let file = write_new_project(&dir).unwrap();
        assert_eq!(file.file_name().unwrap(), "my_shop.clarity");
        let project = clarity_spec::load_dir(&dir).unwrap();
        assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
        assert_eq!(project.apps().next().unwrap().name.name, "MyShop");
        assert!(write_new_project(&dir).unwrap_err().contains("already holds"));
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
        assert!(changes_spec(&event(
            EventKind::Modify(ModifyKind::Any),
            "/s/a.clarity"
        )));
        assert!(changes_spec(&event(
            EventKind::Create(CreateKind::File),
            "/s/a.clarity"
        )));
        assert!(changes_spec(&event(
            EventKind::Modify(ModifyKind::Name(RenameMode::To)),
            "/s/a.clarity"
        )));
    }

    #[test]
    fn ignores_reads_and_other_files() {
        assert!(!changes_spec(&event(
            EventKind::Access(AccessKind::Any),
            "/s/a.clarity"
        )));
        assert!(!changes_spec(&event(
            EventKind::Modify(ModifyKind::Any),
            "/s/a.clarity.tmp"
        )));
        assert!(!changes_spec(&event(
            EventKind::Modify(ModifyKind::Any),
            "/s/notes.txt"
        )));
    }
}
