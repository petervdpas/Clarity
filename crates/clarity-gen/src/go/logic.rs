//! Functions, actions and data events as Go functions. Go has no `?`: a
//! call that can fail (a function or action, a query, loading the record a
//! reference points at) is lifted out of its expression into a statement of
//! its own, `v1, err := ...`, followed by `if err != nil`, as a Go developer
//! writes it. `and` and `or` keep stopping at the first side that decides:
//! when their right side needs lifted statements, those run inside an `if`.
//! Types come from the checker's table; the emitter only chooses how to
//! write them in Go. Records are worked on through pointers, so an action
//! changes the record it is given.

use std::collections::HashMap;
use std::fmt::Write as _;

use clarity_spec::Project;
use clarity_spec::code::check::{FileTypes, NameKind, TypeInfo};
use clarity_spec::code::{
    BinaryOp, Expr, ExprKind, FunctionDecl, FunctionKind, MessageLevel, OrderKey, Stmt, StmtKind, TextPart, TypeRef, TypeRefKind, UnaryOp,
    path_text, template,
};
use clarity_spec::model::{Command, FieldType, Frame, SpecFile, Widget};

use super::model::{constant, decimal_literal};
use super::names::{field, local, pascal, string, value_type, zero};
use crate::print;

/// The functions of `file`'s functions, actions and data events.
pub fn emit(project: &Project, file: &clarity_spec::model::SpecFile, types: &FileTypes) -> Result<String, Vec<String>> {
    let mut out = String::new();
    let mut errors = Vec::new();
    // Data events are actions on their record, run by the record's methods.
    let events: Vec<FunctionDecl> = file
        .code
        .iter()
        .filter(|b| file.entities.iter().any(|e| e.name.name == b.name.name))
        .flat_map(|b| b.events.iter().map(|e| e.action(&b.name.name)))
        .collect();
    for decl in file.functions.iter().chain(&events) {
        let mut emitter = Emitter::new(project, types, Some(decl), None);
        let code = emitter.function();
        if emitter.errors.is_empty() {
            out.push_str(&code);
        } else {
            errors.extend(emitter.errors.into_iter().map(|e| format!("{} '{}': {e}", decl.kind.keyword(), decl.name.name)));
        }
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// What a `TypeRef` names, as a checker type.
fn type_ref_info(project: &Project, ty: &TypeRef) -> TypeInfo {
    match &ty.kind {
        TypeRefKind::Text => TypeInfo::Text,
        TypeRefKind::Integer => TypeInfo::Integer,
        TypeRefKind::Duration => TypeInfo::Duration,
        TypeRefKind::Decimal => TypeInfo::Decimal,
        TypeRefKind::Money => TypeInfo::Money,
        TypeRefKind::Bool => TypeInfo::Bool,
        TypeRefKind::Date => TypeInfo::Date,
        TypeRefKind::DateTime => TypeInfo::DateTime,
        TypeRefKind::Named { name } if project.entity(name).is_some() => TypeInfo::Record(name.clone()),
        TypeRefKind::Named { name } => TypeInfo::Enum(name.clone()),
        TypeRefKind::List { item } => TypeInfo::List(item.clone()),
    }
}

fn is_decimal(ty: &TypeInfo) -> bool {
    matches!(ty, TypeInfo::Decimal | TypeInfo::Money)
}

/// Decimals and money: stored as text in SQLite, compared and ordered as numbers.
fn is_stored_number(ty: &FieldType) -> bool {
    matches!(ty, FieldType::Decimal { .. } | FieldType::Money)
}

/// The same comparison with its sides swapped.
fn flipped(op: BinaryOp) -> BinaryOp {
    match op {
        BinaryOp::Lt => BinaryOp::Gt,
        BinaryOp::LtEq => BinaryOp::GtEq,
        BinaryOp::Gt => BinaryOp::Lt,
        BinaryOp::GtEq => BinaryOp::LtEq,
        other => other,
    }
}

/// The runtime's name for a comparison.
fn op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Eq => "Eq",
        BinaryOp::NotEq => "NotEq",
        BinaryOp::Lt => "Lt",
        BinaryOp::LtEq => "LtEq",
        BinaryOp::Gt => "Gt",
        _ => "GtEq",
    }
}

/// `(a)` without its outer parentheses, when they belong together.
fn bare(code: String) -> String {
    if !(code.starts_with('(') && code.ends_with(')')) {
        return code;
    }
    let mut depth = 0;
    for (i, c) in code.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && i + 1 < code.len() {
                    return code;
                }
            }
            _ => {}
        }
    }
    code[1..code.len() - 1].to_string()
}

/// `lines` indented by `depth` tabs.
fn indent(lines: &str, depth: usize) -> String {
    let pad = "\t".repeat(depth);
    lines.lines().map(|l| if l.is_empty() { "\n".to_string() } else { format!("{pad}{l}\n") }).collect()
}

/// What a handler may read of a part.
#[derive(Debug, Clone, Default)]
pub struct PartInfo {
    /// The data a table or list shows, for its selected row.
    pub bind: Option<String>,
    pub caption: Option<String>,
    /// Whether the design makes the part read-only.
    pub readonly: bool,
}

/// The window or app a handler belongs to.
#[derive(Debug, Clone, Default)]
pub struct WindowCtx {
    pub parts: HashMap<String, PartInfo>,
    /// The window's data: name, entity and whether it is a list.
    pub data: Vec<(String, String, bool)>,
}

impl WindowCtx {
    fn widgets(&mut self, widgets: &[Widget]) {
        for widget in widgets {
            if let Some(name) = &widget.props.name {
                let info = PartInfo { bind: widget.props.bind.clone(), caption: widget.props.caption.clone(), readonly: widget.props.readonly };
                self.parts.insert(name.clone(), info);
            }
            self.widgets(&widget.children);
            for page in &widget.pages {
                self.widgets(&page.children);
            }
        }
    }

