//! Databases Clarity does not own. It reads their schema to import tables
//! into the data dictionary as entities mapped onto them, and to check
//! that the dictionary still fits the tables it maps onto. SQLite for now;
//! the other SQL databases follow with their dialects (D1d).

use std::path::Path;

use clarity_spec::dictionary::{FieldSpec, field_line};
use clarity_spec::model::{Entity, FieldType, Modifiers, snake_case};
use clarity_spec::{Diagnostic, Project};

/// A column as the database declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveColumn {
    pub name: String,
    /// The declared type, as written: `VARCHAR(80)`, `NUMERIC(10,2)`.
    pub declared: String,
    pub not_null: bool,
    pub has_default: bool,
    /// Part of the primary key.
    pub primary_key: bool,
    /// A unique index on this column alone.
    pub unique: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveTable {
    pub name: String,
    pub columns: Vec<LiveColumn>,
    /// (column, table it refers to)
    pub foreign_keys: Vec<(String, String)>,
}

impl LiveTable {
    pub fn column(&self, name: &str) -> Option<&LiveColumn> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// The column that identifies a row, when that is one integer column.
    pub fn key(&self) -> Option<&str> {
        let key: Vec<&LiveColumn> = self.columns.iter().filter(|c| c.primary_key).collect();
        match key.as_slice() {
            [only] if kind(&only.declared) == Kind::Integer => Some(only.name.as_str()),
            _ => None,
        }
    }
}

/// The tables of a database, by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LiveSchema {
    pub tables: Vec<LiveTable>,
}

impl LiveSchema {
    pub fn table(&self, name: &str) -> Option<&LiveTable> {
        self.tables.iter().find(|t| t.name == name)
    }
}

/// Reads the tables of an SQLite database, without changing it.
pub fn read_sqlite(path: &Path) -> Result<LiveSchema, String> {
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = rusqlite::Connection::open_with_flags(path, flags)
        .map_err(|err| format!("cannot open {}: {err}", path.display()))?;
    read(&conn).map_err(|err| format!("cannot read {}: {err}", path.display()))
}

