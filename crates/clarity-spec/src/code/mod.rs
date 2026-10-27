//! Code behind: the handlers of a window or the app, written in Clarity
//! (docs/DESIGN.md section 5.9). This module holds the syntax tree; `parse`
//! reads `code` blocks and `check` resolves names and types against the
//! window they belong to.

pub mod check;
pub(crate) mod parse;
pub mod edit;
pub mod print;

use serde::Serialize;
use ts_rs::TS;

use crate::model::Ident;
use crate::span::Span;

/// `code Name` with the handlers of the window or app called Name, or the
/// data events of the entity called Name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CodeBlock {
    pub name: Ident,
    pub handlers: Vec<HandlerDecl>,
    pub events: Vec<EventDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct HandlerDecl {
    pub name: Ident,
    /// The values a click gives it: `handler digit(d: integer)`.
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// When a data event runs: around storing or removing a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DataEvent {
    BeforeSave,
    AfterSave,
    BeforeDelete,
    AfterDelete,
}

impl DataEvent {
    pub const ALL: [DataEvent; 4] = [Self::BeforeSave, Self::AfterSave, Self::BeforeDelete, Self::AfterDelete];

    /// The event as written: `before save`.
    pub fn text(self) -> &'static str {
        match self {
            Self::BeforeSave => "before save",
            Self::AfterSave => "after save",
            Self::BeforeDelete => "before delete",
            Self::AfterDelete => "after delete",
        }
    }

    /// The event as one word, for names in generated code: `before_save`.
    pub fn word(self) -> &'static str {
        match self {
            Self::BeforeSave => "before_save",
            Self::AfterSave => "after_save",
            Self::BeforeDelete => "before_delete",
            Self::AfterDelete => "after_delete",
        }
    }

    /// Whether the event is about saving, rather than deleting.
    pub fn saves(self) -> bool {
        matches!(self, Self::BeforeSave | Self::AfterSave)
    }
}

/// `before save` and the other data events, in the code block of an
/// entity. The record is named after the entity in snake case: `order`
/// in the events of `Order`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct EventDecl {
    pub event: DataEvent,
    pub body: Vec<Stmt>,
    /// The line that names the event.
    pub head: Span,
    pub span: Span,
}

/// The name of the record in the events of `entity`.
pub fn record_name(entity: &str) -> String {
    crate::model::snake_case(entity)
}

