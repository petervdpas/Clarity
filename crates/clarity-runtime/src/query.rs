//! Queries: which records of an entity to read, and in which order.
//! Generated code builds a `Query` from Clarity's `find`; this module turns
//! it into SQL with parameters. The SQL dialect lives here only, so another
//! database is another way of writing the same query.

use crate::{SqlValue, quoted};

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

    fn sql(&self, params: &mut Vec<SqlValue>) -> String {
        let mut param = |value: SqlValue, numeric: bool| {
            params.push(value);
            let mark = format!("?{}", params.len());
            if numeric { format!("CAST({mark} AS REAL)") } else { mark }
        };
        match self {
            Cond::Compare {
                column,
                op,
                value,
                numeric,
            } => format!("{} {} {}", column_sql(column, *numeric), op.sql(), param(value.clone(), *numeric)),
            Cond::Contains { column, text } => {
                let pattern = format!("%{}%", escape_like(text));
                format!("{} LIKE {} ESCAPE '\\'", quoted(column), param(SqlValue::Text(pattern), false))
            }
            Cond::OneOf {
                column,
                values,
                numeric,
            } => {
                let marks: Vec<String> = values.iter().map(|v| param(v.clone(), *numeric)).collect();
                format!("{} IN ({})", column_sql(column, *numeric), marks.join(", "))
            }
            Cond::IsTrue { column } => format!("{} = 1", quoted(column)),
            Cond::And(left, right) => format!("({} AND {})", left.sql(params), right.sql(params)),
            Cond::Or(left, right) => format!("({} OR {})", left.sql(params), right.sql(params)),
            Cond::Not(inner) => format!("NOT ({})", inner.sql(params)),
        }
    }
}

/// A column as a comparison reads it.
fn column_sql(column: &str, numeric: bool) -> String {
    if numeric {
        format!("CAST({} AS REAL)", quoted(column))
    } else {
        quoted(column)
    }
}

/// Text for LIKE in which `%` and `_` are themselves, not wildcards.
fn escape_like(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out
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

    /// The SELECT statement for `table`, reading `columns`, and its
    /// parameters. Records that sort the same come in the order of `key`.
    pub fn sql(&self, table: &str, key: &str, columns: &str) -> (String, Vec<SqlValue>) {
        let mut params = Vec::new();
        let mut sql = format!("SELECT {columns} FROM {}", quoted(table));
        if let Some(filter) = &self.filter {
            sql.push_str(" WHERE ");
            sql.push_str(&filter.sql(&mut params));
        }
        let mut keys: Vec<String> = self
            .order
            .iter()
            .map(|(column, descending, numeric)| {
                let direction = if *descending { " DESC" } else { "" };
                format!("{}{direction}", column_sql(column, *numeric))
            })
            .collect();
        keys.push(quoted(key));
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