    fn commands(&mut self, commands: &[Command]) {
        for command in commands {
            if let Some(name) = &command.props.name {
                self.parts.insert(name.clone(), PartInfo { caption: command.props.caption.clone(), ..PartInfo::default() });
            }
            self.commands(&command.items);
        }
    }

    fn frame(&mut self, frame: &Frame) {
        if let Some(menu) = &frame.menu {
            self.commands(&menu.items);
        }
        if let Some(toolbar) = &frame.toolbar {
            self.commands(&toolbar.items);
        }
        if let Some(ribbon) = &frame.ribbon {
            for group in ribbon.tabs.iter().flat_map(|t| &t.groups) {
                self.commands(&group.items);
            }
        }
        for sidebar in &frame.sidebars {
            self.widgets(&sidebar.widgets);
        }
        if let Some(footer) = &frame.footer {
            self.widgets(&footer.widgets);
        }
    }
}

/// A window or the app, as main.go dispatches to it.
#[derive(Debug, Clone)]
pub struct Owner {
    pub name: String,
    /// The struct holding its data: `CustomerFormWindow`, `ShopApp`.
    pub struct_name: String,
    /// Handlers by Clarity name, with their method names.
    pub handlers: Vec<(String, String)>,
    pub is_app: bool,
    /// Its own events (open, close, return, start) with their handlers.
    pub events: Vec<(&'static str, String)>,
}

/// The struct, its loading and the handlers of every window and the app in `file`.
pub fn emit_windows(project: &Project, file: &SpecFile, types: &FileTypes) -> Result<(String, Vec<Owner>), Vec<String>> {
    let mut out = String::new();
    let mut owners = Vec::new();
    let mut errors = Vec::new();
    let pages = file.pages.iter().map(|p| (&p.name.name, &p.data, Some(p), None));
    let apps = file.apps.iter().map(|a| (&a.name.name, &a.data, None, Some(a)));
    for (name, data, page, app) in pages.chain(apps) {
        let struct_name = if app.is_some() { format!("{name}App") } else { format!("{name}Window") };
        let mut ctx = WindowCtx::default();
        let mut events = Vec::new();
        if let Some(page) = page {
            ctx.widgets(&page.widgets);
            ctx.frame(&page.frame);
            events.extend(page.handlers.iter().map(|h| (h.event.keyword(), h.handler.clone())));
        }
        if let Some(app) = app {
            ctx.frame(&app.frame);
            events.extend(app.handlers.iter().map(|h| (h.event.keyword(), h.handler.clone())));
        }
        ctx.data = data.iter().map(|d| (d.name.name.clone(), d.entity.name.clone(), d.list)).collect();

        let rows: Vec<Vec<String>> = data
            .iter()
            .map(|d| {
                let ty = if d.list { format!("[]{}", d.entity.name) } else { format!("*{}", d.entity.name) };
                vec![pascal(&d.name.name), ty, format!("`json:\"{}\"`", d.name.name)]
            })
            .collect();
        let mut load = String::new();
        for d in data {
            let (local_name, field_name, entity) = (local(&d.name.name), pascal(&d.name.name), &d.entity.name);
            if d.list {
                let _ = write!(load, "\t{local_name}, err := clarity.All[{entity}](db)\n\tif err != nil {{\n\t\treturn nil, err\n\t}}\n\tw.{field_name} = {local_name}\n");
            } else {
                let _ = write!(load, "\t{local_name} := New{entity}()\n\tw.{field_name} = &{local_name}\n");
            }
        }
        let body = if rows.is_empty() { String::new() } else { super::model::aligned(&rows, "\t") };
        let _ = write!(
            out,
            "// {struct_name} is the data of {name}, as its handlers see it.\ntype {struct_name} struct {{\n{body}}}\n\n// load{struct_name} is the data when {name} opens: every record of a list, a new record otherwise.\nfunc load{struct_name}(db *clarity.Db) (*{struct_name}, error) {{\n\tw := &{struct_name}{{}}\n{load}\treturn w, nil\n}}\n\n"
        );

        let mut handlers = Vec::new();
        for handler in file.code.iter().filter(|b| b.name.name == *name).flat_map(|b| &b.handlers) {
            let method = local(&handler.name.name);
            let mut emitter = Emitter::new(project, types, None, Some(&ctx));
            let mut code = String::new();
            emitter.block(&handler.body, &mut code);
            if !code.trim_end().ends_with("return nil") {
                code.push_str("return nil\n");
            }
            if emitter.errors.is_empty() {
                let _ = write!(out, "func (w *{struct_name}) {method}(db *clarity.Db, ui *clarity.Ui) error {{\n{}}}\n\n", indent(&code, 1));
                handlers.push((handler.name.name.clone(), method));
            } else {
                errors.extend(emitter.errors.into_iter().map(|e| format!("handler '{}' of {name}: {e}", handler.name.name)));
            }
        }
        owners.push(Owner { name: name.clone(), struct_name, handlers, is_app: app.is_some(), events });
    }
    if errors.is_empty() { Ok((out, owners)) } else { Err(errors) }
}

struct Emitter<'a> {
    project: &'a Project,
    types: &'a FileTypes,
    /// The function, action or data event being written; None for a handler.
    decl: Option<&'a FunctionDecl>,
    /// The window of a handler.
    window: Option<&'a WindowCtx>,
    errors: Vec<String>,
    /// Lifted values so far: `v1`, `v2`.
    temps: usize,
    /// Loops so far, for the names of their indexes.
    loops: usize,
    /// Whether the ensures are being written, where `result` is the result.
    ensuring: bool,
}

impl<'a> Emitter<'a> {
    fn new(project: &'a Project, types: &'a FileTypes, decl: Option<&'a FunctionDecl>, window: Option<&'a WindowCtx>) -> Self {
        Emitter { project, types, decl, window, errors: Vec::new(), temps: 0, loops: 0, ensuring: false }
    }

    fn returns(&self) -> Option<TypeInfo> {
        self.decl.and_then(|d| d.returns.as_ref()).map(|r| type_ref_info(self.project, r))
    }

