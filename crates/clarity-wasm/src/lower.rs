//! An engine's checked code as IR (`clarity-engine-ir`): every name resolved
//! to a local, a kept value or a function, every operation saying what it
//! works on, from the checker's type of each expression. The compiler and
//! the interpreter both start from here, so they cannot read the code
//! differently.

use std::collections::HashMap;

use clarity_engine_ir::{self as ir, Arg, Cmp, Condition, IntOp, Kind, NumOp, Place};
use clarity_spec::code::check::{FileTypes, NameKind, TypeInfo, ValueItem};
use clarity_spec::code::{BinaryOp, EngineBody, EngineDecl, Expr, ExprKind, FunctionDecl, Stmt, StmtKind, TextPart, TypeRef, TypeRefKind, UnaryOp};
use clarity_spec::model::{Entity, Field, FieldType, Literal};

/// The services of Clarity an engine may import.
pub const SERVICES: [&str; 4] = ["clock", "random", "log", "language"];

fn kind_of(ty: &TypeInfo) -> Result<Kind, String> {
    match ty {
        TypeInfo::Integer => Ok(Kind::Int),
        TypeInfo::Bool => Ok(Kind::Bool),
        TypeInfo::Text | TypeInfo::Decimal | TypeInfo::Money => Ok(Kind::Value),
        TypeInfo::Values(item) if !matches!(item, ValueItem::Date) => Ok(Kind::Value),
        // Records and lists of them, and enums by their names.
        TypeInfo::Record(_) | TypeInfo::List(_) | TypeInfo::Enum(_) => Ok(Kind::Value),
        other => Err(format!("a value of type {other:?} is not compiled yet")),
    }
}

fn kind_of_ref(ty: &TypeRef) -> Result<Kind, String> {
    match &ty.kind {
        TypeRefKind::Integer => Ok(Kind::Int),
        TypeRefKind::Bool => Ok(Kind::Bool),
        TypeRefKind::Text | TypeRefKind::Decimal | TypeRefKind::Money => Ok(Kind::Value),
        TypeRefKind::List { item } if item != "date" && item != "file" => Ok(Kind::Value),
        TypeRefKind::Named { .. } => Ok(Kind::Value),
        other => Err(format!("a value of type {other:?} is not compiled yet")),
    }
}

/// How an export reads a parameter from JSON.
fn arg_of(ty: &TypeRef) -> Result<Arg, String> {
    Ok(match (&ty.kind, kind_of_ref(ty)?) {
        (_, Kind::Int) => Arg::Int,
        (_, Kind::Bool) => Arg::Bool,
        (TypeRefKind::Decimal | TypeRefKind::Money, _) => Arg::Decimal,
        (TypeRefKind::List { item }, _) if item == "decimal" => Arg::Decimals,
        _ => Arg::Plain,
    })
}

/// How an item of a list is held.
fn item_of(list: &TypeInfo) -> Kind {
    match list {
        TypeInfo::Values(ValueItem::Integer) => Kind::Int,
        TypeInfo::Values(ValueItem::Bool) => Kind::Bool,
        _ => Kind::Value,
    }
}

/// The engine's code as IR.
pub fn engine(types: &FileTypes, records: &[&Entity], engine: &EngineDecl, body: &EngineBody) -> Result<ir::Engine, Vec<String>> {
    let mut errors = Vec::new();
    let services: Vec<String> = engine.uses.iter().filter(|u| SERVICES.contains(&u.name.as_str())).map(|u| u.name.clone()).collect();
    let keeps: Vec<(String, Kind)> = body
        .keeps
        .iter()
        .map(|k| {
            (
                k.name.name.clone(),
                kind_of_ref(&k.ty).unwrap_or_else(|e| {
                    errors.push(format!("keep {}: {e}", k.name.name));
                    Kind::Value
                }),
            )
        })
        .collect();
    let lower = Lower { types, records, keeps: &keeps, functions: &body.functions, services: &services };
    let mut ir_keeps = Vec::new();
    for (keep, (_, kind)) in body.keeps.iter().zip(&keeps) {
        let mut scope = Scope::new(&lower);
        match scope.expr_as(&keep.value, *kind) {
            Ok(start) => ir_keeps.push(ir::Keep { kind: *kind, start }),
            Err(e) => errors.push(format!("keep {}: {e}", keep.name.name)),
        }
    }
    let mut functions = Vec::new();
    for f in &body.functions {
        match lower.function(f) {
            Ok(function) => functions.push(function),
            Err(e) => errors.push(format!("function {}: {e}", f.name.name)),
        }
    }
    if errors.is_empty() {
        Ok(ir::Engine { keeps: ir_keeps, functions, services })
    } else {
        Err(errors.into_iter().map(|e| format!("engine {}: {e}", engine.name.name)).collect())
    }
}