impl EventDecl {
    /// The event as the action it amounts to, so the checker and the
    /// generators treat it as one: it takes the record, and it may read
    /// and write records.
    pub fn action(&self, entity: &str) -> FunctionDecl {
        let ident = |name: String| Ident { name, span: self.head };
        let record = record_name(entity);
        FunctionDecl {
            kind: FunctionKind::Event,
            name: ident(format!("{}_{record}", self.event.word())),
            params: vec![Param {
                name: ident(record),
                ty: TypeRef {
                    kind: TypeRefKind::Named { name: entity.to_string() },
                    span: self.head,
                },
                default: None,
            }],
            returns: None,
            requires: Vec::new(),
            ensures: Vec::new(),
            effects: EFFECTS.iter().map(|e| ident(e.to_string())).collect(),
            body: self.body.clone(),
            span: self.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum StmtKind {
    /// `let name = value`, or `let name: type = value`.
    Let {
        name: Ident,
        ty: Option<TypeRef>,
        value: Expr,
    },
    /// `target = value`; the target is a name or a member path.
    Assign {
        target: Expr,
        value: Expr,
    },
    /// `if` with its `else if` branches, and the `else` block.
    If {
        branches: Vec<Branch>,
        otherwise: Option<Vec<Stmt>>,
    },
    /// `return`, or `return value` in a function or action that returns one.
    Return {
        value: Option<Expr>,
    },
    /// `for name in list` with its block.
    For {
        var: Ident,
        list: Expr,
        body: Vec<Stmt>,
    },
    /// A call on its own line: `approve(order)`.
    Call {
        call: Expr,
    },
    /// A shape on a canvas, in its draw handler: `dial.line(0, 0, 10, 10, "text")`.
    Draw {
        canvas: Ident,
        shape: Ident,
        args: Vec<Expr>,
    },
    /// A hear of a custom part (DESIGN 5.19): `arena.restart(2)`. Read as a
    /// `Draw` and made a hear when loading, where the window's parts are known.
    Hear {
        part: Ident,
        hear: Ident,
        args: Vec<Expr>,
    },
    Message {
        level: MessageLevel,
        text: Expr,
    },
    /// `ask @question ... then handler(values)` (DESIGN 5.14): Clarity's
    /// dialog, shown when the handler has finished. The answer runs `then`
    /// with `args` and, for a value, the answer last; cancelling runs nothing.
    Ask {
        /// None when a dialog window asks: its own title and parts say what.
        question: Option<Expr>,
        form: Box<AskForm>,
        then: Ident,
        args: Vec<Expr>,
    },
    Open {
        window: Ident,
        with: Vec<Arg>,
    },
    Close,
    /// `close with value`: a dialog closes and gives `value` to the handler that asked for it.
    CloseWith {
        value: Expr,
    },
    Save {
        value: Expr,
    },
    /// `reload customers`: loads a window's data again, as when it opened.
    Reload {
        data: Ident,
    },
    Delete {
        value: Expr,
    },
    /// `refuse "text"`: stops with that text as the error, so a save or
    /// delete it is part of does not happen.
    Refuse {
        text: Expr,
    },
    /// `write lines to photos "exports/titles.txt"`: writes text as a file
    /// of a file store, at a path inside it.
    Write {
        text: Expr,
        store: Ident,
        path: Expr,
    },
    /// `print value`, or `print list columns a, b` (a table): writes to
    /// standard output, in a command (DESIGN 5.20).
    Print {
        value: Expr,
        columns: Vec<Ident>,
    },
    /// `exit code`: ends a command-line program with that exit code.
    Exit {
        code: Expr,
    },
    /// `tell finished(time)` in an engine beside the screen (DESIGN 5.12):
    /// a tell with its values, in the answer of the call it is in.
    Tell {
        tell: Ident,
        args: Vec<Expr>,
    },
}

/// What `ask` asks for, which decides its dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum AskForm {
    /// `yes @delete no @keep`: a question with two buttons (Clarity's OK
    /// and Cancel when left out); `then` runs on yes and takes no answer.
    Confirm { yes: Option<Expr>, no: Option<Expr> },
    /// `for type [default value] [from list]`: a value of a type, typed in
    /// or chosen: text, a number, a date, a duration, an enum's value or a
    /// record (of the entity, or of the list of records after `from`).
    Value { ty: TypeRef, default: Option<Expr>, from: Option<Expr> },
    /// `ask PickFriend with name = value, ...`: a dialog window of the
    /// application; the answer is what it closes with (`close with value`).
    Window { window: Ident, with: Vec<Arg> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Branch {
    pub condition: Expr,
    pub body: Vec<Stmt>,
}

/// `name = value` in `open Window with ...`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Arg {
    pub name: Ident,
    pub value: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
}

impl MessageLevel {
    pub fn by_keyword(word: &str) -> Option<Self> {
        match word {
            "info" => Some(Self::Info),
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum ExprKind {
    /// As written: `12`, `-3`, `9.95`.
    Number {
        value: String,
    },
    Str {
        value: String,
    },
    /// Text with values in slots: `"Saved {customer.name}"`. Only text
    /// with at least one slot is this; other text is `Str`.
    Text {
        parts: Vec<TextPart>,
    },
    Bool {
        value: bool,
    },
    /// `none`: the empty value of a date, a date and time or a reference.
    Empty,
    Name {
        name: String,
    },
    /// `object.member`
    Member {
        object: Box<Expr>,
        member: Ident,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// `value in [a, b]`
    In {
        value: Box<Expr>,
        options: Vec<Expr>,
    },
    /// `greeting("Ann", times: 3)`: the values in order, then those given
    /// by name. Generation fills in the rest from the defaults
    /// (clarity_gen::calls), so a stack sees every value in order.
    Call {
        function: Ident,
        args: Vec<Expr>,
        named: Vec<NamedArg>,
    },
    /// A function of an engine: `Pricing.total(lines)` (DESIGN 5.11).
    EngineCall {
        engine: Ident,
        function: Ident,
        args: Vec<Expr>,
    },
    /// `ask @question yes @a no @b` or `ask @question for type ...` in a
    /// command (DESIGN 5.20): waits for the answer on the terminal and
    /// gives it, a bool for yes or no.
    Ask {
        question: Box<Expr>,
        form: Box<AskForm>,
    },
    /// `find c in Customer where c.active order by c.name`: the records of
    /// an entity that meet the condition, in order. `var` names the record
    /// inside the condition and the order.
    Find {
        var: Ident,
        entity: Ident,
        filter: Option<Box<Expr>>,
        order: Vec<OrderKey>,
    },
    /// `files f in photos where f.extension == "jpg" order by f.path`: the
    /// files of a file store, subfolders too, that meet the condition, in
    /// order. `var` names the file inside the condition and the order.
    Files {
        var: Ident,
        store: Ident,
        filter: Option<Box<Expr>>,
        order: Vec<OrderKey>,
    },
    /// `read text of f`: the text a file holds.
    ReadText { file: Box<Expr> },
    /// `read bytes of f`: the contents of a file of a file store, as they are.
    ReadBytes { file: Box<Expr> },
    /// `new Photo`: a new record of the entity, with its defaults.
    New { entity: Ident },
    /// A text by its key, in the person's language, its slots filled by
    /// name: `@orderApproved(number = order.number)`.
    Key { key: Ident, args: Vec<Arg> },
    /// A list of values as written: `[0, 0, 0]`.
    ListOf { items: Vec<Expr> },
    /// The value at a position of a list, from 1: `squares[3]`.
    Index { list: Box<Expr>, index: Box<Expr> },
    /// `json of order`: a record, a list or a value as Clarity's JSON (DESIGN 5.23).
    JsonOf { value: Box<Expr> },
    /// `read list<Order> from json text at "data.orders"`: records read from
    /// JSON, fitted to the entity first.
    ReadJson {
        ty: TypeRef,
        text: Box<Expr>,
        at: Option<Box<Expr>>,
    },
    /// `http post url with body header "Accept": "text/plain"`: a call to a
    /// web API, giving its reply (DESIGN 5.23). Only post and put send a body.
    /// `from shop` calls a path of that web API data source, at its address
    /// and with its auth; `query "status": s` adds an encoded query parameter.
    Http {
        method: HttpMethod,
        url: Box<Expr>,
        from: Option<Ident>,
        body: Option<Box<Expr>>,
        headers: Vec<HttpPair>,
        query: Vec<HttpPair>,
    },
}

/// The kind of an HTTP call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl HttpMethod {
    /// Every method, as code writes it.
    pub const ALL: [HttpMethod; 4] = [HttpMethod::Get, HttpMethod::Post, HttpMethod::Put, HttpMethod::Delete];

    /// As code writes it: `get`.
    pub fn word(self) -> &'static str {
        match self {
            HttpMethod::Get => "get",
            HttpMethod::Post => "post",
            HttpMethod::Put => "put",
            HttpMethod::Delete => "delete",
        }
    }

    /// As HTTP names it: `GET`.
    pub fn verb(self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
        }
    }

    /// Whether the call sends a body: post and put do.
    pub fn sends_body(self) -> bool {
        matches!(self, HttpMethod::Post | HttpMethod::Put)
    }
}

/// The expressions of an HTTP call: its url, its body, and its headers' and
/// query parameters' values.
pub fn http_exprs<'e>(url: &'e Expr, body: Option<&'e Expr>, headers: &'e [HttpPair], query: &'e [HttpPair]) -> Vec<&'e Expr> {
    std::iter::once(url)
        .chain(body)
        .chain(headers.iter().chain(query).flat_map(|h| std::iter::once(&h.value).chain(&h.when)))
        .collect()
}

/// `header "Accept": "text/plain"` or `query "status": s` of an HTTP call:
/// the name as written, its value, a text, and the condition after `when`
/// that leaves it out of the call when it does not hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct HttpPair {
    pub name: Ident,
    pub value: Expr,
    pub when: Option<Expr>,
}

/// `limit: 10` in a call: a value given by its parameter's name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct NamedArg {
    pub name: Ident,
    pub value: Expr,
}

/// `c.name` or `c.name desc` in `order by`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct OrderKey {
    /// The record name before the dot, as written.
    pub var: Ident,
    /// The reference fields between the record and the field, as in
    /// `a.artist.name`: the field is of the record they lead to.
    pub through: Vec<Ident>,
    pub field: Ident,
    pub descending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BinaryOp {
    Or,
    And,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Add,
    Sub,
    Mul,
    Div,
    /// `text contains part`, ignoring case; or whether a list of values holds an item.
    Contains,
    /// `a matches x` in a find: the record holds every word of `x`, each as
    /// the start of a word, in its fields marked `search`.
    Matches,
}

impl BinaryOp {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Or => "or",
            Self::And => "and",
            Self::Eq => "==",
            Self::NotEq => "!=",
            Self::Lt => "<",
            Self::LtEq => "<=",
            Self::Gt => ">",
            Self::GtEq => ">=",
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Contains => "contains",
            Self::Matches => "matches",
        }
    }
}