    /// The statement that returns `err`.
    fn fail(&self, err: &str) -> String {
        match self.returns() {
            Some(ty) => format!("return {}, {err}", zero(&ty)),
            None => format!("return {err}"),
        }
    }

    fn function(&mut self) -> String {
        let decl = self.decl.expect("a function, action or data event");
        let mut params = vec!["db *clarity.Db".to_string()];
        for param in &decl.params {
            params.push(format!("{} {}", local(&param.name.name), value_type(&type_ref_info(self.project, &param.ty))));
        }
        let name = if decl.kind == FunctionKind::Event { local(&decl.name.name) } else { pascal(&decl.name.name) };
        let ret = self.returns().map(|r| format!("({}, error)", value_type(&r))).unwrap_or_else(|| "error".into());
        let mut body = String::new();
        for condition in &decl.requires {
            let mut pre = String::new();
            let holds = bare(self.value(condition, &mut pre));
            body.push_str(&pre);
            body.push_str(&format!(
                "if err := clarity.Require({holds}, {}); err != nil {{\n\t{}\n}}\n",
                string(&print::expr(condition)),
                self.fail("err")
            ));
        }
        if decl.ensures.is_empty() {
            self.block(&decl.body, &mut body);
            if self.returns().is_none() && !body.trim_end().ends_with("return nil") {
                body.push_str("return nil\n");
            }
        } else {
            // The body runs as a function of its own, so every return gives
            // the result the ensures check.
            let ret_ty = self.returns().map(|r| value_type(&r)).unwrap_or_default();
            let mut inner = String::new();
            self.block(&decl.body, &mut inner);
            body.push_str(&format!("body := func() ({ret_ty}, error) {{\n{}}}\n", indent(&inner, 1)));
            body.push_str(&format!("result, err := body()\nif err != nil {{\n\t{}\n}}\n", self.fail("err")));
            self.ensuring = true;
            for condition in &decl.ensures {
                let mut pre = String::new();
                let holds = bare(self.value(condition, &mut pre));
                body.push_str(&pre);
                body.push_str(&format!(
                    "if err := clarity.Ensure({holds}, {}); err != nil {{\n\t{}\n}}\n",
                    string(&print::expr(condition)),
                    self.fail("err")
                ));
            }
            self.ensuring = false;
            body.push_str("return result, nil\n");
        }
        format!("func {name}({}) {ret} {{\n{}}}\n\n", params.join(", "), indent(&body, 1))
    }

    fn block(&mut self, body: &[Stmt], out: &mut String) {
        let mut code = String::new();
        for stmt in body {
            self.stmt(stmt, &mut code);
        }
        out.push_str(&unused_locals(&code));
    }

    /// The lines that lift a call that can fail; gives the name of its value.
    fn lift(&mut self, call: String, pre: &mut String) -> String {
        self.temps += 1;
        let name = format!("v{}", self.temps);
        pre.push_str(&format!("{name}, err := {call}\nif err != nil {{\n\t{}\n}}\n", self.fail("err")));
        name
    }