struct Lower<'a> {
    types: &'a FileTypes,
    records: &'a [&'a Entity],
    keeps: &'a [(String, Kind)],
    functions: &'a [FunctionDecl],
    services: &'a [String],
}

impl Lower<'_> {
    fn function(&self, f: &FunctionDecl) -> Result<ir::Function, String> {
        let mut scope = Scope::new(self);
        let mut args = Vec::new();
        for p in &f.params {
            args.push(arg_of(&p.ty)?);
            let kind = kind_of_ref(&p.ty)?;
            let local = scope.local(kind);
            scope.scopes[0].insert(p.name.name.clone(), (local, kind));
        }
        scope.result = f.returns.as_ref().map(kind_of_ref).transpose()?;
        let requires = scope.conditions(&f.requires)?;
        let body = scope.block(&f.body)?;
        // `ensure` reads the result as `result`.
        let result_local = match (f.ensures.is_empty(), scope.result) {
            (true, _) => None,
            (false, Some(kind)) => {
                let local = scope.local(kind);
                scope.scopes[0].insert("result".into(), (local, kind));
                Some(local)
            }
            (false, None) => return Err("ensure needs a result".into()),
        };
        let ensures = scope.conditions(&f.ensures)?;
        Ok(ir::Function { name: f.name.name.clone(), args, locals: scope.locals, result: scope.result, requires, ensures, result_local, body })
    }
}

/// One function's names and locals as it is lowered.
struct Scope<'l, 'a> {
    lower: &'l Lower<'a>,
    locals: Vec<Kind>,
    scopes: Vec<HashMap<String, (u32, Kind)>>,
    result: Option<Kind>,
}

impl<'l, 'a> Scope<'l, 'a> {
    fn new(lower: &'l Lower<'a>) -> Self {
        Scope { lower, locals: Vec::new(), scopes: vec![HashMap::new()], result: None }
    }

    fn local(&mut self, kind: Kind) -> u32 {
        self.locals.push(kind);
        self.locals.len() as u32 - 1
    }

    fn types(&self) -> &FileTypes {
        self.lower.types
    }

    fn conditions(&mut self, conditions: &[Expr]) -> Result<Vec<Condition>, String> {
        conditions
            .iter()
            .map(|c| Ok(Condition { test: self.expr_as(c, Kind::Bool)?, written: format!("condition failed: {}", clarity_spec::code::print::expr(c)) }))
            .collect()
    }

    /// Where a name's value is: a local in scope, else what the engine keeps.
    fn place(&self, name: &str, expr: &Expr) -> Result<(Place, Kind), String> {
        if self.types().name_of(expr) != Some(&NameKind::Kept) {
            for scope in self.scopes.iter().rev() {
                if let Some((local, kind)) = scope.get(name) {
                    return Ok((Place::Local(*local), *kind));
                }
            }
        }
        match self.lower.keeps.iter().position(|(n, _)| n == name) {
            Some(index) => {
                let kind = self.lower.keeps[index].1;
                Ok((Place::Kept(index as u32, kind), kind))
            }
            None => Err(format!("'{name}' is not known here")),
        }
    }

    fn block(&mut self, stmts: &[Stmt]) -> Result<Vec<ir::Stmt>, String> {
        self.scopes.push(HashMap::new());
        let lowered = stmts.iter().map(|s| self.stmt(s)).collect();
        self.scopes.pop();
        lowered
    }

