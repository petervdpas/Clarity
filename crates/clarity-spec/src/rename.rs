//! Renaming an entity or an enum across the project: its declaration and
//! every reference to it, found through the parsed model rather than by
//! searching text.

use serde::Deserialize;
use ts_rs::TS;

use crate::code::{Expr, ExprKind, Stmt, StmtKind, TextPart};
use crate::model::{DataDecl, FieldType, Ident, Project, SpecFile};
use crate::span::Span;
use crate::widgets::is_identifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RenameKind {
    Entity,
    Enum,
}

/// A file whose text changes, by its path in the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Renamed {
    pub path: String,
    pub text: String,
}

/// Renames `old` to `new` in every file that can see its declaration.
/// `source` gives the current text of a file by its path.
pub fn rename(
    project: &Project,
    source: impl Fn(&str) -> Option<String>,
    kind: RenameKind,
    old: &str,
    new: &str,
) -> Result<Vec<Renamed>, String> {
    let what = match kind {
        RenameKind::Entity => "entity",
        RenameKind::Enum => "enum",
    };
    if !is_identifier(new) {
        return Err(format!("'{new}' is not a valid {what} name"));
    }
    if new == old {
        return Ok(Vec::new());
    }
    let declared = |f: &SpecFile| match kind {
        RenameKind::Entity => f.entities.iter().any(|e| e.name.name == old),
        RenameKind::Enum => f.enums.iter().any(|e| e.name.name == old),
    };
    let owners: Vec<&SpecFile> = project.files.iter().filter(|f| declared(f)).collect();
    let owner = match owners.as_slice() {
        [owner] => *owner,
        [] => return Err(format!("there is no {what} '{old}'")),
        _ => return Err(format!("'{old}' is declared in more than one module; rename it in the file")),
    };
    let module = owner.module.as_ref().map(|m| m.name.as_str());

    let mut changed = Vec::new();
    for file in &project.files {
        let this = file.module.as_ref().map(|m| m.name.as_str());
        let sees = this == module || file.uses.iter().any(|u| Some(u.name.as_str()) == module);
        if !sees {
            continue;
        }
        let taken = file.entities.iter().map(|e| &e.name.name)
            .chain(file.enums.iter().map(|e| &e.name.name))
            .chain(file.pages.iter().map(|p| &p.name.name))
            .chain(file.apps.iter().map(|a| &a.name.name))
            .any(|n| n == new);
        if taken {
            return Err(format!("'{new}' is already declared in module '{}'", this.unwrap_or("")));
        }
        // Each place to change: where it is, what it says now, and what it
        // says after.
        let mut spans: Vec<(usize, usize, String, String)> = references(file, kind, old)
            .into_iter()
            .map(|i| (i.span.start as usize, i.span.end as usize, old.to_string(), new.to_string()))
            .collect();
        if kind == RenameKind::Enum {
            // A window's plain values of the enum: `data pending: Operation`.
            let data = file.pages.iter().flat_map(|p| &p.data).chain(file.apps.iter().flat_map(|a| &a.data));
            for ty in data.filter_map(DataDecl::value) {
                if matches!(&ty.kind, crate::code::TypeRefKind::Named { name } if name == old) {
                    spans.push((ty.span.start as usize, ty.span.end as usize, old.to_string(), new.to_string()));
                }
            }
        }
        if kind == RenameKind::Entity {
            let (was, now) = (crate::code::record_name(old), crate::code::record_name(new));
            spans.extend(
                record_uses(file, old)
                    .into_iter()
                    .map(|s| (s.start as usize, s.end as usize, was.clone(), now.clone())),
            );
        }
        if spans.is_empty() {
            continue;
        }
        let Some(mut text) = source(&file.path) else {
            return Err(format!("cannot read {}", file.path));
        };
        spans.sort_unstable();
        spans.dedup();
        for (start, end, was, now) in spans.into_iter().rev() {
            if text.get(start..end) != Some(was.as_str()) {
                return Err(format!("{} changed while renaming; try again", file.path));
            }
            text.replace_range(start..end, &now);
        }
        changed.push(Renamed {
            path: file.path.clone(),
            text,
        });
    }
    Ok(changed)
}

/// The identifiers in a file that name the entity or enum `name`.
fn references<'f>(file: &'f SpecFile, kind: RenameKind, name: &str) -> Vec<&'f Ident> {
    let mut out: Vec<&Ident> = Vec::new();
    match kind {
        RenameKind::Entity => out.extend(file.entities.iter().map(|e| &e.name)),
        RenameKind::Enum => out.extend(file.enums.iter().map(|e| &e.name)),
    }
    for field in file.entities.iter().flat_map(|e| &e.fields) {
        match (&field.ty, kind) {
            (FieldType::Ref { entity }, RenameKind::Entity) => out.push(entity),
            (FieldType::List { item }, RenameKind::Entity) => out.push(item),
            (FieldType::Enum { name }, RenameKind::Enum) => out.push(name),
            _ => {}
        }
    }
    if kind == RenameKind::Entity {
        let data = file
            .pages
            .iter()
            .flat_map(|p| &p.data)
            .chain(file.apps.iter().flat_map(|a| &a.data));
        out.extend(data.filter_map(|d: &DataDecl| d.entity()));
        // The code block of the entity, with its data events.
        out.extend(file.code.iter().map(|b| &b.name));
    }
    out.retain(|i| i.name == name);
    out
}

