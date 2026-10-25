//! Calls in full (DESIGN 5.24): before a stack is written, every call to a
//! function or action is given all its values in order, those given by
//! name put in their place and those left out filled in from the defaults.
//! So no stack needs default or named parameters of its own language.
//!
//! A default is copied into the calling code. Generation checks the code
//! again for its types, which are kept by the place of each expression in
//! its file; a copy keeps no place of the file it came from, but gets one
//! of its own past the end of every file, so it is never taken for another
//! expression.

use std::collections::HashMap;

use clarity_spec::Project;
use clarity_spec::code::{Expr, ExprKind, FunctionDecl, Param, each_expr_in_mut, each_expr_mut};
use clarity_spec::span::Span;

/// Where the places of the copied defaults start: beyond any file's length.
const COPIES_FROM: u32 = 0xF000_0000;

/// `project` with every call in full.
pub fn filled(project: &Project) -> Project {
    let mut project = project.clone();
    let signatures: HashMap<String, Vec<Param>> =
        project.files.iter().flat_map(|f| &f.functions).map(|f| (f.name.name.clone(), f.params.clone())).collect();
    let mut next = COPIES_FROM;
    for file in &mut project.files {
        for block in &mut file.code {
            for handler in &mut block.handlers {
                fill(&mut handler.body, &signatures, &mut next);
            }
            for event in &mut block.events {
                fill(&mut event.body, &signatures, &mut next);
            }
        }
        for function in &mut file.functions {
            fill_function(function, &signatures, &mut next);
        }
        for command in &mut file.commands {
            fill(&mut command.body, &signatures, &mut next);
        }
        for engine in &mut file.engines {
            let Some(body) = &mut engine.body else { continue };
            // An engine's own functions come before the project's.
            let mut own = signatures.clone();
            own.extend(body.functions.iter().map(|f| (f.name.name.clone(), f.params.clone())));
            for keep in &mut body.keeps {
                each_expr_mut(&mut keep.value, &mut |e| fill_call(e, &own, &mut next));
            }
            for function in &mut body.functions {
                fill_function(function, &own, &mut next);
            }
        }
    }
    project
}

fn fill_function(function: &mut FunctionDecl, signatures: &HashMap<String, Vec<Param>>, next: &mut u32) {
    fill(&mut function.body, signatures, next);
    for condition in function.requires.iter_mut().chain(function.ensures.iter_mut()) {
        each_expr_mut(condition, &mut |e| fill_call(e, signatures, next));
    }
}

fn fill(body: &mut [clarity_spec::code::Stmt], signatures: &HashMap<String, Vec<Param>>, next: &mut u32) {
    each_expr_in_mut(body, &mut |e| fill_call(e, signatures, next));
}

/// Puts a call's named values in their places and fills in the defaults.
/// A call the checker passed has every parameter once.
fn fill_call(expr: &mut Expr, signatures: &HashMap<String, Vec<Param>>, next: &mut u32) {
    let ExprKind::Call { function, args, named } = &mut expr.kind else { return };
    let Some(params) = signatures.get(&function.name) else { return };
    if named.is_empty() && args.len() >= params.len() {
        return;
    }
    let mut given: Vec<Option<Expr>> = std::mem::take(args).into_iter().map(Some).collect();
    given.resize(params.len(), None);
    for arg in std::mem::take(named) {
        if let Some(at) = params.iter().position(|p| p.name.name == arg.name.name) {
            given[at] = Some(arg.value);
        }
    }
    *args = given
        .into_iter()
        .zip(params)
        .filter_map(|(value, param)| value.or_else(|| param.default.as_ref().map(|d| placed(d, next))))
        .collect();
}

/// A copy of `default` with places of its own.
fn placed(default: &Expr, next: &mut u32) -> Expr {
    let mut copy = default.clone();
    each_expr_mut(&mut copy, &mut |e| {
        e.span = Span { start: *next, end: *next + 1, ..e.span };
        *next += 2;
    });
    copy
}