/// A type as written in code: in parameters, results and typed locals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct TypeRef {
    pub kind: TypeRefKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum TypeRefKind {
    Text,
    Integer,
    Decimal,
    Money,
    Bool,
    Date,
    DateTime,
    Duration,
    /// A file of a file store.
    File,
    /// The contents of a file as they are (`read bytes of f`).
    Bytes,
    /// An entity (a record) or an enum.
    Named { name: String },
    List { item: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FunctionKind {
    /// No effects: computes a value from its arguments.
    Function,
    /// May have the effects it declares, such as saving records.
    Action,
    /// A data event of an entity: an action that runs when its record
    /// is saved or deleted.
    Event,
}

impl FunctionKind {
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Action => "action",
            Self::Event => "event",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Param {
    pub name: Ident,
    pub ty: TypeRef,
    /// `limit: integer = 0`: the value a call that leaves it out gives.
    pub default: Option<Expr>,
}

/// `part Arena` (DESIGN 5.19): a part an outside library draws, with what
/// the rest of the project sees of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PartDecl {
    pub name: Ident,
    /// Where the library is; none written is an error the checker reports.
    pub library: Option<PartLibrary>,
    /// `binds integer`: the type of value it shows and edits.
    pub binds: Option<TypeRef>,
    /// `speed: integer, min 1, max 10, default 3`.
    pub properties: Vec<PartProperty>,
    /// `tells scored(points: integer)`: its own events, with their values.
    pub tells: Vec<PartMessage>,
    /// `hears restart(level: integer)`: what code tells it to do.
    pub hears: Vec<PartMessage>,
    /// `size 8 x 6`: the span and rows the Parts palette drops it with.
    pub size: Option<(u32, u32)>,
    /// `preview "arena.png"`: its picture on the design canvas, from the resources.
    pub preview: Option<String>,
    pub span: Span,
}

/// `library Words` (DESIGN 5.21): an outside library in the project's own
/// language, with the functions code calls, written by the adapter in
/// `libraries/Words/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct LibraryDecl {
    pub name: Ident,
    /// Its package; none written is an error the checker reports.
    pub source: Option<LibrarySource>,
    pub functions: Vec<EngineFunction>,
    pub span: Span,
}

/// Where a library comes from, in the project's language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum LibrarySource {
    /// `nuget "Humanizer.Core" 2.14.1` (C#).
    Nuget { package: String, version: String },
    /// `dll "libs/Plot.dll"` (C#).
    Dll { file: String },
    /// `crate "humanize" "0.1"` (Rust).
    Crate { name: String, version: String },
    /// `go "github.com/dustin/go-humanize" v1.0.1` (Go).
    Go { module: String, version: String },
    /// `cmake "SFML"`: what the adapter's CMake finds (C, C++).
    Cmake { package: String },
}

