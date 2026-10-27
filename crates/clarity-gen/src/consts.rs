//! Constants written out (DESIGN 5.12): before a stack is written, every
//! use of a constant becomes its value, so no stack needs constants of its
//! own. Like a call's default (crate::calls), the value is copied with
//! places of its own, past the end of every file, since generation checks
//! the code again for its types.

use clarity_spec::Project;
use clarity_spec::code::check::{FileTypes, NameKind};
use clarity_spec::code::{ConstDecl, Expr, ExprKind, Stmt, TypeRefKind, each_expr_in_mut, each_expr_mut};
use clarity_spec::span::Span;

/// Where the places of the copied values start: beyond any file's length,
/// below where the copied defaults start.
const COPIES_FROM: u32 = 0xE000_0000;

/// `project` with every constant's use as its value.
pub fn inlined(project: &Project) -> Project {
    let types = clarity_spec::code::check::types(&project.files);
    let all: Vec<(Option<String>, ConstDecl)> = project.files.iter().flat_map(|f| f.consts.iter().map(|c| (f.module.as_ref().map(|m| m.name.clone()), c.clone()))).collect();
    let mut project = project.clone();
    let mut next = COPIES_FROM;
    for file in &mut project.files {
        let types = types.get(&file.path).cloned().unwrap_or_default();
        let module = file.module.as_ref().map(|m| m.name.clone());
        // The module's own constants first, then those of the modules it uses.
        let mut visible: Vec<ConstDecl> = all.iter().filter(|(m, _)| *m == module).map(|(_, c)| c.clone()).collect();
        visible.extend(all.iter().filter(|(m, _)| *m != module).map(|(_, c)| c.clone()));
        let rewrite = |body: &mut [Stmt], own: &[ConstDecl], next: &mut u32| each_expr_in_mut(body, &mut |e| written(e, own, &visible, &types, next));
        for block in &mut file.code {
            for handler in &mut block.handlers {
                rewrite(&mut handler.body, &[], &mut next);
            }
            for event in &mut block.events {
                rewrite(&mut event.body, &[], &mut next);
            }
        }
        for function in &mut file.functions {
            rewrite(&mut function.body, &[], &mut next);
            for condition in function.requires.iter_mut().chain(function.ensures.iter_mut()) {
                each_expr_mut(condition, &mut |e| written(e, &[], &visible, &types, &mut next));
            }
        }
        for command in &mut file.commands {
            rewrite(&mut command.body, &[], &mut next);
        }
        for engine in &mut file.engines {
            let Some(body) = &mut engine.body else { continue };
            let own = body.consts.clone();
            for keep in &mut body.keeps {
                each_expr_mut(&mut keep.value, &mut |e| written(e, &own, &visible, &types, &mut next));
            }
            for function in &mut body.functions {
                rewrite(&mut function.body, &own, &mut next);
                for condition in function.requires.iter_mut().chain(function.ensures.iter_mut()) {
                    each_expr_mut(condition, &mut |e| written(e, &own, &visible, &types, &mut next));
                }
            }
        }
    }
    project
}

/// A use of a constant as its value: an engine's own constant first.
fn written(expr: &mut Expr, own: &[ConstDecl], visible: &[ConstDecl], types: &FileTypes, next: &mut u32) {
    let ExprKind::Name { name } = &expr.kind else { return };
    if types.name_of(expr) != Some(&NameKind::Const) {
        return;
    }
    let Some(decl) = own.iter().chain(visible).find(|c| c.name.name == *name) else { return };
    let mut value = decl.value.clone();
    // A whole number in a decimal constant stays a decimal where it is written.
    if let (Some(TypeRefKind::Decimal | TypeRefKind::Money), ExprKind::Number { value: number }) = (decl.ty.as_ref().map(|t| &t.kind), &mut value.kind)
        && !number.contains('.')
    {
        number.push_str(".0");
    }
    each_expr_mut(&mut value, &mut |e| {
        e.span = Span { start: *next, end: *next + 1, ..e.span };
        *next += 2;
    });
    *expr = value;
}
