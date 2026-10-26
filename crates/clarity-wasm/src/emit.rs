//! The engine's functions as wasm code.
//!
//! A whole number is an i64 and yes or no an i32; every other value is a
//! handle (an i32) to a value in the support module. A call stops at the
//! first thing that fails or refuses: after each statement, and before a
//! condition or a list is used, the code asks `rt_failed` and returns.

use std::collections::HashMap;

use clarity_spec::code::check::{FileTypes, NameKind, TypeInfo, ValueItem};
use clarity_spec::model::{Entity, FieldType, Literal};
use clarity_spec::code::{BinaryOp, EngineBody, EngineDecl, Expr, ExprKind, FunctionDecl, Stmt, StmtKind, TextPart, TypeRef, TypeRefKind, UnaryOp};
use wasm_encoder::{BlockType, Function, Instruction as I, ValType};

use crate::merge::{Code, Support};

/// How a value is held in wasm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Int,
    Bool,
    Handle,
}

impl Kind {
    fn val(self) -> ValType {
        match self {
            Kind::Int => ValType::I64,
            Kind::Bool | Kind::Handle => ValType::I32,
        }
    }

    fn of(ty: &TypeInfo) -> Result<Kind, String> {
        match ty {
            TypeInfo::Integer => Ok(Kind::Int),
            TypeInfo::Bool => Ok(Kind::Bool),
            TypeInfo::Text | TypeInfo::Decimal | TypeInfo::Money => Ok(Kind::Handle),
            TypeInfo::Values(item) if !matches!(item, ValueItem::Date) => Ok(Kind::Handle),
            // Records and lists of them, and enums by their names.
            TypeInfo::Record(_) | TypeInfo::List(_) | TypeInfo::Enum(_) => Ok(Kind::Handle),
            other => Err(format!("a value of type {other:?} is not compiled yet")),
        }
    }

    fn of_ref(ty: &TypeRef) -> Result<Kind, String> {
        match &ty.kind {
            TypeRefKind::Integer => Ok(Kind::Int),
            TypeRefKind::Bool => Ok(Kind::Bool),
            TypeRefKind::Text | TypeRefKind::Decimal | TypeRefKind::Money => Ok(Kind::Handle),
            TypeRefKind::List { item } if item != "date" && item != "file" => Ok(Kind::Handle),
            TypeRefKind::Named { .. } => Ok(Kind::Handle),
            other => Err(format!("a value of type {other:?} is not compiled yet")),
        }
    }

    /// Of an item of a list of values.
    fn item(list: &TypeInfo) -> Kind {
        match list {
            TypeInfo::Values(ValueItem::Integer) => Kind::Int,
            TypeInfo::Values(ValueItem::Bool) => Kind::Bool,
            _ => Kind::Handle,
        }
    }
}

/// The services of Clarity an engine may import, with their types.
pub const SERVICES: [(&str, &[ValType], &[ValType]); 4] = [
    ("clock", &[], &[ValType::I64]),
    ("random", &[], &[ValType::I64]),
    ("log", &[ValType::I32, ValType::I32], &[]),
    ("language", &[ValType::I32, ValType::I32], &[ValType::I32]),
];

/// The services an engine imports, in the order it lists them.
pub fn services(engine: &EngineDecl) -> Vec<&'static str> {
    engine.uses.iter().filter_map(|u| SERVICES.iter().find(|(name, ..)| *name == u.name).map(|(name, ..)| *name)).collect()
}

