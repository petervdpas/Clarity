//! Diagrams of the project: their files in the diagrams folder, what one
//! proposes for the project, and making the proposals that are ticked.

use std::path::{Path, PathBuf};

use clarity_spec::diagrams::{self, DIAGRAM_EXTENSION, DIAGRAMS_DIR, Proposal};

use crate::files::{FileChange, write_atomically};

/// The file of diagram `name` in the module folder `dir`; only a plain
/// file name, given the `.mermaid` extension when it has none.
pub fn diagram_file(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\']) {
        return Err(format!("'{name}' is not a file name for the diagrams folder"));
    }
    let suffix = format!(".{DIAGRAM_EXTENSION}");
    let file = if name.ends_with(&suffix) { name.to_string() } else { format!("{name}{suffix}") };
    Ok(dir.join(DIAGRAMS_DIR).join(file))
}

pub fn read_project(dir: &Path) -> Result<clarity_spec::Project, String> {
    clarity_project::load_modules(dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))
}

/// Applies the proposals of `text` at `ticked`: every module they change
/// and every language file their keys go to. Refused when it would add an
/// error anywhere in the project. Gives the files it wrote, for Undo.
pub fn apply_text(dir: &Path, text: &str, ticked: &[usize]) -> Result<Vec<FileChange>, String> {
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
        changes.push(FileChange { file, before: old, after: Some(module.text.clone()) });
    }
    for (path, before_text, after_text) in languages {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
        }
        write_atomically(&path, &after_text)?;
        changes.push(FileChange { file: path, before: before_text, after: Some(after_text) });
    }
    Ok(changes)
}

/// The proposals as the agent reads them: numbered from 1, each with
/// whether it starts ticked.
pub fn proposal_lines(proposals: &[Proposal]) -> String {
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
pub fn proposals_of(dir: &Path, text: &str) -> Result<Vec<Proposal>, String> {
    let (diagram, _) = diagrams::parse(text);
    Ok(diagrams::propose(&read_project(dir)?, &diagram))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_diagram_is_a_plain_mermaid_file_of_the_diagrams_folder() {
        let dir = Path::new("/project");
        assert_eq!(diagram_file(dir, "orders").unwrap(), dir.join("diagrams/orders.mermaid"));
        assert_eq!(diagram_file(dir, " orders.mermaid ").unwrap(), dir.join("diagrams/orders.mermaid"));
        for bad in ["", ".hidden", "../escape", "a/b", "a\\b"] {
            assert!(diagram_file(dir, bad).is_err(), "{bad}");
        }
    }
}
