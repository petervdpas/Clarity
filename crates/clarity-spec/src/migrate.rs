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
use crate::model::{Entity, Field, FieldType, Literal, Project, snake_case};
use crate::parser::{Cursor, Node, ParseResult, build_tree, first_word, no_children, parse_field_at, parse_type};
use crate::span::Span;

/// One stored field of a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub ty: FieldType,
    pub unique: bool,
    /// In the table's full-text index (`search`).
    pub search: bool,
    /// The value existing records get when the field is added.
    pub default: Option<Literal>,
    /// The column it is stored in, when that is not the field's name.
    pub column: Option<String>,
    /// The file store a picture is kept in; the column holds its path there.
    pub store: Option<String>,
}

impl Column {
    fn from_field(field: &Field) -> Column {
        Column {
            name: field.name.name.clone(),
            ty: field.ty.clone(),
            unique: field.modifiers.unique,
            search: field.modifiers.search,
            default: field.modifiers.default.clone(),
            column: field.modifiers.column.clone(),
            store: field.modifiers.store.clone(),
        }
    }

    /// The column the field is stored in.
    pub fn stored(&self) -> &str {
        self.column.as_deref().unwrap_or(&self.name)
    }

    /// The column beside a `file` field that holds the file's name
    /// (`manual_name`), which comes and goes with it; none for other fields.
    pub fn name_column(&self) -> Option<Column> {
        (self.ty == FieldType::File).then(|| Column {
            name: format!("{}_name", self.name),
            ty: FieldType::Text { max_length: Some(FILE_NAME_LENGTH) },
            unique: false,
            search: false,
            default: None,
            column: Some(format!("{}_name", self.stored())),
            store: None,
        })
    }

    /// Whether two columns store the same kind of value; a rename is
    /// proposed only between such columns.
    fn same_type(&self, other: &Column) -> bool {
        type_text(&self.ty) == type_text(&other.ty) && self.store.is_some() == other.store.is_some()
    }

    /// Whether the values of `other` fit this column as they are: the same
    /// type, or an integer and a duration (whole seconds).
    fn stores_alike(&self, other: &Column) -> bool {
        let seconds = |ty: &FieldType| matches!(ty, FieldType::Integer | FieldType::Duration);
        self.same_type(other) || (seconds(&self.ty) && seconds(&other.ty))
    }
}

/// The longest name of a file of a `file` field.
pub const FILE_NAME_LENGTH: u32 = 255;

/// The stored shape of an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    /// The table's name, when it is not the entity's name in snake case.
    pub table: Option<String>,
    /// The key column, when it is not `id`.
    pub key: Option<String>,
}

impl Table {
    pub fn table_name(&self) -> String {
        self.table.clone().unwrap_or_else(|| snake_case(&self.name))
    }

    pub fn key_column(&self) -> &str {
        self.key.as_deref().unwrap_or("id")
    }

    /// The column field `field` is stored in.
    pub fn column_name<'a>(&'a self, field: &'a str) -> &'a str {
        self.column(field).map_or(field, Column::stored)
    }

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
    AddEntity {
        name: String,
        columns: Vec<Column>,
        table: Option<String>,
        key: Option<String>,
    },
    RemoveEntity { name: String },
    RenameEntity { from: String, to: String },
    AddField { entity: String, column: Column },
    RemoveField { entity: String, field: String },
    RenameField { entity: String, from: String, to: String },
    ChangeField { entity: String, field: String, ty: FieldType, store: Option<String> },
    AddUnique { entity: String, field: String },
    RemoveUnique { entity: String, field: String },
    /// The field joins or leaves its table's full-text index.
    AddSearch { entity: String, field: String },
    RemoveSearch { entity: String, field: String },
    /// Store the entity in another table; None is the default name.
    MapTable { entity: String, table: Option<String> },
    /// Another column identifies a record; None is `id`.
    MapKey { entity: String, key: Option<String> },
    /// Store a field in another column; None is the field's name.
    MapColumn { entity: String, field: String, column: Option<String> },
}

impl Op {
    /// Removing data, or changing its type, can lose it.
    pub fn loses_data(&self) -> bool {
        matches!(self, Op::RemoveEntity { .. } | Op::RemoveField { .. } | Op::ChangeField { .. })
    }

