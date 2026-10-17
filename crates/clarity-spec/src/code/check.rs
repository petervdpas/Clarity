//! Checks code blocks against the window or app they belong to: every name
//! resolves (locals, the window's data, its parts, enum values), every
//! expression has a type that fits where it is used, and the handlers
//! written match the handlers wired to events. Functions and actions in
//! modules are checked the same way, with their own rules for effects.

use std::collections::HashMap;

use super::{
    BinaryOp, CodeBlock, EFFECTS, Expr, ExprKind, FunctionDecl, FunctionKind, HandlerDecl, OrderKey, Stmt, StmtKind,
    TypeRef, TypeRefKind, UnaryOp,
};
use crate::diagnostic::Diagnostic;
use crate::model::{
    Command, CommandKind, CommandProps, DataDecl, Entity, EnumDecl, FieldType, Frame, Ident, Page, SpecFile,
    Widget, WidgetProps,
};
use crate::span::Span;
use crate::widgets::{self, WidgetKind};

pub(crate) fn check(files: &[SpecFile]) -> Vec<Diagnostic> {
    run(files).0
}

/// The type of every expression in every code block, function and action,
/// and what each name refers to, keyed by file path. Generators read this
/// instead of working types out again, so the checker stays the one place
/// that decides them.
pub fn types(files: &[SpecFile]) -> HashMap<String, FileTypes> {
    run(files).1
}

fn run(files: &[SpecFile]) -> (Vec<Diagnostic>, HashMap<String, FileTypes>) {
    let mut diagnostics = Vec::new();
    let mut types = HashMap::new();
    for (index, file) in files.iter().enumerate() {
        let env = Env::new(files, index);
        let mut out = Output::default();
        check_file(&env, file, &mut out);
        diagnostics.extend(out.diagnostics.into_iter().map(|d| d.in_file(&file.path)));
        types.insert(file.path.clone(), out.types);
    }
    (diagnostics, types)
}

/// A type as generators see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeInfo {
    Text,
    Integer,
    Decimal,
    Money,
    Bool,
    Date,
    DateTime,
    Enum(String),
    Record(String),
    List(String),
    Unknown,
}

/// What a name in code refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameKind {
    /// A `let`, a `for` variable, a parameter or `result`.
    Local,
    /// The window's data.
    Data,
    /// A part of the window.
    Part,
    /// A value of this enum, written bare or qualified.
    EnumValue(String),
    /// The enum itself, in `CustomerKind.Business`.
    EnumType,
    /// A call that runs another handler of the same window or app.
    Handler,
}

/// Types and names of one file, keyed by the span (start, end) of each
/// expression.
#[derive(Debug, Default, Clone)]
pub struct FileTypes {
    pub types: HashMap<(u32, u32), TypeInfo>,
    pub names: HashMap<(u32, u32), NameKind>,
}

impl FileTypes {
    pub fn type_of(&self, expr: &Expr) -> TypeInfo {
        self.types.get(&(expr.span.start, expr.span.end)).cloned().unwrap_or(TypeInfo::Unknown)
    }

    pub fn name_of(&self, expr: &Expr) -> Option<&NameKind> {
        self.names.get(&(expr.span.start, expr.span.end))
    }
}

/// What checking a file produces.
#[derive(Default)]
struct Output {
    diagnostics: Vec<Diagnostic>,
    types: FileTypes,
}

impl Output {
    fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    fn len(&self) -> usize {
        self.diagnostics.len()
    }

    fn truncate(&mut self, len: usize) {
        self.diagnostics.truncate(len);
    }
}

/// What a file can see: its own module and the modules it uses.
struct Env<'a> {
    visible: Vec<&'a SpecFile>,
    /// Functions of the same module declared in files before this one.
    earlier_functions: Vec<&'a FunctionDecl>,
}

impl<'a> Env<'a> {
    fn new(files: &'a [SpecFile], index: usize) -> Self {
        let file = &files[index];
        let module = |f: &SpecFile| f.module.as_ref().map(|m| m.name.clone());
        let own = module(file);
        let visible = files
            .iter()
            .filter(|f| {
                let m = module(f);
                m == own || file.uses.iter().any(|u| Some(&u.name) == m.as_ref())
            })
            .collect();
        let earlier_functions = files[..index]
            .iter()
            .filter(|f| module(f) == own)
            .flat_map(|f| &f.functions)
            .collect();
        Self {
            visible,
            earlier_functions,
        }
    }

