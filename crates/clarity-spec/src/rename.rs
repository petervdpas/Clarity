//! Renaming an entity or an enum across the project: its declaration and
//! every reference to it, found through the parsed model rather than by
//! searching text.

use serde::Deserialize;
use ts_rs::TS;

use crate::model::{DataDecl, FieldType, Ident, Project, SpecFile};
use crate::widgets::is_identifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RenameKind {
    Entity,
    Enum,
}

/// A file whose text changes, by its path in the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Renamed {
    pub path: String,
    pub text: String,
}

/// Renames `old` to `new` in every file that can see its declaration.
/// `source` gives the current text of a file by its path.
pub fn rename(
    project: &Project,
    source: impl Fn(&str) -> Option<String>,
    kind: RenameKind,
    old: &str,
    new: &str,
) -> Result<Vec<Renamed>, String> {
    let what = match kind {
        RenameKind::Entity => "entity",
        RenameKind::Enum => "enum",
    };
    if !is_identifier(new) {
        return Err(format!("'{new}' is not a valid {what} name"));
    }
    if new == old {
        return Ok(Vec::new());
    }
    let declared = |f: &SpecFile| match kind {
        RenameKind::Entity => f.entities.iter().any(|e| e.name.name == old),
        RenameKind::Enum => f.enums.iter().any(|e| e.name.name == old),
    };
    let owners: Vec<&SpecFile> = project.files.iter().filter(|f| declared(f)).collect();
    let owner = match owners.as_slice() {
        [owner] => *owner,
        [] => return Err(format!("there is no {what} '{old}'")),
        _ => return Err(format!("'{old}' is declared in more than one module; rename it in the file")),
    };
    let module = owner.module.as_ref().map(|m| m.name.as_str());

    let mut changed = Vec::new();
    for file in &project.files {
        let this = file.module.as_ref().map(|m| m.name.as_str());
        let sees = this == module || file.uses.iter().any(|u| Some(u.name.as_str()) == module);
        if !sees {
            continue;
        }
        let taken = file.entities.iter().map(|e| &e.name.name)
            .chain(file.enums.iter().map(|e| &e.name.name))
            .chain(file.pages.iter().map(|p| &p.name.name))
            .chain(file.apps.iter().map(|a| &a.name.name))
            .any(|n| n == new);
        if taken {
            return Err(format!("'{new}' is already declared in module '{}'", this.unwrap_or("")));
        }
        let mut spans: Vec<(usize, usize)> = references(file, kind, old)
            .into_iter()
            .map(|i| (i.span.start as usize, i.span.end as usize))
            .collect();
        if spans.is_empty() {
            continue;
        }
        let Some(mut text) = source(&file.path) else {
            return Err(format!("cannot read {}", file.path));
        };
        spans.sort_unstable();
        spans.dedup();
        for (start, end) in spans.into_iter().rev() {
            if text.get(start..end) != Some(old) {
                return Err(format!("{} changed while renaming; try again", file.path));
            }
            text.replace_range(start..end, new);
        }
        changed.push(Renamed {
            path: file.path.clone(),
            text,
        });
    }
    Ok(changed)
}

/// The identifiers in a file that name the entity or enum `name`.
fn references<'f>(file: &'f SpecFile, kind: RenameKind, name: &str) -> Vec<&'f Ident> {
    let mut out: Vec<&Ident> = Vec::new();
    match kind {
        RenameKind::Entity => out.extend(file.entities.iter().map(|e| &e.name)),
        RenameKind::Enum => out.extend(file.enums.iter().map(|e| &e.name)),
    }
    for field in file.entities.iter().flat_map(|e| &e.fields) {
        match (&field.ty, kind) {
            (FieldType::Ref { entity }, RenameKind::Entity) => out.push(entity),
            (FieldType::List { item }, RenameKind::Entity) => out.push(item),
            (FieldType::Enum { name }, RenameKind::Enum) => out.push(name),
            _ => {}
        }
    }
    if kind == RenameKind::Entity {
        let data = file
            .pages
            .iter()
            .flat_map(|p| &p.data)
            .chain(file.apps.iter().flat_map(|a| &a.data));
        out.extend(data.map(|d: &DataDecl| &d.entity));
    }
    out.retain(|i| i.name == name);
    out
}