    fn stmt(&mut self, stmt: &Stmt) -> Result<ir::Stmt, String> {
        Ok(match &stmt.kind {
            StmtKind::Let { name, value, .. } => {
                let value = self.expr(value)?;
                let kind = value.kind().ok_or("a call that gives nothing is used as a value")?;
                let local = self.local(kind);
                self.scopes.last_mut().unwrap().insert(name.name.clone(), (local, kind));
                ir::Stmt::Set(Place::Local(local), value)
            }
            StmtKind::Assign { target, value } => self.assign(target, value)?,
            StmtKind::If { branches, otherwise } => {
                let mut lowered = Vec::new();
                for branch in branches {
                    lowered.push((self.expr_as(&branch.condition, Kind::Bool)?, self.block(&branch.body)?));
                }
                let otherwise = otherwise.as_deref().map(|b| self.block(b)).transpose()?;
                ir::Stmt::If(lowered, otherwise)
            }
            StmtKind::Return { value } => match value {
                Some(value) => {
                    let kind = self.result.ok_or("it returns a value but gives none")?;
                    ir::Stmt::Return(Some(self.expr_as(value, kind)?))
                }
                None => ir::Stmt::Return(None),
            },
            StmtKind::For { var, list, body } => {
                let item = item_of(&self.types().type_of(list));
                let list = self.expr(list)?;
                let local = self.local(item);
                self.scopes.push(HashMap::from([(var.name.clone(), (local, item))]));
                let body = self.block(body);
                self.scopes.pop();
                ir::Stmt::For { var: local, item, list, body: body? }
            }
            StmtKind::Call { call } => ir::Stmt::Eval(self.call(call)?),
            StmtKind::Tell { tell, args } => ir::Stmt::Tell(tell.name.clone(), args.iter().map(|a| self.expr(a)).collect::<Result<_, _>>()?),
            StmtKind::Refuse { text } => ir::Stmt::Refuse(self.expr(text)?),
            other => return Err(format!("this statement is not compiled yet: {other:?}")),
        })
    }

    fn assign(&mut self, target: &Expr, value: &Expr) -> Result<ir::Stmt, String> {
        match &target.kind {
            ExprKind::Name { name } => {
                let (place, kind) = self.place(name, target)?;
                Ok(ir::Stmt::Set(place, self.expr_as(value, kind)?))
            }
            // `squares[3] = 1`: the list with that item set, back in its place.
            ExprKind::Index { list, index } => {
                let ExprKind::Name { name } = &list.kind else { return Err("only a named list has items set".into()) };
                let (place, _) = self.place(name, list)?;
                Ok(ir::Stmt::SetItem(place, self.expr_as(index, Kind::Int)?, self.expr(value)?))
            }
            // `d.x = 3`: the record with that field set, back in its place.
            ExprKind::Member { object, member } => {
                let ExprKind::Name { name } = &object.kind else { return Err("only a field of a named record is set".into()) };
                let (place, _) = self.place(name, object)?;
                Ok(ir::Stmt::SetField(place, member.name.clone(), self.expr(value)?))
            }
            _ => Err("only a name, an item of a list or a field of a record is set".into()),
        }
    }

    /// An expression whose value is used as `kind`: any value may be held as a value.
    fn expr_as(&mut self, expr: &Expr, kind: Kind) -> Result<ir::Expr, String> {
        let lowered = self.expr(expr)?;
        match lowered.kind() {
            Some(got) if got == kind || kind == Kind::Value => Ok(lowered),
            got => Err(format!("a {got:?} is used as a {kind:?}")),
        }
    }

    fn boxed(&mut self, expr: &Expr) -> Result<Box<ir::Expr>, String> {
        Ok(Box::new(self.expr_as(expr, Kind::Value)?))
    }

