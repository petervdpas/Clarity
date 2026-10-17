//! Queries: which records of an entity to read, and in which order.
//! Generated code builds a `Query` from Clarity's `find`; this module turns
//! it into SQL with parameters, in the dialect of the database the records
//! are in.

use crate::SqlValue;
use crate::dialect::Dialect;

/// A comparison between a column and a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
}

impl Op {
    fn sql(self) -> &'static str {
        match self {
            Op::Eq => "=",
            Op::NotEq => "!=",
            Op::Lt => "<",
            Op::LtEq => "<=",
            Op::Gt => ">",
            Op::GtEq => ">=",
        }
    }
}

/// A condition on the columns of one table.
#[derive(Debug, Clone, PartialEq)]
pub enum Cond {
    /// `column op value`. Numbers stored as text (decimals and money) are
    /// compared as numbers.
    Compare {
        column: &'static str,
        op: Op,
        value: SqlValue,
        numeric: bool,
    },
    /// The column's text contains `text`, ignoring case.
    Contains { column: &'static str, text: String },
    /// The column equals one of the values.
    OneOf {
        column: &'static str,
        values: Vec<SqlValue>,
        numeric: bool,
    },
    /// A bool column that is true.
    IsTrue { column: &'static str },
    And(Box<Cond>, Box<Cond>),
    Or(Box<Cond>, Box<Cond>),
    Not(Box<Cond>),
}

impl Cond {
    pub fn compare(column: &'static str, op: Op, value: SqlValue) -> Cond {
        Cond::Compare {
            column,
            op,
            value,
            numeric: false,
        }
    }

    /// A comparison of a decimal or money column.
    pub fn number(column: &'static str, op: Op, value: SqlValue) -> Cond {
        Cond::Compare {
            column,
            op,
            value,
            numeric: true,
        }
    }

    pub fn contains(column: &'static str, text: &str) -> Cond {
        Cond::Contains {
            column,
            text: text.to_string(),
        }
    }

    pub fn one_of(column: &'static str, values: Vec<SqlValue>) -> Cond {
        Cond::OneOf {
            column,
            values,
            numeric: false,
        }
    }

    /// `one_of` for a decimal or money column.
    pub fn one_of_numbers(column: &'static str, values: Vec<SqlValue>) -> Cond {
        Cond::OneOf {
            column,
            values,
            numeric: true,
        }
    }

    pub fn is_true(column: &'static str) -> Cond {
        Cond::IsTrue { column }
    }

    pub fn and(self, other: Cond) -> Cond {
        Cond::And(Box::new(self), Box::new(other))
    }

    pub fn or(self, other: Cond) -> Cond {
        Cond::Or(Box::new(self), Box::new(other))
    }

    pub fn negated(self) -> Cond {
        Cond::Not(Box::new(self))
    }

    fn sql(&self, dialect: Dialect, params: &mut Vec<SqlValue>) -> String {
        let mut param = |value: SqlValue, numeric: bool| {
            params.push(value);
            let mark = dialect.mark(params.len());
            if numeric { dialect.number(mark) } else { mark }
        };
        match self {
            Cond::Compare {
                column,
                op,
                value,
                numeric,
            } => format!(
                "{} {} {}",
                column_sql(dialect, column, *numeric),
                op.sql(),
                param(value.clone(), *numeric)
            ),
            Cond::Contains { column, text } => {
                let pattern = format!("%{}%", dialect.escape_like(text));
                dialect.contains(column, &param(SqlValue::Text(pattern), false))
            }
            Cond::OneOf {
                column,
                values,
                numeric,
            } => {
                let marks: Vec<String> = values.iter().map(|v| param(v.clone(), *numeric)).collect();
                format!("{} IN ({})", column_sql(dialect, column, *numeric), marks.join(", "))
            }
            Cond::IsTrue { column } => dialect.is_true(column),
            Cond::And(left, right) => format!("({} AND {})", left.sql(dialect, params), right.sql(dialect, params)),
            Cond::Or(left, right) => format!("({} OR {})", left.sql(dialect, params), right.sql(dialect, params)),
            Cond::Not(inner) => format!("NOT ({})", inner.sql(dialect, params)),
        }
    }
}

/// A column as a comparison reads it.
fn column_sql(dialect: Dialect, column: &str, numeric: bool) -> String {
    if numeric {
        dialect.number(dialect.quote(column))
    } else {
        dialect.quote(column)
    }
}

/// The records to read: a condition and an order. Without an order,
/// records come in the order they were stored.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Query {
    filter: Option<Cond>,
    order: Vec<(&'static str, bool, bool)>,
}

impl Query {
    pub fn new() -> Query {
        Query::default()
    }

    pub fn filter(mut self, cond: Cond) -> Query {
        self.filter = Some(cond);
        self
    }

    pub fn order(mut self, column: &'static str, descending: bool) -> Query {
        self.order.push((column, descending, false));
        self
    }

    /// `order` for a decimal or money column.
    pub fn order_number(mut self, column: &'static str, descending: bool) -> Query {
        self.order.push((column, descending, true));
        self
    }

    /// The SELECT statement for `table` in `dialect`, reading `columns`,
    /// and its parameters. Records that sort the same come in the order of
    /// `key`.
    pub fn sql(&self, dialect: Dialect, table: &str, key: &str, columns: &str) -> (String, Vec<SqlValue>) {
        let mut params = Vec::new();
        let mut sql = format!("SELECT {columns} FROM {}", dialect.quote(table));
        if let Some(filter) = &self.filter {
            sql.push_str(" WHERE ");
            sql.push_str(&filter.sql(dialect, &mut params));
        }
        let mut keys: Vec<String> = self
            .order
            .iter()
            .map(|(column, descending, numeric)| {
                let direction = if *descending { " DESC" } else { "" };
                format!("{}{direction}", column_sql(dialect, column, *numeric))
            })
            .collect();
        keys.push(dialect.quote(key));
        sql.push_str(" ORDER BY ");
        sql.push_str(&keys.join(", "));
        (sql, params)
    }
}

/// `text contains part`, ignoring case, as Clarity's `contains` outside a
/// query.
pub fn contains(text: &str, part: &str) -> bool {
    text.to_lowercase().contains(&part.to_lowercase())
}
