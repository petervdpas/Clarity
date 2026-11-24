//! Diagrams of the project: their files in the diagrams folder, what one
//! proposes for the project, and making the proposals that are ticked.

use std::path::{Path, PathBuf};

use clarity_spec::diagrams::{self, DIAGRAM_EXTENSION, DIAGRAMS_DIR, DiagramKind, Proposal};

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

/// Writes diagram `name` with `text`, one way for the IDE and the agents.
/// A project has one ERD: a second is refused, naming the one there is. A
/// class diagram names classes by their own names (`module.Class` is
/// refused). The text ends with a newline; the diagrams folder is made when
/// missing. Gives the file, and its change for Undo when the text changed.
pub fn write_diagram(dir: &Path, name: &str, text: &str) -> Result<(PathBuf, Option<FileChange>), String> {
    let file = diagram_file(dir, name)?;
    let shown = file.file_name().unwrap_or_default().to_string_lossy().to_string();
    let kind = diagrams::kind_of(text);
    if kind == Some(DiagramKind::Erd) {
        let erds: Vec<String> = diagrams::diagram_files(dir)
            .unwrap_or_default()
            .into_iter()
            .filter(|d| d.kind == Some(DiagramKind::Erd) && d.name != shown)
            .map(|d| format!("{DIAGRAMS_DIR}/{}", d.name))
            .collect();
        if !erds.is_empty() {
            return Err(format!("the project has its ERD already ({}): write the change into it, keeping what it holds", erds.join(", ")));
        }
    }
    if kind == Some(DiagramKind::Class)
        && let Some(dotted) = dotted_class(text)
    {
        let own = dotted.rsplit('.').next().unwrap_or_default();
        return Err(format!("{dotted} is no class: name a class by its own name, in relations too ({own}), and put it in its module's namespace (namespace {} {{ class {own} }})", dotted.split('.').next().unwrap_or_default()));
    }
    let text = if text.ends_with('\n') { text.to_string() } else { format!("{text}\n") };
    let before = std::fs::read_to_string(&file).ok();
    if before.as_deref() == Some(text.as_str()) {
        return Ok((file, None));
    }
    std::fs::create_dir_all(dir.join(DIAGRAMS_DIR)).map_err(|err| format!("cannot make the diagrams folder: {err}"))?;
    write_atomically(&file, &text)?;
    Ok((file.clone(), Some(FileChange { file, before, after: Some(text) })))
}

/// Makes diagram `name` of `kind` with the kind's starter text, or with
/// `from_project` drawn from the project; refused when the name is taken,
/// and on every guard of `write_diagram`. Gives its change for Undo.
pub fn add_diagram(dir: &Path, name: &str, kind: DiagramKind, from_project: bool) -> Result<FileChange, String> {
    let file = diagram_file(dir, name)?;
    if file.exists() {
        return Err(format!("the diagrams have {} already", file.file_name().unwrap_or_default().to_string_lossy()));
    }
    let starter = diagrams::starter(kind);
    let text = if from_project { diagrams::views::generate(&read_project(dir)?, &starter, false).unwrap_or(starter) } else { starter };
    let (_, change) = write_diagram(dir, name, &text)?;
    change.ok_or_else(|| format!("{} was not written", file.display()))
}

/// The first class a class diagram names as `module.Class` (on a `class`
/// line or in a relation), which Mermaid and Clarity take as no class.
fn dotted_class(text: &str) -> Option<String> {
    let relation = |line: &str| ["--", ".."].iter().any(|arrow| line.contains(arrow));
    text.lines().map(str::trim).filter(|l| !l.starts_with("%%") && (l.starts_with("class ") || relation(l))).find_map(|line| {
        line.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.')).find_map(|word| {
            let (module, class) = word.split_once('.')?;
            let ident = |w: &str| w.chars().next().is_some_and(|c| c.is_alphabetic());
            (ident(module) && class.chars().next().is_some_and(char::is_uppercase)).then(|| word.to_string())
        })
    })
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

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("clarity-diagrams-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The IDE and the agents write a diagram the one way: the folder made,
    /// a newline at the end, one ERD, classes by their own names; given back
    /// for Undo, and nothing when the text is the same.
    #[test]
    fn a_diagram_is_written_one_way() {
        let dir = folder("write");
        let erd = "erDiagram\n    Customer {\n        text(80) name\n    }";
        let (file, change) = write_diagram(&dir, "data", erd).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), format!("{erd}\n"));
        let change = change.unwrap();
        assert_eq!((change.before, change.after), (None, Some(format!("{erd}\n"))));
        assert!(write_diagram(&dir, "data", erd).unwrap().1.is_none(), "the same text changes nothing");
        let second = write_diagram(&dir, "invoices", erd).unwrap_err();
        assert!(second.contains("diagrams/data.mermaid"), "{second}");
        let dotted = write_diagram(&dir, "classes", "classDiagram\n    class todo.Task\n").unwrap_err();
        assert!(dotted.contains("todo.Task is no class"), "{dotted}");
        assert!(!dir.join("diagrams/invoices.mermaid").exists() && !dir.join("diagrams/classes.mermaid").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A new diagram is refused where the name is taken, and as a second ERD;
    /// its change is kept for Undo.
    #[test]
    fn a_new_diagram_keeps_the_guards_and_its_undo() {
        let dir = folder("add");
        let made = add_diagram(&dir, "data", DiagramKind::Erd, false).unwrap();
        assert_eq!(made.file, dir.join("diagrams/data.mermaid"));
        assert!(made.before.is_none() && made.after.is_some());
        assert!(add_diagram(&dir, "data", DiagramKind::Class, false).unwrap_err().contains("already"));
        assert!(add_diagram(&dir, "more", DiagramKind::Erd, false).unwrap_err().contains("ERD already"));
        assert!(add_diagram(&dir, "classes", DiagramKind::Class, false).is_ok());
        std::fs::remove_dir_all(&dir).unwrap();
    }

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