    fn expr(&mut self, expr: &Expr) -> Result<ir::Expr, String> {
        Ok(match &expr.kind {
            ExprKind::Number { value } => match self.types().type_of(expr) {
                TypeInfo::Integer => ir::Expr::Int(value.parse().map_err(|_| format!("{value} is no whole number"))?),
                TypeInfo::Decimal | TypeInfo::Money => ir::Expr::Decimal(value.clone()),
                other => return Err(format!("the number {value} as {other:?} is not compiled yet")),
            },
            // A text by its key gives the key; the host translates it as any engine's `says`.
            ExprKind::Key { key, args } => {
                if !args.is_empty() {
                    return Err(format!("@{} with values is not compiled yet", key.name));
                }
                ir::Expr::Text(key.name.clone())
            }
            ExprKind::Bool { value } => ir::Expr::Bool(*value),
            ExprKind::Str { value } => ir::Expr::Text(value.clone()),
            ExprKind::Text { parts } => {
                let mut joined = ir::Expr::Text(String::new());
                for part in parts {
                    let next = match part {
                        TextPart::Plain { text } => ir::Expr::Text(text.clone()),
                        TextPart::Slot { value } => self.expr_as(value, Kind::Value)?,
                    };
                    joined = ir::Expr::Join(Box::new(joined), Box::new(next));
                }
                joined
            }
            ExprKind::Name { name } => match self.place(name, expr)? {
                (Place::Local(local), kind) => ir::Expr::Local(local, kind),
                (Place::Kept(index, _), kind) => ir::Expr::Kept(index, kind),
            },
            ExprKind::Unary { op: UnaryOp::Not, operand } => ir::Expr::Not(Box::new(self.expr_as(operand, Kind::Bool)?)),
            ExprKind::Unary { op: UnaryOp::Neg, operand } => ir::Expr::Neg(Box::new(self.expr_as(operand, Kind::Int)?)),
            ExprKind::Binary { op, left, right } => self.binary(expr, *op, left, right)?,
            ExprKind::In { value, options } => {
                let value = self.expr(value)?;
                let options = options.iter().map(|o| self.expr(o)).collect::<Result<_, _>>()?;
                ir::Expr::In(Box::new(value), options)
            }
            ExprKind::Call { .. } => self.call(expr)?,
            // `new Dot`: every field at its default.
            ExprKind::New { entity } => {
                let record = self.lower.records.iter().find(|r| r.name.name == entity.name).copied().ok_or_else(|| format!("{} is no record", entity.name))?;
                ir::Expr::Record(record.fields.iter().map(|f| Ok((f.name.name.clone(), default(f)?))).collect::<Result<_, String>>()?)
            }
            // `d.x`: a field of a record.
            ExprKind::Member { object, member } if matches!(self.types().type_of(object), TypeInfo::Record(_)) => {
                let kind = kind_of(&self.types().type_of(expr))?;
                ir::Expr::Field(Box::new(self.expr(object)?), member.name.clone(), kind)
            }
            ExprKind::ListOf { items } => ir::Expr::List(items.iter().map(|i| self.expr_as(i, Kind::Value)).collect::<Result<_, _>>()?),
            ExprKind::Index { list, index } => {
                let item = item_of(&self.types().type_of(list));
                ir::Expr::Item(Box::new(self.expr(list)?), Box::new(self.expr_as(index, Kind::Int)?), item)
            }
            other => return Err(format!("this is not compiled yet: {other:?}")),
        })
    }

    fn binary(&mut self, expr: &Expr, op: BinaryOp, left: &Expr, right: &Expr) -> Result<ir::Expr, String> {
        let result = self.types().type_of(expr);
        let both = |a: ir::Expr, b: ir::Expr| (Box::new(a), Box::new(b));
        Ok(match op {
            BinaryOp::And | BinaryOp::Or => {
                let (l, r) = both(self.expr_as(left, Kind::Bool)?, self.expr_as(right, Kind::Bool)?);
                if op == BinaryOp::And { ir::Expr::And(l, r) } else { ir::Expr::Or(l, r) }
            }
            BinaryOp::Add if result == TypeInfo::Text => ir::Expr::Join(self.boxed(left)?, self.boxed(right)?),
            BinaryOp::Add if matches!(result, TypeInfo::Values(_) | TypeInfo::List(_)) => ir::Expr::Concat(self.boxed(left)?, self.boxed(right)?),
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div if matches!(result, TypeInfo::Decimal | TypeInfo::Money) => {
                let num = match op {
                    BinaryOp::Add => NumOp::Add,
                    BinaryOp::Sub => NumOp::Sub,
                    BinaryOp::Mul => NumOp::Mul,
                    _ => NumOp::Div,
                };
                ir::Expr::Num(num, self.boxed(left)?, self.boxed(right)?)
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul => {
                if result != TypeInfo::Integer {
                    return Err(format!("{} on {result:?} is not compiled yet", op.symbol()));
                }
                let int = match op {
                    BinaryOp::Add => IntOp::Add,
                    BinaryOp::Sub => IntOp::Sub,
                    _ => IntOp::Mul,
                };
                let (l, r) = both(self.expr_as(left, Kind::Int)?, self.expr_as(right, Kind::Int)?);
                ir::Expr::Int2(int, l, r)
            }
            BinaryOp::Eq | BinaryOp::NotEq => {
                let (l, r) = both(self.expr(left)?, self.expr(right)?);
                let eq = ir::Expr::Eq(l, r);
                if op == BinaryOp::NotEq { ir::Expr::Not(Box::new(eq)) } else { eq }
            }
            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                let cmp = match op {
                    BinaryOp::Lt => Cmp::Lt,
                    BinaryOp::LtEq => Cmp::LtEq,
                    BinaryOp::Gt => Cmp::Gt,
                    _ => Cmp::GtEq,
                };
                let (l, r) = (self.expr(left)?, self.expr(right)?);
                if l.kind() == Some(Kind::Int) && r.kind() == Some(Kind::Int) {
                    ir::Expr::IntCmp(cmp, Box::new(l), Box::new(r))
                } else {
                    ir::Expr::Compare(cmp, Box::new(l), Box::new(r))
                }
            }
            BinaryOp::Contains => ir::Expr::Contains(self.boxed(left)?, self.boxed(right)?),
            BinaryOp::Div => return Err(format!("/ giving {result:?} is not compiled yet")),
            BinaryOp::Matches => return Err("matches is for finding records".into()),
        })
    }