/// The engine's functions: an inner one and an export per function, and
/// one that makes what it keeps on its first call.
pub fn engine(support: &Support, types: &FileTypes, records: &[&Entity], engine: &EngineDecl, body: &EngineBody) -> Result<Code, Vec<String>> {
    let mut code = Code::default();
    let mut errors = Vec::new();
    let imported = services(engine);
    for name in &imported {
        let (_, params, results) = SERVICES.iter().find(|(n, ..)| n == name).unwrap();
        let ty = code.ty(params.to_vec(), results.to_vec());
        code.imports.push((name.to_string(), ty));
    }
    let keeps: Vec<(String, Kind)> = body
        .keeps
        .iter()
        .map(|k| (k.name.name.clone(), Kind::of_ref(&k.ty).unwrap_or_else(|e| {
            errors.push(format!("keep {}: {e}", k.name.name));
            Kind::Handle
        })))
        .collect();
    let inner: Vec<(String, u32)> = body.functions.iter().enumerate().map(|(i, f)| (f.name.name.clone(), i as u32)).collect();
    // The engine's functions in order: the inner ones, then the exports, then the keeps' start.
    let count = body.functions.len() as u32;
    let start = 2 * count;
    let mut world = World { support, types, keeps: &keeps, inner: &inner, functions: &body.functions, texts: Vec::new(), base: support.texts_at(), imported: &imported, records, extra: Vec::new(), extra_base: start + 1 };
    let mut inners = Vec::new();
    for f in &body.functions {
        match world.function(f) {
            Ok(made) => inners.push(made),
            Err(e) => errors.push(format!("function {}: {e}", f.name.name)),
        }
    }
    let keep_start = world.keeps_start(body);
    if let Err(e) = &keep_start {
        errors.push(e.clone());
    }
    if !errors.is_empty() {
        return Err(errors.into_iter().map(|e| format!("engine {}: {e}", engine.name.name)).collect());
    }
    for (params, result, function) in inners {
        let ty = code.ty(params, result);
        code.functions.push((ty, function));
    }
    for (i, f) in body.functions.iter().enumerate() {
        let made = world.export(f, i as u32, if keeps.is_empty() { None } else { Some(start) });
        let ty = code.ty(vec![ValType::I32, ValType::I32], vec![ValType::I64]);
        code.functions.push((ty, made));
        code.exports.push((f.name.name.clone(), count + i as u32));
    }
    let ty = code.ty(vec![], vec![]);
    code.functions.push((ty, keep_start.unwrap()));
    for (params, results, function) in std::mem::take(&mut world.extra) {
        let ty = code.ty(params, results);
        code.functions.push((ty, function));
    }
    code.texts = world.texts;
    Ok(code)
}

struct World<'a> {
    support: &'a Support,
    types: &'a FileTypes,
    keeps: &'a [(String, Kind)],
    inner: &'a [(String, u32)],
    functions: &'a [FunctionDecl],
    texts: Vec<u8>,
    base: u32,
    /// The services it imports: each is the function of its place here.
    imported: &'a [&'static str],
    /// The project's records, for `new`.
    records: &'a [&'a Entity],
    /// Functions written besides one per engine function: the statements of one with ensures.
    extra: Vec<(Vec<ValType>, Vec<ValType>, Function)>,
    /// The number among the engine's functions of the first of `extra`.
    extra_base: u32,
}

