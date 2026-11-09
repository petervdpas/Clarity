//! Diagrams in the project tree (DESIGN 5.13): the Data model view written
//! from the dictionary, and the `.mermaid` files of the `diagrams` folder,
//! which are design and no generated application carries. A written
//! diagram proposes edits; Apply makes the ticked ones as one undo step,
//! with the keys of their labels in every language file.

use std::path::Path;

use clarity_spec::diagrams::{self, DIAGRAMS_DIR, DiagramKind, DiagramKindInfo, Proposal, views};
use clarity_spec::{Diagnostic, Preview};
use tauri::State;

use super::{FileChange, HistoryState, WatchState, load, open_project_of, record, write_atomically};

pub(crate) use clarity_tools::diagrams::{apply_text, diagram_file, proposals_of, read_project};

/// The kinds of diagram, for the New diagram dialog.
#[tauri::command]
pub fn diagram_kinds() -> Vec<DiagramKindInfo> {
    diagrams::kinds()
}

/// The problems of a diagram's text, placed on their lines.
#[tauri::command]
pub fn check_diagram(text: String) -> Vec<Diagnostic> {
    diagrams::parse(&text).1
}

/// An ERD as the designer edits it: entities with their places, fields
/// and relations, with the problems of its text.
#[tauri::command]
pub fn read_erd(text: String) -> diagrams::erd_model::ErdRead {
    let (erd, problems) = diagrams::erd_model::read(&text);
    diagrams::erd_model::ErdRead { erd, problems }
}

/// The designer's ERD written as Mermaid, its places as `%% clarity:` lines.
#[tauri::command]
pub fn write_erd(erd: diagrams::erd_model::Erd) -> String {
    diagrams::erd_model::write(&erd)
}

/// A class diagram as the designer edits it: boxes with their places,
/// members and modules, and relations, with the problems of its text.
#[tauri::command]
pub fn read_classes(text: String) -> diagrams::class_model::ClassRead {
    let (design, problems) = diagrams::class_model::read(&text);
    diagrams::class_model::ClassRead { design, problems }
}

/// The designer's class diagram written as Mermaid, its places as `%% clarity:` lines.
#[tauri::command]
pub fn write_classes(design: diagrams::class_model::ClassDesign) -> String {
    diagrams::class_model::write(&design)
}

/// Makes diagram `name` of `kind`, with the kind's starter text.
#[tauri::command]
pub fn add_diagram(name: String, kind: DiagramKind, from_project: bool, state: State<WatchState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let file = diagram_file(&dir, &name)?;
    if file.exists() {
        return Err(format!("the diagrams have {} already", file.file_name().unwrap_or_default().to_string_lossy()));
    }
    let starter = diagrams::starter(kind);
    let text = if from_project { views::generate(&read_project(&dir)?, &starter, false).unwrap_or(starter) } else { starter };
    std::fs::create_dir_all(dir.join(DIAGRAMS_DIR)).map_err(|err| format!("cannot make the diagrams folder: {err}"))?;
    write_atomically(&file, &text)?;
    load(&dir)
}

/// Writes diagram `name` again from the project, keeping the places of its
/// boxes: every entity, enum, engine and module, or with `only_drawn` those
/// it draws. One undo step takes it back.
#[tauri::command]
pub fn generate_diagram(name: String, only_drawn: bool, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let file = diagram_file(&dir, &name)?;
    let before = std::fs::read_to_string(&file).map_err(|err| format!("cannot read {}: {err}", file.display()))?;
    let text = views::generate(&read_project(&dir)?, &before, only_drawn)
        .ok_or("Clarity writes ERDs and class diagrams from the project; this kind is drawn only")?;
    if text != before {
        write_atomically(&file, &text)?;
        record(&history, vec![FileChange { file, before: Some(before), after: Some(text) }])?;
    }
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
        record(&history, vec![FileChange { file, before, after: Some(text) }])?;
    }
    load(&dir)
}

/// Renames diagram `from` to `to` in the module folder `dir`: the text
/// moves to the new file and the old one goes. Gives both changes, for one
/// Undo.
pub(crate) fn rename_file(dir: &Path, from: &str, to: &str) -> Result<Vec<FileChange>, String> {
    let old = diagram_file(dir, from)?;
    let new = diagram_file(dir, to)?;
    if old == new {
        return Ok(Vec::new());
    }
    if new.exists() {
        return Err(format!("the diagrams have {} already", new.file_name().unwrap_or_default().to_string_lossy()));
    }
    let text = std::fs::read_to_string(&old).map_err(|err| format!("cannot read {}: {err}", old.display()))?;
    let changes = vec![
        FileChange { file: new, before: None, after: Some(text.clone()) },
        FileChange { file: old, before: Some(text), after: None },
    ];
    for change in &changes {
        change.make()?;
    }
    Ok(changes)
}

/// Renames diagram `from` to `to`, as one undo step.
#[tauri::command]
pub fn rename_diagram(from: String, to: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let changes = rename_file(&dir, &from, &to)?;
    if !changes.is_empty() {
        record(&history, changes)?;
    }
    load(&dir)
}

/// Takes diagram `name` away, as one undo step.
#[tauri::command]
pub fn delete_diagram(name: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let file = diagram_file(&dir, &name)?;
    let text = std::fs::read_to_string(&file).map_err(|err| format!("cannot read {}: {err}", file.display()))?;
    let change = FileChange { file, before: Some(text), after: None };
    change.make()?;
    record(&history, vec![change])?;
    load(&dir)
}

/// What the diagram `text` proposes for the open project.
#[tauri::command]
pub fn diagram_proposals(text: String, state: State<WatchState>) -> Result<Vec<Proposal>, String> {
    let (_, dir) = open_project_of(&state)?;
    proposals_of(&dir, &text)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_diagram_is_renamed_by_moving_its_text_to_the_new_file() {
        let dir = std::env::temp_dir().join(format!("clarity-rename-diagram-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(DIAGRAMS_DIR)).unwrap();
        std::fs::write(dir.join("diagrams/orders.mermaid"), "erDiagram\n").unwrap();
        std::fs::write(dir.join("diagrams/taken.mermaid"), "classDiagram\n").unwrap();

        assert!(rename_file(&dir, "orders", "taken").is_err(), "a name in use is refused");
        assert!(rename_file(&dir, "orders.mermaid", "orders").unwrap().is_empty(), "the same name changes nothing");

        let changes = rename_file(&dir, "orders.mermaid", "sales").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("diagrams/sales.mermaid")).unwrap(), "erDiagram\n");
        assert!(!dir.join("diagrams/orders.mermaid").exists());
        // Undo makes the new file go and the old one come back.
        let undo: Vec<(bool, bool)> = changes.iter().map(|c| (c.before.is_some(), c.after.is_some())).collect();
        assert_eq!(undo, vec![(false, true), (true, false)]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
