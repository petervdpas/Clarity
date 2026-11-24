//! Writing project files: which files an agent may write and by which tool,
//! a change with its text before and after, for Undo, and writes through a
//! rename so no reader sees half a file. Every front end writes a project's
//! text files through here: the agents' write_file, an ACP agent's own
//! writes and the IDE's files of a part's folder.

use std::path::{Path, PathBuf};

/// One file's text before and after a change. A file that did not exist
/// before has no text, and undoing its change removes it.
#[derive(Debug)]
pub struct FileChange {
    pub file: PathBuf,
    /// The text before; None when the change made the file.
    pub before: Option<String>,
    /// The text after; None when the change took the file away.
    pub after: Option<String>,
}

impl FileChange {
    /// Makes the change: writes the text after, or takes the file away.
    pub fn make(&self) -> Result<(), String> {
        match &self.after {
            Some(text) => write_atomically(&self.file, text),
            None => std::fs::remove_file(&self.file).map_err(|err| format!("cannot remove {}: {err}", self.file.display())),
        }
    }
}

/// Whether `path` is a Clarity file (`.clarity`).
pub fn is_spec_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == clarity_spec::SPEC_EXTENSION)
}

/// Writes through a temporary file and a rename, so readers (including the
/// watcher and other editors) never see a half-written spec.
pub fn write_atomically(file: &Path, text: &str) -> Result<(), String> {
    let mut tmp = file.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, text).map_err(|err| format!("cannot write {}: {err}", tmp.display()))?;
    std::fs::rename(&tmp, file).map_err(|err| {
        let _ = std::fs::remove_file(&tmp);
        format!("cannot replace {}: {err}", file.display())
    })
}

impl From<crate::Written> for FileChange {
    fn from(w: crate::Written) -> FileChange {
        FileChange { file: w.file, before: w.before, after: w.after }
    }
}

impl From<FileChange> for crate::Written {
    fn from(c: FileChange) -> crate::Written {
        crate::Written { file: c.file, before: c.before, after: c.after }
    }
}

/// Writes the text file `file` the one way every front end writes one: its
/// folder made, a newline at the end of a text that has any, through a
/// rename. Gives the change, for Undo.
pub fn write_text(file: &Path, text: &str) -> Result<FileChange, String> {
    let before = std::fs::read_to_string(file).ok();
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
    }
    let text = if text.is_empty() || text.ends_with('\n') { text.to_string() } else { format!("{text}\n") };
    write_atomically(file, &text)?;
    Ok(FileChange { file: file.to_path_buf(), before, after: Some(text) })
}

/// A project file an agent may write, by the tool that writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Writable {
    /// A module, the project file, the README or a Markdown text of the
    /// resources: written by write_file, which checks the project after.
    Checked(PathBuf),
    /// A diagram, by its name: written by write_diagram, which checks it.
    Diagram(String),
    /// A file of a custom part's or a library's folder, not Clarity's own.
    PartFile(PathBuf),
}

/// The file a tool's path names in the project of the project file `file`
/// and its module folder `dir`, for reading: `project`, a module, README.md,
/// a Markdown text of the resources, a window or view, a diagram, or a file
/// of a custom part's or a library's folder ([`crate::parts::part_path`]).
pub fn project_file(file: &Path, dir: &Path, path: &str) -> Result<PathBuf, String> {
    let path = path.trim();
    if path == "project" {
        return Ok(file.to_path_buf());
    }
    // The project's description: its design goal and guide.
    if path == clarity_spec::README_FILE {
        return Ok(dir.join(clarity_spec::README_FILE));
    }
    // A Markdown file of the resources: `resources/rules-en.md`, a plain name.
    if let Some(name) = path.strip_prefix("resources/") {
        let plain = !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\']);
        if plain && clarity_spec::resources::is_markdown(name) {
            return Ok(dir.join(clarity_spec::resources::RESOURCES_DIR).join(name));
        }
        return Err(format!("'{path}' is not a Markdown file of the resources: write one like resources/rules-en.md; pictures are added in Clarity"));
    }
    // A window or view: `design/Main.cxaml`, a plain name.
    if let Some(name) = path.strip_prefix(&format!("{}/", clarity_spec::cxaml::DESIGN_DIR)) {
        let plain = !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\']);
        if plain && clarity_spec::cxaml::is_page_file(name) {
            return Ok(clarity_spec::cxaml::design_folder(dir).join(name));
        }
        return Err(format!("'{path}' is not a window or view: write one like design/Main.cxaml"));
    }
    // A written diagram: `diagrams/orders.mermaid`.
    if let Some(name) = path.strip_prefix("diagrams/") {
        return crate::diagrams::diagram_file(dir, name);
    }
    // A file of a custom part's or a library's folder: `parts/Arena/src/lib.rs`.
    if let Some((folder, inside)) = part_folder_of(dir, path)? {
        return Ok(dir.join(&folder.folder).join(crate::parts::part_path(&folder, inside)?));
    }
    let relative = Path::new(path);
    let fine = !path.is_empty() && relative.is_relative() && relative.components().all(|c| matches!(c, std::path::Component::Normal(_))) && is_spec_file(relative);
    if !fine {
        return Err(format!("'{path}' is not a module of the project: use a path like todo.clarity inside the project's folder, or `project`"));
    }
    Ok(dir.join(relative))
}