    fn stmt(&mut self, stmt: &Stmt, out: &mut String) {
        let mut pre = String::new();
        match &stmt.kind {
            StmtKind::Let { name, ty, value } => {
                let declared = ty.as_ref().map(|t| type_ref_info(self.project, t));
                let target = local(&name.name);
                let code = match &declared {
                    Some(t) if is_decimal(t) => self.decimal(value, &mut pre),
                    Some(t) if matches!(value.kind, ExprKind::Empty) => {
                        out.push_str(&format!("var {target} {}\n", value_type(t)));
                        return;
                    }
                    _ => self.value(value, &mut pre),
                };
                let code = bare(code);
                // A lifted call gives its value to the local itself.
                let lifted = format!("{code}, err := ");
                if declared.is_none() && pre.contains(&lifted) && pre.matches(code.as_str()).count() == 1 {
                    out.push_str(&pre.replacen(&lifted, &format!("{target}, err := "), 1));
                    return;
                }
                out.push_str(&pre);
                match declared {
                    Some(t) if !is_decimal(&t) && matches!(t, TypeInfo::Integer | TypeInfo::Duration) => {
                        out.push_str(&format!("var {target} {} = {code}\n", value_type(&t)))
                    }
                    _ => out.push_str(&format!("{target} := {code}\n")),
                }
            }
            StmtKind::Assign { target, value } => {
                if let Some((part, property)) = self.part_property(target) {
                    let method = match property.as_str() {
                        "caption" => "SetCaption",
                        "error" => "SetError",
                        "visible" => "SetVisible",
                        "enabled" => "SetEnabled",
                        "readonly" => "SetReadonly",
                        "text" => "SetText",
                        "checked" => "SetChecked",
                        other => {
                            self.errors.push(format!("'{other}' of a part cannot be set"));
                            return;
                        }
                    };
                    // A caption or error is kept as written with its slots, and shown in the person's format.
                    let code = if matches!(property.as_str(), "caption" | "error") { self.shown(value, &mut pre) } else { bare(self.value(value, &mut pre)) };
                    out.push_str(&pre);
                    out.push_str(&format!("ui.{method}({}, {code})\n", string(&part)));
                    return;
                }
                // A record given to a reference field is stored as its id.
                if self.is_reference_field(target) {
                    let stored = self.stored_reference(target, &mut pre);
                    let record = if matches!(value.kind, ExprKind::Empty) {
                        "nil".to_string()
                    } else {
                        format!("clarity.RefTo({}.ID)", self.value(value, &mut pre))
                    };
                    out.push_str(&pre);
                    out.push_str(&format!("{stored} = {record}\n"));
                    return;
                }
                let to = self.types.type_of(target);
                let code = if is_decimal(&to) { self.decimal(value, &mut pre) } else { self.value(value, &mut pre) };
                let place = self.place(target, &mut pre);
                out.push_str(&pre);
                out.push_str(&format!("{place} = {}\n", bare(code)));
            }
            StmtKind::If { branches, otherwise } => self.if_chain(branches, otherwise.as_deref(), out),
            StmtKind::For { var, list, body } => {
                let list = bare(self.value(list, &mut pre));
                out.push_str(&pre);
                self.loops += 1;
                let index = if self.loops == 1 { "i".to_string() } else { format!("i{}", self.loops) };
                let item = local(&var.name);
                let mut inner = format!("{item} := &{list}[{index}]\n");
                self.block(body, &mut inner);
                out.push_str(&format!("for {index} := range {list} {{\n{}}}\n", indent(&unused_locals(&inner), 1)));
            }
            StmtKind::Return { value: Some(value) } => {
                let ty = self.returns().unwrap_or(TypeInfo::Unknown);
                let code = if is_decimal(&ty) { self.decimal(value, &mut pre) } else { self.value(value, &mut pre) };
                out.push_str(&pre);
                out.push_str(&format!("return {}, nil\n", bare(code)));
            }
            StmtKind::Return { value: None } => {
                out.push_str(if self.returns().is_some() { "return\n" } else { "return nil\n" });
            }
            StmtKind::Call { call } => {
                let ExprKind::Call { function, args } = &call.kind else {
                    self.errors.push("only a call can stand on its own".into());
                    return;
                };
                // A handler runs another handler of its window.
                if self.types.name_of(call) == Some(&NameKind::Handler) {
                    out.push_str(&format!("if err := w.{}(db, ui); err != nil {{\n\treturn err\n}}\n", local(&function.name)));
                    return;
                }
                let call_code = self.call_code(&function.name, args, &mut pre);
                out.push_str(&pre);
                let returns = self.project.files.iter().flat_map(|f| &f.functions).find(|f| f.name.name == function.name).and_then(|f| f.returns.as_ref());
                let take = if returns.is_some() { "_, err" } else { "err" };
                out.push_str(&format!("if {take} := {call_code}; err != nil {{\n\t{}\n}}\n", self.fail("err")));
            }
            StmtKind::Save { value } | StmtKind::Delete { value } => {
                let verb = if matches!(stmt.kind, StmtKind::Save { .. }) { "Save" } else { "Delete" };
                let record = bare(self.value(value, &mut pre));
                out.push_str(&pre);
                out.push_str(&format!("if err := clarity.{verb}(db, {record}); err != nil {{\n\t{}\n}}\n", self.fail("err")));
            }
            StmtKind::Refuse { text } => {
                let text = self.shown(text, &mut pre);
                out.push_str(&pre);
                out.push_str(&format!("{}\n", self.fail(&format!("clarity.Refuse({text})"))));
            }
            StmtKind::Message { .. } | StmtKind::Open { .. } | StmtKind::Close | StmtKind::Reload { .. } if self.window.is_none() => {
                self.errors.push("messages, windows and window data belong in handlers".into());
            }
            StmtKind::Message { level, text } => {
                let level = match level {
                    MessageLevel::Info => "Info",
                    MessageLevel::Warning => "Warning",
                    MessageLevel::Error => "Error",
                };
                let text = self.shown(text, &mut pre);
                out.push_str(&pre);
                out.push_str(&format!("ui.Message(clarity.{level}, {text})\n"));
            }
            StmtKind::Open { window, with } => {
                let args: Vec<String> = with.iter().map(|arg| format!("{}: {}", string(&arg.name.name), bare(self.value(&arg.value, &mut pre)))).collect();
                out.push_str(&pre);
                out.push_str(&format!("ui.Open({}, map[string]any{{{}}})\n", string(&window.name), args.join(", ")));
            }
            StmtKind::Close => out.push_str("ui.Close()\n"),
            StmtKind::Reload { data } => {
                let found = self.window.and_then(|w| w.data.iter().find(|(n, _, _)| *n == data.name)).cloned();
                match found {
                    Some((name, entity, true)) => {
                        let all = self.lift(format!("clarity.All[{entity}](db)"), &mut pre);
                        out.push_str(&pre);
                        out.push_str(&format!("w.{} = {all}\n", pascal(&name)));
                    }
                    Some((name, entity, false)) => {
                        let field_name = format!("w.{}", pascal(&name));
                        let mut inner = String::new();
                        let fresh = self.lift(format!("clarity.Get[{entity}](db, {field_name}.ID)"), &mut inner);
                        inner.push_str(&format!("{field_name} = &{fresh}\n"));
                        out.push_str(&format!("if {field_name}.ID != 0 {{\n{}}}\n", indent(&inner, 1)));
                    }
                    None => self.errors.push(format!("there is no data '{}' to reload", data.name)),
                }
            }
        }
    }

    /// `if`, `else if` and `else`. An `else if` whose condition needs lifted
    /// statements becomes an `else` holding them and an `if`.
    fn if_chain(&mut self, branches: &[clarity_spec::code::Branch], otherwise: Option<&[Stmt]>, out: &mut String) {
        let Some((first, rest)) = branches.split_first() else {
            if let Some(body) = otherwise {
                self.block(body, out);
            }
            return;
        };
        let mut pre = String::new();
        let condition = bare(self.value(&first.condition, &mut pre));
        out.push_str(&pre);
        let mut body = String::new();
        self.block(&first.body, &mut body);
        out.push_str(&format!("if {condition} {{\n{}}}", indent(&body, 1)));
        let mut rest = rest;
        loop {
            match rest.split_first() {
                Some((next, after)) => {
                    let mut pre = String::new();
                    let condition = bare(self.value(&next.condition, &mut pre));
                    if !pre.is_empty() {
                        let mut inner = pre;
                        let mut chain = String::new();
                        let mut body = String::new();
                        self.block(&next.body, &mut body);
                        chain.push_str(&format!("if {condition} {{\n{}}}", indent(&body, 1)));
                        let mut tail = String::new();
                        self.if_chain_rest(after, otherwise, &mut tail);
                        chain.push_str(&tail);
                        inner.push_str(&chain);
                        inner.push('\n');
                        out.push_str(&format!(" else {{\n{}}}\n", indent(&inner, 1)));
                        return;
                    }
                    let mut body = String::new();
                    self.block(&next.body, &mut body);
                    out.push_str(&format!(" else if {condition} {{\n{}}}", indent(&body, 1)));
                    rest = after;
                }
                None => {
                    if let Some(body) = otherwise {
                        let mut code = String::new();
                        self.block(body, &mut code);
                        out.push_str(&format!(" else {{\n{}}}", indent(&code, 1)));
                    }
                    out.push('\n');
                    return;
                }
            }
        }
    }

