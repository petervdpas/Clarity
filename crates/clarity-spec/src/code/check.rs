//! Checks code blocks against the window or app they belong to: every name
//! resolves (locals, the window's data, its parts, enum values), every
//! expression has a type that fits where it is used, and the handlers
//! written match the handlers wired to events. Functions and actions in
//! modules are checked the same way, with their own rules for effects.

use std::collections::HashMap;

use super::{AskForm, CommandDecl, EngineBody, EngineState, Param, PartDecl,
    BinaryOp, CodeBlock, DataEvent, EFFECTS, EngineDecl, Expr, TextPart, ExprKind, FunctionDecl, FunctionKind, HandlerDecl, OrderKey, Stmt, StmtKind,
    TypeRef, TypeRefKind, UnaryOp,
};
use crate::diagnostic::Diagnostic;
use crate::model::{
    Command, CommandKind, CommandProps, DataDecl, DataKind, Entity, EnumDecl, FieldType, Frame, Ident, ListItem, Literal, Page, SpecFile,
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
    diagnostics.extend(check_command_lines(files));
    (diagnostics, types)
}

/// Every command line names one command, a group is no command of its
/// own, and a project has one `cli` block (DESIGN 5.20).
fn check_command_lines(files: &[SpecFile]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let commands: Vec<(&SpecFile, &CommandDecl)> = files.iter().flat_map(|f| f.commands.iter().map(move |c| (f, c))).collect();
    for (i, (file, command)) in commands.iter().enumerate() {
        let line = command.line();
        if commands[..i].iter().any(|(_, c)| c.line() == line) {
            out.push(Diagnostic::error(format!("there are two commands '{line}'"), command.name.span).in_file(&file.path));
        } else if command.group.is_none() && commands.iter().any(|(_, c)| c.group.as_ref().is_some_and(|g| crate::code::hyphenated(&g.name) == line)) {
            out.push(Diagnostic::error(format!("'{line}' is a group of commands, so it is no command of its own"), command.name.span).in_file(&file.path));
        }
    }
    let clis: Vec<(&SpecFile, &crate::code::CliDecl)> = files.iter().flat_map(|f| f.clis.iter().map(move |c| (f, c))).collect();
    for (file, cli) in clis.iter().skip(1) {
        out.push(Diagnostic::error("a project has one 'cli' block", cli.name.span).in_file(&file.path));
    }
    out
}

/// A command's arguments and options (plain values, each name once, with
/// defaults that fit), its help, and its statements.
/// `(points: integer)`, for messages.
fn signature(env: &Env, params: &[Param]) -> String {
    let each: Vec<String> = params
        .iter()
        .map(|p| format!("{}: {}", p.name.name, env.type_ref(&p.ty).map_or_else(|_| "?".into(), |t| t.name())))
        .collect();
    format!("({})", each.join(", "))
}

/// Whether a type is one a custom part's property or value can be:
/// plain values, which every library takes as a number or as text.
fn plain_for_part(ty: &Ty) -> bool {
    matches!(ty, Ty::Text | Ty::Integer | Ty::Decimal | Ty::Money | Ty::Bool | Ty::Date | Ty::DateTime | Ty::Duration | Ty::Enum(_) | Ty::Unknown)
}

const PART_TYPES: &str = "a property is text, a number, a date, a duration, an enum or bool";

/// A number as written in Clarity (`10`, `-3`), for a bound.
fn written_number(expr: &Expr) -> Option<f64> {
    match &expr.kind {
        ExprKind::Number { value } => value.parse().ok(),
        ExprKind::Unary { op: UnaryOp::Neg, operand } => written_number(operand).map(|n| -n),
        _ => None,
    }
}

/// A custom part's declaration (DESIGN 5.19): its library, one name per
/// property, tell and hear, none of them a word every part has, plain types
/// and defaults and bounds that fit.
fn check_parts(env: &Env, file: &SpecFile, out: &mut Output) {
    for part in &file.parts {
        let name = &part.name.name;
        if part.library.is_none() {
            out.push(Diagnostic::error(format!("part {name} needs its library: native \"folder\", nuget \"Package\" version or dll \"file\""), part.name.span));
        }
        let mut seen: Vec<&str> = Vec::new();
        let names = part.properties.iter().map(|p| &p.name).chain(part.tells.iter().map(|t| &t.name)).chain(part.hears.iter().map(|h| &h.name));
        for own in names {
            if seen.contains(&own.name.as_str()) {
                out.push(Diagnostic::error(format!("part {name} has two properties, tells or hears named '{}'", own.name), own.span));
            }
            seen.push(&own.name);
        }
        if let Some(binds) = &part.binds {
            match env.type_ref(binds) {
                Ok(ty) if !plain_for_part(&ty) => out.push(Diagnostic::error(format!("a part binds text, a number, a date, a duration, an enum or bool, not {}", ty.name()), binds.span)),
                Ok(_) => {}
                Err(message) => out.push(Diagnostic::error(message, binds.span)),
            }
        }
        for message in part.tells.iter().chain(&part.hears) {
            for param in &message.params {
                match env.type_ref(&param.ty) {
                    Ok(ty) if !plain_for_part(&ty) => out.push(Diagnostic::error(format!("a value of a tell or hear is text, a number, a date, a duration, an enum or bool, not {}", ty.name()), param.ty.span)),
                    Ok(_) => {}
                    Err(problem) => out.push(Diagnostic::error(problem, param.ty.span)),
                }
            }
        }
        if let Some((span, rows)) = part.size
            && (span == 0 || rows == 0)
        {
            out.push(Diagnostic::error("a part spans at least 1 column and 1 row", part.span));
        }
        let mut checker = BodyChecker {
            env,
            owner: None,
            handlers: &[],
            decl: None,
            handler: None,
            event: None,
            returns: None,
            scopes: vec![HashMap::new()],
            saves_reported: false,
            engine: None,
            command: None,
            out: &mut *out,
        };
        for property in &part.properties {
            let own = &property.name.name;
            if widgets::CUSTOM_PART_WORDS.contains(&own.as_str()) {
                checker.error(format!("'{own}' is a word every part has; name the property otherwise"), property.name.span);
            }
            let ty = match env.type_ref(&property.ty) {
                Ok(ty) => ty,
                Err(message) => {
                    checker.error(message, property.ty.span);
                    continue;
                }
            };
            if !plain_for_part(&ty) {
                checker.error(format!("{PART_TYPES}, not {}", ty.name()), property.ty.span);
                continue;
            }
            if let Some(default) = &property.default {
                let given = checker.expr(default, Some(ty));
                if !given.is_unknown() && !ty.accepts(&given) {
                    checker.error(format!("the default of {own} is {}, but {own} is {}", given.name(), ty.name()), default.span);
                }
            }
            let bounds: Vec<&Expr> = property.min.iter().chain(&property.max).collect();
            if !bounds.is_empty() && !ty.is_numeric() {
                checker.error(format!("'min' and 'max' bound a number; {own} is {}", ty.name()), bounds[0].span);
            } else if bounds.iter().any(|b| written_number(b).is_none()) {
                checker.error(format!("'min' and 'max' of {own} are numbers, as in min 1"), bounds[0].span);
            }
        }
    }
}

/// A custom part placed in a window: declared, its values declared and
/// fitting, its tells declared, and what it binds of its type.
fn check_placed_parts(env: &Env, file: &SpecFile, out: &mut Output) {
    fn walk<'w>(widgets: &'w [Widget], found: &mut Vec<&'w Widget>) {
        for widget in widgets {
            if widget.props.kind == WidgetKind::Custom {
                found.push(widget);
            }
            walk(&widget.children, found);
            for page in &widget.pages {
                walk(&page.children, found);
            }
        }
    }
    for page in &file.pages {
        let mut placed = Vec::new();
        walk(&page.widgets, &mut placed);
        for sidebar in &page.frame.sidebars {
            walk(&sidebar.widgets, &mut placed);
        }
        for widget in placed {
            let props = &widget.props;
            let written = props.part.as_deref().unwrap_or_default();
            let Some(decl) = env.part_decl(written) else {
                out.push(Diagnostic::error(format!("unknown part '{written}'"), widget.span));
                continue;
            };
            let name = &decl.name.name;
            for (own, value) in &props.values {
                let Some(property) = decl.properties.iter().find(|p| p.name.name == *own) else {
                    out.push(Diagnostic::error(format!("part {name} has no property '{own}'"), widget.span));
                    continue;
                };
                let Ok(ty) = env.type_ref(&property.ty) else { continue };
                if let Some(problem) = misfit(value, &ty, own) {
                    out.push(Diagnostic::error(format!("{name}: {problem}"), widget.span));
                    continue;
                }
                let Literal::Number(written) = value else { continue };
                let Ok(number) = written.parse::<f64>() else { continue };
                if let Some(min) = property.min.as_ref().and_then(written_number)
                    && number < min
                {
                    out.push(Diagnostic::error(format!("{own} is at least {}", trimmed(min)), widget.span));
                }
                if let Some(max) = property.max.as_ref().and_then(written_number)
                    && number > max
                {
                    out.push(Diagnostic::error(format!("{own} is at most {}", trimmed(max)), widget.span));
                }
            }
            for tell in &props.tells {
                if !decl.tells.iter().any(|t| t.name.name == tell.tell) {
                    out.push(Diagnostic::error(format!("part {name} tells no '{}'", tell.tell), widget.span));
                }
            }
            let Some(bind) = &props.bind else { continue };
            let Some(binds) = &decl.binds else {
                out.push(Diagnostic::error(format!("part {name} binds nothing; leave out 'bind'"), widget.span));
                continue;
            };
            let Ok(wanted) = env.type_ref(binds) else { continue };
            let (data, field) = match bind.split_once('.') {
                Some((d, f)) => (d, Some(f)),
                None => (bind.as_str(), None),
            };
            let Some(decl_data) = page.data.iter().find(|d| d.name.name == data) else { continue };
            let bound = match (env.data_type(decl_data), field) {
                (Ty::Record(entity), Some(field)) => match entity.field(field) {
                    Some(f) => env.field_type(&f.ty),
                    None => continue,
                },
                (ty, None) => ty,
                _ => continue,
            };
            if !bound.is_unknown() && !wanted.accepts(&bound) {
                out.push(Diagnostic::error(format!("part {name} binds {}; '{bind}' is {}", wanted.name(), bound.name()), widget.span));
            }
        }
    }
}

/// `10` for 10.0, as a bound is written.
fn trimmed(n: f64) -> String {
    if n.fract() == 0.0 { format!("{}", n as i64) } else { n.to_string() }
}

fn check_command(env: &Env, command: &CommandDecl, out: &mut Output) {
    let line = command.line();
    let mut names: Vec<String> = Vec::new();
    let mut typed = |name: &Ident, ty: &TypeRef, option: bool, out: &mut Output| -> Ty {
        if names.contains(&name.name) {
            out.push(Diagnostic::error(format!("command {line} has two arguments or options named '{}'", name.name), name.span));
        }
        names.push(name.name.clone());
        let ty = env.type_ref(ty).unwrap_or_else(|message| {
            out.push(Diagnostic::error(message, ty.span));
            Ty::Unknown
        });
        let plain = matches!(ty, Ty::Text | Ty::Integer | Ty::Decimal | Ty::Money | Ty::Date | Ty::DateTime | Ty::Duration | Ty::Enum(_) | Ty::Unknown);
        if matches!(ty, Ty::Bool) && !option {
            out.push(Diagnostic::error(format!("a yes or no is an option, not an argument: option {}: bool", name.name), name.span));
        } else if !plain && !(option && matches!(ty, Ty::Bool)) {
            let what = if option { "an option is text, a number, a date, a duration, an enum or bool" } else { "an argument is text, a number, a date, a duration or an enum" };
            out.push(Diagnostic::error(format!("{what}, not {}; a command looks records up with find", ty.name()), name.span));
        }
        ty
    };
    let params: Vec<(String, Ty)> = command.params.iter().map(|p| (p.name.name.clone(), typed(&p.name, &p.ty, false, out))).collect();
    let options: Vec<Ty> = command.options.iter().map(|o| typed(&o.name, &o.ty, true, out)).collect();
    if command.help.is_none() {
        out.push(Diagnostic::warning(format!("command {line} has no help; write what it does: help @key"), command.name.span));
    }
    let mut checker = BodyChecker {
        env,
        owner: None,
        handlers: &[],
        decl: None,
        handler: None,
        event: None,
        returns: None,
        scopes: vec![HashMap::new()],
        saves_reported: false,
        engine: None,
        command: Some(command),
        out,
    };
    if let Some(help) = &command.help {
        checker.shown_text(help);
    }
    for (option, ty) in command.options.iter().zip(&options) {
        let name = &option.name.name;
        if let Some(help) = &option.help {
            checker.shown_text(help);
        } else {
            checker.out.push(Diagnostic::warning(format!("option {name} of command {line} has no help; write what it does: help @key"), option.name.span));
        }
        if let Some(default) = &option.default {
            let given = checker.expr(default, Some(*ty));
            if !ty.is_unknown() && !given.is_unknown() && !ty.accepts(&given) {
                checker.error(format!("the default of {name} is {}, but {name} is {}", given.name(), ty.name()), default.span);
            }
            if option.required {
                checker.error(format!("{name} is required, so it has no default"), default.span);
            }
        }
        let bounds: Vec<&Expr> = option.min.iter().chain(&option.max).collect();
        if !bounds.is_empty() && !ty.is_numeric() && !ty.is_unknown() {
            checker.error(format!("'min' and 'max' bound a number; {name} is {}", ty.name()), bounds[0].span);
        } else {
            for bound in bounds {
                let given = checker.expr(bound, Some(*ty));
                if !given.is_unknown() && !given.is_numeric() {
                    checker.error(format!("'min' and 'max' of {name} are numbers, not {}", given.name()), bound.span);
                }
            }
        }
    }
    let scope_of = &mut checker.scopes[0];
    for (name, ty) in params {
        scope_of.insert(name, ty);
    }
    for (option, ty) in command.options.iter().zip(options) {
        scope_of.insert(option.name.name.clone(), ty);
    }
    checker.block(&command.body);
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
    Duration,
    /// A picture of an image field.
    Image,
    /// The file of a `file` field: its name and extension are read.
    RecordFile,
    /// A file of a file store, and a list of them.
    File,
    Files,
    /// The contents of a file as they are.
    Bytes,
    Enum(String),
    Record(String),
    List(String),
    /// A list of plain values: `list<integer>`.
    Values(ValueItem),
    Unknown,
}

