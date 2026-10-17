//! Migrations: the steps that bring a database from one version of the
//! data dictionary to the next, written in Clarity in the project's
//! `migrations` folder, one file per step.
//!
//! ```text
//! migration "Phone numbers"
//!     add field Customer.phone: text(20)
//!     rename field Customer.vat_number to vat
//! ```
//!
//! Replaying the steps in order gives the schema as last migrated, so no
//! separate snapshot is kept. What the dictionary has and that schema does
//! not are the pending changes, which become the next step.

use std::collections::HashSet;

use crate::diagnostic::Diagnostic;
use crate::dictionary::{literal_text, type_text};
use crate::lexer::{TokenKind, lex, quote};
use crate::model::{Entity, Field, FieldType, Literal, Project};
use crate::parser::{Cursor, Node, ParseResult, build_tree, first_word, no_children, parse_field_at, parse_type};
use crate::span::Span;

/// One stored field of a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub ty: FieldType,
    pub unique: bool,
    /// The value existing records get when the field is added.
    pub default: Option<Literal>,
}

impl Column {
    fn from_field(field: &Field) -> Column {
        Column {
            name: field.name.name.clone(),
            ty: field.ty.clone(),
            unique: field.modifiers.unique,
            default: field.modifiers.default.clone(),
        }
    }

    /// Whether two columns store the same kind of value; a rename is
    /// proposed only between such columns.
    fn same_type(&self, other: &Column) -> bool {
        type_text(&self.ty) == type_text(&other.ty)
    }
}

/// The stored shape of an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
}

impl Table {
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name == name)
    }

    fn column_mut(&mut self, name: &str) -> Result<&mut Column, String> {
        let table = self.name.clone();
        self.columns
            .iter_mut()
            .find(|c| c.name == name)
            .ok_or_else(|| format!("{table} has no field '{name}'"))
    }
}

/// The tables of a database, as the steps so far made them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schema {
    pub tables: Vec<Table>,
}

/// One change to the schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    AddEntity { name: String, columns: Vec<Column> },
    RemoveEntity { name: String },
    RenameEntity { from: String, to: String },
    AddField { entity: String, column: Column },
    RemoveField { entity: String, field: String },
    RenameField { entity: String, from: String, to: String },
    ChangeField { entity: String, field: String, ty: FieldType },
    AddUnique { entity: String, field: String },
    RemoveUnique { entity: String, field: String },
}

impl Op {
    /// Removing data, or changing its type, can lose it.
    pub fn loses_data(&self) -> bool {
        matches!(self, Op::RemoveEntity { .. } | Op::RemoveField { .. } | Op::ChangeField { .. })
    }

    /// The entity the change is about, as the dictionary names it after
    /// the change.
    fn entity(&self) -> &str {
        match self {
            Op::AddEntity { name, .. } | Op::RemoveEntity { name } => name,
            Op::RenameEntity { to, .. } => to,
            Op::AddField { entity, .. }
            | Op::RemoveField { entity, .. }
            | Op::RenameField { entity, .. }
            | Op::ChangeField { entity, .. }
            | Op::AddUnique { entity, .. }
            | Op::RemoveUnique { entity, .. } => entity,
        }
    }
}

