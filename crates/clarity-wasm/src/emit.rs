//! The engine's IR as wasm code (compiled mode).
//!
//! A whole number is an i64 and yes or no an i32; every other value is a
//! handle (an i32) to a value in the support module. A call stops at the
//! first thing that fails or refuses: after each statement, and before a
//! condition or a list is used, the code asks `rt_failed` and returns.

use clarity_engine_ir::{self as ir, Arg, Cmp, IntOp, Kind, NumOp, Place};
use wasm_encoder::{BlockType, Function, Instruction as I, ValType};

use crate::merge::{Code, Support};

fn val(kind: Kind) -> ValType {
    match kind {
        Kind::Int => ValType::I64,
        Kind::Bool | Kind::Value => ValType::I32,
    }
}

/// The services of Clarity an engine may import, with their types.
pub(crate) const SERVICES: [(&str, &[ValType], &[ValType]); 5] = [
    ("clock", &[], &[ValType::I64]),
    ("random", &[], &[ValType::I64]),
    ("log", &[ValType::I32, ValType::I32], &[]),
    ("language", &[ValType::I32, ValType::I32], &[ValType::I32]),
    ("progress", &[ValType::I64, ValType::I64], &[]),
];

/// The engine's functions: an inner one and an export per function, one
/// that makes what it keeps on its first call, then the statements of each
/// function with ensures, as a function of their own.
pub fn engine(support: &Support, engine: &ir::Engine) -> Code {
    let mut code = Code::default();
    for name in &engine.services {
        let (_, params, results) = SERVICES.iter().find(|(n, ..)| n == name).expect("a service of Clarity");
        let ty = code.ty(params.to_vec(), results.to_vec());
        code.imports.push((name.clone(), ty));
    }
    let count = engine.functions.len() as u32;
    let start = 2 * count;
    let mut world = World { support, engine, texts: Vec::new(), base: support.texts_at(), extra: Vec::new(), extra_base: start + 1 };
    let inners: Vec<_> = engine.functions.iter().map(|f| world.function(f)).collect();
    for (params, results, function) in inners {
        let ty = code.ty(params, results);
        code.functions.push((ty, function));
    }
    for (i, f) in engine.functions.iter().enumerate() {
        let made = world.export(f, i as u32, if engine.keeps.is_empty() { None } else { Some(start) });
        let ty = code.ty(vec![ValType::I32, ValType::I32], vec![ValType::I64]);
        code.functions.push((ty, made));
        code.exports.push((f.name.clone(), count + i as u32));
    }
    let keeps = world.keeps_start();
    let ty = code.ty(vec![], vec![]);
    code.functions.push((ty, keeps));
    for (params, results, function) in std::mem::take(&mut world.extra) {
        let ty = code.ty(params, results);
        code.functions.push((ty, function));
    }
    code.texts = world.texts;
    code
}