impl World<'_> {
    fn rt(&self, name: &str) -> I<'static> {
        I::Call(self.support.rt(name))
    }

    /// Where a text written in the code is, added the first time.
    fn text(&mut self, text: &str) -> (i32, i32) {
        let bytes = text.as_bytes();
        let at = match self.texts.windows(bytes.len().max(1)).position(|w| w == bytes) {
            Some(at) if !bytes.is_empty() => at,
            _ => {
                self.texts.extend_from_slice(bytes);
                self.texts.len() - bytes.len()
            }
        };
        ((self.base as usize + at) as i32, bytes.len() as i32)
    }

    /// A function of the engine. With `ensure`, its statements are a
    /// function of their own, so every `return` gives the result the
    /// ensures then check.
    fn function(&mut self, f: &FunctionDecl) -> Result<(Vec<ValType>, Vec<ValType>, Function), String> {
        let result = f.returns.as_ref().map(Kind::of_ref).transpose()?;
        let statements = if f.ensures.is_empty() {
            None
        } else {
            let (params, results, function) = self.statements(f, result, false)?;
            self.extra.push((params, results, function));
            Some(self.extra_base + self.extra.len() as u32 - 1)
        };
        match statements {
            None => self.statements(f, result, true),
            Some(index) => self.ensuring(f, result, index),
        }
    }

    /// The parameters as locals, and the call stopped if one did not fit.
    fn params(body: &mut Body, f: &FunctionDecl) -> Result<Vec<ValType>, String> {
        let mut params = Vec::new();
        for p in &f.params {
            let kind = Kind::of_ref(&p.ty)?;
            params.push(kind.val());
            let local = body.params;
            body.params += 1;
            body.scopes[0].insert(p.name.name.clone(), (local, kind));
        }
        body.check_failed();
        Ok(params)
    }

    /// The function's statements, after its requires when `requires`.
    fn statements(&mut self, f: &FunctionDecl, result: Option<Kind>, requires: bool) -> Result<(Vec<ValType>, Vec<ValType>, Function), String> {
        let mut body = Body::new(self, result);
        let params = Self::params(&mut body, f)?;
        if requires {
            body.conditions(&f.requires)?;
        }
        body.block(&f.body)?;
        // Falling off the end gives nothing, or a whole number 0 and the like.
        body.dummy();
        body.code.push(I::End);
        let results: Vec<ValType> = body.result.map(|k| vec![k.val()]).unwrap_or_default();
        Ok((params, results, body.finish()))
    }

    /// Its requires, its statements (function `index`), then its ensures on `result`.
    fn ensuring(&mut self, f: &FunctionDecl, result: Option<Kind>, index: u32) -> Result<(Vec<ValType>, Vec<ValType>, Function), String> {
        let kind = result.ok_or("ensure needs a result")?;
        let mut body = Body::new(self, result);
        let params = Self::params(&mut body, f)?;
        body.conditions(&f.requires)?;
        for local in 0..params.len() as u32 {
            body.code.push(I::LocalGet(local));
        }
        let call = I::Call(body.world.support.function(index));
        body.code.push(call);
        let held = body.local(kind);
        body.code.push(I::LocalSet(held));
        body.check_failed();
        body.scopes[0].insert("result".into(), (held, kind));
        body.conditions(&f.ensures)?;
        body.code.extend([I::LocalGet(held), I::End]);
        Ok((params, vec![kind.val()], body.finish()))
    }

    /// The export the host calls: the arguments from JSON, the inner function, the answer.
    fn export(&mut self, f: &FunctionDecl, index: u32, keeps_start: Option<u32>) -> Function {
        let mut code = vec![I::LocalGet(0), I::LocalGet(1), self.rt("rt_begin")];
        if let Some(start) = keeps_start {
            code.extend([self.rt("rt_unkept"), I::If(BlockType::Empty), I::Call(self.support.function(start)), I::End]);
        }
        for (i, p) in f.params.iter().enumerate() {
            code.push(I::I32Const(i as i32));
            code.push(match (&p.ty.kind, Kind::of_ref(&p.ty)) {
                (_, Ok(Kind::Int)) => self.rt("rt_arg_int"),
                (_, Ok(Kind::Bool)) => self.rt("rt_arg_bool"),
                (TypeRefKind::Decimal | TypeRefKind::Money, _) => self.rt("rt_arg_dec"),
                (TypeRefKind::List { item }, _) if item == "decimal" => self.rt("rt_arg_decs"),
                _ => self.rt("rt_arg"),
            });
        }
        code.push(I::Call(self.support.function(index)));
        match f.returns.as_ref().map(Kind::of_ref) {
            Some(Ok(Kind::Int)) => code.push(self.rt("rt_answer_int")),
            Some(Ok(Kind::Bool)) => code.push(self.rt("rt_answer_bool")),
            Some(_) => code.push(self.rt("rt_answer")),
            None => {}
        }
        code.extend([self.rt("rt_finish"), I::End]);
        let mut function = Function::new([]);
        for i in &code {
            function.instruction(i);
        }
        function
    }

    /// Makes what the engine keeps, as written, on its first call.
    fn keeps_start(&mut self, engine: &EngineBody) -> Result<Function, String> {
        let mut body = Body::new(self, None);
        for (index, keep) in engine.keeps.iter().enumerate() {
            let kind = body.world.keeps[index].1;
            body.code.push(I::I32Const(index as i32));
            body.expr_as(&keep.value, kind).map_err(|e| format!("keep {}: {e}", keep.name.name))?;
            let set = body.keep_setter(kind);
            body.code.push(set);
        }
        body.code.push(I::End);
        Ok(body.finish())
    }
}

/// One function's code as it is written.
struct Body<'w, 'a> {
    world: &'w mut World<'a>,
    params: u32,
    locals: Vec<ValType>,
    scopes: Vec<HashMap<String, (u32, Kind)>>,
    code: Vec<I<'static>>,
    result: Option<Kind>,
}

/// Where a name's value is.
enum Place {
    Local(u32, Kind),
    Kept(u32, Kind),
}

impl<'w, 'a> Body<'w, 'a> {
    fn new(world: &'w mut World<'a>, result: Option<Kind>) -> Self {
        Body { world, params: 0, locals: Vec::new(), scopes: vec![HashMap::new()], code: Vec::new(), result }
    }

    fn finish(self) -> Function {
        let mut function = Function::new(self.locals.iter().map(|v| (1, *v)));
        for i in &self.code {
            function.instruction(i);
        }
        function
    }

    fn local(&mut self, kind: Kind) -> u32 {
        self.locals.push(kind.val());
        self.params + self.locals.len() as u32 - 1
    }