/// What a list of plain values holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueItem {
    Text,
    Integer,
    Decimal,
    Bool,
    Date,
    /// The values of an enum, by its name: a field `list<Genre>`.
    Enum(String),
}

impl ValueItem {
    /// The item type as Clarity writes it: `integer`, `Genre`.
    pub fn word(&self) -> &str {
        match self {
            ValueItem::Text => "text",
            ValueItem::Integer => "integer",
            ValueItem::Decimal => "decimal",
            ValueItem::Bool => "bool",
            ValueItem::Date => "date",
            ValueItem::Enum(name) => name,
        }
    }

    /// The item type of `list<word>`, when it is a plain value.
    pub fn of_word(word: &str) -> Option<ValueItem> {
        match word {
            "text" => Some(ValueItem::Text),
            "integer" => Some(ValueItem::Integer),
            "decimal" => Some(ValueItem::Decimal),
            "bool" => Some(ValueItem::Bool),
            "date" => Some(ValueItem::Date),
            _ => None,
        }
    }

    /// What a list field holds, as code sees it.
    pub fn of_list(item: &ListItem) -> ValueItem {
        match item {
            ListItem::Text => ValueItem::Text,
            ListItem::Integer => ValueItem::Integer,
            ListItem::Decimal => ValueItem::Decimal,
            ListItem::Date => ValueItem::Date,
            ListItem::Enum { name } => ValueItem::Enum(name.name.clone()),
        }
    }

    /// The type of one item.
    pub fn info(&self) -> TypeInfo {
        match self {
            ValueItem::Text => TypeInfo::Text,
            ValueItem::Integer => TypeInfo::Integer,
            ValueItem::Decimal => TypeInfo::Decimal,
            ValueItem::Bool => TypeInfo::Bool,
            ValueItem::Date => TypeInfo::Date,
            ValueItem::Enum(name) => TypeInfo::Enum(name.clone()),
        }
    }
}

/// What a list of plain values holds, as the checker knows it.
#[derive(Debug, Clone, Copy)]
enum Item<'a> {
    Text,
    Integer,
    Decimal,
    Bool,
    Date,
    Enum(&'a EnumDecl),
}