    fn entity(&self, name: &str) -> Option<&'a Entity> {
        self.visible.iter().flat_map(|f| &f.entities).find(|e| e.name.name == name)
    }

    fn enum_decl(&self, name: &str) -> Option<&'a EnumDecl> {
        self.visible.iter().flat_map(|f| &f.enums).find(|e| e.name.name == name)
    }

    fn page(&self, name: &str) -> Option<&'a Page> {
        self.visible.iter().flat_map(|f| &f.pages).find(|p| p.name.name == name)
    }

    fn function(&self, name: &str) -> Option<&'a FunctionDecl> {
        self.visible.iter().flat_map(|f| &f.functions).find(|f| f.name.name == name)
    }

    /// The type a parameter, result or typed local names.
    fn type_ref(&self, ty: &TypeRef) -> Result<Ty<'a>, String> {
        Ok(match &ty.kind {
            TypeRefKind::Text => Ty::Text,
            TypeRefKind::Integer => Ty::Integer,
            TypeRefKind::Decimal => Ty::Decimal,
            TypeRefKind::Money => Ty::Money,
            TypeRefKind::Bool => Ty::Bool,
            TypeRefKind::Date => Ty::Date,
            TypeRefKind::DateTime => Ty::DateTime,
            TypeRefKind::Named { name } => match (self.entity(name), self.enum_decl(name)) {
                (Some(entity), _) => Ty::Record(entity),
                (None, Some(decl)) => Ty::Enum(decl),
                (None, None) => return Err(format!("unknown type '{name}'")),
            },
            TypeRefKind::List { item } => match self.entity(item) {
                Some(entity) => Ty::List(entity),
                None => return Err(format!("unknown entity '{item}'")),
            },
        })
    }

    fn data_type(&self, decl: &DataDecl) -> Ty<'a> {
        match self.entity(&decl.entity.name) {
            Some(entity) if decl.list => Ty::List(entity),
            Some(entity) => Ty::Record(entity),
            // Reported by the main checker.
            None => Ty::Unknown,
        }
    }

    fn field_type(&self, ty: &FieldType) -> Ty<'a> {
        match ty {
            FieldType::Text { .. } | FieldType::Email | FieldType::Url => Ty::Text,
            FieldType::Integer | FieldType::Id => Ty::Integer,
            FieldType::Decimal { .. } => Ty::Decimal,
            FieldType::Money => Ty::Money,
            FieldType::Bool => Ty::Bool,
            FieldType::Date => Ty::Date,
            FieldType::DateTime => Ty::DateTime,
            FieldType::Enum { name } => self.enum_decl(&name.name).map_or(Ty::Unknown, Ty::Enum),
            FieldType::Ref { entity } => self.entity(&entity.name).map_or(Ty::Unknown, Ty::Record),
            FieldType::List { item } => self.entity(&item.name).map_or(Ty::Unknown, Ty::List),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Ty<'a> {
    Text,
    Integer,
    Decimal,
    Money,
    Bool,
    Date,
    DateTime,
    Enum(&'a EnumDecl),
    Record(&'a Entity),
    List(&'a Entity),
    /// The type could not be found; the reason is reported once already.
    Unknown,
}

impl Ty<'_> {
    fn info(&self) -> TypeInfo {
        match self {
            Ty::Text => TypeInfo::Text,
            Ty::Integer => TypeInfo::Integer,
            Ty::Decimal => TypeInfo::Decimal,
            Ty::Money => TypeInfo::Money,
            Ty::Bool => TypeInfo::Bool,
            Ty::Date => TypeInfo::Date,
            Ty::DateTime => TypeInfo::DateTime,
            Ty::Enum(e) => TypeInfo::Enum(e.name.name.clone()),
            Ty::Record(e) => TypeInfo::Record(e.name.name.clone()),
            Ty::List(e) => TypeInfo::List(e.name.name.clone()),
            Ty::Unknown => TypeInfo::Unknown,
        }
    }

    fn name(&self) -> String {
        match self {
            Ty::Text => "text".into(),
            Ty::Integer => "integer".into(),
            Ty::Decimal => "decimal".into(),
            Ty::Money => "money".into(),
            Ty::Bool => "bool".into(),
            Ty::Date => "date".into(),
            Ty::DateTime => "datetime".into(),
            Ty::Enum(e) => e.name.name.clone(),
            Ty::Record(e) => e.name.name.clone(),
            Ty::List(e) => format!("list<{}>", e.name.name),
            Ty::Unknown => "unknown".into(),
        }
    }

    fn is_unknown(&self) -> bool {
        matches!(self, Ty::Unknown)
    }

    fn is_numeric(&self) -> bool {
        matches!(self, Ty::Integer | Ty::Decimal | Ty::Money)
    }

    /// A single value that can be shown as text.
    fn is_value(&self) -> bool {
        !matches!(self, Ty::Record(_) | Ty::List(_))
    }

    fn same(&self, other: &Ty) -> bool {
        match (self, other) {
            (Ty::Enum(a), Ty::Enum(b)) => std::ptr::eq(*a, *b),
            (Ty::Record(a), Ty::Record(b)) | (Ty::List(a), Ty::List(b)) => std::ptr::eq(*a, *b),
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }

    /// Whether a value of type `value` can be stored where `self` is
    /// expected. Integers widen to decimals and money; decimals and money
    /// mix.
    fn accepts(&self, value: &Ty) -> bool {
        if self.is_unknown() || value.is_unknown() || self.same(value) {
            return true;
        }
        match self {
            Ty::Decimal | Ty::Money => value.is_numeric(),
            _ => false,
        }
    }

    fn comparable(&self, other: &Ty) -> bool {
        self.accepts(other) || other.accepts(self)
    }
}

/// A named part of a window or the app frame.
#[derive(Clone, Copy)]
enum Part<'a> {
    Widget(&'a WidgetProps),
    Command(&'a CommandProps),
}

impl Part<'_> {
    fn keyword(&self) -> &'static str {
        match self {
            Part::Widget(props) => widgets::keyword(props.kind),
            Part::Command(props) => match props.kind {
                CommandKind::Button => "button",
                _ => "item",
            },
        }
    }

    fn has_caption(&self) -> bool {
        match self {
            Part::Widget(props) => matches!(
                props.kind,
                WidgetKind::Label | WidgetKind::Help | WidgetKind::Heading | WidgetKind::Button
            ),
            Part::Command(_) => true,
        }
    }

    /// Inputs take an error message and can be made read-only.
    fn is_input(&self) -> bool {
        matches!(self, Part::Widget(props) if widgets::info(props.kind).input)
    }
}

/// What a handler can see of its window or app.
struct Owner<'a> {
    name: &'a str,
    data: &'a [DataDecl],
    parts: HashMap<&'a str, Part<'a>>,
    /// Handlers wired to events, with the span of the part that wires them.
    wired: Vec<(&'a str, Span)>,
}

fn collect_widgets<'a>(widgets: &'a [Widget], owner: &mut Owner<'a>) {
    for widget in widgets {
        if let Some(name) = &widget.props.name {
            owner.parts.insert(name, Part::Widget(&widget.props));
        }
        for handler in &widget.props.handlers {
            owner.wired.push((&handler.handler, widget.span));
        }
        collect_widgets(&widget.children, owner);
        for page in &widget.pages {
            collect_widgets(&page.children, owner);
        }
    }
}

fn collect_commands<'a>(commands: &'a [Command], owner: &mut Owner<'a>) {
    for command in commands {
        if let Some(name) = &command.props.name {
            owner.parts.insert(name, Part::Command(&command.props));
        }
        if let Some(handler) = &command.props.handler {
            owner.wired.push((handler, command.span));
        }
        collect_commands(&command.items, owner);
    }
}

fn collect_frame<'a>(frame: &'a Frame, owner: &mut Owner<'a>) {
    if let Some(menu) = &frame.menu {
        collect_commands(&menu.items, owner);
    }
    if let Some(toolbar) = &frame.toolbar {
        collect_commands(&toolbar.items, owner);
    }
    if let Some(ribbon) = &frame.ribbon {
        for group in ribbon.tabs.iter().flat_map(|t| &t.groups) {
            collect_commands(&group.items, owner);
        }
    }
    for sidebar in &frame.sidebars {
        collect_widgets(&sidebar.widgets, owner);
    }
    if let Some(footer) = &frame.footer {
        collect_widgets(&footer.widgets, owner);
    }
}

