//! Clarity's folders, set once for every project on this computer: where
//! new projects go (and Open starts), and where the apps are written. Kept
//! in `folders.json` in Clarity's own folder, beside `environment.json`, so
//! the app and the `clarity` command read the same choice.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{module_dir, Settings, DEFAULT_OUT};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folders {
    /// Where New makes a project and Open starts looking.
    #[serde(default)]
    pub projects: Option<PathBuf>,
    /// Where the apps are written, a folder per project. A project that
    /// names its own app folder keeps it.
    #[serde(default)]
    pub output: Option<PathBuf>,
}

impl Folders {
    /// The folders in `file`; none set when it is not there or cannot be read.
    pub fn load(file: &Path) -> Folders {
        std::fs::read_to_string(file).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
    }

    /// The folders of this computer, from Clarity's own folder.
    pub fn current() -> Folders {
        Folders::load(&file())
    }

    pub fn save(&self, file: &Path) -> std::io::Result<()> {
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(file, serde_json::to_string_pretty(self).unwrap_or_default() + "\n")
    }

    /// Why `path` cannot be one of the folders: it must be a full path,
    /// and not a file. A folder that is not there yet is made when it is saved.
    pub fn problem(path: &str) -> Option<String> {
        let path = path.trim();
        if path.is_empty() {
            return Some("choose a folder".to_string());
        }
        let folder = Path::new(path);
        if !folder.is_absolute() {
            let example = if cfg!(windows) { r"C:\Users\me\Projects" } else { "/home/me/Projects" };
            return Some(format!("{path} is not a full path; choose a folder, like {example}"));
        }
        folder.is_file().then(|| format!("{path} is a file, not a folder"))
    }

    /// The project file a new project named `name` gets in the projects
    /// folder: `shop` is `<projects>/shop.clarity`.
    pub fn new_project_file(&self, name: &str) -> Result<PathBuf, String> {
        let Some(projects) = &self.projects else {
            return Err("no projects folder is set; choose one in Clarity settings, Folders".to_string());
        };
        let name = name.trim();
        let name = name.strip_suffix(".clarity").unwrap_or(name).trim();
        if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
            return Err("give the project a name, like shop".to_string());
        }
        Ok(projects.join(format!("{name}.{}", clarity_spec::SPEC_EXTENSION)))
    }
}

/// Where this computer's folders are kept: `folders.json` in Clarity's own folder.
pub fn file() -> PathBuf {
    crate::clarity_home().join("folders.json")
}

/// The app folder of the project file `file` with `folders`: a project
/// that keeps the default `out` goes to its own folder in the output
/// folder when one is set, else beside its modules; any other app folder
/// is relative to the project's folder, or a full path as it is.
pub fn out_dir_in(file: &Path, settings: &Settings, folders: &Folders) -> PathBuf {
    let out = settings.out.trim();
    match &folders.output {
        Some(output) if out == DEFAULT_OUT => {
            let name = file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            output.join(name)
        }
        _ => module_dir(file).join(out),
    }
}
