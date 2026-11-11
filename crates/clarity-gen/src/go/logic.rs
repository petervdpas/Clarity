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
    AskForm, BinaryOp, Expr, ExprKind, FunctionDecl, FunctionKind, MessageLevel, OrderKey, Stmt, StmtKind, TextPart, TypeRef,
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
    // An entity's `readable when` and `writable when` (docs/APP-CAPABILITIES-PLAN.md
    // step 4): a function each, which a find and a save call.
    for entity in file.entities.iter().filter(|e| !e.record && !e.settings) {
        for (word, condition) in [("readable", &entity.readable), ("writable", &entity.writable)] {
            let Some(condition) = condition else { continue };
            let mut emitter = Emitter::new(project, types, None, None);
            let mut body = String::new();
            let holds = bare(emitter.value(condition, &mut body));
            body.push_str(&format!("return clarity.Yours({holds})\n"));
            let name = &entity.name.name;
            if emitter.errors.is_empty() {
                out.push_str(&format!(
                    "// {function} is `{word} when` of {name}: refused with notYours when it does not hold.\nfunc {function}(db *clarity.Db, {record} *{name}) error {{\n{}}}\n\n",
                    indent(&body, 1),
                    function = access_function(word, name),
                    record = local(&clarity_spec::code::record_name(name)),
                ));
            } else {
                errors.extend(emitter.errors.into_iter().map(|e| format!("{word} when of {name}: {e}")));
            }
        }
    }
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