impl Schema {
    pub fn table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == name)
    }

    fn table_mut(&mut self, name: &str) -> Result<&mut Table, String> {
        self.tables
            .iter_mut()
            .find(|t| t.name == name)
            .ok_or_else(|| format!("there is no entity '{name}'"))
    }

    /// Applies one change, or says why it does not fit this schema.
    pub fn apply(&mut self, op: &Op) -> Result<(), String> {
        match op {
            Op::AddEntity { name, columns } => {
                if self.table(name).is_some() {
                    return Err(format!("there is already an entity '{name}'"));
                }
                self.tables.push(Table {
                    name: name.clone(),
                    columns: columns.clone(),
                });
            }
            Op::RemoveEntity { name } => {
                self.table_mut(name)?;
                let referring = self.tables.iter().find(|t| {
                    t.name != *name
                        && t.columns
                            .iter()
                            .any(|c| matches!(&c.ty, FieldType::Ref { entity } if entity.name == *name))
                });
                if let Some(table) = referring {
                    return Err(format!("{} still refers to {name}; remove that field first", table.name));
                }
                self.tables.retain(|t| t.name != *name);
            }
            Op::RenameEntity { from, to } => {
                if self.table(to).is_some() {
                    return Err(format!("there is already an entity '{to}'"));
                }
                self.table_mut(from)?.name = to.clone();
                // References follow the entity.
                for column in self.tables.iter_mut().flat_map(|t| &mut t.columns) {
                    if let FieldType::Ref { entity } = &mut column.ty
                        && entity.name == *from
                    {
                        entity.name = to.clone();
                    }
                }
            }
            Op::AddField { entity, column } => {
                let table = self.table_mut(entity)?;
                if table.column(&column.name).is_some() {
                    return Err(format!("{entity} already has a field '{}'", column.name));
                }
                table.columns.push(column.clone());
            }
            Op::RemoveField { entity, field } => {
                let table = self.table_mut(entity)?;
                table.column_mut(field)?;
                table.columns.retain(|c| c.name != *field);
            }
            Op::RenameField { entity, from, to } => {
                let table = self.table_mut(entity)?;
                if table.column(to).is_some() {
                    return Err(format!("{entity} already has a field '{to}'"));
                }
                table.column_mut(from)?.name = to.clone();
            }
            Op::ChangeField { entity, field, ty } => {
                self.table_mut(entity)?.column_mut(field)?.ty = ty.clone();
            }
            Op::AddUnique { entity, field } | Op::RemoveUnique { entity, field } => {
                let column = self.table_mut(entity)?.column_mut(field)?;
                let add = matches!(op, Op::AddUnique { .. });
                if column.unique == add {
                    let already = if add { "already unique" } else { "not unique" };
                    return Err(format!("{entity}.{field} is {already}"));
                }
                column.unique = add;
            }
        }
        Ok(())
    }
}

/// One step: a file in the `migrations` folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub path: String,
    pub title: String,
    /// The changes, each with the span of its line.
    pub ops: Vec<(Op, Span)>,
}

/// Reads a step file.
pub fn parse(text: &str, path: &str) -> (Migration, Vec<Diagnostic>) {
    let (lines, mut diagnostics) = lex(text);
    let tree = build_tree(&lines, &mut diagnostics);
    let mut migration = Migration {
        path: path.to_string(),
        title: String::new(),
        ops: Vec::new(),
    };
    match tree.as_slice() {
        [node] if first_word(node) == Some("migration") => {
            let mut cursor = Cursor::new(node.line);
            cursor.next();
            match cursor.expect_string("a title, as in migration \"Phone numbers\"").and_then(|t| {
                cursor.expect_end()?;
                Ok(t)
            }) {
                Ok(title) => migration.title = title,
                Err(diag) => diagnostics.push(diag),
            }
            for child in &node.children {
                match parse_op(child) {
                    Ok(op) => migration.ops.push((op, child.line.span)),
                    Err(diag) => diagnostics.push(diag),
                }
            }
        }
        _ => diagnostics.push(Diagnostic::error(
            "a migration file holds one 'migration \"Title\"' block",
            tree.first().map(|n| n.line.span).unwrap_or_default(),
        )),
    }
    let diagnostics = diagnostics.into_iter().map(|d| d.in_file(path)).collect();
    (migration, diagnostics)
}

/// `Entity.field`
fn field_path(cursor: &mut Cursor) -> ParseResult<(String, String)> {
    let entity = cursor.expect_ident("an entity, as in Customer.name")?;
    cursor.expect(TokenKind::Dot, "'.' and a field, as in Customer.name")?;
    let field = cursor.expect_ident("a field")?;
    Ok((entity.name, field.name))
}

fn expect_to(cursor: &mut Cursor) -> ParseResult<String> {
    if !cursor.eat_keyword("to") {
        return Err(cursor.error_here("expected 'to' and the new name"));
    }
    Ok(cursor.expect_ident("the new name")?.name)
}