/// The part's or library's folder `path` lies in, and the rest of `path` in it.
fn part_folder_of<'p>(dir: &Path, path: &'p str) -> Result<Option<(crate::parts::PartFolder, &'p str)>, String> {
    let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read the project: {err}"))?;
    Ok(crate::parts::part_folders(dir, &project).into_iter().find_map(|folder| {
        let inside = path.strip_prefix(&format!("{}/", folder.folder))?;
        Some((folder, inside))
    }))
}

/// Which file `path` names when an agent writes it, and by which tool: the
/// one rule of the agents' write_file and an ACP agent's own writes. Refused,
/// naming what writes it instead: Clarity's own file in a part's folder,
/// the guidance, a report or its stylesheet, a window, view or app, and
/// anything that is not a module, the project file, README.md, a Markdown
/// text of the resources, a diagram or a file of a part's folder.
pub fn writable(file: &Path, dir: &Path, path: &str) -> Result<Writable, String> {
    let trimmed = path.trim().trim_start_matches("./");
    // A file of a custom part's or a library's folder, by the one rule of part files.
    if let Some((folder, inside)) = part_folder_of(dir, path.trim())? {
        return Ok(Writable::PartFile(crate::parts::part_file(dir, &folder, inside)?));
    }
    // The guidance keeps its sections, which the wizard reads back: only its tools write it.
    if trimmed == clarity_spec::guidance::FILE {
        return Err(format!("{path} is the app's guidance for agents: write it a section at a time with set_guidance, and read it with guidance"));
    }
    // Reports and their stylesheets change only through the report tools, as the Reports tab changes them.
    if trimmed.starts_with(&format!("{}/", clarity_spec::reports::REPORTS_DIR)) && (trimmed.ends_with(".report") || trimmed.ends_with(".css")) {
        let names: Vec<String> = crate::structural::reports::definitions().into_iter().map(|t| t.name).collect();
        return Err(format!("{path} is a report or its stylesheet: change it with the report tools, which check every change: {}", names.join(", ")));
    }
    // Windows, views and the app change only through the structural tools (docs/ledger/mcp-tools.md).
    if clarity_spec::cxaml::is_page_file(path.trim()) {
        let names: Vec<String> = crate::structural::windows::definitions().into_iter().chain(crate::structural::frame::definitions()).map(|t| t.name).collect();
        return Err(format!("{path} is a window, view or app: change it with the structural tools, which check every change and keep the markup right: {}", names.join(", ")));
    }
    let target = project_file(file, dir, path)?;
    if let Some(name) = path.trim().strip_prefix("diagrams/") {
        return Ok(Writable::Diagram(name.to_string()));
    }
    Ok(Writable::Checked(target))
}