/// The library that draws a custom part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum PartLibrary {
    /// `native "parts/Arena"`: C or C++ built from that folder with CMake (the C and C++ stacks).
    Native { folder: String },
    /// `crate "parts/Arena"`: a Rust crate in that folder (the Rust stack).
    Crate { folder: String },
    /// `go "parts/arena"`: a Go package in that folder, with its go.mod (the Go stack).
    Go { folder: String },
    /// `nuget "AvaloniaEdit" 11.1.0`: an Avalonia control from a NuGet package.
    Nuget { package: String, version: String },
    /// `dll "libs/Plot.dll"`: an Avalonia control from a .NET assembly.
    Dll { file: String },
}

/// A property of a custom part, written as a setting is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PartProperty {
    pub name: Ident,
    pub ty: TypeRef,
    pub default: Option<Expr>,
    pub min: Option<Expr>,
    pub max: Option<Expr>,
    pub span: Span,
}

/// A tell or a hear of a custom part: its name and its values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PartMessage {
    pub name: Ident,
    pub params: Vec<Param>,
    pub span: Span,
}

/// The `cli Shop` block (DESIGN 5.20): the command-line program of a
/// project with the `cli` target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CliDecl {
    pub name: Ident,
    /// `help @key`: the program's description in its help.
    pub help: Option<Expr>,
    pub span: Span,
}

