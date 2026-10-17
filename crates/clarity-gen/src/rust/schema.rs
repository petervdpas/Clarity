//! The migration steps as SQLite SQL. Each step of the project's
//! `migrations` folder becomes one string; the runtime applies the ones a
//! database has not had yet, in order. A project loaded without its folder
//! (sources in memory) gets the whole dictionary as its one step.

use clarity_spec::Project;
use clarity_spec::migrate::{self, Column, Op, Schema, Table};
use clarity_spec::model::{FieldType, Literal};

use super::HEADER;
use super::names::table;

fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn sql_text(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// How SQLite stores a field: its type and, for fields that always have a
/// value, the value a row gets when it has none. `enum_first` gives the
/// first value of an enum, for enum fields without a default.
fn storage(column: &Column, enum_first: impl Fn(&str) -> Option<String>) -> (&'static str, Option<String>) {
    let default = column.default.as_ref();
    match (&column.ty, default) {
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url, Some(Literal::Str(s))) => ("TEXT", Some(sql_text(s))),
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url, _) => ("TEXT", Some("''".into())),
        (FieldType::Integer | FieldType::Id, Some(Literal::Number(n))) => ("INTEGER", Some(n.clone())),
        (FieldType::Integer | FieldType::Id, _) => ("INTEGER", Some("0".into())),
        // Decimals are text, so they never lose precision.
        (FieldType::Decimal { .. } | FieldType::Money, Some(Literal::Number(n))) => ("TEXT", Some(sql_text(n))),
        (FieldType::Decimal { .. } | FieldType::Money, _) => ("TEXT", Some("'0'".into())),
        (FieldType::Bool, Some(Literal::Bool(b))) => ("INTEGER", Some(u8::from(*b).to_string())),
        (FieldType::Bool, _) => ("INTEGER", Some("0".into())),
        (FieldType::Enum { name }, default) => {
            let value = match default {
                Some(Literal::Ident(v)) => Some(v.clone()),
                _ => enum_first(&name.name),
            };
            ("TEXT", Some(sql_text(&value.unwrap_or_default())))
        }
        (FieldType::Date | FieldType::DateTime, _) => ("TEXT", None),
        (FieldType::Ref { .. }, _) => ("INTEGER", None),
        (FieldType::List { .. }, _) => ("", None),
    }
}

/// Writes SQL for the steps, with what the schema is before each change.
struct Writer<'a> {
    project: &'a Project,
    /// The data source whose database the steps build.
    source: &'a str,
    schema: Schema,
}

impl Writer<'_> {
    fn enum_first(&self, name: &str) -> Option<String> {
        self.project.enum_decl(name).and_then(|d| d.variants.first()).map(|v| v.name.name.clone())
    }

    fn column_sql(&self, column: &Column) -> String {
        let (ty, default) = storage(column, |e| self.enum_first(e));
        let name = quoted(&column.name);
        match (&column.ty, default) {
            // A reference into another database is a plain id.
            (FieldType::Ref { entity }, _) if self.project.source_of(&entity.name).is_none_or(|s| s == self.source) => {
                format!("{name} INTEGER REFERENCES {} (\"id\")", quoted(&table(&entity.name)))
            }
            (FieldType::Ref { .. }, _) => format!("{name} INTEGER"),
            (_, Some(default)) => format!("{name} {ty} NOT NULL DEFAULT {default}"),
            (_, None) => format!("{name} {ty}"),
        }
    }

    fn table_sql(&self, table_name: &str, columns: &[Column]) -> String {
        let mut lines = vec!["\"id\" INTEGER PRIMARY KEY".to_string()];
        lines.extend(columns.iter().map(|c| self.column_sql(c)));
        format!("CREATE TABLE {} (\n    {}\n);\n", quoted(table_name), lines.join(",\n    "))
    }

    /// A table of entity `name` with its unique indexes.
    fn create_table(&self, name: &str, columns: &[Column]) -> String {
        let mut sql = self.table_sql(&table(name), columns);
        for column in columns.iter().filter(|c| c.unique) {
            sql.push_str(&unique_index(name, column));
        }
        sql
    }

    /// SQLite cannot drop or retype a column in place everywhere, so the
    /// table is built again with the new shape and the rows copied over,
    /// converting values where the stored type changed.
    fn rebuild(&self, before: &Table, after: &Table) -> String {
        let name = table(&after.name);
        let fresh = format!("{name}__new");
        let mut sql = String::new();
        for column in before.columns.iter().filter(|c| c.unique) {
            sql.push_str(&drop_unique(&before.name, &column.name));
        }
        sql.push_str(&self.table_sql(&fresh, &after.columns));
        let mut targets = vec!["\"id\"".to_string()];
        let mut sources = vec!["\"id\"".to_string()];
        for column in &after.columns {
            let Some(old) = before.column(&column.name) else { continue };
            targets.push(quoted(&column.name));
            sources.push(self.converted(old, column));
        }
        sql.push_str(&format!(
            "INSERT INTO {} ({}) SELECT {} FROM {};\nDROP TABLE {};\nALTER TABLE {} RENAME TO {};\n",
            quoted(&fresh),
            targets.join(", "),
            sources.join(", "),
            quoted(&name),
            quoted(&name),
            quoted(&fresh),
            quoted(&name)
        ));
        for column in after.columns.iter().filter(|c| c.unique) {
            sql.push_str(&unique_index(&after.name, column));
        }
        sql
    }

    /// An old column's value as the new column stores it.
    fn converted(&self, old: &Column, new: &Column) -> String {
        let name = quoted(&old.name);
        let (old_ty, _) = storage(old, |e| self.enum_first(e));
        let (new_ty, default) = storage(new, |e| self.enum_first(e));
        let value = match (&old.ty, &new.ty) {
            (FieldType::Ref { entity: a }, FieldType::Ref { entity: b }) if a.name == b.name => name,
            (_, FieldType::Ref { .. }) => "NULL".to_string(),
            _ if old_ty == new_ty => name,
            _ => format!("CAST({name} AS {new_ty})"),
        };
        match default {
            Some(default) => format!("COALESCE({value}, {default})"),
            None => value,
        }
    }

    fn step(&mut self, op: &Op) -> String {
        let before = self.schema.clone();
        if let Err(message) = self.schema.apply(op) {
            // The checker reports steps that do not fit; nothing to write.
            return format!("-- skipped: {message}\n");
        }
        let after = |name: &str| self.schema.table(name).cloned().expect("applied above");
        let old = |name: &str| before.table(name).cloned().expect("checked by apply");
        match op {
            Op::AddEntity { name, columns } => self.create_table(name, columns),
            Op::RemoveEntity { name } => format!("DROP TABLE {};\n", quoted(&table(name))),
            Op::RenameEntity { from, to } => {
                let mut sql = String::new();
                let table_before = old(from);
                for column in table_before.columns.iter().filter(|c| c.unique) {
                    sql.push_str(&drop_unique(from, &column.name));
                }
                sql.push_str(&format!("ALTER TABLE {} RENAME TO {};\n", quoted(&table(from)), quoted(&table(to))));
                for column in table_before.columns.iter().filter(|c| c.unique) {
                    sql.push_str(&unique_index(to, column));
                }
                sql
            }
            Op::AddField { entity, column } => {
                let mut sql = format!("ALTER TABLE {} ADD COLUMN {};\n", quoted(&table(entity)), self.column_sql(column));
                if column.unique {
                    sql.push_str(&unique_index(entity, column));
                }
                sql
            }
            Op::RenameField { entity, from, to } => {
                let unique = old(entity).column(from).is_some_and(|c| c.unique);
                let mut sql = String::new();
                if unique {
                    sql.push_str(&drop_unique(entity, from));
                }
                sql.push_str(&format!(
                    "ALTER TABLE {} RENAME COLUMN {} TO {};\n",
                    quoted(&table(entity)),
                    quoted(from),
                    quoted(to)
                ));
                if let Some(column) = after(entity).column(to).filter(|_| unique) {
                    sql.push_str(&unique_index(entity, column));
                }
                sql
            }
            Op::RemoveField { entity, .. } | Op::ChangeField { entity, .. } => self.rebuild(&old(entity), &after(entity)),
            Op::AddUnique { entity, field } => {
                let table_after = after(entity);
                let column = table_after.column(field).expect("applied above");
                unique_index(entity, column)
            }
            Op::RemoveUnique { entity, field } => drop_unique(entity, field),
        }
    }
}