fn owners(file: &SpecFile) -> Vec<Owner<'_>> {
    let mut owners = Vec::new();
    for page in &file.pages {
        let mut owner = Owner {
            name: &page.name.name,
            data: &page.data,
            parts: HashMap::new(),
            wired: Vec::new(),
        };
        owner.wired.extend(page.handlers.iter().map(|h| (h.handler.as_str(), page.span)));
        collect_widgets(&page.widgets, &mut owner);
        collect_frame(&page.frame, &mut owner);
        owners.push(owner);
    }
    for app in &file.apps {
        let mut owner = Owner {
            name: &app.name.name,
            data: &app.data,
            parts: HashMap::new(),
            wired: Vec::new(),
        };
        owner.wired.extend(app.handlers.iter().map(|h| (h.handler.as_str(), app.span)));
        collect_frame(&app.frame, &mut owner);
        owners.push(owner);
    }
    owners
}

fn check_file(env: &Env, file: &SpecFile, out: &mut Output) {
    check_functions(env, file, out);
    let owners = owners(file);
    let mut seen: Vec<&str> = Vec::new();
    for block in &file.code {
        let name = block.name.name.as_str();
        if seen.contains(&name) {
            out.push(Diagnostic::error(format!("there is already a code block for {name}"), block.name.span));
            continue;
        }
        seen.push(name);
        let Some(owner) = owners.iter().find(|o| o.name == name) else {
            out.push(Diagnostic::error(
                format!("there is no window or app '{name}' in this file"),
                block.name.span,
            ));
            continue;
        };
        check_block(env, owner, block, out);
    }
    // Wired handlers that are not written yet, window by window.
    for owner in &owners {
        let block = file.code.iter().find(|b| b.name.name == owner.name);
        let mut reported: Vec<&str> = Vec::new();
        for (handler, span) in &owner.wired {
            let written = block.is_some_and(|b| b.handlers.iter().any(|h| h.name.name == *handler));
            if !written && !reported.contains(handler) {
                reported.push(handler);
                out.push(Diagnostic::warning(
                    format!("handler '{handler}' is wired but not written yet"),
                    *span,
                ));
            }
        }
    }
}

fn check_block(env: &Env, owner: &Owner, block: &CodeBlock, out: &mut Output) {
    for decl in owner.data {
        if owner.parts.contains_key(decl.name.name.as_str()) {
            out.push(Diagnostic::error(
                format!("'{}' is both data and a part of {}", decl.name.name, owner.name),
                decl.name.span,
            ));
        }
    }
    for (i, handler) in block.handlers.iter().enumerate() {
        let name = handler.name.name.as_str();
        if block.handlers[..i].iter().any(|h| h.name.name == name) {
            out.push(Diagnostic::error(format!("handler '{name}' is written twice"), handler.name.span));
            continue;
        }
        if !owner.wired.iter().any(|(w, _)| *w == name) {
            out.push(Diagnostic::warning(
                format!("handler '{name}' is not wired to any event"),
                handler.name.span,
            ));
        }
        check_handler(env, owner, &block.handlers, handler, out);
    }
}

fn check_handler(env: &Env, owner: &Owner, handlers: &[HandlerDecl], handler: &HandlerDecl, out: &mut Output) {
    let mut checker = BodyChecker {
        env,
        owner: Some(owner),
        handlers,
        decl: None,
        returns: None,
        scopes: vec![HashMap::new()],
        saves_reported: false,
        out,
    };
    checker.block(&handler.body);
}

/// Checks the functions and actions of a file: their names, effects,
/// parameter types, conditions and bodies.
fn check_functions(env: &Env, file: &SpecFile, out: &mut Output) {
    for (i, decl) in file.functions.iter().enumerate() {
        let name = decl.name.name.as_str();
        let earlier = file.functions[..i].iter().any(|f| f.name.name == name)
            || env.earlier_functions.iter().any(|f| f.name.name == name);
        if earlier {
            out.push(Diagnostic::error(
                format!("{} '{name}' is declared twice", decl.kind.keyword()),
                decl.name.span,
            ));
            continue;
        }
        for effect in &decl.effects {
            if decl.kind == FunctionKind::Function {
                out.push(Diagnostic::error(
                    format!("function '{name}' cannot have effects; make it an action"),
                    effect.span,
                ));
            } else if !EFFECTS.contains(&effect.name.as_str()) {
                out.push(Diagnostic::error(
                    format!("unknown effect '{}'; the effects are {}", effect.name, EFFECTS.join(" and ")),
                    effect.span,
                ));
            }
        }
        let mut scope = HashMap::new();
        for param in &decl.params {
            let ty = env.type_ref(&param.ty).unwrap_or_else(|message| {
                out.push(Diagnostic::error(message, param.ty.span));
                Ty::Unknown
            });
            scope.insert(param.name.name.clone(), ty);
        }
        let returns = decl.returns.as_ref().map(|r| {
            env.type_ref(r).unwrap_or_else(|message| {
                out.push(Diagnostic::error(message, r.span));
                Ty::Unknown
            })
        });
        let mut checker = BodyChecker {
            env,
            owner: None,
            handlers: &[],
            decl: Some(decl),
            returns,
            scopes: vec![scope],
            saves_reported: false,
            out,
        };
        for condition in &decl.requires {
            checker.condition(condition);
        }
        if let Some(result) = returns {
            checker.scopes.push(HashMap::from([("result".to_string(), result)]));
            for condition in &decl.ensures {
                checker.condition(condition);
            }
            checker.scopes.pop();
        } else if let Some(first) = decl.ensures.first() {
            checker.error(format!("'ensure' needs a result; {name} returns nothing"), first.span);
        }
        checker.block(&decl.body);
        if returns.is_some() && !always_returns(&decl.body) {
            out.push(Diagnostic::error(
                format!("{} '{name}' does not return a value on every path", decl.kind.keyword()),
                decl.name.span,
            ));
        }
    }
}

/// Whether every path through `body` ends in `return`.
fn always_returns(body: &[Stmt]) -> bool {
    body.iter().any(|stmt| match &stmt.kind {
        StmtKind::Return { .. } => true,
        StmtKind::If { branches, otherwise: Some(otherwise) } => {
            branches.iter().all(|b| always_returns(&b.body)) && always_returns(otherwise)
        }
        _ => false,
    })
}