/// Where the events of entity `entity` name their record, as in
/// `order.total` in the events of `Order`.
fn record_uses(file: &SpecFile, entity: &str) -> Vec<Span> {
    let record = crate::code::record_name(entity);
    let mut found = Vec::new();
    for block in file.code.iter().filter(|b| b.name.name == entity) {
        for event in &block.events {
            stmts(&event.body, &record, &mut found);
        }
    }
    found
}

fn stmts(body: &[Stmt], name: &str, found: &mut Vec<Span>) {
    for stmt in body {
        match &stmt.kind {
            StmtKind::Let { value, .. } => expr(value, name, found),
            StmtKind::Assign { target, value } => {
                expr(target, name, found);
                expr(value, name, found);
            }
            StmtKind::If { branches, otherwise } => {
                for branch in branches {
                    expr(&branch.condition, name, found);
                    stmts(&branch.body, name, found);
                }
                if let Some(body) = otherwise {
                    stmts(body, name, found);
                }
            }
            StmtKind::Return { value } => {
                if let Some(value) = value {
                    expr(value, name, found);
                }
            }
            StmtKind::For { list, body, .. } => {
                expr(list, name, found);
                stmts(body, name, found);
            }
            StmtKind::Call { call } => expr(call, name, found),
            StmtKind::Draw { args, .. } | StmtKind::Hear { args, .. } => {
                for arg in args {
                    expr(arg, name, found);
                }
            }
            StmtKind::Message { text, .. } | StmtKind::Refuse { text } => expr(text, name, found),
            StmtKind::Ask { question, form, args, .. } => {
                if let Some(question) = question {
                    expr(question, name, found);
                }
                for e in crate::code::ask_exprs(form) {
                    expr(e, name, found);
                }
                for arg in args {
                    expr(arg, name, found);
                }
            }
            StmtKind::Open { with, .. } => {
                for arg in with {
                    expr(&arg.value, name, found);
                }
            }
            StmtKind::Save { value } | StmtKind::Delete { value } | StmtKind::CloseWith { value } | StmtKind::Print { value, .. } | StmtKind::Exit { code: value } => {
                expr(value, name, found)
            }
            StmtKind::Write { text, path, .. } => {
                expr(text, name, found);
                expr(path, name, found);
            }
            StmtKind::Close | StmtKind::Reload { .. } => {}
        }
    }
}

fn expr(e: &Expr, name: &str, found: &mut Vec<Span>) {
    match &e.kind {
        ExprKind::Name { name: n } if n == name => found.push(e.span),
        ExprKind::Member { object, .. } => expr(object, name, found),
        ExprKind::Unary { operand, .. } => expr(operand, name, found),
        ExprKind::Binary { left, right, .. } => {
            expr(left, name, found);
            expr(right, name, found);
        }
        ExprKind::In { value, options } => {
            expr(value, name, found);
            for option in options {
                expr(option, name, found);
            }
        }
        ExprKind::Call { args, .. } | ExprKind::EngineCall { args, .. } => {
            for arg in args {
                expr(arg, name, found);
            }
        }
        ExprKind::Find { filter, .. } | ExprKind::Files { filter, .. } => {
            if let Some(filter) = filter {
                expr(filter, name, found);
            }
        }
        ExprKind::ReadText { file } | ExprKind::ReadBytes { file } => expr(file, name, found),
        ExprKind::Ask { question, form } => {
            expr(question, name, found);
            for e in crate::code::ask_exprs(form) {
                expr(e, name, found);
            }
        }
        ExprKind::Key { args, .. } => {
            for arg in args {
                expr(&arg.value, name, found);
            }
        }
        ExprKind::ListOf { items } => {
            for item in items {
                expr(item, name, found);
            }
        }
        ExprKind::Index { list, index } => {
            expr(list, name, found);
            expr(index, name, found);
        }
        ExprKind::Text { parts } => {
            for part in parts {
                if let TextPart::Slot { value } = part {
                    expr(value, name, found);
                }
            }
        }
        ExprKind::Number { .. } | ExprKind::Str { .. } | ExprKind::Bool { .. } | ExprKind::Empty | ExprKind::Name { .. } | ExprKind::New { .. } => {}
    }
}
