//! Renaming a value of an enum across the project: its declaration and
//! every place that names it, found through the parsed model, never by
//! searching text. A field's, a setting's and a command option's default of
//! the enum; code the checker typed as the value, bare (`c.kind = Private`)
//! or with its enum (`Kind.Private`); a window's data default of the enum
//! and a click's value for a handler parameter of the enum (`pick(Private)`).
//! A value of the same name in another enum is never touched. What the
//! database keeps follows through the step's `rename value` (migrate).

use std::collections::BTreeSet;

use crate::code::check::{FileTypes, NameKind};
use crate::code::{Expr, ExprKind, Stmt, TypeRef, TypeRefKind};
use crate::design_edit::Designed;
use crate::model::{DataKind, Field, FieldType, Literal, Project, SpecFile, Widget};
use crate::rename::Renamed;
use crate::span::Span;

/// Renames value `old` of enum `enumeration` to `new` in every file that
/// names it. `source` gives the current text of a file by its path in the project.
pub fn rename_value(project: &Project, source: impl Fn(&str) -> Option<String>, enumeration: &str, old: &str, new: &str) -> Result<Vec<Renamed>, String> {
    if !crate::widgets::is_identifier(new) || !new.starts_with(|c: char| c.is_ascii_uppercase()) {
        return Err(format!("'{new}' is not a valid value name; a value is PascalCase, like Personal"));
    }
    let decls: Vec<_> = project.files.iter().flat_map(|f| f.enums.iter().filter(|e| e.name.name == enumeration).map(move |e| (f, e))).collect();
    let (owner, decl) = match decls.as_slice() {
        [one] => *one,
        [] => return Err(format!("there is no enum {enumeration}")),
        _ => return Err(format!("{enumeration} is declared in more than one module; rename its value in the file")),
    };
    let Some(variant) = decl.variants.iter().find(|v| v.name.name == old) else {
        return Err(format!("{enumeration} has no value {old}"));
    };
    if new == old {
        return Ok(Vec::new());
    }
    if decl.variants.iter().any(|v| v.name.name == new) {
        return Err(format!("{enumeration} already has a value {new}"));
    }
    let types = crate::code::check::types(&project.files);
    let rename = Rename { enumeration, old, new };
    let mut changed = Vec::new();
    for file in &project.files {
        let Some(text) = source(&file.path) else {
            return Err(format!("cannot read {}", file.path));
        };
        if crate::cxaml::is_page_file(&file.path) {
            let mut designed = crate::cxaml::read_designed(&text, &file.path)?;
            if rename.in_designed(project, &mut designed) {
                changed.push(Renamed { path: file.path.clone(), text: crate::cxaml::written_again(&text, &designed), moved_from: None });
            }
            continue;
        }
        let mut spans = rename.in_code(file, &types.get(&file.path).cloned().unwrap_or_default());
        if std::ptr::eq(file, owner) {
            spans.insert(range(variant.name.span));
        }
        for field in file.entities.iter().chain(&file.records).flat_map(|e| &e.fields) {
            if rename.defaults(field)
                && let Some(at) = default_at(&text, field.span, old)
            {
                spans.insert(at);
            }
        }
        if spans.is_empty() {
            continue;
        }
        let mut text = text;
        for (start, end) in spans.into_iter().rev() {
            if text.get(start..end) != Some(old) {
                return Err(format!("{} changed while renaming; try again", file.path));
            }
            text.replace_range(start..end, new);
        }
        changed.push(Renamed { path: file.path.clone(), text, moved_from: None });
    }
    Ok(changed)
}

fn range(span: Span) -> (usize, usize) {
    (span.start as usize, span.end as usize)
}

/// Where `default old` names the value in a declaration's line.
fn default_at(text: &str, span: Span, old: &str) -> Option<(usize, usize)> {
    let (start, end) = range(span);
    let line = text.get(start..end.min(text.len()))?;
    let line = &line[..line.find('\n').unwrap_or(line.len())];
    let mut words = line.match_indices(|c: char| !(c.is_alphanumeric() || c == '_')).fold((Vec::new(), 0), |(mut out, from), (at, sep)| {
        if at > from {
            out.push((from, &line[from..at]));
        }
        (out, at + sep.len())
    });
    if words.1 < line.len() {
        words.0.push((words.1, &line[words.1..]));
    }
    let words = words.0;
    words.windows(2).find(|w| w[0].1 == "default" && w[1].1 == old).map(|w| (start + w[1].0, start + w[1].0 + old.len()))
}

