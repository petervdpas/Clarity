//! The IR of an engine written in Clarity (DESIGN 5.12): its code lowered
//! once from the checked source, so the compiler (to wasm) and the
//! interpreter (in the support module) run the same thing. Every name is
//! resolved to a local, a kept value or a function by number, and every
//! operation says what it works on, so neither needs the checker's types.
//!
//! How a value is held: a whole number (`Int`), yes or no (`Bool`), or any
//! other value (`Value`: text, decimals, lists, records), which compiled
//! code keeps as a handle and the interpreter as a value.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Int,
    Bool,
    Value,
}

/// How an export reads an argument from the call's JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Arg {
    Int,
    Bool,
    /// A decimal or money, exact text in JSON.
    Decimal,
    /// A list of decimals.
    Decimals,
    /// Text, a list or a record, as JSON has it.
    Plain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Engine {
    /// What it keeps, each with how it starts.
    pub keeps: Vec<Keep>,
    pub functions: Vec<Function>,
    /// The services it imports, in order: `random` among them.
    pub services: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keep {
    pub kind: Kind,
    pub start: Expr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    /// How each argument is read; the parameters are the first locals.
    pub args: Vec<Arg>,
    /// Every local, parameters first.
    pub locals: Vec<Kind>,
    pub result: Option<Kind>,
    pub requires: Vec<Condition>,
    /// Checked on the result, which is local `result_local`.
    pub ensures: Vec<Condition>,
    pub result_local: Option<u32>,
    pub body: Vec<Stmt>,
}

/// A `require` or `ensure`, and how it reads, for the failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    pub test: Expr,
    pub written: String,
}

/// Where a value is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Place {
    Local(u32),
    Kept(u32, Kind),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    Set(Place, Expr),
    /// `xs[i] = value`: the list at the place with item `i` set.
    SetItem(Place, Expr, Expr),
    /// `d.x = value`: the record at the place with the field set.
    SetField(Place, String, Expr),
    /// Branches in order, then what runs when none holds.
    If(Vec<(Expr, Vec<Stmt>)>, Option<Vec<Stmt>>),
    /// `for` over a list: each item into local `var`, of kind `item`.
    For { var: u32, item: Kind, list: Expr, body: Vec<Stmt> },
    Return(Option<Expr>),
    Refuse(Expr),
    Tell(String, Vec<Expr>),
    /// A call for what it does.
    Eval(Expr),
    /// `progress(done, total)`: how far a long call is, told to the host.
    Progress(Expr, Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntOp {
    Add,
    Sub,
    Mul,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NumOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cmp {
    Lt,
    LtEq,
    Gt,
    GtEq,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Int(i64),
    Bool(bool),
    Text(String),
    /// A decimal as written.
    Decimal(String),
    Local(u32, Kind),
    Kept(u32, Kind),
    /// Whole numbers.
    Int2(IntOp, Box<Expr>, Box<Expr>),
    IntCmp(Cmp, Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    /// Decimals, whole numbers among them.
    Num(NumOp, Box<Expr>, Box<Expr>),
    /// Text or numbers compared.
    Compare(Cmp, Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    In(Box<Expr>, Vec<Expr>),
    /// Two values as one text.
    Join(Box<Expr>, Box<Expr>),
    /// Two lists as one.
    Concat(Box<Expr>, Box<Expr>),
    Contains(Box<Expr>, Box<Expr>),
    List(Vec<Expr>),
    Item(Box<Expr>, Box<Expr>, Kind),
    Count(Box<Expr>),
    /// A new record: its fields in order, each at its default.
    Record(Vec<(String, Expr)>),
    Field(Box<Expr>, String, Kind),
    /// One of the engine's functions by number.
    Call(u32, Vec<Expr>, Option<Kind>),
    Round(Box<Expr>, Box<Expr>),
    Sin(Box<Expr>),
    Cos(Box<Expr>),
    Random(Box<Expr>),
}

impl Expr {
    /// How the value of this expression is held; none for a call that gives nothing.
    pub fn kind(&self) -> Option<Kind> {
        Some(match self {
            Expr::Int(_) | Expr::Int2(..) | Expr::Neg(_) | Expr::Count(_) | Expr::Random(_) => Kind::Int,
            Expr::Bool(_)
            | Expr::IntCmp(..)
            | Expr::Compare(..)
            | Expr::Eq(..)
            | Expr::And(..)
            | Expr::Or(..)
            | Expr::Not(_)
            | Expr::In(..)
            | Expr::Contains(..) => Kind::Bool,
            Expr::Local(_, kind) | Expr::Kept(_, kind) | Expr::Item(_, _, kind) | Expr::Field(_, _, kind) => *kind,
            Expr::Call(_, _, result) => return *result,
            _ => Kind::Value,
        })
    }
}