fn read(conn: &rusqlite::Connection) -> rusqlite::Result<LiveSchema> {
    let mut names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name != 'clarity_schema'")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    names.sort();
    let mut tables = Vec::new();
    for name in names {
        let unique = unique_columns(conn, &name)?;
        let columns = conn
            .prepare("SELECT name, type, \"notnull\", dflt_value IS NOT NULL, pk FROM pragma_table_info(?1)")?
            .query_map([&name], |row| {
                let column: String = row.get(0)?;
                Ok(LiveColumn {
                    unique: unique.contains(&column),
                    name: column,
                    declared: row.get(1)?,
                    not_null: row.get::<_, i64>(2)? != 0,
                    has_default: row.get(3)?,
                    primary_key: row.get::<_, i64>(4)? > 0,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let foreign_keys = conn
            .prepare("SELECT \"from\", \"table\" FROM pragma_foreign_key_list(?1)")?
            .query_map([&name], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        tables.push(LiveTable {
            name,
            columns,
            foreign_keys,
        });
    }
    Ok(LiveSchema { tables })
}

/// The columns that have a unique index of their own.
fn unique_columns(conn: &rusqlite::Connection, table: &str) -> rusqlite::Result<Vec<String>> {
    let indexes: Vec<String> = conn
        .prepare("SELECT name FROM pragma_index_list(?1) WHERE \"unique\" = 1")?
        .query_map([table], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut columns = Vec::new();
    for index in indexes {
        let names: Vec<String> = conn
            .prepare("SELECT name FROM pragma_index_info(?1)")?
            .query_map([&index], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        if let [only] = names.as_slice() {
            columns.push(only.clone());
        }
    }
    Ok(columns)
}

/// What a declared column type holds, read from its name the way SQLite
/// reads it, with dates and bools told apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Bool,
    DateTime,
    Date,
    Integer,
    Text,
    Number,
    Blob,
    /// No declared type: anything goes.
    Any,
}

fn kind(declared: &str) -> Kind {
    let d = declared.to_ascii_uppercase();
    let has = |words: &[&str]| words.iter().any(|w| d.contains(w));
    if d.trim().is_empty() {
        Kind::Any
    } else if has(&["BOOL", "BIT"]) {
        Kind::Bool
    } else if has(&["DATETIME", "TIMESTAMP"]) {
        Kind::DateTime
    } else if has(&["DATE"]) {
        Kind::Date
    } else if has(&["INT"]) {
        Kind::Integer
    } else if has(&["CHAR", "CLOB", "TEXT"]) {
        Kind::Text
    } else if has(&["REAL", "FLOA", "DOUB", "NUMERIC", "DECIMAL", "MONEY"]) {
        Kind::Number
    } else if has(&["BLOB"]) {
        Kind::Blob
    } else {
        Kind::Any
    }
}

/// The numbers in parentheses after a type: `VARCHAR(80)`, `NUMERIC(10,2)`.
fn sizes(declared: &str) -> Vec<u32> {
    let Some(inner) = declared.split_once('(').and_then(|(_, rest)| rest.split_once(')')).map(|(inner, _)| inner) else {
        return Vec::new();
    };
    inner.split(',').filter_map(|n| n.trim().parse().ok()).collect()
}

/// Whether a field of type `ty` can be stored in a column declared so.
fn fits(ty: &FieldType, declared: &str) -> bool {
    let k = kind(declared);
    if k == Kind::Any {
        return true;
    }
    match ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme | FieldType::Enum { .. } => k == Kind::Text,
        FieldType::Integer | FieldType::Id | FieldType::Duration => matches!(k, Kind::Integer | Kind::Number),
        FieldType::Decimal { .. } | FieldType::Money => matches!(k, Kind::Number | Kind::Integer),
        FieldType::Bool => matches!(k, Kind::Bool | Kind::Integer),
        FieldType::Date => matches!(k, Kind::Date | Kind::DateTime | Kind::Text),
        FieldType::DateTime => matches!(k, Kind::DateTime | Kind::Text),
        FieldType::Ref { .. } => k == Kind::Integer,
        // The picture itself, or the path of its file in a file store.
        FieldType::Image => matches!(k, Kind::Blob | Kind::Text),
        FieldType::List { .. } => false,
    }
}

/// Words of a database name: `tblCustomers` is tbl, Customers; `CustID`
/// is Cust, ID; `order_lines` is order, lines.
fn words(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut words = Vec::new();
    let mut word = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_ascii_alphanumeric() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            continue;
        }
        let prev = i.checked_sub(1).map(|j| chars[j]);
        let next = chars.get(i + 1).copied();
        let starts = c.is_ascii_uppercase()
            && prev.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit()
                || (p.is_ascii_uppercase() && next.is_some_and(|n| n.is_ascii_lowercase())));
        if starts && !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
        word.push(c);
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// `tblCustomers` as `Customers`: a table prefix such as tbl goes.
fn entity_name(table: &str) -> String {
    let mut parts = words(table);
    if parts.len() > 1 && parts[0].eq_ignore_ascii_case("tbl") {
        parts.remove(0);
    }
    parts
        .iter()
        .map(|w| {
            let lower = w.to_ascii_lowercase();
            let mut chars = lower.chars();
            chars.next().map(|f| f.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
        })
        .collect()
}

/// `CustName` as `cust_name`.
fn field_name(column: &str) -> String {
    let name = words(column).iter().map(|w| w.to_ascii_lowercase()).collect::<Vec<_>>().join("_");
    if name.starts_with(|c: char| c.is_ascii_digit()) || name.is_empty() { format!("f_{name}") } else { name }
}

/// The text of an imported module, and the tables left out with why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    pub text: String,
    pub skipped: Vec<(String, String)>,
}

/// Writes `tables` of `schema` as a module of entities in data source
/// `source`, mapped onto the tables and columns. Names already in the
/// project (`taken`) get the source's name after them.
pub fn import(schema: &LiveSchema, tables: &[String], module: &str, source: &str, taken: &[String]) -> Imported {
    let mut skipped = Vec::new();
    let mut chosen: Vec<(&LiveTable, String)> = Vec::new();
    for name in tables {
        let Some(table) = schema.table(name) else {
            skipped.push((name.clone(), "the database has no such table".to_string()));
            continue;
        };
        if let Some(reason) = not_importable(table) {
            skipped.push((name.clone(), reason));
            continue;
        }
        let mut entity = entity_name(name);
        if taken.contains(&entity) || chosen.iter().any(|(_, e)| *e == entity) {
            entity.push_str(&entity_name(source));
        }
        chosen.push((table, entity));
    }

    let mut text = format!("// Imported by Clarity from data source {source}.\nmodule {module}\n\nsource {source}\n");
    for (table, entity) in &chosen {
        let key = table.key().expect("checked above");
        text.push_str(&format!("\nentity {entity}\n"));
        if *table.name != snake_case(entity) {
            text.push_str(&format!("    table {}\n", clarity_spec::lexer::quote(&table.name)));
        }
        if key != "id" {
            text.push_str(&format!("    key {}\n", clarity_spec::lexer::quote(key)));
        }
        let mut left_out = Vec::new();
        for column in table.columns.iter().filter(|c| c.name != key) {
            let referred = table.foreign_keys.iter().find(|(from, _)| *from == column.name);
            let target = referred.and_then(|(_, to)| chosen.iter().find(|(t, _)| t.name == *to));
            let type_text = match (target, kind(&column.declared)) {
                (Some((_, entity)), _) => format!("ref {entity}"),
                (None, Kind::Blob) => {
                    left_out.push(column.name.clone());
                    continue;
                }
                (None, Kind::Bool) => "bool".to_string(),
                (None, Kind::DateTime) => "datetime".to_string(),
                (None, Kind::Date) => "date".to_string(),
                (None, Kind::Integer) => "integer".to_string(),
                (None, Kind::Number) => match sizes(&column.declared).as_slice() {
                    [p, s] => format!("decimal({p}, {s})"),
                    _ if column.declared.to_ascii_uppercase().contains("MONEY") => "money".to_string(),
                    // Floating point and bare NUMERIC: room for most amounts and ratios.
                    _ => "decimal(18, 6)".to_string(),
                },
                (None, Kind::Text | Kind::Any) => match sizes(&column.declared).as_slice() {
                    [n] => format!("text({n})"),
                    _ => "text".to_string(),
                },
            };
            let name = field_name(&column.name);
            let spec = FieldSpec {
                name: name.clone(),
                type_text,
                modifiers: Modifiers {
                    required: column.not_null,
                    unique: column.unique,
                    column: (name != column.name).then(|| column.name.clone()),
                    ..Modifiers::default()
                },
            };
            text.push_str(&format!("    {}\n", field_line(&spec)));
        }
        for column in left_out {
            text.push_str(&format!("    // {column} is not imported: binary data is not supported yet.\n"));
        }
    }
    Imported { text, skipped }
}

/// Where the entities of data source `source` no longer fit its tables:
/// missing tables and columns are errors, a type that does not fit and a
/// required column no field fills are warnings.
pub fn drift(project: &Project, source: &str, schema: &LiveSchema) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for file in &project.files {
        for entity in file.entities.iter().filter(|e| !e.settings && project.source_of(&e.name.name) == Some(source)) {
            found.extend(entity_drift(entity, source, schema).into_iter().map(|d| d.in_file(&file.path)));
        }
    }
    found
}

fn entity_drift(entity: &Entity, source: &str, schema: &LiveSchema) -> Vec<Diagnostic> {
    let name = &entity.name.name;
    let table_name = entity.table_name();
    let Some(table) = schema.table(&table_name) else {
        return vec![Diagnostic::error(format!("{name}: data source {source} has no table '{table_name}'"), entity.name.span)];
    };
    let mut found = Vec::new();
    let key = entity.key_column();
    if table.column(key).is_none() {
        found.push(Diagnostic::error(
            format!("{name}: table '{table_name}' has no column '{key}' for the key"),
            entity.name.span,
        ));
    }
    let stored: Vec<_> = entity.fields.iter().filter(|f| !matches!(f.ty, FieldType::List { .. })).collect();
    for field in &stored {
        let column = entity.column_of(&field.name.name);
        match table.column(column) {
            None => found.push(Diagnostic::error(
                format!("{name}.{}: table '{table_name}' has no column '{column}'", field.name.name),
                field.span,
            )),
            Some(live) if !fits(&field.ty, &live.declared) => found.push(Diagnostic::warning(
                format!(
                    "{name}.{}: column '{column}' holds {}, which does not fit {}",
                    field.name.name, live.declared, field.type_text
                ),
                field.span,
            )),
            Some(_) => {}
        }
    }
    for live in &table.columns {
        let filled = live.name == key || stored.iter().any(|f| entity.column_of(&f.name.name) == live.name);
        if live.not_null && !live.has_default && !live.primary_key && !filled {
            found.push(Diagnostic::warning(
                format!(
                    "{name}: column '{}' of '{table_name}' needs a value, but no field is stored in it; new records cannot be saved",
                    live.name
                ),
                entity.name.span,
            ));
        }
    }
    found
}

/// Why a table cannot be imported yet, if it cannot.
pub fn not_importable(table: &LiveTable) -> Option<String> {
    table.key().is_none().then(|| "its key is not one integer column, which is not supported yet".to_string())
}