/// `command [group] name(arguments)` with its `option` lines and its
/// statements (DESIGN 5.20).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CommandDecl {
    /// `customer` in `command customer add`.
    pub group: Option<Ident>,
    pub name: Ident,
    /// The arguments, given in this order, all required.
    pub params: Vec<Param>,
    pub help: Option<Expr>,
    pub options: Vec<CommandOption>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// `option limit: money, default 1000, min 0, max 5000, required, help @key`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CommandOption {
    pub name: Ident,
    pub ty: TypeRef,
    pub default: Option<Expr>,
    pub min: Option<Expr>,
    pub max: Option<Expr>,
    pub required: bool,
    pub help: Option<Expr>,
    pub span: Span,
}

/// `addAll` as the command line writes it: `add-all`.
pub fn hyphenated(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('-');
            }
            out.extend(c.to_lowercase());
        } else if c == '_' {
            out.push('-');
        } else {
            out.push(c);
        }
    }
    out
}

impl CommandDecl {
    /// The command as typed after the program's name: `customer add`.
    pub fn line(&self) -> String {
        let name = hyphenated(&self.name.name);
        match &self.group {
            Some(group) => format!("{} {name}", hyphenated(&group.name)),
            None => name,
        }
    }
}

impl CommandOption {
    /// `--dry-run` for `dryRun`.
    pub fn flag(&self) -> String {
        format!("--{}", hyphenated(&self.name.name))
    }
}

/// A `function` or `action` declared in a module (docs/DESIGN.md 5.10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct FunctionDecl {
    pub kind: FunctionKind,
    pub name: Ident,
    pub params: Vec<Param>,
    pub returns: Option<TypeRef>,
    /// Conditions on the arguments, checked before the body runs.
    pub requires: Vec<Expr>,
    /// Conditions on `result`, checked after the body.
    pub ensures: Vec<Expr>,
    /// Effects an action may have, such as `db.write`.
    pub effects: Vec<Ident>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// An engine (DESIGN 5.11): a processing module compiled to WebAssembly,
/// its file in the project's `engines` folder, and its functions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct EngineDecl {
    pub name: Ident,
    /// The file in the `engines` folder: `pricing.wasm`.
    pub file: String,
    /// `uses clock, random, log, language`: the services of Clarity it may import.
    pub uses: Vec<Ident>,
    /// `says "You win", "A draw"`: the texts it gives, translated like the project's own.
    pub says: Vec<String>,
    /// `state per app` or `state per window`: what an instance serves; none
    /// keeps no state.
    pub state: EngineState,
    /// `on open`: the engine's `clarity_open` runs on every new instance.
    pub opens: bool,
    /// `beside the screen` (DESIGN 5.12): it runs in the renderer, where the
    /// canvas that `runs` it is drawn, one instance per canvas; the backend
    /// never runs it.
    pub beside: bool,
    /// `tells finished(score: integer)`: what an engine beside the screen
    /// tells its window, which runs the handler wired on the canvas.
    pub tells: Vec<PartMessage>,
    /// `hears start(level: integer)`: what a handler tells it, through the
    /// canvas (`sky.start(2)`); its file has a function of each name.
    pub hears: Vec<PartMessage>,
    /// `limit 2 seconds, 64 MB`: how long one call may run; none written
    /// keeps the default (`engines::DEFAULT_SECONDS`).
    pub seconds: Option<u32>,
    /// How much memory the engine may grow to; none written keeps the
    /// default (`engines::DEFAULT_MEGABYTES`).
    pub megabytes: Option<u32>,
    pub functions: Vec<EngineFunction>,
    /// An engine written in Clarity (no `from` file): what it keeps and
    /// its functions with their code. Every stack runs it as generated code.
    pub body: Option<EngineBody>,
    pub span: Span,
}

/// The Clarity of an engine written in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct EngineBody {
    /// Its constants: `const width = 160`.
    #[serde(default)]
    pub consts: Vec<ConstDecl>,
    /// What an instance keeps between calls: `keep squares: list<integer> = [0, 0, 0]`.
    pub keeps: Vec<Keep>,
    /// Its functions, with their code; `functions` of the engine are their signatures.
    pub functions: Vec<FunctionDecl>,
}

