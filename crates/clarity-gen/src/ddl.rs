//! How each database writes the statements of a migration step: its
//! names, types, defaults and the ways it changes a table. SQLite gets the
//! statements it has always had; the server databases share the shape of
//! their steps and differ in the words.

use clarity_project::SourceKind;
use clarity_spec::migrate::{Column, Table};
use clarity_spec::model::{FieldType, Literal};

/// The SQL dialect of a data source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sql {
    Sqlite,
    Postgres,
    Mysql,
    Mssql,
}

/// The longest text each server database can keep in a unique index, in
/// characters.
const MYSQL_KEY_TEXT: u32 = 768;
const MSSQL_KEY_TEXT: u32 = 850;
/// The longest text kept in the row itself, rather than as a long text.
const MSSQL_SHORT_TEXT: u32 = 4000;
/// The longest path of a picture inside its file store.
const PICTURE_PATH: u32 = 400;

impl Sql {
    /// The dialect of a data source; None for one that is not SQL.
    pub fn of(kind: SourceKind) -> Option<Sql> {
        match kind {
            SourceKind::Sqlite => Some(Sql::Sqlite),
            SourceKind::Postgres => Some(Sql::Postgres),
            SourceKind::Mysql => Some(Sql::Mysql),
            SourceKind::Mssql => Some(Sql::Mssql),
            SourceKind::Rest => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Sql::Sqlite => "SQLite",
            Sql::Postgres => "PostgreSQL",
            Sql::Mysql => "MySQL",
            Sql::Mssql => "SQL Server",
        }
    }

