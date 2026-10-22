//! Diagrams in the project tree (DESIGN 5.13): the Data model view written
//! from the dictionary, and the `.mmd` files of the `diagrams` folder,
//! which are design and no generated application carries. A written
//! diagram proposes edits; Apply makes the ticked ones as one undo step,
//! with the keys of their labels in every language file.

use std::path::{Path, PathBuf};

use clarity_spec::diagrams::{self, DIAGRAM_EXTENSION, DIAGRAMS_DIR, DiagramKind, DiagramKindInfo, Proposal, views};
use clarity_spec::{Diagnostic, Preview};
use tauri::State;

use super::{FileChange, HistoryState, WatchState, load, open_project_of, record, write_atomically};

/// The file of diagram `name` in the module folder `dir`; only a plain
/// file name, given the `.mmd` extension when it has none.
pub(crate) fn diagram_file(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\']) {
        return Err(format!("'{name}' is not a file name for the diagrams folder"));
    }
    let suffix = format!(".{DIAGRAM_EXTENSION}");
    let file = if name.ends_with(&suffix) { name.to_string() } else { format!("{name}{suffix}") };
    Ok(dir.join(DIAGRAMS_DIR).join(file))
}

fn read_project(dir: &Path) -> Result<clarity_spec::Project, String> {
    clarity_project::load_modules(dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))
}

/// The kinds of diagram, for the New diagram dialog.
#[tauri::command]
pub fn diagram_kinds() -> Vec<DiagramKindInfo> {
    diagrams::kinds()
}

/// The Data model of the open project: an ERD written from its dictionary,
/// each line linked to the entity or field it shows.
#[tauri::command]
pub fn data_model_view(state: State<WatchState>) -> Result<views::View, String> {
    let (_, dir) = open_project_of(&state)?;
    Ok(views::data_model(&read_project(&dir)?))
}

/// The problems of a diagram's text, placed on their lines.
#[tauri::command]
pub fn check_diagram(text: String) -> Vec<Diagnostic> {
    diagrams::parse(&text).1
}

/// Makes diagram `name` of `kind`, with the kind's starter text.
#[tauri::command]
pub fn add_diagram(name: String, kind: DiagramKind, state: State<WatchState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let file = diagram_file(&dir, &name)?;
    if file.exists() {
        return Err(format!("the diagrams have {} already", file.file_name().unwrap_or_default().to_string_lossy()));
    }
    std::fs::create_dir_all(dir.join(DIAGRAMS_DIR)).map_err(|err| format!("cannot make the diagrams folder: {err}"))?;
    write_atomically(&file, &diagrams::starter(kind))?;
    load(&dir)
}

/// Writes the text of diagram `name`.
#[tauri::command]
pub fn save_diagram(name: String, text: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let file = diagram_file(&dir, &name)?;
    let before = std::fs::read_to_string(&file).ok();
    if before.as_deref() != Some(text.as_str()) {
        write_atomically(&file, &text)?;
        record(&history, vec![FileChange { file, before, after: text }])?;
    }
    load(&dir)
}

/// What the diagram `text` proposes for the open project.
#[tauri::command]
pub fn diagram_proposals(text: String, state: State<WatchState>) -> Result<Vec<Proposal>, String> {
    let (_, dir) = open_project_of(&state)?;
    proposals_of(&dir, &text)
}

/// Applies the proposals of `text` at `ticked`: every module they change
/// and every language file their keys go to. Refused when it would add an
/// error anywhere in the project. Gives the files it wrote, for Undo.
pub(crate) fn apply_text(dir: &Path, text: &str, ticked: &[usize]) -> Result<Vec<FileChange>, String> {
    let before = read_project(dir)?;
    let (diagram, _) = diagrams::parse(text);
    let proposals = diagrams::propose(&before, &diagram);
    let mut sources = Vec::new();
    for path in proposals.iter().enumerate().filter(|(i, _)| ticked.contains(i)).map(|(_, p)| &p.file) {
        if !sources.iter().any(|(p, _): &(String, String)| p == path) {
            let source = std::fs::read_to_string(path).map_err(|err| format!("cannot read {path}: {err}"))?;
            sources.push((path.clone(), source));
        }
    }
    let modules = diagrams::apply(&before, &sources, &proposals, ticked).map_err(|err| err.to_string())?;

    let mut keys: Vec<(String, String)> = Vec::new();
    for (key, value) in modules.iter().flat_map(|m| &m.keys) {
        if !keys.iter().any(|(k, _)| k == key) {
            keys.push((key.clone(), value.clone()));
        }
    }
    let read_err = |err: std::io::Error| format!("cannot read {}: {err}", dir.display());
    let default = clarity_spec::translations::default_language(&before);
    let languages = clarity_spec::keyed::plan_keys(dir, &default, &keys).map_err(read_err)?;
    let mut replaced: Vec<(&Path, &str)> = modules.iter().map(|m| (Path::new(m.path.as_str()), m.text.as_str())).collect();
    replaced.extend(languages.iter().map(|(path, _, after)| (path.as_path(), after.as_str())));
    let after = clarity_project::load_modules_with_files(dir, &replaced).map_err(read_err)?;
    if let Some(error) = clarity_spec::new_error(&before, &after) {
        return Err(error.message.clone());
    }

    let mut changes = Vec::new();
    for module in &modules {
        let file = PathBuf::from(&module.path);
        let old = sources.iter().find(|(p, _)| *p == module.path).map(|(_, t)| t.clone());
        write_atomically(&file, &module.text)?;
        changes.push(FileChange { file, before: old, after: module.text.clone() });
    }
    for (path, before_text, after_text) in languages {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
        }
        write_atomically(&path, &after_text)?;
        changes.push(FileChange { file: path, before: before_text, after: after_text });
    }
    Ok(changes)
}

/// Applies the proposals of `text` at `ticked`, as one undo step.
#[tauri::command]
pub fn apply_diagram(text: String, ticked: Vec<usize>, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let changes = apply_text(&dir, &text, &ticked)?;
    if !changes.is_empty() {
        record(&history, changes)?;
    }
    load(&dir)
}

/// The proposals as the agent reads them: numbered from 1, each with
/// whether it starts ticked.
pub(crate) fn proposal_lines(proposals: &[Proposal]) -> String {
    let mut out = String::new();
    for (i, p) in proposals.iter().enumerate() {
        let change = match p.change {
            diagrams::Change::Add => "add",
            diagrams::Change::Change => "change",
            diagrams::Change::Remove => "remove",
        };
        let ticked = if p.ticked { "" } else { " (unticked: a removal, only when the person asks)" };
        out.push_str(&format!("{}. {change} {}: {}{ticked}\n", i + 1, p.subject, p.description));
    }
    out
}

/// What the diagram `text` proposes for the project in `dir`.
pub(crate) fn proposals_of(dir: &Path, text: &str) -> Result<Vec<Proposal>, String> {
    let (diagram, _) = diagrams::parse(text);
    Ok(diagrams::propose(&read_project(dir)?, &diagram))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_diagram_is_a_plain_mmd_file_of_the_diagrams_folder() {
        let dir = Path::new("/project");
        assert_eq!(diagram_file(dir, "orders").unwrap(), dir.join("diagrams/orders.mmd"));
        assert_eq!(diagram_file(dir, " orders.mmd ").unwrap(), dir.join("diagrams/orders.mmd"));
        for bad in ["", ".hidden", "../escape", "a/b", "a\\b"] {
            assert!(diagram_file(dir, bad).is_err(), "{bad}");
        }
    }
}