fn parse_op(node: &Node) -> ParseResult<Op> {
    let mut cursor = Cursor::new(node.line);
    let verb = cursor.expect_ident("a migration step")?;
    let what = cursor.expect_ident("'entity', 'field' or 'unique'")?;
    let op = match (verb.name.as_str(), what.name.as_str()) {
        ("add", "entity") => {
            let name = cursor.expect_ident("an entity name")?.name;
            cursor.expect_end()?;
            let mut columns = Vec::new();
            for child in &node.children {
                no_children(child)?;
                let mut c = Cursor::new(child.line);
                columns.push(Column::from_field(&parse_field_at(&mut c, child.line.span)?));
            }
            return Ok(Op::AddEntity { name, columns });
        }
        ("remove", "entity") => Op::RemoveEntity {
            name: cursor.expect_ident("an entity name")?.name,
        },
        ("rename", "entity") => Op::RenameEntity {
            from: cursor.expect_ident("an entity name")?.name,
            to: expect_to(&mut cursor)?,
        },
        ("add", "field") => {
            let entity = cursor.expect_ident("an entity, as in Customer.phone")?.name;
            cursor.expect(TokenKind::Dot, "'.' and a field, as in Customer.phone")?;
            let field = parse_field_at(&mut cursor, node.line.span)?;
            no_children(node)?;
            return Ok(Op::AddField {
                entity,
                column: Column::from_field(&field),
            });
        }
        ("remove", "field") => {
            let (entity, field) = field_path(&mut cursor)?;
            Op::RemoveField { entity, field }
        }
        ("rename", "field") => {
            let (entity, from) = field_path(&mut cursor)?;
            Op::RenameField {
                entity,
                from,
                to: expect_to(&mut cursor)?,
            }
        }
        ("change", "field") => {
            let (entity, field) = field_path(&mut cursor)?;
            cursor.expect(TokenKind::Colon, "':' and the new type")?;
            Op::ChangeField {
                entity,
                field,
                ty: parse_type(&mut cursor)?,
            }
        }
        ("add", "unique") => {
            let (entity, field) = field_path(&mut cursor)?;
            Op::AddUnique { entity, field }
        }
        ("remove", "unique") => {
            let (entity, field) = field_path(&mut cursor)?;
            Op::RemoveUnique { entity, field }
        }
        ("add" | "remove" | "rename" | "change", _) => {
            return Err(Diagnostic::error(
                format!("expected 'entity', 'field' or 'unique' after '{}'", verb.name),
                what.span,
            ));
        }
        (other, _) => {
            return Err(Diagnostic::error(
                format!("unknown migration step '{other}'; the steps add, remove, rename and change"),
                verb.span,
            ));
        }
    };
    cursor.expect_end()?;
    no_children(node)?;
    Ok(op)
}

/// The schema the project's steps build, skipping steps that do not fit
/// (the checker reports them).
pub fn schema(project: &Project) -> Schema {
    replay(project.migrations.as_deref().unwrap_or_default()).0
}

/// Replays the steps, with the errors of the ones that do not fit.
fn replay(migrations: &[Migration]) -> (Schema, Vec<Diagnostic>) {
    let mut schema = Schema::default();
    let mut errors = Vec::new();
    for migration in migrations {
        for (op, span) in &migration.ops {
            if let Err(message) = schema.apply(op) {
                errors.push(Diagnostic::error(message, *span).in_file(&migration.path));
            }
        }
    }
    (schema, errors)
}

/// A rename the developer confirmed: of an entity (`entity` is None) or of
/// a field of `entity`. Without it, a name that went and a name that came
/// are a removal and an addition.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct Rename {
    pub entity: Option<String>,
    pub from: String,
    pub to: String,
}

fn stored(entity: &Entity) -> impl Iterator<Item = &Field> {
    entity.fields.iter().filter(|f| !matches!(f.ty, FieldType::List { .. }))
}