fn index_name(entity: &str, column: &str) -> String {
    quoted(&format!("{}_{column}_unique", table(entity)))
}

/// A unique field gets a unique index. Empty text is no value, so for text
/// the index leaves empty text out and many records can leave it empty.
fn unique_index(entity: &str, column: &Column) -> String {
    let name = quoted(&column.name);
    let textual = matches!(column.ty, FieldType::Text { .. } | FieldType::Email | FieldType::Url);
    let filter = if textual { format!(" WHERE {name} != ''") } else { String::new() };
    format!(
        "CREATE UNIQUE INDEX {} ON {} ({name}){filter};\n",
        index_name(entity, &column.name),
        quoted(&table(entity))
    )
}

fn drop_unique(entity: &str, column: &str) -> String {
    format!("DROP INDEX IF EXISTS {};\n", index_name(entity, column))
}

/// The steps of `source` as SQL, or why they cannot be written yet.
fn source_steps(project: &Project, source: &str) -> Result<Vec<(String, Vec<Op>)>, String> {
    let Some(migrations) = &project.migrations else {
        return Ok(vec![("the schema from the data dictionary".to_string(), migrate::pending(project, source, &[]))]);
    };
    let pending = migrate::pending(project, source, &migrate::proposed_renames(project, source));
    if !pending.is_empty() {
        let changes: Vec<String> = pending.iter().map(|op| migrate::op_text(op).lines().next().unwrap_or_default().to_string()).collect();
        return Err(format!(
            "the data dictionary has changes that are not in a migration yet ({}); create the next step in the Project tab or with 'clarity migrate'",
            changes.join("; ")
        ));
    }
    Ok(migrations
        .get(source)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|m| (m.title.clone(), m.ops.iter().map(|(op, _)| op.clone()).collect()))
        .collect())
}

/// The steps of every database of the application, as SQL per data source.
pub fn emit(project: &Project, sources: &[String]) -> Result<String, Vec<String>> {
    let mut out = format!(
        "{HEADER}
/// The migration steps per data source, in order. Each database remembers
/// how many it has applied, and applies the rest when the application
/// starts.
pub const SOURCES: &[(&str, &[&str])] = &[
"
    );
    let mut errors = Vec::new();
    for source in sources {
        let steps = match source_steps(project, source) {
            Ok(steps) => steps,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        let mut writer = Writer {
            project,
            source,
            schema: Schema::default(),
        };
        out.push_str(&format!("    ({source:?}, &[\n"));
        for (i, (title, ops)) in steps.iter().enumerate() {
            let sql: String = ops.iter().map(|op| writer.step(op)).collect();
            out.push_str(&format!("        // {}: {title}\n        r#\"{sql}\"#,\n", i + 1));
        }
        out.push_str("    ]),\n");
    }
    out.push_str("];\n");
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}
