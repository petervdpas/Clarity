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
    AskForm, BinaryOp, EngineBody, EngineDecl, EngineState, Expr, ExprKind, FunctionDecl, FunctionKind, MessageLevel, OrderKey, Stmt, StmtKind, TextPart, TypeRef,
    UnaryOp, path_text, template,
};
use clarity_spec::model::{Command, DataKind, FieldType, Frame, SpecFile, Widget};

use super::model::{constant, decimal_literal};
use super::names::{field, function as function_name, local, pascal, string, value_type, zero};
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
    for engine in &file.engines {
        if let Some(body) = &engine.body {
            match emit_engine(project, types, engine, body) {
                Ok(code) => out.push_str(&code),
                Err(mut problems) => errors.append(&mut problems),
            }
        }
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// The Go type of what an engine written in Clarity keeps: `boardKept`.
fn kept_type(engine: &str) -> String {
    format!("{}Kept", local(engine))
}

/// An engine written in Clarity: a struct of what it keeps, the kept values
/// of each instance (a window, or 0 for the app), and per function an entry
/// that finds the instance's values and an inner one that works on them. A
/// call works on a copy that is kept only when the call succeeds, so a call
/// that fails or refuses changes nothing.
fn emit_engine(project: &Project, types: &FileTypes, engine: &EngineDecl, body: &EngineBody) -> Result<String, Vec<String>> {
    let name = &engine.name.name;
    let var = local(name);
    let kept = kept_type(name);
    let mut errors = Vec::new();
    let mut fields = String::new();
    let mut start = String::new();
    let mut copies = String::new();
    // Field types line up, as gofmt writes them.
    let width = body.keeps.iter().map(|k| local(&k.name.name).len()).max().unwrap_or(0);
    for keep in &body.keeps {
        let info = type_ref_info(project, &keep.ty);
        let field = local(&keep.name.name);
        fields.push_str(&format!("\t{field:width$} {}\n", value_type(&info)));
        let mut emitter = Emitter::new(project, types, None, None);
        emitter.engine = Some(engine);
        let mut pre = String::new();
        let value = if is_decimal(&info) { emitter.decimal(&keep.value, &mut pre) } else { emitter.value(&keep.value, &mut pre) };
        if emitter.errors.is_empty() {
            start.push_str(&pre);
            start.push_str(&format!("k.{field} = {}\n", bare(value)));
        } else {
            errors.extend(emitter.errors.into_iter().map(|e| format!("engine {name} keeps {}: {e}", keep.name.name)));
        }
        if matches!(info, TypeInfo::Values(_)) {
            copies.push_str(&format!("\tc.{field} = slices.Clone(k.{field})\n"));
        }
    }
    let stored = engine.state != EngineState::None;
    let mut out = format!(
        "// {kept} is what engine {name} keeps between calls.\ntype {kept} struct {{\n{fields}}}\n\n// start gives a new instance what it starts with.\nfunc (k *{kept}) start(db *clarity.Db) error {{\n{}\treturn nil\n}}\n\n",
        indent(&start, 1)
    );
    if stored {
        out.push_str(&format!(
            "// copy is a copy of its own to work on.\nfunc (k *{kept}) copy() *{kept} {{\n\tc := *k\n{copies}\treturn &c\n}}\n\nvar {var}All = map[uint64]*{kept}{{}}\nvar {var}Lock sync.Mutex\n\n// A closed window's values go.\nfunc init() {{\n\tclarity.OnWindowClosed({var}Close)\n}}\n\n// {var}Close drops what instance (a window, or 0 for the app) kept of engine {name}.\nfunc {var}Close(instance uint64) {{\n\t{var}Lock.Lock()\n\tdefer {var}Lock.Unlock()\n\tdelete({var}All, instance)\n}}\n\n"
        ));
    }
    for decl in &body.functions {
        let mut emitter = Emitter::new(project, types, Some(decl), None);
        emitter.engine = Some(engine);
        let inner = emitter.function();
        if !emitter.errors.is_empty() {
            errors.extend(emitter.errors.into_iter().map(|e| format!("function '{}' of engine {name}: {e}", decl.name.name)));
            continue;
        }
        let wrapper = super::engine_function(name, &decl.name.name);
        let params: String = decl.params.iter().map(|p| format!(", {} {}", local(&p.name.name), value_type(&type_ref_info(project, &p.ty)))).collect();
        let names: String = decl.params.iter().map(|p| format!(", {}", local(&p.name.name))).collect();
        let returns = decl.returns.as_ref().map(|r| type_ref_info(project, r));
        let (ret, fail, answer, give) = match &returns {
            Some(ty) => (format!("({}, error)", value_type(ty)), format!("return {}, err", zero(ty)), "answer, err := ", "return answer, nil"),
            None => ("error".to_string(), "return err".to_string(), "err := ", "return nil"),
        };
        let run = if stored {
            format!(
                "\t{var}Lock.Lock()\n\tdefer {var}Lock.Unlock()\n\tvar kept *{kept}\n\tif found, ok := {var}All[instance]; ok {{\n\t\tkept = found.copy()\n\t}} else {{\n\t\tkept = &{kept}{{}}\n\t\tif err := kept.start(db); err != nil {{\n\t\t\t{fail}\n\t\t}}\n\t}}\n\t{answer}{wrapper}With(db, kept{names})\n\tif err != nil {{\n\t\t{fail}\n\t}}\n\t{var}All[instance] = kept\n\t{give}\n"
            )
        } else {
            format!("\tkept := &{kept}{{}}\n\tif err := kept.start(db); err != nil {{\n\t\t{fail}\n\t}}\n\treturn {wrapper}With(db, kept{names})\n")
        };
        out.push_str(&format!(
            "// {wrapper} is {name}.{} in code, for instance (a window, or 0 for the app).\nfunc {wrapper}(db *clarity.Db, instance uint64{params}) {ret} {{\n{run}}}\n\n",
            decl.name.name
        ));
        out.push_str(&inner);
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

pub(crate) use crate::type_ref_info;

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
    /// Whether the design starts a foldable part folded.
    pub folded: bool,
    /// Whether the design starts a collapsible sidebar folded to its button.
    pub collapsed: bool,
}

/// The window or app a handler belongs to.
#[derive(Debug, Clone, Default)]
pub struct WindowCtx {
    pub parts: HashMap<String, PartInfo>,
    /// The window's data: name, entity and whether it is a list.
    pub data: Vec<(String, String, bool)>,
    /// The named tables and lists showing each list data (`crate::rows_shown`).
    pub showing: std::collections::BTreeMap<String, Vec<String>>,
    /// The parameter types of each handler that takes values, by name.
    pub params: HashMap<String, Vec<TypeRef>>,
}

impl WindowCtx {
    fn widgets(&mut self, widgets: &[Widget]) {
        for widget in widgets {
            if let Some(name) = &widget.props.name {
                let info = PartInfo { bind: widget.props.bind.clone(), caption: widget.props.caption.clone(), readonly: widget.props.readonly, folded: widget.props.folded, collapsed: false };
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
            if let Some(name) = &menu.name {
                self.parts.insert(name.clone(), PartInfo::default());
            }
            self.commands(&menu.items);
        }
        if let Some(toolbar) = &frame.toolbar {
            if let Some(name) = &toolbar.name {
                self.parts.insert(name.clone(), PartInfo::default());
            }
            self.commands(&toolbar.items);
        }
        if let Some(ribbon) = &frame.ribbon {
            if let Some(name) = &ribbon.name {
                self.parts.insert(name.clone(), PartInfo { collapsed: ribbon.collapsed, ..PartInfo::default() });
            }
            for tab in &ribbon.tabs {
                for name in tab.name.iter().chain(tab.groups.iter().filter_map(|g| g.name.as_ref())) {
                    self.parts.insert(name.clone(), PartInfo::default());
                }
            }
            for group in ribbon.tabs.iter().flat_map(|t| &t.groups) {
                self.commands(&group.items);
            }
        }
        for sidebar in &frame.sidebars {
            if let Some(name) = &sidebar.props.name {
                self.parts.insert(name.clone(), PartInfo { collapsed: sidebar.props.collapsed, ..PartInfo::default() });
            }
            self.widgets(&sidebar.widgets);
        }
        if let Some(footer) = &frame.footer {
            self.widgets(&footer.widgets);
        }
    }
}

/// A value written in the design as Go, of the parameter's type.
fn literal(project: &Project, ty: &TypeRef, value: &clarity_spec::model::Literal) -> String {
    use clarity_spec::model::Literal;
    match (type_ref_info(project, ty), value) {
        (TypeInfo::Decimal | TypeInfo::Money, Literal::Number(n)) => super::model::decimal_literal(n),
        (_, Literal::Number(n)) => n.clone(),
        // A key starts in the language the screen picked.
        (_, Literal::Str(text)) if text.starts_with('@') => format!("clarity.TextOf({})", string(&text[1..])),
        (_, Literal::Str(text)) => string(text),
        (_, Literal::Bool(b)) => b.to_string(),
        (TypeInfo::Enum(e), Literal::Ident(v)) => super::model::constant(&e, v),
        (_, Literal::Ident(v)) => v.clone(),
    }
}

/// A window or the app, as main.go dispatches to it.
#[derive(Debug, Clone)]
pub struct Owner {
    pub name: String,
    /// The struct holding its data: `CustomerFormWindow`, `ShopApp`.
    pub struct_name: String,
    /// The handlers called by name (those taking no values), with their method names.
    pub handlers: Vec<(String, String)>,
    /// The calls with values its parts wire: as written (`digit(7)`), the
    /// method, and the values as Go.
    pub calls: Vec<(String, String, Vec<String>)>,
    pub is_app: bool,
    /// Its own events (open, close, return, start) with their handlers.
    pub events: Vec<(&'static str, String)>,
    /// The canvases its draw handlers draw, with their methods.
    pub draws: Vec<(String, String)>,
    /// The handlers an answer runs (DESIGN 5.14): the Clarity name, the
    /// method, and each value's Go type, a record's by its struct.
    pub answers: Vec<(String, String, Vec<AnswerValue>)>,
}

/// A value an answering handler takes: a record (by its struct) or another value (by its Go type).
#[derive(Debug, Clone)]
pub enum AnswerValue {
    Record(String),
    Value(String),
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
        ctx.data = data.iter().filter_map(|d| d.entity().map(|e| (d.name.name.clone(), e.name.clone(), d.is_list()))).collect();
        ctx.showing = crate::rows_shown(page);
        let written: Vec<&clarity_spec::code::HandlerDecl> = file.code.iter().filter(|b| b.name.name == *name).flat_map(|b| &b.handlers).collect();
        ctx.params = written.iter().map(|h| (h.name.name.clone(), h.params.iter().map(|p| p.ty.clone()).collect())).collect();
        // Each call with values the parts wire, with its values as Go.
        let calls: Vec<(String, String, Vec<String>)> = page
            .map(crate::wired_calls)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|h| {
                let decl = written.iter().find(|d| d.name.name == h.handler)?;
                let values = h.args.iter().zip(&decl.params).map(|(v, p)| literal(project, &p.ty, v)).collect();
                Some((clarity_spec::widgets::handler_call(h), local(&h.handler), values))
            })
            .collect();

        let rows: Vec<Vec<String>> = data
            .iter()
            .map(|d| {
                let ty = match &d.of {
                    DataKind::Record { entity } => format!("*{}", entity.name),
                    DataKind::List { entity } => format!("[]{}", entity.name),
                    DataKind::Value { ty, .. } => super::names::value_type(&type_ref_info(project, ty)),
                };
                vec![pascal(&d.name.name), ty, format!("`json:\"{}\"`", d.name.name)]
            })
            .collect();
        let mut load = String::new();
        for d in data {
            let (local_name, field_name) = (local(&d.name.name), pascal(&d.name.name));
            match &d.of {
                DataKind::List { entity } => {
                    let entity = &entity.name;
                    let _ = write!(load, "\t{local_name}, err := clarity.All[{entity}](db)\n\tif err != nil {{\n\t\treturn nil, err\n\t}}\n\tw.{field_name} = {local_name}\n");
                }
                DataKind::Record { entity } => {
                    let _ = write!(load, "\t{local_name} := New{}()\n\tw.{field_name} = &{local_name}\n", entity.name);
                }
                DataKind::Value { ty, default: Some(value) } => {
                    let _ = writeln!(load, "\tw.{field_name} = {}", literal(project, ty, value));
                }
                // An enum starts at its first value; the rest at Go's zero value.
                DataKind::Value { ty, default: None } => {
                    if let TypeInfo::Enum(e) = type_ref_info(project, ty)
                        && let Some(first) = project.enum_decl(&e).and_then(|d| d.variants.first())
                    {
                        let _ = writeln!(load, "\tw.{field_name} = {}", super::model::constant(&e, &first.name.name));
                    }
                }
            }
        }
        let body = if rows.is_empty() { String::new() } else { super::model::aligned(&rows, "\t") };
        let _ = write!(
            out,
            "// {struct_name} is the data of {name}, as its handlers see it.\ntype {struct_name} struct {{\n{body}}}\n\n// load{struct_name} is the data when {name} opens: every record of a list, a new record otherwise, each value at its default.\nfunc load{struct_name}(db *clarity.Db) (*{struct_name}, error) {{\n\tw := &{struct_name}{{}}\n{load}\treturn w, nil\n}}\n\n"
        );

        let mut handlers = Vec::new();
        for handler in file.code.iter().filter(|b| b.name.name == *name).flat_map(|b| &b.handlers) {
            let method = local(&handler.name.name);
            // The values a click gives it, typed.
            let values: String = handler
                .params
                .iter()
                .map(|p| format!(", {} {}", local(&p.name.name), super::names::value_type(&type_ref_info(project, &p.ty))))
                .collect();
            let mut emitter = Emitter::new(project, types, None, Some(&ctx));
            let mut code = String::new();
            emitter.block(&handler.body, &mut code);
            if !code.trim_end().ends_with("return nil") {
                code.push_str("return nil\n");
            }
            if emitter.errors.is_empty() {
                let _ = write!(out, "func (w *{struct_name}) {method}(db *clarity.Db, ui *clarity.Ui{values}) error {{\n{}}}\n\n", indent(&code, 1));
                // One that takes values is called with them, through the calls.
                if handler.params.is_empty() {
                    handlers.push((handler.name.name.clone(), method));
                }
            } else {
                errors.extend(emitter.errors.into_iter().map(|e| format!("handler '{}' of {name}: {e}", handler.name.name)));
            }
        }
        let draws = page.map(crate::canvas_draws).unwrap_or_default().into_iter().map(|(canvas, handler)| (canvas, local(&handler))).collect();
        let written: Vec<&clarity_spec::code::HandlerDecl> = file.code.iter().filter(|b| b.name.name == *name).flat_map(|b| &b.handlers).collect();
        let answers = crate::answering_handlers(&written)
            .into_iter()
            .map(|h| {
                let values = h
                    .params
                    .iter()
                    .map(|p| match type_ref_info(project, &p.ty) {
                        TypeInfo::Record(entity) => AnswerValue::Record(entity),
                        other => AnswerValue::Value(super::names::value_type(&other)),
                    })
                    .collect();
                (h.name.name.clone(), local(&h.name.name), values)
            })
            .collect();
        owners.push(Owner {
            calls, name: name.clone(), struct_name, handlers, is_app: app.is_some(), events, draws, answers });
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
    /// The engine written in Clarity whose function or keep value this is.
    engine: Option<&'a EngineDecl>,
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
        Emitter { project, types, decl, window, engine: None, errors: Vec::new(), temps: 0, loops: 0, ensuring: false }
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
        if let Some(engine) = self.engine {
            params.push(format!("kept *{}", kept_type(&engine.name.name)));
        }
        for param in &decl.params {
            params.push(format!("{} {}", local(&param.name.name), value_type(&type_ref_info(self.project, &param.ty))));
        }
        let name = match self.engine {
            Some(engine) => format!("{}With", super::engine_function(&engine.name.name, &decl.name.name)),
            None if decl.kind == FunctionKind::Event => local(&decl.name.name),
            None => function_name(self.project, &decl.name.name),
        };
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

    /// `files f in store where ... order by ...`: the files of a store,
    /// filtered in a function of their own and sorted by the runtime.
    fn files(&mut self, var: &str, store: &str, filter: Option<&Expr>, order: &[OrderKey], pre: &mut String) -> String {
        let mut list = self.lift(format!("db.Files({})", string(store)), pre);
        if let Some(filter) = filter {
            let mut inner = String::new();
            let cond = bare(self.value(filter, &mut inner));
            if !inner.is_empty() {
                self.errors.push("a condition on files cannot call what can fail yet".into());
            }
            self.temps += 1;
            let kept = format!("v{}", self.temps);
            pre.push_str(&format!("{kept} := clarity.FilterFiles({list}, func({} clarity.FileEntry) bool {{\n\treturn {cond}\n}})\n", local(var)));
            list = kept;
        }
        if order.is_empty() {
            return list;
        }
        let keys: Vec<String> =
            order.iter().map(|k| format!("clarity.{}({})", if k.descending { "Desc" } else { "Asc" }, string(&k.field.name))).collect();
        format!("clarity.SortFiles({list}, {})", keys.join(", "))
    }

    /// The lines that lift a call that can fail; gives the name of its value.
    /// `code` as a plain operand: an expression with an operator is held in a
    /// variable of its own first.
    fn held_operand(&mut self, code: String, pre: &mut String) -> String {
        if !code.contains(' ') || code.starts_with('"') && code.ends_with('"') && !code[1..code.len() - 1].contains('"') {
            return code;
        }
        self.temps += 1;
        let name = format!("v{}", self.temps);
        pre.push_str(&format!("{name} := {code}\n"));
        name
    }

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
                    // Go would make a bare number an int.
                    None if matches!(self.types.type_of(value), TypeInfo::Integer | TypeInfo::Duration) => {
                        out.push_str(&format!("var {target} {} = {code}\n", value_type(&self.types.type_of(value))))
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
                        "folded" => "SetFolded",
                        "collapsed" => "SetCollapsed",
                        "maximized" => "SetMaximized",
                        "tab" => "SetTab",
                        "step" => "SetStep",
                        "open" => "SetOpenSection",
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
                // A file as a picture: linked in the field's own store, else copied.
                if self.types.type_of(value) == TypeInfo::File
                    && let ExprKind::Member { object, member } = &target.kind
                    && let TypeInfo::Record(owner) = self.types.type_of(object)
                {
                    let store = self.project.entity(&owner).and_then(|e| e.field(&member.name)).and_then(|f| f.modifiers.store.clone()).unwrap_or_default();
                    let file = bare(self.value(value, &mut pre));
                    // A file field keeps any file, under its name; an image only a picture.
                    let made = if self.types.type_of(target) == TypeInfo::RecordFile { "RecordFileFrom" } else { "PictureFrom" };
                    let picture = self.lift(format!("db.{made}({file}, {})", string(&store)), &mut pre);
                    let place = self.place(target, &mut pre);
                    out.push_str(&pre);
                    out.push_str(&format!("{place} = {picture}\n"));
                    return;
                }
                let to = self.types.type_of(target);
                // A list position, counted from 1.
                if let ExprKind::Index { list, index } = &target.kind {
                    let list = self.list_ref(list, &mut pre);
                    let at = bare(self.value(index, &mut pre));
                    let at = self.held_operand(at, &mut pre);
                    let code = bare(if is_decimal(&to) { self.decimal(value, &mut pre) } else { self.value(value, &mut pre) });
                    let code = self.held_operand(code, &mut pre);
                    out.push_str(&pre);
                    out.push_str(&format!("if err := clarity.SetAt({list}, {at}, {code}); err != nil {{\n\t{}\n}}\n", self.fail("err")));
                    return;
                }
                let code = if is_decimal(&to) { self.decimal(value, &mut pre) } else { self.value(value, &mut pre) };
                let place = self.place(target, &mut pre);
                out.push_str(&pre);
                out.push_str(&format!("{place} = {}\n", bare(code)));
                if let ExprKind::Name { name } = &target.kind {
                    out.push_str(&self.deselect(name));
                    // A text value shows its text in the person's language, as a label does.
                    if matches!(self.types.name_of(target), Some(NameKind::Data)) && to == TypeInfo::Text {
                        let mut pre = String::new();
                        let shown = match value.kind {
                            ExprKind::Str { .. } | ExprKind::Text { .. } | ExprKind::Key { .. } => self.shown(value, &mut pre),
                            _ => format!("clarity.PlainText({place})"),
                        };
                        out.push_str(&pre);
                        out.push_str(&format!("ui.SetShown({}, {shown})\n", string(name)));
                    }
                }
            }
            StmtKind::If { branches, otherwise } => self.if_chain(branches, otherwise.as_deref(), out),
            StmtKind::Write { text, store, path } => {
                // Bytes are written as they are, text as UTF-8.
                let write = if self.types.type_of(text) == TypeInfo::Bytes { "WriteBytes" } else { "WriteText" };
                let path = bare(self.value(path, &mut pre));
                let text = bare(self.value(text, &mut pre));
                // A sum among the arguments is held first, as gofmt writes a
                // call with several arguments more tightly than one on its own.
                let path = self.held_operand(path, &mut pre);
                let text = self.held_operand(text, &mut pre);
                out.push_str(&pre);
                out.push_str(&format!(
                    "if err := db.{write}({}, {path}, {text}); err != nil {{\n\t{}\n}}\n",
                    string(&store.name),
                    self.fail("err")
                ));
            }
            StmtKind::For { var, list, body } => {
                // Files are worked on as values, records through pointers.
                let values = matches!(self.types.type_of(list), TypeInfo::Values(_));
                let by_value = self.types.type_of(list) == TypeInfo::Files;
                let list = bare(self.value(list, &mut pre));
                // Plain values one by one, from a copy, so the body may change the list.
                if values {
                    out.push_str(&pre);
                    let mut inner = String::new();
                    self.block(body, &mut inner);
                    out.push_str(&format!("for _, {} := range {list} {{\n{}}}\n", local(&var.name), indent(&unused_locals(&inner), 1)));
                    return;
                }
                out.push_str(&pre);
                self.loops += 1;
                let index = if self.loops == 1 { "i".to_string() } else { format!("i{}", self.loops) };
                let item = local(&var.name);
                let mut inner = if by_value { format!("{item} := {list}[{index}]\n") } else { format!("{item} := &{list}[{index}]\n") };
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
                    let params = self.window.and_then(|w| w.params.get(&function.name).cloned()).unwrap_or_default();
                    let mut values = String::new();
                    for (arg, param) in args.iter().zip(&params) {
                        let ty = type_ref_info(self.project, param);
                        let value = if is_decimal(&ty) { self.decimal(arg, &mut pre) } else { self.value(arg, &mut pre) };
                        values.push_str(&format!(", {}", bare(value)));
                    }
                    out.push_str(&pre);
                    out.push_str(&format!("if err := w.{}(db, ui{values}); err != nil {{\n\treturn err\n}}\n", local(&function.name)));
                    return;
                }
                let call_code = self.call_code(&function.name, args, &mut pre);
                out.push_str(&pre);
                let own = self.engine.and_then(|e| e.body.as_ref()).and_then(|b| b.functions.iter().find(|f| f.name.name == function.name));
                let returns = own.or_else(|| self.project.files.iter().flat_map(|f| &f.functions).find(|f| f.name.name == function.name)).and_then(|f| f.returns.as_ref());
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
            // A shape on a canvas, as conformance/drawing.json sends it.
            StmtKind::Draw { canvas, shape, args } => {
                use clarity_spec::code::check::ShapeValue;
                let mut fields = vec![format!("\"op\": {}", string(&shape.name))];
                for (key, kind, given) in crate::shape_values(&shape.name, args) {
                    let value = match (kind, given) {
                        (ShapeValue::Number, Ok(arg)) => match self.types.type_of(arg) {
                            TypeInfo::Decimal | TypeInfo::Money => format!("clarity.Units({})", bare(self.value(arg, &mut pre))),
                            _ => format!("float64({})", bare(self.value(arg, &mut pre))),
                        },
                        (ShapeValue::Number, Err(default)) => format!("{default}.0"),
                        (ShapeValue::Color, Ok(arg)) => bare(self.value(arg, &mut pre)),
                        (ShapeValue::Color | ShapeValue::Text, Err(default)) => string(default),
                        (ShapeValue::Text, Ok(arg)) => self.shown(arg, &mut pre),
                    };
                    fields.push(format!("{}: {value}", string(key)));
                }
                out.push_str(&pre);
                out.push_str(&format!("ui.Draw({}, map[string]any{{{}}})\n", string(&canvas.name), fields.join(", ")));
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
            StmtKind::Ask { question, form, then, args } => {
                // A dialog window says what it asks itself.
                let question = match question {
                    Some(question) => self.shown(question, &mut pre),
                    None => "clarity.PlainText(\"\")".to_string(),
                };
                let form = match form.as_ref() {
                    AskForm::Confirm { yes, no } => {
                        let yes = yes.as_ref().map_or("nil".to_string(), |t| format!("clarity.Button({})", self.shown(t, &mut pre)));
                        let no = no.as_ref().map_or("nil".to_string(), |t| format!("clarity.Button({})", self.shown(t, &mut pre)));
                        format!("clarity.Confirm({yes}, {no})")
                    }
                    AskForm::Value { ty, default, from } => {
                        let (word, of) = crate::ask_type(self.project, ty);
                        let of = of.map_or("nil".to_string(), |n| format!("clarity.Named({})", string(&n)));
                        let default = default.as_ref().map_or("nil".to_string(), |d| bare(self.value(d, &mut pre)));
                        // The records `from` offers, by their ids.
                        let only = from.as_ref().map_or("nil".to_string(), |list| format!("clarity.IDs({})", bare(self.value(list, &mut pre))));
                        format!("clarity.ValueForm({}, {of}, {only}, {default})", string(word))
                    }
                    AskForm::Window { window, with } => {
                        let with: Vec<String> = with.iter().map(|a| format!("{}: {}", string(&a.name.name), bare(self.value(&a.value, &mut pre)))).collect();
                        format!("clarity.WindowForm({}, map[string]any{{{}}})", string(&window.name), with.join(", "))
                    }
                };
                let values: String = args.iter().map(|a| format!(", {}", bare(self.value(a, &mut pre)))).collect();
                out.push_str(&pre);
                out.push_str(&format!("ui.Ask({question}, {form}, {}{values})\n", string(&then.name)));
            }
            StmtKind::Open { window, with } => {
                let args: Vec<String> = with.iter().map(|arg| format!("{}: {}", string(&arg.name.name), bare(self.value(&arg.value, &mut pre)))).collect();
                out.push_str(&pre);
                out.push_str(&format!("ui.Open({}, map[string]any{{{}}})\n", string(&window.name), args.join(", ")));
            }
            StmtKind::Close => out.push_str("ui.Close()\n"),
            StmtKind::CloseWith { value } => {
                let value = bare(self.value(value, &mut pre));
                out.push_str(&pre);
                out.push_str(&format!("ui.CloseWith({value})\n"));
            }
            StmtKind::Reload { data } => {
                let found = self.window.and_then(|w| w.data.iter().find(|(n, _, _)| *n == data.name)).cloned();
                match found {
                    Some((name, entity, true)) => {
                        let all = self.lift(format!("clarity.All[{entity}](db)"), &mut pre);
                        out.push_str(&pre);
                        out.push_str(&format!("w.{} = {all}\n", pascal(&name)));
                        out.push_str(&self.deselect(&name));
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
            // A key, whose value the screen looks up, with its slots by name.
            ExprKind::Key { key, args } => {
                let mut code = format!("clarity.KeyText({})", string(&key.name));
                for arg in args {
                    let slot = self.slot(&arg.value, pre);
                    code.push_str(&format!(".With({}, {slot})", string(&arg.name.name)));
                }
                code
            }
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

    /// After a handler replaced the rows of `data` (`reload`, or a new
    /// list): the tables and lists showing them lose their selected row.
    fn deselect(&self, data: &str) -> String {
        let parts = self.window.and_then(|w| w.showing.get(data)).into_iter().flatten();
        parts.map(|part| format!("ui.Deselect({})\n", string(part))).collect()
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
                Some(NameKind::Kept) => format!("kept.{}", local(name)),
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
                // A file's details: Name, Path, Extension, Size, Modified.
                if self.types.type_of(object) == TypeInfo::File {
                    let object = self.place(object, pre);
                    return format!("{object}.{}", pascal(&member.name));
                }
                if self.types.type_of(object) == TypeInfo::Bytes {
                    let object = self.place(object, pre);
                    return format!("int64(len({object}))");
                }
                // A record's file: its name, and the extension of its name.
                if self.types.type_of(object) == TypeInfo::RecordFile {
                    let object = self.place(object, pre);
                    let read = if member.name == "extension" { "RecordFileExtension" } else { "RecordFileName" };
                    return format!("clarity.{read}({object})");
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
            "folded" => format!("ui.Folded({quoted}, {})", info.folded),
            "collapsed" => format!("ui.Collapsed({quoted}, {})", info.collapsed),
            "maximized" => format!("ui.Maximized({quoted})"),
            "tab" => format!("ui.Tab({quoted})"),
            "step" => format!("ui.Step({quoted})"),
            "open" => format!("ui.OpenSection({quoted})"),
            "text" => format!("ui.Text({quoted})"),
            "checked" => format!("ui.Checked({quoted})"),
            "x" => format!("ui.X({quoted})"),
            "y" => format!("ui.Y({quoted})"),
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
            ExprKind::ListOf { items } => {
                let decimals = matches!(&ty, TypeInfo::Values(item) if is_decimal(&item.info()));
                let items: Vec<String> = items.iter().map(|i| bare(if decimals { self.decimal(i, pre) } else { self.value(i, pre) })).collect();
                format!("{}{{{}}}", value_type(&ty), items.join(", "))
            }
            ExprKind::Index { list, index } => {
                let list = self.list_ref(list, pre);
                let at = bare(self.value(index, pre));
                let at = self.held_operand(at, pre);
                self.lift(format!("clarity.At({list}, {at})"), pre)
            }
            // A list of values is a value: what reads it gets a copy of its own.
            ExprKind::Name { .. } | ExprKind::Member { .. } if matches!(ty, TypeInfo::Values(_)) && self.part_property(expr).is_none() => {
                format!("slices.Clone({})", self.place(expr, pre))
            }
            ExprKind::Number { value } if is_decimal(&ty) => decimal_literal(value),
            ExprKind::Number { value } => value.clone(),
            ExprKind::Str { value } => string(value),
            // A text with slots in code is filled plainly; a key takes its value
            // in the language the screen picked, then its values.
            ExprKind::Text { .. } | ExprKind::Key { .. } => format!("{}.Show()", self.shown(expr, pre)),
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
            ExprKind::Files { var, store, filter, order } => self.files(&var.name, &store.name, filter.as_deref(), order, pre),
            ExprKind::ReadText { file } => {
                let file = bare(self.value(file, pre));
                self.lift(format!("db.ReadText({file})"), pre)
            }
            ExprKind::ReadBytes { file } => {
                let file = bare(self.value(file, pre));
                self.lift(format!("db.ReadBytes({file})"), pre)
            }
            ExprKind::EngineCall { engine, function, args } if crate::engine(self.project, &engine.name).is_some_and(|e| e.body.is_some()) => {
                self.written_call(&engine.name, &function.name, args, pre)
            }
            // A function of an engine, through its typed function in engines.go (DESIGN 5.11).
            ExprKind::EngineCall { engine, function, args } => {
                let params: Vec<clarity_spec::code::TypeRef> = self
                    .project
                    .files
                    .iter()
                    .flat_map(|f| &f.engines)
                    .find(|e| e.name.name == engine.name)
                    .and_then(|e| e.functions.iter().find(|f| f.name.name == function.name))
                    .map(|f| f.params.iter().map(|p| p.ty.clone()).collect())
                    .unwrap_or_default();
                // An engine with state per window is called in the handler's own window.
                let per_window = self.project.files.iter().flat_map(|f| &f.engines).any(|e| e.name.name == engine.name && e.state == clarity_spec::code::EngineState::Window);
                let mut values = if per_window { vec!["ui.Instance()".to_string()] } else { Vec::new() };
                for (arg, param) in args.iter().zip(&params) {
                    let ty = type_ref_info(self.project, param);
                    values.push(bare(if is_decimal(&ty) { self.decimal(arg, pre) } else { self.value(arg, pre) }));
                }
                let call = format!("{}({})", super::engine_function(&engine.name, &function.name), values.join(", "));
                let value = self.lift(call, pre);
                // The text an engine gives is the value of its key, as its `says` lists it.
                if self.types.type_of(expr) == TypeInfo::Text { format!("clarity.TextOf({value})") } else { value }
            }
            ExprKind::New { entity } => format!("clarity.New(New{}())", entity.name),
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
            // Whether a list holds an item.
            BinaryOp::Contains if let TypeInfo::Values(item) = &lt => {
                let l = self.list_ref(left, pre);
                let r = bare(if is_decimal(&item.info()) { self.decimal(right, pre) } else { self.value(right, pre) });
                format!("clarity.ListHolds({l}, {r})")
            }
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
    /// A list as it is, for a position or a count: a name or field read in
    /// place, anything else computed.
    fn list_ref(&mut self, list: &Expr, pre: &mut String) -> String {
        match &list.kind {
            ExprKind::Name { .. } | ExprKind::Member { .. } if self.part_property(list).is_none() => self.place(list, pre),
            _ => bare(self.value(list, pre)),
        }
    }

    /// A function of an engine written in Clarity, on the instance of the
    /// handler's window (state per window) or the app's (0).
    fn written_call(&mut self, engine: &str, function: &str, args: &[Expr], pre: &mut String) -> String {
        let Some(decl) = crate::engine(self.project, engine) else { return String::new() };
        let Some(callee) = decl.body.as_ref().and_then(|b| b.functions.iter().find(|f| f.name.name == function)) else {
            self.errors.push(format!("engine {engine} has no function '{function}'"));
            return String::new();
        };
        let instance = match decl.state {
            EngineState::Window if self.window.is_some() => "ui.Instance()",
            EngineState::Window => {
                self.errors.push(format!("engine {engine} keeps state per window; call it from a handler"));
                ""
            }
            _ => "0",
        };
        let mut values = vec!["db".to_string(), instance.to_string()];
        for (arg, param) in args.iter().zip(&callee.params) {
            let ty = type_ref_info(self.project, &param.ty);
            let value = bare(if is_decimal(&ty) { self.decimal(arg, pre) } else { self.value(arg, pre) });
            values.push(self.held_operand(value, pre));
        }
        self.lift(format!("{}({})", super::engine_function(engine, function), values.join(", ")), pre)
    }

    fn call_code(&mut self, name: &str, args: &[Expr], pre: &mut String) -> String {
        // An engine's own function, on the same kept values.
        if let Some(engine) = self.engine
            && let Some(own) = engine.body.as_ref().and_then(|b| b.functions.iter().find(|f| f.name.name == name))
        {
            let mut code = vec!["db".to_string(), "kept".to_string()];
            for (arg, param) in args.iter().zip(&own.params) {
                let ty = type_ref_info(self.project, &param.ty);
                let value = bare(if is_decimal(&ty) { self.decimal(arg, pre) } else { self.value(arg, pre) });
                code.push(self.held_operand(value, pre));
            }
            return format!("{}With({})", super::engine_function(&engine.name.name, name), code.join(", "));
        }
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
        format!("{}({})", function_name(self.project, name), code.join(", "))
    }

    fn call(&mut self, name: &str, args: &[Expr], pre: &mut String) -> String {
        use clarity_spec::code::check::{NOW, TODAY};
        if name == TODAY {
            return "clarity.Today()".into();
        }
        if name == NOW {
            return "clarity.Now()".into();
        }
        // round(value, places), as every runtime rounds (conformance/rounding.json).
        // sin(degrees), cos(degrees), as every runtime gives them (conformance/drawing.json).
        if name == clarity_spec::code::check::SIN || name == clarity_spec::code::check::COS {
            let arg = args.first();
            let value = match arg.map(|a| self.types.type_of(a)) {
                Some(TypeInfo::Decimal | TypeInfo::Money) => bare(self.value(arg.unwrap(), pre)),
                Some(_) => format!("decimal.NewFromInt({})", bare(self.value(arg.unwrap(), pre))),
                None => String::new(),
            };
            let function = if name == clarity_spec::code::check::SIN { "Sin" } else { "Cos" };
            return format!("clarity.{function}({value})");
        }
        if name == clarity_spec::code::check::ROUND {
            let values: Vec<String> = args.iter().map(|a| bare(self.value(a, pre))).collect();
            return format!("clarity.Round({})", values.join(", "));
        }
        if name == "count" {
            let list = args.first().map(|a| self.list_ref(a, pre)).unwrap_or_default();
            return format!("int64(len({list}))");
        }
        // `random(6)` in an engine that uses random.
        if name == clarity_spec::code::check::RANDOM && !self.project.files.iter().flat_map(|f| &f.functions).any(|f| f.name.name == name) {
            let max = args.first().map(|a| bare(self.value(a, pre))).unwrap_or_default();
            return self.lift(format!("clarity.Random({max})"), pre);
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
                } else if let FieldType::Values { item } = &ty {
                    // A list field holds the item.
                    format!("clarity.Holds({name}, {})", self.sql_value(&item.field_type(), value, pre))
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
