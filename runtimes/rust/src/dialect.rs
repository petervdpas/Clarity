//! The SQL dialects of the databases a data source can be. Statements that
//! read and write records are written here, so the rest of the runtime
//! works the same on every database.

use crate::sql::SqlValue;

/// The database a data source is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Sqlite,
    Postgres,
    Mysql,
    Mssql,
}

impl Dialect {
    /// The database's name, for messages.
    pub fn label(self) -> &'static str {
        match self {
            Dialect::Sqlite => "SQLite",
            Dialect::Postgres => "PostgreSQL",
            Dialect::Mysql => "MySQL",
            Dialect::Mssql => "SQL Server",
        }
    }

    /// A table or column name as SQL needs it, so names like `order` work.
    pub fn quote(self, name: &str) -> String {
        match self {
            Dialect::Sqlite | Dialect::Postgres => format!("\"{}\"", name.replace('"', "\"\"")),
            Dialect::Mysql => format!("`{}`", name.replace('`', "``")),
            Dialect::Mssql => format!("[{}]", name.replace(']', "]]")),
        }
    }

    /// The mark of parameter `n`, counted from 1.
    pub fn mark(self, n: usize) -> String {
        match self {
            Dialect::Sqlite => format!("?{n}"),
            Dialect::Postgres => format!("${n}"),
            Dialect::Mysql => "?".to_string(),
            Dialect::Mssql => format!("@P{n}"),
        }
    }

    /// A decimal value as a comparison reads it. SQLite stores decimals as
    /// text, so it compares them as numbers; the others store numbers.
    pub(crate) fn number(self, expr: String) -> String {
        match self {
            Dialect::Sqlite => format!("CAST({expr} AS REAL)"),
            _ => expr,
        }
    }

    /// `column` contains the LIKE pattern `mark`, ignoring case. The
    /// pattern escapes its wildcards with `!`.
    pub(crate) fn contains(self, column: &str, mark: &str) -> String {
        match self {
            // SQLite's LIKE ignores case already.
            Dialect::Sqlite => format!("{} LIKE {mark} ESCAPE '!'", self.quote(column)),
            _ => format!("LOWER({}) LIKE LOWER({mark}) ESCAPE '!'", self.quote(column)),
        }
    }

    /// Whether the JSON list kept in `column` holds the item `mark`, exactly
    /// (a list field, U1c), as `item` gives it.
    pub(crate) fn holds(self, column: &str, mark: &str) -> String {
        let column = self.quote(column);
        match self {
            Dialect::Sqlite => format!("EXISTS (SELECT 1 FROM json_each({column}) WHERE value = {mark})"),
            Dialect::Postgres => format!("{column}::jsonb @> {mark}::jsonb"),
            Dialect::Mysql => format!("JSON_CONTAINS({column}, {mark})"),
            Dialect::Mssql => format!("EXISTS (SELECT 1 FROM OPENJSON({column}) WHERE [value] = {mark})"),
        }
    }

    /// An item of a list as `holds` looks for it: SQLite compares the item
    /// itself, PostgreSQL a list of it as JSON, MySQL the item as JSON, and
    /// SQL Server its text. A date or a decimal is its text, as a list keeps it.
    pub(crate) fn item(self, item: &SqlValue) -> SqlValue {
        let text = match item {
            SqlValue::Integer(n) if self == Dialect::Sqlite => return SqlValue::Integer(*n),
            SqlValue::Integer(n) => return SqlValue::Text(if self == Dialect::Postgres { format!("[{n}]") } else { n.to_string() }),
            SqlValue::Text(s) => s.clone(),
            SqlValue::Decimal(d) => d.to_string(),
            SqlValue::Date(d) => d.format(crate::sql::DATE).to_string(),
            other => format!("{other:?}"),
        };
        let json = serde_json::to_string(&text).unwrap_or_default();
        SqlValue::Text(match self {
            Dialect::Sqlite | Dialect::Mssql => text,
            Dialect::Postgres => format!("[{json}]"),
            Dialect::Mysql => json,
        })
    }

    /// Text that LIKE matches as it is: wildcards are escaped with `!`.
    pub(crate) fn escape_like(self, text: &str) -> String {
        let mut out = String::new();
        for c in text.chars() {
            if matches!(c, '!' | '%' | '_') || (c == '[' && self == Dialect::Mssql) {
                out.push('!');
            }
            out.push(c);
        }
        out
    }

    /// A bool column that is true.
    pub(crate) fn is_true(self, column: &str) -> String {
        match self {
            Dialect::Postgres => format!("{} = TRUE", self.quote(column)),
            _ => format!("{} = 1", self.quote(column)),
        }
    }

    /// INSERT of a new record into `table`, with a mark per column. On
    /// PostgreSQL and SQL Server it returns the new key; SQLite and MySQL
    /// tell it afterwards.
    pub(crate) fn insert(self, table: &str, key: &str, columns: &[&str]) -> String {
        let table = self.quote(table);
        let key = self.quote(key);
        let names = columns.iter().map(|c| self.quote(c)).collect::<Vec<_>>().join(", ");
        let marks = (1..=columns.len()).map(|i| self.mark(i)).collect::<Vec<_>>().join(", ");
        match (self, columns.is_empty()) {
            (Dialect::Mysql, true) => format!("INSERT INTO {table} () VALUES ()"),
            (Dialect::Sqlite, true) => format!("INSERT INTO {table} DEFAULT VALUES"),
            (Dialect::Postgres, true) => format!("INSERT INTO {table} DEFAULT VALUES RETURNING {key}"),
            (Dialect::Mssql, true) => format!("INSERT INTO {table} OUTPUT INSERTED.{key} DEFAULT VALUES"),
            (Dialect::Sqlite | Dialect::Mysql, false) => format!("INSERT INTO {table} ({names}) VALUES ({marks})"),
            (Dialect::Postgres, false) => format!("INSERT INTO {table} ({names}) VALUES ({marks}) RETURNING {key}"),
            (Dialect::Mssql, false) => format!("INSERT INTO {table} ({names}) OUTPUT INSERTED.{key} VALUES ({marks})"),
        }
    }

    pub(crate) fn begin(self) -> &'static str {
        match self {
            Dialect::Mysql => "START TRANSACTION",
            Dialect::Mssql => "BEGIN TRANSACTION",
            Dialect::Sqlite | Dialect::Postgres => "BEGIN",
        }
    }

    pub(crate) fn commit(self) -> &'static str {
        match self {
            Dialect::Mssql => "COMMIT TRANSACTION",
            _ => "COMMIT",
        }
    }

    pub(crate) fn rollback(self) -> &'static str {
        match self {
            Dialect::Mssql => "ROLLBACK TRANSACTION",
            _ => "ROLLBACK",
        }
    }

    /// Creates the table that remembers how many migration steps a
    /// database has had.
    pub(crate) fn version_table(self) -> &'static str {
        match self {
            Dialect::Mssql => "IF OBJECT_ID(N'clarity_schema', N'U') IS NULL CREATE TABLE clarity_schema (version INT NOT NULL)",
            _ => "CREATE TABLE IF NOT EXISTS clarity_schema (version INTEGER NOT NULL)",
        }
    }
}
