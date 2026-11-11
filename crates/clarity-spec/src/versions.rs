//! A save never overwrites someone else's change (docs/APP-CAPABILITIES-PLAN.md
//! step 5): which entities carry a version, in which column, and the checks
//! of the entity lines that say so.
//!
//! - An entity of a database Clarity owns has `row_version` once its steps
//!   add it (`add version Customer`, or `version` in its `add entity`).
//! - An entity of an external database names its column, `version
//!   "RowVersion"`, or says its saves are not checked, `version none`.
//! - A file source's file is its own version; a web API has none.

use crate::diagnostic::Diagnostic;
use crate::migrate::{self, VERSION_COLUMN};
use crate::model::{Entity, Project};

/// Where the records of a data source are kept, as far as versions go.
#[derive(Clone, Copy)]
enum Place {
    /// A database Clarity owns: `row_version`.
    Owned,
    /// An external database: the column its entities name.
    External { readonly: bool },
    /// A file in a format: the file is the unit.
    File,
    /// A web API.
    Web,
}

fn place(project: &Project, source: &str) -> Place {
    match project.sources.iter().find(|s| s.name == source) {
        None => Place::Owned,
        Some(s) if s.file => Place::File,
        Some(s) if !s.sql => Place::Web,
        Some(s) if s.external => Place::External { readonly: s.readonly },
        Some(_) => Place::Owned,
    }
}

/// The column that holds the version of entity `name`'s records; see
/// `Project::version_column`.
pub(crate) fn version_column(project: &Project, name: &str) -> Option<String> {
    let entity = project.entities().find(|e| e.name.name == name && !e.settings)?;
    let source = project.source_of(name)?;
    match place(project, source) {
        // Loaded without its steps, the dictionary is the one step, with versions.
        Place::Owned if project.migrations.is_none() => Some(VERSION_COLUMN.to_string()),
        Place::Owned => migrate::schema(project, source).table(name).filter(|t| t.version).map(|_| VERSION_COLUMN.to_string()),
        Place::External { .. } => entity.version.clone(),
        Place::File | Place::Web => None,
    }
}

/// Checks `version` and `savedSince` of every entity against its data source.
pub(crate) fn check(project: &mut Project) {
    let mut found = Vec::new();
    for file in &project.files {
        for entity in file.entities.iter().filter(|e| !e.settings) {
            let Some(source) = project.source_of(&entity.name.name) else { continue };
            found.extend(check_entity(entity, place(project, source)).into_iter().map(|d| d.in_file(&file.path)));
        }
    }
    project.diagnostics.extend(found);
}

fn check_entity(entity: &Entity, place: Place) -> Vec<Diagnostic> {
    let name = &entity.name.name;
    let written = entity.version.is_some() || entity.version_none;
    let mut found = Vec::new();
    let wrong = match place {
        Place::Owned => Some(format!("{name} is stored in a database Clarity owns, which keeps its version in {VERSION_COLUMN} itself; remove the version line")),
        Place::File => Some(format!("{name} is kept in a file, which is its own version; remove the version line")),
        Place::Web => Some(format!("{name} comes from a web API, which has no version column; remove the version line")),
        // Records that are never saved need no check.
        Place::External { readonly } if !written && !readonly => {
            let message = format!("{name} has no version column, so saves of it are not checked: name its column with version \"Column\", or write version none");
            found.push(Diagnostic::warning(message, entity.name.span));
            None
        }
        Place::External { .. } => None,
    };
    if let Some(wrong) = wrong.filter(|_| written) {
        found.push(Diagnostic::error(wrong, entity.name.span));
    }
    // The version is a column of its own, beside the key and the fields.
    if let (Place::External { .. }, Some(version)) = (&place, &entity.version) {
        let shared = if version == entity.key_column() {
            Some("the key".to_string())
        } else {
            entity.fields.iter().find(|f| entity.column_of(&f.name.name) == *version).map(|f| format!("the column of field '{}'", f.name.name))
        };
        if let Some(shared) = shared {
            found.push(Diagnostic::error(format!("{name}: its version column '{version}' is {shared}"), entity.name.span));
        }
    }
    let checked = match place {
        Place::Owned | Place::File => true,
        Place::External { readonly } => entity.version.is_some() && !readonly,
        Place::Web => false,
    };
    if entity.saved_since.is_some() && !checked {
        found.push(Diagnostic::warning(format!("{name}'s savedSince is never shown, since its saves are not checked; remove the savedSince line"), entity.name.span));
    }
    found
}

/// The slots the text of an entity's `savedSince` key may use: the record as
/// stored now, `current.` and a field (`current.name`), its id, and the
/// fields of a record it refers to (`current.changedBy.name`).
pub fn saved_since_slots(project: &Project, entity: &Entity) -> Vec<String> {
    let mut slots = vec![format!("current.{}", crate::code::check::RECORD_ID)];
    for field in &entity.fields {
        let path = format!("current.{}", field.name.name);
        if let crate::model::FieldType::Ref { entity: target } = &field.ty
            && let Some(target) = project.entity(&target.name)
        {
            slots.push(format!("{path}.{}", crate::code::check::RECORD_ID));
            slots.extend(target.fields.iter().map(|f| format!("{path}.{}", f.name.name)));
        }
        slots.push(path);
    }
    slots
}