/// `const lives = 3`: a value written out, read by name and never set
/// (DESIGN 5.12), in a module or an engine written in Clarity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ConstDecl {
    pub name: Ident,
    /// `const speed: decimal = 1.5`; none takes the value's own type.
    pub ty: Option<TypeRef>,
    /// `const pill: picture = "pill.svg"`: text naming a picture of the
    /// resources folder, which the checker finds there.
    pub picture: bool,
    pub value: Expr,
    pub span: Span,
}

/// A value an engine written in Clarity keeps between calls, and what it starts as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Keep {
    pub name: Ident,
    pub ty: TypeRef,
    pub value: Expr,
    pub span: Span,
}

/// Whether an engine keeps an instance from call to call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum EngineState {
    /// Every call runs on a fresh instance.
    #[default]
    None,
    /// `state per app`: one instance serves every call of the application.
    App,
    /// `state per window`: each open window has an instance of its own.
    Window,
}

impl EngineDecl {
    /// How long one call may run, in seconds.
    pub fn seconds(&self) -> u32 {
        self.seconds.unwrap_or(crate::engines::DEFAULT_SECONDS)
    }

    /// How much memory the engine may grow to, in megabytes.
    pub fn megabytes(&self) -> u32 {
        self.megabytes.unwrap_or(crate::engines::DEFAULT_MEGABYTES)
    }
}

/// A function of an engine: values in, a value out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct EngineFunction {
    pub name: Ident,
    pub params: Vec<Param>,
    /// None while it is not written; the checker asks for it.
    pub returns: Option<TypeRef>,
    pub span: Span,
}

/// The effects an action can declare.
pub const EFFECTS: [&str; 5] = ["db.read", "db.write", "files.read", "files.write", "net"];

/// A piece of a text with slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum TextPart {
    /// Text as it is shown, with `{{` already read as a brace.
    Plain { text: String },
    /// `{customer.name}`: a name or a member path.
    Slot { value: Expr },
}

/// The text as written, with its slots: `Saved {customer.name}`. Braces
/// that are text are doubled, so the text reads back the same. It is what
/// a translation is found by, and what the screen fills.
pub fn template(parts: &[TextPart]) -> String {
    parts
        .iter()
        .map(|part| match part {
            TextPart::Plain { text } => text.replace('{', "{{").replace('}', "}}"),
            TextPart::Slot { value } => format!("{{{}}}", path_text(value)),
        })
        .collect()
}

/// A slot's path as written: `customer.name`.
pub fn path_text(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Name { name } => name.clone(),
        ExprKind::Member { object, member } => format!("{}.{}", path_text(object), member.name),
        _ => String::new(),
    }
}

/// Calls `found` for every expression in `body` and inside them, nested
/// blocks too, outer expressions first.
pub fn each_expr_in(body: &[Stmt], found: &mut impl FnMut(&Expr)) {
    for stmt in body {
        match &stmt.kind {
            StmtKind::Let { value, .. } | StmtKind::Save { value } | StmtKind::Delete { value } | StmtKind::CloseWith { value } => each_expr(value, found),
            StmtKind::Assign { target, value } => {
                each_expr(target, found);
                each_expr(value, found);
            }
            StmtKind::If { branches, otherwise } => {
                for branch in branches {
                    each_expr(&branch.condition, found);
                    each_expr_in(&branch.body, found);
                }
                if let Some(body) = otherwise {
                    each_expr_in(body, found);
                }
            }
            StmtKind::Return { value } => {
                if let Some(value) = value {
                    each_expr(value, found);
                }
            }
            StmtKind::For { list, body, .. } => {
                each_expr(list, found);
                each_expr_in(body, found);
            }
            StmtKind::Call { call } => each_expr(call, found),
            StmtKind::Draw { args, .. } | StmtKind::Hear { args, .. } | StmtKind::Tell { args, .. } => args.iter().for_each(|a| each_expr(a, found)),
            StmtKind::Message { text, .. } | StmtKind::Refuse { text } => each_expr(text, found),
            StmtKind::Open { with, .. } => with.iter().for_each(|a| each_expr(&a.value, found)),
            StmtKind::Ask { question, form, args, .. } => {
                if let Some(question) = question {
                    each_expr(question, found);
                }
                for e in ask_exprs(form) {
                    each_expr(e, found);
                }
                args.iter().for_each(|a| each_expr(a, found));
            }
            StmtKind::Write { text, path, .. } => {
                each_expr(text, found);
                each_expr(path, found);
            }
            StmtKind::Print { value, .. } => each_expr(value, found),
            StmtKind::Exit { code } => each_expr(code, found),
            _ => {}
        }
    }
}

