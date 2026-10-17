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
    /// Writing a baseline step: create only what is missing.
    baseline: bool,
}

impl Writer<'_> {
    fn enum_first(&self, name: &str) -> Option<String> {
        self.project.enum_decl(name).and_then(|d| d.variants.first()).map(|v| v.name.name.clone())
    }

    /// The table and key column of entity `name`: as the steps made it, or
    /// as the dictionary has it when the steps do not know it.
    fn target(&self, name: &str) -> (String, String) {
        match (self.schema.table(name), self.project.entity(name)) {
            (Some(table), _) => (table.table_name(), table.key_column().to_string()),
            (None, Some(entity)) => (entity.table_name(), entity.key_column().to_string()),
            (None, None) => (table(name), "id".to_string()),
        }
    }

    fn column_sql(&self, column: &Column) -> String {
        let (ty, default) = storage(column, |e| self.enum_first(e));
        let name = quoted(column.stored());
        match (&column.ty, default) {
            // A reference into another database is a plain id.
            (FieldType::Ref { entity }, _) if self.project.source_of(&entity.name).is_none_or(|s| s == self.source) => {
                let (target, key) = self.target(&entity.name);
                format!("{name} INTEGER REFERENCES {} ({})", quoted(&target), quoted(&key))
            }
            (FieldType::Ref { .. }, _) => format!("{name} INTEGER"),
            (_, Some(default)) => format!("{name} {ty} NOT NULL DEFAULT {default}"),
            (_, None) => format!("{name} {ty}"),
        }
    }

    /// CREATE TABLE for `table` under the name `table_name`, without its
    /// indexes. A baseline creates only what is missing.
    fn table_sql(&self, table: &Table, table_name: &str) -> String {
        let mut lines = vec![format!("{} INTEGER PRIMARY KEY", quoted(table.key_column()))];
        lines.extend(table.columns.iter().map(|c| self.column_sql(c)));
        let missing = if self.baseline { " IF NOT EXISTS" } else { "" };
        format!("CREATE TABLE{missing} {} (\n    {}\n);\n", quoted(table_name), lines.join(",\n    "))
    }

    /// A table with its unique indexes.
    fn create_table(&self, table: &Table) -> String {
        let mut sql = self.table_sql(table, &table.table_name());
        sql.push_str(&self.unique_indexes(table));
        sql
    }

    fn unique_indexes(&self, table: &Table) -> String {
        table.columns.iter().filter(|c| c.unique).map(|c| unique_index(table, c, self.baseline)).collect()
    }

    fn drop_uniques(table: &Table) -> String {
        table.columns.iter().filter(|c| c.unique).map(|c| drop_unique(table, c)).collect()
    }

    /// SQLite cannot drop or retype a column in place everywhere, so the
    /// table is built again with the new shape and the rows copied over,
    /// converting values where the stored type changed.
    fn rebuild(&self, before: &Table, after: &Table) -> String {
        let name = after.table_name();
        let fresh = format!("{name}__new");
        let mut sql = Self::drop_uniques(before);
        sql.push_str(&self.table_sql(after, &fresh));
        let mut targets = vec![quoted(after.key_column())];
        let mut sources = vec![quoted(before.key_column())];
        for column in &after.columns {
            let Some(old) = before.column(&column.name) else { continue };
            targets.push(quoted(column.stored()));
            sources.push(self.converted(old, column));
        }
        sql.push_str(&format!(
            "INSERT INTO {} ({}) SELECT {} FROM {};\nDROP TABLE {};\nALTER TABLE {} RENAME TO {};\n",
            quoted(&fresh),
            targets.join(", "),
            sources.join(", "),
            quoted(&before.table_name()),
            quoted(&before.table_name()),
            quoted(&fresh),
            quoted(&name)
        ));
        sql.push_str(&self.unique_indexes(after));
        sql
    }

    /// An old column's value as the new column stores it.
    fn converted(&self, old: &Column, new: &Column) -> String {
        let name = quoted(old.stored());
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

    /// Field `from` before the step is field `to` after it; its column is
    /// renamed when the stored name changed.
    fn moved_column(&self, before: &Table, after: &Table, from: &str, to: &str) -> String {
        match (before.column(from), after.column(to)) {
            (Some(was), Some(now)) => Self::rename_column(before, after, was, now, self.baseline),
            _ => String::new(),
        }
    }

    /// Renames column `from` of `table` to `to`, keeping its unique index.
    fn rename_column(table_before: &Table, table_after: &Table, from: &Column, to: &Column, baseline: bool) -> String {
        if from.stored() == to.stored() {
            return String::new();
        }
        let mut sql = if from.unique { drop_unique(table_before, from) } else { String::new() };
        sql.push_str(&format!(
            "ALTER TABLE {} RENAME COLUMN {} TO {};\n",
            quoted(&table_after.table_name()),
            quoted(from.stored()),
            quoted(to.stored())
        ));
        if to.unique {
            sql.push_str(&unique_index(table_after, to, baseline));
        }
        sql
    }

    /// Renames the table of an entity, keeping its unique indexes.
    fn rename_table(&self, before: &Table, after: &Table) -> String {
        if before.table_name() == after.table_name() {
            return String::new();
        }
        let mut sql = Self::drop_uniques(before);
        sql.push_str(&format!(
            "ALTER TABLE {} RENAME TO {};\n",
            quoted(&before.table_name()),
            quoted(&after.table_name())
        ));
        sql.push_str(&self.unique_indexes(after));
        sql
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
            Op::AddEntity { name, .. } => self.create_table(&after(name)),
            Op::RemoveEntity { name } => format!("DROP TABLE {};\n", quoted(&old(name).table_name())),
            Op::RenameEntity { from, to } => self.rename_table(&old(from), &after(to)),
            Op::MapTable { entity, .. } => self.rename_table(&old(entity), &after(entity)),
            Op::MapKey { entity, .. } => {
                let (b, a) = (old(entity), after(entity));
                if b.key_column() == a.key_column() {
                    return String::new();
                }
                format!(
                    "ALTER TABLE {} RENAME COLUMN {} TO {};\n",
                    quoted(&a.table_name()),
                    quoted(b.key_column()),
                    quoted(a.key_column())
                )
            }
            Op::AddField { entity, column } => {
                let table_after = after(entity);
                let mut sql = format!(
                    "ALTER TABLE {} ADD COLUMN {};\n",
                    quoted(&table_after.table_name()),
                    self.column_sql(column)
                );
                if column.unique {
                    sql.push_str(&unique_index(&table_after, column, self.baseline));
                }
                sql
            }
            Op::RenameField { entity, from, to } => self.moved_column(&old(entity), &after(entity), from, to),
            Op::MapColumn { entity, field, .. } => self.moved_column(&old(entity), &after(entity), field, field),
            Op::RemoveField { entity, .. } | Op::ChangeField { entity, .. } => self.rebuild(&old(entity), &after(entity)),
            Op::AddUnique { entity, field } => {
                let table_after = after(entity);
                let column = table_after.column(field).expect("applied above");
                unique_index(&table_after, column, self.baseline)
            }
            Op::RemoveUnique { entity, field } => {
                let table_before = old(entity);
                let column = table_before.column(field).expect("checked by apply");
                drop_unique(&table_before, column)
            }
        }
    }
}

