//! The paths a query's condition and order take through references
//! (`o.customer.name`), resolved once for every backend: the steps to the
//! referred record, with the column, table and key column of each, and the
//! entity and field at the end. The runtimes turn the steps into subqueries.

use clarity_spec::Project;
use clarity_spec::code::{Expr, ExprKind, OrderKey};

/// A step through a reference, by the names the database uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Hop {
    /// The column of the reference in the record before the step.
    pub column: String,
    pub table: String,
    pub key: String,
}

/// A field of the query's record, or of a record its references lead to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldPath {
    pub hops: Vec<Hop>,
    /// The entity the field is of.
    pub entity: String,
    pub field: String,
}

/// The path `expr` takes from the query's record `var` of `entity`, when it
/// is a member path of it (`o.total`, `o.customer.name`).
pub(crate) fn field_path(project: &Project, entity: &str, var: &str, expr: &Expr) -> Option<FieldPath> {
    let mut members = Vec::new();
    let mut at = expr;
    while let ExprKind::Member { object, member } = &at.kind {
        members.push(member.name.as_str());
        at = object;
    }
    let ExprKind::Name { name } = &at.kind else { return None };
    if name != var || members.is_empty() {
        return None;
    }
    members.reverse();
    let field = members.pop()?;
    resolve(project, entity, &members, field)
}

/// The path of an order key.
pub(crate) fn order_path(project: &Project, entity: &str, key: &OrderKey) -> Option<FieldPath> {
    let through: Vec<&str> = key.through.iter().map(|i| i.name.as_str()).collect();
    resolve(project, entity, &through, &key.field.name)
}

fn resolve(project: &Project, entity: &str, through: &[&str], field: &str) -> Option<FieldPath> {
    let mut current = project.entity(entity)?;
    let mut hops = Vec::new();
    for name in through {
        let clarity_spec::model::FieldType::Ref { entity: target } = &current.field(name)?.ty else { return None };
        let next = project.entity(&target.name)?;
        hops.push(Hop {
            column: current.column_of(name).to_string(),
            table: next.table_name(),
            key: next.key_column().to_string(),
        });
        current = next;
    }
    Some(FieldPath {
        hops,
        entity: current.name.name.clone(),
        field: field.to_string(),
    })
}

/// What `a matches x` looks in: the entity's table and key, and the columns
/// of its fields marked `search`, by the names the database uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Search {
    pub table: String,
    pub key: String,
    pub columns: Vec<String>,
}

/// The search of `entity`, when `expr` is `var matches text` (the checker
/// made sure of the rest); its text is the right side.
pub(crate) fn search<'e>(project: &Project, entity: &str, var: &str, expr: &'e Expr) -> Option<(Search, &'e Expr)> {
    let ExprKind::Binary { op: clarity_spec::code::BinaryOp::Matches, left, right } = &expr.kind else { return None };
    if !matches!(&left.kind, ExprKind::Name { name } if name == var) {
        return None;
    }
    let decl = project.entity(entity)?;
    let columns = decl.fields.iter().filter(|f| f.modifiers.search).map(|f| decl.column_of(&f.name.name).to_string()).collect();
    Some((Search { table: decl.table_name(), key: decl.key_column().to_string(), columns }, right))
}
