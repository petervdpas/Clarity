//! The written diagrams of a project: the `.mermaid` files of its `diagrams`
//! folder, read with the project, and the text a new diagram starts with.

use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

use super::{DATA_MODEL_FILE, DIAGRAM_EXTENSION, DIAGRAMS_DIR, DiagramKind, kind_of};

/// A written diagram: its file name, its text and its kind (None for a
/// kind Clarity only draws).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiagramFile {
    pub name: String,
    pub text: String,
    pub kind: Option<DiagramKind>,
}

/// The `.mermaid` files of the diagrams folder in `dir`, by name; none when
/// there is no folder.
pub fn diagram_files(dir: &Path) -> std::io::Result<Vec<DiagramFile>> {
    let folder = dir.join(DIAGRAMS_DIR);
    if !folder.is_dir() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&folder)? {
        let path = entry?.path();
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let diagram = path.is_file() && !name.starts_with('.') && path.extension().is_some_and(|e| e == DIAGRAM_EXTENSION);
        if diagram && name != DATA_MODEL_FILE {
            let text = std::fs::read_to_string(&path)?;
            found.push(DiagramFile { kind: kind_of(&text), name, text });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

/// What a new diagram of `kind` starts with: a small example of the kind,
/// to change.
pub fn starter(kind: DiagramKind) -> String {
    match kind {
        DiagramKind::Erd => "erDiagram
    Customer {
        text(80) name \"required\"
    }
    Order {
        date ordered_on \"required\"
    }
    Order }o..|| Customer : customer
"
        .to_string(),
        DiagramKind::Class => "classDiagram
    class Customer {
        +name: text(80), required
    }
    class Order {
        +ordered_on: date, required
    }
    Order --> Customer
"
        .to_string(),
        other => {
            let header = super::kinds().into_iter().find(|k| k.kind == other).map(|k| k.header).unwrap_or_default();
            format!("{header}\n")
        }
    }
}