/// Checks the statements of a handler, function or action.
struct BodyChecker<'e, 'a> {
    env: &'e Env<'a>,
    /// The window or app of a handler; functions and actions have none.
    owner: Option<&'e Owner<'a>>,
    /// The handlers of the same window or app, which a handler can run.
    handlers: &'e [HandlerDecl],
    /// The function or action being checked; None for a handler.
    decl: Option<&'e FunctionDecl>,
    returns: Option<Ty<'a>>,
    scopes: Vec<HashMap<String, Ty<'a>>>,
    /// A missing `effects db.write` is reported once per action.
    saves_reported: bool,
    out: &'e mut Output,
}

/// What a name or path refers to.
enum Place<'a> {
    Value { ty: Ty<'a>, writable: Result<(), String> },
    Part(Part<'a>, &'a str),
    Enum(&'a EnumDecl),
}

impl<'a> BodyChecker<'_, 'a> {
    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.out.push(Diagnostic::error(message, span));
    }

    fn data(&self, name: &str) -> Option<&'a DataDecl> {
        self.owner.and_then(|o| o.data.iter().find(|d| d.name.name == name))
    }

    fn part(&self, name: &str) -> Option<(&'a str, Part<'a>)> {
        self.owner.and_then(|o| o.parts.get_key_value(name)).map(|(k, p)| (*k, *p))
    }

    /// "a handler", "a function" or "an action", for messages.
    fn context(&self) -> &'static str {
        match self.decl.map(|d| d.kind) {
            None => "a handler",
            Some(FunctionKind::Function) => "a function",
            Some(FunctionKind::Action) => "an action",
        }
    }

    fn block(&mut self, body: &[Stmt]) {
        self.scopes.push(HashMap::new());
        for stmt in body {
            self.stmt(stmt);
        }
        self.scopes.pop();
    }

    fn local(&self, name: &str) -> Option<Ty<'a>> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }

    fn define(&mut self, name: &Ident, ty: Ty<'a>) {
        let n = name.name.as_str();
        if self.local(n).is_some() {
            self.error(format!("'{n}' is already defined"), name.span);
        } else if self.part(n).is_some() || self.data(n).is_some() {
            self.error(format!("'{n}' is already a name in this window"), name.span);
        } else if let Some(scope) = self.scopes.last_mut() {
            scope.insert(n.to_string(), ty);
        }
    }

    /// `require`, `ensure` and `if` conditions must be bool.
    fn condition(&mut self, condition: &Expr) {
        let ty = self.expr(condition, Some(Ty::Bool));
        if !Ty::Bool.accepts(&ty) {
            self.error(format!("a condition must be bool, not {}", ty.name()), condition.span);
        }
    }

    /// Statements that show or close windows belong in handlers.
    fn ui_only(&mut self, word: &str, span: Span) {
        if self.decl.is_some() {
            let context = self.context();
            self.error(format!("'{word}' belongs in a handler, not in {context}"), span);
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { name, ty, value } => {
                let declared = ty.as_ref().map(|t| {
                    self.env.type_ref(t).unwrap_or_else(|message| {
                        self.error(message, t.span);
                        Ty::Unknown
                    })
                });
                let value_ty = self.expr(value, declared);
                if let Some(declared) = declared
                    && !declared.accepts(&value_ty)
                {
                    self.error(format!("cannot set {} to {}", declared.name(), value_ty.name()), value.span);
                }
                self.define(name, declared.unwrap_or(value_ty));
            }
            StmtKind::Assign { target, value } => self.assign(target, value),
            StmtKind::If { branches, otherwise } => {
                for branch in branches {
                    let ty = self.expr(&branch.condition, Some(Ty::Bool));
                    if !Ty::Bool.accepts(&ty) {
                        self.error(
                            format!("the condition must be bool, not {}", ty.name()),
                            branch.condition.span,
                        );
                    }
                    self.block(&branch.body);
                }
                if let Some(body) = otherwise {
                    self.block(body);
                }
            }
            StmtKind::For { var, list, body } => {
                let ty = self.expr(list, None);
                let item = match ty {
                    Ty::List(entity) => Ty::Record(entity),
                    Ty::Unknown => Ty::Unknown,
                    other => {
                        self.error(format!("'for' needs a list, not {}", other.name()), list.span);
                        Ty::Unknown
                    }
                };
                self.scopes.push(HashMap::new());
                self.define(var, item);
                self.block(body);
                self.scopes.pop();
            }
            StmtKind::Return { value } => self.check_return(value.as_ref(), stmt.span),
            StmtKind::Call { call } => {
                self.call(call, true);
            }
            StmtKind::Close => self.ui_only("close", stmt.span),
            StmtKind::Reload { data } => {
                self.ui_only("reload", stmt.span);
                if self.decl.is_none() && self.data(&data.name).is_none() {
                    let owner = self.owner.map(|o| o.name).unwrap_or("");
                    self.error(format!("{owner} has no data '{}'", data.name), data.span);
                }
            }
            StmtKind::Message { text, .. } => {
                self.ui_only("message", stmt.span);
                let ty = self.expr(text, None);
                if !ty.is_value() {
                    self.error(format!("a message is text or a value, not {}", ty.name()), text.span);
                }
            }
            StmtKind::Open { window, with } => {
                self.ui_only("open", stmt.span);
                let Some(page) = self.env.page(&window.name) else {
                    self.error(format!("unknown window '{}'", window.name), window.span);
                    for arg in with {
                        self.expr(&arg.value, None);
                    }
                    return;
                };
                for arg in with {
                    let Some(decl) = page.data.iter().find(|d| d.name.name == arg.name.name) else {
                        self.error(format!("{} has no data '{}'", page.name.name, arg.name.name), arg.name.span);
                        self.expr(&arg.value, None);
                        continue;
                    };
                    let expected = self.env.data_type(decl);
                    let ty = self.expr(&arg.value, Some(expected));
                    if !expected.accepts(&ty) {
                        self.error(
                            format!(
                                "'{}' of {} is {} {}, not {}",
                                arg.name.name,
                                page.name.name,
                                article(&expected.name()),
                                expected.name(),
                                ty.name()
                            ),
                            arg.value.span,
                        );
                    }
                }
            }
            StmtKind::Save { value } | StmtKind::Delete { value } => {
                let word = if matches!(stmt.kind, StmtKind::Save { .. }) { "save" } else { "delete" };
                self.check_write(word, stmt.span);
                let ty = self.expr(value, None);
                if !matches!(ty, Ty::Record(_) | Ty::Unknown) {
                    self.error(format!("'{word}' needs a record, not {}", ty.name()), value.span);
                }
            }
        }
    }

    /// `save` and `delete`: not in functions; in actions only with
    /// `effects db.write`.
    fn check_write(&mut self, word: &str, span: Span) {
        let Some(decl) = self.decl else { return };
        match decl.kind {
            FunctionKind::Function => {
                self.error(format!("'{word}' is not allowed in a function; make it an action"), span);
            }
            FunctionKind::Action => {
                let declared = decl.effects.iter().any(|e| e.name == "db.write");
                if !declared && !self.saves_reported {
                    self.saves_reported = true;
                    self.error(
                        format!("action '{}' saves but does not declare 'effects db.write'", decl.name.name),
                        decl.name.span,
                    );
                }
            }
        }
    }

    fn check_return(&mut self, value: Option<&Expr>, span: Span) {
        let name = self.decl.map(|d| d.name.name.clone());
        match (self.returns, value, name) {
            (_, Some(value), None) => {
                self.expr(value, None);
                self.error("a handler does not return a value", value.span);
            }
            (None, Some(value), Some(name)) => {
                self.expr(value, None);
                self.error(format!("{name} does not return a value; use 'return' alone"), value.span);
            }
            (Some(expected), None, Some(name)) => {
                self.error(format!("{name} must return {}", expected.name()), span);
            }
            (Some(expected), Some(value), Some(name)) => {
                let ty = self.expr(value, Some(expected));
                if !expected.accepts(&ty) {
                    self.error(format!("{name} must return {}, not {}", expected.name(), ty.name()), value.span);
                }
            }
            _ => {}
        }
    }

    fn assign(&mut self, target: &Expr, value: &Expr) {
        let place = self.place(target);
        let (ty, writable) = match place {
            Some(Place::Value { ty, writable }) => (ty, writable),
            Some(Place::Part(part, name)) => {
                self.error(
                    format!("{} '{name}' is a part; set one of its properties", part.keyword()),
                    target.span,
                );
                self.expr(value, None);
                return;
            }
            Some(Place::Enum(decl)) => {
                self.error(format!("{} is an enum and cannot be set", decl.name.name), target.span);
                self.expr(value, None);
                return;
            }
            None => {
                self.expr(value, None);
                return;
            }
        };
        if let Err(message) = writable {
            self.error(message, target.span);
        }
        let value_ty = self.expr(value, Some(ty));
        if !ty.accepts(&value_ty) {
            self.error(format!("cannot set {} to {}", ty.name(), value_ty.name()), value.span);
        }
    }

    /// Resolves a name or member path. Reports and returns None when it
    /// does not resolve.
    fn place(&mut self, expr: &Expr) -> Option<Place<'a>> {
        let place = self.resolve(expr);
        if let Some(Place::Value { ty, .. }) = &place {
            self.out.types.types.insert((expr.span.start, expr.span.end), ty.info());
        }
        place
    }

    fn resolve(&mut self, expr: &Expr) -> Option<Place<'a>> {
        match &expr.kind {
            ExprKind::Name { name } => {
                if let Some(ty) = self.local(name) {
                    self.name_is(expr, NameKind::Local);
                    return Some(Place::Value { ty, writable: Ok(()) });
                }
                if let Some(decl) = self.data(name) {
                    self.name_is(expr, NameKind::Data);
                    return Some(Place::Value {
                        ty: self.env.data_type(decl),
                        writable: Ok(()),
                    });
                }
                if let Some((key, part)) = self.part(name) {
                    self.name_is(expr, NameKind::Part);
                    return Some(Place::Part(part, key));
                }
                if let Some(decl) = self.env.enum_decl(name) {
                    self.name_is(expr, NameKind::EnumType);
                    return Some(Place::Enum(decl));
                }
                self.error(format!("unknown name '{name}'"), expr.span);
                None
            }
            ExprKind::Member { object, member } => {
                let m = member.name.as_str();
                match self.place(object)? {
                    Place::Enum(decl) => {
                        if decl.variants.iter().any(|v| v.name.name == m) {
                            self.name_is(expr, NameKind::EnumValue(decl.name.name.clone()));
                            Some(Place::Value {
                                ty: Ty::Enum(decl),
                                writable: Err(format!("{}.{m} is a value and cannot be set", decl.name.name)),
                            })
                        } else {
                            self.error(format!("{} has no value '{m}'", decl.name.name), member.span);
                            None
                        }
                    }
                    Place::Part(Part::Widget(props), name) if m == "text" && props.bind.is_some() && has_text(props) => {
                        let bind = props.bind.as_deref().unwrap_or_default();
                        self.error(
                            format!("{} '{name}' is bound to {bind}; use {bind}", widgets::keyword(props.kind)),
                            member.span,
                        );
                        None
                    }
                    Place::Part(part, name) => match self.property(part, name, m) {
                        Some(place) => Some(place),
                        None => {
                            self.error(format!("{} '{name}' has no property '{m}'", part.keyword()), member.span);
                            None
                        }
                    },
                    Place::Value { ty, .. } => self.member(ty, member.name.as_str(), member.span),
                }
            }
            _ => {
                let ty = self.expr(expr, None);
                Some(Place::Value {
                    ty,
                    writable: Err("only a local, a data field or a part property can be set".into()),
                })
            }
        }
    }

    fn property(&self, part: Part<'a>, name: &str, property: &str) -> Option<Place<'a>> {
        let value = |ty| Some(Place::Value { ty, writable: Ok(()) });
        match property {
            "visible" | "enabled" => value(Ty::Bool),
            "caption" if part.has_caption() => value(Ty::Text),
            "error" if part.is_input() => value(Ty::Text),
            "readonly" if part.is_input() => value(Ty::Bool),
            "text" if matches!(part, Part::Widget(props) if has_text(props)) => value(Ty::Text),
            "selected" => {
                let Part::Widget(props) = part else { return None };
                if !matches!(props.kind, WidgetKind::Table | WidgetKind::List) {
                    return None;
                }
                let item = props.bind.as_deref().and_then(|b| self.data(b)).map(|d| self.env.data_type(d));
                let ty = match item {
                    Some(Ty::List(entity)) => Ty::Record(entity),
                    _ => Ty::Unknown,
                };
                Some(Place::Value {
                    ty,
                    writable: Err(format!("'selected' of {} '{name}' can only be read", part.keyword())),
                })
            }
            _ => None,
        }
    }

    fn member(&mut self, ty: Ty<'a>, member: &str, span: Span) -> Option<Place<'a>> {
        match ty {
            Ty::Unknown => Some(Place::Value {
                ty: Ty::Unknown,
                writable: Ok(()),
            }),
            Ty::Record(entity) => match entity.field(member) {
                Some(field) => Some(Place::Value {
                    ty: self.env.field_type(&field.ty),
                    writable: Ok(()),
                }),
                None => {
                    self.error(format!("{} has no field '{member}'", entity.name.name), span);
                    None
                }
            },
            other => {
                self.error(format!("{} has no member '{member}'", other.name()), span);
                None
            }
        }
    }

    /// The type of `expr`. `expected` lets a bare enum value (`Business`)
    /// find its enum.
    fn expr(&mut self, expr: &Expr, expected: Option<Ty<'a>>) -> Ty<'a> {
        let ty = self.expr_type(expr, expected);
        self.out.types.types.insert((expr.span.start, expr.span.end), ty.info());
        ty
    }

    fn name_is(&mut self, expr: &Expr, kind: NameKind) {
        self.out.types.names.insert((expr.span.start, expr.span.end), kind);
    }

    fn expr_type(&mut self, expr: &Expr, expected: Option<Ty<'a>>) -> Ty<'a> {
        match &expr.kind {
            ExprKind::Number { value } => {
                if value.contains('.') {
                    Ty::Decimal
                } else {
                    Ty::Integer
                }
            }
            ExprKind::Str { .. } => Ty::Text,
            ExprKind::Bool { .. } => Ty::Bool,
            ExprKind::Name { name } => {
                if let Some(Ty::Enum(decl)) = expected
                    && !self.in_scope(name)
                {
                    if decl.variants.iter().any(|v| v.name.name == *name) {
                        self.name_is(expr, NameKind::EnumValue(decl.name.name.clone()));
                        return Ty::Enum(decl);
                    }
                    self.error(format!("{} has no value '{name}'", decl.name.name), expr.span);
                    return Ty::Unknown;
                }
                self.value_of(expr)
            }
            ExprKind::Member { .. } => self.value_of(expr),
            ExprKind::Unary { op, operand } => {
                let ty = self.expr(operand, None);
                let (ok, needs, result) = match op {
                    UnaryOp::Not => (Ty::Bool.accepts(&ty), "bool", Ty::Bool),
                    UnaryOp::Neg => (ty.is_unknown() || ty.is_numeric(), "a number", ty),
                };
                if !ok {
                    let word = if *op == UnaryOp::Not { "'not'" } else { "'-'" };
                    self.error(format!("{word} needs {needs}, not {}", ty.name()), operand.span);
                    return Ty::Unknown;
                }
                result
            }
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right),
            ExprKind::In { value, options } => {
                let ty = self.typed_pair(value, options.first());
                for option in options {
                    let option_ty = self.expr(option, Some(ty));
                    if !ty.comparable(&option_ty) {
                        self.error(format!("cannot compare {} with {}", ty.name(), option_ty.name()), option.span);
                    }
                }
                Ty::Bool
            }
            ExprKind::Call { .. } => self.call(expr, false).unwrap_or(Ty::Unknown),
            ExprKind::Find { var, entity, filter, order } => self.find(var, entity, filter.as_deref(), order),
        }
    }

    /// Checks a call. Returns its result type; None when it returns
    /// nothing (reported unless the call is a statement of its own).
    fn call(&mut self, expr: &Expr, statement: bool) -> Option<Ty<'a>> {
        let ExprKind::Call { function, args } = &expr.kind else {
            return Some(self.expr(expr, None));
        };
        if function.name == "count" {
            let tys: Vec<Ty> = args.iter().map(|a| self.expr(a, None)).collect();
            match tys.as_slice() {
                [Ty::List(_) | Ty::Unknown] => {}
                [other] => self.error(format!("'count' needs a list, not {}", other.name()), args[0].span),
                _ => self.error("'count' takes one list", expr.span),
            }
            return Some(Ty::Integer);
        }
        // A handler runs another handler of its window by name.
        if self.decl.is_none() && self.handlers.iter().any(|h| h.name.name == function.name) {
            self.name_is(expr, NameKind::Handler);
            let name = &function.name;
            for arg in args {
                self.expr(arg, None);
            }
            if !args.is_empty() {
                self.error(format!("handler '{name}' takes no arguments"), expr.span);
            }
            if !statement {
                self.error(format!("handler '{name}' does not return a value"), expr.span);
            }
            return None;
        }
        let Some(callee) = self.env.function(&function.name) else {
            for arg in args {
                self.expr(arg, None);
            }
            self.error(format!("unknown function '{}'", function.name), function.span);
            return Some(Ty::Unknown);
        };
        self.check_callee(callee, function.span);
        if args.len() != callee.params.len() {
            let n = callee.params.len();
            self.error(
                format!(
                    "{} takes {n} argument{}, not {}",
                    callee.name.name,
                    if n == 1 { "" } else { "s" },
                    args.len()
                ),
                expr.span,
            );
            for arg in args {
                self.expr(arg, None);
            }
        } else {
            for (arg, param) in args.iter().zip(&callee.params) {
                let expected = self.env.type_ref(&param.ty).unwrap_or(Ty::Unknown);
                let ty = self.expr(arg, Some(expected));
                if !expected.accepts(&ty) {
                    self.error(
                        format!(
                            "argument '{}' of {} is {} {}, not {}",
                            param.name.name,
                            callee.name.name,
                            article(&expected.name()),
                            expected.name(),
                            ty.name()
                        ),
                        arg.span,
                    );
                }
            }
        }
        match &callee.returns {
            Some(r) => Some(self.env.type_ref(r).unwrap_or(Ty::Unknown)),
            None => {
                if !statement {
                    self.error(format!("{} does not return a value", callee.name.name), expr.span);
                }
                None
            }
        }
    }

    /// A function may not call actions; an action may call actions whose
    /// effects it declares too.
    fn check_callee(&mut self, callee: &FunctionDecl, span: Span) {
        let Some(caller) = self.decl else { return };
        if callee.kind != FunctionKind::Action {
            return;
        }
        match caller.kind {
            FunctionKind::Function => self.error(
                format!("function '{}' cannot call action '{}'", caller.name.name, callee.name.name),
                span,
            ),
            FunctionKind::Action => {
                let missing = callee
                    .effects
                    .iter()
                    .find(|e| !caller.effects.iter().any(|c| c.name == e.name));
                if let Some(effect) = missing {
                    self.error(
                        format!(
                            "action '{}' calls '{}', which needs 'effects {}'",
                            caller.name.name, callee.name.name, effect.name
                        ),
                        span,
                    );
                }
            }
        }
    }

    /// `find c in Customer where ... order by ...`: a list of the entity.
    /// `c` is a local of the condition and the order only.
    fn find(&mut self, var: &Ident, entity: &Ident, filter: Option<&Expr>, order: &[OrderKey]) -> Ty<'a> {
        let Some(decl) = self.env.entity(&entity.name) else {
            self.error(format!("unknown entity '{}'", entity.name), entity.span);
            return Ty::Unknown;
        };
        self.scopes.push(HashMap::new());
        self.define(var, Ty::Record(decl));
        if let Some(filter) = filter {
            self.query_condition(filter, &var.name);
        }
        for key in order {
            if key.var.name != var.name {
                self.error(
                    format!("order by a field of {0}, as in '{0}.name'", var.name),
                    key.var.span,
                );
                continue;
            }
            match decl.field(&key.field.name) {
                None => self.error(
                    format!("{} has no field '{}'", decl.name.name, key.field.name),
                    key.field.span,
                ),
                Some(field) if matches!(field.ty, FieldType::List { .. }) => self.error(
                    format!("cannot order by '{}', a list", key.field.name),
                    key.field.span,
                ),
                Some(_) => {}
            }
        }
        self.scopes.pop();
        Ty::List(decl)
    }

    /// A query condition: `and`, `or` and `not` of comparisons between a
    /// field of the record and a value, `contains`, `in`, and bool fields.
    /// Each becomes a condition the database evaluates.
    fn query_condition(&mut self, expr: &Expr, var: &str) {
        let general = format!("a query compares a field of {var} with a value, as in '{var}.name == x'");
        match &expr.kind {
            ExprKind::Binary { op: BinaryOp::And | BinaryOp::Or, left, right } => {
                self.query_condition(left, var);
                self.query_condition(right, var);
            }
            ExprKind::Unary { op: UnaryOp::Not, operand } => self.query_condition(operand, var),
            ExprKind::Binary { op, left, right } if is_comparison(*op) => {
                let sides = (self.query_field(left, var), self.query_field(right, var));
                let (field, value) = match sides {
                    (Some(Ok(())), None) => (left, right),
                    (None, Some(Ok(()))) => (right, left),
                    (Some(Err(message)), _) | (_, Some(Err(message))) => return self.error(message, expr.span),
                    _ => return self.error(general, expr.span),
                };
                let field_ty = self.expr(field, None);
                // A reference is compared with a record of its entity.
                if let Ty::Record(entity) = field_ty
                    && matches!(op, BinaryOp::Eq | BinaryOp::NotEq)
                {
                    let value_ty = self.expr(value, Some(field_ty));
                    if !matches!(value_ty, Ty::Unknown) && !Ty::Record(entity).same(&value_ty) {
                        self.error(
                            format!("cannot compare {} with {}", field_ty.name(), value_ty.name()),
                            expr.span,
                        );
                    }
                } else {
                    // The usual rules, with the field on the left.
                    self.binary(*op, field, value);
                }
                self.out.types.types.insert((expr.span.start, expr.span.end), TypeInfo::Bool);
            }
            ExprKind::In { value, options } => {
                match self.query_field(value, var) {
                    Some(Ok(())) => {}
                    Some(Err(message)) => return self.error(message, expr.span),
                    None => return self.error(general, expr.span),
                }
                if options.iter().any(|o| self.query_field(o, var).is_some()) {
                    return self.error(general, expr.span);
                }
                self.expr(expr, Some(Ty::Bool));
            }
            _ => match self.query_field(expr, var) {
                Some(Ok(())) => self.condition(expr),
                Some(Err(message)) => self.error(message, expr.span),
                None => self.error(general, expr.span),
            },
        }
    }

    /// Whether `expr` uses the query's record: `None` when it does not,
    /// `Ok` when it is one of its fields (`c.name`), and the reason when it
    /// uses the record some other way.
    fn query_field(&self, expr: &Expr, var: &str) -> Option<Result<(), String>> {
        if !mentions(expr, var) {
            return None;
        }
        let ExprKind::Member { object, .. } = &expr.kind else {
            return Some(Err(format!(
                "a query compares a field of {var} with a value, as in '{var}.name == x'"
            )));
        };
        match &object.kind {
            ExprKind::Name { name } if name == var => Some(Ok(())),
            _ => Some(Err(format!(
                "a query compares a field of {var} itself; '{}' goes through a reference",
                path_text(expr)
            ))),
        }
    }

    fn in_scope(&self, name: &str) -> bool {
        self.local(name).is_some() || self.data(name).is_some() || self.part(name).is_some()
    }

    /// The value a name or path reads.
    fn value_of(&mut self, expr: &Expr) -> Ty<'a> {
        match self.place(expr) {
            Some(Place::Value { ty, .. }) => ty,
            Some(Place::Part(part, name)) => {
                self.error(format!("{} '{name}' is a part, not a value", part.keyword()), expr.span);
                Ty::Unknown
            }
            Some(Place::Enum(decl)) => {
                self.error(format!("{} is an enum, not a value", decl.name.name), expr.span);
                Ty::Unknown
            }
            None => Ty::Unknown,
        }
    }

    /// Checks `left` and gives it the type of `right` when left is a bare
    /// enum value; returns the type of left.
    fn typed_pair(&mut self, left: &Expr, right: Option<&Expr>) -> Ty<'a> {
        let bare = matches!(&left.kind, ExprKind::Name { name } if !self.in_scope(name) && self.env.enum_decl(name).is_none());
        match (bare, right) {
            (true, Some(right)) => {
                let right_ty = self.expr_quiet(right);
                self.expr(left, Some(right_ty))
            }
            _ => self.expr(left, None),
        }
    }

    /// The type of an expression without reporting anything, to give a
    /// bare enum value on the other side its type.
    fn expr_quiet(&mut self, expr: &Expr) -> Ty<'a> {
        let before = self.out.len();
        let ty = self.expr(expr, None);
        self.out.truncate(before);
        ty
    }

    fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr) -> Ty<'a> {
        let symbol = op.symbol();
        match op {
            BinaryOp::Contains => {
                let l = self.expr(left, None);
                let r = self.expr(right, None);
                let text = |t: &Ty| t.is_unknown() || matches!(t, Ty::Text);
                if !(text(&l) && text(&r)) {
                    self.error(
                        format!("'contains' needs text, not {} and {}", l.name(), r.name()),
                        join(left.span, right.span),
                    );
                }
                Ty::Bool
            }
            BinaryOp::And | BinaryOp::Or => {
                for side in [left, right] {
                    let ty = self.expr(side, Some(Ty::Bool));
                    if !Ty::Bool.accepts(&ty) {
                        self.error(format!("'{symbol}' needs bool, not {}", ty.name()), side.span);
                    }
                }
                Ty::Bool
            }
            BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                let l = self.typed_pair(left, Some(right));
                let r = self.expr(right, Some(l));
                let ordered = !matches!(op, BinaryOp::Eq | BinaryOp::NotEq);
                let fits = l.comparable(&r)
                    && l.is_value()
                    && (!ordered || l.is_unknown() || matches!(l, Ty::Text | Ty::Date | Ty::DateTime) || l.is_numeric());
                if !fits {
                    self.error(format!("cannot compare {} with {}", l.name(), r.name()), join(left.span, right.span));
                }
                Ty::Bool
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                let l = self.expr(left, None);
                let r = self.expr(right, None);
                if l.is_unknown() || r.is_unknown() {
                    return Ty::Unknown;
                }
                // `+` joins text with any single value.
                if op == BinaryOp::Add && (matches!(l, Ty::Text) || matches!(r, Ty::Text)) {
                    if l.is_value() && r.is_value() {
                        return Ty::Text;
                    }
                    self.error(format!("cannot join {} with {}", l.name(), r.name()), join(left.span, right.span));
                    return Ty::Unknown;
                }
                if !(l.is_numeric() && r.is_numeric()) {
                    self.error(
                        format!("'{symbol}' needs numbers, not {} and {}", l.name(), r.name()),
                        join(left.span, right.span),
                    );
                    return Ty::Unknown;
                }
                match (l, r) {
                    (Ty::Integer, Ty::Integer) if op != BinaryOp::Div => Ty::Integer,
                    (Ty::Money, _) | (_, Ty::Money) => Ty::Money,
                    _ => Ty::Decimal,
                }
            }
        }
    }
}

