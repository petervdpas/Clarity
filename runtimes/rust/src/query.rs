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

/// A step through a reference: the column that holds it, and the table and
/// key column of the records it refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hop {
    pub column: &'static str,
    pub table: &'static str,
    pub key: &'static str,
}

impl Hop {
    pub fn new(column: &'static str, table: &'static str, key: &'static str) -> Hop {
        Hop { column, table, key }
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
    /// The JSON list kept in the column holds the item, exactly (a list field).
    Holds { column: &'static str, item: SqlValue },
    /// The column equals one of the values.
    OneOf {
        column: &'static str,
        values: Vec<SqlValue>,
        numeric: bool,
    },
    /// A bool column that is true.
    IsTrue { column: &'static str },
    /// A column with no value: an empty date or reference (`none`).
    IsNone { column: &'static str },
    /// The record the reference in `hop.column` refers to meets the
    /// condition (`a.artist.name contains x`).
    Through { hop: Hop, cond: Box<Cond> },
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

    /// `a.tags contains "live"`: whether a list field holds an item.
    pub fn holds(column: &'static str, item: SqlValue) -> Cond {
        Cond::Holds { column, item }
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

    pub fn is_none(column: &'static str) -> Cond {
        Cond::IsNone { column }
    }

    /// `cond` on the record that `column` refers to, in `table` by `key`.
    pub fn through(column: &'static str, table: &'static str, key: &'static str, cond: Cond) -> Cond {
        Cond::Through {
            hop: Hop::new(column, table, key),
            cond: Box::new(cond),
        }
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
            Cond::Holds { column, item } => dialect.holds(column, &param(dialect.item(item), false)),
            Cond::OneOf {
                column,
                values,
                numeric,
            } => {
                let marks: Vec<String> = values.iter().map(|v| param(v.clone(), *numeric)).collect();
                format!("{} IN ({})", column_sql(dialect, column, *numeric), marks.join(", "))
            }
            Cond::IsTrue { column } => dialect.is_true(column),
            Cond::IsNone { column } => format!("{} IS NULL", dialect.quote(column)),
            // The inner condition names the columns of the referred table,
            // which a subquery's own FROM resolves first, even for a table
            // that refers to itself.
            Cond::Through { hop, cond } => format!(
                "{} IN (SELECT {} FROM {} WHERE {})",
                dialect.quote(hop.column),
                dialect.quote(hop.key),
                dialect.quote(hop.table),
                cond.sql(dialect, params)
            ),
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
    order: Vec<OrderKey>,
}

/// A column to sort by, perhaps of a record the references in `through`
/// lead to.
#[derive(Debug, Clone, PartialEq)]
struct OrderKey {
    through: Vec<Hop>,
    column: &'static str,
    descending: bool,
    numeric: bool,
}

impl OrderKey {
    /// The key as SQL. Through references, each step is a subquery on its
    /// own alias (t1, t2, ...) that finds the referred record by the column
    /// of the step before.
    fn sql(&self, dialect: Dialect, table: &str) -> String {
        let direction = if self.descending { " DESC" } else { "" };
        if self.through.is_empty() {
            return format!("{}{direction}", column_sql(dialect, self.column, self.numeric));
        }
        format!("{}{direction}", self.step(dialect, &dialect.quote(table), 0))
    }

    fn step(&self, dialect: Dialect, outer: &str, at: usize) -> String {
        let Some(hop) = self.through.get(at) else {
            let column = format!("{outer}.{}", dialect.quote(self.column));
            return if self.numeric { dialect.number(column) } else { column };
        };
        let alias = dialect.quote(&format!("t{}", at + 1));
        format!(
            "(SELECT {} FROM {} {alias} WHERE {alias}.{} = {outer}.{})",
            self.step(dialect, &alias, at + 1),
            dialect.quote(hop.table),
            dialect.quote(hop.key),
            dialect.quote(hop.column)
        )
    }
}

impl Query {
    pub fn new() -> Query {
        Query::default()
    }

    pub fn filter(mut self, cond: Cond) -> Query {
        self.filter = Some(cond);
        self
    }

    pub fn order(self, column: &'static str, descending: bool) -> Query {
        self.order_through(&[], column, descending, false)
    }

    /// `order` for a decimal or money column.
    pub fn order_number(self, column: &'static str, descending: bool) -> Query {
        self.order_through(&[], column, descending, true)
    }

    /// `order` by a column of the record the references `through` lead to
    /// (`order by a.artist.name`); `numeric` for a decimal or money column.
    pub fn order_through(mut self, through: &[Hop], column: &'static str, descending: bool, numeric: bool) -> Query {
        self.order.push(OrderKey {
            through: through.to_vec(),
            column,
            descending,
            numeric,
        });
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
        let mut keys: Vec<String> = self.order.iter().map(|key| key.sql(dialect, table)).collect();
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
