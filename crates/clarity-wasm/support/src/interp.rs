//! The interpreter (DESIGN 5.12): an engine written in Clarity run from its
//! IR, which the engine's file carries as data, with the values of
//! `values` and the call, kept values and answer of the compiled engines.
//! Each export of the engine's file calls `rt_run` with the place of the
//! IR, which function, the call's arguments and the function-table entries
//! of the host's `random` and `progress` (-1 when the engine does not use one).

use std::cell::OnceCell;

use clarity_engine_ir::{self as ir, Arg, Cmp, IntOp, Kind, NumOp, Place};

use crate::values::{self, Num, Value};
use crate::{fail, keep, kept, rt_begin, rt_finish, text_at, with};

thread_local! {
    static ENGINE: OnceCell<ir::Engine> = const { OnceCell::new() };
}

/// The call stopped: it failed or was refused, and why is in the call's state.
struct Stopped;

/// What a statement did: go on, or return from the function.
enum Flow {
    Next,
    Return(Option<Value>),
}

type Run<T> = Result<T, Stopped>;

fn stop<T>(why: String) -> Run<T> {
    with(|s| fail(s, why));
    Err(Stopped)
}

fn failed(value: Result<Value, String>) -> Run<Value> {
    value.or_else(stop)
}

/// Runs function `index` of the engine whose IR is at `at`.
#[unsafe(no_mangle)]
pub extern "C" fn rt_run(ir_at: i32, ir_len: i32, index: i32, at: i32, len: i32, random: i32, progress: i32) -> i64 {
    ENGINE.with(|engine| {
        let engine = engine.get_or_init(|| serde_json::from_str(&text_at(ir_at, ir_len)).expect("the engine's IR"));
        rt_begin(at, len);
        let machine = Machine { engine, random, progress };
        let _ = machine.call(index as usize);
        rt_finish()
    })
}

struct Machine<'a> {
    engine: &'a ir::Engine,
    random: i32,
    /// The function-table entry of the host's `progress`; -1 when the engine does not use it.
    progress: i32,
}