    /// The rest of a chain after an `if` that is inside an `else`: written
    /// as a chain of its own, starting with ` else`.
    fn if_chain_rest(&mut self, branches: &[clarity_spec::code::Branch], otherwise: Option<&[Stmt]>, out: &mut String) {
        if branches.is_empty() {
            if let Some(body) = otherwise {
                let mut code = String::new();
                self.block(body, &mut code);
                out.push_str(&format!(" else {{\n{}}}", indent(&code, 1)));
            }
            return;
        }
        let mut chain = String::new();
        self.if_chain(branches, otherwise, &mut chain);
        out.push_str(&format!(" else {{\n{}}}", indent(&chain, 1)));
    }

    /// Text the screen shows (a refusal) as the runtime's `Text`: the text as
    /// written with a typed value per slot.
    fn shown(&mut self, expr: &Expr, pre: &mut String) -> String {
        match &expr.kind {
            ExprKind::Str { value } => format!("clarity.PlainText({})", string(value)),
            ExprKind::Text { parts } => {
                let mut code = format!("clarity.NewText({})", string(&template(parts)));
                for part in parts {
                    if let TextPart::Slot { value } = part {
                        let slot = self.slot(value, pre);
                        code.push_str(&format!(".With({}, {slot})", string(&path_text(value))));
                    }
                }
                code
            }
            _ => {
                let ty = self.types.type_of(expr);
                let value = self.value(expr, pre);
                format!("clarity.PlainText({})", self.show(value, &ty))
            }
        }
    }

    /// A value as text, the way `"text" + value` shows it.
    fn show(&self, code: String, ty: &TypeInfo) -> String {
        match ty {
            TypeInfo::Text => code,
            TypeInfo::Enum(_) => format!("{code}.Label()"),
            _ => format!("clarity.Show({})", bare(code)),
        }
    }

    /// The value of a slot with what the screen needs to show it.
    fn slot(&mut self, expr: &Expr, pre: &mut String) -> String {
        let value = bare(self.value(expr, pre));
        let field = self.field_of(expr);
        let format = string(field.and_then(|f| f.modifiers.format.as_deref()).unwrap_or_default());
        let scale = match field.map(|f| &f.ty) {
            Some(FieldType::Decimal { scale, .. }) => scale.to_string(),
            _ => "-1".to_string(),
        };
        match self.types.type_of(expr) {
            TypeInfo::Text => format!("clarity.SlotText({value})"),
            TypeInfo::Integer => format!("clarity.SlotInteger({value}, {format})"),
            TypeInfo::Duration => format!("clarity.SlotDuration({value})"),
            TypeInfo::Decimal => format!("clarity.SlotDecimal({value}, {scale}, {format})"),
            TypeInfo::Money => format!("clarity.SlotMoney({value}, {format})"),
            TypeInfo::Date => format!("clarity.SlotDate({value}, {format})"),
            TypeInfo::DateTime => format!("clarity.SlotDateTime({value}, {format})"),
            TypeInfo::Bool => format!("clarity.SlotBool({value})"),
            TypeInfo::Enum(name) => format!("clarity.SlotEnum(string({value}), {})", string(&name)),
            other => {
                self.errors.push(format!("a slot cannot show {other:?}"));
                String::new()
            }
        }
    }