impl Item<'_> {
    fn of_word(word: &str) -> Option<Self> {
        Some(match ValueItem::of_word(word)? {
            ValueItem::Text => Item::Text,
            ValueItem::Integer => Item::Integer,
            ValueItem::Decimal => Item::Decimal,
            ValueItem::Bool => Item::Bool,
            ValueItem::Date => Item::Date,
            ValueItem::Enum(_) => return None,
        })
    }

    fn info(self) -> ValueItem {
        match self {
            Item::Text => ValueItem::Text,
            Item::Integer => ValueItem::Integer,
            Item::Decimal => ValueItem::Decimal,
            Item::Bool => ValueItem::Bool,
            Item::Date => ValueItem::Date,
            Item::Enum(e) => ValueItem::Enum(e.name.name.clone()),
        }
    }

    fn word(self) -> String {
        self.info().word().to_string()
    }

    fn same(self, other: Item) -> bool {
        match (self, other) {
            (Item::Enum(a), Item::Enum(b)) => std::ptr::eq(a, b),
            (a, b) => std::mem::discriminant(&a) == std::mem::discriminant(&b),
        }
    }
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
    /// What an engine written in Clarity keeps between calls.
    Kept,
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
    /// The project's settings, seen from every module.
    settings: Option<&'a Entity>,
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
            settings: files.iter().flat_map(|f| &f.entities).find(|e| e.settings),
            earlier_functions,
        }
    }

    fn entity(&self, name: &str) -> Option<&'a Entity> {
        self.visible.iter().flat_map(|f| &f.entities).find(|e| e.name.name == name && !e.settings).or_else(|| self.settings.filter(|s| s.name.name == name))
    }

    fn enum_decl(&self, name: &str) -> Option<&'a EnumDecl> {
        self.visible.iter().flat_map(|f| &f.enums).find(|e| e.name.name == name)
    }

    fn page(&self, name: &str) -> Option<&'a Page> {
        self.visible.iter().flat_map(|f| &f.pages).find(|p| p.name.name == name)
    }

    /// A custom part declared in a visible module (DESIGN 5.19).
    fn part_decl(&self, name: &str) -> Option<&'a PartDecl> {
        self.visible.iter().flat_map(|f| &f.parts).find(|p| p.name.name == name)
    }

    fn engine(&self, name: &str) -> Option<&'a EngineDecl> {
        self.visible.iter().flat_map(|f| &f.engines).find(|e| e.name.name == name)
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
            TypeRefKind::Duration => Ty::Duration,
            TypeRefKind::File => Ty::File,
            TypeRefKind::Bytes => Ty::Bytes,
            TypeRefKind::List { item } if item == "file" => Ty::Files,
            TypeRefKind::List { item } if Item::of_word(item).is_some() => Ty::Values(Item::of_word(item).expect("a value item")),
            TypeRefKind::Named { name } => match (self.entity(name), self.enum_decl(name)) {
                (Some(entity), _) => Ty::Record(entity),
                (None, Some(decl)) => Ty::Enum(decl),
                (None, None) => return Err(format!("unknown type '{name}'")),
            },
            TypeRefKind::List { item } => match (self.entity(item), self.enum_decl(item)) {
                (Some(entity), _) => Ty::List(entity),
                (None, Some(decl)) => Ty::Values(Item::Enum(decl)),
                (None, None) => return Err(format!("unknown entity '{item}'")),
            },
        })
    }

    fn data_type(&self, decl: &DataDecl) -> Ty<'a> {
        match &decl.of {
            DataKind::Value { ty, .. } => self.type_ref(ty).unwrap_or(Ty::Unknown),
            DataKind::List { entity } => self.entity(&entity.name).map_or(Ty::Unknown, Ty::List),
            DataKind::Record { entity } => self.entity(&entity.name).map_or(Ty::Unknown, Ty::Record),
        }
    }

    fn field_type(&self, ty: &FieldType) -> Ty<'a> {
        match ty {
            FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme => Ty::Text,
            FieldType::Integer | FieldType::Id => Ty::Integer,
            FieldType::Decimal { .. } => Ty::Decimal,
            FieldType::Money => Ty::Money,
            FieldType::Bool => Ty::Bool,
            FieldType::Date => Ty::Date,
            FieldType::DateTime => Ty::DateTime,
            FieldType::Duration => Ty::Duration,
            FieldType::Image => Ty::Image,
            FieldType::File => Ty::RecordFile,
            FieldType::Enum { name } => self.enum_decl(&name.name).map_or(Ty::Unknown, Ty::Enum),
            FieldType::Ref { entity } => self.entity(&entity.name).map_or(Ty::Unknown, Ty::Record),
            FieldType::List { item } => self.entity(&item.name).map_or(Ty::Unknown, Ty::List),
            FieldType::Values { item } => match item {
                ListItem::Text => Ty::Values(Item::Text),
                ListItem::Integer => Ty::Values(Item::Integer),
                ListItem::Decimal => Ty::Values(Item::Decimal),
                ListItem::Date => Ty::Values(Item::Date),
                ListItem::Enum { name } => self.enum_decl(&name.name).map_or(Ty::Unknown, |e| Ty::Values(Item::Enum(e))),
            },
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
    Duration,
    Image,
    /// The file of a `file` field.
    RecordFile,
    File,
    Files,
    Bytes,
    Enum(&'a EnumDecl),
    Record(&'a Entity),
    List(&'a Entity),
    /// A list of plain values.
    Values(Item<'a>),
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
            Ty::Duration => TypeInfo::Duration,
            Ty::Image => TypeInfo::Image,
            Ty::RecordFile => TypeInfo::RecordFile,
            Ty::File => TypeInfo::File,
            Ty::Bytes => TypeInfo::Bytes,
            Ty::Files => TypeInfo::Files,
            Ty::Enum(e) => TypeInfo::Enum(e.name.name.clone()),
            Ty::Record(e) => TypeInfo::Record(e.name.name.clone()),
            Ty::List(e) => TypeInfo::List(e.name.name.clone()),
            Ty::Values(item) => TypeInfo::Values(item.info()),
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
            Ty::Duration => "duration".into(),
            Ty::Image => "image".into(),
            Ty::RecordFile => "file".into(),
            Ty::File => "file".into(),
            Ty::Bytes => "bytes".into(),
            Ty::Files => "list<file>".into(),
            Ty::Enum(e) => e.name.name.clone(),
            Ty::Record(e) => e.name.name.clone(),
            Ty::List(e) => format!("list<{}>", e.name.name),
            Ty::Values(item) => format!("list<{}>", item.word()),
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
        !matches!(self, Ty::Record(_) | Ty::List(_) | Ty::Values(_) | Ty::Image | Ty::RecordFile | Ty::File | Ty::Files | Ty::Bytes)
    }

    fn same(&self, other: &Ty) -> bool {
        match (self, other) {
            (Ty::Enum(a), Ty::Enum(b)) => std::ptr::eq(*a, *b),
            (Ty::Record(a), Ty::Record(b)) | (Ty::List(a), Ty::List(b)) => std::ptr::eq(*a, *b),
            (Ty::Values(a), Ty::Values(b)) => a.same(*b),
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
            // A number of seconds.
            Ty::Duration => matches!(value, Ty::Integer),
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
    Sidebar(&'a crate::model::SidebarProps),
    Ribbon(&'a crate::model::Ribbon),
    /// A part code only hides and disables: a menu, a toolbar, or a ribbon's tab or group.
    Plain(&'static str),
}

impl Part<'_> {
    fn keyword(&self) -> &'static str {
        match self {
            Part::Widget(props) => widgets::keyword(props.kind),
            Part::Command(props) => match props.kind {
                CommandKind::Button => "button",
                _ => "item",
            },
            Part::Sidebar(_) => "sidebar",
            Part::Ribbon(_) => "ribbon",
            Part::Plain(kind) => kind,
        }
    }

    fn has_caption(&self) -> bool {
        match self {
            Part::Widget(props) => matches!(
                props.kind,
                WidgetKind::Label | WidgetKind::Help | WidgetKind::Heading | WidgetKind::Button
            ),
            Part::Command(_) => true,
            Part::Sidebar(_) | Part::Ribbon(_) | Part::Plain(_) => false,
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
    /// Whether it is the app, whose code runs in no window of its own.
    is_app: bool,
    data: &'a [DataDecl],
    parts: HashMap<&'a str, Part<'a>>,
    /// Handlers wired to events, with the span of the part that wires them.
    wired: Vec<Wired<'a>>,
}

/// A handler wired to an event: `on click: digit(7)`.
struct Wired<'a> {
    handler: &'a str,
    /// The values it is run with.
    args: &'a [Literal],
    /// The event's word, like `click`; a window's own events give no values.
    event: &'a str,
    own_event: bool,
    /// A custom part's tell: the part's declared name. The tell gives the handler its values.
    tell_of: Option<&'a str>,
    span: Span,
}

fn collect_widgets<'a>(widgets: &'a [Widget], owner: &mut Owner<'a>) {
    for widget in widgets {
        if let Some(name) = &widget.props.name {
            owner.parts.insert(name, Part::Widget(&widget.props));
        }
        for handler in &widget.props.handlers {
            owner.wired.push(Wired { handler: &handler.handler, args: &handler.args, event: handler.event.keyword(), own_event: false, tell_of: None, span: widget.span });
        }
        for tell in &widget.props.tells {
            owner.wired.push(Wired { handler: &tell.handler, args: &[], event: &tell.tell, own_event: false, tell_of: widget.props.part.as_deref(), span: widget.span });
        }
        // Its context menu: the menu and its items are parts, their clicks run handlers.
        if let Some(menu) = &widget.menu {
            if let Some(name) = &menu.name {
                owner.parts.insert(name, Part::Plain("menu"));
            }
            collect_commands(&menu.items, owner);
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
            owner.wired.push(Wired { handler, args: &[], event: "click", own_event: false, tell_of: None, span: command.span });
        }
        collect_commands(&command.items, owner);
    }
}

fn collect_frame<'a>(frame: &'a Frame, owner: &mut Owner<'a>) {
    if let Some(menu) = &frame.menu {
        if let Some(name) = &menu.name {
            owner.parts.insert(name, Part::Plain("menu"));
        }
        collect_commands(&menu.items, owner);
    }
    if let Some(toolbar) = &frame.toolbar {
        if let Some(name) = &toolbar.name {
            owner.parts.insert(name, Part::Plain("toolbar"));
        }
        collect_commands(&toolbar.items, owner);
    }
    if let Some(ribbon) = &frame.ribbon {
        if let Some(name) = &ribbon.name {
            owner.parts.insert(name, Part::Ribbon(ribbon));
        }
        for tab in &ribbon.tabs {
            if let Some(name) = &tab.name {
                owner.parts.insert(name, Part::Plain("tab"));
            }
            for group in &tab.groups {
                if let Some(name) = &group.name {
                    owner.parts.insert(name, Part::Plain("group"));
                }
                collect_commands(&group.items, owner);
            }
        }
    }
    for sidebar in &frame.sidebars {
        if let Some(name) = &sidebar.props.name {
            owner.parts.insert(name, Part::Sidebar(&sidebar.props));
        }
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
            is_app: false,
            data: &page.data,
            parts: HashMap::new(),
            wired: Vec::new(),
        };
        owner.wired.extend(page.handlers.iter().map(|h| Wired { handler: &h.handler, args: &h.args, event: h.event.keyword(), own_event: true, tell_of: None, span: page.span }));
        collect_widgets(&page.widgets, &mut owner);
        collect_frame(&page.frame, &mut owner);
        owners.push(owner);
    }
    for app in &file.apps {
        let mut owner = Owner {
            name: &app.name.name,
            is_app: true,
            data: &app.data,
            parts: HashMap::new(),
            wired: Vec::new(),
        };
        owner.wired.extend(app.handlers.iter().map(|h| Wired { handler: &h.handler, args: &h.args, event: h.event.keyword(), own_event: true, tell_of: None, span: app.span }));
        collect_frame(&app.frame, &mut owner);
        owners.push(owner);
    }
    owners
}

fn check_file(env: &Env, file: &SpecFile, out: &mut Output) {
    check_functions(env, file, out);
    for command in &file.commands {
        check_command(env, command, out);
    }
    check_engines(env, file, out);
    check_parts(env, file, out);
    check_placed_parts(env, file, out);
    let owners = owners(file);
    for owner in &owners {
        check_defaults(env, owner, out);
    }
    let mut seen: Vec<&str> = Vec::new();
    for block in &file.code {
        let name = block.name.name.as_str();
        if seen.contains(&name) {
            out.push(Diagnostic::error(format!("there is already a code block for {name}"), block.name.span));
            continue;
        }
        seen.push(name);
        if let Some(owner) = owners.iter().find(|o| o.name == name) {
            for event in &block.events {
                out.push(Diagnostic::error(
                    format!("'{}' belongs in the code of an entity; {name} is a window", event.event.text()),
                    event.head,
                ));
            }
            check_block(env, owner, block, out);
        } else if file.entities.iter().any(|e| e.name.name == name) {
            check_events(env, block, out);
        } else {
            out.push(Diagnostic::error(
                format!("there is no window, app or entity '{name}' in this file"),
                block.name.span,
            ));
        }
    }
    // Wired handlers that are not written yet, window by window.
    for owner in &owners {
        let block = file.code.iter().find(|b| b.name.name == owner.name);
        let mut reported: Vec<&str> = Vec::new();
        for wired in &owner.wired {
            match block.and_then(|b| b.handlers.iter().find(|h| h.name.name == wired.handler)) {
                Some(decl) => check_values(env, wired, decl, out),
                None if !reported.contains(&wired.handler) => {
                    reported.push(wired.handler);
                    out.push(Diagnostic::warning(
                        format!("handler '{}' is wired but not written yet", wired.handler),
                        wired.span,
                    ));
                }
                None => {}
            }
        }
    }
}

/// The values an event gives a handler fit its parameters: as many, each
/// of the parameter's type. A window's own events give none.
fn check_values(env: &Env, wired: &Wired, decl: &HandlerDecl, out: &mut Output) {
    let params = || {
        let each: Vec<String> = decl
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name.name, env.type_ref(&p.ty).map_or_else(|_| "?".into(), |t| t.name())))
            .collect();
        format!("({})", each.join(", "))
    };
    let call = format!(
        "on {}: {}",
        wired.event,
        crate::widgets::handler_call(&crate::model::Handler {
            event: crate::widgets::Event::Click,
            handler: wired.handler.to_string(),
            args: wired.args.to_vec(),
        })
    );
    // A tell gives its values: the handler takes them, each of its type.
    if let Some(part) = wired.tell_of {
        let Some(message) = env.part_decl(part).and_then(|p| p.tells.iter().find(|t| t.name.name == wired.event)) else { return };
        let fits = message.params.len() == decl.params.len()
            && message.params.iter().zip(&decl.params).all(|(given, taken)| match (env.type_ref(&given.ty), env.type_ref(&taken.ty)) {
                (Ok(given), Ok(taken)) => taken.accepts(&given),
                _ => true,
            });
        if !fits {
            out.push(Diagnostic::error(
                format!("tell {} of part {part} gives {}; handler {} takes {}", wired.event, signature(env, &message.params), wired.handler, params()),
                wired.span,
            ));
        }
        return;
    }
    if wired.own_event {
        if !decl.params.is_empty() {
            out.push(Diagnostic::error(
                format!("handler {} takes values {}; 'on {}' has none to give", wired.handler, params(), wired.event),
                wired.span,
            ));
        }
        return;
    }
    if wired.args.len() != decl.params.len() {
        out.push(Diagnostic::error(
            format!("'{call}' gives {} values; handler {} takes {} {}", wired.args.len(), wired.handler, decl.params.len(), params()),
            wired.span,
        ));
        return;
    }
    for (value, param) in wired.args.iter().zip(&decl.params) {
        let Ok(ty) = env.type_ref(&param.ty) else { continue };
        if let Some(problem) = misfit(value, &ty, &param.name.name) {
            out.push(Diagnostic::error(format!("'{call}': {problem}"), wired.span));
        }
    }
}

/// Why a value written in the design does not fit `ty`, said of `who`:
/// "d is an integer, not text"; None when it fits.
fn misfit(value: &Literal, ty: &Ty, who: &str) -> Option<String> {
    match (value, ty) {
        (_, Ty::Unknown) => None,
        (Literal::Number(n), Ty::Integer) if !n.contains('.') => None,
        (Literal::Number(_), Ty::Decimal | Ty::Money | Ty::Duration) => None,
        (Literal::Str(_), Ty::Text) => None,
        (Literal::Bool(_), Ty::Bool) => None,
        (Literal::Ident(v), Ty::Enum(e)) if e.variants.iter().any(|x| x.name.name == *v) => None,
        (Literal::Ident(v), Ty::Enum(e)) => Some(format!("{} has no value {v}", e.name.name)),
        (value, ty) => Some(format!(
            "{who} is {}, not {}",
            described(ty),
            match value {
                Literal::Number(n) => n.clone(),
                Literal::Str(_) => "text".into(),
                Literal::Bool(_) => "a bool".into(),
                Literal::Ident(v) => v.clone(),
            }
        )),
    }
}

/// The defaults of a window's plain values fit their types.
fn check_defaults(env: &Env, owner: &Owner, out: &mut Output) {
    for decl in owner.data {
        let DataKind::Value { ty, default: Some(value) } = &decl.of else { continue };
        let Ok(ty) = env.type_ref(ty) else { continue };
        if let Some(problem) = misfit(value, &ty, &format!("'{}'", decl.name.name)) {
            out.push(Diagnostic::error(problem, decl.span));
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
        if !crate::names::is_camel_case(name) {
            out.push(Diagnostic::error(crate::names::camel_case_problem("handler", name), handler.name.span));
        }
        // A handler is in use when an event runs it, or another handler does.
        let mut called = false;
        for other in block.handlers.iter().filter(|h| h.name.name != name) {
            each_stmt_expr(&other.body, &mut |e| called |= matches!(&e.kind, ExprKind::Call { function, .. } if function.name == name));
            // Or the answer to a question it asks.
            each_stmt(&other.body, &mut |s| called |= matches!(&s.kind, StmtKind::Ask { then, .. } if then.name == name));
        }
        // A question needs its window open for the answer.
        let (mut asks, mut closes) = (false, false);
        each_stmt(&handler.body, &mut |s| {
            asks |= matches!(s.kind, StmtKind::Ask { .. });
            closes |= matches!(s.kind, StmtKind::Close | StmtKind::CloseWith { .. });
        });
        if asks && closes {
            out.push(Diagnostic::error(format!("handler '{name}' asks and closes its window: the answer would have nowhere to go"), handler.name.span));
        }
        if !called && !owner.wired.iter().any(|w| w.handler == name) {
            out.push(Diagnostic::warning(
                format!("handler '{name}' is not wired to any event"),
                handler.name.span,
            ));
        }
        check_handler(env, owner, &block.handlers, handler, out);
    }
}

/// The code block of an entity: its data events, each once.
fn check_events(env: &Env, block: &CodeBlock, out: &mut Output) {
    let entity = block.name.name.as_str();
    for handler in &block.handlers {
        out.push(Diagnostic::error(
            format!("the code of entity {entity} holds data events (before save, after save, before delete, after delete), not handlers"),
            handler.span,
        ));
    }
    for (i, event) in block.events.iter().enumerate() {
        if block.events[..i].iter().any(|e| e.event == event.event) {
            out.push(Diagnostic::error(format!("'{}' is written twice", event.event.text()), event.head));
            continue;
        }
        check_body(env, &event.action(entity), Some(event.event), out);
    }
}

fn check_handler(env: &Env, owner: &Owner, handlers: &[HandlerDecl], handler: &HandlerDecl, out: &mut Output) {
    // The values a click gives are names in its body.
    let mut scope = HashMap::new();
    for param in &handler.params {
        let ty = env.type_ref(&param.ty).unwrap_or_else(|message| {
            out.push(Diagnostic::error(message, param.ty.span));
            Ty::Unknown
        });
        scope.insert(param.name.name.clone(), ty);
    }
    let mut checker = BodyChecker {
        env,
        owner: Some(owner),
        handlers,
        decl: None,
        handler: Some(&handler.name.name),
        event: None,
        returns: None,
        scopes: vec![scope],
        saves_reported: false,
        engine: None,
        command: None,
        out,
    };
    checker.block(&handler.body);
}

/// The engines of a file: a .wasm file each, and functions that take and
/// give values (DESIGN 5.11).
fn check_engines(env: &Env, file: &SpecFile, out: &mut Output) {
    let mut seen: Vec<&str> = Vec::new();
    for engine in &file.engines {
        if seen.contains(&engine.name.name.as_str()) {
            out.push(Diagnostic::error(format!("there is already an engine {}", engine.name.name), engine.name.span));
        }
        seen.push(&engine.name.name);
        if let Some(body) = &engine.body {
            check_engine_body(env, engine, body, out);
        } else if !engine.file.ends_with(".wasm") {
            out.push(Diagnostic::error("an engine is a .wasm file in the engines folder", engine.span));
        }
        let mut used: Vec<&str> = Vec::new();
        for service in &engine.uses {
            if !crate::engines::SERVICES.contains(&service.name.as_str()) {
                out.push(Diagnostic::error(format!("'{}' is no service; an engine uses clock, random, log or language", service.name), service.span));
            } else if used.contains(&service.name.as_str()) {
                out.push(Diagnostic::error(format!("engine {} uses {} twice", engine.name.name, service.name), service.span));
            }
            used.push(&service.name);
        }
        if engine.seconds.is_some_and(|s| s == 0 || s > crate::engines::MAX_SECONDS) {
            out.push(Diagnostic::error(format!("an engine's time limit is between 1 and {} seconds", crate::engines::MAX_SECONDS), engine.span));
        }
        if engine.megabytes.is_some_and(|m| m == 0 || m > crate::engines::MAX_MEGABYTES) {
            out.push(Diagnostic::error(format!("an engine's memory limit is between 1 and {} MB", crate::engines::MAX_MEGABYTES), engine.span));
        }
        let mut names: Vec<&str> = Vec::new();
        for function in &engine.functions {
            if names.contains(&function.name.name.as_str()) {
                out.push(Diagnostic::error(format!("engine {} has function {} twice", engine.name.name, function.name.name), function.name.span));
            }
            names.push(&function.name.name);
            let types = function.params.iter().map(|p| &p.ty).chain(&function.returns);
            for ty in types {
                match env.type_ref(ty) {
                    Err(message) => out.push(Diagnostic::error(message, ty.span)),
                    Ok(Ty::File | Ty::Files) => out.push(Diagnostic::error(
                        format!("an engine takes values, records and lists of records, not {}", if matches!(ty.kind, TypeRefKind::File) { "file" } else { "files" }),
                        ty.span,
                    )),
                    Ok(_) => {}
                }
            }
            if function.returns.is_none() {
                out.push(Diagnostic::error(format!("engine function {} gives a value: write its type after '->'", function.name.name), function.span));
            }
        }
    }
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
                    format!("unknown effect '{}'; the effects are {}", effect.name, crate::check::listed(&EFFECTS)),
                    effect.span,
                ));
            }
        }
        check_body(env, decl, None, out);
    }
}