struct Rename<'a> {
    enumeration: &'a str,
    old: &'a str,
    new: &'a str,
}

impl Rename<'_> {
    /// Whether a type is this enum.
    fn is_enum(&self, ty: &TypeRef) -> bool {
        matches!(&ty.kind, TypeRefKind::Named { name } if name == self.enumeration)
    }

    /// Whether a field of this enum starts at the value.
    fn defaults(&self, field: &Field) -> bool {
        matches!(&field.ty, FieldType::Enum { name } if name.name == self.enumeration) && field.modifiers.default == Some(Literal::Ident(self.old.to_string()))
    }

    // -----------------------------------------------------------------
    // Code

    /// The places in a module file's code that name the value: what the
    /// checker typed as it, and a default of a parameter or an option of the enum.
    fn in_code(&self, file: &SpecFile, types: &FileTypes) -> BTreeSet<(usize, usize)> {
        let mut found = BTreeSet::new();
        let mut bodies: Vec<&[Stmt]> = Vec::new();
        let mut defaults: Vec<(&TypeRef, &Expr)> = Vec::new();
        for block in &file.code {
            for handler in &block.handlers {
                bodies.push(&handler.body);
                defaults.extend(handler.params.iter().filter_map(|p| p.default.as_ref().map(|d| (&p.ty, d))));
            }
            bodies.extend(block.events.iter().map(|e| e.body.as_slice()));
        }
        let engine_functions = file.engines.iter().filter_map(|e| e.body.as_ref()).flat_map(|b| &b.functions);
        for function in file.functions.iter().chain(engine_functions) {
            bodies.push(&function.body);
            defaults.extend(function.params.iter().filter_map(|p| p.default.as_ref().map(|d| (&p.ty, d))));
        }
        for command in &file.commands {
            bodies.push(&command.body);
            defaults.extend(command.options.iter().filter_map(|o| o.default.as_ref().map(|d| (&o.ty, d))));
        }
        let mut look = |e: &Expr| {
            if types.name_of(e) != Some(&NameKind::EnumValue(self.enumeration.to_string())) {
                return;
            }
            match &e.kind {
                ExprKind::Name { name } if name == self.old => {
                    found.insert(range(e.span));
                }
                ExprKind::Member { member, .. } if member.name == self.old => {
                    found.insert(range(member.span));
                }
                _ => {}
            }
        };
        for body in bodies {
            crate::code::each_expr_in(body, &mut look);
        }
        for (ty, default) in defaults {
            if self.is_enum(ty)
                && matches!(&default.kind, ExprKind::Name { name } if name == self.old)
            {
                found.insert(range(default.span));
            }
        }
        found
    }

    // -----------------------------------------------------------------
    // A window's file

    /// A window's data default of the enum, and a click's value for a
    /// handler parameter of the enum; whether anything changed.
    fn in_designed(&self, project: &Project, designed: &mut Designed) -> bool {
        let Designed::Page(page) = designed else { return false };
        let mut changed = false;
        for data in &mut page.data {
            if let DataKind::Value { ty, default: Some(Literal::Ident(value)) } = &mut data.of
                && self.is_enum(ty)
                && value == self.old
            {
                *value = self.new.to_string();
                changed = true;
            }
        }
        // The handlers of the window's code, by name, with their parameters' types.
        let handlers: Vec<(String, Vec<TypeRef>)> = project
            .files
            .iter()
            .flat_map(|f| f.code.iter().filter(|b| b.name.name == page.name.name))
            .flat_map(|b| b.handlers.iter().map(|h| (h.name.name.clone(), h.params.iter().map(|p| p.ty.clone()).collect())))
            .collect();
        let mut all: Vec<&mut Widget> = page.widgets.iter_mut().collect();
        while let Some(widget) = all.pop() {
            for call in &mut widget.props.handlers {
                let Some((_, params)) = handlers.iter().find(|(name, _)| *name == call.handler) else { continue };
                for (at, arg) in call.args.iter_mut().enumerate() {
                    if *arg == Literal::Ident(self.old.to_string()) && params.get(at).is_some_and(|ty| self.is_enum(ty)) {
                        *arg = Literal::Ident(self.new.to_string());
                        changed = true;
                    }
                }
            }
            all.extend(widget.children.iter_mut());
            all.extend(widget.pages.iter_mut().flat_map(|p| p.children.iter_mut()));
        }
        changed
    }
}
