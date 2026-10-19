//! Functions, actions, data events, windows and their handlers as C. Every
//! one of them returns whether it worked: a call that can fail (a function
//! or action, a query, loading the record a reference points at) is lifted
//! out of its expression into a statement of its own, `if (!call(..., &v1))
//! return false;`, as a C developer writes it, with the error left for the
//! shell to show. `and` and `or` keep stopping at the first side that
//! decides: when their right side needs lifted statements, those run inside
//! an `if`. A function's result goes out through its last parameter.
//!
//! Records are worked on through pointers, so an action changes the record
//! it is given and a loop the records of its list; a record kept in a local
//! is a value of its own. Types come from the checker's table; the emitter
//! only chooses how to write them in C.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use clarity_spec::Project;
use clarity_spec::code::check::{FileTypes, NameKind, TypeInfo};
use clarity_spec::code::{
    BinaryOp, Expr, ExprKind, FunctionDecl, MessageLevel, OrderKey, Stmt, StmtKind, TextPart, TypeRef, TypeRefKind, UnaryOp, path_text,
    template,
};
use clarity_spec::model::{Command, FieldType, Frame, SpecFile, Widget};

use super::names::{constant, decimal, ident, list_type, param_type, prefix, snake, string, value_type, zero};
use crate::print;

/// What a `TypeRef` names, as a checker type.
pub fn type_ref_info(project: &Project, ty: &TypeRef) -> TypeInfo {
    match &ty.kind {
        TypeRefKind::Text => TypeInfo::Text,
        TypeRefKind::Integer => TypeInfo::Integer,
        TypeRefKind::Duration => TypeInfo::Duration,
        TypeRefKind::Decimal => TypeInfo::Decimal,
        TypeRefKind::Money => TypeInfo::Money,
        TypeRefKind::Bool => TypeInfo::Bool,
        TypeRefKind::Date => TypeInfo::Date,
        TypeRefKind::DateTime => TypeInfo::DateTime,
        TypeRefKind::File => TypeInfo::File,
        TypeRefKind::List { item } if item == "file" => TypeInfo::Files,
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
        BinaryOp::Eq => "CLARITY_EQ",
        BinaryOp::NotEq => "CLARITY_NOT_EQ",
        BinaryOp::Lt => "CLARITY_LT",
        BinaryOp::LtEq => "CLARITY_LT_EQ",
        BinaryOp::Gt => "CLARITY_GT",
        _ => "CLARITY_GT_EQ",
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

/// `lines` indented by `depth` steps of four spaces.
fn indent(lines: &str, depth: usize) -> String {
    let pad = "    ".repeat(depth);
    lines.lines().map(|l| if l.is_empty() { "\n".to_string() } else { format!("{pad}{l}\n") }).collect()
}

/// Whether `code` is a name or a path of names, which `&` and `.` take as it is.
fn is_path(code: &str) -> bool {
    !code.is_empty() && code.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '>' | '[' | ']'))
}

/// A field of the record `pointer` points at: `order->total`, or
/// `v1.total` for a pointer to a value at hand.
fn arrow(pointer: &str, field: &str) -> String {
    match pointer.strip_prefix('&') {
        Some(value) if is_path(value) => format!("{value}.{field}"),
        _ if is_path(pointer) => format!("{pointer}->{field}"),
        _ => format!("({pointer})->{field}"),
    }
}

/// The record `pointer` points at, as a value.
fn deref(pointer: &str) -> String {
    match pointer.strip_prefix('&') {
        Some(value) if is_path(value) => value.to_string(),
        _ => format!("*{pointer}"),
    }
}

/// The name of a function, action or data event.
fn function_name(decl: &FunctionDecl) -> String {
    ident(&decl.name.name)
}

/// The declaration of a function, action or data event, for the header:
/// whether it worked, with its result through `result`.
pub fn signature(project: &Project, decl: &FunctionDecl) -> String {
    format!("bool {}({})", function_name(decl), params(project, decl).join(", "))
}

fn params(project: &Project, decl: &FunctionDecl) -> Vec<String> {
    let mut params = vec!["ClarityDb* db".to_string()];
    for param in &decl.params {
        params.push(format!("{} {}", param_type(&type_ref_info(project, &param.ty)), ident(&param.name.name)));
    }
    if let Some(ret) = &decl.returns {
        params.push(format!("{}* result", value_type(&type_ref_info(project, ret))));
    }
    params
}

/// The data events written in `file`, as actions on their record.
pub fn events(file: &SpecFile) -> Vec<FunctionDecl> {
    file.code.iter().flat_map(|b| b.events.iter().map(|e| e.action(&b.name.name))).collect()
}