fn join(a: Span, b: Span) -> Span {
    Span { end: b.end, ..a }
}

fn article(word: &str) -> &'static str {
    match word.chars().next() {
        Some(c) if "aeiouAEIOU".contains(c) => "an",
        _ => "a",
    }
}

/// `==`, `!=`, `<`, `<=`, `>`, `>=` and `contains`.
fn is_comparison(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Gt
            | BinaryOp::GtEq
            | BinaryOp::Contains
    )
}

/// Whether `expr` uses the name `var` anywhere.
fn mentions(expr: &Expr, var: &str) -> bool {
    match &expr.kind {
        ExprKind::Name { name } => name == var,
        ExprKind::Member { object, .. } => mentions(object, var),
        ExprKind::Unary { operand, .. } => mentions(operand, var),
        ExprKind::Binary { left, right, .. } => mentions(left, var) || mentions(right, var),
        ExprKind::In { value, options } => mentions(value, var) || options.iter().any(|o| mentions(o, var)),
        ExprKind::Call { args, .. } => args.iter().any(|a| mentions(a, var)),
        // An inner query has its own record name.
        ExprKind::Find { var: inner, filter, .. } => {
            inner.name != var && filter.as_deref().is_some_and(|f| mentions(f, var))
        }
        ExprKind::Number { .. } | ExprKind::Str { .. } | ExprKind::Bool { .. } => false,
    }
}