    /// The dictionary field a member path such as `order.total` reads.
    fn field_of(&self, expr: &Expr) -> Option<&'a clarity_spec::model::Field> {
        let ExprKind::Member { object, member } = &expr.kind else { return None };
        let TypeInfo::Record(entity) = self.types.type_of(object) else { return None };
        self.project.entity(&entity)?.field(&member.name)
    }

    fn field_type(&self, entity: &str, field_name: &str) -> Option<FieldType> {
        self.project.entity(entity).and_then(|e| e.field(field_name)).map(|f| f.ty.clone())
    }

    /// The column a field is stored in.
    fn column(&self, entity: &str, field_name: &str) -> String {
        self.project.entity(entity).map_or_else(|| field_name.to_string(), |e| e.column_of(field_name).to_string())
    }

    /// An expression as a place: what can be assigned or read in place.
    fn place(&mut self, expr: &Expr, pre: &mut String) -> String {
        if self.part_property(expr).is_some() {
            return self.read_part(expr, pre);
        }
        match &expr.kind {
            ExprKind::Name { name } => match self.types.name_of(expr) {
                Some(NameKind::Local) if self.ensuring && name == "result" => "result".into(),
                Some(NameKind::Local) => local(name),
                Some(NameKind::Data) if self.window.is_some() => format!("w.{}", pascal(name)),
                Some(NameKind::EnumValue(e)) => constant(e, name),
                _ => {
                    self.errors.push(format!("'{name}' is not generated yet for Go"));
                    String::new()
                }
            },
            ExprKind::Member { object, member } => {
                if let Some(NameKind::EnumValue(e)) = self.types.name_of(expr) {
                    return constant(e, &member.name);
                }
                let TypeInfo::Record(entity) = self.types.type_of(object) else {
                    self.errors.push(format!("'{}' is not generated yet for Go", print::expr(expr)));
                    return String::new();
                };
                let object = self.place(object, pre);
                match self.field_type(&entity, &member.name) {
                    // Every record has an id.
                    None if member.name == clarity_spec::code::check::RECORD_ID => format!("{object}.ID"),
                    Some(FieldType::Ref { entity: target }) => {
                        self.lift(format!("clarity.Ref[{}](db, {object}.{})", target.name, field(&member.name)), pre)
                    }
                    Some(FieldType::List { .. }) => {
                        self.errors.push(format!("list fields like '{}' are not loaded yet", print::expr(expr)));
                        String::new()
                    }
                    _ => format!("{object}.{}", field(&member.name)),
                }
            }
            _ => self.value(expr, pre),
        }
    }

    /// A part property such as `grid.selected` or `name_input.error`: the
    /// part's name and the property, when `expr` is one.
    fn part_property(&self, expr: &Expr) -> Option<(String, String)> {
        let ExprKind::Member { object, member } = &expr.kind else { return None };
        let ExprKind::Name { name } = &object.kind else { return None };
        (self.types.name_of(object) == Some(&NameKind::Part)).then(|| (name.clone(), member.name.clone()))
    }

    /// Reading a part property in a handler; the selected row is a record.
    fn read_part(&mut self, expr: &Expr, pre: &mut String) -> String {
        let (part, property) = self.part_property(expr).unwrap_or_default();
        let info = self.window.and_then(|w| w.parts.get(&part)).cloned().unwrap_or_default();
        let quoted = string(&part);
        match property.as_str() {
            "visible" => format!("ui.Visible({quoted})"),
            "enabled" => format!("ui.Enabled({quoted})"),
            "caption" => format!("ui.Caption({quoted}, {})", string(info.caption.as_deref().unwrap_or_default())),
            "error" => format!("ui.Error({quoted})"),
            "readonly" => format!("ui.Readonly({quoted}, {})", info.readonly),
            "text" => format!("ui.Text({quoted})"),
            "checked" => format!("ui.Checked({quoted})"),
            "selected" => match info.bind {
                Some(bind) => self.lift(format!("clarity.Selected(ui, {quoted}, w.{})", pascal(&bind)), pre),
                None => {
                    self.errors.push(format!("'{part}' is not bound to a list"));
                    String::new()
                }
            },
            other => {
                self.errors.push(format!("'{other}' of a part is not generated yet for Go"));
                String::new()
            }
        }
    }

    /// An expression as a decimal: an integer is widened.
    fn decimal(&mut self, expr: &Expr, pre: &mut String) -> String {
        if let ExprKind::Number { value } = &expr.kind {
            return decimal_literal(value);
        }
        let ty = self.types.type_of(expr);
        let code = self.value(expr, pre);
        if is_decimal(&ty) { code } else { format!("decimal.NewFromInt({})", bare(code)) }
    }

    /// An expression as a value.
    fn value(&mut self, expr: &Expr, pre: &mut String) -> String {
        let ty = self.types.type_of(expr);
        match &expr.kind {
            ExprKind::Number { value } if is_decimal(&ty) => decimal_literal(value),
            ExprKind::Number { value } => value.clone(),
            ExprKind::Str { value } => string(value),
            ExprKind::Text { .. } => {
                self.errors.push("text with slots is text to show".into());
                String::new()
            }
            ExprKind::Bool { value } => value.to_string(),
            ExprKind::Empty => "nil".to_string(),
            ExprKind::Name { .. } | ExprKind::Member { .. } => self.place(expr, pre),
            ExprKind::Unary { op: UnaryOp::Not, operand } => format!("!{}", self.value(operand, pre)),
            ExprKind::Unary { op: UnaryOp::Neg, operand } if is_decimal(&ty) => format!("{}.Neg()", self.value(operand, pre)),
            ExprKind::Unary { op: UnaryOp::Neg, operand } => format!("-{}", self.value(operand, pre)),
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right, &ty, pre),
            ExprKind::In { value, options } => {
                let value_ty = self.types.type_of(value);
                if is_decimal(&value_ty) || options.iter().any(|o| is_decimal(&self.types.type_of(o))) {
                    let v = self.decimal(value, pre);
                    let opts: Vec<String> = options.iter().map(|o| self.decimal(o, pre)).collect();
                    return format!("clarity.OneOfDecimals({}, {})", bare(v), opts.join(", "));
                }
                let v = self.value(value, pre);
                let opts: Vec<String> = options.iter().map(|o| bare(self.value(o, pre))).collect();
                format!("slices.Contains([]{}{{{}}}, {})", value_type(&value_ty), opts.join(", "), bare(v))
            }
            ExprKind::Call { function, args } => self.call(&function.name, args, pre),
            ExprKind::Find { entity, filter, order, var } => {
                let query = self.find(&var.name, &entity.name, filter.as_deref(), order, pre);
                self.lift(query, pre)
            }
        }
    }

    /// Whether a date or reference field is empty (`x == none`); a record
    /// that is no field is none until it is stored.
    fn is_none(&mut self, expr: &Expr, pre: &mut String) -> String {
        if self.is_reference_field(expr) {
            return format!("{} == nil", self.stored_reference(expr, pre));
        }
        if matches!(self.types.type_of(expr), TypeInfo::Record(_)) {
            return format!("{}.ID == 0", self.value(expr, pre));
        }
        format!("{} == nil", self.value(expr, pre))
    }

    fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr, result: &TypeInfo, pre: &mut String) -> String {
        if matches!(op, BinaryOp::Eq | BinaryOp::NotEq) {
            let other = match (&left.kind, &right.kind) {
                (_, ExprKind::Empty) => Some(left),
                (ExprKind::Empty, _) => Some(right),
                _ => None,
            };
            if let Some(other) = other {
                let test = self.is_none(other, pre);
                return if op == BinaryOp::Eq { format!("({test})") } else { format!("!({test})") };
            }
        }
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            return self.logical(op, left, right, pre);
        }
        let (lt, rt) = (self.types.type_of(left), self.types.type_of(right));
        match op {
            BinaryOp::Contains => {
                let l = bare(self.value(left, pre));
                let r = bare(self.value(right, pre));
                format!("clarity.TextContains({l}, {r})")
            }
            BinaryOp::Add if *result == TypeInfo::Text => {
                let l = self.value(left, pre);
                let r = self.value(right, pre);
                format!("({} + {})", self.show(l, &lt), self.show(r, &rt))
            }
            _ if is_decimal(result) || is_decimal(&lt) || is_decimal(&rt) => {
                let l = self.decimal(left, pre);
                let r = self.decimal(right, pre);
                let method = match op {
                    BinaryOp::Add => "Add",
                    BinaryOp::Sub => "Sub",
                    BinaryOp::Mul => "Mul",
                    BinaryOp::Div => "Div",
                    BinaryOp::Eq | BinaryOp::NotEq => "Equal",
                    BinaryOp::Lt => "LessThan",
                    BinaryOp::LtEq => "LessThanOrEqual",
                    BinaryOp::Gt => "GreaterThan",
                    _ => "GreaterThanOrEqual",
                };
                let negate = if op == BinaryOp::NotEq { "!" } else { "" };
                format!("{negate}{l}.{method}({})", bare(r))
            }
            _ if matches!(lt, TypeInfo::Date | TypeInfo::DateTime) => {
                let compare = if lt == TypeInfo::Date { "CompareDates" } else { "CompareDateTimes" };
                let l = bare(self.value(left, pre));
                let r = bare(self.value(right, pre));
                format!("(clarity.{compare}({l}, {r}) {} 0)", op.symbol())
            }
            _ if matches!(lt, TypeInfo::Record(_)) => {
                let l = self.value(left, pre);
                let r = self.value(right, pre);
                format!("({l}.ID {} {r}.ID)", op.symbol())
            }
            _ => {
                let l = self.value(left, pre);
                let r = self.value(right, pre);
                format!("({l} {} {r})", op.symbol())
            }
        }
    }

    /// `and` and `or`, stopping at the left side when it decides.
    fn logical(&mut self, op: BinaryOp, left: &Expr, right: &Expr, pre: &mut String) -> String {
        let symbol = if op == BinaryOp::And { "&&" } else { "||" };
        let l = self.value(left, pre);
        let before = self.temps;
        self.temps += 1;
        let held = format!("v{}", self.temps);
        let mut right_pre = String::new();
        let r = self.value(right, &mut right_pre);
        if right_pre.is_empty() {
            self.temps = before;
            return format!("({l} {symbol} {r})");
        }
        let test = if op == BinaryOp::And { held.clone() } else { format!("!{held}") };
        pre.push_str(&format!("{held} := {}\nif {test} {{\n{}\t{held} = {}\n}}\n", bare(l), indent(&right_pre, 1), bare(r)));
        held
    }

    /// A call that can fail, without lifting it.
    fn call_code(&mut self, name: &str, args: &[Expr], pre: &mut String) -> String {
        let Some(callee) = self.project.files.iter().flat_map(|f| &f.functions).find(|f| f.name.name == name) else {
            self.errors.push(format!("unknown function '{name}'"));
            return String::new();
        };
        let mut code = vec!["db".to_string()];
        for (arg, param) in args.iter().zip(&callee.params) {
            let ty = type_ref_info(self.project, &param.ty);
            let value = if is_decimal(&ty) { self.decimal(arg, pre) } else { self.value(arg, pre) };
            code.push(bare(value));
        }
        format!("{}({})", pascal(name), code.join(", "))
    }

    fn call(&mut self, name: &str, args: &[Expr], pre: &mut String) -> String {
        use clarity_spec::code::check::{NOW, TODAY};
        if name == TODAY {
            return "clarity.Today()".into();
        }
        if name == NOW {
            return "clarity.Now()".into();
        }
        if name == "count" {
            let list = args.first().map(|a| bare(self.value(a, pre))).unwrap_or_default();
            return format!("int64(len({list}))");
        }
        let call = self.call_code(name, args, pre);
        self.lift(call, pre)
    }

    /// `find c in Entity where ... order by ...` as a database query.
    fn find(&mut self, var: &str, entity: &str, filter: Option<&Expr>, order: &[OrderKey], pre: &mut String) -> String {
        let mut query = "clarity.Query{}".to_string();
        if let Some(filter) = filter {
            let cond = self.cond(filter, var, entity, pre);
            query.push_str(&format!(".Filter({cond})"));
        }
        for key in order {
            let Some(path) = crate::query::order_path(self.project, entity, key) else {
                self.errors.push(format!("{entity} cannot be ordered by '{}'", key.field.name));
                continue;
            };
            let numeric = self.field_type(&path.entity, &path.field).is_some_and(|t| is_stored_number(&t));
            let column = string(&self.column(&path.entity, &path.field));
            if path.hops.is_empty() {
                let method = if numeric { "OrderNumber" } else { "Order" };
                query.push_str(&format!(".{method}({column}, {})", key.descending));
            } else {
                let hops: Vec<String> = path
                    .hops
                    .iter()
                    .map(|h| format!("{{Column: {}, Table: {}, Key: {}}}", string(&h.column), string(&h.table), string(&h.key)))
                    .collect();
                query.push_str(&format!(".OrderThrough([]clarity.Hop{{{}}}, {column}, {}, {numeric})", hops.join(", "), key.descending));
            }
        }
        format!("clarity.Find[{entity}](db, {query})")
    }

    /// A query condition as a `clarity.Cond`.
    fn cond(&mut self, expr: &Expr, var: &str, entity: &str, pre: &mut String) -> String {
        let project = self.project;
        let path_of = |e: &Expr| crate::query::field_path(project, entity, var, e);
        match &expr.kind {
            ExprKind::Binary { op: BinaryOp::And, left, right } => {
                let l = self.cond(left, var, entity, pre);
                format!("clarity.And({l}, {})", self.cond(right, var, entity, pre))
            }
            ExprKind::Binary { op: BinaryOp::Or, left, right } => {
                let l = self.cond(left, var, entity, pre);
                format!("clarity.Or({l}, {})", self.cond(right, var, entity, pre))
            }
            ExprKind::Unary { op: UnaryOp::Not, operand } => format!("clarity.Not({})", self.cond(operand, var, entity, pre)),
            ExprKind::Binary { op, left, right } => {
                let (path, value, op) = match (path_of(left), path_of(right)) {
                    (Some(path), _) => (path, right, *op),
                    (None, Some(path)) => (path, left, flipped(*op)),
                    (None, None) => {
                        self.errors.push(format!("'{}' compares no field of {var}", print::expr(expr)));
                        return String::new();
                    }
                };
                let Some(ty) = self.field_type(&path.entity, &path.field) else {
                    self.errors.push(format!("{} has no field '{}'", path.entity, path.field));
                    return String::new();
                };
                let name = string(&self.column(&path.entity, &path.field));
                let own = if matches!(value.kind, ExprKind::Empty) {
                    let test = format!("clarity.IsNone({name})");
                    if op == BinaryOp::Eq { test } else { format!("clarity.Not({test})") }
                } else if op == BinaryOp::Contains {
                    format!("clarity.Contains({name}, {})", bare(self.value(value, pre)))
                } else {
                    let constructor = if is_stored_number(&ty) { "Number" } else { "Compare" };
                    let value = self.sql_value(&ty, value, pre);
                    format!("clarity.{constructor}({name}, clarity.{}, {value})", op_name(op))
                };
                through(&path.hops, own)
            }
            ExprKind::In { value, options } => {
                let Some(path) = path_of(value) else {
                    self.errors.push(format!("'{}' compares no field of {var}", print::expr(expr)));
                    return String::new();
                };
                let Some(ty) = self.field_type(&path.entity, &path.field) else {
                    self.errors.push(format!("{} has no field '{}'", path.entity, path.field));
                    return String::new();
                };
                let constructor = if is_stored_number(&ty) { "OneOfNumbers" } else { "OneOf" };
                let values: Vec<String> = options.iter().map(|o| self.sql_value(&ty, o, pre)).collect();
                let own =
                    format!("clarity.{constructor}({}, []any{{{}}})", string(&self.column(&path.entity, &path.field)), values.join(", "));
                through(&path.hops, own)
            }
            _ => match path_of(expr) {
                Some(path) => through(&path.hops, format!("clarity.IsTrue({})", string(&self.column(&path.entity, &path.field)))),
                None => {
                    self.errors.push(format!("'{}' is not a condition on {var}", print::expr(expr)));
                    String::new()
                }
            },
        }
    }

    /// A value compared with a field of type `ty`, as the runtime takes it.
    fn sql_value(&mut self, ty: &FieldType, value: &Expr, pre: &mut String) -> String {
        let from = self.types.type_of(value);
        match ty {
            FieldType::Decimal { .. } | FieldType::Money => bare(self.decimal(value, pre)),
            FieldType::Enum { .. } => format!("string({})", bare(self.value(value, pre))),
            FieldType::Ref { .. } => match from {
                // A reference field holds the id already; a record has its own.
                TypeInfo::Record(_) if self.is_reference_field(value) => self.stored_reference(value, pre),
                TypeInfo::Record(_) => format!("{}.ID", self.value(value, pre)),
                _ => bare(self.value(value, pre)),
            },
            FieldType::List { .. } => {
                self.errors.push("a list field cannot be compared in a query".into());
                String::new()
            }
            _ => bare(self.value(value, pre)),
        }
    }

    /// Whether `expr` reads a reference field, such as `order.customer`.
    fn is_reference_field(&self, expr: &Expr) -> bool {
        let ExprKind::Member { object, member } = &expr.kind else { return false };
        let TypeInfo::Record(owner) = self.types.type_of(object) else { return false };
        matches!(self.field_type(&owner, &member.name), Some(FieldType::Ref { .. }))
    }

    /// The id a reference field holds, without loading the record.
    fn stored_reference(&mut self, expr: &Expr, pre: &mut String) -> String {
        let ExprKind::Member { object, member } = &expr.kind else { return String::new() };
        format!("{}.{}", self.place(object, pre), field(&member.name))
    }
}