    /// The entities the change is about: its name before and after a rename.
    pub fn entities(&self) -> Vec<&str> {
        match self {
            Op::RenameEntity { from, to } => vec![from, to],
            other => vec![other.entity()],
        }
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
            | Op::RemoveUnique { entity, .. }
            | Op::AddSearch { entity, .. }
            | Op::RemoveSearch { entity, .. }
            | Op::MapTable { entity, .. }
            | Op::MapKey { entity, .. }
            | Op::MapColumn { entity, .. } => entity,
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
            Op::AddEntity { name, columns, table, key } => {
                if self.table(name).is_some() {
                    return Err(format!("there is already an entity '{name}'"));
                }
                self.tables.push(Table {
                    name: name.clone(),
                    columns: columns.clone(),
                    table: table.clone(),
                    key: key.clone(),
                });
            }
            Op::MapTable { entity, table } => self.table_mut(entity)?.table = table.clone(),
            Op::MapKey { entity, key } => self.table_mut(entity)?.key = key.clone(),
            Op::MapColumn { entity, field, column } => self.table_mut(entity)?.column_mut(field)?.column = column.clone(),
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
            Op::ChangeField { entity, field, ty, store } => {
                let column = self.table_mut(entity)?.column_mut(field)?;
                // A file has a column for its name, which no other value has.
                if (column.ty == FieldType::File) != (*ty == FieldType::File) {
                    return Err(format!("{entity}.{field} cannot become a file or stop being one; remove it and add a new field"));
                }
                // A list keeps its items as JSON, which no other value is.
                let list = |ty: &FieldType| matches!(ty, FieldType::Values { .. });
                if list(&column.ty) != list(ty) {
                    return Err(format!("{entity}.{field} cannot become a list or stop being one; remove it and add a new field"));
                }
                column.ty = ty.clone();
                column.store = store.clone();
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
            Op::AddSearch { entity, field } | Op::RemoveSearch { entity, field } => {
                let column = self.table_mut(entity)?.column_mut(field)?;
                let add = matches!(op, Op::AddSearch { .. });
                if column.search == add {
                    let already = if add { "searched already" } else { "not searched" };
                    return Err(format!("{entity}.{field} is {already}"));
                }
                column.search = add;
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
    /// `migration "Taken over" baseline`: the tables were there before the
    /// step, so it creates only what is missing.
    pub baseline: bool,
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
        baseline: false,
        ops: Vec::new(),
    };
    match tree.as_slice() {
        [node] if first_word(node) == Some("migration") => {
            let mut cursor = Cursor::new(node.line);
            cursor.next();
            match cursor.expect_string("a title, as in migration \"Phone numbers\"").and_then(|t| {
                let baseline = cursor.eat_keyword("baseline");
                cursor.expect_end()?;
                Ok((t, baseline))
            }) {
                Ok((title, baseline)) => {
                    migration.title = title;
                    migration.baseline = baseline;
                }
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
    let what = cursor.expect_ident("'entity', 'field', 'unique' or 'search'")?;
    let op = match (verb.name.as_str(), what.name.as_str()) {
        ("add", "entity") => {
            let name = cursor.expect_ident("an entity name")?.name;
            cursor.expect_end()?;
            let (mut columns, mut table, mut key) = (Vec::new(), None, None);
            for child in &node.children {
                no_children(child)?;
                let mut c = Cursor::new(child.line);
                // `table "x"` and `key "x"`, as in an entity.
                if let Some(word @ ("table" | "key")) = first_word(child)
                    && child.line.tokens.get(1).is_some_and(|t| t.kind == TokenKind::Str)
                {
                    c.next();
                    let value = Some(c.expect_string("a name")?);
                    c.expect_end()?;
                    if word == "table" { table = value } else { key = value }
                    continue;
                }
                columns.push(Column::from_field(&parse_field_at(&mut c, child.line.span)?));
            }
            return Ok(Op::AddEntity { name, columns, table, key });
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
            let ty = parse_type(&mut cursor)?;
            // A picture kept in a file store: `image, store photos`.
            let store = if cursor.eat(TokenKind::Comma) {
                let word = cursor.expect_ident("'store' and a file store")?;
                if word.name != "store" {
                    return Err(Diagnostic::error("expected 'store' and a file store", word.span));
                }
                Some(cursor.expect_ident("a file store")?.name)
            } else {
                None
            };
            Op::ChangeField { entity, field, ty, store }
        }
        ("add", "unique") => {
            let (entity, field) = field_path(&mut cursor)?;
            Op::AddUnique { entity, field }
        }
        ("remove", "unique") => {
            let (entity, field) = field_path(&mut cursor)?;
            Op::RemoveUnique { entity, field }
        }
        ("add", "search") => {
            let (entity, field) = field_path(&mut cursor)?;
            Op::AddSearch { entity, field }
        }
        ("remove", "search") => {
            let (entity, field) = field_path(&mut cursor)?;
            Op::RemoveSearch { entity, field }
        }
        ("map", "table") => {
            let entity = cursor.expect_ident("an entity")?.name;
            Op::MapTable { entity, table: mapped_to(&mut cursor)? }
        }
        ("map", "key") => {
            let entity = cursor.expect_ident("an entity")?.name;
            Op::MapKey { entity, key: mapped_to(&mut cursor)? }
        }
        ("map", "column") => {
            let (entity, field) = field_path(&mut cursor)?;
            Op::MapColumn { entity, field, column: mapped_to(&mut cursor)? }
        }
        ("add" | "remove" | "rename" | "change" | "map", _) => {
            return Err(Diagnostic::error(
                format!("expected 'entity', 'field', 'unique' or 'search' after '{}'", verb.name),
                what.span,
            ));
        }
        (other, _) => {
            return Err(Diagnostic::error(
                format!("unknown migration step '{other}'; the steps add, remove, rename, change and map"),
                verb.span,
            ));
        }
    };
    cursor.expect_end()?;
    no_children(node)?;
    Ok(op)
}

/// The schema the steps of data source `source` build, skipping steps that
/// do not fit (the checker reports them).
pub fn schema(project: &Project, source: &str) -> Schema {
    replay(steps(project, source)).0
}

/// The steps of data source `source`.
fn steps<'a>(project: &'a Project, source: &str) -> &'a [Migration] {
    project.migrations.as_ref().and_then(|m| m.get(source)).map_or(&[], Vec::as_slice)
}

/// Replays the steps, with the errors of the ones that do not fit.
fn replay(migrations: &[Migration]) -> (Schema, Vec<Diagnostic>) {
    let mut schema = Schema::default();
    let mut errors = Vec::new();
    for (i, migration) in migrations.iter().enumerate() {
        if migration.baseline && i > 0 {
            errors.push(
                Diagnostic::error("only the first step of a data source can be a baseline", Span::default()).in_file(&migration.path),
            );
        }
        for (op, span) in &migration.ops {
            if migration.baseline && !matches!(op, Op::AddEntity { .. }) {
                errors.push(Diagnostic::error("a baseline step only adds entities", *span).in_file(&migration.path));
            }
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
pub fn pending(project: &Project, source: &str, renames: &[Rename]) -> Vec<Op> {
    let mut schema = schema(project, source);
    let entities: Vec<&Entity> = project.entities_in(source).collect();
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
            let op = Op::AddEntity {
                name: name.clone(),
                columns,
                table: entity.table.clone(),
                key: entity.key.clone(),
            };
            take(&mut schema, op, &mut ops);
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
        // A name that maps elsewhere; the names compared are the ones stored.
        if table.table_name() != entity.table_name() {
            take(&mut schema, Op::MapTable { entity: name.clone(), table: entity.table.clone() }, &mut ops);
        }
        if table.key_column() != entity.key_column() {
            take(&mut schema, Op::MapKey { entity: name.clone(), key: entity.key.clone() }, &mut ops);
        }
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
                    store: wanted.store.clone(),
                };
                take(&mut schema, op, &mut ops);
            }
            if column.unique != wanted.unique {
                let (entity, field) = (name.clone(), wanted.name.clone());
                let op = if wanted.unique { Op::AddUnique { entity, field } } else { Op::RemoveUnique { entity, field } };
                take(&mut schema, op, &mut ops);
            }
            if column.search != wanted.search {
                let (entity, field) = (name.clone(), wanted.name.clone());
                let op = if wanted.search { Op::AddSearch { entity, field } } else { Op::RemoveSearch { entity, field } };
                take(&mut schema, op, &mut ops);
            }
            if column.stored() != wanted.stored() {
                let op = Op::MapColumn {
                    entity: name.clone(),
                    field: wanted.name.clone(),
                    column: wanted.column.clone(),
                };
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
pub fn proposed_renames(project: &Project, source: &str) -> Vec<Rename> {
    let mut schema = schema(project, source);
    let entities: Vec<&Entity> = project.entities_in(source).collect();
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
            if let Some(new) = came.iter().find(|c| !taken.contains(&c.name) && c.stores_alike(gone)) {
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
    if column.search {
        text.push_str(", search");
    }
    if let Some(default) = &column.default {
        text.push_str(&format!(", default {}", literal_text(default)));
    }
    if let Some(stored) = &column.column {
        text.push_str(&format!(", column {}", quote(stored)));
    }
    if let Some(store) = &column.store {
        text.push_str(&format!(", store {store}"));
    }
    text
}

/// `to "Name"`, or `to default` for the default name.
fn mapped_text(name: &Option<String>) -> String {
    name.as_deref().map_or_else(|| "to default".to_string(), |n| format!("to {}", quote(n)))
}

/// A change as a line of a step (an added entity takes several lines).
pub fn op_text(op: &Op) -> String {
    match op {
        Op::AddEntity { name, columns, table, key } => {
            let mut text = format!("add entity {name}");
            if let Some(table) = table {
                text.push_str(&format!("\n    table {}", quote(table)));
            }
            if let Some(key) = key {
                text.push_str(&format!("\n    key {}", quote(key)));
            }
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
        Op::ChangeField { entity, field, ty, store } => {
            let store = store.as_ref().map(|s| format!(", store {s}")).unwrap_or_default();
            format!("change field {entity}.{field}: {}{store}", type_text(ty))
        }
        Op::AddUnique { entity, field } => format!("add unique {entity}.{field}"),
        Op::RemoveUnique { entity, field } => format!("remove unique {entity}.{field}"),
        Op::AddSearch { entity, field } => format!("add search {entity}.{field}"),
        Op::RemoveSearch { entity, field } => format!("remove search {entity}.{field}"),
        Op::MapTable { entity, table } => format!("map table {entity} {}", mapped_text(table)),
        Op::MapKey { entity, key } => format!("map key {entity} {}", mapped_text(key)),
        Op::MapColumn { entity, field, column } => format!("map column {entity}.{field} {}", mapped_text(column)),
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
        Op::AddSearch { field, .. } => format!("search {field}"),
        Op::RemoveSearch { field, .. } => format!("stop searching {field}"),
        Op::MapColumn { field, column: Some(column), .. } => format!("store {field} in column {}", quote(column)),
        Op::MapColumn { field, column: None, .. } => format!("store {field} in a column of its own name"),
        other => op_text(other),
    }
}

/// The text of a new step file.
pub fn step_text(title: &str, ops: &[Op]) -> String {
    let mut text = String::from(
        "// Written by Clarity from the changes in the data dictionary. Check it\n// before it runs; once a step has run on a database, it never changes.\n",
    );
    text.push_str(&format!("migration {}\n", quote(title)));
    push_ops(&mut text, ops);
    text
}

/// The text of the baseline step Take ownership writes: the tables as they
/// are, created only where they are missing.
pub fn step_text_baseline(title: &str, ops: &[Op]) -> String {
    let mut text = String::from(
        "// Written by Clarity when it took ownership of this database: the tables\n// as they were. It creates only what is missing, so it changes nothing\n// in the database it was taken from.\n",
    );
    text.push_str(&format!("migration {} baseline\n", quote(title)));
    push_ops(&mut text, ops);
    text
}

fn push_ops(text: &mut String, ops: &[Op]) {
    for op in ops {
        for line in op_text(op).lines() {
            text.push_str(&format!("    {line}\n"));
        }
    }
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
    let mut found = Vec::new();
    for (source, steps) in migrations {
        let message = match project.sources.iter().find(|s| s.name == *source && s.sql) {
            None => format!("the project has no data source '{source}' for these steps"),
            Some(s) if s.external => {
                format!("data source '{source}' is external; Clarity does not change its schema, so it has no migration steps")
            }
            Some(_) => continue,
        };
        let path = steps.first().map(|m| m.path.clone()).unwrap_or_default();
        found.push(Diagnostic::error(message, Span::default()).in_file(&path));
    }
    // The databases Clarity owns; external ones keep the schema they have.
    let sql: Vec<&str> = project.sources.iter().filter(|s| s.sql && !s.external).map(|s| s.name.as_str()).collect();
    for source in &sql {
        found.extend(replay(steps(project, source)).1);
        // With one database there is no need to say which.
        let place = if sql.len() > 1 { format!(" (data source {source})") } else { String::new() };
        let ops = pending(project, source, &proposed_renames(project, source));
        for file in project.files.iter() {
            for entity in file.entities.iter().filter(|e| project.source_of(&e.name.name) == Some(source)) {
                let name = &entity.name.name;
                let mine: Vec<&Op> = ops.iter().filter(|op| op.entity() == name).collect();
                let message = match mine.as_slice() {
                    [] => continue,
                    [Op::AddEntity { .. }] => format!("entity '{name}' is not in a migration yet{place}"),
                    changes => format!(
                        "{name} has changes that are not in a migration yet{place}: {}",
                        changes.iter().map(|op| short_text(op)).collect::<Vec<_>>().join(", ")
                    ),
                };
                found.push(Diagnostic::warning(message, entity.name.span).in_file(&file.path));
            }
        }
        let folder = steps(project, source)
            .last()
            .map(|m| m.path.clone())
            .unwrap_or_else(|| format!("migrations/{source}"));
        for op in &ops {
            if let Op::RemoveEntity { name } = op {
                found.push(
                    Diagnostic::warning(format!("entity '{name}' is removed, but not in a migration yet{place}"), Span::default())
                        .in_file(&folder),
                );
            }
        }
    }
    project.diagnostics.extend(found);
}

/// `to "Name"` or `to default`, after `map table`, `map key` or `map column`.
fn mapped_to(cursor: &mut Cursor) -> ParseResult<Option<String>> {
    if !cursor.eat_keyword("to") {
        return Err(cursor.error_here("expected 'to' and a name in quotes, or 'to default'"));
    }
    if cursor.eat_keyword("default") {
        return Ok(None);
    }
    Ok(Some(cursor.expect_string("a name in quotes, or 'default'")?))
}
