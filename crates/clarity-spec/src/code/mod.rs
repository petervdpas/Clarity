//! Code behind: the handlers of a window or the app, written in Clarity
//! (docs/DESIGN.md section 5.9). This module holds the syntax tree; `parse`
//! reads `code` blocks and `check` resolves names and types against the
//! window they belong to.

pub(crate) mod check;
pub(crate) mod parse;
pub mod edit;

use serde::Serialize;
use ts_rs::TS;

use crate::model::Ident;
use crate::span::Span;

/// `code Name` with the handlers of the window or app called Name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CodeBlock {
    pub name: Ident,
    pub handlers: Vec<HandlerDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct HandlerDecl {
    pub name: Ident,
    pub body: Vec<Stmt>,
    pub span: Span,
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
    Delete {
        value: Expr,
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
}

impl FunctionKind {
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Action => "action",
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
