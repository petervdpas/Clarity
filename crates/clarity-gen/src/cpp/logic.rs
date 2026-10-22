//! Functions, actions, data events, windows and their handlers as C++.
//! What can fail throws, as C++ code is written, so expressions stay
//! expressions: a query, a call or loading the record a reference points at
//! is written where it is used. Records are changed in place: an action
//! takes its record by reference, and a loop works on the records of its
//! list. Types come from the checker's table; the emitter only chooses how
//! to write them in C++.

use std::collections::HashMap;
use std::fmt::Write as _;

use clarity_spec::Project;
use clarity_spec::code::check::{FileTypes, NameKind, TypeInfo};
use clarity_spec::code::{
    AskForm, BinaryOp, EngineBody, EngineDecl, EngineState, Expr, ExprKind, FunctionDecl, MessageLevel, OrderKey, Stmt, StmtKind, TextPart, TypeRef,
    UnaryOp, path_text, template,
};
use clarity_spec::model::{Command, DataKind, FieldType, Frame, SpecFile, Widget};

use super::model::constant;
use super::names::{decimal, ident, param_type, snake, string, value_type};
use crate::print;

pub(crate) use crate::type_ref_info;

fn is_decimal(ty: &TypeInfo) -> bool {
    matches!(ty, TypeInfo::Decimal | TypeInfo::Money)
}

/// Decimals and money: stored as text in SQLite, compared and ordered as numbers.
fn is_stored_number(ty: &FieldType) -> bool {
    matches!(ty, FieldType::Decimal { .. } | FieldType::Money)
}

fn flipped(op: BinaryOp) -> BinaryOp {
    match op {
        BinaryOp::Lt => BinaryOp::Gt,
        BinaryOp::LtEq => BinaryOp::GtEq,
        BinaryOp::Gt => BinaryOp::Lt,
        BinaryOp::GtEq => BinaryOp::LtEq,
        other => other,
    }
}

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

/// `lines` indented by `depth` steps of four spaces.
fn indent(lines: &str, depth: usize) -> String {
    let pad = "    ".repeat(depth);
    lines.lines().map(|l| if l.is_empty() { "\n".to_string() } else { format!("{pad}{l}\n") }).collect()
}

/// The declaration of a function, action or data event, for the header.
pub fn signature(project: &Project, decl: &FunctionDecl) -> String {
    let mut params = vec!["clarity::Db& db".to_string()];
    for param in &decl.params {
        params.push(format!("{} {}", param_type(&type_ref_info(project, &param.ty)), ident(&param.name.name)));
    }
    let ret = decl.returns.as_ref().map(|r| value_type(&type_ref_info(project, r))).unwrap_or_else(|| "void".into());
    format!("{ret} {}({})", ident(&decl.name.name), params.join(", "))
}

/// The C++ struct of what an engine written in Clarity keeps: `BoardKept`.
fn kept_struct(engine: &str) -> String {
    format!("{engine}Kept")
}

/// The declaration of a function of an engine written in Clarity, for the
/// header: `std::string board_play(clarity::Db& db, uint64_t instance, int64_t square)`.
pub fn engine_signature(project: &Project, engine: &EngineDecl, decl: &FunctionDecl) -> String {
    let mut params = vec!["clarity::Db& db".to_string(), "uint64_t instance".to_string()];
    for param in &decl.params {
        params.push(format!("{} {}", param_type(&type_ref_info(project, &param.ty)), ident(&param.name.name)));
    }
    let ret = decl.returns.as_ref().map(|r| value_type(&type_ref_info(project, r))).unwrap_or_else(|| "void".into());
    format!("{ret} {}_{}({})", snake(&engine.name.name), snake(&decl.name.name), params.join(", "))
}

/// The inner function that works on the kept values.
fn inner_signature(project: &Project, engine: &EngineDecl, decl: &FunctionDecl) -> String {
    let mut params = vec!["[[maybe_unused]] clarity::Db& db".to_string(), format!("[[maybe_unused]] {}& kept", kept_struct(&engine.name.name))];
    for param in &decl.params {
        params.push(format!("{} {}", param_type(&type_ref_info(project, &param.ty)), ident(&param.name.name)));
    }
    let ret = decl.returns.as_ref().map(|r| value_type(&type_ref_info(project, r))).unwrap_or_else(|| "void".into());
    format!("{ret} {}_{}_with({})", snake(&engine.name.name), snake(&decl.name.name), params.join(", "))
}

/// The header lines of the engines written in Clarity in `file`.
pub fn engine_declarations(project: &Project, file: &SpecFile) -> String {
    let mut out = String::new();
    for engine in &file.engines {
        let Some(body) = &engine.body else { continue };
        for decl in &body.functions {
            let _ = writeln!(out, "{};", engine_signature(project, engine, decl));
        }
        if engine.state != EngineState::None {
            let _ = writeln!(out, "void {}_close(uint64_t instance);", snake(&engine.name.name));
        }
    }
    out
}