/// `code` with `_ = name` after each local it declares and never uses, as
/// Go refuses unused locals.
fn unused_locals(code: &str) -> String {
    let mut out = String::new();
    let lines: Vec<&str> = code.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        out.push_str(line);
        out.push('\n');
        let trimmed = line.trim_start();
        let Some(name) = trimmed.split_once(" := ").map(|(n, _)| n).filter(|n| is_ident(n)) else { continue };
        if !name.starts_with('v') || !name[1..].chars().all(|c| c.is_ascii_digit()) {
            let used = lines[i + 1..].iter().any(|l| mentions(l, name)) || mentions(trimmed.split_once(" := ").map(|(_, v)| v).unwrap_or_default(), name);
            if !used {
                let pad: String = line.chars().take_while(|c| *c == '\t').collect();
                out.push_str(&format!("{pad}_ = {name}\n"));
            }
        }
    }
    out
}

fn is_ident(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Whether `line` uses the name `name` as a word.
fn mentions(line: &str, name: &str) -> bool {
    let bytes = line.as_bytes();
    let mut start = 0;
    while let Some(at) = line[start..].find(name) {
        let at = start + at;
        let before = at == 0 || !(bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_' || bytes[at - 1] == b'.');
        let end = at + name.len();
        let after = end >= bytes.len() || !(bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_');
        if before && after {
            return true;
        }
        start = end;
    }
    false
}

/// A condition on the record the steps lead to, as one on the query's own
/// record: each step a `clarity.Through`, the first one outermost.
fn through(hops: &[crate::query::Hop], cond: String) -> String {
    hops.iter().rev().fold(cond, |inner, h| {
        format!("clarity.Through({}, {}, {}, {inner})", string(&h.column), string(&h.table), string(&h.key))
    })
}