/// An engine written in Clarity: what it keeps starts as a value of its
/// type, and its functions are checked with those values in scope.
fn check_engine_body(env: &Env, engine: &EngineDecl, body: &EngineBody, out: &mut Output) {
    let mut kept = HashMap::new();
    for keep in &body.keeps {
        let ty = env.type_ref(&keep.ty).unwrap_or_else(|message| {
            out.push(Diagnostic::error(message, keep.ty.span));
            Ty::Unknown
        });
        if kept.contains_key(&keep.name.name) {
            out.push(Diagnostic::error(format!("engine {} keeps {} twice", engine.name.name, keep.name.name), keep.name.span));
        }
        // What it starts as: a value written out, checked against its type.
        let mut checker = BodyChecker {
            env,
            owner: None,
            handlers: &[],
            decl: None,
            handler: None,
            event: None,
            returns: None,
            scopes: vec![HashMap::new()],
            saves_reported: false,
            engine: Some(engine),
            command: None,
        out,
        };
        let value = checker.expr(&keep.value, Some(ty));
        if !ty.accepts(&value) {
            checker.error(format!("{} starts as {}: expected {}", keep.name.name, value.name(), ty.name()), keep.value.span);
        }
        kept.insert(keep.name.name.clone(), ty);
    }
    for decl in &body.functions {
        check_body_in(env, decl, None, Some(engine), kept.clone(), out);
    }
}

/// Checks the parameters, conditions and statements of a function, action
/// or data event.
fn check_body<'a>(env: &Env<'a>, decl: &FunctionDecl, event: Option<DataEvent>, out: &mut Output) {
    check_body_in(env, decl, event, None, HashMap::new(), out);
}

/// `check_body` for a function of an engine written in Clarity: `kept` are
/// the engine's keep values, in scope beneath its parameters.
fn check_body_in<'a>(env: &Env<'a>, decl: &FunctionDecl, event: Option<DataEvent>, engine: Option<&EngineDecl>, kept: HashMap<String, Ty<'a>>, out: &mut Output) {
    let name = decl.name.name.as_str();
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
        handler: None,
        event,
        returns,
        scopes: vec![kept, scope],
        saves_reported: false,
        engine,
        command: None,
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
    /// The function, action or data event being checked; None for a handler.
    decl: Option<&'e FunctionDecl>,
    /// The handler being checked, by name.
    handler: Option<&'e str>,
    /// The data event being checked, if it is one.
    event: Option<DataEvent>,
    returns: Option<Ty<'a>>,
    scopes: Vec<HashMap<String, Ty<'a>>>,
    /// A missing `effects db.write` is reported once per action.
    saves_reported: bool,
    /// The engine written in Clarity whose function this is: its keep
    /// values are in scope, its functions callable, and it shows nothing.
    engine: Option<&'e EngineDecl>,
    /// The command being checked (DESIGN 5.20): it has no window, prints,
    /// asks on the terminal and exits.
    command: Option<&'e CommandDecl>,
    out: &'e mut Output,
}

/// What a name or path refers to.
/// The id every record has, which code can read.
pub const RECORD_ID: &str = "id";

/// A random number from 1 to its argument, in an engine that uses random.
pub const RANDOM: &str = "random";

/// The built-in calls of the clock: the date, and the date and time, where
/// the application runs.
pub const TODAY: &str = "today";
pub const NOW: &str = "now";

/// The built-in `round(value, places)`: a decimal or money with at most
/// `places` decimals, rounded half away from zero (conformance/rounding.json).
pub const ROUND: &str = "round";

/// The built-in `sin(degrees)` and `cos(degrees)`, rounded to 10 decimals
/// (conformance/drawing.json).
pub const SIN: &str = "sin";
pub const COS: &str = "cos";

/// What a value of a shape is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeValue {
    Number,
    /// A color of the theme by name, or `#rrggbb`.
    Color,
    /// Text shown in the person's format, with slots.
    Text,
}

/// A shape a draw handler draws on its canvas: its name and its values, in
/// order, each with the value it has when left out (conformance/drawing.json).
pub struct Shape {
    pub name: &'static str,
    pub values: &'static [(&'static str, ShapeValue, Option<&'static str>)],
}

pub const SHAPES: [Shape; 6] = [
    Shape { name: "clear", values: &[("color", ShapeValue::Color, None)] },
    Shape {
        name: "line",
        values: &[
            ("x1", ShapeValue::Number, None),
            ("y1", ShapeValue::Number, None),
            ("x2", ShapeValue::Number, None),
            ("y2", ShapeValue::Number, None),
            ("color", ShapeValue::Color, None),
            ("width", ShapeValue::Number, Some("1")),
        ],
    },
    Shape {
        name: "rect",
        values: &[
            ("x", ShapeValue::Number, None),
            ("y", ShapeValue::Number, None),
            ("width", ShapeValue::Number, None),
            ("height", ShapeValue::Number, None),
            ("color", ShapeValue::Color, None),
        ],
    },
    Shape {
        name: "circle",
        values: &[("x", ShapeValue::Number, None), ("y", ShapeValue::Number, None), ("r", ShapeValue::Number, None), ("color", ShapeValue::Color, None)],
    },
    Shape {
        name: "ring",
        values: &[
            ("x", ShapeValue::Number, None),
            ("y", ShapeValue::Number, None),
            ("r", ShapeValue::Number, None),
            ("color", ShapeValue::Color, None),
            ("width", ShapeValue::Number, Some("1")),
        ],
    },
    Shape {
        name: "text",
        values: &[
            ("x", ShapeValue::Number, None),
            ("y", ShapeValue::Number, None),
            ("text", ShapeValue::Text, None),
            ("color", ShapeValue::Color, Some("text")),
            ("size", ShapeValue::Number, Some("10")),
        ],
    },
];

/// The shape `name`, if a canvas draws it.
pub fn shape(name: &str) -> Option<&'static Shape> {
    SHAPES.iter().find(|s| s.name == name)
}