/// The expressions of what `ask` asks for: the button texts, or the default and the list.
pub fn ask_exprs(form: &AskForm) -> Vec<&Expr> {
    match form {
        AskForm::Confirm { yes, no } => yes.iter().chain(no).collect(),
        AskForm::Value { default, from, .. } => default.iter().chain(from).collect(),
        AskForm::Window { with, .. } => with.iter().map(|a| &a.value).collect(),
    }
}

/// Calls `found` for `expr` and every expression inside it, outer first.
pub fn each_expr(expr: &Expr, found: &mut impl FnMut(&Expr)) {
    found(expr);
    match &expr.kind {
        ExprKind::Member { object, .. } => each_expr(object, found),
        ExprKind::Unary { operand, .. } => each_expr(operand, found),
        ExprKind::Binary { left, right, .. } => {
            each_expr(left, found);
            each_expr(right, found);
        }
        ExprKind::In { value, options } => {
            each_expr(value, found);
            options.iter().for_each(|o| each_expr(o, found));
        }
        ExprKind::Call { args, named, .. } => args.iter().chain(named.iter().map(|n| &n.value)).for_each(|a| each_expr(a, found)),
        ExprKind::EngineCall { args, .. } => args.iter().for_each(|a| each_expr(a, found)),
        ExprKind::Find { filter, .. } | ExprKind::Files { filter, .. } => {
            if let Some(filter) = filter {
                each_expr(filter, found);
            }
        }
        ExprKind::ReadText { file } | ExprKind::ReadBytes { file } => each_expr(file, found),
        ExprKind::Ask { question, form } => {
            each_expr(question, found);
            for e in ask_exprs(form) {
                each_expr(e, found);
            }
        }
        ExprKind::Key { args, .. } => args.iter().for_each(|a| each_expr(&a.value, found)),
        ExprKind::ListOf { items } => items.iter().for_each(|i| each_expr(i, found)),
        ExprKind::Index { list, index } => {
            each_expr(list, found);
            each_expr(index, found);
        }
        ExprKind::JsonOf { value } => each_expr(value, found),
        ExprKind::ReadJson { text, at, .. } => {
            each_expr(text, found);
            if let Some(at) = at {
                each_expr(at, found);
            }
        }
        ExprKind::Http { url, body, headers, query, .. } => {
            for e in http_exprs(url, body.as_deref(), headers, query) {
                each_expr(e, found);
            }
        }
        ExprKind::Text { parts } => {
            for part in parts {
                if let TextPart::Slot { value } = part {
                    each_expr(value, found);
                }
            }
        }
        ExprKind::Number { .. } | ExprKind::Str { .. } | ExprKind::Bool { .. } | ExprKind::Empty | ExprKind::Name { .. } | ExprKind::New { .. } => {}
    }
}

/// Calls `found` for every expression in `body` and inside them, outer
/// first, and lets it change them: for rewrites before generation, such as
/// filling in a call's defaults (clarity_gen::calls). Every statement is
/// named here, so none is passed over.
pub fn each_expr_in_mut(body: &mut [Stmt], found: &mut impl FnMut(&mut Expr)) {
    for stmt in body {
        match &mut stmt.kind {
            StmtKind::Let { value, .. }
            | StmtKind::Save { value }
            | StmtKind::Delete { value }
            | StmtKind::CloseWith { value }
            | StmtKind::Message { text: value, .. }
            | StmtKind::Refuse { text: value }
            | StmtKind::Print { value, .. }
            | StmtKind::Exit { code: value }
            | StmtKind::Call { call: value } => each_expr_mut(value, found),
            StmtKind::Assign { target, value } => {
                each_expr_mut(target, found);
                each_expr_mut(value, found);
            }
            StmtKind::If { branches, otherwise } => {
                for branch in branches {
                    each_expr_mut(&mut branch.condition, found);
                    each_expr_in_mut(&mut branch.body, found);
                }
                if let Some(body) = otherwise {
                    each_expr_in_mut(body, found);
                }
            }
            StmtKind::Return { value } => {
                if let Some(value) = value {
                    each_expr_mut(value, found);
                }
            }
            StmtKind::For { list, body, .. } => {
                each_expr_mut(list, found);
                each_expr_in_mut(body, found);
            }
            StmtKind::Draw { args, .. } | StmtKind::Hear { args, .. } | StmtKind::Tell { args, .. } => args.iter_mut().for_each(|a| each_expr_mut(a, found)),
            StmtKind::Open { with, .. } => with.iter_mut().for_each(|a| each_expr_mut(&mut a.value, found)),
            StmtKind::Ask { question, form, args, .. } => {
                if let Some(question) = question {
                    each_expr_mut(question, found);
                }
                ask_exprs_mut(form, found);
                args.iter_mut().for_each(|a| each_expr_mut(a, found));
            }
            StmtKind::Write { text, path, .. } => {
                each_expr_mut(text, found);
                each_expr_mut(path, found);
            }
            StmtKind::Close | StmtKind::Reload { .. } => {}
        }
    }
}