/// Writes `text` into the project file `path` for an agent that writes files
/// beside Clarity's tools (an ACP agent's own writes): a file of a part's
/// folder is written as the tools write it; a file the tools check goes to
/// the tool that checks it, and the rest is refused as the tools refuse it.
pub fn write_beside_tools(file: &Path, dir: &Path, path: &str, text: &str) -> Result<FileChange, String> {
    match writable(file, dir, path)? {
        Writable::PartFile(target) => write_text(&target, text),
        Writable::Checked(_) => Err(format!("{path} is checked as it is written: write it with write_file of Clarity's tools")),
        Writable::Diagram(_) => Err(format!("{path} is a diagram: write it with write_diagram of Clarity's tools, which checks it")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A new project with a custom part in Rust, its folder written by Clarity's starter.
    fn with_part(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("clarity-files-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        std::fs::write(dir.join("arena.clarity"), "module arena\n\npart Arena\n    crate \"parts/Arena\"\n    hears restart\n    size 4 x 3\n").unwrap();
        let project = clarity_project::load_modules(&dir).unwrap();
        clarity_gen::parts::write_part_folders(&dir, &project).unwrap();
        (root, file, dir)
    }

    /// One rule for which files an agent writes, and by which tool: the
    /// agents' write_file and an ACP agent's own writes read the same.
    #[test]
    fn which_files_an_agent_writes_is_one_rule() {
        let (root, file, dir) = with_part("writable");
        let ok = |path: &str| writable(&file, &dir, path).unwrap_or_else(|why| panic!("{path}: {why}"));
        assert_eq!(ok("arena.clarity"), Writable::Checked(dir.join("arena.clarity")));
        assert_eq!(ok("translations/en.clarity"), Writable::Checked(dir.join("translations/en.clarity")));
        assert_eq!(ok("project"), Writable::Checked(file.clone()));
        assert_eq!(ok("README.md"), Writable::Checked(dir.join("README.md")));
        assert_eq!(ok("resources/rules-en.md"), Writable::Checked(dir.join("resources/rules-en.md")));
        assert_eq!(ok("diagrams/data.mermaid"), Writable::Diagram("data.mermaid".into()));
        assert_eq!(ok("parts/Arena/src/sound.rs"), Writable::PartFile(dir.join("parts/Arena/src/sound.rs")));
        let refused = |path: &str, says: &str| {
            let why = writable(&file, &dir, path).expect_err(path);
            assert!(why.contains(says), "{path}: {why}");
        };
        refused("parts/Arena/src/clarity.rs", "Clarity writes it from the declaration");
        refused(clarity_spec::guidance::FILE, "set_guidance");
        refused("reports/invoice.report", "report tools");
        refused("reports/invoice.css", "report tools");
        refused("design/Main.cxaml", "structural tools");
        refused("parts/Arena/target/debug/x", "build's output");
        refused("parts/Arena/../escape.rs", "is not a file of part Arena");
        refused("parts/Arena/.env", "is not a file of part Arena");
        refused("parts/Arena/src\\lib.rs", "is not a file of part Arena");
        refused("notes.txt", "is not a module of the project");
        refused("diagrams/../x.mermaid", "not a file name for the diagrams folder");
        let _ = std::fs::remove_dir_all(root);
    }

    /// An agent writing beside the tools writes a part's code as the tools
    /// do, and is sent to the tool that checks anything else.
    #[test]
    fn an_agent_beside_the_tools_writes_a_part_s_code_only() {
        let (root, file, dir) = with_part("beside");
        let change = write_beside_tools(&file, &dir, "parts/Arena/src/sound.rs", "pub fn ping() {}").unwrap();
        assert_eq!((change.before, change.after.as_deref()), (None, Some("pub fn ping() {}\n")));
        assert_eq!(std::fs::read_to_string(dir.join("parts/Arena/src/sound.rs")).unwrap(), "pub fn ping() {}\n");
        assert!(write_beside_tools(&file, &dir, "arena.clarity", "x").unwrap_err().contains("write_file"));
        assert!(write_beside_tools(&file, &dir, "diagrams/data.mermaid", "x").unwrap_err().contains("write_diagram"));
        assert!(write_beside_tools(&file, &dir, "design/Main.cxaml", "x").unwrap_err().contains("structural tools"));
        assert!(write_beside_tools(&file, &dir, "notes.md", "x").is_err());
        assert!(!dir.join("notes.md").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_text_is_written_whole_with_its_folder_and_a_last_newline() {
        let root = std::env::temp_dir().join(format!("clarity-files-text-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = root.join("a/b/c.txt");
        let made = write_text(&file, "one").unwrap();
        assert_eq!((made.before, made.after.as_deref()), (None, Some("one\n")));
        let changed = write_text(&file, "two\n").unwrap();
        assert_eq!((changed.before.as_deref(), changed.after.as_deref()), (Some("one\n"), Some("two\n")));
        assert_eq!(write_text(&root.join("empty"), "").unwrap().after.as_deref(), Some(""), "an empty file stays empty");
        let _ = std::fs::remove_dir_all(root);
    }
}
