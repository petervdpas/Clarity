//! Code behind: the handlers of a window or the app, written in Clarity
//! (docs/DESIGN.md section 5.9). This module holds the syntax tree; `parse`
//! reads `code` blocks and `check` resolves names and types against the
//! window they belong to.

pub mod check;
pub(crate) mod parse;
pub mod edit;

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
    Message {
        level: MessageLevel,
        text: Expr,
    },
    Open {
        window: Ident,
        with: Vec<Arg>,
    },
    Close,
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
    Call {
        function: Ident,
        args: Vec<Expr>,
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
}

/// `c.name` or `c.name desc` in `order by`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct OrderKey {
    /// The record name before the dot, as written.
    pub var: Ident,
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
    /// `text contains part`, ignoring case.
    Contains,
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

/// The effects an action can declare.
pub const EFFECTS: [&str; 2] = ["db.read", "db.write"];

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