    /// A call: a function of Clarity's, or one of the engine's own.
    fn call(&mut self, expr: &Expr) -> Result<ir::Expr, String> {
        let ExprKind::Call { function, args, .. } = &expr.kind else { return Err("only a call stands on its own line".into()) };
        let name = function.name.as_str();
        match (name, args.as_slice()) {
            ("count", [list]) => return Ok(ir::Expr::Count(self.boxed(list)?)),
            ("sin", [degrees]) => return Ok(ir::Expr::Sin(self.boxed(degrees)?)),
            ("cos", [degrees]) => return Ok(ir::Expr::Cos(self.boxed(degrees)?)),
            ("round", [value, places]) => return Ok(ir::Expr::Round(self.boxed(value)?, Box::new(self.expr_as(places, Kind::Int)?))),
            ("random", [max]) if self.lower.services.iter().any(|s| s == "random") => return Ok(ir::Expr::Random(Box::new(self.expr_as(max, Kind::Int)?))),
            _ => {}
        }
        let Some(index) = self.lower.functions.iter().position(|f| f.name.name == name) else {
            return Err(format!("the function '{name}' is not compiled yet: only the engine's own are"));
        };
        let callee = &self.lower.functions[index];
        if args.len() != callee.params.len() {
            return Err(format!("'{name}' takes {} values", callee.params.len()));
        }
        let mut lowered = Vec::new();
        for (arg, param) in args.iter().zip(&callee.params) {
            lowered.push(self.expr_as(arg, kind_of_ref(&param.ty)?)?);
        }
        let result = callee.returns.as_ref().map(kind_of_ref).transpose()?;
        Ok(ir::Expr::Call(index as u32, lowered, result))
    }
}

/// A field's default: written with `default`, or the empty value of its type.
fn default(field: &Field) -> Result<ir::Expr, String> {
    let written = field.modifiers.default.as_ref();
    Ok(match (&field.ty, written) {
        (FieldType::Integer, Some(Literal::Number(n))) => ir::Expr::Int(n.parse().map_err(|_| format!("{n} is no whole number"))?),
        (FieldType::Integer, _) => ir::Expr::Int(0),
        (FieldType::Bool, written) => ir::Expr::Bool(matches!(written, Some(Literal::Bool(true)))),
        (FieldType::Decimal { .. } | FieldType::Money, Some(Literal::Number(n))) => ir::Expr::Decimal(n.clone()),
        (FieldType::Decimal { .. } | FieldType::Money, _) => ir::Expr::Decimal("0".into()),
        (FieldType::Values { .. }, _) => ir::Expr::List(Vec::new()),
        (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Enum { .. }, written) => ir::Expr::Text(match written {
            Some(Literal::Str(t) | Literal::Ident(t)) => t.clone(),
            _ => String::new(),
        }),
        (other, _) => return Err(format!("a field of type {other:?} in a record is not compiled yet")),
    })
}