/// The changes that bring the migrated schema to the dictionary, renames
/// first, then new entities, then the fields of each entity, then removed
/// entities.
pub fn pending(project: &Project, renames: &[Rename]) -> Vec<Op> {
    let mut schema = schema(project);
    let entities: Vec<&Entity> = project.entities().collect();
    let in_dictionary = |name: &str| entities.iter().any(|e| e.name.name == name);
    let mut ops = Vec::new();
    let take = |schema: &mut Schema, op: Op, ops: &mut Vec<Op>| {
        if schema.apply(&op).is_ok() {
            ops.push(op);
        }
    };

    for rename in renames.iter().filter(|r| r.entity.is_none()) {
        if schema.table(&rename.from).is_some() && in_dictionary(&rename.to) && !in_dictionary(&rename.from) {
            let op = Op::RenameEntity {
                from: rename.from.clone(),
                to: rename.to.clone(),
            };
            take(&mut schema, op, &mut ops);
        }
    }
    for entity in &entities {
        let name = &entity.name.name;
        if schema.table(name).is_none() {
            let columns = stored(entity).map(Column::from_field).collect();
            take(&mut schema, Op::AddEntity { name: name.clone(), columns }, &mut ops);
            continue;
        }
        for rename in renames.iter().filter(|r| r.entity.as_deref() == Some(name)) {
            let table = schema.table(name).expect("checked above");
            if table.column(&rename.from).is_some() && entity.field(&rename.to).is_some() && entity.field(&rename.from).is_none() {
                let op = Op::RenameField {
                    entity: name.clone(),
                    from: rename.from.clone(),
                    to: rename.to.clone(),
                };
                take(&mut schema, op, &mut ops);
            }
        }
        let table = schema.table(name).expect("checked above").clone();
        for field in stored(entity) {
            let wanted = Column::from_field(field);
            let Some(column) = table.column(&wanted.name) else {
                take(&mut schema, Op::AddField { entity: name.clone(), column: wanted }, &mut ops);
                continue;
            };
            if !column.same_type(&wanted) {
                let op = Op::ChangeField {
                    entity: name.clone(),
                    field: wanted.name.clone(),
                    ty: wanted.ty.clone(),
                };
                take(&mut schema, op, &mut ops);
            }
            if column.unique != wanted.unique {
                let (entity, field) = (name.clone(), wanted.name.clone());
                let op = if wanted.unique { Op::AddUnique { entity, field } } else { Op::RemoveUnique { entity, field } };
                take(&mut schema, op, &mut ops);
            }
        }
        for column in &table.columns {
            if entity.field(&column.name).is_none_or(|f| matches!(f.ty, FieldType::List { .. })) {
                let op = Op::RemoveField {
                    entity: name.clone(),
                    field: column.name.clone(),
                };
                take(&mut schema, op, &mut ops);
            }
        }
    }
    let removed: Vec<String> = schema.tables.iter().map(|t| t.name.clone()).filter(|n| !in_dictionary(n)).collect();
    for name in removed {
        take(&mut schema, Op::RemoveEntity { name }, &mut ops);
    }
    ops
}

/// Renames worth proposing: an entity that went while one with mostly the
/// same fields came, and within an entity, a field that went while a field
/// of the same type came.
pub fn proposed_renames(project: &Project) -> Vec<Rename> {
    let mut schema = schema(project);
    let entities: Vec<&Entity> = project.entities().collect();
    let mut proposals = Vec::new();

    let removed: Vec<Table> = schema
        .tables
        .iter()
        .filter(|t| !entities.iter().any(|e| e.name.name == t.name))
        .cloned()
        .collect();
    let mut used = HashSet::new();
    let added: Vec<&Entity> = entities.iter().copied().filter(|e| schema.table(&e.name.name).is_none()).collect();
    for entity in added {
        let fields: HashSet<&str> = stored(entity).map(|f| f.name.name.as_str()).collect();
        let best = removed
            .iter()
            .filter(|t| !used.contains(&t.name))
            .map(|t| (t, t.columns.iter().filter(|c| fields.contains(c.name.as_str())).count()))
            .filter(|(t, shared)| *shared > 0 && shared * 2 >= t.columns.len().max(fields.len()))
            .max_by_key(|(_, shared)| *shared);
        if let Some((table, _)) = best {
            used.insert(table.name.clone());
            let rename = Rename {
                entity: None,
                from: table.name.clone(),
                to: entity.name.name.clone(),
            };
            let _ = schema.apply(&Op::RenameEntity {
                from: rename.from.clone(),
                to: rename.to.clone(),
            });
            proposals.push(rename);
        }
    }

    for entity in &entities {
        let Some(table) = schema.table(&entity.name.name) else { continue };
        let came: Vec<Column> = stored(entity)
            .filter(|f| table.column(&f.name.name).is_none())
            .map(Column::from_field)
            .collect();
        let mut taken = HashSet::new();
        for gone in table.columns.iter().filter(|c| entity.field(&c.name).is_none()) {
            if let Some(new) = came.iter().find(|c| !taken.contains(&c.name) && c.same_type(gone)) {
                taken.insert(new.name.clone());
                proposals.push(Rename {
                    entity: Some(entity.name.name.clone()),
                    from: gone.name.clone(),
                    to: new.name.clone(),
                });
            }
        }
    }
    proposals
}

