//! The browser files of a generated web program (round B, DESIGN 7.9): every
//! language's web server serves the same page, on which Clarity.UI's
//! library, compiled once for the browser, draws the program's windows and
//! asks the program on the server over the wire (`/clarity/invoke`).
//!
//! A web program carries them in `browser/` (ROOT), which its server serves as
//! they are:
//!
//! - `index.html`: the page, titled as the app;
//! - `clarity/files.json`: what the library draws from, the same content a
//!   desktop program carries: the windows and app frames as `.cxaml`, the
//!   resolved themes, the model, the resources' names, the title and the
//!   main window's size;
//! - `clarity/resources/<name>`: each file of the project's resources;
//! - the library's four files (`clarity_ui::BROWSER_FILES`), copied from
//!   the browser folder (`clarity_ui::browser_library`).
//!
//! A language's generator writes `site` (no library) into its web program;
//! Clarity's build and command line then add the library's files to every
//! program that carries `browser/index.html` (`with_library`), from the folder
//! of this machine. `files` gives both at once.

use std::path::Path;

use clarity_project::Settings;
use clarity_spec::model::Project;
use serde_json::Value;

use crate::clarity_ui::BROWSER_FILES;
use crate::OutFile;

/// The folder of a generated program its web server serves.
pub const ROOT: &str = "browser";

/// The page, its program's files and its resources, below ROOT.
pub const PAGE: &str = "browser/index.html";
pub const FILES_JSON: &str = "browser/clarity/files.json";
pub const RESOURCES: &str = "browser/clarity/resources";

