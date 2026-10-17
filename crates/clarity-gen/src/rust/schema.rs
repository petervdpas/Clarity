//! The migration steps as SQL, in the dialect of each data source. Each
//! step of the project's `migrations` folder becomes a list of statements;
//! the runtime applies the steps a database has not had yet, in order. A
//! project loaded without its folder (sources in memory) gets the whole
//! dictionary as its one step.

use clarity_spec::Project;
use clarity_spec::migrate::{self, Column, Op, Schema, Table};
use clarity_spec::model::FieldType;

use super::HEADER;
use super::ddl::Sql;
use super::names::table;

/// Writes SQL for the steps, with what the schema is before each change.
struct Writer<'a> {
    project: &'a Project,
    /// The data source whose database the steps build.
    source: &'a str,
    sql: Sql,
    schema: Schema,
    /// Writing a baseline step: create only what is missing.
    baseline: bool,
    /// The step's statements so far.
    out: Vec<String>,
    /// Reference fields whose foreign key is added at the end of the step,
    /// when every table it can point at exists: entity and field.
    references: Vec<(String, String)>,
    errors: Vec<String>,
}

impl Writer<'_> {
    fn enum_first(&self, name: &str) -> Option<String> {
        self.project.enum_decl(name).and_then(|d| d.variants.first()).map(|v| v.name.name.clone())
    }

    fn q(&self, name: &str) -> String {
        self.sql.quote(name)
    }

    fn push(&mut self, statement: String) {
        self.out.push(statement);
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

    /// Whether `column` refers to an entity in this database; a reference
    /// into another database is a plain id.
    fn local_target<'c>(&self, column: &'c Column) -> Option<&'c str> {
        match &column.ty {
            FieldType::Ref { entity } if self.project.source_of(&entity.name).is_none_or(|s| s == self.source) => Some(&entity.name),
            _ => None,
        }
    }

    /// A column definition. SQLite writes the reference in it; the server
    /// databases add it at the end of the step.
    fn column_sql(&self, column: &Column) -> String {
        let (ty, default) = self.sql.storage(column, |e| self.enum_first(e));
        let name = self.q(column.stored());
        match (self.local_target(column), default) {
            (Some(entity), _) if self.sql == Sql::Sqlite => {
                let (target, key) = self.target(entity);
                format!("{name} {ty} REFERENCES {} ({})", self.q(&target), self.q(&key))
            }
            (_, Some(default)) => format!("{name} {ty} NOT NULL DEFAULT {default}"),
            (_, None) => format!("{name} {ty}"),
        }
    }

    /// Remembers the foreign key of a new reference column.
    fn refer(&mut self, entity: &str, column: &Column) {
        if self.sql != Sql::Sqlite && self.local_target(column).is_some() {
            self.references.push((entity.to_string(), column.name.clone()));
        }
    }

    fn check_unique(&mut self, entity: &str, column: &Column) {
        if column.unique
            && let Some(problem) = self.sql.not_unique(column)
        {
            self.errors.push(format!("{entity}: {problem}"));
        }
    }

    /// CREATE TABLE for `table` under the name `table_name`, without its
    /// indexes. A baseline creates only what is missing.
    fn table_sql(&self, table: &Table, table_name: &str) -> String {
        let mut lines = vec![self.sql.key(table.key_column())];
        lines.extend(table.columns.iter().map(|c| self.column_sql(c)));
        let missing = if self.baseline { " IF NOT EXISTS" } else { "" };
        format!("CREATE TABLE{missing} {} (\n    {}\n);", self.q(table_name), lines.join(",\n    "))
    }

    /// A table with its unique indexes.
    fn create_table(&mut self, table: &Table) {
        for column in &table.columns {
            self.check_unique(&table.name, column);
            self.refer(&table.name, column);
        }
        self.push(self.table_sql(table, &table.table_name()));
        self.unique_indexes(table);
    }

    fn unique_indexes(&mut self, table: &Table) {
        for column in table.columns.iter().filter(|c| c.unique) {
            self.push(self.sql.unique_index(table, column, self.baseline));
        }
    }

    fn drop_uniques(&mut self, table: &Table) {
        for column in table.columns.iter().filter(|c| c.unique) {
            self.push(self.sql.drop_unique(table, column));
        }
    }

    /// SQLite cannot drop or retype a column in place everywhere, so the
    /// table is built again with the new shape and the rows copied over,
    /// converting values where the stored type changed.
    fn rebuild(&mut self, before: &Table, after: &Table) {
        let name = after.table_name();
        let fresh = format!("{name}__new");
        self.drop_uniques(before);
        self.push(self.table_sql(after, &fresh));
        let mut targets = vec![self.q(after.key_column())];
        let mut sources = vec![self.q(before.key_column())];
        for column in &after.columns {
            let Some(old) = before.column(&column.name) else { continue };
            targets.push(self.q(column.stored()));
            sources.push(self.converted(old, column));
        }
        self.push(format!(
            "INSERT INTO {} ({}) SELECT {} FROM {};",
            self.q(&fresh),
            targets.join(", "),
            sources.join(", "),
            self.q(&before.table_name())
        ));
        self.push(format!("DROP TABLE {};", self.q(&before.table_name())));
        self.push(format!("ALTER TABLE {} RENAME TO {};", self.q(&fresh), self.q(&name)));
        self.unique_indexes(after);
    }

    /// An old column's value as the new column stores it, or None when it
    /// starts empty: a reference that points at another entity now.
    fn conversion(&self, old: &Column, new: &Column) -> Option<String> {
        let name = self.q(old.stored());
        let (old_ty, _) = self.sql.storage(old, |e| self.enum_first(e));
        let (new_ty, _) = self.sql.storage(new, |e| self.enum_first(e));
        match (&old.ty, &new.ty) {
            (FieldType::Ref { entity: a }, FieldType::Ref { entity: b }) if a.name == b.name => Some(name),
            (_, FieldType::Ref { .. }) => None,
            _ if old_ty == new_ty => Some(name),
            _ if self.sql == Sql::Sqlite => Some(format!("CAST({name} AS {new_ty})")),
            _ => Some(self.sql.convert(&name, &old.ty, &new.ty, &new_ty)),
        }
    }

    /// `conversion`, with the default for values that are missing.
    fn converted(&self, old: &Column, new: &Column) -> String {
        let (_, default) = self.sql.storage(new, |e| self.enum_first(e));
        let value = self.conversion(old, new).unwrap_or_else(|| "NULL".to_string());
        match default {
            Some(default) => format!("COALESCE({value}, {default})"),
            None => value,
        }
    }

    /// Drops what depends on a column before it goes or changes: its
    /// references and its unique index.
    fn release(&mut self, table: &Table, column: &Column) {
        if self.local_target(column).is_some() {
            for statement in self.sql.drop_foreign_keys(&table.table_name(), column.stored()) {
                self.push(statement);
            }
        }
        if column.unique {
            self.push(self.sql.drop_unique(table, column));
        }
    }

    fn remove_column(&mut self, before: &Table, column: &Column) {
        self.release(before, column);
        for statement in self.sql.drop_column(&before.table_name(), column.stored()) {
            self.push(statement);
        }
    }

    /// The server databases change a column's type through a new column:
    /// add it, copy the values over converted, drop the old one and give
    /// the new one its name.
    fn retype(&mut self, entity: &str, before: &Table, after: &Table, field: &str) {
        let (Some(old), Some(new)) = (before.column(field), after.column(field)) else { return };
        let table = after.table_name();
        let fresh = Column {
            column: Some(format!("{}__new", new.stored())),
            unique: false,
            ..new.clone()
        };
        self.release(before, old);
        self.push(self.sql.add_column(&table, &self.column_sql(&fresh)));
        if self.conversion(old, new).is_some() {
            self.push(format!(
                "UPDATE {} SET {} = {};",
                self.q(&table),
                self.q(fresh.stored()),
                self.converted(old, new)
            ));
        }
        for statement in self.sql.drop_column(&table, old.stored()) {
            self.push(statement);
        }
        self.push(self.sql.rename_column(&table, fresh.stored(), new.stored()));
        if new.unique {
            self.check_unique(entity, new);
            self.push(self.sql.unique_index(after, new, false));
        }
        self.refer(entity, new);
    }

    /// Field `from` before the step is field `to` after it; its column is
    /// renamed when the stored name changed.
    fn moved_column(&mut self, before: &Table, after: &Table, from: &str, to: &str) {
        if let (Some(was), Some(now)) = (before.column(from), after.column(to)) {
            self.rename_column(before, after, was, now);
        }
    }

    /// Renames column `from` of `table` to `to`, keeping its unique index.
    fn rename_column(&mut self, table_before: &Table, table_after: &Table, from: &Column, to: &Column) {
        if from.stored() == to.stored() {
            return;
        }
        if from.unique {
            self.push(self.sql.drop_unique(table_before, from));
        }
        self.push(self.sql.rename_column(&table_after.table_name(), from.stored(), to.stored()));
        if to.unique {
            self.push(self.sql.unique_index(table_after, to, self.baseline));
        }
    }

    /// Renames the table of an entity, keeping its unique indexes.
    fn rename_table(&mut self, before: &Table, after: &Table) {
        if before.table_name() == after.table_name() {
            return;
        }
        self.drop_uniques(before);
        self.push(self.sql.rename_table(&before.table_name(), &after.table_name()));
        self.unique_indexes(after);
    }

    fn step(&mut self, op: &Op) {
        let before = self.schema.clone();
        if let Err(message) = self.schema.apply(op) {
            // The checker reports steps that do not fit; nothing to write.
            self.push(format!("-- skipped: {message}"));
            return;
        }
        let after = |name: &str| self.schema.table(name).cloned().expect("applied above");
        let old = |name: &str| before.table(name).cloned().expect("checked by apply");
        match op {
            Op::AddEntity { name, .. } => self.create_table(&after(name)),
            Op::RemoveEntity { name } => self.push(format!("DROP TABLE {};", self.q(&old(name).table_name()))),
            Op::RenameEntity { from, to } => self.rename_table(&old(from), &after(to)),
            Op::MapTable { entity, .. } => self.rename_table(&old(entity), &after(entity)),
            Op::MapKey { entity, .. } => {
                let (b, a) = (old(entity), after(entity));
                if b.key_column() != a.key_column() {
                    self.push(self.sql.rename_column(&a.table_name(), b.key_column(), a.key_column()));
                }
            }
            Op::AddField { entity, column } => {
                let table_after = after(entity);
                let column = table_after.column(&column.name).cloned().expect("applied above");
                self.check_unique(entity, &column);
                self.push(self.sql.add_column(&table_after.table_name(), &self.column_sql(&column)));
                if column.unique {
                    self.push(self.sql.unique_index(&table_after, &column, self.baseline));
                }
                self.refer(entity, &column);
            }
            Op::RenameField { entity, from, to } => self.moved_column(&old(entity), &after(entity), from, to),
            Op::MapColumn { entity, field, .. } => self.moved_column(&old(entity), &after(entity), field, field),
            Op::RemoveField { entity, .. } | Op::ChangeField { entity, .. } if self.sql == Sql::Sqlite => {
                self.rebuild(&old(entity), &after(entity))
            }
            Op::RemoveField { entity, field } => {
                let table = old(entity);
                let column = table.column(field).cloned().expect("checked by apply");
                self.remove_column(&table, &column);
            }
            Op::ChangeField { entity, field, .. } => self.retype(entity, &old(entity), &after(entity), field),
            Op::AddUnique { entity, field } => {
                let table_after = after(entity);
                let column = table_after.column(field).cloned().expect("applied above");
                self.check_unique(entity, &column);
                self.push(self.sql.unique_index(&table_after, &column, self.baseline));
            }
            Op::RemoveUnique { entity, field } => {
                let table_before = old(entity);
                let column = table_before.column(field).expect("checked by apply");
                self.push(self.sql.drop_unique(&table_before, column));
            }
        }
    }

    /// The statements of one step, with the foreign keys of its new
    /// references last.
    fn write(&mut self, ops: &[Op]) -> Vec<String> {
        for op in ops {
            self.step(op);
        }
        for (entity, field) in std::mem::take(&mut self.references) {
            let Some(table) = self.schema.table(&entity) else { continue };
            let Some(column) = table.column(&field) else { continue };
            let Some(target) = self.local_target(column) else { continue };
            let (target, key) = self.target(target);
            let statement = self.sql.add_foreign_key(&table.table_name(), column.stored(), &target, &key);
            self.out.push(statement);
        }
        std::mem::take(&mut self.out)
    }
}