/// "a, b and c".
fn and_list(words: &[&str]) -> String {
    match words {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// What a file of a file store has: its name (`photo.jpg`), its path inside
/// the store (`2024/photo.jpg`), its extension in lower case without the
/// dot (`jpg`), its size in bytes and when it was last changed.
pub const FILE_MEMBERS: [&str; 5] = ["name", "path", "extension", "size", "modified"];

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

    /// Whether `handler` draws for the draw handler `draws`: it is that
    /// handler, or one only it runs, itself or through such handlers, and no
    /// event runs. Shapes drawn from anywhere else would land outside a drawing.
    fn draws_for(&self, draws: &str, handler: &str) -> bool {
        let calls = |from: &str, to: &str| {
            let mut found = false;
            if let Some(from) = self.handlers.iter().find(|h| h.name.name == from) {
                each_stmt_expr(&from.body, &mut |e| found |= matches!(&e.kind, ExprKind::Call { function, .. } if function.name == to));
            }
            found
        };
        let names: Vec<&str> = self.handlers.iter().map(|h| h.name.name.as_str()).collect();
        // What the draw handler runs, itself or through what it runs.
        let mut reach = vec![draws];
        while let Some(next) = names.iter().find(|n| !reach.contains(n) && reach.iter().any(|r| calls(r, n))) {
            reach.push(next);
        }
        // Less what something else runs too.
        let wired = |name: &str| self.owner.is_some_and(|o| o.wired.iter().any(|w| w.handler == name));
        while let Some(at) = reach
            .iter()
            .skip(1)
            .position(|h| wired(h) || names.iter().any(|n| !reach.contains(n) && calls(n, h)))
        {
            reach.remove(at + 1);
        }
        reach.contains(&handler)
    }

    /// A shape on a canvas, in the canvas's own draw handler or one it runs.
    /// `arena.restart(2)`: a hear of a custom part, with values of its types.
    fn hear(&mut self, part: &Ident, hear: &Ident, args: &[Expr]) {
        let decl = match self.part(&part.name) {
            Some((_, Part::Widget(p))) => p.part.as_deref().and_then(|name| self.env.part_decl(name)),
            _ => None,
        };
        let message = decl.and_then(|d| d.hears.iter().find(|h| h.name.name == hear.name));
        let Some(message) = message else {
            if let Some(decl) = decl {
                self.error(format!("part {} hears no '{}'", decl.name.name, hear.name), hear.span);
            }
            for arg in args {
                self.expr(arg, None);
            }
            return;
        };
        let called = format!("{}.{}", part.name, hear.name);
        if args.len() != message.params.len() {
            let n = message.params.len();
            self.error(format!("{called} takes {n} value{}, not {}", if n == 1 { "" } else { "s" }, args.len()), hear.span);
        }
        for (i, arg) in args.iter().enumerate() {
            let expected = message.params.get(i).and_then(|p| self.env.type_ref(&p.ty).ok());
            let ty = self.expr(arg, expected);
            if let (Some(expected), Some(param)) = (expected, message.params.get(i))
                && !expected.accepts(&ty)
            {
                self.error(format!("value '{}' of {called} is {}, not {}", param.name.name, expected.name(), ty.name()), arg.span);
            }
        }
    }

    fn draw(&mut self, canvas: &Ident, shape: &Ident, args: &[Expr]) {
        match self.part(&canvas.name) {
            Some((_, Part::Widget(p))) if p.kind == crate::widgets::WidgetKind::Canvas => {
                let draws = p.handlers.iter().find(|h| h.event == crate::widgets::Event::Draw).map(|h| h.handler.as_str());
                match draws {
                    None => self.error(format!("'{}' has no draw handler; wire 'on draw: handler'", canvas.name), canvas.span),
                    Some(draws) if !self.handler.is_some_and(|h| self.draws_for(draws, h)) => self.error(
                        format!("'{}' is drawn only by its draw handler, {draws}, and the handlers only it runs", canvas.name),
                        canvas.span,
                    ),
                    Some(_) => {}
                }
            }
            Some((_, part)) => self.error(format!("'{}' is a {}; only a canvas draws", canvas.name, part.keyword()), canvas.span),
            None => self.error(format!("unknown canvas '{}'", canvas.name), canvas.span),
        }
        let Some(found) = self::shape(&shape.name) else {
            let names: Vec<&str> = SHAPES.iter().map(|s| s.name).collect();
            self.error(format!("a canvas draws {}, not '{}'", and_list(&names), shape.name), shape.span);
            for arg in args {
                self.expr(arg, None);
            }
            return;
        };
        let needed = found.values.iter().filter(|(_, _, default)| default.is_none()).count();
        if args.len() < needed || args.len() > found.values.len() {
            let names: Vec<&str> = found.values.iter().map(|(n, _, _)| *n).collect();
            let optional: Vec<&str> = found.values.iter().filter(|(_, _, d)| d.is_some()).map(|(n, _, _)| *n).collect();
            let left_out = match optional.len() {
                0 => String::new(),
                1 => format!(" ({} can be left out)", optional[0]),
                _ => format!(" ({} can be left out)", and_list(&optional)),
            };
            self.error(format!("'{}' takes {}{left_out}", shape.name, and_list(&names)), shape.span);
        }
        for (arg, (name, kind, _)) in args.iter().zip(found.values.iter()) {
            let ty = match kind {
                ShapeValue::Text => self.shown_text(arg),
                _ => self.expr(arg, None),
            };
            let problem = match kind {
                ShapeValue::Number if !matches!(ty, Ty::Integer | Ty::Decimal | Ty::Money | Ty::Unknown) => Some("a number"),
                ShapeValue::Color if !matches!(ty, Ty::Text | Ty::Unknown) => Some("text"),
                ShapeValue::Text if !ty.is_value() => Some("text or a value"),
                _ => None,
            };
            if let Some(wanted) = problem {
                self.error(format!("{name} of '{}' is {wanted}, not {}", shape.name, ty.name()), arg.span);
            }
        }
        for arg in args.iter().skip(found.values.len()) {
            self.expr(arg, None);
        }
    }

    fn part(&self, name: &str) -> Option<(&'a str, Part<'a>)> {
        self.owner.and_then(|o| o.parts.get_key_value(name)).map(|(k, p)| (*k, *p))
    }

    /// "a handler", "a function" or "an action", for messages.
    fn context(&self) -> &'static str {
        if self.command.is_some() {
            return "a command";
        }
        match self.decl.map(|d| d.kind) {
            None => "a handler",
            Some(FunctionKind::Function) => "a function",
            Some(FunctionKind::Action) => "an action",
            Some(FunctionKind::Event) => "a data event",
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
        if self.command.is_some() {
            self.error(format!("a command has no window: '{word}' belongs in a handler"), span);
        } else if let Some(engine) = self.engine {
            self.error(format!("an engine shows nothing: '{word}' belongs in a handler, which shows what engine {} gives", engine.name.name), span);
        } else if self.decl.is_some() {
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
            StmtKind::Write { text, path, .. } => {
                self.check_files("writes a file", "files.write", stmt.span);
                // What is written is text, or bytes as they are.
                let ty = self.expr(text, None);
                if !matches!(ty, Ty::Bytes | Ty::Unknown) && !Ty::Text.accepts(&ty) {
                    self.error(format!("what to write must be text or bytes, not {}", ty.name()), text.span);
                }
                let ty = self.expr(path, Some(Ty::Text));
                if !Ty::Text.accepts(&ty) {
                    self.error(format!("the path to write it to must be text, not {}", ty.name()), path.span);
                }
            }
            StmtKind::For { var, list, body } => {
                let ty = self.expr(list, None);
                let item = match ty {
                    Ty::List(entity) => Ty::Record(entity),
                    Ty::Values(item) => item_ty(item),
                    Ty::Files => Ty::File,
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
            StmtKind::Draw { canvas, shape, args } => self.draw(canvas, shape, args),
            StmtKind::Hear { part, hear, args } => self.hear(part, hear, args),
            StmtKind::Call { call } => {
                self.call(call, true);
            }
            StmtKind::Close => self.ui_only("close", stmt.span),
            StmtKind::CloseWith { value } => self.check_close_with(stmt.span, value),
            StmtKind::Reload { data } => {
                self.ui_only("reload", stmt.span);
                match self.data(&data.name) {
                    None if self.decl.is_none() && self.command.is_none() => {
                        let owner = self.owner.map(|o| o.name).unwrap_or("");
                        self.error(format!("{owner} has no data '{}'", data.name), data.span);
                    }
                    Some(decl) if decl.value().is_some() => {
                        self.error(format!("'{}' is a plain value; only records and lists are read again", data.name), data.span);
                    }
                    _ => {}
                }
            }
            StmtKind::Message { text, .. } => {
                // A command writes its messages to standard error.
                if self.command.is_none() {
                    self.ui_only("message", stmt.span);
                }
                let ty = self.shown_text(text);
                if !ty.is_value() {
                    self.error(format!("a message is text or a value, not {}", ty.name()), text.span);
                }
            }
            StmtKind::Ask { .. } if self.command.is_some() => {
                self.error("in a command, ask gives its answer, as in 'let answer = ask ...' or 'if ask ...'", stmt.span);
            }
            StmtKind::Ask { question, form, then, args } => self.check_ask(stmt.span, question.as_ref(), form, then, args),
            StmtKind::Print { value, columns } => self.check_print(stmt.span, value, columns),
            StmtKind::Exit { code } => self.check_exit(stmt.span, code),
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
                self.check_own_record(word, value);
            }
            StmtKind::Refuse { text } => {
                let ty = self.shown_text(text);
                if !ty.is_value() {
                    self.error(format!("'refuse' needs text or a value, not {}", ty.name()), text.span);
                }
            }
        }
    }

    /// `ask ... then handler(values)`: a question that is text, a form that
    /// can be asked, and a handler of the window that takes the values
    /// written and, for a value, the answer last.
    fn check_ask(&mut self, span: Span, question: Option<&Expr>, form: &AskForm, then: &Ident, args: &[Expr]) {
        self.ui_only("ask", span);
        if let Some(question) = question {
            let ty = self.shown_text(question);
            if !ty.is_value() {
                self.error(format!("a question is text or a value, not {}", ty.name()), question.span);
            }
        }
        let answer = match form {
            AskForm::Window { window, with } => Some(self.asked_dialog(window, with)),
            AskForm::Confirm { yes, no } => {
                self.button_texts(yes.as_ref(), no.as_ref());
                None
            }
            AskForm::Value { ty, default, from } => Some(self.asked_value(ty, default.as_ref(), from.as_ref())),
        };
        let owner = self.owner.map(|o| o.name).unwrap_or("");
        let Some(handler) = self.handlers.iter().find(|h| h.name.name == then.name) else {
            if self.decl.is_none() {
                self.error(format!("{owner} has no handler '{}' to run with the answer", then.name), then.span);
            }
            for arg in args {
                self.expr(arg, None);
            }
            return;
        };
        let name = &then.name;
        let wanted = args.len() + usize::from(answer.is_some());
        if answer.is_none() && handler.params.len() == args.len() + 1 {
            self.error(format!("handler '{name}' runs on yes and takes no answer: only the values written after 'then'"), then.span);
        } else if handler.params.len() != wanted {
            let n = handler.params.len();
            let more = if answer.is_some() { format!("{} written and the answer", args.len()) } else { format!("{} written", args.len()) };
            self.error(format!("handler '{name}' takes {n} parameters, not {wanted}: {more}"), then.span);
        }
        for (arg, param) in args.iter().zip(&handler.params) {
            let expected = self.env.type_ref(&param.ty).unwrap_or(Ty::Unknown);
            let ty = self.expr(arg, Some(expected));
            if !expected.accepts(&ty) {
                self.error(format!("value '{}' of handler {name} is {}, not {}", param.name.name, described(&expected), ty.name()), arg.span);
            }
        }
        if let (Some(answer), Some(last)) = (answer, handler.params.get(args.len()))
            && handler.params.len() == wanted
        {
            let expected = self.env.type_ref(&last.ty).unwrap_or(Ty::Unknown);
            if !answer.is_unknown() && !expected.accepts(&answer) {
                self.error(
                    format!("the answer is {}, but '{}', the last parameter of handler {name}, is {}", answer.name(), last.name.name, expected.name()),
                    then.span,
                );
            }
        }
    }

    /// The dialog `ask` names, with its data filled as `open` fills it: the
    /// type of what it gives, which is the answer.
    /// The words of a yes-or-no question's buttons.
    fn button_texts(&mut self, yes: Option<&Expr>, no: Option<&Expr>) {
        for text in yes.into_iter().chain(no) {
            let ty = self.shown_text(text);
            if !ty.is_value() {
                self.error(format!("a button says text or a value, not {}", ty.name()), text.span);
            }
        }
    }

    /// `for type [default value] [from list]`: the type of the answer.
    fn asked_value(&mut self, ty: &TypeRef, default: Option<&Expr>, from: Option<&Expr>) -> Ty<'a> {
        let asked = self.env.type_ref(ty).unwrap_or_else(|message| {
            self.error(message, ty.span);
            Ty::Unknown
        });
        let fits = matches!(
            asked,
            Ty::Text | Ty::Integer | Ty::Decimal | Ty::Money | Ty::Date | Ty::DateTime | Ty::Duration | Ty::Enum(_) | Ty::Record(_) | Ty::Unknown
        );
        if !fits {
            self.error(format!("ask for text, a number, a date, a duration, an enum or an entity, not {}", asked.name()), ty.span);
        }
        if let Some(default) = default {
            let given = self.expr(default, Some(asked));
            if fits && !asked.accepts(&given) {
                self.error(format!("the default is {}, but the answer is {}", given.name(), asked.name()), default.span);
            }
        }
        if let Some(from) = from {
            let given = self.expr(from, None);
            let wanted = format!("list<{}>", asked.name());
            if !matches!(asked, Ty::Record(_) | Ty::Unknown) {
                self.error(format!("'from' gives the records to choose from; {} is no entity", asked.name()), from.span);
            } else if !asked.is_unknown() && !given.is_unknown() && given.name() != wanted {
                self.error(format!("'from' gives the choices: a {wanted}, not {}", given.name()), from.span);
            }
        }
        asked
    }

    /// `ask` as a value, in a command: it waits for the answer and gives it.
    fn ask_value(&mut self, question: &Expr, form: &AskForm) -> Ty<'a> {
        if self.command.is_none() {
            let message = if self.decl.is_some() || self.engine.is_some() {
                format!("'ask' belongs in a handler or a command, not in {}", self.context())
            } else {
                "a handler asks with 'then' and the handler the answer runs, as in 'ask ... then answered'".to_string()
            };
            self.error(message, question.span);
        }
        let ty = self.shown_text(question);
        if !ty.is_value() {
            self.error(format!("a question is text or a value, not {}", ty.name()), question.span);
        }
        match form {
            AskForm::Confirm { yes, no } => {
                self.button_texts(yes.as_ref(), no.as_ref());
                Ty::Bool
            }
            AskForm::Value { ty, default, from } => {
                let asked = self.asked_value(ty, default.as_ref(), from.as_ref());
                if let (Ty::Record(entity), None) = (asked, from) {
                    let list = format!("{}s", crate::model::snake_case(&entity.name.name));
                    self.error(format!("a command asks for a record from a list: 'ask ... for {} from {list}'", entity.name.name), ty.span);
                }
                asked
            }
            AskForm::Window { .. } => Ty::Unknown,
        }
    }

    /// `print value` or `print list columns a, b`, in a command.
    fn check_print(&mut self, span: Span, value: &Expr, columns: &[Ident]) {
        if self.command.is_none() {
            let context = self.context();
            self.error(format!("'print' belongs in a command, not in {context}"), span);
        }
        if columns.is_empty() {
            let ty = self.shown_text(value);
            if matches!(ty, Ty::List(_)) {
                self.error("print a list of records as a table, with its columns: 'print found columns name'", value.span);
            } else if !ty.is_value() {
                self.error(format!("print writes text or a value, not {}", ty.name()), value.span);
            }
            return;
        }
        match self.expr(value, None) {
            Ty::List(entity) => {
                for column in columns {
                    if column.name != RECORD_ID && !entity.fields.iter().any(|f| f.name.name == column.name) {
                        self.error(format!("{} has no field '{}'", entity.name.name, column.name), column.span);
                    }
                }
            }
            Ty::Unknown => {}
            other => self.error(format!("'columns' prints a list of records as a table, not {}", other.name()), value.span),
        }
    }

    /// `exit code`, in a command: an integer from 0 to 255.
    fn check_exit(&mut self, span: Span, code: &Expr) {
        if self.command.is_none() {
            let context = self.context();
            self.error(format!("'exit' belongs in a command, not in {context}"), span);
        }
        let ty = self.expr(code, Some(Ty::Integer));
        if !Ty::Integer.accepts(&ty) {
            self.error(format!("the exit code is an integer, not {}", ty.name()), code.span);
        } else if let ExprKind::Number { value } = &code.kind
            && value.parse::<u32>().map_or(true, |n| n > 255)
        {
            self.error("the exit code is 0 to 255", code.span);
        }
    }

    fn asked_dialog(&mut self, window: &Ident, with: &[crate::code::Arg]) -> Ty<'a> {
        for arg in with {
            self.expr(&arg.value, None);
        }
        let Some(page) = self.env.page(&window.name) else {
            self.error(format!("unknown window '{}'", window.name), window.span);
            return Ty::Unknown;
        };
        for arg in with {
            if !page.data.iter().any(|d| d.name.name == arg.name.name) {
                self.error(format!("{} has no data '{}'", page.name.name, arg.name.name), arg.name.span);
            }
        }
        if page.kind != crate::model::PageKind::Dialog {
            self.error(format!("{} is no dialog; ask a dialog that gives a value, or open {} instead", page.name.name, page.name.name), window.span);
            return Ty::Unknown;
        }
        match &page.gives {
            Some(gives) => self.env.type_ref(gives).unwrap_or(Ty::Unknown),
            None => {
                self.error(format!("{} gives nothing; write 'page {}: dialog gives Type', or open it instead", page.name.name, page.name.name), window.span);
                Ty::Unknown
            }
        }
    }

    /// `close with value`: only in a dialog, and of the type it gives.
    fn check_close_with(&mut self, span: Span, value: &Expr) {
        self.ui_only("close", span);
        let given = self.expr(value, None);
        let Some(owner) = self.owner else { return };
        let Some(page) = self.env.page(owner.name) else { return };
        if page.kind != crate::model::PageKind::Dialog {
            self.error(format!("only a dialog closes with a value; {} is a window", page.name.name), span);
            return;
        }
        let Some(gives) = &page.gives else {
            self.error(format!("{} gives nothing; write 'page {}: dialog gives Type' to close with a value", page.name.name, page.name.name), span);
            return;
        };
        let wanted = self.env.type_ref(gives).unwrap_or(Ty::Unknown);
        if !wanted.is_unknown() && !given.is_unknown() && !wanted.accepts(&given) {
            self.error(format!("{} gives {}, not {}", page.name.name, wanted.name(), given.name()), value.span);
        }
    }

    /// A data event that saves its own record would run itself again, and
    /// so on without end; the same for deleting.
    fn check_own_record(&mut self, word: &str, value: &Expr) {
        let (Some(event), Some(decl)) = (self.event, self.decl) else { return };
        let ExprKind::Name { name } = &value.kind else { return };
        let own = decl.params.first().is_some_and(|p| p.name.name == *name);
        if own && event.saves() == (word == "save") {
            let doing = if word == "save" { "saving" } else { "deleting" };
            self.error(
                format!("{doing} {name} in its own '{}' would run the event again, without end", event.text()),
                value.span,
            );
        }
    }

    /// Listing, reading and writing files: not in functions; in actions and
    /// data events only with the effect.
    fn check_files(&mut self, doing: &str, effect: &str, span: Span) {
        let Some(decl) = self.decl else { return };
        match decl.kind {
            FunctionKind::Function => self.error(
                format!("function '{}' cannot reach files; an action can, with 'effects {effect}'", decl.name.name),
                span,
            ),
            FunctionKind::Action | FunctionKind::Event => {
                if !decl.effects.iter().any(|e| e.name == effect) {
                    self.error(format!("action '{}' {doing}, which needs 'effects {effect}'", decl.name.name), span);
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
            FunctionKind::Action | FunctionKind::Event => {
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
        if self.command.is_some() {
            if let Some(value) = value {
                self.expr(value, None);
                self.error("a command does not return a value; it ends with 'exit' and a code", value.span);
            }
            return;
        }
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


    /// Whether `target` is text the screen shows: a part's caption or error.
    fn shows(&self, target: &Expr) -> bool {
        let ExprKind::Member { object, member } = &target.kind else { return false };
        let ExprKind::Name { name } = &object.kind else { return false };
        self.local(name).is_none() && self.part(name).is_some() && matches!(member.name.as_str(), "caption" | "error")
    }

    /// Text the screen shows (a message, a refusal, a caption or an error):
    /// text with slots is at home here. Text joined with `+` cannot be
    /// translated or shown in the person's format, so slots are suggested.
    fn shown_text(&mut self, expr: &Expr) -> Ty<'a> {
        if let ExprKind::Text { parts } = &expr.kind {
            self.slots(parts);
            self.out.types.types.insert((expr.span.start, expr.span.end), TypeInfo::Text);
            return Ty::Text;
        }
        let ty = self.expr(expr, None);
        if joins_text(expr) {
            self.out.push(Diagnostic::warning(
                "write this as one text with slots, like \"Saved {customer.name}\", so it can be translated and its values shown in the person's format",
                expr.span,
            ));
        }
        ty
    }

    /// Each slot of a text names a value: a field, a local or a parameter
    /// that is not a whole record or list.
    fn slots(&mut self, parts: &[TextPart]) {
        for part in parts {
            let TextPart::Slot { value } = part else { continue };
            let ty = self.expr(value, None);
            if !ty.is_value() {
                let path = crate::code::path_text(value);
                self.error(
                    format!("a slot shows a value, and {{{path}}} is {}; name one of its fields, like {{{path}.name}}", ty.name()),
                    value.span,
                );
            }
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
        // A file of a store becomes a record's file; another record's cannot be copied yet.
        if matches!(ty, Ty::RecordFile) && !matches!(value.kind, ExprKind::Empty) {
            if !matches!(self.expr(value, Some(ty)), Ty::File | Ty::Unknown) {
                self.error("a file cannot be copied from one record to another yet", value.span);
            }
            return;
        }
        if matches!(ty, Ty::Image) && !matches!(value.kind, ExprKind::Empty) {
            // A file of a file store becomes the picture; another record's
            // picture cannot be copied yet.
            if !matches!(self.expr(value, Some(ty)), Ty::File | Ty::Unknown) {
                self.error("a picture cannot be copied from one record to another yet", value.span);
            }
            return;
        }
        let value_ty = if self.shows(target) { self.shown_text(value) } else { self.expr(value, Some(ty)) };
        if !ty.accepts(&value_ty) {
            self.error(format!("cannot set {} to {}", ty.name(), value_ty.name()), value.span);
        }
        // A color field takes text, written as a color is kept.
        if let ExprKind::Str { value: text } = &value.kind
            && self.field_of(target).is_some_and(|f| f.ty == FieldType::Color)
            && !crate::colors::is_kept(text)
        {
            self.error(crate::colors::COLOR_PROBLEM, value.span);
        }
    }

    /// The field a member path names, as `product.tint`, once its record is resolved.
    fn field_of(&self, target: &Expr) -> Option<&'a crate::model::Field> {
        let ExprKind::Member { object, member } = &target.kind else { return None };
        match self.out.types.types.get(&(object.span.start, object.span.end))? {
            TypeInfo::Record(entity) => self.env.entity(entity)?.field(&member.name),
            _ => None,
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
                    let kept = self.engine.is_some() && self.decl.is_some() && self.scopes.iter().rposition(|s| s.contains_key(name)) == Some(0);
                    self.name_is(expr, if kept { NameKind::Kept } else { NameKind::Local });
                    return Some(Place::Value { ty, writable: Ok(()) });
                }
                // A command has the settings as data of its own, as every window does.
                if self.command.is_some()
                    && name == crate::parser::SETTINGS_DATA
                    && let Some(settings) = self.env.settings
                {
                    self.name_is(expr, NameKind::Data);
                    return Some(Place::Value { ty: Ty::Record(settings), writable: Ok(()) });
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
                    Place::Part(Part::Widget(props), name)
                        if props.bind.is_some() && ((m == "text" && has_text(props)) || (m == "checked" && has_checked(props))) =>
                    {
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
            // `squares[3]`: a position of a list, set when the list can be.
            ExprKind::Index { list, index } => {
                let (ty, writable) = match self.place(list)? {
                    Place::Value { ty, writable } => (ty, writable),
                    _ => return None,
                };
                self.position(index);
                let item = match ty {
                    Ty::Values(item) => item_ty(item),
                    Ty::List(entity) => Ty::Record(entity),
                    Ty::Unknown => Ty::Unknown,
                    other => {
                        self.error(format!("only a list has positions, not {}", other.name()), list.span);
                        Ty::Unknown
                    }
                };
                self.out.types.types.insert((expr.span.start, expr.span.end), item.info());
                Some(Place::Value { ty: item, writable })
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

    /// A position in a list: an integer, counted from 1.
    fn position(&mut self, index: &Expr) {
        let ty = self.expr(index, Some(Ty::Integer));
        if !matches!(ty, Ty::Integer | Ty::Unknown) {
            self.error(format!("a position in a list is an integer, counted from 1, not {}", ty.name()), index.span);
        }
    }

    fn property(&self, part: Part<'a>, name: &str, property: &str) -> Option<Place<'a>> {
        let value = |ty| Some(Place::Value { ty, writable: Ok(()) });
        // A custom part's own properties, as its declaration types them.
        if let Part::Widget(props) = part
            && let Some(decl) = props.part.as_deref().and_then(|p| self.env.part_decl(p))
            && let Some(own) = decl.properties.iter().find(|p| p.name.name == property)
        {
            return value(self.env.type_ref(&own.ty).unwrap_or(Ty::Unknown));
        }
        match property {
            "visible" | "enabled" => value(Ty::Bool),
            "caption" if part.has_caption() => value(Ty::Text),
            "error" if part.is_input() => value(Ty::Text),
            "readonly" if part.is_input() => value(Ty::Bool),
            // A sidebar folded to its button, or filling the window.
            "collapsed" if matches!(part, Part::Sidebar(p) if p.collapsible) => value(Ty::Bool),
            "maximized" if matches!(part, Part::Sidebar(p) if p.maximizable) => value(Ty::Bool),
            // A ribbon folded to its tabs, and its open tab, counted from 1.
            "collapsed" if matches!(part, Part::Ribbon(r) if r.collapsible) => value(Ty::Bool),
            "tab" if matches!(part, Part::Ribbon(_)) => value(Ty::Integer),
            // The step a wizard is on, counted from 1.
            "step" if matches!(part, Part::Widget(props) if props.kind == WidgetKind::Steps) => value(Ty::Integer),
            // The open section of an accordion, from 1; 0 when none is open.
            "open" if matches!(part, Part::Widget(props) if props.kind == WidgetKind::Accordion) => value(Ty::Integer),
            "folded" if matches!(part, Part::Widget(props) if props.foldable) => value(Ty::Bool),
            "text" if matches!(part, Part::Widget(props) if has_text(props)) => value(Ty::Text),
            "checked" if matches!(part, Part::Widget(props) if has_checked(props)) => value(Ty::Bool),
            // Where a canvas was clicked last, in its own units.
            "x" | "y" if matches!(part, Part::Widget(props) if props.kind == WidgetKind::Canvas) => {
                Some(Place::Value { ty: Ty::Integer, writable: Err(format!("{name}.{property} is read, not set: it is where the canvas was clicked")) })
            }
            "selected" => {
                let Part::Widget(props) = part else { return None };
                if !matches!(props.kind, WidgetKind::Table | WidgetKind::List | WidgetKind::Tree | WidgetKind::TreeTable) {
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
            // A record's file: its name and extension, read only.
            Ty::RecordFile => match member {
                "name" | "extension" => Some(Place::Value { ty: Ty::Text, writable: Err(format!("a file's {member} is read, not set")) }),
                _ => {
                    self.error(format!("a file of a record has name and extension, not '{member}'"), span);
                    None
                }
            },
            Ty::Bytes => match member {
                "size" => Some(Place::Value { ty: Ty::Integer, writable: Err("the size of bytes is read, not set".into()) }),
                _ => {
                    self.error(format!("bytes have size, not '{member}'"), span);
                    None
                }
            },
            Ty::File => {
                let ty = match member {
                    "name" | "path" | "extension" => Ty::Text,
                    "size" => Ty::Integer,
                    "modified" => Ty::DateTime,
                    _ => {
                        self.error(format!("a file has {}, not '{member}'", crate::check::listed(&FILE_MEMBERS)), span);
                        return None;
                    }
                };
                Some(Place::Value { ty, writable: Err("a file's details are read, not set".into()) })
            }
            Ty::Record(entity) => match entity.field(member) {
                Some(field) => Some(Place::Value {
                    ty: self.env.field_type(&field.ty),
                    writable: Ok(()),
                }),
                // Every record has an id; `save` gives it one.
                None if member == RECORD_ID => Some(Place::Value {
                    ty: Ty::Integer,
                    writable: Err("a record's id is set by 'save'".into()),
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
            // `[0, 0, 0]`: values of one plain type; an empty one takes the type expected.
            ExprKind::ListOf { items } => {
                if items.is_empty() {
                    return match expected {
                        Some(ty @ Ty::Values(_)) => ty,
                        _ => {
                            self.error("an empty list needs its type, as in let squares: list<integer> = []", expr.span);
                            Ty::Unknown
                        }
                    };
                }
                let wanted = match expected {
                    Some(Ty::Values(item)) => Some(item_ty(item)),
                    _ => None,
                };
                let first = self.expr(&items[0], wanted);
                let item = match first {
                    Ty::Text => Item::Text,
                    Ty::Integer => Item::Integer,
                    Ty::Decimal | Ty::Money => Item::Decimal,
                    Ty::Bool => Item::Bool,
                    Ty::Date => Item::Date,
                    Ty::Enum(e) => Item::Enum(e),
                    Ty::Unknown => return Ty::Unknown,
                    other => {
                        self.error(format!("a list holds text, integers, decimals, bools, dates or the values of an enum, not {}", other.name()), items[0].span);
                        return Ty::Unknown;
                    }
                };
                // An expected list of decimals takes integers too.
                let item = match (wanted, item) {
                    (Some(Ty::Decimal), Item::Integer) => Item::Decimal,
                    _ => item,
                };
                for other in &items[1..] {
                    let ty = self.expr(other, Some(item_ty(item)));
                    if !item_ty(item).accepts(&ty) {
                        self.error(format!("a list holds values of one type: this is {}, the list {}", ty.name(), item.word()), other.span);
                    }
                }
                Ty::Values(item)
            }
            ExprKind::Index { .. } => match self.place(expr) {
                Some(Place::Value { ty, .. }) => ty,
                _ => Ty::Unknown,
            },
            // A text by its key; each slot gets a value, once. That the key has
            // these slots is checked against the language files.
            ExprKind::Key { args, .. } => {
                let mut seen = std::collections::HashSet::new();
                for arg in args {
                    if !seen.insert(arg.name.name.as_str()) {
                        self.error(format!("the slot {{{}}} gets two values", arg.name.name), arg.name.span);
                    }
                    let ty = self.expr(&arg.value, None);
                    if !ty.is_value() {
                        self.error(format!("a slot shows a value, and {} is {}; name one of its fields", arg.name.name, ty.name()), arg.value.span);
                    }
                }
                Ty::Text
            }
            // A text with slots is a text, translated where code makes it.
            ExprKind::Text { parts } => {
                self.slots(parts);
                Ty::Text
            }
            ExprKind::Bool { .. } => Ty::Bool,
            ExprKind::Empty => self.empty(expected, expr.span),
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
            ExprKind::EngineCall { engine, function, args } => self.engine_call(expr, engine, function, args),
            ExprKind::Find { var, entity, filter, order } => self.find(var, entity, filter.as_deref(), order),
            ExprKind::Files { var, filter, order, .. } => self.files(var, filter.as_deref(), order, expr.span),
            ExprKind::Ask { question, form } => self.ask_value(question, form),
            ExprKind::ReadText { file } => {
                self.check_files("reads a file", "files.read", expr.span);
                let ty = self.expr(file, None);
                if !matches!(ty, Ty::File | Ty::Unknown) {
                    self.error(format!("'read text of' needs a file, not {}", ty.name()), file.span);
                }
                Ty::Text
            }
            ExprKind::ReadBytes { file } => {
                self.check_files("reads a file", "files.read", expr.span);
                let ty = self.expr(file, None);
                if !matches!(ty, Ty::File | Ty::Unknown) {
                    self.error(format!("'read bytes of' needs a file, not {}", ty.name()), file.span);
                }
                Ty::Bytes
            }
            ExprKind::New { entity } => match self.env.entity(&entity.name) {
                Some(decl) => Ty::Record(decl),
                None => {
                    self.error(format!("unknown entity '{}'", entity.name), entity.span);
                    Ty::Unknown
                }
            },
        }
    }

    /// A call to a function of an engine: checked against its signature
    /// like a call to a function (DESIGN 5.11).
    fn engine_call(&mut self, expr: &Expr, engine: &Ident, function: &Ident, args: &[Expr]) -> Ty<'a> {
        let Some(decl) = self.env.engine(&engine.name) else {
            for arg in args {
                self.expr(arg, None);
            }
            self.error(format!("unknown engine '{}'", engine.name), engine.span);
            return Ty::Unknown;
        };
        let Some(callee) = decl.functions.iter().find(|f| f.name.name == function.name) else {
            for arg in args {
                self.expr(arg, None);
            }
            self.error(format!("engine {} has no function {}", engine.name, function.name), function.span);
            return Ty::Unknown;
        };
        // An engine with state per window has an instance in each open window,
        // so it is called in a window: by the window's own handlers.
        if decl.state == EngineState::Window {
            let caller = match self.owner {
                Some(owner) if !owner.is_app => None,
                Some(_) => Some("the app"),
                None => Some(self.context()),
            };
            if let Some(caller) = caller {
                self.error(format!("engine {} keeps state per window, so only a window's handlers call it, not {caller}", engine.name), expr.span);
            }
        }
        let called = format!("{}.{}", engine.name, function.name);
        if args.len() != callee.params.len() {
            let n = callee.params.len();
            self.error(format!("{called} takes {n} argument{}, not {}", if n == 1 { "" } else { "s" }, args.len()), expr.span);
            for arg in args {
                self.expr(arg, None);
            }
        } else {
            for (arg, param) in args.iter().zip(&callee.params) {
                let expected = self.env.type_ref(&param.ty).unwrap_or(Ty::Unknown);
                let ty = self.expr(arg, Some(expected));
                if !expected.accepts(&ty) {
                    self.error(
                        format!("argument '{}' of {called} is {} {}, not {}", param.name.name, article(&expected.name()), expected.name(), ty.name()),
                        arg.span,
                    );
                }
            }
        }
        callee.returns.as_ref().map_or(Ty::Unknown, |r| self.env.type_ref(r).unwrap_or(Ty::Unknown))
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
                [Ty::List(_) | Ty::Values(_) | Ty::Files | Ty::Unknown] => {}
                [other] => self.error(format!("'count' needs a list, not {}", other.name()), args[0].span),
                _ => self.error("'count' takes one list", expr.span),
            }
            return Some(Ty::Integer);
        }
        if function.name == SIN || function.name == COS {
            let tys: Vec<Ty> = args.iter().map(|a| self.expr(a, None)).collect();
            match tys.as_slice() {
                [Ty::Integer | Ty::Decimal | Ty::Money | Ty::Unknown] => {}
                [other] => self.error(format!("'{}' takes degrees, a number, not {}", function.name, other.name()), args[0].span),
                _ => self.error(format!("'{}' takes degrees, as in {}(45)", function.name, function.name), expr.span),
            }
            return Some(Ty::Decimal);
        }
        if function.name == ROUND {
            let tys: Vec<Ty> = args.iter().map(|a| self.expr(a, None)).collect();
            let [value, places] = tys.as_slice() else {
                self.error("'round' takes a number and its decimals, as in round(total, 2)", expr.span);
                return Some(Ty::Decimal);
            };
            if !matches!(places, Ty::Integer | Ty::Unknown) {
                self.error(format!("the decimals of 'round' are an integer, not {}", places.name()), args[1].span);
            }
            return Some(match value {
                Ty::Decimal | Ty::Money | Ty::Unknown => *value,
                other => {
                    self.error(format!("'round' needs a decimal or money, not {}", other.name()), args[0].span);
                    Ty::Decimal
                }
            });
        }
        if function.name == TODAY || function.name == NOW {
            for arg in args {
                self.expr(arg, None);
            }
            if !args.is_empty() {
                self.error(format!("'{}' takes no arguments", function.name), expr.span);
            }
            return Some(if function.name == TODAY { Ty::Date } else { Ty::DateTime });
        }
        // A handler runs another handler of its window by name.
        if self.decl.is_none()
            && let Some(handler) = self.handlers.iter().find(|h| h.name.name == function.name)
        {
            self.name_is(expr, NameKind::Handler);
            let name = &function.name;
            if args.len() != handler.params.len() {
                for arg in args {
                    self.expr(arg, None);
                }
                let n = handler.params.len();
                self.error(format!("handler '{name}' takes {n} value{}, not {}", if n == 1 { "" } else { "s" }, args.len()), expr.span);
            } else {
                for (arg, param) in args.iter().zip(&handler.params) {
                    let expected = self.env.type_ref(&param.ty).unwrap_or(Ty::Unknown);
                    let ty = self.expr(arg, Some(expected));
                    if !expected.accepts(&ty) {
                        self.error(format!("value '{}' of handler {name} is {}, not {}", param.name.name, described(&expected), ty.name()), arg.span);
                    }
                }
            }
            if !statement {
                self.error(format!("handler '{name}' does not return a value"), expr.span);
            }
            return None;
        }
        // `random(6)`: a number from 1 to 6, for an engine that uses random.
        if function.name == RANDOM && self.env.function(RANDOM).is_none() {
            let tys: Vec<Ty> = args.iter().map(|a| self.expr(a, Some(Ty::Integer))).collect();
            match self.engine {
                Some(engine) if !engine.uses.iter().any(|u| u.name == RANDOM) => {
                    self.error(format!("random needs 'uses random' in engine {}", engine.name.name), function.span)
                }
                Some(_) => {}
                None => self.error("random is a service of an engine; a function gives the same answer for the same values", function.span),
            }
            if !matches!(tys.as_slice(), [Ty::Integer | Ty::Unknown]) {
                self.error("random takes the highest number it may give, as in random(6)", expr.span);
            }
            return Some(Ty::Integer);
        }
        // An engine's own functions call each other by name.
        let own = self.engine.and_then(|e| e.body.as_ref()).and_then(|b| b.functions.iter().find(|f| f.name.name == function.name));
        let Some(callee) = own.or_else(|| self.env.function(&function.name)) else {
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
            FunctionKind::Action | FunctionKind::Event => {
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

    /// `files f in photos where ... order by ...`: a list of files. `f` is a
    /// local of the condition and the order, which run in memory, so the
    /// condition may be any bool.
    fn files(&mut self, var: &Ident, filter: Option<&Expr>, order: &[OrderKey], span: Span) -> Ty<'a> {
        self.check_files("lists files", "files.read", span);
        self.scopes.push(HashMap::new());
        self.define(var, Ty::File);
        if let Some(filter) = filter {
            self.condition(filter);
        }
        for key in order {
            if key.var.name != var.name || !key.through.is_empty() {
                self.error(format!("order by a detail of {0}, as in '{0}.name'", var.name), key.var.span);
                continue;
            }
            if !FILE_MEMBERS.contains(&key.field.name.as_str()) {
                self.error(format!("a file has {}, not '{}'", crate::check::listed(&FILE_MEMBERS), key.field.name), key.field.span);
            }
        }
        self.scopes.pop();
        Ty::Files
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
            self.query_condition(filter, &var.name, decl);
        }
        for key in order {
            if key.var.name != var.name {
                self.error(
                    format!("order by a field of {0}, as in '{0}.name'", var.name),
                    key.var.span,
                );
                continue;
            }
            let hops: Vec<&Ident> = key.through.iter().collect();
            let target = match self.through(decl, &var.name, &hops) {
                Ok(Some(target)) => target,
                Ok(None) => continue,
                Err(message) => {
                    self.error(message, join(key.var.span, key.field.span));
                    continue;
                }
            };
            match target.field(&key.field.name) {
                None => self.error(
                    format!("{} has no field '{}'", target.name.name, key.field.name),
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
    fn query_condition(&mut self, expr: &Expr, var: &str, decl: &'a Entity) {
        let general = format!("a query compares a field of {var} with a value, as in '{var}.name == x'");
        match &expr.kind {
            ExprKind::Binary { op: BinaryOp::And | BinaryOp::Or, left, right } => {
                self.query_condition(left, var, decl);
                self.query_condition(right, var, decl);
            }
            ExprKind::Unary { op: UnaryOp::Not, operand } => self.query_condition(operand, var, decl),
            // `a matches x`: the words of x in the record's searched fields.
            ExprKind::Binary { op: BinaryOp::Matches, left, right } => {
                if !matches!(&left.kind, ExprKind::Name { name } if name == var) {
                    return self.error(format!("'matches' looks for words in the records found, as in '{var} matches x'"), expr.span);
                }
                let words = self.expr(right, Some(Ty::Text));
                if !words.is_unknown() && !matches!(words, Ty::Text) {
                    self.error(format!("'matches' looks for text, not {}", words.name()), right.span);
                }
                if !decl.fields.iter().any(|f| f.modifiers.search) {
                    self.error(format!("{} has no field marked search to look for words in", decl.name.name), expr.span);
                }
                self.out.types.types.insert((expr.span.start, expr.span.end), TypeInfo::Bool);
            }
            ExprKind::Binary { op, left, right } if is_comparison(*op) => {
                let sides = (self.query_field(left, var), self.query_field(right, var));
                let (field, value) = match sides {
                    (Some(Ok(())), None) => (left, right),
                    (None, Some(Ok(()))) => (right, left),
                    (Some(Err(message)), _) | (_, Some(Err(message))) => return self.error(message, expr.span),
                    _ => return self.error(general, expr.span),
                };
                let field_ty = self.expr(field, None);
                // A list is looked into for one item: `a.tags contains "live"`.
                if let Ty::Values(item) = field_ty {
                    if *op != BinaryOp::Contains || !std::ptr::eq(field, left) {
                        return self.error(format!("a query looks into {} with contains, as in 'a.tags contains x'", field_ty.name()), expr.span);
                    }
                    let value_ty = self.expr(value, Some(item_ty(item)));
                    if !item_ty(item).accepts(&value_ty) {
                        self.error(format!("cannot look for {} in {}", value_ty.name(), field_ty.name()), expr.span);
                    }
                    self.out.types.types.insert((expr.span.start, expr.span.end), TypeInfo::Bool);
                    return;
                }
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
    /// `Ok` when it is one of its fields (`c.name`) or a field of a record
    /// its references lead to (`o.customer.name`), and the reason when it
    /// uses the record some other way.
    fn query_field(&self, expr: &Expr, var: &str) -> Option<Result<(), String>> {
        if !mentions(expr, var) {
            return None;
        }
        let general = || Some(Err(format!("a query compares a field of {var} with a value, as in '{var}.name == x'")));
        let mut members = Vec::new();
        let mut at = expr;
        while let ExprKind::Member { object, member } = &at.kind {
            members.push(member);
            at = object;
        }
        match &at.kind {
            ExprKind::Name { name } if name == var && !members.is_empty() => {}
            _ => return general(),
        }
        members.reverse();
        let Some(Ty::Record(entity)) = self.local(var) else { return Some(Ok(())) };
        match self.through(entity, var, &members[..members.len() - 1]) {
            Ok(_) => Some(Ok(())),
            Err(message) => Some(Err(message)),
        }
    }

    /// The entity the reference fields `hops` of a record of `entity`
    /// (named `var`) lead to; `None` when one of them has an unknown type,
    /// which is reported where it is declared.
    fn through(&self, entity: &'a Entity, var: &str, hops: &[&Ident]) -> Result<Option<&'a Entity>, String> {
        let mut current = entity;
        let mut path = var.to_string();
        for hop in hops {
            path = format!("{path}.{}", hop.name);
            let Some(field) = current.field(&hop.name) else {
                return Err(format!("{} has no field '{}'", current.name.name, hop.name));
            };
            match self.env.field_type(&field.ty) {
                Ty::Record(next) => current = next,
                Ty::Unknown => return Ok(None),
                other => {
                    return Err(format!(
                        "'{path}' is {}, not a reference; a query goes through references only",
                        described(&other)
                    ));
                }
            }
        }
        Ok(Some(current))
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

    /// `none` where a value of `expected` goes: the empty value of a date,
    /// a date and time or a reference, and of nothing else.
    fn empty(&mut self, expected: Option<Ty<'a>>, span: Span) -> Ty<'a> {
        match expected {
            Some(ty @ (Ty::Date | Ty::DateTime | Ty::Record(_) | Ty::Image | Ty::RecordFile | Ty::Unknown)) => ty,
            Some(Ty::Text) => {
                self.error("text is never none; empty text is \"\"", span);
                Ty::Unknown
            }
            Some(other) => {
                self.error(format!("{} is never none; only a date, a date and time or a reference can be", other.name()), span);
                Ty::Unknown
            }
            None => {
                self.error("'none' needs a date, a date and time or a reference to be the empty value of", span);
                Ty::Unknown
            }
        }
    }

    fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr) -> Ty<'a> {
        let symbol = op.symbol();
        // `none == x` reads as `x == none`.
        if matches!(left.kind, ExprKind::Empty) && !matches!(right.kind, ExprKind::Empty) && is_comparison(op) {
            return self.binary(op, right, left);
        }
        if is_comparison(op) && !matches!(op, BinaryOp::Eq | BinaryOp::NotEq) && matches!(right.kind, ExprKind::Empty) {
            self.expr(left, None);
            self.error(format!("'{symbol}' cannot compare with none; use == or !="), right.span);
            return Ty::Bool;
        }
        match op {
            BinaryOp::Matches => {
                self.expr(left, None);
                self.expr(right, None);
                self.error(
                    "'matches' looks for words in the records a find looks through, as in 'find a in Album where a matches x'",
                    join(left.span, right.span),
                );
                Ty::Bool
            }
            BinaryOp::Contains => {
                let l = self.expr(left, None);
                // `album.tags contains "jazz"`: whether a list holds an item.
                if let Ty::Values(item) = l {
                    let r = self.expr(right, Some(item_ty(item)));
                    if !item_ty(item).accepts(&r) {
                        self.error(format!("cannot look for {} in {}", r.name(), l.name()), join(left.span, right.span));
                    }
                    return Ty::Bool;
                }
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
                    && (l.is_value() || matches!(right.kind, ExprKind::Empty))
                    && (!ordered || l.is_unknown() || matches!(l, Ty::Text | Ty::Date | Ty::DateTime | Ty::Duration) || l.is_numeric());
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
                // Durations add up and scale: 3:45 + 0:15, 3:45 * 2, 3:45 / 2.
                if matches!(l, Ty::Duration) || matches!(r, Ty::Duration) {
                    let fits = match op {
                        BinaryOp::Add | BinaryOp::Sub => matches!((l, r), (Ty::Duration | Ty::Integer, Ty::Duration | Ty::Integer)),
                        BinaryOp::Mul => matches!((l, r), (Ty::Duration, Ty::Integer) | (Ty::Integer, Ty::Duration)),
                        _ => matches!((l, r), (Ty::Duration, Ty::Integer)),
                    };
                    if !fits {
                        self.error(format!("cannot use '{symbol}' on {} and {}", l.name(), r.name()), join(left.span, right.span));
                        return Ty::Unknown;
                    }
                    return Ty::Duration;
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

/// A type as a sentence names it: "text", "money", "a date", "an integer".
fn described(ty: &Ty) -> String {
    match ty {
        Ty::Text | Ty::Money | Ty::Bool => ty.name(),
        other => format!("{} {}", article(&other.name()), other.name()),
    }
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
        ExprKind::Call { args, .. } | ExprKind::EngineCall { args, .. } => args.iter().any(|a| mentions(a, var)),
        // An inner query has its own record name.
        ExprKind::Find { var: inner, filter, .. } | ExprKind::Files { var: inner, filter, .. } => {
            inner.name != var && filter.as_deref().is_some_and(|f| mentions(f, var))
        }
        ExprKind::ReadText { file } | ExprKind::ReadBytes { file } => mentions(file, var),
        ExprKind::Ask { question, form } => mentions(question, var) || crate::code::ask_exprs(form).iter().any(|e| mentions(e, var)),
        ExprKind::New { .. } => false,
        ExprKind::Text { parts } => parts.iter().any(|p| matches!(p, TextPart::Slot { value } if mentions(value, var))),
        ExprKind::Key { args, .. } => args.iter().any(|a| mentions(&a.value, var)),
        ExprKind::ListOf { items } => items.iter().any(|i| mentions(i, var)),
        ExprKind::Index { list, index } => mentions(list, var) || mentions(index, var),
        ExprKind::Number { .. } | ExprKind::Str { .. } | ExprKind::Bool { .. } | ExprKind::Empty => false,
    }
}

/// Checkboxes and switches, which hold whether they are on when they are not bound.
fn has_checked(props: &WidgetProps) -> bool {
    matches!(props.kind, WidgetKind::Checkbox | WidgetKind::Switch | WidgetKind::Toggle)
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
            .flat_map(|b| b.handlers.iter().map(|h| &h.body).chain(b.events.iter().map(|e| &e.body)))
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

/// A query runs in one database: its condition and order may go through
/// references only to records of the same data source.
pub(crate) fn check_query_sources(project: &mut crate::model::Project) {
    if project.sources.len() < 2 {
        return;
    }
    let mut found = Vec::new();
    for file in &project.files {
        let bodies = file
            .code
            .iter()
            .flat_map(|b| b.handlers.iter().map(|h| &h.body).chain(b.events.iter().map(|e| &e.body)))
            .chain(file.functions.iter().map(|f| &f.body));
        let mut check = |expr: &Expr| {
            let ExprKind::Find { var, entity, filter, order } = &expr.kind else { return };
            let mut paths: Vec<(Vec<Ident>, Span)> = order.iter().filter(|k| k.var.name == var.name).map(|k| (k.through.clone(), join(k.var.span, k.field.span))).collect();
            if let Some(filter) = filter {
                each_expr(filter, &mut |e| {
                    if let Some(hops) = member_hops(e, &var.name) {
                        paths.push((hops.into_iter().cloned().collect(), e.span));
                    }
                });
            }
            for (hops, span) in paths {
                if let Some(message) = other_source(project, &entity.name, &var.name, &hops) {
                    found.push(Diagnostic::error(message, span).in_file(&file.path));
                }
            }
        };
        for body in bodies {
            each_stmt_expr(body, &mut check);
        }
        for function in &file.functions {
            for e in function.requires.iter().chain(&function.ensures) {
                each_expr(e, &mut check);
            }
        }
    }
    project.diagnostics.extend(found);
}

/// The reference fields a member path of the query's record goes through
/// before its last field: `o.customer.name` gives `[customer]`. Only the
/// longest path counts, not the paths inside it.
fn member_hops<'e>(expr: &'e Expr, var: &str) -> Option<Vec<&'e Ident>> {
    let mut members = Vec::new();
    let mut at = expr;
    while let ExprKind::Member { object, member } = &at.kind {
        members.push(member);
        at = object;
    }
    match &at.kind {
        ExprKind::Name { name } if name == var && members.len() > 1 => {
            members.reverse();
            members.pop();
            Some(members)
        }
        _ => None,
    }
}

/// Why going through `hops` from a record of `entity` leaves its data
/// source, if it does.
fn other_source(project: &crate::model::Project, entity: &str, var: &str, hops: &[Ident]) -> Option<String> {
    let home = project.source_of(entity)?;
    let mut current = project.entity(entity)?;
    let mut path = var.to_string();
    for hop in hops {
        path = format!("{path}.{}", hop.name);
        let FieldType::Ref { entity: target } = &current.field(&hop.name)?.ty else { return None };
        let source = project.source_of(&target.name)?;
        if source != home {
            return Some(format!("'{path}' refers to {} in data source '{source}'; a query reads one database", target.name));
        }
        current = project.entity(&target.name)?;
    }
    None
}

/// The file stores code reaches, by file: every `files ... in store` and
/// `write ... to store`.
pub(crate) fn code_stores(project: &crate::model::Project) -> Vec<(String, Ident)> {
    fn writes(body: &[Stmt], found: &mut Vec<Ident>) {
        for stmt in body {
            match &stmt.kind {
                StmtKind::Write { store, .. } => found.push(store.clone()),
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
    let mut out = Vec::new();
    for file in &project.files {
        let bodies = file
            .code
            .iter()
            .flat_map(|b| b.handlers.iter().map(|h| &h.body).chain(b.events.iter().map(|e| &e.body)))
            .chain(file.functions.iter().map(|f| &f.body));
        let mut stores = Vec::new();
        for body in bodies {
            each_stmt_expr(body, &mut |e| {
                if let ExprKind::Files { store, .. } = &e.kind {
                    stores.push(store.clone());
                }
            });
            writes(body, &mut stores);
        }
        out.extend(stores.into_iter().map(|s| (file.path.clone(), s)));
    }
    out
}

/// Calls `found` for every expression in `body` and in the expressions
/// inside it, nested blocks included.
fn each_stmt_expr(body: &[Stmt], found: &mut impl FnMut(&Expr)) {
    for stmt in body {
        match &stmt.kind {
            StmtKind::Let { value, .. } | StmtKind::Save { value } | StmtKind::Delete { value } => each_expr(value, found),
            StmtKind::Assign { target, value } => {
                each_expr(target, found);
                each_expr(value, found);
            }
            StmtKind::Return { value } => {
                if let Some(value) = value {
                    each_expr(value, found);
                }
            }
            StmtKind::If { branches, otherwise } => {
                for branch in branches {
                    each_expr(&branch.condition, found);
                    each_stmt_expr(&branch.body, found);
                }
                if let Some(otherwise) = otherwise {
                    each_stmt_expr(otherwise, found);
                }
            }
            StmtKind::For { list, body, .. } => {
                each_expr(list, found);
                each_stmt_expr(body, found);
            }
            StmtKind::Call { call } => each_expr(call, found),
            StmtKind::Draw { args, .. } | StmtKind::Hear { args, .. } => {
                for arg in args {
                    each_expr(arg, found);
                }
            }
            StmtKind::Message { text, .. } | StmtKind::Refuse { text } => each_expr(text, found),
            StmtKind::Ask { question, form, args, .. } => {
                if let Some(question) = question {
                    each_expr(question, found);
                }
                for e in super::ask_exprs(form) {
                    each_expr(e, found);
                }
                for arg in args {
                    each_expr(arg, found);
                }
            }
            StmtKind::Open { with, .. } => {
                for arg in with {
                    each_expr(&arg.value, found);
                }
            }
            StmtKind::Write { text, path, .. } => {
                each_expr(text, found);
                each_expr(path, found);
            }
            StmtKind::CloseWith { value } | StmtKind::Print { value, .. } | StmtKind::Exit { code: value } => each_expr(value, found),
            StmtKind::Close | StmtKind::Reload { .. } => {}
        }
    }
}

/// Calls `found` for every statement in `body`, nested blocks included.
fn each_stmt(body: &[Stmt], found: &mut impl FnMut(&Stmt)) {
    for stmt in body {
        found(stmt);
        match &stmt.kind {
            StmtKind::If { branches, otherwise } => {
                for branch in branches {
                    each_stmt(&branch.body, found);
                }
                if let Some(otherwise) = otherwise {
                    each_stmt(otherwise, found);
                }
            }
            StmtKind::For { body, .. } => each_stmt(body, found),
            _ => {}
        }
    }
}

/// Calls `found` for `expr` and every expression inside it, outer first.
fn each_expr(expr: &Expr, found: &mut impl FnMut(&Expr)) {
    found(expr);
    match &expr.kind {
        // A member path is one value; its inner paths are not visited.
        ExprKind::Member { .. } => {}
        ExprKind::Unary { operand, .. } => each_expr(operand, found),
        ExprKind::Binary { left, right, .. } => {
            each_expr(left, found);
            each_expr(right, found);
        }
        ExprKind::In { value, options } => {
            each_expr(value, found);
            for option in options {
                each_expr(option, found);
            }
        }
        ExprKind::Call { args, .. } | ExprKind::EngineCall { args, .. } => {
            for arg in args {
                each_expr(arg, found);
            }
        }
        ExprKind::Find { filter, .. } | ExprKind::Files { filter, .. } => {
            if let Some(filter) = filter {
                each_expr(filter, found);
            }
        }
        ExprKind::ReadText { file } | ExprKind::ReadBytes { file } => each_expr(file, found),
        ExprKind::Ask { question, form } => {
            each_expr(question, found);
            for e in super::ask_exprs(form) {
                each_expr(e, found);
            }
        }
        ExprKind::Key { args, .. } => {
            for arg in args {
                each_expr(&arg.value, found);
            }
        }
        ExprKind::ListOf { items } => {
            for item in items {
                each_expr(item, found);
            }
        }
        ExprKind::Index { list, index } => {
            each_expr(list, found);
            each_expr(index, found);
        }
        ExprKind::Text { parts } => {
            for part in parts {
                if let TextPart::Slot { value } = part {
                    each_expr(value, found);
                }
            }
        }
        ExprKind::Name { .. } | ExprKind::Number { .. } | ExprKind::Str { .. } | ExprKind::Bool { .. } | ExprKind::Empty | ExprKind::New { .. } => {}
    }
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

/// Whether `expr` joins a piece of literal text with something else
/// through `+`: `"Saved " + customer.name`.
fn joins_text(expr: &Expr) -> bool {
    let ExprKind::Binary { op: BinaryOp::Add, left, right } = &expr.kind else { return false };
    let literal = |e: &Expr| matches!(e.kind, ExprKind::Str { .. });
    literal(left) || literal(right) || joins_text(left) || joins_text(right)
}

/// The type of one item of a list of plain values.
fn item_ty(item: Item<'_>) -> Ty<'_> {
    match item {
        Item::Text => Ty::Text,
        Item::Integer => Ty::Integer,
        Item::Decimal => Ty::Decimal,
        Item::Bool => Ty::Bool,
        Item::Date => Ty::Date,
        Item::Enum(e) => Ty::Enum(e),
    }
}