/// An engine written in Clarity: a struct of what it keeps, the kept values
/// of each instance (a window, or 0 for the app), and per function an entry
/// that finds the instance's values and an inner one that works on them. A
/// call works on a copy that is kept only when the call succeeds, so a call
/// that fails or refuses changes nothing.
fn emit_engine(project: &Project, types: &FileTypes, engine: &EngineDecl, body: &EngineBody) -> Result<String, Vec<String>> {
    let name = &engine.name.name;
    let snake_name = snake(name);
    let kept = kept_struct(name);
    let mut errors = Vec::new();
    let mut fields = String::new();
    let mut starts = String::new();
    for keep in &body.keeps {
        let info = type_ref_info(project, &keep.ty);
        let field = ident(&keep.name.name);
        let _ = writeln!(fields, "    {} {field};", value_type(&info));
        let mut emitter = Emitter::new(project, types, None, None);
        emitter.engine = Some(engine);
        let value = emitter.value(&keep.value);
        if emitter.errors.is_empty() {
            let _ = writeln!(starts, "    kept.{field} = {value};");
        } else {
            errors.extend(emitter.errors.into_iter().map(|e| format!("engine {name} keeps {}: {e}", keep.name.name)));
        }
    }
    let stored = engine.state != EngineState::None;
    let mut inner = String::new();
    let mut prototypes = String::new();
    let mut entries = String::new();
    for decl in &body.functions {
        let mut emitter = Emitter::new(project, types, Some(decl), None);
        emitter.engine = Some(engine);
        let code = emitter.function();
        if !emitter.errors.is_empty() {
            errors.extend(emitter.errors.into_iter().map(|e| format!("function '{}' of engine {name}: {e}", decl.name.name)));
            continue;
        }
        let _ = writeln!(prototypes, "{};", inner_signature(project, engine, decl));
        inner.push_str(&code);
        let function = format!("{snake_name}_{}", snake(&decl.name.name));
        let names: String = decl.params.iter().map(|p| format!(", {}", ident(&p.name.name))).collect();
        let void = decl.returns.is_none();
        let run = if stored {
            let (answer, give) = if void { ("", "") } else { ("auto answer = ", "    return answer;\n") };
            format!(
                "    auto& store = {snake_name}_store();\n    std::lock_guard<std::mutex> held(store.lock);\n    auto found = store.all.find(instance);\n    {kept} kept = found != store.all.end() ? found->second : {snake_name}_new(db);\n    {answer}{function}_with(db, kept{names});\n    store.all.insert_or_assign(instance, kept);\n{give}"
            )
        } else {
            let give = if void { "" } else { "return " };
            format!("    {kept} kept = {snake_name}_new(db);\n    {give}{function}_with(db, kept{names});\n")
        };
        let _ = write!(
            entries,
            "/// {name}.{} in code, for instance `instance` (a window, or 0 for the app).\n{} {{\n{run}}}\n\n",
            decl.name.name,
            engine_signature(project, engine, decl).replacen("uint64_t instance", "[[maybe_unused]] uint64_t instance", 1)
        );
    }
    let store = if stored {
        format!(
            "/// The values each instance (a window, or 0 for the app) keeps; a closed window's go.\nstruct {kept}Store {{\n    std::mutex lock;\n    std::map<uint64_t, {kept}> all;\n    std::shared_ptr<void> closing = clarity::on_window_closed([this](uint64_t instance) {{\n        std::lock_guard<std::mutex> held(lock);\n        all.erase(instance);\n    }});\n}};\n\n{kept}Store& {snake_name}_store() {{\n    static {kept}Store store;\n    return store;\n}}\n\n"
        )
    } else {
        String::new()
    };
    let close = if stored {
        format!("void {snake_name}_close(uint64_t instance) {{\n    auto& store = {snake_name}_store();\n    std::lock_guard<std::mutex> held(store.lock);\n    store.all.erase(instance);\n}}\n\n")
    } else {
        String::new()
    };
    let out = format!(
        "namespace {{\n\n/// What engine {name} keeps between calls.\nstruct {kept} {{\n{fields}}};\n\n/// What a new instance starts with.\n{kept} {snake_name}_new([[maybe_unused]] clarity::Db& db) {{\n    {kept} kept;\n{starts}    return kept;\n}}\n\n{store}{prototypes}\n{inner}}}  // namespace\n\n{close}{entries}"
    );
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// The data events written in `file`, as actions on their record.
pub fn events(file: &SpecFile) -> Vec<FunctionDecl> {
    file.code
        .iter()
        .flat_map(|b| b.events.iter().map(|e| e.action(&b.name.name)))
        .collect()
}

/// The definitions of `file`'s functions, actions and data events, and the
/// record methods that run the events.
pub fn emit(project: &Project, file: &SpecFile, types: &FileTypes) -> Result<String, Vec<String>> {
    let mut out = String::new();
    let mut errors = Vec::new();
    let events = events(file);
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
    // Each data event runs from its record's method.
    for block in &file.code {
        for event in &block.events {
            let record = clarity_spec::code::record_name(&block.name.name);
            let _ = write!(
                out,
                "void {}::{}(clarity::Db& db) {{\n    {}(db, *this);\n}}\n\n",
                block.name.name,
                event.event.word(),
                ident(&format!("{}_{record}", event.event.word()))
            );
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
    /// The types of the values each handler takes, by handler.
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

/// A window or the app, as main.cpp dispatches to it.
#[derive(Debug, Clone)]
pub struct Owner {
    pub name: String,
    /// The struct holding its data: `CustomerFormWindow`, `ShopApp`.
    pub struct_name: String,
    /// The handlers called by name (those taking no values), with their member function names.
    pub handlers: Vec<(String, String)>,
    /// The calls with values its parts wire: as written (`digit(7)`), the
    /// member function, and the values as C++.
    pub calls: Vec<(String, String, Vec<String>)>,
    pub is_app: bool,
    /// Its own events (open, close, return, start) with their handlers.
    pub events: Vec<(&'static str, String)>,
    /// The canvases its draw handlers draw, with their member functions.
    pub draws: Vec<(String, String)>,
    /// The handlers an answer runs (DESIGN 5.14): the Clarity name, the
    /// member function, and each value's type.
    pub answers: Vec<(String, String, Vec<TypeInfo>)>,
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
        // Each call with values the parts wire, with its values as C++.
        let calls: Vec<(String, String, Vec<String>)> = page
            .map(crate::wired_calls)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|h| {
                let decl = written.iter().find(|d| d.name.name == h.handler)?;
                let values = h.args.iter().zip(&decl.params).map(|(v, p)| literal(project, &p.ty, v)).collect();
                Some((clarity_spec::widgets::handler_call(h), method(&h.handler), values))
            })
            .collect();

        let mut fields = String::new();
        let mut load = String::new();
        let mut to = String::new();
        let mut from = String::new();
        for d in data {
            let (member, key) = (ident(&d.name.name), string(&d.name.name));
            let ty = match &d.of {
                DataKind::Record { entity } => entity.name.clone(),
                DataKind::List { entity } => format!("std::vector<{}>", entity.name),
                DataKind::Value { ty, .. } => value_type(&type_ref_info(project, ty)),
            };
            match &d.of {
                // A plain value starts at its default, or empty.
                DataKind::Value { ty: value_ty, default: Some(value) } => {
                    let _ = writeln!(fields, "    {ty} {member} = {};", literal(project, value_ty, value));
                }
                DataKind::Value { default: None, .. } => {
                    let _ = writeln!(fields, "    {ty} {member}{{}};");
                }
                _ => {
                    let _ = writeln!(fields, "    {ty} {member};");
                }
            }
            if let DataKind::List { entity } = &d.of {
                let _ = writeln!(load, "    w.{member} = db.all<{}>();", entity.name);
            }
            let _ = writeln!(to, "    out[{key}] = w.{member};");
            let _ = writeln!(from, "    if (in.contains({key}) && !in[{key}].is_null()) w.{member} = in[{key}].get<{ty}>();");
        }

        let mut methods = String::new();
        let mut handlers = Vec::new();
        for handler in file.code.iter().filter(|b| b.name.name == *name).flat_map(|b| &b.handlers) {
            let method = method(&handler.name.name);
            let mut emitter = Emitter::new(project, types, None, Some(&ctx));
            let mut code = String::new();
            emitter.block(&handler.body, &mut code);
            // The values a click gives it, typed.
            let values: String = handler
                .params
                .iter()
                .map(|p| format!(", {} {}", value_type(&type_ref_info(project, &p.ty)), ident(&p.name.name)))
                .collect();
            if emitter.errors.is_empty() {
                let _ = writeln!(methods, "    void {method}(clarity::Db& db, clarity::Ui& ui{values});");
                let _ = write!(source, "void {struct_name}::{method}(clarity::Db& db, clarity::Ui& ui{values}) {{\n{}}}\n\n", indent(&code, 1));
                // One that takes values is called with them, through the calls.
                if handler.params.is_empty() {
                    handlers.push((handler.name.name.clone(), method));
                }
            } else {
                errors.extend(emitter.errors.into_iter().map(|e| format!("handler '{}' of {name}: {e}", handler.name.name)));
            }
        }
        let what = if app.is_some() { "the app" } else { name.as_str() };
        let _ = write!(
            header,
            "/// The data of {what}, as its handlers see it.\nstruct {struct_name} {{\n{fields}\n    /// The data when it opens: every record of a list, a new record otherwise, each value at its default.\n    static {struct_name} load(clarity::Db& db);\n{methods}}};\nvoid to_json(nlohmann::json& out, const {struct_name}& w);\nvoid from_json(const nlohmann::json& in, {struct_name}& w);\n\n"
        );
        let _ = write!(
            source,
            "{struct_name} {struct_name}::load([[maybe_unused]] clarity::Db& db) {{\n    {struct_name} w;\n{load}    return w;\n}}\n\nvoid to_json(nlohmann::json& out, [[maybe_unused]] const {struct_name}& w) {{\n    out = nlohmann::json::object();\n{to}}}\n\nvoid from_json([[maybe_unused]] const nlohmann::json& in, [[maybe_unused]] {struct_name}& w) {{\n{from}}}\n\n"
        );
        let draws = page.map(crate::canvas_draws).unwrap_or_default().into_iter().map(|(canvas, handler)| (canvas, method(&handler))).collect();
        let written: Vec<&clarity_spec::code::HandlerDecl> = file.code.iter().filter(|b| b.name.name == *name).flat_map(|b| &b.handlers).collect();
        let answers = crate::answering_handlers(&written)
            .into_iter()
            .map(|h| (h.name.name.clone(), method(&h.name.name), h.params.iter().map(|p| type_ref_info(project, &p.ty)).collect()))
            .collect();
        owners.push(Owner { name: name.clone(), struct_name, handlers, calls, is_app: app.is_some(), events, draws, answers });
    }
    if errors.is_empty() { Ok((header, source, owners)) } else { Err(errors) }
}

/// A value written in the design as C++, of the parameter's type.
fn literal(project: &Project, ty: &TypeRef, value: &clarity_spec::model::Literal) -> String {
    use clarity_spec::model::Literal;
    match (type_ref_info(project, ty), value) {
        (TypeInfo::Decimal | TypeInfo::Money, Literal::Number(n)) => decimal(n),
        (_, Literal::Number(n)) => n.clone(),
        // A key starts in the language the screen picked.
        (_, Literal::Str(text)) if text.starts_with('@') => format!("clarity::text_of({})", string(&text[1..])),
        (_, Literal::Str(text)) => string(text),
        (_, Literal::Bool(b)) => b.to_string(),
        (TypeInfo::Enum(e), Literal::Ident(v)) => constant(&e, v),
        (_, Literal::Ident(v)) => v.clone(),
    }
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
}

impl<'a> Emitter<'a> {
    fn new(project: &'a Project, types: &'a FileTypes, decl: Option<&'a FunctionDecl>, window: Option<&'a WindowCtx>) -> Self {
        Emitter { project, types, decl, window, engine: None, errors: Vec::new() }
    }

    fn returns(&self) -> Option<TypeInfo> {
        self.decl.and_then(|d| d.returns.as_ref()).map(|r| type_ref_info(self.project, r))
    }

    fn function(&mut self) -> String {
        let decl = self.decl.expect("a function, action or data event");
        let mut body = String::new();
        for condition in &decl.requires {
            let holds = self.value(condition);
            let _ = writeln!(body, "clarity::require({holds}, {});", string(&print::expr(condition)));
        }
        if decl.ensures.is_empty() {
            self.block(&decl.body, &mut body);
        } else {
            // The body runs as a function of its own, so every return gives
            // the result the ensures check.
            let mut inner = String::new();
            self.block(&decl.body, &mut inner);
            match self.returns() {
                Some(ret) => {
                    let _ = write!(body, "auto result = [&]() -> {} {{\n{}}}();\n", value_type(&ret), indent(&inner, 1));
                }
                None => {
                    let _ = write!(body, "[&]() {{\n{}}}();\n", indent(&inner, 1));
                }
            }
            for condition in &decl.ensures {
                let holds = self.value(condition);
                let _ = writeln!(body, "clarity::require({holds}, {});", string(&print::expr(condition)));
            }
            if self.returns().is_some() {
                body.push_str("return result;\n");
            }
        }
        let signature = match self.engine {
            Some(engine) => inner_signature(self.project, engine, decl),
            None => signature(self.project, decl),
        };
        format!("{signature} {{\n{}}}\n\n", indent(&body, 1))
    }

    fn block(&mut self, body: &[Stmt], out: &mut String) {
        for stmt in body {
            self.stmt(stmt, out);
        }
    }

    fn stmt(&mut self, stmt: &Stmt, out: &mut String) {
        match &stmt.kind {
            StmtKind::Let { name, ty, value } => {
                let target = ident(&name.name);
                match ty.as_ref().map(|t| type_ref_info(self.project, t)) {
                    Some(declared) => {
                        let code = self.value(value);
                        let _ = writeln!(out, "{} {target} = {code};", value_type(&declared));
                    }
                    None => {
                        let code = self.value(value);
                        let _ = writeln!(out, "auto {target} = {code};");
                    }
                }
            }
            StmtKind::Assign { target, value } => {
                if let Some((part, property)) = self.part_property(target) {
                    let method = match property.as_str() {
                        "caption" | "error" | "visible" | "enabled" | "readonly" | "text" | "checked" | "folded" | "collapsed" | "maximized" | "tab" => format!("set_{property}"),
                        other => {
                            self.errors.push(format!("'{other}' of a part cannot be set"));
                            return;
                        }
                    };
                    // A caption or error is kept as written with its slots, and shown in the person's format.
                    let code = if matches!(property.as_str(), "caption" | "error") { self.shown(value) } else { self.value(value) };
                    let _ = writeln!(out, "ui.{method}({}, {code});", string(&part));
                    return;
                }
                // A record given to a reference field is stored as its id.
                if self.is_reference_field(target) {
                    let stored = self.stored_reference(target);
                    let record = if matches!(value.kind, ExprKind::Empty) { "std::nullopt".to_string() } else { format!("{}.id", self.value(value)) };
                    let _ = writeln!(out, "{stored} = {record};");
                    return;
                }
                // A file as a picture: linked in the field's own store, else copied.
                if self.types.type_of(value) == TypeInfo::File
                    && let ExprKind::Member { object, member } = &target.kind
                    && let TypeInfo::Record(owner) = self.types.type_of(object)
                {
                    let store = self.project.entity(&owner).and_then(|e| e.field(&member.name)).and_then(|f| f.modifiers.store.clone());
                    let store = store.map_or("std::nullopt".to_string(), |s| string(&s));
                    let (place, file) = (self.place(target), self.value(value));
                    let _ = writeln!(out, "{place} = db.picture_from({file}, {store});");
                    return;
                }
                let (place, code) = (self.place(target), self.value(value));
                let _ = writeln!(out, "{place} = {code};");
                if let ExprKind::Name { name } = &target.kind {
                    out.push_str(&self.deselect(name));
                    // A text value shows its text in the person's language, as a label does.
                    if matches!(self.types.name_of(target), Some(NameKind::Data)) && self.types.type_of(target) == TypeInfo::Text {
                        let shown = match value.kind {
                            ExprKind::Str { .. } | ExprKind::Text { .. } | ExprKind::Key { .. } => self.shown(value),
                            _ => format!("clarity::Text::plain({place})"),
                        };
                        let _ = writeln!(out, "ui.set_shown({}, {shown});", string(name));
                    }
                }
            }
            StmtKind::If { branches, otherwise } => {
                for (i, branch) in branches.iter().enumerate() {
                    let condition = self.value(&branch.condition);
                    let mut body = String::new();
                    self.block(&branch.body, &mut body);
                    let keyword = if i == 0 { "if" } else { " else if" };
                    let _ = write!(out, "{keyword} ({condition}) {{\n{}}}", indent(&body, 1));
                }
                if let Some(other) = otherwise {
                    let mut body = String::new();
                    self.block(other, &mut body);
                    let _ = write!(out, " else {{\n{}}}", indent(&body, 1));
                }
                out.push('\n');
            }
            StmtKind::For { var, list, body } if matches!(self.types.type_of(list), TypeInfo::Values(_)) => {
                // Plain values one by one, from a copy, so the body may change the list.
                let ty = value_type(&self.types.type_of(list));
                let list = if matches!(list.kind, ExprKind::ListOf { .. }) { self.value(list) } else { format!("{ty}({})", self.value(list)) };
                let mut inner = String::new();
                self.block(body, &mut inner);
                let _ = write!(out, "for (auto {} : {list}) {{\n{}}}\n", ident(&var.name), indent(&inner, 1));
            }
            StmtKind::For { var, list, body } => {
                let list = self.value(list);
                let mut inner = String::new();
                self.block(body, &mut inner);
                let _ = write!(out, "for (auto& {} : {list}) {{\n{}}}\n", ident(&var.name), indent(&inner, 1));
            }
            StmtKind::Return { value: Some(value) } => {
                let code = self.value(value);
                let _ = writeln!(out, "return {code};");
            }
            StmtKind::Return { value: None } => out.push_str("return;\n"),
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
                        let value = if is_decimal(&ty) && !is_decimal(&self.types.type_of(arg)) { format!("clarity::Decimal({})", self.value(arg)) } else { self.value(arg) };
                        let _ = write!(values, ", {value}");
                    }
                    let _ = writeln!(out, "{}(db, ui{values});", method(&function.name));
                    return;
                }
                let code = self.value(call);
                let _ = writeln!(out, "{code};");
            }
            StmtKind::Save { value } | StmtKind::Delete { value } => {
                let verb = if matches!(stmt.kind, StmtKind::Save { .. }) { "save" } else { "remove" };
                let record = self.record_arg(value);
                let _ = writeln!(out, "db.{verb}({record});");
            }
            StmtKind::Refuse { text } => {
                let text = self.shown(text);
                let _ = writeln!(out, "clarity::refuse({text});");
            }
            StmtKind::Write { text, store, path } => {
                // Bytes are written as they are, text as UTF-8.
                let write = if self.types.type_of(text) == TypeInfo::Bytes { "write_bytes" } else { "write_text" };
                let (path, text) = (self.value(path), self.value(text));
                let _ = writeln!(out, "db.{write}({}, {path}, {text});", string(&store.name));
            }
            StmtKind::Message { .. } | StmtKind::Ask { .. } | StmtKind::Open { .. } | StmtKind::Close | StmtKind::Reload { .. } if self.window.is_none() => {
                self.errors.push("messages, windows and window data belong in handlers".into());
            }
            // A shape on a canvas, as conformance/drawing.json sends it.
            StmtKind::Draw { canvas, shape, args } => {
                use clarity_spec::code::check::ShapeValue;
                let mut fields = vec![format!("{{\"op\", {}}}", string(&shape.name))];
                for (key, kind, given) in crate::shape_values(&shape.name, args) {
                    let value = match (kind, given) {
                        (ShapeValue::Number, Ok(arg)) => match self.types.type_of(arg) {
                            TypeInfo::Decimal | TypeInfo::Money => format!("clarity::units({})", self.value(arg)),
                            _ => format!("double({})", self.value(arg)),
                        },
                        (ShapeValue::Number, Err(default)) => format!("{default}.0"),
                        (ShapeValue::Color, Ok(arg)) => self.value(arg),
                        (ShapeValue::Color | ShapeValue::Text, Err(default)) => string(default),
                        (ShapeValue::Text, Ok(arg)) => format!("nlohmann::json({})", self.shown(arg)),
                    };
                    fields.push(format!("{{{}, {value}}}", string(key)));
                }
                let _ = writeln!(out, "ui.draw({}, nlohmann::json{{{}}});", string(&canvas.name), fields.join(", "));
            }
            StmtKind::Message { level, text } => {
                let level = match level {
                    MessageLevel::Info => "Info",
                    MessageLevel::Warning => "Warning",
                    MessageLevel::Error => "Error",
                };
                let text = self.shown(text);
                let _ = writeln!(out, "ui.message(clarity::Level::{level}, {text});");
            }
            StmtKind::Ask { question, form, then, args } => {
                let question = self.shown(question);
                let form = match form.as_ref() {
                    AskForm::Confirm { yes, no } => {
                        let yes = yes.as_ref().map_or("std::nullopt".to_string(), |t| self.shown(t));
                        let no = no.as_ref().map_or("std::nullopt".to_string(), |t| self.shown(t));
                        format!("clarity::ask_confirm({yes}, {no})")
                    }
                    AskForm::Value { ty, default, from } => {
                        let (word, of) = crate::ask_type(self.project, ty);
                        let of = of.map_or("std::nullopt".to_string(), |n| format!("std::string({})", string(&n)));
                        let start = default.as_ref().map_or("nullptr".to_string(), |d| format!("nlohmann::json({})", self.value(d)));
                        // The records `from` offers, by their ids.
                        let only = from.as_ref().map_or("nullptr".to_string(), |list| {
                            format!("[&] {{ auto only = nlohmann::json::array(); for (const auto& r : {}) only.push_back(r.id); return only; }}()", self.value(list))
                        });
                        format!("clarity::ask_value({}, {of}, {only}, {start})", string(word))
                    }
                };
                let values: Vec<String> = args.iter().map(|a| format!("nlohmann::json({})", self.value(a))).collect();
                let values = if values.is_empty() { "nlohmann::json::array()".to_string() } else { format!("nlohmann::json::array({{{}}})", values.join(", ")) };
                let _ = writeln!(out, "ui.ask({question}, {form}, {}, {values});", string(&then.name));
            }
            StmtKind::Open { window, with } => {
                let args: Vec<String> = with.iter().map(|arg| format!("{{{}, {}}}", string(&arg.name.name), self.value(&arg.value))).collect();
                let _ = writeln!(out, "ui.open({}, nlohmann::json::object({{{}}}));", string(&window.name), args.join(", "));
            }
            StmtKind::Close => out.push_str("ui.close();\n"),
            StmtKind::Reload { data } => {
                let found = self.window.and_then(|w| w.data.iter().find(|(n, _, _)| *n == data.name)).cloned();
                match found {
                    Some((name, entity, true)) => {
                        let _ = writeln!(out, "{} = db.all<{entity}>();", ident(&name));
                        out.push_str(&self.deselect(&name));
                    }
                    Some((name, entity, false)) => {
                        let member = ident(&name);
                        let _ = writeln!(out, "if ({member}.id != 0) {member} = db.get<{entity}>({member}.id);");
                    }
                    None => self.errors.push(format!("there is no data '{}' to reload", data.name)),
                }
            }
        }
    }

    /// A record to save, delete or hand to an action, which changes it in
    /// place: the record itself where it is held, else a temporary.
    fn record_arg(&mut self, expr: &Expr) -> String {
        let held = match &expr.kind {
            ExprKind::Name { .. } => true,
            ExprKind::Member { .. } => self.part_property(expr).is_none() && !self.is_reference_field(expr),
            _ => false,
        };
        let code = self.value(expr);
        if held { code } else { format!("clarity::lvalue({code})") }
    }

    /// Text the screen shows as the runtime's `Text`: the text as written
    /// with a typed value per slot.
    fn shown(&mut self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Str { value } => format!("clarity::Text::plain({})", string(value)),
            // A key, whose value the screen looks up, with its slots by name.
            ExprKind::Key { key, args } => {
                let mut code = format!("clarity::Text::key({})", string(&key.name));
                for arg in args {
                    let slot = self.slot(&arg.value);
                    code.push_str(&format!(".with({}, {slot})", string(&arg.name.name)));
                }
                code
            }
            ExprKind::Text { parts } => {
                let mut code = format!("clarity::Text({})", string(&template(parts)));
                for part in parts {
                    if let TextPart::Slot { value } = part {
                        let slot = self.slot(value);
                        let _ = write!(code, ".with({}, {slot})", string(&path_text(value)));
                    }
                }
                code
            }
            _ => {
                let ty = self.types.type_of(expr);
                let value = self.value(expr);
                format!("clarity::Text::plain({})", show(value, &ty))
            }
        }
    }

    /// The value of a slot with what the screen needs to show it.
    fn slot(&mut self, expr: &Expr) -> String {
        let value = self.value(expr);
        let field = self.field_of(expr);
        let format = string(field.and_then(|f| f.modifiers.format.as_deref()).unwrap_or_default());
        let scale = match field.map(|f| &f.ty) {
            Some(FieldType::Decimal { scale, .. }) => scale.to_string(),
            _ => "-1".to_string(),
        };
        match self.types.type_of(expr) {
            TypeInfo::Text => format!("clarity::slot_text({value})"),
            TypeInfo::Integer => format!("clarity::slot_integer({value}, {format})"),
            TypeInfo::Duration => format!("clarity::slot_duration({value})"),
            TypeInfo::Decimal => format!("clarity::slot_decimal({value}, {scale}, {format})"),
            TypeInfo::Money => format!("clarity::slot_money({value}, {format})"),
            TypeInfo::Date => format!("clarity::slot_date({value}, {format})"),
            TypeInfo::DateTime => format!("clarity::slot_date_time({value}, {format})"),
            TypeInfo::Bool => format!("clarity::slot_bool({value})"),
            TypeInfo::Enum(name) => format!("clarity::slot_enum(name_of({value}), {})", string(&name)),
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
        parts.map(|part| format!("ui.deselect({});\n", string(part))).collect()
    }

    /// An expression as a place: what can be assigned or read in place.
    fn place(&mut self, expr: &Expr) -> String {
        if self.part_property(expr).is_some() {
            return self.read_part(expr);
        }
        match &expr.kind {
            ExprKind::Name { name } => match self.types.name_of(expr) {
                Some(NameKind::Local) | Some(NameKind::Data) => ident(name),
                Some(NameKind::Kept) => format!("kept.{}", ident(name)),
                Some(NameKind::EnumValue(e)) => constant(e, name),
                _ => {
                    self.errors.push(format!("'{name}' is not generated yet for C++"));
                    String::new()
                }
            },
            ExprKind::Member { object, member } => {
                if let Some(NameKind::EnumValue(e)) = self.types.name_of(expr) {
                    return constant(e, &member.name);
                }
                // A file's details have the same names in the runtime.
                if self.types.type_of(object) == TypeInfo::File {
                    return format!("{}.{}", self.place(object), member.name);
                }
                if self.types.type_of(object) == TypeInfo::Bytes {
                    return format!("static_cast<int64_t>({}.size())", self.place(object));
                }
                let TypeInfo::Record(entity) = self.types.type_of(object) else {
                    self.errors.push(format!("'{}' is not generated yet for C++", print::expr(expr)));
                    return String::new();
                };
                let object = self.place(object);
                match self.field_type(&entity, &member.name) {
                    // Every record has an id.
                    None if member.name == clarity_spec::code::check::RECORD_ID => format!("{object}.id"),
                    Some(FieldType::Ref { entity: target }) => format!("db.load<{}>({object}.{})", target.name, ident(&member.name)),
                    Some(FieldType::List { .. }) => {
                        self.errors.push(format!("list fields like '{}' are not loaded yet", print::expr(expr)));
                        String::new()
                    }
                    _ => format!("{object}.{}", ident(&member.name)),
                }
            }
            // `squares[3]`: counted from 1; a position outside the list fails.
            ExprKind::Index { list, index } => format!("clarity::at({}, {})", self.place(list), self.value(index)),
            _ => self.value(expr),
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
    fn read_part(&mut self, expr: &Expr) -> String {
        let (part, property) = self.part_property(expr).unwrap_or_default();
        let info = self.window.and_then(|w| w.parts.get(&part)).cloned().unwrap_or_default();
        let quoted = string(&part);
        match property.as_str() {
            "visible" | "enabled" | "error" | "text" | "checked" | "x" | "y" => format!("ui.{property}({quoted})"),
            "caption" => format!("ui.caption({quoted}, {})", string(info.caption.as_deref().unwrap_or_default())),
            "readonly" => format!("ui.readonly({quoted}, {})", info.readonly),
            "folded" => format!("ui.folded({quoted}, {})", info.folded),
            "collapsed" => format!("ui.collapsed({quoted}, {})", info.collapsed),
            "maximized" => format!("ui.maximized({quoted})"),
            "tab" => format!("ui.tab({quoted})"),
            "selected" => match info.bind {
                Some(bind) => format!("ui.selected({quoted}, {})", ident(&bind)),
                None => {
                    self.errors.push(format!("'{part}' is not bound to a list"));
                    String::new()
                }
            },
            other => {
                self.errors.push(format!("'{other}' of a part is not generated yet for C++"));
                String::new()
            }
        }
    }

    /// An expression as a value.
    fn value(&mut self, expr: &Expr) -> String {
        let ty = self.types.type_of(expr);
        match &expr.kind {
            ExprKind::ListOf { items } => {
                let items: Vec<String> = items.iter().map(|i| self.value(i)).collect();
                format!("{}{{{}}}", value_type(&ty), items.join(", "))
            }
            ExprKind::Index { .. } => self.place(expr),
            ExprKind::Number { value } if is_decimal(&ty) => decimal(value),
            ExprKind::Number { value } => format!("int64_t{{{value}}}"),
            ExprKind::Str { value } => format!("std::string({})", string(value)),
            // A text with slots in code is filled plainly; a key takes its value
            // in the language the screen picked, then its values.
            ExprKind::Text { .. } | ExprKind::Key { .. } => format!("{}.show()", self.shown(expr)),
            ExprKind::Bool { value } => value.to_string(),
            ExprKind::Empty => "std::nullopt".to_string(),
            ExprKind::Name { .. } | ExprKind::Member { .. } => self.place(expr),
            ExprKind::Unary { op: UnaryOp::Not, operand } => format!("!{}", self.value(operand)),
            ExprKind::Unary { op: UnaryOp::Neg, operand } => format!("-{}", self.value(operand)),
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right, &ty),
            ExprKind::In { value, options } => {
                let value_ty = self.types.type_of(value);
                let common = if is_decimal(&value_ty) || options.iter().any(|o| is_decimal(&self.types.type_of(o))) { TypeInfo::Decimal } else { value_ty };
                let v = self.value(value);
                let opts: Vec<String> = options.iter().map(|o| self.value(o)).collect();
                format!("clarity::one_of<{}>({v}, {{{}}})", value_type(&common), opts.join(", "))
            }
            ExprKind::Call { function, args } => self.call(&function.name, args),
            ExprKind::Find { entity, filter, order, var } => self.find(&var.name, &entity.name, filter.as_deref(), order),
            ExprKind::Files { var, store, filter, order } => self.files(&var.name, &store.name, filter.as_deref(), order),
            ExprKind::ReadText { file } => format!("db.read_text({})", self.value(file)),
            ExprKind::ReadBytes { file } => format!("db.read_bytes({})", self.value(file)),
            ExprKind::EngineCall { engine, function, args } if crate::engine(self.project, &engine.name).is_some_and(|e| e.body.is_some()) => {
                self.written_call(&engine.name, &function.name, args)
            }
            ExprKind::EngineCall { engine, function, args } => {
                let returns = self
                    .project
                    .files
                    .iter()
                    .flat_map(|f| &f.engines)
                    .find(|e| e.name.name == engine.name)
                    .and_then(|e| e.functions.iter().find(|f| f.name.name == function.name))
                    .and_then(|f| f.returns.as_ref())
                    .map(|r| value_type(&type_ref_info(self.project, r)));
                // Bytes cross as base64 text; other values as their JSON.
                let values: Vec<String> = args
                    .iter()
                    .map(|a| if self.types.type_of(a) == TypeInfo::Bytes { format!("clarity::base64({})", self.value(a)) } else { self.value(a) })
                    .collect();
                let bytes_back = returns.as_deref() == Some("clarity::Bytes");
                let returns = if bytes_back { Some("std::string".to_string()) } else { returns };
                // An engine with state per window is called in the handler's own window.
                let per_window = self.project.files.iter().flat_map(|f| &f.engines).any(|e| e.name.name == engine.name && e.state == clarity_spec::code::EngineState::Window);
                let (call, window) = match (returns, per_window) {
                    (Some(ty), true) => (format!("call_in_as<{ty}>"), "ui.instance(), "),
                    (Some(ty), false) => (format!("call_as<{ty}>"), ""),
                    (None, true) => ("call_in".to_string(), "ui.instance(), "),
                    (None, false) => ("call".to_string(), ""),
                };
                let args = if values.is_empty() { "nlohmann::json::array()".to_string() } else { format!("nlohmann::json::array({{{}}})", values.join(", ")) };
                let call = format!("engines::{}().{call}({window}{}, {args})", snake(&engine.name), string(&function.name));
                if bytes_back {
                    format!("clarity::from_base64({call})")
                } else if self.types.type_of(expr) == TypeInfo::Text {
                    // The text an engine gives is the value of its key, as its `says` lists it.
                    format!("clarity::text_of({call})")
                } else {
                    call
                }
            }
            ExprKind::New { entity } => format!("{}{{}}", entity.name),
        }
    }

    /// Whether a date, reference or picture is empty (`x == none`); a record
    /// that is no field is none until it is stored.
    fn is_none(&mut self, expr: &Expr) -> String {
        if self.is_reference_field(expr) {
            return format!("!{}.has_value()", self.stored_reference(expr));
        }
        if matches!(self.types.type_of(expr), TypeInfo::Record(_)) {
            return format!("{}.id == 0", self.value(expr));
        }
        format!("!{}.has_value()", self.value(expr))
    }

    fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr, result: &TypeInfo) -> String {
        if matches!(op, BinaryOp::Eq | BinaryOp::NotEq) {
            let other = match (&left.kind, &right.kind) {
                (_, ExprKind::Empty) => Some(left),
                (ExprKind::Empty, _) => Some(right),
                _ => None,
            };
            if let Some(other) = other {
                let test = self.is_none(other);
                return if op == BinaryOp::Eq { format!("({test})") } else { format!("!({test})") };
            }
        }
        let (lt, rt) = (self.types.type_of(left), self.types.type_of(right));
        match op {
            BinaryOp::And | BinaryOp::Or => {
                let symbol = if op == BinaryOp::And { "&&" } else { "||" };
                let (l, r) = (self.value(left), self.value(right));
                format!("({l} {symbol} {r})")
            }
            BinaryOp::Contains => {
                let (l, r) = (self.value(left), self.value(right));
                format!("clarity::text_contains({l}, {r})")
            }
            BinaryOp::Add if *result == TypeInfo::Text => {
                let (l, r) = (self.value(left), self.value(right));
                format!("({} + {})", show(l, &lt), show(r, &rt))
            }
            _ if matches!(lt, TypeInfo::Record(_)) => {
                let (l, r) = (self.value(left), self.value(right));
                format!("({l}.id {} {r}.id)", op.symbol())
            }
            // Decimals take whole numbers as they are; dates compare as optionals, empty first.
            _ => {
                let (mut l, mut r) = (self.value(left), self.value(right));
                if is_decimal(&rt) && !is_decimal(&lt) {
                    l = format!("clarity::Decimal({l})");
                }
                if is_decimal(&lt) && !is_decimal(&rt) {
                    r = format!("clarity::Decimal({r})");
                }
                format!("({l} {} {r})", op.symbol())
            }
        }
    }

    /// The arguments of a call, each as its parameter takes it.
    fn args(&mut self, args: &[Expr], params: &[clarity_spec::code::Param]) -> Vec<String> {
        let mut code = Vec::new();
        for (arg, param) in args.iter().zip(params) {
            let ty = type_ref_info(self.project, &param.ty);
            code.push(match ty {
                TypeInfo::Record(_) => self.record_arg(arg),
                ref t if is_decimal(t) && !is_decimal(&self.types.type_of(arg)) => format!("clarity::Decimal({})", self.value(arg)),
                _ => self.value(arg),
            });
        }
        code
    }

    /// A function of an engine written in Clarity, on the instance of the
    /// handler's window (state per window) or the app's (0).
    fn written_call(&mut self, engine: &str, function: &str, args: &[Expr]) -> String {
        let Some(decl) = crate::engine(self.project, engine) else { return String::new() };
        let Some(callee) = decl.body.as_ref().and_then(|b| b.functions.iter().find(|f| f.name.name == function)) else {
            self.errors.push(format!("engine {engine} has no function '{function}'"));
            return String::new();
        };
        let instance = match decl.state {
            EngineState::Window if self.window.is_some() => "ui.instance()",
            EngineState::Window => {
                self.errors.push(format!("engine {engine} keeps state per window; call it from a handler"));
                ""
            }
            _ => "0",
        };
        let mut code = vec!["db".to_string(), instance.to_string()];
        code.extend(self.args(args, &callee.params));
        format!("{}_{}({})", snake(engine), snake(function), code.join(", "))
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> String {
        use clarity_spec::code::check::{NOW, TODAY};
        if name == TODAY {
            return "std::optional<clarity::Date>(clarity::today())".into();
        }
        if name == NOW {
            return "std::optional<clarity::DateTime>(clarity::now())".into();
        }
        // round(value, places), as every runtime rounds (conformance/rounding.json).
        // sin(degrees), cos(degrees), as every runtime gives them (conformance/drawing.json).
        if name == clarity_spec::code::check::SIN || name == clarity_spec::code::check::COS {
            let value = match args.first() {
                Some(arg) if is_decimal(&self.types.type_of(arg)) => self.value(arg),
                Some(arg) => format!("clarity::Decimal({})", self.value(arg)),
                None => String::new(),
            };
            return format!("clarity::{name}({value})");
        }
        if name == clarity_spec::code::check::ROUND {
            let value = args.first().map(|a| self.value(a)).unwrap_or_default();
            let places = args.get(1).map(|a| self.value(a)).unwrap_or_default();
            let simple = value.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.');
            let value = if simple { value } else { format!("clarity::Decimal({value})") };
            return format!("{value}.rounded(int({places}))");
        }
        if name == "count" {
            let list = args.first().map(|a| self.value(a)).unwrap_or_default();
            return format!("int64_t({list}.size())");
        }
        // `random(6)` in an engine that uses random.
        if name == clarity_spec::code::check::RANDOM && !self.project.files.iter().flat_map(|f| &f.functions).any(|f| f.name.name == name) {
            let max = args.first().map(|a| self.value(a)).unwrap_or_default();
            return format!("clarity::random({max})");
        }
        // An engine's own function, on the same kept values.
        if let Some(engine) = self.engine
            && let Some(own) = engine.body.as_ref().and_then(|b| b.functions.iter().find(|f| f.name.name == name))
        {
            let mut code = vec!["db".to_string(), "kept".to_string()];
            code.extend(self.args(args, &own.params));
            return format!("{}_{}_with({})", snake(&engine.name.name), snake(name), code.join(", "));
        }
        let Some(callee) = self.project.files.iter().flat_map(|f| &f.functions).find(|f| f.name.name == name) else {
            self.errors.push(format!("unknown function '{name}'"));
            return String::new();
        };
        let mut code = vec!["db".to_string()];
        for (arg, param) in args.iter().zip(&callee.params) {
            let ty = type_ref_info(self.project, &param.ty);
            let value = match ty {
                TypeInfo::Record(_) => self.record_arg(arg),
                ref t if is_decimal(t) && !is_decimal(&self.types.type_of(arg)) => format!("clarity::Decimal({})", self.value(arg)),
                _ => self.value(arg),
            };
            code.push(value);
        }
        format!("{}({})", ident(name), code.join(", "))
    }

    /// `find c in Entity where ... order by ...` as a database query.
    fn find(&mut self, var: &str, entity: &str, filter: Option<&Expr>, order: &[OrderKey]) -> String {
        let mut query = "clarity::Query()".to_string();
        if let Some(filter) = filter {
            let cond = self.cond(filter, var, entity);
            let _ = write!(query, ".filter({cond})");
        }
        for key in order {
            let Some(path) = crate::query::order_path(self.project, entity, key) else {
                self.errors.push(format!("{entity} cannot be ordered by '{}'", key.field.name));
                continue;
            };
            let numeric = self.field_type(&path.entity, &path.field).is_some_and(|t| is_stored_number(&t));
            let column = string(&self.column(&path.entity, &path.field));
            if path.hops.is_empty() {
                let method = if numeric { "order_number" } else { "order" };
                let _ = write!(query, ".{method}({column}, {})", key.descending);
            } else {
                let hops: Vec<String> = path.hops.iter().map(|h| format!("{{{}, {}, {}}}", string(&h.column), string(&h.table), string(&h.key))).collect();
                let _ = write!(query, ".order_through({{{}}}, {column}, {}, {numeric})", hops.join(", "), key.descending);
            }
        }
        format!("db.find<{entity}>({query})")
    }

    /// `files f in store where ... order by ...`: the files of a store,
    /// filtered and sorted in memory.
    fn files(&mut self, var: &str, store: &str, filter: Option<&Expr>, order: &[OrderKey]) -> String {
        let mut list = format!("db.files({})", string(store));
        if let Some(filter) = filter {
            let cond = self.value(filter);
            list = format!("clarity::filter_files({list}, [&](const clarity::FileEntry& {}) {{ return {cond}; }})", ident(var));
        }
        if order.is_empty() {
            return list;
        }
        let keys: Vec<String> = order.iter().map(|k| format!("clarity::{}({})", if k.descending { "desc" } else { "asc" }, string(&k.field.name))).collect();
        format!("clarity::sort_files({list}, {{{}}})", keys.join(", "))
    }

    /// A query condition as a `clarity::Cond`.
    fn cond(&mut self, expr: &Expr, var: &str, entity: &str) -> String {
        let project = self.project;
        let path_of = |e: &Expr| crate::query::field_path(project, entity, var, e);
        match &expr.kind {
            ExprKind::Binary { op: BinaryOp::And, left, right } => {
                let l = self.cond(left, var, entity);
                format!("({l} && {})", self.cond(right, var, entity))
            }
            ExprKind::Binary { op: BinaryOp::Or, left, right } => {
                let l = self.cond(left, var, entity);
                format!("({l} || {})", self.cond(right, var, entity))
            }
            ExprKind::Unary { op: UnaryOp::Not, operand } => format!("!{}", self.cond(operand, var, entity)),
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
                    let test = format!("clarity::Cond::is_none({name})");
                    if op == BinaryOp::Eq { test } else { format!("!{test}") }
                } else if op == BinaryOp::Contains {
                    format!("clarity::Cond::contains({name}, {})", self.value(value))
                } else {
                    let constructor = if is_stored_number(&ty) { "number" } else { "compare" };
                    let value = self.sql_value(&ty, value);
                    format!("clarity::Cond::{constructor}({name}, clarity::Op::{}, {value})", op_name(op))
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
                let constructor = if is_stored_number(&ty) { "one_of_numbers" } else { "one_of" };
                let values: Vec<String> = options.iter().map(|o| self.sql_value(&ty, o)).collect();
                let own = format!("clarity::Cond::{constructor}({}, {{{}}})", string(&self.column(&path.entity, &path.field)), values.join(", "));
                through(&path.hops, own)
            }
            _ => match path_of(expr) {
                Some(path) => through(&path.hops, format!("clarity::Cond::is_true({})", string(&self.column(&path.entity, &path.field)))),
                None => {
                    self.errors.push(format!("'{}' is not a condition on {var}", print::expr(expr)));
                    String::new()
                }
            },
        }
    }

    /// A value compared with a field of type `ty`, as the runtime stores it.
    fn sql_value(&mut self, ty: &FieldType, value: &Expr) -> String {
        let from = self.types.type_of(value);
        match ty {
            FieldType::Text { .. } | FieldType::Email | FieldType::Url => format!("clarity::sql::text({})", self.value(value)),
            FieldType::Integer | FieldType::Id | FieldType::Duration => format!("clarity::sql::integer({})", self.value(value)),
            FieldType::Decimal { .. } | FieldType::Money => format!("clarity::sql::decimal(clarity::Decimal({}))", self.value(value)),
            FieldType::Bool => format!("clarity::sql::boolean({})", self.value(value)),
            FieldType::Date => format!("clarity::sql::date({})", self.value(value)),
            FieldType::DateTime => format!("clarity::sql::date_time({})", self.value(value)),
            FieldType::Enum { .. } => format!("clarity::sql::text(name_of({}))", self.value(value)),
            FieldType::Ref { .. } => match from {
                // A reference field holds the id already; a record has its own.
                TypeInfo::Record(_) if self.is_reference_field(value) => format!("clarity::sql::reference({})", self.stored_reference(value)),
                TypeInfo::Record(_) => format!("clarity::sql::integer({}.id)", self.value(value)),
                _ => format!("clarity::sql::reference({})", self.value(value)),
            },
            FieldType::Image | FieldType::List { .. } => {
                self.errors.push("a picture or a list is compared with none only".into());
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
    fn stored_reference(&mut self, expr: &Expr) -> String {
        let ExprKind::Member { object, member } = &expr.kind else { return String::new() };
        format!("{}.{}", self.place(object), ident(&member.name))
    }
}

/// A value as text, the way `"text" + value` shows it.
fn show(code: String, ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::Text => code,
        TypeInfo::Enum(_) => format!("label_of({code})"),
        _ => format!("clarity::show({code})"),
    }
}

/// A condition on the record the steps lead to, as one on the query's own
/// record: each step a `clarity::Cond::through`, the first one outermost.
fn through(hops: &[crate::query::Hop], cond: String) -> String {
    hops.iter().rev().fold(cond, |inner, h| {
        format!("clarity::Cond::through({}, {}, {}, {inner})", string(&h.column), string(&h.table), string(&h.key))
    })
}

/// A handler (camelCase in Clarity) as the member function of its window,
/// in C++'s snake_case: `approveClicked` is `approve_clicked`.
fn method(handler: &str) -> String {
    ident(&super::names::snake(handler))
}