/// A member path as written: `o.customer.name`.
fn path_text(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Name { name } => name.clone(),
        ExprKind::Member { object, member } => format!("{}.{}", path_text(object), member.name),
        _ => "...".to_string(),
    }
}

/// Text inputs that hold their own text when they are not bound.
fn has_text(props: &WidgetProps) -> bool {
    matches!(props.kind, WidgetKind::Textbox | WidgetKind::Memo | WidgetKind::Password)
}

/// Records of a read-only data source are never saved or deleted.
pub(crate) fn check_readonly(project: &mut crate::model::Project) {
    let readonly: Vec<&str> = project
        .entities()
        .map(|e| e.name.name.as_str())
        .filter(|e| {
            let source = project.source_of(e).unwrap_or_default();
            project.sources.iter().any(|s| s.name == source && s.readonly)
        })
        .collect();
    if readonly.is_empty() {
        return;
    }
    let types = types(&project.files);
    let mut found = Vec::new();
    for file in &project.files {
        let Some(file_types) = types.get(&file.path) else { continue };
        let bodies = file
            .code
            .iter()
            .flat_map(|b| b.handlers.iter().map(|h| &h.body))
            .chain(file.functions.iter().map(|f| &f.body));
        for body in bodies {
            writes(body, &mut |stmt, value, word| {
                let TypeInfo::Record(entity) = file_types.type_of(value) else { return };
                if readonly.contains(&entity.as_str()) {
                    let source = project.source_of(&entity).unwrap_or_default();
                    found.push(
                        Diagnostic::error(
                            format!("{entity} is in data source {source}, which is read-only; its records cannot be {word}"),
                            stmt.span,
                        )
                        .in_file(&file.path),
                    );
                }
            });
        }
    }
    project.diagnostics.extend(found);
}

/// Calls `found` for every `save` and `delete` in `body`, nested blocks
/// included, with the value and "saved" or "deleted".
fn writes(body: &[Stmt], found: &mut impl FnMut(&Stmt, &Expr, &str)) {
    for stmt in body {
        match &stmt.kind {
            StmtKind::Save { value } => found(stmt, value, "saved"),
            StmtKind::Delete { value } => found(stmt, value, "deleted"),
            StmtKind::If { branches, otherwise } => {
                for branch in branches {
                    writes(&branch.body, found);
                }
                if let Some(otherwise) = otherwise {
                    writes(otherwise, found);
                }
            }
            StmtKind::For { body, .. } => writes(body, found),
            _ => {}
        }
    }
}
