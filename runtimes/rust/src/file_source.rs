//! A data source that is a file in a format (docs/FORMATS-PLAN.md, "A file
//! as a data source"): `datasource logins csv "imports/logins.csv" external`.
//! The file is read through Clarity.Formats into one table of an in-memory
//! SQLite database (one per entity when its mapping holds records inside
//! records, each linked to the row it sits in by that row's key), so the generated code finds, saves and deletes as it
//! does in any SQLite database. After each change that is kept (outside a
//! transaction, or when it commits) the table is written back to a new file
//! that then replaces the old one, so a crash never leaves half a file; the
//! file's own column order and the columns no entity maps are kept.

use std::path::{Path, PathBuf};

use crate::dialect::Dialect;
use crate::sql::SqlValue;
use crate::sqlite::Sqlite;
use crate::store::{Driver, Failure, Outcome};
use crate::{Error, Id, Result};

/// What the generator says of a file source: its format and options, the
/// entity's table and key column, and the columns the entity maps with
/// their kinds (JSON `[{"name":"site","kind":"text"},...]`).
#[derive(Debug, Clone, Copy)]
pub struct FileSource<'a> {
    pub name: &'a str,
    pub format: &'a str,
    pub path: &'a str,
    pub options: &'a str,
    pub table: &'a str,
    pub key: &'a str,
    pub fields: &'a str,
    pub readonly: bool,
}

pub(crate) struct FileDriver {
    db: Sqlite,
    path: PathBuf,
    format: String,
    options: String,
    fields: String,
    /// The tables the file is read into: one, or with records inside (a mapping's `nested`) one per entity.
    tables: Vec<Kept>,
    /// Whether the file holds records inside its records, so its tables go back as several.
    several: bool,
    /// The file as it was last read or written, for formats that keep more than the table.
    original: Vec<u8>,
    readonly: bool,
    dirty: bool,
}

/// One table read from the file and kept in SQLite.
struct Kept {
    /// Its name in the file's tables: "" for the source's own, `lines` or `lines/notes` for records inside.
    name: String,
    table: String,
    key: String,
    /// The file's columns, in its order; for records inside, the second links a row to its row.
    columns: Vec<String>,
    /// Whether the second column is a link: the place of the row in the table it sits in.
    linked: bool,
}

/// The table a nested table's rows sit in: `lines/notes` in `lines`, `lines` in the source's own.
fn parent_of(name: &str) -> &str {
    name.rsplit_once('/').map_or("", |(parent, _)| parent)
}

/// The records field of the file source's fields `name` (`lines/notes`) leads to: its fields, table and key.
fn records_field<'t>(fields: &'t clarity_formats::tree::Tree, name: &str) -> Option<&'t clarity_formats::tree::Tree> {
    let mut here = fields;
    let mut found = None;
    for step in name.split('/') {
        let clarity_formats::tree::Tree::List(list) = here else { return None };
        let field = list.iter().find(|f| f.get("name").and_then(|n| n.as_text()) == Some(step))?;
        here = field.get("fields")?;
        found = Some(field);
    }
    found
}

fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// The SQLite type a mapped column of `kind` gets, so its values are kept as the generated code reads them.
fn column_type(kind: &str) -> &'static str {
    match kind {
        "integer" | "bool" => " INTEGER",
        "text" | "decimal" | "date" | "datetime" | "enum" | "values" => " TEXT",
        _ => "",
    }
}

fn cell(value: &clarity_formats::tree::Tree) -> SqlValue {
    use clarity_formats::tree::Tree;
    match value {
        Tree::Null => SqlValue::Null,
        Tree::Bool(b) => SqlValue::Integer(i64::from(*b)),
        Tree::Number(n) => n.parse::<i64>().map(SqlValue::Integer).unwrap_or_else(|_| SqlValue::Text(n.clone())),
        Tree::Text(t) => SqlValue::Text(t.clone()),
        other => SqlValue::Text(clarity_formats::tree::write(other)),
    }
}

fn tree(value: &SqlValue) -> clarity_formats::tree::Tree {
    use clarity_formats::tree::Tree;
    match value {
        SqlValue::Integer(n) => Tree::Number(n.to_string()),
        SqlValue::Real(r) => Tree::Number(r.to_string()),
        SqlValue::Text(t) => Tree::Text(t.clone()),
        SqlValue::Bool(b) => Tree::Number(if *b { "1" } else { "0" }.into()),
        SqlValue::Decimal(d) => Tree::Text(d.to_string()),
        SqlValue::Date(d) => Tree::Text(d.format("%Y-%m-%d").to_string()),
        SqlValue::DateTime(d) => Tree::Text(d.format("%Y-%m-%dT%H:%M:%S").to_string()),
        SqlValue::Null | SqlValue::Blob(_) | SqlValue::Kept => Tree::Null,
    }
}