    /// The runtime's name for the dialect, as generated code writes it.
    pub fn runtime(self) -> &'static str {
        match self {
            Sql::Sqlite => "Dialect::Sqlite",
            Sql::Postgres => "Dialect::Postgres",
            Sql::Mysql => "Dialect::Mysql",
            Sql::Mssql => "Dialect::Mssql",
        }
    }

    /// The runtime's cargo feature that carries the driver.
    pub fn feature(self) -> &'static str {
        match self {
            Sql::Sqlite => "sqlite",
            Sql::Postgres => "postgres",
            Sql::Mysql => "mysql",
            Sql::Mssql => "mssql",
        }
    }

    pub fn quote(self, name: &str) -> String {
        match self {
            Sql::Sqlite | Sql::Postgres => format!("\"{}\"", name.replace('"', "\"\"")),
            Sql::Mysql => format!("`{}`", name.replace('`', "``")),
            Sql::Mssql => format!("[{}]", name.replace(']', "]]")),
        }
    }

    /// A text value in SQL.
    pub fn text(self, text: &str) -> String {
        let escaped = text.replace('\'', "''");
        match self {
            // MySQL reads a backslash in text as the start of an escape.
            Sql::Mysql => format!("'{}'", escaped.replace('\\', "\\\\")),
            Sql::Mssql => format!("N'{escaped}'"),
            Sql::Sqlite | Sql::Postgres => format!("'{escaped}'"),
        }
    }

    /// The column type that stores a field, and the value a row gets when
    /// it has none, for fields that always have a value. `enum_first` gives
    /// the first value of an enum, for enum fields without a default.
    pub fn storage(self, column: &Column, enum_first: impl Fn(&str) -> Option<String>) -> (String, Option<String>) {
        let default = column.default.as_ref();
        let number = |n: &str| if self == Sql::Sqlite { self.text(n) } else { n.to_string() };
        let ty = if matches!(column.ty, FieldType::Image | FieldType::File) { self.picture_type(column.store.is_some()) } else { self.column_type(&column.ty) };
        let value = match (&column.ty, default) {
            (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme | FieldType::Secret, Some(Literal::Str(s))) => Some(self.text(s)),
            (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme | FieldType::Secret, _) => Some(self.text("")),
            (FieldType::Integer | FieldType::Id | FieldType::Duration, Some(Literal::Number(n))) => Some(n.clone()),
            (FieldType::Integer | FieldType::Id | FieldType::Duration, _) => Some("0".into()),
            (FieldType::Decimal { .. } | FieldType::Money, Some(Literal::Number(n))) => Some(number(n)),
            (FieldType::Decimal { .. } | FieldType::Money, _) => Some(number("0")),
            (FieldType::Bool, Some(Literal::Bool(b))) => Some(self.boolean(*b)),
            (FieldType::Bool, _) => Some(self.boolean(false)),
            (FieldType::Enum { name }, default) => {
                let value = match default {
                    Some(Literal::Ident(v)) => Some(v.clone()),
                    _ => enum_first(&name.name),
                };
                Some(self.text(&value.unwrap_or_default()))
            }
            // A list starts empty.
            (FieldType::Values { .. }, _) => Some(self.text("[]")),
            (FieldType::Date | FieldType::DateTime | FieldType::Ref { .. } | FieldType::List { .. } | FieldType::Record { .. } | FieldType::Image | FieldType::File, _) => None,
        };
        // MySQL takes a default for long text only as an expression.
        let value = match value {
            Some(v) if self == Sql::Mysql && ty == "TEXT" => Some(format!("({v})")),
            other => other,
        };
        (ty, value)
    }

    fn boolean(self, b: bool) -> String {
        match (self, b) {
            (Sql::Postgres | Sql::Mysql, true) => "TRUE".into(),
            (Sql::Postgres | Sql::Mysql, false) => "FALSE".into(),
            (Sql::Sqlite | Sql::Mssql, b) => u8::from(b).to_string(),
        }
    }

    /// The column of a picture: the picture itself, or, for a picture in a
    /// file store, the path of its file in the store.
    fn picture_type(self, in_store: bool) -> String {
        match (self, in_store) {
            (Sql::Sqlite, true) => "TEXT".into(),
            (Sql::Mssql, true) => format!("NVARCHAR({PICTURE_PATH})"),
            (_, true) => format!("VARCHAR({PICTURE_PATH})"),
            (Sql::Sqlite, false) => "BLOB".into(),
            (Sql::Postgres, false) => "BYTEA".into(),
            (Sql::Mysql, false) => "LONGBLOB".into(),
            (Sql::Mssql, false) => "VARBINARY(MAX)".into(),
        }
    }

    fn column_type(self, ty: &FieldType) -> String {
        let decimal = |p: u32, s: u32| match self {
            Sql::Postgres => format!("NUMERIC({p},{s})"),
            _ => format!("DECIMAL({p},{s})"),
        };
        match (self, ty) {
            // SQLite keeps decimals and dates as text, so nothing is lost.
            (Sql::Sqlite, FieldType::Integer | FieldType::Id | FieldType::Duration | FieldType::Bool | FieldType::Ref { .. }) => "INTEGER".into(),
            (Sql::Sqlite, FieldType::List { .. } | FieldType::Record { .. }) => String::new(),
            (Sql::Sqlite, _) => "TEXT".into(),

            (_, FieldType::Text { max_length: Some(n) }) => match self {
                Sql::Mysql if *n > MYSQL_KEY_TEXT => "TEXT".into(),
                Sql::Mysql | Sql::Postgres => format!("VARCHAR({n})"),
                _ if *n > MSSQL_SHORT_TEXT => "NVARCHAR(MAX)".into(),
                _ => format!("NVARCHAR({n})"),
            },
            (Sql::Mssql, FieldType::Text { max_length: None } | FieldType::Url) => "NVARCHAR(MAX)".into(),
            (_, FieldType::Text { max_length: None } | FieldType::Url | FieldType::Language | FieldType::Theme | FieldType::Secret) => "TEXT".into(),
            // The longest address there can be.
            (Sql::Mssql, FieldType::Email) => "NVARCHAR(320)".into(),
            (_, FieldType::Email) => "VARCHAR(320)".into(),
            // `#rrggbb`.
            (Sql::Mssql, FieldType::Color) => "NVARCHAR(7)".into(),
            (_, FieldType::Color) => "VARCHAR(7)".into(),
            (_, FieldType::Integer | FieldType::Id | FieldType::Duration | FieldType::Ref { .. }) => "BIGINT".into(),
            (_, FieldType::Decimal { precision, scale }) => decimal(*precision, *scale),
            (_, FieldType::Money) => decimal(19, 4),
            (Sql::Mssql, FieldType::Bool) => "BIT".into(),
            (_, FieldType::Bool) => "BOOLEAN".into(),
            (_, FieldType::Date) => "DATE".into(),
            (Sql::Postgres, FieldType::DateTime) => "TIMESTAMP".into(),
            (Sql::Mysql, FieldType::DateTime) => "DATETIME".into(),
            (_, FieldType::DateTime) => "DATETIME2".into(),
            (_, FieldType::Image | FieldType::File) => self.picture_type(false),
            (Sql::Postgres, FieldType::Enum { .. }) => "TEXT".into(),
            (Sql::Mssql, FieldType::Enum { .. }) => "NVARCHAR(100)".into(),
            (_, FieldType::Enum { .. }) => "VARCHAR(100)".into(),
            (_, FieldType::List { .. } | FieldType::Record { .. }) => String::new(),
            // A list of values as JSON: `["jazz","live"]`.
            (Sql::Mssql, FieldType::Values { .. }) => "NVARCHAR(MAX)".into(),
            (_, FieldType::Values { .. }) => "TEXT".into(),
        }
    }

    /// Why a unique index cannot be made on `column`, if it cannot.
    pub fn not_unique(self, column: &Column) -> Option<String> {
        let longest = match self {
            Sql::Mysql => MYSQL_KEY_TEXT,
            Sql::Mssql => MSSQL_KEY_TEXT,
            Sql::Sqlite | Sql::Postgres => return None,
        };
        let fits = match &column.ty {
            FieldType::Text { max_length: Some(n) } => *n <= longest,
            FieldType::Text { max_length: None } | FieldType::Url => false,
            _ => true,
        };
        (!fits).then(|| {
            format!(
                "{} keeps unique text up to {longest} characters; give field '{}' a maximum length, such as text(200)",
                self.label(),
                column.name
            )
        })
    }

    /// The key column of a new table.
    pub fn key(self, name: &str) -> String {
        let name = self.quote(name);
        match self {
            Sql::Sqlite => format!("{name} INTEGER PRIMARY KEY"),
            Sql::Postgres => format!("{name} BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY"),
            Sql::Mysql => format!("{name} BIGINT AUTO_INCREMENT PRIMARY KEY"),
            Sql::Mssql => format!("{name} BIGINT IDENTITY(1,1) PRIMARY KEY"),
        }
    }

    pub fn add_column(self, table: &str, definition: &str) -> String {
        let column = if self == Sql::Mssql { "" } else { " COLUMN" };
        format!("ALTER TABLE {} ADD{column} {definition};", self.quote(table))
    }

    pub fn rename_column(self, table: &str, from: &str, to: &str) -> String {
        match self {
            Sql::Mssql => format!(
                "EXEC sp_rename {}, {}, 'COLUMN';",
                self.text(&format!("{}.{}", self.quote(table), self.quote(from))),
                self.text(to)
            ),
            _ => format!(
                "ALTER TABLE {} RENAME COLUMN {} TO {};",
                self.quote(table),
                self.quote(from),
                self.quote(to)
            ),
        }
    }

    pub fn rename_table(self, from: &str, to: &str) -> String {
        match self {
            Sql::Mysql => format!("RENAME TABLE {} TO {};", self.quote(from), self.quote(to)),
            Sql::Mssql => format!("EXEC sp_rename {}, {};", self.text(&self.quote(from)), self.text(to)),
            Sql::Sqlite | Sql::Postgres => format!("ALTER TABLE {} RENAME TO {};", self.quote(from), self.quote(to)),
        }
    }

    fn index_name(self, table: &Table, column: &Column) -> String {
        self.quote(&format!("{}_{}_unique", table.table_name(), column.stored()))
    }

    /// A unique field gets a unique index. Empty text is no value, so for
    /// text the index leaves empty text out and many records can leave it
    /// empty; SQL Server leaves out missing values too, which the others do
    /// by themselves.
    pub fn unique_index(self, table: &Table, column: &Column, baseline: bool) -> String {
        let name = self.quote(column.stored());
        let index = self.index_name(table, column);
        let on = self.quote(&table.table_name());
        let textual = matches!(column.ty, FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme | FieldType::Secret);
        let optional = matches!(column.ty, FieldType::Date | FieldType::DateTime | FieldType::Ref { .. });
        let missing = if baseline { " IF NOT EXISTS" } else { "" };
        match self {
            Sql::Sqlite | Sql::Postgres if textual => {
                format!("CREATE UNIQUE INDEX{missing} {index} ON {on} ({name}) WHERE {name} != '';")
            }
            // MySQL has no partial indexes; empty text becomes NULL, which
            // a unique index allows many times.
            Sql::Mysql if textual => format!("CREATE UNIQUE INDEX {index} ON {on} ((NULLIF({name}, '')));"),
            Sql::Mssql if textual => format!("CREATE UNIQUE INDEX {index} ON {on} ({name}) WHERE {name} <> N'';"),
            Sql::Mssql if optional => format!("CREATE UNIQUE INDEX {index} ON {on} ({name}) WHERE {name} IS NOT NULL;"),
            _ => format!("CREATE UNIQUE INDEX{missing} {index} ON {on} ({name});"),
        }
    }

    pub fn drop_unique(self, table: &Table, column: &Column) -> String {
        let index = self.index_name(table, column);
        match self {
            Sql::Sqlite | Sql::Postgres => format!("DROP INDEX IF EXISTS {index};"),
            Sql::Mysql => format!("DROP INDEX {index} ON {};", self.quote(&table.table_name())),
            Sql::Mssql => format!("DROP INDEX IF EXISTS {index} ON {};", self.quote(&table.table_name())),
        }
    }

    /// The full-text index of the table's fields marked `search`, which
    /// `a matches x` looks in: an FTS5 table kept by triggers on SQLite, a
    /// GIN index on PostgreSQL, a FULLTEXT index on MySQL; SQL Server looks
    /// with LIKE, as its full-text search is seldom installed. The names
    /// and the document match the runtimes' queries.
    pub fn search_index(self, table: &Table, baseline: bool) -> Vec<String> {
        let columns: Vec<String> = table.columns.iter().filter(|c| c.search).map(|c| self.quote(c.stored())).collect();
        if columns.is_empty() {
            return vec![];
        }
        let name = table.table_name();
        let index = self.quote(&search_name(&name));
        let on = self.quote(&name);
        let list = columns.join(", ");
        let missing = if baseline { " IF NOT EXISTS" } else { "" };
        match self {
            Sql::Sqlite => {
                let key = self.quote(table.key_column());
                let literal = |text: &str| format!("'{}'", text.replace('\'', "''"));
                let new: Vec<String> = columns.iter().map(|c| format!("new.{c}")).collect();
                let old: Vec<String> = columns.iter().map(|c| format!("old.{c}")).collect();
                let insert = format!("INSERT INTO {index}(rowid, {list}) VALUES (new.{key}, {});", new.join(", "));
                let delete = format!("INSERT INTO {index}({index}, rowid, {list}) VALUES ('delete', old.{key}, {});", old.join(", "));
                let trigger = |when: &str| self.quote(&format!("{}_{when}", search_name(&name)));
                vec![
                    format!(
                        "CREATE VIRTUAL TABLE{missing} {index} USING fts5({list}, content={}, content_rowid={}, tokenize='unicode61 remove_diacritics 0');",
                        literal(&name),
                        literal(table.key_column())
                    ),
                    format!("CREATE TRIGGER{missing} {} AFTER INSERT ON {on} BEGIN {insert} END;", trigger("insert")),
                    format!("CREATE TRIGGER{missing} {} AFTER DELETE ON {on} BEGIN {delete} END;", trigger("delete")),
                    format!("CREATE TRIGGER{missing} {} AFTER UPDATE ON {on} BEGIN {delete} {insert} END;", trigger("update")),
                    format!("INSERT INTO {index}({index}) VALUES ('rebuild');"),
                ]
            }
            Sql::Postgres => {
                let document = columns.iter().map(|c| format!("coalesce({c}, '')")).collect::<Vec<_>>().join(" || ' ' || ");
                vec![format!("CREATE INDEX{missing} {index} ON {on} USING GIN (to_tsvector('simple', {document}));")]
            }
            // Without stopwords, so every word is found.
            Sql::Mysql => vec![
                "SET SESSION innodb_ft_enable_stopword = 0;".to_string(),
                format!("CREATE FULLTEXT INDEX {index} ON {on} ({list});"),
            ],
            Sql::Mssql => vec![],
        }
    }

    /// Drops the table's full-text index, before a step changes the table.
    pub fn drop_search(self, table: &Table) -> Vec<String> {
        if !table.columns.iter().any(|c| c.search) {
            return vec![];
        }
        let name = table.table_name();
        let index = self.quote(&search_name(&name));
        match self {
            Sql::Sqlite => {
                let mut out: Vec<String> =
                    ["insert", "delete", "update"].iter().map(|when| format!("DROP TRIGGER IF EXISTS {};", self.quote(&format!("{}_{when}", search_name(&name))))).collect();
                out.push(format!("DROP TABLE IF EXISTS {index};"));
                out
            }
            Sql::Postgres => vec![format!("DROP INDEX IF EXISTS {index};")],
            Sql::Mysql => vec![format!("DROP INDEX {index} ON {};", self.quote(&name))],
            Sql::Mssql => vec![],
        }
    }

    pub fn add_foreign_key(self, table: &str, column: &str, target: &str, key: &str) -> String {
        format!(
            "ALTER TABLE {} ADD FOREIGN KEY ({}) REFERENCES {} ({});",
            self.quote(table),
            self.quote(column),
            self.quote(target),
            self.quote(key)
        )
    }

    /// Drops the references from `column` to another table, whatever the
    /// database named them. PostgreSQL drops them with the column.
    pub fn drop_foreign_keys(self, table: &str, column: &str) -> Vec<String> {
        match self {
            Sql::Sqlite | Sql::Postgres => Vec::new(),
            Sql::Mysql => vec![
                format!(
                    "SET @clarity = (SELECT IFNULL(CONCAT({}, GROUP_CONCAT(CONCAT('DROP FOREIGN KEY `', CONSTRAINT_NAME, '`'))), 'DO 0') FROM information_schema.KEY_COLUMN_USAGE WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = {} AND COLUMN_NAME = {} AND REFERENCED_TABLE_NAME IS NOT NULL);",
                    self.text(&format!("ALTER TABLE {} ", self.quote(table))),
                    self.text(table),
                    self.text(column)
                ),
                "PREPARE clarity_step FROM @clarity;".into(),
                "EXECUTE clarity_step;".into(),
                "DEALLOCATE PREPARE clarity_step;".into(),
            ],
            Sql::Mssql => vec![format!(
                "DECLARE @sql nvarchar(max) = N'';
SELECT @sql += {} + QUOTENAME(fk.name) + N';' FROM sys.foreign_keys fk JOIN sys.foreign_key_columns fc ON fc.constraint_object_id = fk.object_id
    WHERE fk.parent_object_id = OBJECT_ID({}) AND COL_NAME(fc.parent_object_id, fc.parent_column_id) = {};
EXEC sp_executesql @sql;",
                self.text(&format!("ALTER TABLE {} DROP CONSTRAINT ", self.quote(table))),
                self.text(&self.quote(table)),
                self.text(column)
            )],
        }
    }

    /// Drops a column. SQL Server first drops the column's default, which
    /// it keeps as a constraint of its own.
    pub fn drop_column(self, table: &str, column: &str) -> Vec<String> {
        let mut statements = Vec::new();
        if self == Sql::Mssql {
            statements.push(format!(
                "DECLARE @sql nvarchar(max) = N'';
SELECT @sql += {} + QUOTENAME(d.name) + N';' FROM sys.default_constraints d
    WHERE d.parent_object_id = OBJECT_ID({t}) AND d.parent_column_id = COLUMNPROPERTY(OBJECT_ID({t}), {}, 'ColumnId');
EXEC sp_executesql @sql;",
                self.text(&format!("ALTER TABLE {} DROP CONSTRAINT ", self.quote(table))),
                self.text(column),
                t = self.text(&self.quote(table)),
            ));
        }
        statements.push(format!("ALTER TABLE {} DROP COLUMN {};", self.quote(table), self.quote(column)));
        statements
    }

    /// The value of `name`, a column stored as `old`, as a column of type
    /// `new` stores it.
    pub fn convert(self, name: &str, old: &FieldType, new: &FieldType, new_ty: &str) -> String {
        let textual = |ty: &FieldType| matches!(ty, FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme | FieldType::Secret | FieldType::Enum { .. });
        // Empty text is no value, so it does not become a number or a date.
        let value = if textual(old) && !textual(new) {
            format!("NULLIF({name}, '')")
        } else {
            name.to_string()
        };
        match (self, old, new) {
            (Sql::Postgres, FieldType::Bool, FieldType::Integer | FieldType::Id | FieldType::Duration | FieldType::Decimal { .. } | FieldType::Money) => {
                format!("CASE WHEN {name} THEN 1 ELSE 0 END")
            }
            (Sql::Postgres, FieldType::Integer | FieldType::Id | FieldType::Duration | FieldType::Decimal { .. } | FieldType::Money, FieldType::Bool) => {
                format!("({name} <> 0)")
            }
            // MySQL converts a value as it stores it.
            (Sql::Mysql, ..) => value,
            _ => format!("CAST({value} AS {new_ty})"),
        }
    }
}

/// The name of a table's full-text index, as the runtimes look in it.
pub fn search_name(table: &str) -> String {
    format!("{table}__search")
}