fn column_text(column: &Column) -> String {
    let mut text = format!("{}: {}", column.name, type_text(&column.ty));
    if column.unique {
        text.push_str(", unique");
    }
    if let Some(default) = &column.default {
        text.push_str(&format!(", default {}", literal_text(default)));
    }
    text
}

/// A change as a line of a step (an added entity takes several lines).
pub fn op_text(op: &Op) -> String {
    match op {
        Op::AddEntity { name, columns } => {
            let mut text = format!("add entity {name}");
            for column in columns {
                text.push_str(&format!("\n    {}", column_text(column)));
            }
            text
        }
        Op::RemoveEntity { name } => format!("remove entity {name}"),
        Op::RenameEntity { from, to } => format!("rename entity {from} to {to}"),
        Op::AddField { entity, column } => format!("add field {entity}.{}", column_text(column)),
        Op::RemoveField { entity, field } => format!("remove field {entity}.{field}"),
        Op::RenameField { entity, from, to } => format!("rename field {entity}.{from} to {to}"),
        Op::ChangeField { entity, field, ty } => format!("change field {entity}.{field}: {}", type_text(ty)),
        Op::AddUnique { entity, field } => format!("add unique {entity}.{field}"),
        Op::RemoveUnique { entity, field } => format!("remove unique {entity}.{field}"),
    }
}

/// A change within its entity, for the warning on that entity.
fn short_text(op: &Op) -> String {
    match op {
        Op::AddField { column, .. } => format!("add field {}", column.name),
        Op::RemoveField { field, .. } => format!("remove field {field}"),
        Op::RenameField { from, to, .. } => format!("rename field {from} to {to}"),
        Op::ChangeField { field, ty, .. } => format!("change field {field} to {}", type_text(ty)),
        Op::AddUnique { field, .. } => format!("make {field} unique"),
        Op::RemoveUnique { field, .. } => format!("make {field} not unique"),
        other => op_text(other),
    }
}

/// The text of a new step file.
pub fn step_text(title: &str, ops: &[Op]) -> String {
    let mut text = String::from(
        "// Written by Clarity from the changes in the data dictionary. Check it\n// before it runs; once a step has run on a database, it never changes.\n",
    );
    text.push_str(&format!("migration {}\n", quote(title)));
    for op in ops {
        for line in op_text(op).lines() {
            text.push_str(&format!("    {line}\n"));
        }
    }
    text
}

/// The file name of the step after `existing`: `0003.clarity`.
pub fn next_file_name(existing: &[String]) -> String {
    let highest = existing
        .iter()
        .filter_map(|name| name.split('.').next()?.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("{:04}.{}", highest + 1, crate::SPEC_EXTENSION)
}

/// Checks the steps of a project: each one fits the schema before it, and
/// the dictionary has no changes that no step covers yet.
pub(crate) fn check(project: &mut Project) {
    let Some(migrations) = &project.migrations else { return };
    let (_, errors) = replay(migrations);
    let mut found = errors;
    let ops = pending(project, &proposed_renames(project));
    for entity in project.files.iter().flat_map(|f| f.entities.iter().map(move |e| (f, e))) {
        let (file, entity) = entity;
        let name = &entity.name.name;
        let mine: Vec<&Op> = ops.iter().filter(|op| op.entity() == name).collect();
        let message = match mine.as_slice() {
            [] => continue,
            [Op::AddEntity { .. }] => format!("entity '{name}' is not in a migration yet"),
            changes => format!(
                "{name} has changes that are not in a migration yet: {}",
                changes.iter().map(|op| short_text(op)).collect::<Vec<_>>().join(", ")
            ),
        };
        found.push(Diagnostic::warning(message, entity.name.span).in_file(&file.path));
    }
    let folder = migrations.last().map(|m| m.path.clone()).unwrap_or_else(|| "migrations".to_string());
    for op in &ops {
        if let Op::RemoveEntity { name } = op {
            found.push(
                Diagnostic::warning(format!("entity '{name}' is removed, but not in a migration yet"), Span::default())
                    .in_file(&folder),
            );
        }
    }
    project.diagnostics.extend(found);
}