impl FileDriver {
    /// Reads the file at `path` (already found as a SQLite file's location is) into its table.
    pub(crate) fn open(source: &FileSource, path: &Path) -> Result<FileDriver> {
        let storage = |e: String| Error::Storage(format!("data source {}: {e}", source.name));
        // A file not there yet is a new one, made at the first save; a readonly source needs it.
        let original = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !source.readonly => Vec::new(),
            Err(e) => return Err(storage(format!("cannot read {}: {e}", path.display()))),
        };
        let read = clarity_formats::read_table(source.format, &original, source.options, source.fields).map_err(|p| storage(p.text))?;
        let read = clarity_formats::tree::parse(&read).ok_or_else(|| storage("the table read is no table".into()))?;
        let all_fields = clarity_formats::tree::parse(source.fields).unwrap_or(clarity_formats::tree::Tree::List(Vec::new()));
        let several = matches!(read.get("tables"), Some(clarity_formats::tree::Tree::List(_)));
        let read: Vec<clarity_formats::tree::Tree> = match read.get("tables") {
            Some(clarity_formats::tree::Tree::List(list)) => list.clone(),
            _ => vec![read],
        };
        let mut db = Sqlite::new(rusqlite::Connection::open_in_memory()?)?;
        let mut tables = Vec::new();
        // The keys of each table's rows, in the order read, so a link to a place becomes a key.
        let mut keys: Vec<(String, Vec<i64>)> = Vec::new();
        for given in &read {
            let name = given.get("name").and_then(|n| n.as_text()).unwrap_or("").to_string();
            let table = clarity_formats::table::Table::from_json(&clarity_formats::tree::write(given)).ok_or_else(|| storage("the table read is no table".into()))?;
            let (sql_table, key, kinds) = if name.is_empty() {
                (source.table.to_string(), source.key.to_string(), &all_fields)
            } else {
                let field = records_field(&all_fields, &name).ok_or_else(|| storage(format!("the records inside '{name}' are no field of the fields given")))?;
                let text = |k: &str| field.get(k).and_then(|v| v.as_text()).unwrap_or("").to_string();
                (text("table"), text("key"), field.get("fields").unwrap_or(&all_fields))
            };
            let linked = !name.is_empty();
            let kind_of = |column: &str| -> &'static str {
                let clarity_formats::tree::Tree::List(fields) = kinds else { return "" };
                fields
                    .iter()
                    .find(|f| f.get("name").and_then(|n| n.as_text()).is_some_and(|n| clarity_formats::fit::plain(n) == clarity_formats::fit::plain(column)))
                    .and_then(|f| f.get("kind").and_then(|k| k.as_text()))
                    .map(column_type)
                    .unwrap_or("")
            };
            let keyed = table.columns.iter().any(|c| *c == key);
            let mut columns: Vec<String> = Vec::new();
            if !keyed {
                // The file has no key column: the row's place keys it, never written back.
                columns.push(format!("{} INTEGER PRIMARY KEY", quote(&key)));
            }
            for (i, column) in table.columns.iter().enumerate() {
                let ty = if *column == key {
                    " INTEGER PRIMARY KEY"
                } else if linked && i == 1 {
                    " INTEGER"
                } else {
                    kind_of(column)
                };
                columns.push(format!("{}{ty}", quote(column)));
            }
            db.batch(&format!("CREATE TABLE {} ({})", quote(&sql_table), columns.join(", "))).map_err(Error::from)?;
            let names = table.columns.iter().map(|c| quote(c)).collect::<Vec<_>>().join(", ");
            let marks = vec!["?"; table.columns.len()].join(", ");
            let insert = format!("INSERT INTO {} ({names}) VALUES ({marks})", quote(&sql_table));
            let parent_keys = keys.iter().find(|(n, _)| linked && n == parent_of(&name)).map(|(_, k)| k.clone()).unwrap_or_default();
            let mut inserted = Vec::with_capacity(table.rows.len());
            for (i, row) in table.rows.iter().enumerate() {
                let mut values: Vec<SqlValue> = row.iter().map(cell).collect();
                if linked && values.len() > 1 {
                    // The place of the row it sits in, from 1, is that row's key.
                    values[1] = match &values[1] {
                        SqlValue::Integer(place) if *place >= 1 => parent_keys.get(*place as usize - 1).map_or(SqlValue::Null, |k| SqlValue::Integer(*k)),
                        _ => SqlValue::Null,
                    };
                }
                let id = db.insert(&insert, &values).map_err(|f| storage(format!("row {}: {}", i + 1, Error::from(f))))?;
                inserted.push(id);
            }
            keys.push((name.clone(), inserted));
            tables.push(Kept { name, table: sql_table, key, columns: table.columns, linked });
        }
        Ok(FileDriver {
            db,
            path: path.to_path_buf(),
            format: source.format.to_string(),
            options: source.options.to_string(),
            fields: source.fields.to_string(),
            tables,
            several,
            original,
            readonly: source.readonly,
            dirty: false,
        })
    }

    /// The tables as the file takes them back: each table's rows in their
    /// order, and a link turned back into the place of its row among the
    /// rows of the table it sits in.
    fn tables_json(&mut self) -> Outcome<String> {
        use clarity_formats::tree::Tree;
        let mut places: Vec<(String, std::collections::HashMap<i64, usize>)> = Vec::new();
        let mut out = Vec::new();
        for kept in &self.tables {
            let names = kept.columns.iter().map(|c| quote(c)).collect::<Vec<_>>().join(", ");
            let rows = self.db.rows(&format!("SELECT {}, {names} FROM {} ORDER BY rowid", quote(&kept.key), quote(&kept.table)), &[])?;
            let parent = places.iter().find(|(n, _)| kept.linked && n == parent_of(&kept.name)).map(|(_, p)| p);
            let mut own = std::collections::HashMap::new();
            let mut cells = Vec::with_capacity(rows.len());
            for (i, row) in rows.iter().enumerate() {
                if let SqlValue::Integer(key) = row[0] {
                    own.insert(key, i + 1);
                }
                let mut line: Vec<Tree> = row[1..].iter().map(tree).collect();
                if kept.linked && line.len() > 1 {
                    let place = match &row[2] {
                        SqlValue::Integer(key) => parent.and_then(|p| p.get(key)),
                        _ => None,
                    };
                    line[1] = place.map_or(Tree::Null, |p| Tree::Number(p.to_string()));
                }
                cells.push(line);
            }
            places.push((kept.name.clone(), own));
            let table = clarity_formats::table::Table { columns: kept.columns.clone(), rows: cells };
            out.push((kept.name.clone(), table));
        }
        if !self.several {
            return Ok(out.remove(0).1.to_json());
        }
        let list = out
            .into_iter()
            .map(|(name, table)| {
                let mut tree = clarity_formats::tree::parse(&table.to_json()).unwrap_or(Tree::Object(Vec::new()));
                tree.set("name", Tree::Text(name));
                tree
            })
            .collect();
        Ok(clarity_formats::tree::write(&Tree::Object(vec![("tables".into(), Tree::List(list))])))
    }

    /// Writes the table back when a change is kept: to a new file beside the old one, which then replaces it.
    fn keep(&mut self) -> Outcome<()> {
        if !self.dirty || !self.db.0.is_autocommit() || self.readonly {
            return Ok(());
        }
        self.dirty = false;
        let tables = self.tables_json()?;
        let bytes = clarity_formats::write_table(&self.format, &self.original, &tables, &self.options, &self.fields).map_err(|p| Failure::Other(p.text))?;
        if let Some(folder) = self.path.parent().filter(|f| !f.as_os_str().is_empty()) {
            std::fs::create_dir_all(folder).map_err(|e| Failure::Other(format!("cannot make {}: {e}", folder.display())))?;
        }
        let mut new = self.path.clone().into_os_string();
        new.push(".new");
        let new = PathBuf::from(new);
        std::fs::write(&new, &bytes).map_err(|e| Failure::Other(format!("cannot write {}: {e}", new.display())))?;
        std::fs::rename(&new, &self.path).map_err(|e| Failure::Other(format!("cannot replace {}: {e}", self.path.display())))?;
        self.original = bytes;
        Ok(())
    }
}

impl Driver for FileDriver {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }

    fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Outcome<u64> {
        let changed = self.db.execute(sql, params)?;
        self.dirty |= changed > 0;
        self.keep()?;
        Ok(changed)
    }

    fn insert(&mut self, sql: &str, params: &[SqlValue]) -> Outcome<Id> {
        let id = self.db.insert(sql, params)?;
        self.dirty = true;
        self.keep()?;
        Ok(id)
    }

    fn rows(&mut self, sql: &str, params: &[SqlValue]) -> Outcome<Vec<Vec<SqlValue>>> {
        self.db.rows(sql, params)
    }

    /// A transaction's end keeps what it changed.
    fn batch(&mut self, sql: &str) -> Outcome<()> {
        self.db.batch(sql)?;
        self.keep()
    }

    /// A file source is external: Clarity writes no steps for it.
    fn apply(&mut self, _version: usize, _statements: &[&str]) -> Outcome<()> {
        Ok(())
    }
}