struct World<'a> {
    support: &'a Support,
    engine: &'a ir::Engine,
    texts: Vec<u8>,
    base: u32,
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
    fn function(&mut self, f: &ir::Function) -> (Vec<ValType>, Vec<ValType>, Function) {
        if f.ensures.is_empty() {
            return self.statements(f, true);
        }
        let statements = self.statements(f, false);
        self.extra.push(statements);
        let index = self.extra_base + self.extra.len() as u32 - 1;
        self.ensuring(f, index)
    }

    fn signature(f: &ir::Function) -> (Vec<ValType>, Vec<ValType>) {
        let params = f.locals[..f.args.len()].iter().map(|k| val(*k)).collect();
        (params, f.result.map(|k| vec![val(k)]).unwrap_or_default())
    }

    /// The function's statements, after its requires when `requires`.
    fn statements(&mut self, f: &ir::Function, requires: bool) -> (Vec<ValType>, Vec<ValType>, Function) {
        let (params, results) = Self::signature(f);
        let mut body = Body::new(self, f);
        // An argument that did not fit has failed the call already.
        body.check_failed();
        if requires {
            body.conditions(&f.requires);
        }
        body.block(&f.body);
        // Falling off the end gives nothing, or a whole number 0 and the like.
        body.dummy();
        body.code.push(I::End);
        (params, results, body.finish())
    }

    /// Its requires, its statements (function `index`), then its ensures on its result.
    fn ensuring(&mut self, f: &ir::Function, index: u32) -> (Vec<ValType>, Vec<ValType>, Function) {
        let (params, results) = Self::signature(f);
        let held = f.result_local.expect("ensures read the result");
        let mut body = Body::new(self, f);
        body.check_failed();
        body.conditions(&f.requires);
        for local in 0..f.args.len() as u32 {
            body.code.push(I::LocalGet(local));
        }
        let call = I::Call(body.world.support.function(index));
        body.code.extend([call, I::LocalSet(held)]);
        body.check_failed();
        body.conditions(&f.ensures);
        body.code.extend([I::LocalGet(held), I::End]);
        (params, results, body.finish())
    }

    /// The export the host calls: the arguments from JSON, the inner function, the answer.
    fn export(&mut self, f: &ir::Function, index: u32, keeps_start: Option<u32>) -> Function {
        let mut code = vec![I::LocalGet(0), I::LocalGet(1), self.rt("rt_begin")];
        if let Some(start) = keeps_start {
            code.extend([self.rt("rt_unkept"), I::If(BlockType::Empty), I::Call(self.support.function(start)), I::End]);
        }
        for (i, arg) in f.args.iter().enumerate() {
            code.push(I::I32Const(i as i32));
            code.push(self.rt(match arg {
                Arg::Int => "rt_arg_int",
                Arg::Bool => "rt_arg_bool",
                Arg::Decimal => "rt_arg_dec",
                Arg::Decimals => "rt_arg_decs",
                Arg::Plain => "rt_arg",
            }));
        }
        code.push(I::Call(self.support.function(index)));
        match f.result {
            Some(Kind::Int) => code.push(self.rt("rt_answer_int")),
            Some(Kind::Bool) => code.push(self.rt("rt_answer_bool")),
            Some(Kind::Value) => code.push(self.rt("rt_answer")),
            None => {}
        }
        code.extend([self.rt("rt_finish"), I::End]);
        let mut function = Function::new(Vec::<(u32, ValType)>::new());
        for i in &code {
            function.instruction(i);
        }
        function
    }

    /// Makes what the engine keeps, as written, on its first call.
    fn keeps_start(&mut self) -> Function {
        let none = ir::Function { name: String::new(), args: Vec::new(), locals: Vec::new(), result: None, requires: Vec::new(), ensures: Vec::new(), result_local: None, body: Vec::new() };
        let engine = self.engine;
        let mut body = Body::new(self, &none);
        for (index, keep) in engine.keeps.iter().enumerate() {
            body.code.push(I::I32Const(index as i32));
            body.expr_as(&keep.start, keep.kind);
            let set = body.keep_setter(keep.kind);
            body.code.push(set);
        }
        body.code.push(I::End);
        body.finish()
    }
}

/// One function's code as it is written.
struct Body<'w, 'a> {
    world: &'w mut World<'a>,
    params: usize,
    /// Every local's kind, the function's own first, then those this writes.
    locals: Vec<Kind>,
    code: Vec<I<'static>>,
    result: Option<Kind>,
}

impl<'w, 'a> Body<'w, 'a> {
    fn new(world: &'w mut World<'a>, f: &ir::Function) -> Self {
        Body { world, params: f.args.len(), locals: f.locals.clone(), code: Vec::new(), result: f.result }
    }

    fn finish(self) -> Function {
        let mut function = Function::new(self.locals[self.params..].iter().map(|k| (1, val(*k))));
        for i in &self.code {
            function.instruction(i);
        }
        function
    }

    fn local(&mut self, kind: Kind) -> u32 {
        self.locals.push(kind);
        self.locals.len() as u32 - 1
    }

    fn rt(&mut self, name: &str) {
        let call = self.world.rt(name);
        self.code.push(call);
    }

    /// Each condition that does not hold fails the call with it as written.
    fn conditions(&mut self, conditions: &[ir::Condition]) {
        for condition in conditions {
            self.expr_as(&condition.test, Kind::Bool);
            self.code.extend([I::I32Eqz, I::If(BlockType::Empty)]);
            self.text_const(&condition.written);
            self.rt("rt_refuse");
            self.dummy();
            self.code.extend([I::Return, I::End]);
        }
    }