/// The commands of a module (DESIGN 5.20): each a function taking the
/// database, the session it prints and asks through, the settings (as
/// `w.Settings`, as a window has them) and its arguments and options, and
/// giving its exit code.
pub fn emit_commands(project: &Project, file: &SpecFile, types: &FileTypes) -> Result<String, Vec<String>> {
    let mut out = String::new();
    let mut errors = Vec::new();
    for command in &file.commands {
        let mut emitter = Emitter::new(project, types, None, None);
        emitter.command = true;
        let mut params = vec!["db *clarity.Db".to_string(), "cli *clarity.CliSession".to_string(), "w *CommandData".to_string()];
        for (name, ty) in command.params.iter().map(|p| (&p.name, &p.ty)).chain(command.options.iter().map(|o| (&o.name, &o.ty))) {
            params.push(format!("{} {}", local(&name.name), value_type(&type_ref_info(project, ty))));
            emitter.note_params(std::iter::once((&name.name, ty)));
        }
        let mut body = command.allow.as_ref().map(|allow| allow_check(&mut emitter, allow)).unwrap_or_default();
        emitter.block(&command.body, &mut body);
        // Go refuses code after a return, so the command's own last return stays the last.
        if !body.trim_end().lines().last().is_some_and(|l| l.starts_with("return ")) {
            body.push_str("return 0, nil\n");
        }
        if emitter.errors.is_empty() {
            out.push_str(&format!("// {} is `{}`.\nfunc {}({}) (int, error) {{\n{}}}\n\n", command_function(command), command.line(), command_function(command), params.join(", "), indent(&body, 1)));
        } else {
            errors.extend(emitter.errors.into_iter().map(|e| format!("command '{}': {e}", command.line())));
        }
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// The Go function of a command: `commandCustomerAdd`.
pub fn command_function(command: &clarity_spec::code::CommandDecl) -> String {
    let group = command.group.as_ref().map(|g| pascal(&g.name)).unwrap_or_default();
    format!("command{group}{}", pascal(&command.name.name))
}

pub(crate) use crate::type_ref_info;

/// The Go function of an entity's `readable when` or `writable when`: `readableMember`.
pub(crate) fn access_function(word: &str, entity: &str) -> String {
    format!("{word}{}", pascal(entity))
}

/// The lines that refuse a record code is given, `record` (a pointer), when
/// its entity's `readable when` does not hold; none without one.
pub(crate) fn readable_check(project: &Project, entity: &str, record: &str, fail: &str) -> String {
    if project.entity(entity).is_none_or(|e| e.readable.is_none()) {
        return String::new();
    }
    format!("if err := {}(db, {record}); err != nil {{\n\t{fail}\n}}\n", access_function("readable", entity))
}

/// As `readable_check`, for every record of the list `list`.
pub(crate) fn readable_list_check(project: &Project, entity: &str, list: &str, fail: &str) -> String {
    let check = readable_check(project, entity, &format!("&{list}[i]"), fail);
    if check.is_empty() {
        return check;
    }
    format!("for i := range {list} {{\n{}}}\n", indent(&check, 1))
}

/// `allow when` (docs/APP-CAPABILITIES-PLAN.md step 4): the lines that refuse
/// with notAllowed when `condition` does not hold.
fn allow_check(emitter: &mut Emitter, condition: &clarity_spec::code::Expr) -> String {
    let mut pre = String::new();
    let holds = bare(emitter.value(condition, &mut pre));
    pre.push_str(&format!("if err := clarity.Allow({holds}); err != nil {{\n\t{}\n}}\n", emitter.fail("err")));
    pre
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

/// One of several values of a call, as gofmt writes it there: a sum of
/// two plain operands without spaces (`p.Name+"/c"`); a longer one keeps
/// its parentheses, inside which gofmt keeps the spaces; anything else bare.
fn call_arg(code: String) -> String {
    let inner = bare(code.clone());
    if inner.len() == code.len() {
        return code;
    }
    // The operators at the top level, outside brackets and literals.
    let bytes = inner.as_bytes();
    let (mut depth, mut quote, mut operators, mut spaces) = (0i32, None::<u8>, Vec::new(), 0usize);
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match quote {
            Some(q) if c == b'\\' && q != b'`' => i += 1,
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                b'"' | b'`' | b'\'' => quote = Some(c),
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                b' ' if depth == 0 => {
                    spaces += 1;
                    if bytes.get(i + 2) == Some(&b' ') && matches!(bytes.get(i + 1), Some(b'+' | b'-' | b'*' | b'/' | b'%')) {
                        operators.push(i);
                    }
                }
                _ => {}
            },
        }
        i += 1;
    }
    match operators.as_slice() {
        // Only the operator's own two spaces, and no sign on the right that gofmt keeps apart.
        [at] if spaces == 2 && !matches!(bytes.get(at + 3), Some(b'-' | b'+' | b'*' | b'&')) => format!("{}{}{}", &inner[..*at], &inner[at + 1..at + 2], &inner[at + 3..]),
        [] => inner,
        _ => code,
    }
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
    /// A popover or slide-out: closed until code or a click opens it, so `visible` reads false until then.
    pub closed: bool,
    /// A canvas's grid as its line says (0 for none), and whether code snaps to it.
    pub grid: u32,
    pub snap: bool,
    /// A canvas's background as its line says; empty is the theme's panel.
    pub background: String,
    /// A canvas's grid dots as its line says: pixels across (2 without it) and color (empty is the theme's border).
    pub dot_size: u32,
    /// A spin box's range as its line writes it.
    pub spin: (i64, i64),
    pub dot_color: String,
    /// A canvas's mode (DESIGN 5.31), the list of its objects, and its tool, brush and ink as its line says.
    pub mode: Option<clarity_spec::model::CanvasMode>,
    pub objects: Option<String>,
    pub tool: String,
    pub brush: u32,
    pub ink: String,
    /// A custom part (DESIGN 5.19): its kind and the values written on its line.
    pub custom: Option<(String, std::collections::BTreeMap<String, clarity_spec::model::Literal>)>,
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
                let custom = crate::parts::kept_of(&widget.props);
                let info = PartInfo { bind: widget.props.bind.clone(), caption: widget.props.caption.clone(), readonly: widget.props.readonly, folded: widget.props.folded, collapsed: false, closed: matches!(widget.props.kind, clarity_spec::widgets::WidgetKind::Popover | clarity_spec::widgets::WidgetKind::Slideout), grid: widget.props.grid.unwrap_or(0), snap: widget.props.snap, background: widget.props.background.clone().unwrap_or_default(), dot_size: widget.props.dot_size.unwrap_or(2), dot_color: widget.props.dot_color.clone().unwrap_or_default(), spin: crate::spin_range(&widget.props), mode: widget.props.mode, objects: widget.props.objects.as_ref().map(|o| o.list.clone()), tool: crate::canvas_tool(&widget.props), brush: widget.props.brush.unwrap_or(2), ink: widget.props.ink.clone().unwrap_or_else(|| "text".to_string()), custom };
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
    /// The function of the window's `allow when`, checked before it opens.
    pub allow: Option<String>,
    /// The function of the `allow when` of its frame's commands that run each handler.
    pub handler_allows: Vec<(String, String)>,
    /// The function of the `allow when` of its frame's commands that open each window.
    pub open_allows: Vec<(String, String)>,
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
                // A list of records starts empty: code builds it (DESIGN 5.12).
                DataKind::List { entity } if project.entity(&entity.name).is_some_and(|e| e.record) => {
                    let _ = writeln!(load, "\tw.{field_name} = []{}{{}}", entity.name);
                }
                DataKind::List { entity } => {
                    let entity = &entity.name;
                    let _ = write!(load, "\t{local_name}, err := clarity.All[{entity}](db)\n\tif err != nil {{\n\t\treturn nil, err\n\t}}\n");
                    // Every record the window is given is checked against `readable when`.
                    load.push_str(&indent(&readable_list_check(project, entity, &local_name, "return nil, err"), 1));
                    let _ = writeln!(load, "\tw.{field_name} = {local_name}");
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
            emitter.note_params(handler.params.iter().map(|p| (&p.name.name, &p.ty)));
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
        let answers = crate::given_values_handlers(page, &written)
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
        // `allow when` (docs/APP-CAPABILITIES-PLAN.md step 4): a window's, decided before it opens,
        // and its frame's commands', checked when the handler runs or the window opens.
        let mut allows: Vec<(String, Vec<&clarity_spec::code::Expr>)> = Vec::new();
        let allow = page.and_then(|p| p.allow_condition.as_ref()).map(|condition| {
            let function = format!("allow{}", pascal(name));
            allows.push((function.clone(), vec![condition]));
            function
        });
        let (frame, widgets) = match (page, app) {
            (Some(p), _) => (&p.frame, p.widgets.as_slice()),
            (None, Some(a)) => (&a.frame, &[][..]),
            (None, None) => unreachable!("a window or the app"),
        };
        let mut handler_allows: Vec<(String, String)> = Vec::new();
        let mut open_allows: Vec<(String, String)> = Vec::new();
        for command in clarity_spec::code::check::all_commands(frame, widgets) {
            let Some(condition) = &command.props.allow_condition else { continue };
            let (function, kept) = if let Some(handler) = &command.props.handler {
                (format!("allow{}{}", pascal(name), pascal(handler)), (&mut handler_allows, handler.clone()))
            } else if let Some(window) = command.props.on_click.as_ref().filter(|w| project.pages().any(|p| p.name.name == **w)) {
                (format!("allow{}Opens{}", pascal(name), pascal(window)), (&mut open_allows, window.clone()))
            } else {
                continue;
            };
            let (list, key) = kept;
            if !list.iter().any(|(k, _)| *k == key) {
                list.push((key, function.clone()));
            }
            match allows.iter_mut().find(|(f, _)| *f == function) {
                Some((_, conditions)) => conditions.push(condition),
                None => allows.push((function, vec![condition])),
            }
        }
        for (function, conditions) in &allows {
            let mut emitter = Emitter::new(project, types, None, None);
            let mut body = String::new();
            for condition in conditions {
                body.push_str(&allow_check(&mut emitter, condition));
            }
            body.push_str("return nil\n");
            if emitter.errors.is_empty() {
                let _ = write!(out, "// {function} is who may: refused with notAllowed when `allow when` does not hold.\nfunc {function}(db *clarity.Db) error {{\n{}}}\n\n", indent(&body, 1));
            } else {
                errors.extend(emitter.errors.into_iter().map(|e| format!("allow when of {name}: {e}")));
            }
        }
        owners.push(Owner {
            calls, name: name.clone(), struct_name, handlers, is_app: app.is_some(), events, draws, answers, allow, handler_allows, open_allows });
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
    /// Whether this is a command of the command-line program (DESIGN 5.20):
    /// it prints, asks and says through `cli`, and gives its exit code.
    command: bool,
    /// The entity of each record local and parameter by its name, for
    /// `reload` of a record, which names it bare.
    records: HashMap<String, String>,
}

impl<'a> Emitter<'a> {
    fn new(project: &'a Project, types: &'a FileTypes, decl: Option<&'a FunctionDecl>, window: Option<&'a WindowCtx>) -> Self {
        let mut emitter = Emitter { project, types, decl, window, errors: Vec::new(), temps: 0, loops: 0, ensuring: false, command: false, records: HashMap::new() };
        if let Some(decl) = decl {
            emitter.note_params(decl.params.iter().map(|p| (&p.name.name, &p.ty)));
        }
        emitter
    }

    /// Notes the record parameters among `params`.
    fn note_params<'p>(&mut self, params: impl Iterator<Item = (&'p String, &'p TypeRef)>) {
        for (name, ty) in params {
            self.note_record(name, &type_ref_info(self.project, ty));
        }
    }

    /// Notes local `name` when it holds a record.
    fn note_record(&mut self, name: &str, ty: &TypeInfo) {
        if let TypeInfo::Record(entity) = ty {
            self.records.insert(name.to_string(), entity.clone());
        }
    }

    fn returns(&self) -> Option<TypeInfo> {
        self.decl.and_then(|d| d.returns.as_ref()).map(|r| type_ref_info(self.project, r))
    }

    /// The statement that returns `err`.
    fn fail(&self, err: &str) -> String {
        if self.command {
            return format!("return 0, {err}");
        }
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
        let name = if decl.kind == FunctionKind::Event { local(&decl.name.name) } else { function_name(self.project, &decl.name.name) };
        let ret = self.returns().map(|r| format!("({}, error)", value_type(&r))).unwrap_or_else(|| "error".into());
        let mut body = decl.allow.as_ref().map(|allow| allow_check(self, allow)).unwrap_or_default();
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

    /// The headers or query pairs of an http call; with `when`, the list is
    /// built in lines before it, leaving out a pair whose condition does not hold.
    fn http_pairs(&mut self, pairs: &[clarity_spec::code::HttpPair], pre: &mut String) -> String {
        if pairs.is_empty() {
            return "nil".to_string();
        }
        let item = |this: &mut Self, p: &clarity_spec::code::HttpPair, pre: &mut String| {
            format!("clarity.Header{{Name: {}, Value: {}}}", string(&p.name.name), bare(this.value(&p.value, pre)))
        };
        if pairs.iter().all(|p| p.when.is_none()) {
            let items: Vec<String> = pairs.iter().map(|p| item(self, p, pre).trim_start_matches("clarity.Header").to_string()).collect();
            return format!("[]clarity.Header{{{}}}", items.join(", "));
        }
        self.temps += 1;
        let list = format!("v{}", self.temps);
        pre.push_str(&format!("{list} := []clarity.Header{{}}\n"));
        for p in pairs {
            let value = item(self, p, pre);
            match &p.when {
                Some(when) => {
                    let cond = bare(self.value(when, pre));
                    pre.push_str(&format!("if {cond} {{\n\t{list} = append({list}, {value})\n}}\n"));
                }
                None => pre.push_str(&format!("{list} = append({list}, {value})\n")),
            }
        }
        list
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

    /// The entity or record of the rows of the window's list data `list`, whose
    /// New function makes the empty record a table with no row chosen gives.
    fn row_entity(&self, list: &str) -> String {
        self.window.and_then(|w| w.data.iter().find(|(n, _, _)| n == list)).map(|(_, entity, _)| entity.clone()).unwrap_or_default()
    }

    /// A picture given to a picture function: a record's picture read first
    /// (its bytes from the database or its file store), read bytes made a picture.
    fn picture_in_hand(&mut self, expr: &Expr, pre: &mut String) -> String {
        if self.types.type_of(expr) == TypeInfo::Bytes {
            return format!("&clarity.Picture{{Bytes: {}}}", bare(self.value(expr, pre)));
        }
        let Some(read) = crate::read_picture(self.project, |e| self.types.type_of(e), expr) else {
            return bare(self.value(expr, pre));
        };
        let object = self.place(read.object, pre);
        let call = format!(
            "clarity.CopiedPicture[{}](db, {object}.{}, {object}.ID, {}, {}, \"\")",
            read.entity,
            field(&read.field),
            string(&read.column),
            string(&read.from.unwrap_or_default())
        );
        self.lift(call, pre)
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
                let held = declared.clone().unwrap_or_else(|| self.types.type_of(value));
                self.note_record(&name.name, &held);
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
            StmtKind::Assign { target, value } if clarity_spec::code::sets_splash_status(target) => {
                let text = self.shown(value, &mut pre);
                out.push_str(&pre);
                out.push_str(&format!("clarity.SetSplashStatus({text})\n"));
            }
            StmtKind::Assign { target, value } => {
                if let Some((part, property)) = self.part_property(target)
                    && self.custom_property(&part, &property)
                {
                    let code = bare(self.value(value, &mut pre));
                    out.push_str(&pre);
                    out.push_str(&format!("ui.SetPartValue({}, {}, {code})\n", string(&part), string(&property)));
                    return;
                }
                // Code selects a row: the first that is the record given (a
                // stored one by its id, a record by its fields), or none.
                if let Some((part, property)) = self.part_property(target)
                    && property == "selected"
                {
                    let Some(bind) = self.window.and_then(|w| w.parts.get(&part)).and_then(|i| i.bind.clone()) else {
                        self.errors.push(format!("'{part}' is not bound to a list"));
                        return;
                    };
                    if matches!(value.kind, ExprKind::Empty) {
                        out.push_str(&format!("ui.Select({}, -1)\n", string(&part)));
                        return;
                    }
                    let stored = matches!(self.types.type_of(value), TypeInfo::Record(e) if self.project.entity(&e).is_some_and(|d| !d.record));
                    let helper = if stored { "SelectStored" } else { "SelectRecord" };
                    let code = bare(self.value(value, &mut pre));
                    out.push_str(&pre);
                    out.push_str(&format!("clarity.{helper}(ui, {}, w.{}, {code})\n", string(&part), pascal(&bind)));
                    return;
                }
                if let Some((part, property)) = self.part_property(target) {
                    let method = match property.as_str() {
                        "caption" => "SetCaption",
                        "error" => "SetError",
                        "visible" => "SetVisible",
                        "enabled" => "SetEnabled",
                        "readonly" => "SetReadonly",
                        "folded" => "SetFolded",
                        "collapsed" => "SetCollapsed",
                        "grid" => "SetGrid",
                        "snap" => "SetSnap",
                        "connecting" => "SetConnecting",
                        "resizable" => "SetResizable",
                        "background" => "SetBackground",
                        "dotSize" => "SetDotSize",
                        "min" => "SetSpinMin",
                        "max" => "SetSpinMax",
                        "dotColor" => "SetDotColor",
                        "tool" => "SetTool",
                        "brush" => "SetBrush",
                        "ink" => "SetInk",
                        "maximized" => "SetMaximized",
                        "pinned" => "SetPinned",
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
                    // Another reference is copied as it is, set or not.
                    let record = if matches!(value.kind, ExprKind::Empty) {
                        "nil".to_string()
                    } else if self.is_reference_field(value) {
                        self.stored_reference(value, &mut pre)
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
                // Another record's picture or file: copied, its bytes read when it is only stored.
                if let Some(copy) = crate::copied(self.project, |e| self.types.type_of(e), target, value) {
                    let made = if copy.file { "CopiedFile" } else { "CopiedPicture" };
                    let object = self.place(copy.object, &mut pre);
                    let copied = self.lift(
                        format!(
                            "clarity.{made}[{}](db, {object}.{}, {object}.ID, {}, {}, {})",
                            copy.entity,
                            field(&copy.field),
                            string(&copy.column),
                            string(&copy.from.unwrap_or_default()),
                            string(&copy.to.unwrap_or_default())
                        ),
                        &mut pre,
                    );
                    let place = self.place(target, &mut pre);
                    out.push_str(&pre);
                    out.push_str(&format!("{place} = {copied}\n"));
                    return;
                }
                // Text assigned to a hashed field keeps its hash (docs/APP-CAPABILITIES-PLAN.md
                // step 4), by the field's method, refused when shorter than its minimum.
                if let ExprKind::Member { object, member } = &target.kind
                    && let TypeInfo::Record(owner) = self.types.type_of(object)
                    && self.project.entity(&owner).and_then(|e| e.field(&member.name)).is_some_and(|f| f.modifiers.hashed)
                {
                    let hashing = crate::signin::hashing(self.project, &member.name);
                    let text = call_arg(self.value(value, &mut pre));
                    let minimum = hashing.minimum.map(|m| format!(", {m}")).unwrap_or_default();
                    let kept = self.lift(format!("clarity.Hashed(authCall, {}, {text}{minimum})", string(hashing.method)), &mut pre);
                    let place = self.place(target, &mut pre);
                    out.push_str(&pre);
                    out.push_str(&format!("{place} = {kept}\n"));
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
                if let TypeInfo::List(entity) = self.types.type_of(list) {
                    self.note_record(&var.name, &TypeInfo::Record(entity));
                }
                // Files are worked on as values, records through pointers.
                let values = matches!(self.types.type_of(list), TypeInfo::Values(_));
                let by_value = self.types.type_of(list) == TypeInfo::Files;
                let list = bare(self.value(list, &mut pre));
                // Plain values one by one, from a copy, so the body may change the list.
                if values {
                    out.push_str(&pre);
                    let mut inner = String::new();
                    self.block(body, &mut inner);
                    // A loop that never reads its value takes none: Go refuses a variable it never reads.
                    let name = local(&var.name);
                    let head = if inner.lines().any(|l| mentions(l, &name)) { format!("for _, {name} := range") } else { "for range".to_string() };
                    out.push_str(&format!("{head} {list} {{\n{}}}\n", indent(&unused_locals(&inner), 1)));
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
            StmtKind::Return { value: None } if self.command => out.push_str("return 0, nil\n"),
            StmtKind::Return { value: None } => {
                out.push_str(if self.returns().is_some() { "return\n" } else { "return nil\n" });
            }
            StmtKind::Call { call } => {
                let ExprKind::Call { function, args, .. } = &call.kind else {
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
                let returns = self.project.files.iter().flat_map(|f| &f.functions).find(|f| f.name.name == function.name).and_then(|f| f.returns.as_ref());
                let take = if returns.is_some() { "_, err" } else { "err" };
                out.push_str(&format!("if {take} := {call_code}; err != nil {{\n\t{}\n}}\n", self.fail("err")));
            }
            StmtKind::Save { value, .. } | StmtKind::Delete { value } => {
                // `save ... anyway` writes regardless of the version (docs/APP-CAPABILITIES-PLAN.md step 5).
                let verb = match stmt.kind {
                    StmtKind::Save { anyway: true, .. } => "SaveAnyway",
                    StmtKind::Save { .. } => "Save",
                    _ => "Delete",
                };
                // An item of a list is saved in place: the record in the list, by its position from 1.
                let record = if let ExprKind::Index { list, index } = &value.kind {
                    let list = self.list_ref(list, &mut pre);
                    let at = bare(self.value(index, &mut pre));
                    let at = self.held_operand(at, &mut pre);
                    pre.push_str(&format!("if _, err := clarity.At({list}, {at}); err != nil {{\n\t{}\n}}\n", self.fail("err")));
                    format!("&{list}[{at}-1]")
                } else {
                    bare(self.value(value, &mut pre))
                };
                out.push_str(&pre);
                out.push_str(&format!("if err := clarity.{verb}(db, {record}); err != nil {{\n\t{}\n}}\n", self.fail("err")));
            }
            StmtKind::Refuse { text } => {
                let text = self.shown(text, &mut pre);
                out.push_str(&pre);
                out.push_str(&format!("{}\n", self.fail(&format!("clarity.Refuse({text})"))));
            }
            // A command says it on standard error.
            StmtKind::Message { text, .. } if self.command => {
                let text = self.shown(text, &mut pre);
                out.push_str(&pre);
                out.push_str(&format!("cli.Message({text})\n"));
            }
            // A record local or parameter is read again as it is stored now, in place.
            StmtKind::Reload { data } if self.records.contains_key(&data.name) => {
                let entity = self.records[&data.name].clone();
                let record = local(&data.name);
                let fail = self.fail("err");
                out.push_str(&format!("if err := clarity.Reload(db, {record}); err != nil {{\n\t{fail}\n}}\n"));
                out.push_str(&readable_check(self.project, &entity, &record, &fail));
            }
            StmtKind::Message { .. } | StmtKind::Open { .. } | StmtKind::Close | StmtKind::Reload { .. } if self.window.is_none() => {
                self.errors.push("messages, windows and window data belong in handlers".into());
            }
            // A shape on a canvas, as conformance/drawing.json sends it.
            // `generate` refuses a project with custom parts until CP1 builds them.
            // A hear of a custom part (DESIGN 5.19), sent with the reply.
            // An engine's statement; engines are compiled to WebAssembly before generating (N5b).
            StmtKind::Tell { .. } => self.errors.push("'tell' is for an engine beside the screen".into()),
            StmtKind::Hear { part, hear, args } => {
                let values: String = args.iter().map(|a| format!(", {}", bare(self.value(a, &mut pre)))).collect();
                out.push_str(&pre);
                out.push_str(&format!("ui.Hear({}, {}{values})\n", string(&part.name), string(&hear.name)));
            }
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
                        (ShapeValue::Color | ShapeValue::Picture | ShapeValue::Name | ShapeValue::Points, Ok(arg)) => bare(self.value(arg, &mut pre)),
                        (ShapeValue::Color | ShapeValue::Picture | ShapeValue::Text | ShapeValue::Name | ShapeValue::Points, Err(default)) => string(default),
                        // An object sends its record's id (DESIGN 5.31).
                        (ShapeValue::Record, Ok(arg)) => bare(self.value(&clarity_spec::code::check::object_id(arg), &mut pre)),
                        (ShapeValue::Record, Err(_)) => "nil".to_string(),
                        (ShapeValue::Text, Ok(arg)) => self.shown(arg, &mut pre),
                    };
                    fields.push(format!("{}: {value}", string(key)));
                }
                out.push_str(&pre);
                out.push_str(&format!("ui.Draw({}, map[string]any{{{}}})\n", string(&canvas.name), fields.join(", ")));
            }
            StmtKind::Copy { text } => {
                let text = self.shown(text, &mut pre);
                out.push_str(&pre);
                out.push_str(&format!("ui.Copy({text})\n"));
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
            StmtKind::Print { value, columns } if columns.is_empty() => {
                let text = self.shown(value, &mut pre);
                out.push_str(&pre);
                out.push_str(&format!("cli.Print({text})\n"));
            }
            StmtKind::Print { value, columns } => {
                let code = self.print_table(value, columns, &mut pre);
                out.push_str(&pre);
                out.push_str(&code);
            }
            StmtKind::Exit { code } => {
                let code = bare(self.value(code, &mut pre));
                out.push_str(&pre);
                out.push_str(&format!("return int({code}), nil\n"));
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
                        let fail = self.fail("err");
                        pre.push_str(&readable_list_check(self.project, &entity, &all, &fail));
                        out.push_str(&pre);
                        out.push_str(&format!("w.{} = {all}\n", pascal(&name)));
                        out.push_str(&self.deselect(&name));
                    }
                    Some((name, entity, false)) => {
                        let field_name = format!("w.{}", pascal(&name));
                        let mut inner = String::new();
                        let fresh = self.lift(format!("clarity.Get[{entity}](db, {field_name}.ID)"), &mut inner);
                        let fail = self.fail("err");
                        inner.push_str(&readable_check(self.project, &entity, &format!("&{fresh}"), &fail));
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

    /// `print list columns a, b`: a heading per field (its label), then the
    /// records' values as the command line writes them.
    fn print_table(&mut self, list: &Expr, columns: &[clarity_spec::model::Ident], pre: &mut String) -> String {
        let TypeInfo::List(entity) = self.types.type_of(list) else {
            self.errors.push("'columns' prints a list of records".into());
            return String::new();
        };
        let Some(decl) = self.project.entity(&entity) else { return String::new() };
        let (mut headings, mut right, mut cells) = (Vec::new(), Vec::new(), Vec::new());
        for column in columns {
            let found = decl.field(&column.name);
            let label = found.and_then(|f| f.modifiers.label.clone()).unwrap_or_else(|| clarity_spec::model::default_label(&column.name));
            headings.push(match clarity_spec::translations::key_of(&label) {
                Some(key) => format!("clarity.KeyText({})", string(key)),
                None => format!("clarity.PlainText({})", string(&label)),
            });
            let ty = found.map(|f| &f.ty);
            right.push(matches!(ty, Some(FieldType::Integer | FieldType::Decimal { .. } | FieldType::Money | FieldType::Duration) | None).to_string());
            let at = if column.name == "id" { "r.ID".to_string() } else { format!("r.{}", field(&column.name)) };
            cells.push(match ty {
                Some(FieldType::Bool) => format!("clarity.CliYesNo({at})"),
                _ => format!("clarity.CliTextOf({at})"),
            });
        }
        let records = bare(self.value(list, pre));
        self.temps += 1;
        let rows = format!("v{}", self.temps);
        pre.push_str(&format!("{rows} := [][]string{{}}\nfor _, r := range {records} {{\n\t{rows} = append({rows}, []string{{{}}})\n}}\n", cells.join(", ")));
        format!("cli.PrintTable([]clarity.Text{{{}}}, []bool{{{}}}, {rows})\n", headings.join(", "), right.join(", "))
    }

    /// `ask` in a command: it waits on the terminal and gives the answer.
    fn ask_value(&mut self, question: &Expr, form: &AskForm, pre: &mut String) -> String {
        let question = self.shown(question, pre);
        match form {
            AskForm::Confirm { yes, no } => {
                let yes = yes.as_ref().map_or("nil".to_string(), |t| format!("clarity.CliWord({})", self.shown(t, pre)));
                let no = no.as_ref().map_or("nil".to_string(), |t| format!("clarity.CliWord({})", self.shown(t, pre)));
                format!("cli.Confirm({question}, {yes}, {no})")
            }
            AskForm::Value { ty, from: Some(list), .. } => {
                let entity = match type_ref_info(self.project, ty) {
                    TypeInfo::Record(e) => e,
                    _ => String::new(),
                };
                let shown = self.project.entity(&entity).and_then(|e| e.fields.iter().find(|f| matches!(f.ty, FieldType::Text { .. }))).map(|f| field(&f.name.name)).unwrap_or_else(|| "ID".to_string());
                let records = bare(self.value(list, pre));
                self.temps += 2;
                let (held, names) = (format!("v{}", self.temps - 1), format!("v{}", self.temps));
                pre.push_str(&format!("{held} := {records}\n{names} := make([]string, len({held}))\nfor i := range {held} {{\n\t{names}[i] = clarity.CliTextOf({held}[i].{shown})\n}}\n"));
                format!("&{held}[cli.Choose({question}, {names})]")
            }
            AskForm::Value { ty, default, from: None } => {
                let info = type_ref_info(self.project, ty);
                let fallback = default.as_ref().map_or("nil".to_string(), |d| format!("clarity.CliDefault(clarity.CliTextOf({}))", bare(self.value(d, pre))));
                let asked = |kind: &str, values: &str| format!("cli.Ask({question}, {}, {values}, {fallback})", string(kind));
                match &info {
                    TypeInfo::Integer => format!("clarity.CliInteger({})", asked("integer", "nil")),
                    TypeInfo::Duration => format!("clarity.CliInteger({})", asked("duration", "nil")),
                    TypeInfo::Decimal => format!("clarity.CliDecimal({})", asked("decimal", "nil")),
                    TypeInfo::Money => format!("clarity.CliDecimal({})", asked("money", "nil")),
                    TypeInfo::Date => format!("clarity.CliDate({})", asked("date", "nil")),
                    TypeInfo::DateTime => format!("clarity.CliDateTime({})", asked("datetime", "nil")),
                    TypeInfo::Enum(name) => {
                        let values: Vec<String> = self.project.enum_decl(name).map(|e| e.variants.iter().map(|v| string(&v.name.name)).collect()).unwrap_or_default();
                        format!("{name}(clarity.CliText({}))", asked("enum", &format!("[]string{{{}}}", values.join(", "))))
                    }
                    _ => format!("clarity.CliText({})", asked("text", "nil")),
                }
            }
            AskForm::Window { .. } => {
                self.errors.push("a command asks no window".into());
                String::new()
            }
        }
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
                // A command has the settings as data of its own, as a window does.
                Some(NameKind::Data) if self.window.is_some() || self.command => format!("w.{}", pascal(name)),
                Some(NameKind::EnumValue(e)) => constant(e, name),
                // `me`: the record signed in for this call, refused with notAllowed for nobody.
                Some(NameKind::Me) => {
                    let TypeInfo::Record(people) = self.types.type_of(expr) else {
                        self.errors.push("'me' names no entity of people".into());
                        return String::new();
                    };
                    let me = self.lift(format!("clarity.Me[{people}](db)"), pre);
                    format!("(&{me})")
                }
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
                // A reply's Status, Ok and Text.
                if self.types.type_of(object) == TypeInfo::HttpReply {
                    let object = self.place(object, pre);
                    return format!("{object}.{}", pascal(&member.name));
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
                // A field of `me` is read from the record itself.
                let object = match object.strip_prefix("(&").and_then(|o| o.strip_suffix(')')) {
                    Some(record) if is_ident(record) => record.to_string(),
                    _ => object,
                };
                match self.field_type(&entity, &member.name) {
                    // Every record has an id.
                    None if member.name == clarity_spec::code::check::RECORD_ID => format!("{object}.ID"),
                    Some(FieldType::Ref { entity: target }) => {
                        let record = self.lift(format!("clarity.Ref[{}](db, {object}.{})", target.name, field(&member.name)), pre);
                        let fail = self.fail("err");
                        pre.push_str(&readable_check(self.project, &target.name, &record, &fail));
                        record
                    }
                    Some(FieldType::List { .. }) if !self.project.in_web_api(&entity) => {
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

    /// Whether `property` is a custom part's own (DESIGN 5.19).
    fn custom_property(&self, part: &str, property: &str) -> bool {
        let info = self.window.and_then(|w| w.parts.get(part));
        info.and_then(|i| i.custom.as_ref()).is_some_and(|(kind, _)| crate::parts::keeps(self.project, kind, property))
    }

    /// Reading a part property in a handler; the selected row is a record.
    fn read_part(&mut self, expr: &Expr, pre: &mut String) -> String {
        let (part, property) = self.part_property(expr).unwrap_or_default();
        let info = self.window.and_then(|w| w.parts.get(&part)).cloned().unwrap_or_default();
        let quoted = string(&part);
        // A custom part's own property: as a handler set it, else as written or declared.
        if self.custom_property(&part, &property)
            && let Some((kind, written)) = &info.custom
        {
            let fallback = crate::parts::fallback_json(self.project, kind, written, &property);
            let ty = value_type(&self.types.type_of(expr));
            return self.lift(format!("clarity.PartValueAs[{ty}](ui, {quoted}, {}, {})", string(&property), string(&fallback)), pre);
        }
        match property.as_str() {
            "visible" if info.closed => format!("ui.VisibleOr({quoted}, false)"),
            "visible" => format!("ui.Visible({quoted})"),
            "enabled" => format!("ui.Enabled({quoted})"),
            "caption" => format!("ui.Caption({quoted}, {})", string(info.caption.as_deref().unwrap_or_default())),
            "error" => format!("ui.Error({quoted})"),
            "readonly" => format!("ui.Readonly({quoted}, {})", info.readonly),
            "folded" => format!("ui.Folded({quoted}, {})", info.folded),
            "collapsed" => format!("ui.Collapsed({quoted}, {})", info.collapsed),
            "grid" => format!("ui.Grid({quoted}, {})", info.grid),
            "snap" => format!("ui.Snap({quoted}, {})", info.snap),
            "connecting" => format!("ui.Connecting({quoted})"),
            "resizable" => format!("ui.Resizable({quoted})"),
            "background" => format!("ui.Background({quoted}, {})", string(&info.background)),
            "dotSize" => format!("ui.DotSize({quoted}, {})", info.dot_size),
            "min" => format!("ui.SpinMin({quoted}, {})", info.spin.0),
            "max" => format!("ui.SpinMax({quoted}, {})", info.spin.1),
            "dotColor" => format!("ui.DotColor({quoted}, {})", string(&info.dot_color)),
            "tool" => format!("ui.Tool({quoted}, {})", string(&info.tool)),
            "brush" => format!("ui.Brush({quoted}, {})", info.brush),
            "ink" => format!("ui.Ink({quoted}, {})", string(&info.ink)),
            // A canvas's choice (DESIGN 5.31): the object's record, or the shape's place from 1.
            "selected" if info.mode == Some(clarity_spec::model::CanvasMode::Vector) => format!("ui.ShapeChosen({quoted})"),
            "selected" if info.objects.is_some() => {
                let list = info.objects.clone().unwrap_or_default();
                format!("clarity.Selected(ui, {quoted}, w.{}, New{})", pascal(&list), self.row_entity(&list))
            }
            "maximized" => format!("ui.Maximized({quoted})"),
            "pinned" => format!("ui.Pinned({quoted})"),
            "tab" => format!("ui.Tab({quoted})"),
            "step" => format!("ui.Step({quoted})"),
            "open" => format!("ui.OpenSection({quoted})"),
            "text" => format!("ui.Text({quoted})"),
            "checked" => format!("ui.Checked({quoted})"),
            "x" => format!("ui.X({quoted})"),
            "y" => format!("ui.Y({quoted})"),
            "fromX" => format!("ui.FromX({quoted})"),
            "bent" => format!("ui.Bent({quoted})"),
            "via" => format!("ui.Via({quoted})"),
            "fromY" => format!("ui.FromY({quoted})"),
            "selected" => match info.bind {
                Some(bind) => format!("clarity.Selected(ui, {quoted}, w.{}, New{})", pascal(&bind), self.row_entity(&bind)),
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
    /// A format's options as a Go expression of their JSON object (`""` for none).
    /// The format of a reading or writing and its options object (docs/FORMATS-PLAN.md, step 4).
    fn format_options(&mut self, mapping: &clarity_spec::code::MappingUse, pre: &mut String) -> (String, String) {
        let project = self.project;
        match crate::shape::mapping_pieces(project, mapping, |v| format!("clarity.JSONOf({})", bare(self.value(v, pre)))) {
            Some((format, pieces)) => (format, go_pieces(&pieces)),
            None => {
                self.errors.push("a mapping the project does not declare".into());
                (String::new(), go_pieces(&[]))
            }
        }
    }

    fn value(&mut self, expr: &Expr, pre: &mut String) -> String {
        // `app.version`: the project's version, written as the literal text it is.
        if clarity_spec::code::reads_app_version(expr) {
            let literal = Expr { kind: ExprKind::Str { value: self.project.version.clone().unwrap_or_default() }, span: expr.span };
            return self.value(&literal, pre);
        }
        let ty = self.types.type_of(expr);
        match &expr.kind {
            // A list of records (DESIGN 5.12): a copy of each, as code holds records by pointer.
            ExprKind::ListOf { items } if matches!(ty, TypeInfo::List(_)) => {
                let items: Vec<String> = items
                    .iter()
                    .map(|i| {
                        let record = bare(self.value(i, pre));
                        match record.strip_prefix('&') {
                            Some(place) => place.to_string(),
                            None => format!("*{record}"),
                        }
                    })
                    .collect();
                format!("{}{{{}}}", value_type(&ty), items.join(", "))
            }
            ExprKind::ListOf { items } => {
                let decimals = matches!(&ty, TypeInfo::Values(item) if is_decimal(&item.info()));
                let items: Vec<String> = items.iter().map(|i| bare(if decimals { self.decimal(i, pre) } else { self.value(i, pre) })).collect();
                format!("{}{{{}}}", value_type(&ty), items.join(", "))
            }
            // A record of a list by its position from 1: the record in the list itself, as records are pointers;
            // in parentheses, so a field of it (`items[1].done = true`) is the record's field.
            ExprKind::Index { list, index } if matches!(self.types.type_of(list), TypeInfo::List(_)) => {
                let list = self.list_ref(list, pre);
                let at = bare(self.value(index, pre));
                let at = self.held_operand(at, pre);
                pre.push_str(&format!("if _, err := clarity.At({list}, {at}); err != nil {{\n\t{}\n}}\n", self.fail("err")));
                format!("(&{list}[{at}-1])")
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
            ExprKind::Ask { question, form } => self.ask_value(question, form, pre),
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
            // A list of values, or of records, grows by a list of its items (DESIGN 5.26, 5.12).
            ExprKind::Binary { op: BinaryOp::Add, left, right } if matches!(ty, TypeInfo::Values(_) | TypeInfo::List(_)) => {
                let (left, right) = (bare(self.value(left, pre)), bare(self.value(right, pre)));
                // A copy first, so the list it grows from stays as it was.
                let copy = if left.starts_with("slices.Clone(") { left } else { format!("slices.Clone({left})") };
                format!("append({copy}, {right}...)")
            }
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right, &ty, pre),
            // `1 to n`: the whole numbers from one to the other.
            ExprKind::Range { from, to } => {
                let (from, to) = (bare(self.value(from, pre)), bare(self.value(to, pre)));
                format!("clarity.Range({from}, {to})")
            }
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
            ExprKind::Call { function, args, .. } => self.call(&function.name, args, pre),
            ExprKind::Find { entity, filter, order, var } => {
                let query = self.find(&var.name, &entity.name, filter.as_deref(), order, pre);
                let found = self.lift(query, pre);
                // Each record found is checked against its entity's `readable when`.
                let fail = self.fail("err");
                pre.push_str(&readable_list_check(self.project, &entity.name, &found, &fail));
                found
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
            // A format of the catalog (docs/FORMATS-PLAN.md): a value written in it, and records read from it, through Clarity.Formats.
            ExprKind::FormatOf { value, mapping } => {
                let Some(kind) = crate::shape::write_kind(self.project, &self.types.type_of(value)) else {
                    self.errors.push(format!("'{}' is not generated yet", print::expr(expr)));
                    return String::new();
                };
                let (format, options) = self.format_options(mapping, pre);
                format!("clarity.WriteFormat(formatsWrite, {}, {}, {options}, {})", string(&format), bare(self.value(value, pre)), string(&kind))
            }
            ExprKind::ReadFormat { ty, input, mapping } => {
                let Some(kind) = crate::shape::read_kind(self.project, ty) else {
                    self.errors.push(format!("'{}' is not generated yet", print::expr(expr)));
                    return String::new();
                };
                let read = match &ty.kind {
                    // One record is held by pointer, a list holds its records, as the generator writes them everywhere.
                    clarity_spec::code::TypeRefKind::List { item } => format!("[]{item}"),
                    clarity_spec::code::TypeRefKind::Named { name } => format!("*{name}"),
                    _ => String::new(),
                };
                let input = bare(self.value(input, pre));
                let (format, options) = self.format_options(mapping, pre);
                self.lift(format!("clarity.ReadFormat[{read}](formatsRead, {}, {input}, {options}, {})", string(&format), go_pieces(&kind)), pre)
            }
            // A call to a web API (DESIGN 5.23): it never fails, its reply says how it went.
            ExprKind::Http { method, url, from, pages, body, headers, query } => {
                let url = bare(self.value(url, pre));
                let body = match body {
                    Some(body) => bare(self.value(body, pre)),
                    None => "\"\"".to_string(),
                };
                let headers = self.http_pairs(headers, pre);
                if from.is_none() && query.is_empty() {
                    return format!("clarity.HTTP({}, {url}, {body}, {headers})", string(method.verb()));
                }
                // A path of a web API data source (DESIGN 5.23): its address and auth come from the project file.
                let query = self.http_pairs(query, pre);
                let (base, auth) = from.as_ref().and_then(|f| crate::web_source(self.project, &f.name)).unwrap_or(("", None));
                let auth = match auth {
                    Some((kind, secret)) => format!("clarity.HTTPAuth{{Kind: {}, Secret: {}}}", string(kind), string(secret)),
                    None => "clarity.HTTPAuth{}".to_string(),
                };
                // Every page, as the source's paging says (DESIGN 5.23).
                if let Some(p) = from.as_ref().filter(|_| *pages).and_then(|f| crate::web_paging(self.project, &f.name)) {
                    return format!(
                        "clarity.HTTPPages({}, {url}, {query}, {headers}, {auth}, clarity.HTTPPaging{{Kind: {}, Param: {}, SizeParam: {}, Size: {}, Next: {}, Items: {}}})",
                        string(base),
                        string(crate::paging_word(p.kind)),
                        string(&p.param),
                        string(&p.size_param),
                        p.size,
                        string(&p.next),
                        string(&p.items)
                    );
                }
                format!("clarity.HTTPAt({}, {}, {url}, {query}, {body}, {headers}, {auth})", string(method.verb()), string(base))
            }
            // A function of a library, through its package (DESIGN 5.21).
            ExprKind::EngineCall { engine, function, args } if crate::libraries::library(self.project, &engine.name).is_some() => {
                let values: Vec<String> = args.iter().map(|a| bare(self.value(a, pre))).collect();
                format!("{}lib.{}({})", crate::libraries::go_package(&engine.name), pascal(&function.name), values.join(", "))
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
        // Whole numbers divided, a division by zero an error.
        if matches!(op, BinaryOp::WholeDiv | BinaryOp::Mod) {
            let name = if op == BinaryOp::WholeDiv { "WholeDiv" } else { "WholeMod" };
            let l = self.value(left, pre);
            let r = self.value(right, pre);
            return self.lift(format!("clarity.{name}({l}, {r})"), pre);
        }
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
                let r = call_arg(if is_decimal(&item.info()) { self.decimal(right, pre) } else { self.value(right, pre) });
                format!("clarity.ListHolds({l}, {r})")
            }
            BinaryOp::Contains => {
                let l = call_arg(self.value(left, pre));
                let r = call_arg(self.value(right, pre));
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
                let l = call_arg(self.value(left, pre));
                let r = call_arg(self.value(right, pre));
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

    fn call_code(&mut self, name: &str, args: &[Expr], pre: &mut String) -> String {
        let Some(callee) = self.project.files.iter().flat_map(|f| &f.functions).find(|f| f.name.name == name) else {
            self.errors.push(format!("unknown function '{name}'"));
            return String::new();
        };
        let mut code = vec!["db".to_string()];
        for (arg, param) in args.iter().zip(&callee.params) {
            let ty = type_ref_info(self.project, &param.ty);
            let value = if is_decimal(&ty) { self.decimal(arg, pre) } else { self.value(arg, pre) };
            code.push(call_arg(value));
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
        if name == clarity_spec::code::check::SNAPPED && self.project.files.iter().flat_map(|f| &f.functions).all(|f| f.name.name != name) {
            let values: Vec<String> = args.iter().map(|a| call_arg(self.value(a, pre))).collect();
            return format!("clarity.Snapped({})", values.join(", "));
        }
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
        // newToken() and newRecoveryCodes(count) (docs/APP-CAPABILITIES-PLAN.md step 4), through Clarity.Auth.
        let project = self.project;
        let own = |n: &str| project.files.iter().flat_map(|f| &f.functions).any(|f| f.name.name == n);
        if name == clarity_spec::code::check::NEW_TOKEN && !own(name) {
            return self.lift("clarity.NewToken(authCall)".to_string(), pre);
        }
        if name == clarity_spec::code::check::NEW_RECOVERY_CODES && !own(name) {
            let count = args.first().map(|a| call_arg(self.value(a, pre))).unwrap_or_default();
            return self.lift(format!("clarity.NewRecoveryCodes(authCall, {count})"), pre);
        }
        if name == clarity_spec::code::check::ROUND {
            let values: Vec<String> = args.iter().map(|a| call_arg(self.value(a, pre))).collect();
            return format!("clarity.Round({})", values.join(", "));
        }
        // A picture function (docs/IMAGE-PLAN.md), in the program's pictures.go: a
        // record's picture read first, the call able to refuse.
        if let Some(built) = crate::built_in(self.project, name)
            && let Some((takes, _)) = clarity_spec::code::check::picture_function(name)
        {
            use clarity_spec::code::check::BuiltInValue;
            let values: Vec<String> = args.iter().zip(takes).map(|(a, t)| match t {
                BuiltInValue::Picture => self.picture_in_hand(a, pre),
                BuiltInValue::Number if matches!(self.types.type_of(a), TypeInfo::Decimal | TypeInfo::Money) => format!("{}.InexactFloat64()", bare(self.value(a, pre))),
                BuiltInValue::Number => format!("float64({})", bare(self.value(a, pre))),
                _ => bare(self.value(a, pre)),
            }).collect();
            return self.lift(format!("{}({})", built.go, values.join(", ")), pre);
        }
        // A built-in function (conformance/texts.json, conformance/zip.json), by the runtime's name for it.
        if let Some(built) = crate::built_in(self.project, name) {
            let values: Vec<String> = args.iter().map(|a| bare(self.value(a, pre))).collect();
            return format!("{}({})", built.go, values.join(", "));
        }
        if name == clarity_spec::code::check::LENGTH && self.project.files.iter().flat_map(|f| &f.functions).all(|f| f.name.name != name) {
            let text = args.first().map(|a| bare(self.value(a, pre))).unwrap_or_default();
            return format!("int64(len([]rune({text})))");
        }
        if name == clarity_spec::code::check::JOIN && self.project.files.iter().flat_map(|f| &f.functions).all(|f| f.name.name != name) {
            let list = args.first().map(|a| call_arg(self.value(a, pre))).unwrap_or_default();
            let separator = args.get(1).map(|a| call_arg(self.value(a, pre))).unwrap_or_default();
            return format!("clarity.Join({list}, {separator})");
        }
        if name == "count" {
            let list = args.first().map(|a| self.list_ref(a, pre)).unwrap_or_default();
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
            let numeric = crate::query_field_type(self.project, &path.entity, &path.field).is_some_and(|t| is_stored_number(&t));
            let column = string(&crate::query_column(self.project, &path.entity, &path.field));
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
            ExprKind::Binary { op: BinaryOp::Matches, .. } => {
                let Some((search, text)) = crate::query::search(project, entity, var, expr) else {
                    self.errors.push(format!("'{}' looks for words in no record of {var}", print::expr(expr)));
                    return String::new();
                };
                let columns: Vec<String> = search.columns.iter().map(|c| string(c)).collect();
                format!(
                    "clarity.Matches({}, {}, []string{{{}}}, {})",
                    string(&search.table),
                    string(&search.key),
                    columns.join(", "),
                    bare(self.value(text, pre))
                )
            }
            ExprKind::Binary { op, left, right } => {
                let (path, value, op) = match (path_of(left), path_of(right)) {
                    (Some(path), _) => (path, right, *op),
                    (None, Some(path)) => (path, left, flipped(*op)),
                    (None, None) => {
                        self.errors.push(format!("'{}' compares no field of {var}", print::expr(expr)));
                        return String::new();
                    }
                };
                let Some(ty) = crate::query_field_type(self.project, &path.entity, &path.field) else {
                    self.errors.push(format!("{} has no field '{}'", path.entity, path.field));
                    return String::new();
                };
                let name = string(&crate::query_column(self.project, &path.entity, &path.field));
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
                let Some(ty) = crate::query_field_type(self.project, &path.entity, &path.field) else {
                    self.errors.push(format!("{} has no field '{}'", path.entity, path.field));
                    return String::new();
                };
                let constructor = if is_stored_number(&ty) { "OneOfNumbers" } else { "OneOf" };
                let values: Vec<String> = options.iter().map(|o| self.sql_value(&ty, o, pre)).collect();
                let own =
                    format!("clarity.{constructor}({}, []any{{{}}})", string(&crate::query_column(self.project, &path.entity, &path.field)), values.join(", "));
                through(&path.hops, own)
            }
            _ => match path_of(expr) {
                Some(path) => through(&path.hops, format!("clarity.IsTrue({})", string(&crate::query_column(self.project, &path.entity, &path.field)))),
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
            FieldType::List { .. } | FieldType::Record { .. } => {
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


/// A kind's or options' pieces (crate::shape) as a Go expression of a
/// string: a base is a new record as encoding/json writes it.
fn go_pieces(pieces: &[crate::shape::Piece]) -> String {
    use crate::shape::Piece;
    if pieces.is_empty() {
        return "\"\"".to_string();
    }
    pieces
        .iter()
        .map(|p| match p {
            Piece::Text(t) => string(t),
            Piece::Base(entity) => format!("clarity.JSONOf(New{entity}())"),
            Piece::Code(code) => code.clone(),
        })
        .collect::<Vec<_>>()
        // Always a call's argument, where gofmt writes `+` without spaces.
        .join("+")
}