fn ask_exprs_mut(form: &mut AskForm, found: &mut impl FnMut(&mut Expr)) {
    match form {
        AskForm::Confirm { yes, no } => yes.iter_mut().chain(no.iter_mut()).for_each(|e| each_expr_mut(e, found)),
        AskForm::Value { default, from, .. } => default.iter_mut().chain(from.iter_mut()).for_each(|e| each_expr_mut(e, found)),
        AskForm::Window { with, .. } => with.iter_mut().for_each(|a| each_expr_mut(&mut a.value, found)),
    }
}

/// `each_expr` that lets `found` change each expression; outer first, so
/// what it puts in is visited too.
pub fn each_expr_mut(expr: &mut Expr, found: &mut impl FnMut(&mut Expr)) {
    found(expr);
    match &mut expr.kind {
        ExprKind::Member { object, .. } => each_expr_mut(object, found),
        ExprKind::Unary { operand, .. } => each_expr_mut(operand, found),
        ExprKind::Binary { left, right, .. } => {
            each_expr_mut(left, found);
            each_expr_mut(right, found);
        }
        ExprKind::In { value, options } => {
            each_expr_mut(value, found);
            options.iter_mut().for_each(|o| each_expr_mut(o, found));
        }
        ExprKind::Call { args, named, .. } => {
            args.iter_mut().for_each(|a| each_expr_mut(a, found));
            named.iter_mut().for_each(|n| each_expr_mut(&mut n.value, found));
        }
        ExprKind::EngineCall { args, .. } | ExprKind::ListOf { items: args } => args.iter_mut().for_each(|a| each_expr_mut(a, found)),
        ExprKind::Find { filter, .. } | ExprKind::Files { filter, .. } => {
            if let Some(filter) = filter {
                each_expr_mut(filter, found);
            }
        }
        ExprKind::ReadText { file } | ExprKind::ReadBytes { file } | ExprKind::JsonOf { value: file } => each_expr_mut(file, found),
        ExprKind::Ask { question, form } => {
            each_expr_mut(question, found);
            ask_exprs_mut(form, found);
        }
        ExprKind::Key { args, .. } => args.iter_mut().for_each(|a| each_expr_mut(&mut a.value, found)),
        ExprKind::Index { list, index } => {
            each_expr_mut(list, found);
            each_expr_mut(index, found);
        }
        ExprKind::ReadJson { text, at, .. } => {
            each_expr_mut(text, found);
            if let Some(at) = at {
                each_expr_mut(at, found);
            }
        }
        ExprKind::Http { url, body, headers, query, .. } => {
            each_expr_mut(url, found);
            if let Some(body) = body {
                each_expr_mut(body, found);
            }
            for pair in headers.iter_mut().chain(query.iter_mut()) {
                each_expr_mut(&mut pair.value, found);
                if let Some(when) = &mut pair.when {
                    each_expr_mut(when, found);
                }
            }
        }
        ExprKind::Text { parts } => {
            for part in parts {
                if let TextPart::Slot { value } = part {
                    each_expr_mut(value, found);
                }
            }
        }
        ExprKind::Number { .. } | ExprKind::Str { .. } | ExprKind::Bool { .. } | ExprKind::Empty | ExprKind::Name { .. } | ExprKind::New { .. } => {}
    }
}