    /// The function's result when it stops early: nothing, 0 or no.
    fn dummy(&mut self) {
        match self.result {
            Some(Kind::Int) => self.code.push(I::I64Const(0)),
            Some(Kind::Bool | Kind::Value) => self.code.push(I::I32Const(0)),
            None => {}
        }
    }

    /// Returns when the call has failed or been refused.
    fn check_failed(&mut self) {
        self.rt("rt_failed");
        self.code.push(I::If(BlockType::Empty));
        self.dummy();
        self.code.extend([I::Return, I::End]);
    }

    fn keep_setter(&self, kind: Kind) -> I<'static> {
        self.world.rt(match kind {
            Kind::Int => "rt_keep_int",
            Kind::Bool => "rt_keep_bool",
            Kind::Value => "rt_keep",
        })
    }

    /// The value at a place, as a handle.
    fn read(&mut self, place: Place) {
        match place {
            Place::Local(local) => self.code.push(I::LocalGet(local)),
            Place::Kept(index, _) => {
                self.code.push(I::I32Const(index as i32));
                self.rt("rt_kept");
            }
        }
    }

    /// The handle on the stack, back in its place.
    fn write(&mut self, place: Place) {
        match place {
            Place::Local(local) => self.code.push(I::LocalSet(local)),
            Place::Kept(index, _) => {
                let changed = self.local(Kind::Value);
                self.code.extend([I::LocalSet(changed), I::I32Const(index as i32), I::LocalGet(changed)]);
                self.rt("rt_keep");
            }
        }
    }

    fn block(&mut self, stmts: &[ir::Stmt]) {
        for stmt in stmts {
            self.stmt(stmt);
            self.check_failed();
        }
    }

    fn stmt(&mut self, stmt: &ir::Stmt) {
        match stmt {
            ir::Stmt::Set(Place::Local(local), value) => {
                let kind = self.locals[*local as usize];
                self.expr_as(value, kind);
                self.code.push(I::LocalSet(*local));
            }
            ir::Stmt::Set(Place::Kept(index, kind), value) => {
                self.code.push(I::I32Const(*index as i32));
                self.expr_as(value, *kind);
                let set = self.keep_setter(*kind);
                self.code.push(set);
            }
            ir::Stmt::SetItem(place, at, value) => {
                self.read(*place);
                self.expr_as(at, Kind::Int);
                self.boxed(value);
                self.rt("rt_set_item");
                self.write(*place);
            }
            ir::Stmt::SetField(place, name, value) => {
                self.read(*place);
                self.name(name);
                self.boxed(value);
                self.rt("rt_with");
                self.write(*place);
            }
            ir::Stmt::If(branches, otherwise) => self.branches(branches, otherwise.as_deref()),
            ir::Stmt::Return(value) => {
                match (value, self.result) {
                    (Some(value), Some(kind)) => {
                        self.expr_as(value, kind);
                        self.check_failed();
                    }
                    _ => self.dummy(),
                }
                self.code.push(I::Return);
            }
            ir::Stmt::For { var, item, list, body } => {
                self.boxed(list);
                let items = self.local(Kind::Value);
                self.code.push(I::LocalSet(items));
                self.check_failed();
                let count = self.local(Kind::Int);
                self.code.push(I::LocalGet(items));
                self.rt("rt_count");
                self.code.push(I::LocalSet(count));
                let at = self.local(Kind::Int);
                self.code.extend([I::I64Const(0), I::LocalSet(at), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
                self.code.extend([I::LocalGet(at), I::I64Const(1), I::I64Add, I::LocalTee(at), I::LocalGet(count), I::I64GtS, I::BrIf(1)]);
                self.code.extend([I::LocalGet(items), I::LocalGet(at)]);
                self.item(*item);
                self.code.push(I::LocalSet(*var));
                self.block(body);
                self.code.extend([I::Br(0), I::End, I::End]);
            }
            ir::Stmt::Eval(call) => {
                if self.expr(call).is_some() {
                    self.code.push(I::Drop);
                }
            }
            ir::Stmt::Progress(done, total) => {
                let service = self.world.engine.services.iter().position(|s| s == "progress").expect("progress is imported");
                self.expr(done);
                self.expr(total);
                self.code.push(I::Call(service as u32));
            }
            // `tell finished(time)`: its values as a list, in the call's answer.
            ir::Stmt::Tell(name, args) => {
                self.name(name);
                self.rt("rt_list");
                for arg in args {
                    self.boxed(arg);
                    self.rt("rt_push");
                }
                self.rt("rt_tell");
            }
            ir::Stmt::Refuse(text) => {
                self.boxed(text);
                self.rt("rt_refuse");
                self.dummy();
                self.code.push(I::Return);
            }
        }
    }

    fn branches(&mut self, branches: &[(ir::Expr, Vec<ir::Stmt>)], otherwise: Option<&[ir::Stmt]>) {
        let Some(((condition, body), rest)) = branches.split_first() else {
            if let Some(body) = otherwise {
                self.block(body);
            }
            return;
        };
        self.expr_as(condition, Kind::Bool);
        let held = self.local(Kind::Bool);
        self.code.push(I::LocalSet(held));
        self.check_failed();
        self.code.extend([I::LocalGet(held), I::If(BlockType::Empty)]);
        self.block(body);
        if !rest.is_empty() || otherwise.is_some() {
            self.code.push(I::Else);
            self.branches(rest, otherwise);
        }
        self.code.push(I::End);
    }

    /// An expression whose value is used as `kind`: a value takes any, as a handle.
    fn expr_as(&mut self, expr: &ir::Expr, kind: Kind) {
        if kind == Kind::Value {
            self.boxed(expr);
        } else {
            self.expr(expr);
        }
    }

    /// An expression as a handle, whatever it holds.
    fn boxed(&mut self, expr: &ir::Expr) {
        match self.expr(expr) {
            Some(Kind::Int) => self.rt("rt_int"),
            Some(Kind::Bool) => self.rt("rt_bool"),
            _ => {}
        }
    }

    fn text_const(&mut self, text: &str) {
        self.name(text);
        self.rt("rt_text");
    }

    /// A text's place and length, for the support functions that take a name.
    fn name(&mut self, text: &str) {
        let (at, len) = self.world.text(text);
        self.code.extend([I::I32Const(at), I::I32Const(len)]);
    }

    fn item(&mut self, kind: Kind) {
        self.rt(match kind {
            Kind::Int => "rt_item_int",
            Kind::Bool => "rt_item_bool",
            Kind::Value => "rt_item",
        });
    }

    /// Whether the two values on the stack, held as `kind`, are equal.
    fn equal(&mut self, kind: Kind) {
        match kind {
            Kind::Int => self.code.push(I::I64Eq),
            Kind::Bool => self.code.push(I::I32Eq),
            Kind::Value => self.rt("rt_eq"),
        }
    }

    /// How two values are compared: natively when both are held alike, else as handles.
    fn common(a: &ir::Expr, b: &ir::Expr) -> Kind {
        match (a.kind(), b.kind()) {
            (Some(x), Some(y)) if x == y => x,
            _ => Kind::Value,
        }
    }

    fn expr(&mut self, expr: &ir::Expr) -> Option<Kind> {
        use ir::Expr as E;
        match expr {
            E::Int(n) => self.code.push(I::I64Const(*n)),
            E::Bool(b) => self.code.push(I::I32Const(*b as i32)),
            E::Text(text) => self.text_const(text),
            E::Decimal(number) => {
                self.name(number);
                self.rt("rt_dec");
            }
            E::Local(local, _) => self.code.push(I::LocalGet(*local)),
            E::Kept(index, kind) => {
                self.code.push(I::I32Const(*index as i32));
                self.rt(match kind {
                    Kind::Int => "rt_kept_int",
                    Kind::Bool => "rt_kept_bool",
                    Kind::Value => "rt_kept",
                });
            }
            E::Int2(op, a, b) => {
                self.expr(a);
                self.expr(b);
                self.code.push(match op {
                    IntOp::Add => I::I64Add,
                    IntOp::Sub => I::I64Sub,
                    IntOp::Mul => I::I64Mul,
                });
            }
            E::IntCmp(cmp, a, b) => {
                self.expr(a);
                self.expr(b);
                self.code.push(match cmp {
                    Cmp::Lt => I::I64LtS,
                    Cmp::LtEq => I::I64LeS,
                    Cmp::Gt => I::I64GtS,
                    Cmp::GtEq => I::I64GeS,
                });
            }
            E::Neg(a) => {
                self.code.push(I::I64Const(0));
                self.expr(a);
                self.code.push(I::I64Sub);
            }
            E::Num(op, a, b) => {
                self.boxed(a);
                self.boxed(b);
                self.rt(match op {
                    NumOp::Add => "rt_num_add",
                    NumOp::Sub => "rt_num_sub",
                    NumOp::Mul => "rt_num_mul",
                    NumOp::Div => "rt_num_div",
                });
            }
            E::Compare(cmp, a, b) => {
                self.boxed(a);
                self.boxed(b);
                self.rt("rt_compare");
                self.code.push(I::I32Const(0));
                self.code.push(match cmp {
                    Cmp::Lt => I::I32LtS,
                    Cmp::LtEq => I::I32LeS,
                    Cmp::Gt => I::I32GtS,
                    Cmp::GtEq => I::I32GeS,
                });
            }
            E::Eq(a, b) => {
                let kind = Self::common(a, b);
                self.expr_as(a, kind);
                self.expr_as(b, kind);
                self.equal(kind);
            }
            E::And(a, b) => {
                self.expr(a);
                self.code.push(I::If(BlockType::Result(ValType::I32)));
                self.expr(b);
                self.code.extend([I::Else, I::I32Const(0), I::End]);
            }
            E::Or(a, b) => {
                self.expr(a);
                self.code.extend([I::If(BlockType::Result(ValType::I32)), I::I32Const(1), I::Else]);
                self.expr(b);
                self.code.push(I::End);
            }
            E::Not(a) => {
                self.expr(a);
                self.code.push(I::I32Eqz);
            }
            E::In(value, options) => {
                let alike = value.kind().filter(|k| *k != Kind::Value && options.iter().all(|o| o.kind() == Some(*k)));
                let kind = alike.unwrap_or(Kind::Value);
                self.expr_as(value, kind);
                let held = self.local(kind);
                self.code.extend([I::LocalSet(held), I::I32Const(0)]);
                for option in options {
                    self.code.push(I::LocalGet(held));
                    self.expr_as(option, kind);
                    self.equal(kind);
                    self.code.push(I::I32Or);
                }
            }
            E::Join(a, b) | E::Concat(a, b) | E::Contains(a, b) => {
                self.boxed(a);
                self.boxed(b);
                self.rt(match expr {
                    E::Join(..) => "rt_join",
                    E::Concat(..) => "rt_concat",
                    _ => "rt_contains",
                });
            }
            E::List(items) => {
                self.rt("rt_list");
                for item in items {
                    self.boxed(item);
                    self.rt("rt_push");
                }
            }
            E::Item(list, at, kind) => {
                self.boxed(list);
                self.expr(at);
                self.item(*kind);
            }
            E::Count(list) => {
                self.boxed(list);
                self.rt("rt_count");
            }
            E::Record(fields) => {
                self.rt("rt_record");
                for (name, value) in fields {
                    self.name(name);
                    self.boxed(value);
                    self.rt("rt_with");
                }
            }
            E::Field(record, name, kind) => {
                self.boxed(record);
                self.name(name);
                self.rt(match kind {
                    Kind::Int => "rt_field_int",
                    Kind::Bool => "rt_field_bool",
                    Kind::Value => "rt_field",
                });
            }
            E::Call(index, args, _) => {
                let callee = &self.world.engine.functions[*index as usize];
                let kinds = callee.locals[..callee.args.len()].to_vec();
                for (arg, kind) in args.iter().zip(kinds) {
                    self.expr_as(arg, kind);
                }
                let call = I::Call(self.world.support.function(*index));
                self.code.push(call);
            }
            E::Round(value, places) => {
                self.boxed(value);
                self.expr(places);
                self.rt("rt_round");
            }
            E::Sin(degrees) | E::Cos(degrees) => {
                self.boxed(degrees);
                self.rt(if matches!(expr, E::Sin(_)) { "rt_sin" } else { "rt_cos" });
            }
            E::Random(max) => {
                let service = self.world.engine.services.iter().position(|s| s == "random").expect("random is imported");
                self.code.push(I::Call(service as u32));
                self.expr(max);
                self.rt("rt_random");
            }
        }
        expr.kind()
    }
}