    fn rt(&mut self, name: &str) {
        let call = self.world.rt(name);
        self.code.push(call);
    }

    /// Each condition that does not hold fails the call with it as written.
    fn conditions(&mut self, conditions: &[Expr]) -> Result<(), String> {
        for condition in conditions {
            self.expr_as(condition, Kind::Bool)?;
            self.code.extend([I::I32Eqz, I::If(BlockType::Empty)]);
            self.text_const(&format!("condition failed: {}", clarity_spec::code::print::expr(condition)));
            self.rt("rt_refuse");
            self.dummy();
            self.code.extend([I::Return, I::End]);
        }
        Ok(())
    }

    /// The function's result when it stops early: nothing, 0 or no.
    fn dummy(&mut self) {
        match self.result {
            Some(Kind::Int) => self.code.push(I::I64Const(0)),
            Some(Kind::Bool | Kind::Handle) => self.code.push(I::I32Const(0)),
            None => {}
        }
    }

    /// Returns when the call has failed or been refused.
    fn check_failed(&mut self) {
        self.rt("rt_failed");
        self.code.push(I::If(BlockType::Empty));
        self.dummy();
        self.code.push(I::Return);
        self.code.push(I::End);
    }

    fn keep_setter(&self, kind: Kind) -> I<'static> {
        self.world.rt(match kind {
            Kind::Int => "rt_keep_int",
            Kind::Bool => "rt_keep_bool",
            Kind::Handle => "rt_keep",
        })
    }

    fn place(&self, name: &str, expr: Option<&Expr>) -> Result<Place, String> {
        let kept = expr.map(|e| self.world.types.name_of(e) == Some(&NameKind::Kept)).unwrap_or(false);
        if !kept {
            for scope in self.scopes.iter().rev() {
                if let Some((local, kind)) = scope.get(name) {
                    return Ok(Place::Local(*local, *kind));
                }
            }
        }
        match self.world.keeps.iter().position(|(n, _)| n == name) {
            Some(index) => Ok(Place::Kept(index as u32, self.world.keeps[index].1)),
            None => Err(format!("'{name}' is not known here")),
        }
    }

    fn block(&mut self, stmts: &[Stmt]) -> Result<(), String> {
        self.scopes.push(HashMap::new());
        for stmt in stmts {
            self.stmt(stmt)?;
            self.check_failed();
        }
        self.scopes.pop();
        Ok(())
    }

    fn stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match &stmt.kind {
            StmtKind::Let { name, value, .. } => {
                let kind = self.expr(value)?;
                let local = self.local(kind);
                self.code.push(I::LocalSet(local));
                self.scopes.last_mut().unwrap().insert(name.name.clone(), (local, kind));
            }
            StmtKind::Assign { target, value } => self.assign(target, value)?,
            StmtKind::If { branches, otherwise } => self.branches(branches, otherwise.as_deref())?,
            StmtKind::Return { value } => {
                if let Some(value) = value {
                    let kind = self.result.ok_or("it returns a value but gives none")?;
                    self.expr_as(value, kind)?;
                    self.check_failed();
                } else {
                    self.dummy();
                }
                self.code.push(I::Return);
            }
            StmtKind::For { var, list, body } => {
                let item = Kind::item(&self.world.types.type_of(list));
                self.expr(list)?;
                let items = self.local(Kind::Handle);
                self.code.push(I::LocalSet(items));
                self.check_failed();
                let count = self.local(Kind::Int);
                self.code.extend([I::LocalGet(items)]);
                self.rt("rt_count");
                self.code.push(I::LocalSet(count));
                let at = self.local(Kind::Int);
                let each = self.local(item);
                self.code.extend([I::I64Const(0), I::LocalSet(at), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
                self.code.extend([I::LocalGet(at), I::I64Const(1), I::I64Add, I::LocalTee(at), I::LocalGet(count), I::I64GtS, I::BrIf(1)]);
                self.code.extend([I::LocalGet(items), I::LocalGet(at)]);
                self.rt(match item {
                    Kind::Int => "rt_item_int",
                    Kind::Bool => "rt_item_bool",
                    Kind::Handle => "rt_item",
                });
                self.code.push(I::LocalSet(each));
                self.scopes.push(HashMap::from([(var.name.clone(), (each, item))]));
                self.block(body)?;
                self.scopes.pop();
                self.code.extend([I::Br(0), I::End, I::End]);
            }
            StmtKind::Call { call } => {
                if self.call_expr(call)?.is_some() {
                    self.code.push(I::Drop);
                }
            }
            // `tell finished(time)`: its values as a list, in the call's answer.
            StmtKind::Tell { tell, args } => {
                let (at, len) = self.world.text(&tell.name);
                self.code.extend([I::I32Const(at), I::I32Const(len)]);
                self.rt("rt_list");
                for arg in args {
                    self.boxed(arg)?;
                    self.rt("rt_push");
                }
                self.rt("rt_tell");
            }
            StmtKind::Refuse { text } => {
                self.boxed(text)?;
                self.rt("rt_refuse");
                self.dummy();
                self.code.push(I::Return);
            }
            other => return Err(format!("this statement is not compiled yet: {other:?}")),
        }
        Ok(())
    }

    fn assign(&mut self, target: &Expr, value: &Expr) -> Result<(), String> {
        match &target.kind {
            ExprKind::Name { name } => match self.place(name, Some(target))? {
                Place::Local(local, kind) => {
                    self.expr_as(value, kind)?;
                    self.code.push(I::LocalSet(local));
                }
                Place::Kept(index, kind) => {
                    self.code.push(I::I32Const(index as i32));
                    self.expr_as(value, kind)?;
                    let set = self.keep_setter(kind);
                    self.code.push(set);
                }
            },
            // `squares[3] = 1`: the list with that item set, back in its place.
            ExprKind::Index { list, index } => {
                let ExprKind::Name { name } = &list.kind else { return Err("only a named list has items set".into()) };
                let place = self.place(name, Some(list))?;
                match place {
                    Place::Local(local, _) => self.code.push(I::LocalGet(local)),
                    Place::Kept(index, _) => {
                        self.code.push(I::I32Const(index as i32));
                        self.rt("rt_kept");
                    }
                }
                self.expr_as(index, Kind::Int)?;
                self.boxed(value)?;
                self.rt("rt_set_item");
                match place {
                    Place::Local(local, _) => self.code.push(I::LocalSet(local)),
                    Place::Kept(index, _) => {
                        let changed = self.local(Kind::Handle);
                        self.code.extend([I::LocalSet(changed), I::I32Const(index as i32), I::LocalGet(changed)]);
                        self.rt("rt_keep");
                    }
                }
            }
            // `d.x = 3`: the record with that field set, back in its place.
            ExprKind::Member { object, member } => {
                let ExprKind::Name { name } = &object.kind else { return Err("only a field of a named record is set".into()) };
                let place = self.place(name, Some(object))?;
                match place {
                    Place::Local(local, _) => self.code.push(I::LocalGet(local)),
                    Place::Kept(index, _) => {
                        self.code.push(I::I32Const(index as i32));
                        self.rt("rt_kept");
                    }
                }
                let (at, len) = self.world.text(&member.name);
                self.code.extend([I::I32Const(at), I::I32Const(len)]);
                self.boxed(value)?;
                self.rt("rt_with");
                match place {
                    Place::Local(local, _) => self.code.push(I::LocalSet(local)),
                    Place::Kept(index, _) => {
                        let changed = self.local(Kind::Handle);
                        self.code.extend([I::LocalSet(changed), I::I32Const(index as i32), I::LocalGet(changed)]);
                        self.rt("rt_keep");
                    }
                }
            }
            _ => return Err("only a name, an item of a list or a field of a record is set".into()),
        }
        Ok(())
    }

    fn branches(&mut self, branches: &[clarity_spec::code::Branch], otherwise: Option<&[Stmt]>) -> Result<(), String> {
        let Some((first, rest)) = branches.split_first() else {
            if let Some(body) = otherwise {
                self.block(body)?;
            }
            return Ok(());
        };
        self.expr_as(&first.condition, Kind::Bool)?;
        let condition = self.local(Kind::Bool);
        self.code.push(I::LocalSet(condition));
        self.check_failed();
        self.code.extend([I::LocalGet(condition), I::If(BlockType::Empty)]);
        self.block(&first.body)?;
        if !rest.is_empty() || otherwise.is_some() {
            self.code.push(I::Else);
            self.branches(rest, otherwise)?;
        }
        self.code.push(I::End);
        Ok(())
    }

    /// An expression whose value is used as `kind`.
    fn expr_as(&mut self, expr: &Expr, kind: Kind) -> Result<(), String> {
        if kind == Kind::Handle {
            return self.boxed(expr);
        }
        let got = self.expr(expr)?;
        if got != kind {
            return Err(format!("a {got:?} is used as a {kind:?}"));
        }
        Ok(())
    }

    /// An expression as a handle, whatever it holds.
    fn boxed(&mut self, expr: &Expr) -> Result<(), String> {
        match self.expr(expr)? {
            Kind::Int => self.rt("rt_int"),
            Kind::Bool => self.rt("rt_bool"),
            Kind::Handle => {}
        }
        Ok(())
    }

    fn text_const(&mut self, text: &str) {
        let (at, len) = self.world.text(text);
        self.code.extend([I::I32Const(at), I::I32Const(len)]);
        self.rt("rt_text");
    }

    fn expr(&mut self, expr: &Expr) -> Result<Kind, String> {
        match &expr.kind {
            ExprKind::Number { value } => match self.world.types.type_of(expr) {
                TypeInfo::Integer => {
                    let n: i64 = value.parse().map_err(|_| format!("{value} is no whole number"))?;
                    self.code.push(I::I64Const(n));
                    Ok(Kind::Int)
                }
                TypeInfo::Decimal | TypeInfo::Money => {
                    let (at, len) = self.world.text(value);
                    self.code.extend([I::I32Const(at), I::I32Const(len)]);
                    self.rt("rt_dec");
                    Ok(Kind::Handle)
                }
                other => Err(format!("the number {value} as {other:?} is not compiled yet")),
            },
            // A text by its key gives the key; the host translates it as any engine's `says`.
            ExprKind::Key { key, args } => {
                if !args.is_empty() {
                    return Err(format!("@{} with values is not compiled yet", key.name));
                }
                self.text_const(&key.name);
                Ok(Kind::Handle)
            }
            ExprKind::Bool { value } => {
                self.code.push(I::I32Const(*value as i32));
                Ok(Kind::Bool)
            }
            ExprKind::Str { value } => {
                self.text_const(value);
                Ok(Kind::Handle)
            }
            ExprKind::Text { parts } => {
                self.text_const("");
                for part in parts {
                    match part {
                        TextPart::Plain { text } => self.text_const(text),
                        TextPart::Slot { value } => self.boxed(value)?,
                    }
                    self.rt("rt_join");
                }
                Ok(Kind::Handle)
            }
            ExprKind::Name { name } => Ok(match self.place(name, Some(expr))? {
                Place::Local(local, kind) => {
                    self.code.push(I::LocalGet(local));
                    kind
                }
                Place::Kept(index, kind) => {
                    self.code.push(I::I32Const(index as i32));
                    self.rt(match kind {
                        Kind::Int => "rt_kept_int",
                        Kind::Bool => "rt_kept_bool",
                        Kind::Handle => "rt_kept",
                    });
                    kind
                }
            }),
            ExprKind::Unary { op: UnaryOp::Not, operand } => {
                self.expr_as(operand, Kind::Bool)?;
                self.code.push(I::I32Eqz);
                Ok(Kind::Bool)
            }
            ExprKind::Unary { op: UnaryOp::Neg, operand } => {
                self.code.push(I::I64Const(0));
                self.expr_as(operand, Kind::Int)?;
                self.code.push(I::I64Sub);
                Ok(Kind::Int)
            }
            ExprKind::Binary { op, left, right } => self.binary(expr, *op, left, right),
            ExprKind::In { value, options } => {
                let kind = self.expr(value)?;
                let held = self.local(kind);
                self.code.push(I::LocalSet(held));
                self.code.push(I::I32Const(0));
                for option in options {
                    self.code.push(I::LocalGet(held));
                    if kind == Kind::Handle {
                        self.boxed(option)?;
                    } else {
                        self.expr_as(option, kind)?;
                    }
                    self.equal(kind);
                    self.code.push(I::I32Or);
                }
                Ok(Kind::Bool)
            }
            ExprKind::Call { .. } => self.call_expr(expr)?.ok_or_else(|| "a call that gives nothing is used as a value".to_string()),
            // `new Dot`: every field at its default.
            ExprKind::New { entity } => {
                let record = self.world.records.iter().find(|r| r.name.name == entity.name).copied().ok_or_else(|| format!("{} is no record", entity.name))?;
                self.rt("rt_record");
                for field in &record.fields {
                    let (at, len) = self.world.text(&field.name.name);
                    self.code.extend([I::I32Const(at), I::I32Const(len)]);
                    self.default(field)?;
                    self.rt("rt_with");
                }
                Ok(Kind::Handle)
            }
            // `d.x`: a field of a record.
            ExprKind::Member { object, member } if matches!(self.world.types.type_of(object), TypeInfo::Record(_)) => {
                let kind = Kind::of(&self.world.types.type_of(expr))?;
                self.expr(object)?;
                let (at, len) = self.world.text(&member.name);
                self.code.extend([I::I32Const(at), I::I32Const(len)]);
                self.rt(match kind {
                    Kind::Int => "rt_field_int",
                    Kind::Bool => "rt_field_bool",
                    Kind::Handle => "rt_field",
                });
                Ok(kind)
            }
            ExprKind::ListOf { items } => {
                self.rt("rt_list");
                for item in items {
                    self.boxed(item)?;
                    self.rt("rt_push");
                }
                Ok(Kind::Handle)
            }
            ExprKind::Index { list, index } => {
                let item = Kind::item(&self.world.types.type_of(list));
                self.expr(list)?;
                self.expr_as(index, Kind::Int)?;
                self.rt(match item {
                    Kind::Int => "rt_item_int",
                    Kind::Bool => "rt_item_bool",
                    Kind::Handle => "rt_item",
                });
                Ok(item)
            }
            other => Err(format!("this is not compiled yet: {other:?}")),
        }
    }

    /// A field's default, as a handle: written with `default`, or the empty value of its type.
    fn default(&mut self, field: &clarity_spec::model::Field) -> Result<(), String> {
        let written = field.modifiers.default.as_ref();
        match (&field.ty, written) {
            (FieldType::Integer, Some(Literal::Number(n))) => {
                self.code.push(I::I64Const(n.parse().map_err(|_| format!("{n} is no whole number"))?));
                self.rt("rt_int");
            }
            (FieldType::Integer, _) => {
                self.code.push(I::I64Const(0));
                self.rt("rt_int");
            }
            (FieldType::Bool, written) => {
                self.code.push(I::I32Const(matches!(written, Some(Literal::Bool(true))) as i32));
                self.rt("rt_bool");
            }
            (FieldType::Decimal { .. } | FieldType::Money, written) => {
                let number = match written {
                    Some(Literal::Number(n)) => n.clone(),
                    _ => "0".to_string(),
                };
                let (at, len) = self.world.text(&number);
                self.code.extend([I::I32Const(at), I::I32Const(len)]);
                self.rt("rt_dec");
            }
            (FieldType::Values { .. }, _) => self.rt("rt_list"),
            (FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Enum { .. }, written) => {
                let text = match written {
                    Some(Literal::Str(t) | Literal::Ident(t)) => t.clone(),
                    _ => String::new(),
                };
                self.text_const(&text);
            }
            (other, _) => return Err(format!("a field of type {other:?} in a record is not compiled yet")),
        }
        Ok(())
    }

    /// Whether the two values on the stack are equal.
    fn equal(&mut self, kind: Kind) {
        match kind {
            Kind::Int => self.code.push(I::I64Eq),
            Kind::Bool => self.code.push(I::I32Eq),
            Kind::Handle => self.rt("rt_eq"),
        }
    }

    fn binary(&mut self, expr: &Expr, op: BinaryOp, left: &Expr, right: &Expr) -> Result<Kind, String> {
        let result = self.world.types.type_of(expr);
        let lt = Kind::of(&self.world.types.type_of(left))?;
        let rt = Kind::of(&self.world.types.type_of(right))?;
        // Compared natively only when both sides are held alike in wasm.
        let native = if lt == rt { lt } else { Kind::Handle };
        match op {
            BinaryOp::And | BinaryOp::Or => {
                self.expr_as(left, Kind::Bool)?;
                self.code.push(I::If(BlockType::Result(ValType::I32)));
                if op == BinaryOp::And {
                    self.expr_as(right, Kind::Bool)?;
                    self.code.extend([I::Else, I::I32Const(0)]);
                } else {
                    self.code.extend([I::I32Const(1), I::Else]);
                    self.expr_as(right, Kind::Bool)?;
                }
                self.code.push(I::End);
                Ok(Kind::Bool)
            }
            BinaryOp::Add if result == TypeInfo::Text => {
                self.boxed(left)?;
                self.boxed(right)?;
                self.rt("rt_join");
                Ok(Kind::Handle)
            }
            BinaryOp::Add if matches!(result, TypeInfo::Values(_)) => {
                self.boxed(left)?;
                self.boxed(right)?;
                self.rt("rt_concat");
                Ok(Kind::Handle)
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div if matches!(result, TypeInfo::Decimal | TypeInfo::Money) => {
                self.boxed(left)?;
                self.boxed(right)?;
                self.rt(match op {
                    BinaryOp::Add => "rt_num_add",
                    BinaryOp::Sub => "rt_num_sub",
                    BinaryOp::Mul => "rt_num_mul",
                    _ => "rt_num_div",
                });
                Ok(Kind::Handle)
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul => {
                if result != TypeInfo::Integer {
                    return Err(format!("{} on {result:?} is not compiled yet", op.symbol()));
                }
                self.expr_as(left, Kind::Int)?;
                self.expr_as(right, Kind::Int)?;
                match op {
                    BinaryOp::Add => self.code.push(I::I64Add),
                    BinaryOp::Sub => self.code.push(I::I64Sub),
                    _ => self.code.push(I::I64Mul),
                }
                Ok(Kind::Int)
            }
            BinaryOp::Eq | BinaryOp::NotEq => {
                if native == Kind::Handle {
                    self.boxed(left)?;
                    self.boxed(right)?;
                } else {
                    self.expr_as(left, native)?;
                    self.expr_as(right, native)?;
                }
                self.equal(native);
                if op == BinaryOp::NotEq {
                    self.code.push(I::I32Eqz);
                }
                Ok(Kind::Bool)
            }
            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                if native == Kind::Int {
                    self.expr_as(left, Kind::Int)?;
                    self.expr_as(right, Kind::Int)?;
                    self.code.push(match op {
                        BinaryOp::Lt => I::I64LtS,
                        BinaryOp::LtEq => I::I64LeS,
                        BinaryOp::Gt => I::I64GtS,
                        _ => I::I64GeS,
                    });
                } else {
                    self.boxed(left)?;
                    self.boxed(right)?;
                    self.rt("rt_compare");
                    self.code.push(I::I32Const(0));
                    self.code.push(match op {
                        BinaryOp::Lt => I::I32LtS,
                        BinaryOp::LtEq => I::I32LeS,
                        BinaryOp::Gt => I::I32GtS,
                        _ => I::I32GeS,
                    });
                }
                Ok(Kind::Bool)
            }
            BinaryOp::Contains => {
                self.boxed(left)?;
                self.boxed(right)?;
                self.rt("rt_contains");
                Ok(Kind::Bool)
            }
            BinaryOp::Div => Err(format!("/ giving {result:?} is not compiled yet")),
            BinaryOp::Matches => Err("matches is for finding records".into()),
        }
    }

    /// A call, and the kind of what it gives, if it gives anything.
    fn call_expr(&mut self, expr: &Expr) -> Result<Option<Kind>, String> {
        let ExprKind::Call { function, args, .. } = &expr.kind else { return Err("only a call stands on its own line".into()) };
        let name = function.name.as_str();
        if name == "count" && args.len() == 1 {
            self.boxed(&args[0])?;
            self.rt("rt_count");
            return Ok(Some(Kind::Int));
        }
        if (name == "sin" || name == "cos") && args.len() == 1 {
            self.boxed(&args[0])?;
            self.rt(if name == "sin" { "rt_sin" } else { "rt_cos" });
            return Ok(Some(Kind::Handle));
        }
        if name == "round" && args.len() == 2 {
            self.boxed(&args[0])?;
            self.expr_as(&args[1], Kind::Int)?;
            self.rt("rt_round");
            return Ok(Some(Kind::Handle));
        }
        if name == "random"
            && args.len() == 1
            && let Some(service) = self.world.imported.iter().position(|s| *s == "random")
        {
            self.code.push(I::Call(service as u32));
            self.expr_as(&args[0], Kind::Int)?;
            self.rt("rt_random");
            return Ok(Some(Kind::Int));
        }
        let Some((_, index)) = self.world.inner.iter().find(|(n, _)| n == name).cloned() else {
            return Err(format!("the function '{name}' is not compiled yet: only the engine's own are"));
        };
        let callee = &self.world.functions[index as usize];
        let params: Vec<Kind> = callee.params.iter().map(|p| Kind::of_ref(&p.ty)).collect::<Result<_, _>>()?;
        let result = callee.returns.as_ref().map(Kind::of_ref).transpose()?;
        if args.len() != params.len() {
            return Err(format!("'{name}' takes {} values", params.len()));
        }
        for (arg, kind) in args.iter().zip(params) {
            self.expr_as(arg, kind)?;
        }
        let call = I::Call(self.world.support.function(index));
        self.code.push(call);
        Ok(result)
    }
}