fn index_name(table: &Table, column: &Column) -> String {
    quoted(&format!("{}_{}_unique", table.table_name(), column.stored()))
}

/// A unique field gets a unique index. Empty text is no value, so for text
/// the index leaves empty text out and many records can leave it empty.
fn unique_index(table: &Table, column: &Column, baseline: bool) -> String {
    let name = quoted(column.stored());
    let textual = matches!(column.ty, FieldType::Text { .. } | FieldType::Email | FieldType::Url);
    let filter = if textual { format!(" WHERE {name} != ''") } else { String::new() };
    let missing = if baseline { " IF NOT EXISTS" } else { "" };
    format!(
        "CREATE UNIQUE INDEX{missing} {} ON {} ({name}){filter};\n",
        index_name(table, column),
        quoted(&table.table_name())
    )
}

fn drop_unique(table: &Table, column: &Column) -> String {
    format!("DROP INDEX IF EXISTS {};\n", index_name(table, column))
}

/// The steps of `source` as SQL, or why they cannot be written yet.
/// A step to write: its title, whether it is a baseline, and its changes.
type Step = (String, bool, Vec<Op>);

fn source_steps(project: &Project, source: &str) -> Result<Vec<Step>, String> {
    let Some(migrations) = &project.migrations else {
        return Ok(vec![("the schema from the data dictionary".to_string(), false, migrate::pending(project, source, &[]))]);
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
        .map(|m| (m.title.clone(), m.baseline, m.ops.iter().map(|(op, _)| op.clone()).collect()))
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
            baseline: false,
        };
        out.push_str(&format!("    ({source:?}, &[\n"));
        for (i, (title, baseline, ops)) in steps.iter().enumerate() {
            writer.baseline = *baseline;
            let sql: String = ops.iter().map(|op| writer.step(op)).collect();
            out.push_str(&format!("        // {}: {title}\n        r#\"{sql}\"#,\n", i + 1));
        }
        out.push_str("    ]),\n");
    }
    out.push_str("];\n");
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}