/// The definitions of `file`'s functions, actions and data events.
pub fn emit(project: &Project, file: &SpecFile, types: &FileTypes) -> Result<String, Vec<String>> {
    let mut out = String::new();
    let mut errors = Vec::new();
    for decl in file.functions.iter().chain(&events(file)) {
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
    /// The named tables and lists showing each list data (`crate::rows_shown`).
    pub showing: std::collections::BTreeMap<String, Vec<String>>,
    /// The prefix of its functions: `customer_form_window`.
    pub prefix: String,
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

/// A window or the app, as main.c dispatches to it.
#[derive(Debug, Clone)]
pub struct Owner {
    pub name: String,
    /// The struct holding its data: `CustomerFormWindow`, `ShopApp`.
    pub struct_name: String,
    /// The prefix of its functions: `customer_form_window`.
    pub prefix: String,
    /// Handlers by Clarity name, with their function names.
    pub handlers: Vec<(String, String)>,
    pub is_app: bool,
    /// Its own events (open, close, return, start) with their handlers.
    pub events: Vec<(&'static str, String)>,
}

/// The struct of every window and the app in `file`, for the header, and
/// their loading, JSON and handlers, for the source.
pub fn emit_windows(project: &Project, file: &SpecFile, types: &FileTypes) -> Result<(String, String, Vec<Owner>), Vec<String>> {
    let (mut header, mut source) = (String::new(), String::new());
    let mut owners = Vec::new();
    let mut errors = Vec::new();
    let pages = file.pages.iter().map(|p| (&p.name.name, &p.data, Some(p), None));
    let apps = file.apps.iter().map(|a| (&a.name.name, &a.data, None, Some(a)));
    for (name, data, page, app) in pages.chain(apps) {
        let struct_name = if app.is_some() { format!("{name}App") } else { format!("{name}Window") };
        let p = snake(&struct_name);
        let mut ctx = WindowCtx { prefix: p.clone(), ..WindowCtx::default() };
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
        ctx.showing = crate::rows_shown(page);

        let mut fields = String::new();
        let mut load = String::new();
        let mut to = String::new();
        let mut from = String::new();
        for d in data {
            let (member, entity, key) = (ident(&d.name.name), &d.entity.name, string(&d.name.name));
            let e = prefix(entity);
            if d.list {
                let _ = writeln!(fields, "    {} {member};", list_type(entity));
                let _ = writeln!(load, "    if (!{e}_all(db, &w->{member})) return false;");
                let _ = writeln!(to, "    cJSON_AddItemToObject(out, {key}, {e}_list_json(&w->{member}));");
                let _ = writeln!(from, "    if ((value = clarity_json_given(in, {key})) && !{e}_list_from_json(value, &w->{member})) return false;");
            } else {
                let _ = writeln!(fields, "    {entity} {member};");
                let _ = writeln!(load, "    w->{member} = {e}_new();");
                let _ = writeln!(to, "    cJSON_AddItemToObject(out, {key}, {e}_json(&w->{member}));");
                let _ = writeln!(from, "    w->{member} = {e}_new();\n    if ((value = clarity_json_given(in, {key})) && !{e}_from_json(value, &w->{member})) return false;");
            }
        }
        if fields.is_empty() {
            // C has no struct without members.
            fields.push_str("    char nothing;\n");
        }

        let mut declarations = String::new();
        let mut handlers = Vec::new();
        for handler in file.code.iter().filter(|b| b.name.name == *name).flat_map(|b| &b.handlers) {
            let function = format!("{p}_{}", snake(&handler.name.name));
            let mut emitter = Emitter::new(project, types, None, Some(&ctx));
            let mut code = String::new();
            emitter.block(&handler.body, &mut code);
            if !code.trim_end().ends_with("return true;") {
                code.push_str("return true;\n");
            }
            let code = unused(&code, &["db", "ui", "w"]);
            if emitter.errors.is_empty() {
                let signature = format!("bool {function}(ClarityDb* db, ClarityUi* ui, {struct_name}* w)");
                let _ = writeln!(declarations, "{signature};");
                let _ = write!(source, "{signature} {{\n{}}}\n\n", indent(&code, 1));
                handlers.push((handler.name.name.clone(), function));
            } else {
                errors.extend(emitter.errors.into_iter().map(|e| format!("handler '{}' of {name}: {e}", handler.name.name)));
            }
        }
        let what = if app.is_some() { "the app" } else { name.as_str() };
        let _ = write!(
            header,
            "/// The data of {what}, as its handlers see it.\ntypedef struct {{\n{fields}}} {struct_name};\n/// The data when it opens: every record of a list, a new record otherwise.\nbool {p}_load(ClarityDb* db, {struct_name}* w);\ncJSON* {p}_json(const {struct_name}* w);\nbool {p}_from_json(const cJSON* in, {struct_name}* w);\n{declarations}\n"
        );
        let value = if from.is_empty() { "" } else { "    const cJSON* value;\n" };
        let load = unused(&format!("*w = ({struct_name}){{0}};\n{}return true;\n", dedent(&load)), &["db"]);
        let to = unused(&format!("cJSON* out = cJSON_CreateObject();\n{}return out;\n", dedent(&to)), &["w"]);
        let from = unused(&format!("*w = ({struct_name}){{0}};\n{}{}return true;\n", dedent(value), dedent(&from)), &["in"]);
        let _ = write!(
            source,
            "bool {p}_load(ClarityDb* db, {struct_name}* w) {{\n{}}}\n\ncJSON* {p}_json(const {struct_name}* w) {{\n{}}}\n\nbool {p}_from_json(const cJSON* in, {struct_name}* w) {{\n{}}}\n\n",
            indent(&load, 1),
            indent(&to, 1),
            indent(&from, 1)
        );
        owners.push(Owner { name: name.clone(), struct_name, prefix: p, handlers, is_app: app.is_some(), events });
    }
    if errors.is_empty() { Ok((header, source, owners)) } else { Err(errors) }
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
    /// Locals that hold a record of their own, rather than point at one.
    record_values: HashSet<String>,
}

impl<'a> Emitter<'a> {
    fn new(project: &'a Project, types: &'a FileTypes, decl: Option<&'a FunctionDecl>, window: Option<&'a WindowCtx>) -> Self {
        Emitter { project, types, decl, window, errors: Vec::new(), temps: 0, loops: 0, ensuring: false, record_values: HashSet::new() }
    }

    fn returns(&self) -> Option<TypeInfo> {
        self.decl.and_then(|d| d.returns.as_ref()).map(|r| type_ref_info(self.project, r))
    }

    fn function(&mut self) -> String {
        let decl = self.decl.expect("a function, action or data event");
        let name = function_name(decl);
        let mut out = String::new();
        let mut body = String::new();
        for condition in &decl.requires {
            let mut pre = String::new();
            let holds = bare(self.value(condition, &mut pre));
            body.push_str(&pre);
            let _ = writeln!(body, "if (!clarity_require({holds}, {})) return false;", string(&print::expr(condition)));
        }
        if decl.ensures.is_empty() {
            self.block(&decl.body, &mut body);
            if !body.trim_end().ends_with("return true;") {
                body.push_str("return true;\n");
            }
        } else {
            // The body is a function of its own, so every return gives the
            // result the ensures check.
            let mut inner = String::new();
            self.block(&decl.body, &mut inner);
            if !inner.trim_end().ends_with("return true;") {
                inner.push_str("return true;\n");
            }
            let names = param_names(decl);
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let _ = write!(out, "static bool {name}_body({}) {{\n{}}}\n\n", params(self.project, decl).join(", "), indent(&unused(&inner, &names), 1));
            let mut args = vec!["db".to_string()];
            args.extend(decl.params.iter().map(|p| ident(&p.name.name)));
            if self.returns().is_some() {
                args.push("result".into());
            }
            let _ = writeln!(body, "if (!{name}_body({})) return false;", args.join(", "));
            self.ensuring = true;
            for condition in &decl.ensures {
                let mut pre = String::new();
                let holds = bare(self.value(condition, &mut pre));
                body.push_str(&pre);
                let _ = writeln!(body, "if (!clarity_require({holds}, {})) return false;", string(&print::expr(condition)));
            }
            self.ensuring = false;
            body.push_str("return true;\n");
        }
        let names = param_names(decl);
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let _ = write!(out, "{} {{\n{}}}\n\n", signature(self.project, decl), indent(&unused(&body, &names), 1));
        out
    }

    fn block(&mut self, body: &[Stmt], out: &mut String) {
        for stmt in body {
            self.stmt(stmt, out);
        }
    }

    /// A new lifted value: `v1`, `v2`.
    fn temp(&mut self) -> String {
        self.temps += 1;
        format!("v{}", self.temps)
    }

    /// The lines that lift a call that can fail, its value going out through
    /// `&vN` at `{out}` in `call`; gives the name of the value.
    fn lift(&mut self, ty: &str, call: &str, pre: &mut String) -> String {
        let name = self.temp();
        let call = call.replace("{out}", &format!("&{name}"));
        let empty = if ty.ends_with('*') {
            "NULL"
        } else if ty == "bool" {
            "false"
        } else if ty == "int64_t" {
            "0"
        } else {
            "{0}"
        };
        let _ = write!(pre, "{ty} {name} = {empty};\nif (!{call}) return false;\n");
        name
    }

    /// `lift` for a call that gives a record, as a pointer to it.
    fn lift_record(&mut self, entity: &str, call: &str, pre: &mut String) -> String {
        format!("&{}", self.lift(entity, call, pre))
    }

    /// `files f in store where ... order by ...`: the files of a store,
    /// kept by a loop of their own and sorted by the runtime.
    fn files(&mut self, var: &str, store: &str, filter: Option<&Expr>, order: &[OrderKey], pre: &mut String) -> String {
        let mut list = self.lift("ClarityFileList", &format!("clarity_files(db, {}, {{out}})", string(store)), pre);
        if let Some(filter) = filter {
            let kept = self.temp();
            self.loops += 1;
            let index = format!("i{}", self.loops);
            let mut inner = String::new();
            let cond = bare(self.value(filter, &mut inner));
            let file = ident(var);
            let _ = write!(
                pre,
                "ClarityFileList {kept} = clarity_file_list({list}.count);\nfor (size_t {index} = 0; {index} < {list}.count; {index}++) {{\n    ClarityFile* {file} = &{list}.items[{index}];\n{}    if ({cond}) {kept}.items[{kept}.count++] = *{file};\n}}\n",
                indent(&inner, 1)
            );
            list = kept;
        }
        if !order.is_empty() {
            let keys: Vec<String> =
                order.iter().map(|k| format!("clarity_{}({})", if k.descending { "desc" } else { "asc" }, string(&k.field.name))).collect();
            let _ = writeln!(pre, "clarity_sort_files(&{list}, {}, (ClarityFileOrder[]){{{}}});", keys.len(), keys.join(", "));
        }
        list
    }

    fn stmt(&mut self, stmt: &Stmt, out: &mut String) {
        let mut pre = String::new();
        match &stmt.kind {
            StmtKind::Let { name, ty, value } => {
                let declared = ty.as_ref().map(|t| type_ref_info(self.project, t)).unwrap_or_else(|| self.types.type_of(value));
                let target = ident(&name.name);
                let code = match &declared {
                    _ if matches!(value.kind, ExprKind::Empty) => zero(&declared),
                    t if is_decimal(t) => self.decimal(value, &mut pre),
                    TypeInfo::Record(_) => deref(&self.value(value, &mut pre)),
                    _ => self.value(value, &mut pre),
                };
                if matches!(declared, TypeInfo::Record(_)) {
                    self.record_values.insert(target.clone());
                }
                // A value a call just gave out goes to the local itself.
                let code = bare(code);
                let ty = value_type(&declared);
                if let Some(lifted) = pre.find(&format!("{ty} {code} = ")).filter(|_| is_temp(&code)) {
                    let renamed = rename(&pre[lifted..], &code, &target);
                    out.push_str(&pre[..lifted]);
                    out.push_str(&renamed);
                    return;
                }
                out.push_str(&pre);
                let _ = writeln!(out, "{ty} {target} = {code};");
            }
            StmtKind::Assign { target, value } => {
                if let Some((part, property)) = self.part_property(target) {
                    let function = match property.as_str() {
                        "caption" | "error" | "visible" | "enabled" | "readonly" | "text" | "checked" => format!("clarity_ui_set_{property}"),
                        other => {
                            self.errors.push(format!("'{other}' of a part cannot be set"));
                            return;
                        }
                    };
                    // A caption or error is kept as written with its slots, and shown in the person's format.
                    let code = if matches!(property.as_str(), "caption" | "error") { self.shown(value, &mut pre) } else { bare(self.value(value, &mut pre)) };
                    out.push_str(&pre);
                    let _ = writeln!(out, "{function}(ui, {}, {code});", string(&part));
                    return;
                }
                // A record given to a reference field is stored as its id.
                if self.is_reference_field(target) {
                    let stored = self.stored_reference(target, &mut pre);
                    let record = if matches!(value.kind, ExprKind::Empty) {
                        "0".to_string()
                    } else {
                        let record = self.value(value, &mut pre);
                        arrow(&record, "id")
                    };
                    out.push_str(&pre);
                    let _ = writeln!(out, "{stored} = {record};");
                    return;
                }
                let to = self.types.type_of(target);
                // A file as a picture: linked in the field's own store, else copied.
                if self.types.type_of(value) == TypeInfo::File
                    && let ExprKind::Member { object, member } = &target.kind
                    && let TypeInfo::Record(owner) = self.types.type_of(object)
                {
                    let store = self.project.entity(&owner).and_then(|e| e.field(&member.name)).and_then(|f| f.modifiers.store.clone());
                    let file = bare(self.value(value, &mut pre));
                    let store = store.map_or_else(|| "NULL".to_string(), |s| string(&s));
                    let picture = self.lift("ClarityPicture*", &format!("clarity_picture_from(db, {file}, {store}, {{out}})"), &mut pre);
                    let place = self.place(target, &mut pre);
                    out.push_str(&pre);
                    let _ = writeln!(out, "{place} = {picture};");
                    return;
                }
                let code = match &to {
                    _ if matches!(value.kind, ExprKind::Empty) => zero(&to),
                    t if is_decimal(t) => self.decimal(value, &mut pre),
                    TypeInfo::Record(_) => deref(&self.value(value, &mut pre)),
                    _ => self.value(value, &mut pre),
                };
                let place = self.place(target, &mut pre);
                let place = if matches!(to, TypeInfo::Record(_)) { deref(&place) } else { place };
                out.push_str(&pre);
                let _ = writeln!(out, "{place} = {};", bare(code));
                if let ExprKind::Name { name } = &target.kind {
                    out.push_str(&self.deselect(name));
                }
            }
            StmtKind::If { branches, otherwise } => self.if_chain(branches, otherwise.as_deref(), out),
            StmtKind::Write { text, store, path } => {
                let path = bare(self.value(path, &mut pre));
                let text = bare(self.value(text, &mut pre));
                out.push_str(&pre);
                let _ = writeln!(out, "if (!clarity_write_text(db, {}, {path}, {text})) return false;", string(&store.name));
            }
            StmtKind::For { var, list, body } => {
                let (item_type, list_c) = match self.types.type_of(list) {
                    TypeInfo::Files => ("ClarityFile".to_string(), "ClarityFileList".to_string()),
                    TypeInfo::List(item) => (item.clone(), list_type(&item)),
                    other => {
                        self.errors.push(format!("a loop goes over a list, not {other:?}"));
                        return;
                    }
                };
                let list = bare(self.value(list, &mut pre));
                // The loop needs its list in a place of its own.
                let list = if is_path(&list) {
                    list
                } else {
                    let held = self.temp();
                    let _ = writeln!(pre, "{list_c} {held} = {list};");
                    held
                };
                out.push_str(&pre);
                self.loops += 1;
                let index = format!("i{}", self.loops);
                let item = ident(&var.name);
                let was_value = self.record_values.remove(&item);
                let mut inner = format!("{item_type}* {item} = &{list}.items[{index}];\n");
                self.block(body, &mut inner);
                if was_value {
                    self.record_values.insert(item);
                }
                let _ = write!(out, "for (size_t {index} = 0; {index} < {list}.count; {index}++) {{\n{}}}\n", indent(&inner, 1));
            }
            StmtKind::Return { value: Some(value) } => {
                let ty = self.returns().unwrap_or(TypeInfo::Unknown);
                let code = match &ty {
                    _ if matches!(value.kind, ExprKind::Empty) => zero(&ty),
                    t if is_decimal(t) => self.decimal(value, &mut pre),
                    TypeInfo::Record(_) => deref(&self.value(value, &mut pre)),
                    _ => self.value(value, &mut pre),
                };
                out.push_str(&pre);
                let _ = write!(out, "*result = {};\nreturn true;\n", bare(code));
            }
            StmtKind::Return { value: None } => out.push_str("return true;\n"),
            StmtKind::Call { call } => {
                let ExprKind::Call { function, args } = &call.kind else {
                    self.errors.push("only a call can stand on its own".into());
                    return;
                };
                // A handler runs another handler of its window.
                if self.types.name_of(call) == Some(&NameKind::Handler) {
                    let prefix = self.window.map(|w| w.prefix.clone()).unwrap_or_default();
                    let _ = writeln!(out, "if (!{prefix}_{}(db, ui, w)) return false;", snake(&function.name));
                    return;
                }
                let returns = self.callee(&function.name).and_then(|f| f.returns.as_ref()).map(|r| type_ref_info(self.project, r));
                let call_code = self.call_code(&function.name, args, &mut pre);
                match returns {
                    // The result nobody uses still needs a place to go.
                    Some(ty) => {
                        self.lift(&value_type(&ty), &call_code, &mut pre);
                        out.push_str(&pre);
                    }
                    None => {
                        out.push_str(&pre);
                        let _ = writeln!(out, "if (!{}) return false;", call_code.replace(", {out}", ""));
                    }
                }
            }
            StmtKind::Save { value } | StmtKind::Delete { value } => {
                let verb = if matches!(stmt.kind, StmtKind::Save { .. }) { "save" } else { "delete" };
                let TypeInfo::Record(entity) = self.types.type_of(value) else {
                    self.errors.push(format!("only a record can be {verb}d"));
                    return;
                };
                let record = bare(self.value(value, &mut pre));
                out.push_str(&pre);
                let _ = writeln!(out, "if (!{}_{verb}(db, {record})) return false;", prefix(&entity));
            }
            StmtKind::Refuse { text } => {
                let text = self.shown(text, &mut pre);
                out.push_str(&pre);
                let _ = writeln!(out, "return clarity_refuse({text});");
            }
            StmtKind::Message { .. } | StmtKind::Open { .. } | StmtKind::Close | StmtKind::Reload { .. } if self.window.is_none() => {
                self.errors.push("messages, windows and window data belong in handlers".into());
            }
            StmtKind::Message { level, text } => {
                let level = match level {
                    MessageLevel::Info => "CLARITY_INFO",
                    MessageLevel::Warning => "CLARITY_WARNING",
                    MessageLevel::Error => "CLARITY_ERROR",
                };
                let text = self.shown(text, &mut pre);
                out.push_str(&pre);
                let _ = writeln!(out, "clarity_ui_message(ui, {level}, {text});");
            }
            StmtKind::Open { window, with } => {
                let mut args = Vec::new();
                for arg in with {
                    let json = self.json(&arg.value, &mut pre);
                    args.push(format!("{}, {json}", string(&arg.name.name)));
                }
                let with = if args.is_empty() { "cJSON_CreateObject()".to_string() } else { format!("clarity_json_object({}, {})", args.len(), args.join(", ")) };
                out.push_str(&pre);
                let _ = writeln!(out, "clarity_ui_open(ui, {}, {with});", string(&window.name));
            }
            StmtKind::Close => out.push_str("clarity_ui_close(ui);\n"),
            StmtKind::Reload { data } => {
                let found = self.window.and_then(|w| w.data.iter().find(|(n, _, _)| *n == data.name)).cloned();
                match found {
                    Some((name, entity, true)) => {
                        let _ = writeln!(out, "if (!{}_all(db, &w->{})) return false;", prefix(&entity), ident(&name));
                        out.push_str(&self.deselect(&name));
                    }
                    Some((name, entity, false)) => {
                        let member = format!("w->{}", ident(&name));
                        let _ = writeln!(out, "if ({member}.id != 0 && !{}_get(db, {member}.id, &{member})) return false;", prefix(&entity));
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
        let _ = write!(out, "if ({condition}) {{\n{}}}", indent(&body, 1));
        let mut rest = rest;
        loop {
            match rest.split_first() {
                Some((next, after)) => {
                    let mut pre = String::new();
                    let condition = bare(self.value(&next.condition, &mut pre));
                    if !pre.is_empty() {
                        let mut inner = pre;
                        let mut body = String::new();
                        self.block(&next.body, &mut body);
                        let _ = write!(inner, "if ({condition}) {{\n{}}}", indent(&body, 1));
                        let mut tail = String::new();
                        self.if_chain_rest(after, otherwise, &mut tail);
                        inner.push_str(&tail);
                        inner.push('\n');
                        let _ = writeln!(out, " else {{\n{}}}", indent(&inner, 1));
                        return;
                    }
                    let mut body = String::new();
                    self.block(&next.body, &mut body);
                    let _ = write!(out, " else if ({condition}) {{\n{}}}", indent(&body, 1));
                    rest = after;
                }
                None => {
                    if let Some(body) = otherwise {
                        let mut code = String::new();
                        self.block(body, &mut code);
                        let _ = write!(out, " else {{\n{}}}", indent(&code, 1));
                    }
                    out.push('\n');
                    return;
                }
            }
        }
    }

    /// The rest of a chain after an `if` inside an `else`: a chain of its own after ` else`.
    fn if_chain_rest(&mut self, branches: &[clarity_spec::code::Branch], otherwise: Option<&[Stmt]>, out: &mut String) {
        if branches.is_empty() {
            if let Some(body) = otherwise {
                let mut code = String::new();
                self.block(body, &mut code);
                let _ = write!(out, " else {{\n{}}}", indent(&code, 1));
            }
            return;
        }
        let mut chain = String::new();
        self.if_chain(branches, otherwise, &mut chain);
        let _ = write!(out, " else {{\n{}}}", indent(&chain, 1));
    }

    /// Text the screen shows as the runtime's text: as written, with a typed value per slot.
    fn shown(&mut self, expr: &Expr, pre: &mut String) -> String {
        match &expr.kind {
            ExprKind::Str { value } => format!("clarity_plain_text({})", string(value)),
            ExprKind::Text { parts } => {
                let mut code = format!("clarity_text({})", string(&template(parts)));
                for part in parts {
                    if let TextPart::Slot { value } = part {
                        let slot = self.slot(value, pre);
                        code = format!("clarity_text_with({code}, {}, {slot})", string(&path_text(value)));
                    }
                }
                code
            }
            _ => {
                let ty = self.types.type_of(expr);
                let value = self.value(expr, pre);
                format!("clarity_plain_text({})", self.show(value, &ty))
            }
        }
    }

    /// A value as text, the way `"text" + value` shows it.
    fn show(&mut self, code: String, ty: &TypeInfo) -> String {
        let code = bare(code);
        match ty {
            TypeInfo::Text => code,
            TypeInfo::Enum(name) => format!("{}_label({code})", prefix(name)),
            TypeInfo::Integer | TypeInfo::Duration => format!("clarity_show_integer({code})"),
            TypeInfo::Decimal | TypeInfo::Money => format!("clarity_show_decimal({code})"),
            TypeInfo::Bool => format!("clarity_show_bool({code})"),
            TypeInfo::Date => format!("clarity_show_date({code})"),
            TypeInfo::DateTime => format!("clarity_show_date_time({code})"),
            other => {
                self.errors.push(format!("{other:?} cannot be shown as text"));
                code
            }
        }
    }

    /// The value of a slot with what the screen needs to show it.
    fn slot(&mut self, expr: &Expr, pre: &mut String) -> String {
        let value = bare(self.value(expr, pre));
        let field = self.field_of(expr);
        let format = field.and_then(|f| f.modifiers.format.as_deref()).map_or_else(|| "NULL".to_string(), string);
        let scale = match field.map(|f| &f.ty) {
            Some(FieldType::Decimal { scale, .. }) => scale.to_string(),
            _ => "-1".to_string(),
        };
        match self.types.type_of(expr) {
            TypeInfo::Text => format!("clarity_slot_text({value})"),
            TypeInfo::Integer => format!("clarity_slot_integer({value}, {format})"),
            TypeInfo::Duration => format!("clarity_slot_duration({value})"),
            TypeInfo::Decimal => format!("clarity_slot_decimal({value}, {scale}, {format})"),
            TypeInfo::Money => format!("clarity_slot_money({value}, {format})"),
            TypeInfo::Date => format!("clarity_slot_date({value}, {format})"),
            TypeInfo::DateTime => format!("clarity_slot_date_time({value}, {format})"),
            TypeInfo::Bool => format!("clarity_slot_bool({value})"),
            TypeInfo::Enum(name) => format!("clarity_slot_enum({}_name({value}), {})", prefix(&name), string(&name)),
            other => {
                self.errors.push(format!("a slot cannot show {other:?}"));
                String::new()
            }
        }
    }

    /// A value as the JSON a window's data holds, such as the `with` of `open`.
    fn json(&mut self, expr: &Expr, pre: &mut String) -> String {
        let ty = self.types.type_of(expr);
        let value = bare(self.value(expr, pre));
        match ty {
            TypeInfo::Record(entity) => format!("{}_json({value})", prefix(&entity)),
            TypeInfo::List(entity) => format!("{}_list_json(&{value})", prefix(&entity)),
            TypeInfo::Text => format!("clarity_json_text({value})"),
            TypeInfo::Integer | TypeInfo::Duration => format!("clarity_json_integer({value})"),
            TypeInfo::Decimal | TypeInfo::Money => format!("clarity_json_decimal({value})"),
            TypeInfo::Bool => format!("clarity_json_bool({value})"),
            TypeInfo::Date => format!("clarity_json_date({value})"),
            TypeInfo::DateTime => format!("clarity_json_date_time({value})"),
            TypeInfo::Enum(name) => format!("clarity_json_text({}_name({value}))", prefix(&name)),
            TypeInfo::Image => format!("clarity_picture_json({value})"),
            other => {
                self.errors.push(format!("{other:?} cannot be given to a window"));
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

    /// An expression as a place: what can be assigned or read in place. A
    /// record is a pointer to it.
    /// After a handler replaced the rows of `data` (`reload`, or a new
    /// list): the tables and lists showing them lose their selected row.
    fn deselect(&self, data: &str) -> String {
        let parts = self.window.and_then(|w| w.showing.get(data)).into_iter().flatten();
        parts.map(|part| format!("clarity_ui_deselect(ui, {});\n", string(part))).collect()
    }

    fn place(&mut self, expr: &Expr, pre: &mut String) -> String {
        if self.part_property(expr).is_some() {
            return self.read_part(expr, pre);
        }
        match &expr.kind {
            ExprKind::Name { name } => match self.types.name_of(expr) {
                Some(NameKind::Local) if self.ensuring && name == "result" => {
                    if matches!(self.returns(), Some(TypeInfo::Record(_))) { "result".into() } else { "(*result)".into() }
                }
                Some(NameKind::Local) => {
                    let local = ident(name);
                    if self.record_values.contains(&local) { format!("&{local}") } else { local }
                }
                Some(NameKind::Data) if self.window.is_some() => {
                    let list = self.window.and_then(|w| w.data.iter().find(|(n, _, _)| n == name)).is_some_and(|(_, _, list)| *list);
                    if list { format!("w->{}", ident(name)) } else { format!("&w->{}", ident(name)) }
                }
                Some(NameKind::EnumValue(e)) => constant(e, name),
                _ => {
                    self.errors.push(format!("'{name}' is not generated yet for C"));
                    String::new()
                }
            },
            ExprKind::Member { object, member } => {
                if let Some(NameKind::EnumValue(e)) = self.types.name_of(expr) {
                    return constant(e, &member.name);
                }
                // A file's details: name, path, extension, size, modified.
                if self.types.type_of(object) == TypeInfo::File {
                    let object = self.place(object, pre);
                    return arrow(&object, &ident(&member.name));
                }
                let TypeInfo::Record(entity) = self.types.type_of(object) else {
                    self.errors.push(format!("'{}' is not generated yet for C", print::expr(expr)));
                    return String::new();
                };
                let object = self.place(object, pre);
                match self.field_type(&entity, &member.name) {
                    // Every record has an id.
                    None if member.name == clarity_spec::code::check::RECORD_ID => arrow(&object, "id"),
                    Some(FieldType::Ref { entity: target }) => {
                        let id = arrow(&object, &ident(&member.name));
                        self.lift_record(&target.name, &format!("{}_load(db, {id}, {{out}})", prefix(&target.name)), pre)
                    }
                    Some(FieldType::List { .. }) => {
                        self.errors.push(format!("list fields like '{}' are not loaded yet", print::expr(expr)));
                        String::new()
                    }
                    _ => arrow(&object, &ident(&member.name)),
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

    /// Reading a part property in a handler; the selected row is a copy of its record.
    fn read_part(&mut self, expr: &Expr, pre: &mut String) -> String {
        let (part, property) = self.part_property(expr).unwrap_or_default();
        let info = self.window.and_then(|w| w.parts.get(&part)).cloned().unwrap_or_default();
        let quoted = string(&part);
        match property.as_str() {
            "visible" | "enabled" | "error" | "text" | "checked" => format!("clarity_ui_{property}(ui, {quoted})"),
            "caption" => format!("clarity_ui_caption(ui, {quoted}, {})", string(info.caption.as_deref().unwrap_or_default())),
            "readonly" => format!("clarity_ui_readonly(ui, {quoted}, {})", info.readonly),
            "selected" => {
                let found = info.bind.and_then(|bind| self.window.and_then(|w| w.data.iter().find(|(n, _, _)| *n == bind)).cloned());
                match found {
                    Some((bind, entity, true)) => {
                        let rows = format!("w->{}", ident(&bind));
                        self.loops += 1;
                        let index = format!("i{}", self.loops);
                        let _ = writeln!(pre, "size_t {index} = 0;\nif (!clarity_ui_selected(ui, {quoted}, {rows}.count, &{index})) return false;");
                        let row = self.temp();
                        let _ = writeln!(pre, "{entity} {row} = {rows}.items[{index}];");
                        format!("&{row}")
                    }
                    _ => {
                        self.errors.push(format!("'{part}' is not bound to a list"));
                        String::new()
                    }
                }
            }
            other => {
                self.errors.push(format!("'{other}' of a part is not generated yet for C"));
                String::new()
            }
        }
    }

    /// An expression as a decimal: an integer is widened.
    fn decimal(&mut self, expr: &Expr, pre: &mut String) -> String {
        if let ExprKind::Number { value } = &expr.kind {
            return decimal(value);
        }
        let ty = self.types.type_of(expr);
        let code = self.value(expr, pre);
        if is_decimal(&ty) { code } else { format!("clarity_decimal_of({})", bare(code)) }
    }

    /// An expression as a value; a record as a pointer to it.
    fn value(&mut self, expr: &Expr, pre: &mut String) -> String {
        let ty = self.types.type_of(expr);
        match &expr.kind {
            ExprKind::Number { value } if is_decimal(&ty) => decimal(value),
            ExprKind::Number { value } => value.clone(),
            ExprKind::Str { value } => string(value),
            ExprKind::Text { .. } => {
                self.errors.push("text with slots is text to show".into());
                String::new()
            }
            ExprKind::Bool { value } => value.to_string(),
            ExprKind::Empty => zero(&ty),
            ExprKind::Name { .. } | ExprKind::Member { .. } => self.place(expr, pre),
            ExprKind::Unary { op: UnaryOp::Not, operand } => format!("!{}", self.value(operand, pre)),
            ExprKind::Unary { op: UnaryOp::Neg, operand } if is_decimal(&ty) => format!("clarity_decimal_neg({})", bare(self.value(operand, pre))),
            ExprKind::Unary { op: UnaryOp::Neg, operand } => format!("-{}", self.value(operand, pre)),
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right, &ty, pre),
            ExprKind::In { value, options } => {
                let value_ty = self.types.type_of(value);
                let decimals = is_decimal(&value_ty) || options.iter().any(|o| is_decimal(&self.types.type_of(o)));
                let v = if decimals { bare(self.decimal(value, pre)) } else { bare(self.value(value, pre)) };
                let tests: Vec<String> = options
                    .iter()
                    .map(|o| {
                        if decimals {
                            format!("clarity_decimal_compare({v}, {}) == 0", bare(self.decimal(o, pre)))
                        } else if value_ty == TypeInfo::Text {
                            format!("clarity_same_text({v}, {})", bare(self.value(o, pre)))
                        } else {
                            format!("{v} == {}", bare(self.value(o, pre)))
                        }
                    })
                    .collect();
                if tests.is_empty() { "false".into() } else { format!("({})", tests.join(" || ")) }
            }
            ExprKind::Call { function, args } => self.call(&function.name, args, pre),
            ExprKind::Find { entity, filter, order, var } => {
                let query = self.find(&var.name, &entity.name, filter.as_deref(), order, pre);
                self.lift(&list_type(&entity.name), &format!("{}_find(db, {query}, {{out}})", prefix(&entity.name)), pre)
            }
            ExprKind::Files { var, store, filter, order } => self.files(&var.name, &store.name, filter.as_deref(), order, pre),
            ExprKind::ReadText { file } => {
                let file = bare(self.value(file, pre));
                self.lift("const char*", &format!("clarity_read_text(db, {file}, {{out}})"), pre)
            }
            ExprKind::New { entity } => {
                let fresh = self.temp();
                let _ = writeln!(pre, "{} {fresh} = {}_new();", entity.name, prefix(&entity.name));
                format!("&{fresh}")
            }
        }
    }

    /// Whether a date, picture or reference field is empty (`x == none`); a
    /// record that is no field is none until it is stored.
    fn is_none(&mut self, expr: &Expr, pre: &mut String) -> String {
        if self.is_reference_field(expr) {
            return format!("{} == 0", self.stored_reference(expr, pre));
        }
        match self.types.type_of(expr) {
            TypeInfo::Record(_) => {
                let record = self.value(expr, pre);
                format!("{} == 0", arrow(&record, "id"))
            }
            TypeInfo::Date => format!("clarity_date_is_none({})", bare(self.value(expr, pre))),
            TypeInfo::DateTime => format!("clarity_date_time_is_none({})", bare(self.value(expr, pre))),
            TypeInfo::Text => format!("clarity_same_text({}, \"\")", bare(self.value(expr, pre))),
            _ => format!("{} == NULL", self.value(expr, pre)),
        }
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
                format!("clarity_text_contains({l}, {r})")
            }
            BinaryOp::Add if *result == TypeInfo::Text => {
                let l = self.value(left, pre);
                let l = self.show(l, &lt);
                let r = self.value(right, pre);
                let r = self.show(r, &rt);
                format!("clarity_join({l}, {r})")
            }
            _ if is_decimal(result) || is_decimal(&lt) || is_decimal(&rt) => {
                let l = bare(self.decimal(left, pre));
                let r = bare(self.decimal(right, pre));
                match op {
                    BinaryOp::Add => format!("clarity_decimal_add({l}, {r})"),
                    BinaryOp::Sub => format!("clarity_decimal_sub({l}, {r})"),
                    BinaryOp::Mul => format!("clarity_decimal_mul({l}, {r})"),
                    BinaryOp::Div => self.lift("ClarityDecimal", &format!("clarity_decimal_div({l}, {r}, {{out}})"), pre),
                    _ => format!("(clarity_decimal_compare({l}, {r}) {} 0)", op.symbol()),
                }
            }
            _ if matches!(lt, TypeInfo::Date | TypeInfo::DateTime) => {
                let compare = if lt == TypeInfo::Date { "clarity_date_compare" } else { "clarity_date_time_compare" };
                let l = bare(self.value(left, pre));
                let r = bare(self.value(right, pre));
                format!("({compare}({l}, {r}) {} 0)", op.symbol())
            }
            _ if matches!(lt, TypeInfo::Record(_)) => {
                let l = self.value(left, pre);
                let r = self.value(right, pre);
                format!("({} {} {})", arrow(&l, "id"), op.symbol(), arrow(&r, "id"))
            }
            _ if lt == TypeInfo::Text => {
                let l = bare(self.value(left, pre));
                let r = bare(self.value(right, pre));
                match op {
                    BinaryOp::Eq => format!("clarity_same_text({l}, {r})"),
                    BinaryOp::NotEq => format!("!clarity_same_text({l}, {r})"),
                    _ => format!("(strcmp({l}, {r}) {} 0)", op.symbol()),
                }
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
        let held = self.temp();
        let mut right_pre = String::new();
        let r = self.value(right, &mut right_pre);
        if right_pre.is_empty() {
            self.temps = before;
            return format!("({l} {symbol} {r})");
        }
        let test = if op == BinaryOp::And { held.clone() } else { format!("!{held}") };
        let _ = write!(pre, "bool {held} = {};\nif ({test}) {{\n{}    {held} = {};\n}}\n", bare(l), indent(&right_pre, 1), bare(r));
        held
    }

    fn callee(&self, name: &str) -> Option<&'a FunctionDecl> {
        self.project.files.iter().flat_map(|f| &f.functions).find(|f| f.name.name == name)
    }

    /// A call that can fail, without lifting it: `{out}` marks where its result goes.
    fn call_code(&mut self, name: &str, args: &[Expr], pre: &mut String) -> String {
        let Some(callee) = self.callee(name) else {
            self.errors.push(format!("unknown function '{name}'"));
            return String::new();
        };
        let mut code = vec!["db".to_string()];
        for (arg, param) in args.iter().zip(&callee.params) {
            let ty = type_ref_info(self.project, &param.ty);
            let value = if is_decimal(&ty) { self.decimal(arg, pre) } else { self.value(arg, pre) };
            code.push(bare(value));
        }
        code.push("{out}".into());
        format!("{}({})", ident(name), code.join(", "))
    }

    fn call(&mut self, name: &str, args: &[Expr], pre: &mut String) -> String {
        use clarity_spec::code::check::{NOW, TODAY};
        if name == TODAY {
            return "clarity_today()".into();
        }
        if name == NOW {
            return "clarity_now()".into();
        }
        if name == "count" {
            let list = args.first().map(|a| bare(self.value(a, pre))).unwrap_or_default();
            return format!("(int64_t){list}.count");
        }
        let returns = self.callee(name).and_then(|f| f.returns.as_ref()).map(|r| type_ref_info(self.project, r)).unwrap_or(TypeInfo::Unknown);
        let call = self.call_code(name, args, pre);
        match returns {
            TypeInfo::Record(entity) => self.lift_record(&entity, &call, pre),
            other => self.lift(&value_type(&other), &call, pre),
        }
    }

    /// `find c in Entity where ... order by ...` as the runtime's query.
    fn find(&mut self, var: &str, entity: &str, filter: Option<&Expr>, order: &[OrderKey], pre: &mut String) -> String {
        let mut query = "clarity_query()".to_string();
        if let Some(filter) = filter {
            let cond = self.cond(filter, var, entity, pre);
            query = format!("clarity_where({query}, {cond})");
        }
        for key in order {
            let Some(path) = crate::query::order_path(self.project, entity, key) else {
                self.errors.push(format!("{entity} cannot be ordered by '{}'", key.field.name));
                continue;
            };
            let numeric = self.field_type(&path.entity, &path.field).is_some_and(|t| is_stored_number(&t));
            let column = string(&self.column(&path.entity, &path.field));
            if path.hops.is_empty() {
                let function = if numeric { "clarity_order_by_number" } else { "clarity_order_by" };
                query = format!("{function}({query}, {column}, {})", key.descending);
            } else {
                let hops: Vec<String> =
                    path.hops.iter().map(|h| format!("{{{}, {}, {}}}", string(&h.column), string(&h.table), string(&h.key))).collect();
                query = format!(
                    "clarity_order_through({query}, {}, (ClarityHop[]){{{}}}, {column}, {}, {numeric})",
                    hops.len(),
                    hops.join(", "),
                    key.descending
                );
            }
        }
        query
    }

    /// A query condition as the runtime's condition.
    fn cond(&mut self, expr: &Expr, var: &str, entity: &str, pre: &mut String) -> String {
        let project = self.project;
        let path_of = |e: &Expr| crate::query::field_path(project, entity, var, e);
        match &expr.kind {
            ExprKind::Binary { op: BinaryOp::And, left, right } => {
                let l = self.cond(left, var, entity, pre);
                format!("clarity_and({l}, {})", self.cond(right, var, entity, pre))
            }
            ExprKind::Binary { op: BinaryOp::Or, left, right } => {
                let l = self.cond(left, var, entity, pre);
                format!("clarity_or({l}, {})", self.cond(right, var, entity, pre))
            }
            ExprKind::Unary { op: UnaryOp::Not, operand } => format!("clarity_not({})", self.cond(operand, var, entity, pre)),
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
                    let test = format!("clarity_is_none({name})");
                    if op == BinaryOp::Eq { test } else { format!("clarity_not({test})") }
                } else if op == BinaryOp::Contains {
                    format!("clarity_contains({name}, {})", bare(self.value(value, pre)))
                } else {
                    let function = if is_stored_number(&ty) { "clarity_number" } else { "clarity_compare" };
                    let value = self.sql_value(&ty, value, pre);
                    format!("{function}({name}, {}, {value})", op_name(op))
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
                let function = if is_stored_number(&ty) { "clarity_one_of_numbers" } else { "clarity_one_of" };
                let values: Vec<String> = options.iter().map(|o| self.sql_value(&ty, o, pre)).collect();
                let own = format!(
                    "{function}({}, {}, (ClarityValue[]){{{}}})",
                    string(&self.column(&path.entity, &path.field)),
                    values.len(),
                    values.join(", ")
                );
                through(&path.hops, own)
            }
            _ => match path_of(expr) {
                Some(path) => through(&path.hops, format!("clarity_is_true({})", string(&self.column(&path.entity, &path.field)))),
                None => {
                    self.errors.push(format!("'{}' is not a condition on {var}", print::expr(expr)));
                    String::new()
                }
            },
        }
    }

    /// A value compared with a field of type `ty`, as the runtime takes it.
    fn sql_value(&mut self, ty: &FieldType, value: &Expr, pre: &mut String) -> String {
        match ty {
            FieldType::Text { .. } | FieldType::Email | FieldType::Url => format!("clarity_text_value({})", bare(self.value(value, pre))),
            FieldType::Integer | FieldType::Id | FieldType::Duration => format!("clarity_integer_value({})", bare(self.value(value, pre))),
            FieldType::Decimal { .. } | FieldType::Money => format!("clarity_decimal_value({})", bare(self.decimal(value, pre))),
            FieldType::Bool => format!("clarity_bool_value({})", bare(self.value(value, pre))),
            FieldType::Date => format!("clarity_date_value({})", bare(self.value(value, pre))),
            FieldType::DateTime => format!("clarity_date_time_value({})", bare(self.value(value, pre))),
            FieldType::Enum { name } => format!("clarity_text_value({}_name({}))", prefix(&name.name), bare(self.value(value, pre))),
            // A reference field holds the id already; a record has its own.
            FieldType::Ref { .. } => match self.types.type_of(value) {
                TypeInfo::Record(_) if self.is_reference_field(value) => format!("clarity_reference_value({})", self.stored_reference(value, pre)),
                TypeInfo::Record(_) => {
                    let record = self.value(value, pre);
                    format!("clarity_reference_value({})", arrow(&record, "id"))
                }
                _ => format!("clarity_reference_value({})", bare(self.value(value, pre))),
            },
            FieldType::Image | FieldType::List { .. } => {
                self.errors.push("a picture or a list field can only be compared with none in a query".into());
                String::new()
            }
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
        let object = self.place(object, pre);
        arrow(&object, &ident(&member.name))
    }
}

/// Whether `code` is a lifted value: `v1`, `v2`.
fn is_temp(code: &str) -> bool {
    code.len() > 1 && code.starts_with('v') && code[1..].chars().all(|c| c.is_ascii_digit())
}

/// `code` with every use of the name `from` as `to`.
fn rename(code: &str, from: &str, to: &str) -> String {
    let mut out = String::new();
    let bytes = code.as_bytes();
    let mut at = 0;
    while let Some(found) = code[at..].find(from) {
        let start = at + found;
        let end = start + from.len();
        let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
        out.push_str(&code[at..start]);
        if (start == 0 || !word(bytes[start - 1])) && (end >= bytes.len() || !word(bytes[end])) {
            out.push_str(to);
        } else {
            out.push_str(from);
        }
        at = end;
    }
    out.push_str(&code[at..]);
    out
}

/// `lines` with the four spaces of one step of indent taken off.
fn dedent(lines: &str) -> String {
    lines.lines().map(|l| format!("{}\n", l.strip_prefix("    ").unwrap_or(l))).collect()
}

/// The names of a function's parameters, as its C code writes them.
fn param_names(decl: &FunctionDecl) -> Vec<String> {
    let mut names = vec!["db".to_string()];
    names.extend(decl.params.iter().map(|p| ident(&p.name.name)));
    if decl.returns.is_some() {
        names.push("result".into());
    }
    names
}

/// `code` with `(void)name;` first for each parameter it never mentions,
/// as C code says a parameter is left alone on purpose.
fn unused(code: &str, params: &[&str]) -> String {
    let mut out = String::new();
    for name in params {
        if !code.lines().any(|l| mentions(l, name)) {
            let _ = writeln!(out, "(void){name};");
        }
    }
    out.push_str(code);
    out
}

/// Whether `line` uses `name` as a word of its own, not as a field after `.` or `->`.
fn mentions(line: &str, name: &str) -> bool {
    let bytes = line.as_bytes();
    let mut start = 0;
    while let Some(at) = line[start..].find(name) {
        let at = start + at;
        let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
        let before = at == 0 || !(word(bytes[at - 1]) || bytes[at - 1] == b'.' || (bytes[at - 1] == b'>' && at >= 2 && bytes[at - 2] == b'-'));
        let end = at + name.len();
        let after = end >= bytes.len() || !word(bytes[end]);
        // Text in quotes is no use of the name.
        let quoted = line[..at].matches('"').count() % 2 == 1;
        if before && after && !quoted {
            return true;
        }
        start = end;
    }
    false
}

/// A condition on the record the steps lead to, as one on the query's own
/// record: each step a `clarity_through`, the first one outermost.
fn through(hops: &[crate::query::Hop], cond: String) -> String {
    hops.iter().rev().fold(cond, |inner, h| format!("clarity_through({}, {}, {}, {inner})", string(&h.column), string(&h.table), string(&h.key)))
}