impl Machine<'_> {
    /// A call of the host: what it keeps made on the first, the arguments, the function, its answer.
    fn call(&self, index: usize) -> Run<()> {
        if with(|s| s.call.failed.is_some()) {
            return Err(Stopped);
        }
        if with(|s| s.kept.is_none()) {
            for (i, keep_start) in self.engine.keeps.iter().enumerate() {
                let value = self.held(keep_start.kind, self.expr(&keep_start.start, &mut Vec::new())?);
                with(|s| keep(s, i as i32, value));
            }
        }
        let function = &self.engine.functions[index];
        let mut args = Vec::new();
        for (i, arg) in function.args.iter().enumerate() {
            let value = with(|s| match arg {
                Arg::Int => values::arg_int(&s.call.args, i),
                Arg::Bool => values::arg_bool(&s.call.args, i),
                Arg::Decimal => values::arg_decimal(&s.call.json, i),
                Arg::Decimals => values::arg_decimals(&s.call.json, i),
                Arg::Plain => values::arg(&s.call.args, i),
            });
            args.push(failed(value)?);
        }
        let answer = self.function(index, args)?;
        with(|s| s.call.answer = answer);
        Ok(())
    }

    /// A function with its arguments: its requires, its statements, its ensures.
    fn function(&self, index: usize, args: Vec<Value>) -> Run<Option<Value>> {
        let function = &self.engine.functions[index];
        let mut locals: Vec<Value> = function.locals.iter().map(|k| empty(*k)).collect();
        for (local, value) in locals.iter_mut().zip(args) {
            *local = value;
        }
        self.conditions(&function.requires, &mut locals)?;
        let result = match self.block(&function.body, &mut locals)? {
            Flow::Return(value) => value,
            Flow::Next => function.result.map(empty),
        };
        if let (Some(held), Some(value)) = (function.result_local, &result) {
            locals[held as usize] = value.clone();
            self.conditions(&function.ensures, &mut locals)?;
        }
        Ok(result.map(|v| self.held(function.result.unwrap_or(Kind::Value), v)))
    }

    fn conditions(&self, conditions: &[ir::Condition], locals: &mut Vec<Value>) -> Run<()> {
        for condition in conditions {
            if !self.expr(&condition.test, locals)?.truth() {
                return stop(condition.written.clone());
            }
        }
        Ok(())
    }

    /// A value as the kind it is held as: a whole number or yes or no as compiled code reads it.
    fn held(&self, kind: Kind, value: Value) -> Value {
        match kind {
            Kind::Int => Value::Int(value.int()),
            Kind::Bool => Value::Bool(value.truth()),
            Kind::Value => value,
        }
    }

    fn read(&self, place: ir::Place, locals: &[Value]) -> Value {
        match place {
            Place::Local(local) => locals[local as usize].clone(),
            Place::Kept(index, _) => with(|s| kept(s, index as i32)),
        }
    }

    fn write(&self, place: ir::Place, value: Value, locals: &mut [Value]) {
        match place {
            Place::Local(local) => locals[local as usize] = value,
            Place::Kept(index, _) => with(|s| keep(s, index as i32, value)),
        }
    }

    fn block(&self, stmts: &[ir::Stmt], locals: &mut Vec<Value>) -> Run<Flow> {
        for stmt in stmts {
            if let Flow::Return(value) = self.stmt(stmt, locals)? {
                return Ok(Flow::Return(value));
            }
        }
        Ok(Flow::Next)
    }

    fn stmt(&self, stmt: &ir::Stmt, locals: &mut Vec<Value>) -> Run<Flow> {
        match stmt {
            ir::Stmt::Set(place, value) => {
                let value = self.expr(value, locals)?;
                self.write(*place, value, locals);
            }
            ir::Stmt::SetItem(place, at, value) => {
                let list = self.read(*place, locals);
                let at = self.expr(at, locals)?.int();
                let value = self.expr(value, locals)?;
                let changed = failed(values::set_item(&list, at, value))?;
                self.write(*place, changed, locals);
            }
            ir::Stmt::SetField(place, name, value) => {
                let record = self.read(*place, locals);
                let value = self.expr(value, locals)?;
                self.write(*place, values::with_field(&record, name, value), locals);
            }
            ir::Stmt::If(branches, otherwise) => {
                for (condition, body) in branches {
                    if self.expr(condition, locals)?.truth() {
                        return self.block(body, locals);
                    }
                }
                if let Some(body) = otherwise {
                    return self.block(body, locals);
                }
            }
            ir::Stmt::For { var, item, list, body } => {
                for each in self.expr(list, locals)?.items() {
                    locals[*var as usize] = self.held(*item, each);
                    if let Flow::Return(value) = self.block(body, locals)? {
                        return Ok(Flow::Return(value));
                    }
                }
            }
            ir::Stmt::Return(value) => return Ok(Flow::Return(value.as_ref().map(|v| self.expr(v, locals)).transpose()?)),
            ir::Stmt::Refuse(text) => return stop(self.expr(text, locals)?.show()),
            ir::Stmt::Tell(name, args) => {
                let mut told = Value::List(Vec::new());
                for arg in args {
                    told = values::push(&told, self.expr(arg, locals)?);
                }
                with(|s| s.call.tells.push(values::tell(name, &told)));
            }
            ir::Stmt::Eval(call) => {
                self.expr(call, locals)?;
            }
            ir::Stmt::Progress(done, total) => {
                let (done, total) = (self.expr(done, locals)?.int(), self.expr(total, locals)?.int());
                if self.progress >= 0 {
                    let tell: extern "C" fn(i64, i64) = unsafe { std::mem::transmute(self.progress as usize) };
                    tell(done, total);
                }
            }
        }
        Ok(Flow::Next)
    }

    fn expr(&self, expr: &ir::Expr, locals: &mut Vec<Value>) -> Run<Value> {
        use ir::Expr as E;
        Ok(match expr {
            E::Int(n) => Value::Int(*n),
            E::Bool(b) => Value::Bool(*b),
            E::Text(t) => Value::Text(t.clone()),
            E::Decimal(d) => values::decimal(d),
            E::Local(local, _) => locals[*local as usize].clone(),
            E::Kept(index, kind) => self.held(*kind, with(|s| kept(s, *index as i32))),
            E::Int2(op, a, b) => {
                let (a, b) = (self.expr(a, locals)?.int(), self.expr(b, locals)?.int());
                Value::Int(match op {
                    IntOp::Add => a.wrapping_add(b),
                    IntOp::Sub => a.wrapping_sub(b),
                    IntOp::Mul => a.wrapping_mul(b),
                })
            }
            E::IntCmp(cmp, a, b) => {
                let (a, b) = (self.expr(a, locals)?.int(), self.expr(b, locals)?.int());
                Value::Bool(compared(*cmp, a.cmp(&b) as i32))
            }
            E::Neg(a) => Value::Int(self.expr(a, locals)?.int().wrapping_neg()),
            E::Num(op, a, b) => {
                let (a, b) = (self.expr(a, locals)?, self.expr(b, locals)?);
                let op = match op {
                    NumOp::Add => Num::Add,
                    NumOp::Sub => Num::Sub,
                    NumOp::Mul => Num::Mul,
                    NumOp::Div => Num::Div,
                };
                failed(values::num(op, &a, &b))?
            }
            E::Compare(cmp, a, b) => {
                let (a, b) = (self.expr(a, locals)?, self.expr(b, locals)?);
                Value::Bool(compared(*cmp, values::compare(&a, &b)))
            }
            E::Eq(a, b) => Value::Bool(self.expr(a, locals)? == self.expr(b, locals)?),
            E::And(a, b) => Value::Bool(self.expr(a, locals)?.truth() && self.expr(b, locals)?.truth()),
            E::Or(a, b) => Value::Bool(self.expr(a, locals)?.truth() || self.expr(b, locals)?.truth()),
            E::Not(a) => Value::Bool(!self.expr(a, locals)?.truth()),
            E::In(value, options) => {
                let value = self.expr(value, locals)?;
                let mut found = false;
                for option in options {
                    found |= self.expr(option, locals)? == value;
                }
                Value::Bool(found)
            }
            E::Join(a, b) => values::join(&self.expr(a, locals)?, &self.expr(b, locals)?),
            E::Concat(a, b) => values::concat(&self.expr(a, locals)?, &self.expr(b, locals)?),
            E::Contains(a, b) => Value::Bool(values::contains(&self.expr(a, locals)?, &self.expr(b, locals)?)),
            E::List(items) => {
                let mut list = Value::List(Vec::new());
                for item in items {
                    list = values::push(&list, self.expr(item, locals)?);
                }
                list
            }
            E::Item(list, at, kind) => {
                let (list, at) = (self.expr(list, locals)?, self.expr(at, locals)?.int());
                self.held(*kind, failed(values::item(&list, at))?)
            }
            E::Count(list) => Value::Int(values::count(&self.expr(list, locals)?)),
            E::Record(fields) => {
                let mut record = Value::Record(Vec::new());
                for (name, value) in fields {
                    record = values::with_field(&record, name, self.expr(value, locals)?);
                }
                record
            }
            E::Field(record, name, kind) => self.held(*kind, values::field(&self.expr(record, locals)?, name)),
            E::Call(index, args, _) => {
                let callee = &self.engine.functions[*index as usize];
                let mut values = Vec::new();
                for (arg, kind) in args.iter().zip(&callee.locals) {
                    values.push(self.held(*kind, self.expr(arg, locals)?));
                }
                self.function(*index as usize, values)?.unwrap_or(Value::None)
            }
            E::Round(value, places) => values::round(&self.expr(value, locals)?, self.expr(places, locals)?.int()),
            E::Sin(degrees) => values::angle(&self.expr(degrees, locals)?, f64::sin),
            E::Cos(degrees) => values::angle(&self.expr(degrees, locals)?, f64::cos),
            E::Random(max) => {
                let max = self.expr(max, locals)?.int();
                Value::Int(failed(values::random(self.next_random(), max).map(Value::Int))?.int())
            }
        })
    }

    /// The host's next random number, through the function table.
    fn next_random(&self) -> i64 {
        let next: extern "C" fn() -> i64 = unsafe { std::mem::transmute(self.random as usize) };
        next()
    }
}

/// The empty value a local of `kind` starts as.
fn empty(kind: Kind) -> Value {
    match kind {
        Kind::Int => Value::Int(0),
        Kind::Bool => Value::Bool(false),
        Kind::Value => Value::None,
    }
}

fn compared(cmp: Cmp, order: i32) -> bool {
    match cmp {
        Cmp::Lt => order < 0,
        Cmp::LtEq => order <= 0,
        Cmp::Gt => order > 0,
        Cmp::GtEq => order >= 0,
    }
}