/// The page every web program serves at `/`: a canvas filling the window
/// and the library's page script, which starts the rest.
pub fn page(title: &str) -> String {
    format!(
        "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>html, body {{ margin: 0; height: 100%; overflow: hidden; }} #clarity-ui {{ display: block; width: 100vw; height: 100vh; touch-action: none; }}</style>\n</head>\n<body>\n<canvas id=\"clarity-ui\"></canvas>\n<script src=\"clarity-ui-page.js\"></script>\n</body>\n</html>\n",
        escaped(title)
    )
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A JSON object of `pairs` in their order (each value already JSON).
fn object(pairs: impl IntoIterator<Item = (String, String)>) -> String {
    let fields: Vec<String> = pairs.into_iter().map(|(key, value)| format!("{}:{value}", Value::String(key))).collect();
    format!("{{{}}}", fields.join(","))
}

/// A file's text as a JSON value: itself when it is JSON, else as a string.
fn json_of(text: &str) -> String {
    match serde_json::from_str::<Value>(text) {
        Ok(value) => value.to_string(),
        Err(_) => Value::String(text.to_string()).to_string(),
    }
}

/// `files.json`: what the library draws the program from (`{"title",
/// "width", "height", "windows", "apps", "themes", "model", "resources"}`),
/// windows and themes in the project's order, the light theme before the dark.
pub fn files_json(project: &Project, settings: &Settings) -> String {
    let size = crate::main_window(project).map_or((clarity_spec::layout::WINDOW_WIDTH, clarity_spec::layout::WINDOW_HEIGHT), |main| crate::window_size(project, &main));
    files_json_as(project, &clarity_project::title(settings), size)
}

/// `files_json` with its title and size given: the IDE's preview and
/// designer show any window of the project (`crate::preview`).
pub fn files_json_as(project: &Project, title: &str, (width, height): (u32, u32)) -> String {
    let markup = |files: Vec<OutFile>, folder: &str| -> String {
        object(files.into_iter().filter_map(|f| {
            let name = f.path.strip_prefix(folder)?.strip_prefix('/')?.strip_suffix(".cxaml")?.to_string();
            Some((name, Value::String(f.text).to_string()))
        }))
    };
    let windows = markup(crate::web_windows::window_files(project), crate::web_windows::WINDOWS_FOLDER);
    let apps = markup(crate::web_windows::app_files(project), crate::web_windows::APPS_FOLDER);
    let themes = object(
        crate::csharp::themes::resolved_files(project, "themes")
            .into_iter()
            .filter_map(|f| Some((f.path.strip_prefix("themes/")?.strip_suffix(".json")?.to_string(), json_of(&f.text)))),
    );
    let resources: Vec<Value> = project.resources.iter().flat_map(|r| &r.files).map(|f| Value::String(f.name.clone())).collect();
    object([
        ("title".to_string(), Value::String(title.to_string()).to_string()),
        ("width".to_string(), width.to_string()),
        ("height".to_string(), height.to_string()),
        ("windows".to_string(), windows),
        ("apps".to_string(), apps),
        ("themes".to_string(), themes),
        ("model".to_string(), json_of(&crate::web::model_json(project))),
        ("resources".to_string(), Value::Array(resources).to_string()),
    ]) + "\n"
}

/// The web program's own browser files, below ROOT: the page, `files.json`
/// and the resources. The library's files come with `with_library`.
pub fn site(project: &Project, settings: &Settings) -> Vec<OutFile> {
    let mut files = vec![
        OutFile { path: PAGE.to_string(), text: page(&clarity_project::title(settings)), bytes: None },
        OutFile { path: FILES_JSON.to_string(), text: files_json(project, settings), bytes: None },
    ];
    files.extend(crate::web::resource_files(project, RESOURCES));
    files
}

/// The library's four browser files from `folder`, below ROOT.
pub fn library(folder: &Path) -> Result<Vec<OutFile>, String> {
    BROWSER_FILES
        .iter()
        .map(|name| {
            let from = folder.join(name);
            let bytes = std::fs::read(&from).map_err(|err| format!("Clarity.UI's browser file {} cannot be read: {err}", from.display()))?;
            Ok(OutFile { path: format!("{ROOT}/{name}"), text: String::new(), bytes: Some(bytes.into()) })
        })
        .collect()
}

/// All browser files of the web program: `site` and the library's files from `folder`.
pub fn files(project: &Project, settings: &Settings, folder: &Path) -> Result<Vec<OutFile>, String> {
    let mut files = site(project, settings);
    files.extend(library(folder)?);
    Ok(files)
}

/// Whether a program's files are served to a browser: they carry the page.
pub fn serves(files: &[OutFile]) -> bool {
    files.iter().any(|f| f.path == PAGE)
}

/// Whether any of `programs` is served to a browser, so needs the library's browser files.
pub fn needs_library(programs: &[crate::Program]) -> bool {
    programs.iter().any(|p| serves(&p.files))
}

/// Adds the library's browser files from `folder` to each program served
/// to a browser, beside its page; the others stay as they are.
pub fn with_library(programs: &mut [crate::Program], folder: &Path) -> Result<(), String> {
    for program in programs.iter_mut().filter(|p| serves(&p.files)) {
        let library = library(folder)?;
        program.files.retain(|f| !library.iter().any(|l| l.path == f.path));
        program.files.extend(library);
        program.files.sort_by(|a, b| a.path.cmp(&b.path));
    }
    Ok(())
}

/// The library's browser folder for a build on this machine without a
/// download: the one CLARITY_UI_BROWSER names, the one the installed Clarity
/// carries, or the one fetched before; else `repository` (Clarity's own
/// build in its repository). The IDE fetches the release's when there is none
/// (`clarity_ui::provide`).
pub fn local_folder(repository: Option<&Path>) -> Result<std::path::PathBuf, String> {
    use crate::clarity_ui::{browser_library, Library};
    match browser_library() {
        Library::Named(folder) | Library::Bundled(folder) | Library::Fetched(folder) => Ok(folder),
        _ => repository.and_then(crate::clarity_ui::repository_browser).ok_or_else(|| NOT_HERE.to_string()),
    }
}

/// Why a web program cannot be written without the browser files.
pub const NOT_HERE: &str = "Clarity.UI's browser files are not on this machine: set the environment variable CLARITY_UI_BROWSER to a folder with clarity-ui.js, clarity-ui.wasm, clarity-ui-worker.js and clarity-ui-page.js (in Clarity's repository runtimes/rust-ui-browser/build.sh makes them)";