/// A step to write: its title, whether it is a baseline, and its changes.
type Step = (String, bool, Vec<Op>);

/// The steps of `source`, or why they cannot be written yet.
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

/// The steps of data source `source` as statements in `sql`, each step
/// with its title.
pub fn steps(project: &Project, source: &str, sql: Sql) -> Result<Vec<(String, Vec<String>)>, Vec<String>> {
    let steps = source_steps(project, source).map_err(|e| vec![e])?;
    let mut writer = Writer {
        project,
        source,
        sql,
        schema: Schema::default(),
        baseline: false,
        out: Vec::new(),
        references: Vec::new(),
        errors: Vec::new(),
    };
    let mut out = Vec::new();
    for (title, baseline, ops) in &steps {
        if *baseline && sql != Sql::Sqlite {
            writer.errors.push(format!(
                "data source '{source}': a baseline step is for SQLite only for now, and {} is not",
                sql.label()
            ));
        }
        writer.baseline = *baseline;
        out.push((title.clone(), writer.write(ops)));
    }
    if writer.errors.is_empty() { Ok(out) } else { Err(writer.errors) }
}

/// SQL as a Rust string literal that needs no escapes.
fn raw(text: &str) -> String {
    let mut hashes = "#".to_string();
    while text.contains(&format!("\"{hashes}")) {
        hashes.push('#');
    }
    format!("r{hashes}\"{text}\"{hashes}")
}

/// The steps of every database of the application, as SQL per data source.
pub fn emit(project: &Project, sources: &[(String, Sql)]) -> Result<String, Vec<String>> {
    let mut out = format!(
        "{HEADER}
/// The migration steps per data source, in order, each a list of
/// statements. Each database remembers how many steps it has applied, and
/// applies the rest when the application starts.
pub const SOURCES: &[(&str, &[&[&str]])] = &[
"
    );
    let mut errors = Vec::new();
    for (source, sql) in sources {
        let steps = match steps(project, source, *sql) {
            Ok(steps) => steps,
            Err(mut problems) => {
                errors.append(&mut problems);
                continue;
            }
        };
        out.push_str(&format!("    ({source:?}, &[\n"));
        for (i, (title, statements)) in steps.iter().enumerate() {
            out.push_str(&format!("        // {}: {title}\n        &[\n", i + 1));
            for statement in statements {
                out.push_str(&format!("            {},\n", raw(statement)));
            }
            out.push_str("        ],\n");
        }
        out.push_str("    ]),\n");
    }
    out.push_str("];\n");
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}
