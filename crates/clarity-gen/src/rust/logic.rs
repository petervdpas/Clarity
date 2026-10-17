//! Functions and actions as Rust functions. Types come from the checker's
//! table, so the emitter never decides a type on its own: it only chooses
//! how to write it in Rust (borrowing records, widening integers, loading
//! referenced records through the database).

use std::collections::{HashMap, HashSet};

use clarity_spec::Project;
use clarity_spec::code::check::{FileTypes, NameKind, TypeInfo};
use clarity_spec::code::{
    BinaryOp, Expr, ExprKind, FunctionDecl, FunctionKind, MessageLevel, OrderKey, Stmt, StmtKind, TypeRef, TypeRefKind,
    UnaryOp,
};
use clarity_spec::model::{Command, FieldType, Frame, SpecFile, Widget};

use super::names::{decimal, ident, value_type};
use crate::print;

pub fn emit(project: &Project, file: &SpecFile, types: &FileTypes) -> Result<String, Vec<String>> {
    let mut out = String::new();
    let mut errors = Vec::new();
    // Data events are actions on their record, run by the record's hooks.
    let events: Vec<FunctionDecl> = file
        .code
        .iter()
        .filter(|b| file.entities.iter().any(|e| e.name.name == b.name.name))
        .flat_map(|b| b.events.iter().map(|e| e.action(&b.name.name)))
        .collect();
    for decl in file.functions.iter().chain(&events) {
        let mut emitter = Emitter {
            project,
            types,
            decl: Some(decl),
            window: None,
            refs: vec![HashSet::new()],
            errors: Vec::new(),
        };
        let code = emitter.function();
        if emitter.errors.is_empty() {
            out.push_str(&code);
        } else {
            errors.extend(emitter.errors.into_iter().map(|e| format!("{} '{}': {e}", decl.kind.keyword(), decl.name.name)));
        }
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// What a `TypeRef` names, as a checker type.
fn type_ref_info(project: &Project, ty: &TypeRef) -> TypeInfo {
    match &ty.kind {
        TypeRefKind::Text => TypeInfo::Text,
        TypeRefKind::Integer => TypeInfo::Integer,
        TypeRefKind::Decimal => TypeInfo::Decimal,
        TypeRefKind::Money => TypeInfo::Money,
        TypeRefKind::Bool => TypeInfo::Bool,
        TypeRefKind::Date => TypeInfo::Date,
        TypeRefKind::DateTime => TypeInfo::DateTime,
        TypeRefKind::Named { name } if project.entity(name).is_some() => TypeInfo::Record(name.clone()),
        TypeRefKind::Named { name } => TypeInfo::Enum(name.clone()),
        TypeRefKind::List { item } => TypeInfo::List(item.clone()),
    }
}

/// Values of these types are copied; the others are cloned when read.
fn is_copy(ty: &TypeInfo) -> bool {
    !matches!(ty, TypeInfo::Text | TypeInfo::Record(_) | TypeInfo::List(_))
}

fn is_decimal(ty: &TypeInfo) -> bool {
    matches!(ty, TypeInfo::Decimal | TypeInfo::Money)
}

/// Widens an integer where a decimal is expected.
fn coerce(code: String, from: &TypeInfo, to: &TypeInfo) -> String {
    if *from == TypeInfo::Integer && is_decimal(to) {
        format!("Decimal::from({code})")
    } else {
        code
    }
}

struct Emitter<'a> {
    project: &'a Project,
    types: &'a FileTypes,
    /// The function or action being written; None for a handler.
    decl: Option<&'a FunctionDecl>,
    /// The window or app of a handler.
    window: Option<WindowCtx>,
    /// Locals that hold a borrowed record or list (parameters and `for`
    /// variables), per block.
    refs: Vec<HashSet<String>>,
    errors: Vec<String>,
}

impl Emitter<'_> {
    fn is_ref(&self, name: &str) -> bool {
        self.refs.iter().any(|scope| scope.contains(name))
    }

    fn returns(&self) -> Option<TypeInfo> {
        self.decl.and_then(|d| d.returns.as_ref()).map(|r| type_ref_info(self.project, r))
    }

    fn function(&mut self) -> String {
        let decl = self.decl.expect("a function or action");
        let action = decl.kind != FunctionKind::Function;
        let mut params = vec!["db: &Db".to_string()];
        for param in &decl.params {
            let info = type_ref_info(self.project, &param.ty);
            let name = ident(&param.name.name);
            let ty = match &info {
                TypeInfo::Record(e) if action => format!("&mut {e}"),
                TypeInfo::Record(e) => format!("&{e}"),
                TypeInfo::List(e) => format!("&[{e}]"),
                other => value_type(other),
            };
            if matches!(info, TypeInfo::Record(_) | TypeInfo::List(_)) {
                self.refs[0].insert(param.name.name.clone());
            }
            params.push(format!("{name}: {ty}"));
        }
        let ret = self.returns().map(|r| value_type(&r)).unwrap_or_else(|| "()".into());
        let mut out = format!("pub fn {}({}) -> Result<{ret}> {{\n", ident(&decl.name.name), params.join(", "));
        for condition in &decl.requires {
            out.push_str(&format!(
                "    require({}, {:?})?;\n",
                self.value(condition),
                print::expr(condition)
            ));
        }
        let mut body = String::new();
        self.block(&decl.body, 1, &mut body);
        if self.returns().is_none() {
            body.push_str("    Ok(())\n");
        }
        if decl.ensures.is_empty() {
            out.push_str(&body);
        } else {
            let indented: String = body.lines().map(|l| format!("    {l}\n")).collect();
            out.push_str(&format!("    let result: {ret} = (|| -> Result<{ret}> {{\n{indented}    }})()?;\n"));
            for condition in &decl.ensures {
                out.push_str(&format!(
                    "    ensure({}, {:?})?;\n",
                    self.value(condition),
                    print::expr(condition)
                ));
            }
            out.push_str("    Ok(result)\n");
        }
        out.push_str("}\n\n");
        out
    }

    fn block(&mut self, body: &[Stmt], depth: usize, out: &mut String) {
        self.refs.push(HashSet::new());
        for stmt in body {
            self.stmt(stmt, depth, out);
        }
        self.refs.pop();
    }

    fn stmt(&mut self, stmt: &Stmt, depth: usize, out: &mut String) {
        let pad = "    ".repeat(depth);
        match &stmt.kind {
            StmtKind::Let { name, ty, value } => {
                let from = self.types.type_of(value);
                let code = match ty {
                    Some(declared) => {
                        let to = type_ref_info(self.project, declared);
                        format!(
                            "let mut {}: {} = {};",
                            ident(&name.name),
                            value_type(&to),
                            coerce(self.value(value), &from, &to)
                        )
                    }
                    None => format!("let mut {} = {};", ident(&name.name), self.value(value)),
                };
                out.push_str(&format!("{pad}{code}\n"));
            }
            StmtKind::Assign { target, value } => {
                let to = self.types.type_of(target);
                let from = self.types.type_of(value);
                if let Some((part, property)) = self.part_property(target) {
                    let value = coerce(self.value(value), &from, &to);
                    out.push_str(&format!("{pad}ui.set_{property}({part:?}, {value});\n"));
                    return;
                }
                let place = self.place(target);
                let value = coerce(self.value(value), &from, &to);
                out.push_str(&format!("{pad}{place} = {value};\n"));
            }
            StmtKind::If { branches, otherwise } => {
                for (i, branch) in branches.iter().enumerate() {
                    let keyword = if i == 0 { format!("{pad}if") } else { " else if".to_string() };
                    out.push_str(&format!("{keyword} {} {{\n", self.value(&branch.condition)));
                    self.block(&branch.body, depth + 1, out);
                    out.push_str(&format!("{pad}}}"));
                }
                if let Some(body) = otherwise {
                    out.push_str(" else {\n");
                    self.block(body, depth + 1, out);
                    out.push_str(&format!("{pad}}}"));
                }
                out.push('\n');
            }
            StmtKind::For { var, list, body } => {
                let source = match self.held(list) {
                    Some((place, _)) => place,
                    None => format!("({})", self.value(list)),
                };
                out.push_str(&format!("{pad}for {} in {source}.iter() {{\n", ident(&var.name)));
                self.refs.push(HashSet::from([var.name.clone()]));
                self.block(body, depth + 1, out);
                self.refs.pop();
                out.push_str(&format!("{pad}}}\n"));
            }
            StmtKind::Return { value: Some(value) } => {
                let to = self.returns().unwrap_or(TypeInfo::Unknown);
                let from = self.types.type_of(value);
                out.push_str(&format!("{pad}return Ok({});\n", coerce(self.value(value), &from, &to)));
            }
            StmtKind::Return { value: None } => out.push_str(&format!("{pad}return Ok(());\n")),
            StmtKind::Call { call } => {
                let code = self.value(call);
                if matches!(self.types.type_of(call), TypeInfo::Unknown) {
                    out.push_str(&format!("{pad}{code};\n"));
                } else {
                    out.push_str(&format!("{pad}let _ = {code};\n"));
                }
            }
            StmtKind::Save { value } => {
                let record = self.record_mut(value, "save");
                out.push_str(&format!("{pad}db.save({record})?;\n"));
            }
            StmtKind::Delete { value } => {
                let record = self.record_mut(value, "delete");
                out.push_str(&format!("{pad}db.delete({record})?;\n"));
            }
            StmtKind::Refuse { text } => {
                let text = if self.types.type_of(text) == TypeInfo::Text {
                    self.value(text)
                } else {
                    format!("{}.show()", self.value(text))
                };
                out.push_str(&format!("{pad}return Err(Error::Refused({text}));\n"));
            }
            StmtKind::Message { .. } | StmtKind::Open { .. } | StmtKind::Close if self.window.is_none() => {
                self.errors.push("messages and windows belong in handlers".into());
            }
            StmtKind::Message { level, text } => {
                let level = match level {
                    MessageLevel::Info => "Info",
                    MessageLevel::Warning => "Warning",
                    MessageLevel::Error => "Error",
                };
                let text = if self.types.type_of(text) == TypeInfo::Text {
                    self.value(text)
                } else {
                    format!("{}.show()", self.value(text))
                };
                out.push_str(&format!("{pad}ui.message(Level::{level}, {text});\n"));
            }
            StmtKind::Open { window, with } => {
                let args: Vec<String> = with
                    .iter()
                    .map(|arg| format!("({:?}, to_value(&{})?)", arg.name.name, self.value(&arg.value)))
                    .collect();
                out.push_str(&format!("{pad}ui.open({:?}, vec![{}]);\n", window.name, args.join(", ")));
            }
            StmtKind::Close => out.push_str(&format!("{pad}ui.close = true;\n")),
            StmtKind::Reload { data } => {
                let found = self.window.as_ref().and_then(|w| w.data.iter().find(|(n, _, _)| *n == data.name).cloned());
                match found {
                    Some((name, entity, true)) => {
                        out.push_str(&format!("{pad}w.{} = db.all::<{entity}>()?;\n", ident(&name)));
                    }
                    Some((name, entity, false)) => {
                        let field = format!("w.{}", ident(&name));
                        out.push_str(&format!("{pad}if {field}.id != 0 {{\n{pad}    {field} = db.get::<{entity}>({field}.id)?;\n{pad}}}\n"));
                    }
                    None => self.errors.push(format!("there is no data '{}' to reload", data.name)),
                }
            }
        }
    }

    /// A name that holds a record or list, as a place, and whether it is
    /// already borrowed (a parameter or `for` variable): locals and the
    /// window's data.
    fn held(&self, expr: &Expr) -> Option<(String, bool)> {
        let ExprKind::Name { name } = &expr.kind else { return None };
        match self.types.name_of(expr) {
            Some(NameKind::Local) => Some((ident(name), self.is_ref(name))),
            Some(NameKind::Data) => Some((format!("w.{}", ident(name)), false)),
            _ => None,
        }
    }

    /// A part property such as `part.visible` or `part.error`: the part and
    /// the property, when `expr` is one.
    fn part_property(&self, expr: &Expr) -> Option<(String, &'static str)> {
        let ExprKind::Member { object, member } = &expr.kind else { return None };
        let ExprKind::Name { name } = &object.kind else { return None };
        if self.types.name_of(object) != Some(&NameKind::Part) {
            return None;
        }
        let property = match member.name.as_str() {
            "visible" => "visible",
            "enabled" => "enabled",
            "caption" => "caption",
            "text" => "text",
            "error" => "error",
            "readonly" => "readonly",
            "selected" => "selected",
            _ => return None,
        };
        Some((name.clone(), property))
    }

    /// Reading a part property in a handler.
    fn read_part(&mut self, part: &str, property: &str) -> String {
        let info = self.window.as_ref().and_then(|w| w.parts.get(part).cloned());
        match (property, info) {
            ("caption", Some(info)) => format!("ui.caption({part:?}, {:?})", info.caption.unwrap_or_default()),
            ("readonly", Some(info)) => format!("ui.readonly({part:?}, {})", info.readonly),
            ("selected", Some(PartInfo { bind: Some(bind), .. })) => {
                format!("ui.selected({part:?}, &w.{})?", ident(&bind))
            }
            ("selected", _) => {
                self.errors.push(format!("'{part}' is not bound to a list"));
                String::new()
            }
            (property, _) => format!("ui.{property}({part:?})"),
        }
    }

    /// The record `save` or `delete` works on: a name holding it.
    fn record_mut(&mut self, value: &Expr, word: &str) -> String {
        match self.held(value) {
            Some((place, true)) => place,
            Some((place, false)) => format!("&mut {place}"),
            None => {
                self.errors.push(format!(
                    "'{word} {}' needs the record in a name; use 'let' first",
                    print::expr(value)
                ));
                String::new()
            }
        }
    }

    /// An expression as a place: what can be assigned or borrowed.
    fn place(&mut self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Name { name } => match self.types.name_of(expr) {
                Some(NameKind::Local) if self.is_ref(name) && matches!(self.types.type_of(expr), TypeInfo::Record(_)) => {
                    format!("(*{})", ident(name))
                }
                Some(NameKind::Local) => ident(name),
                Some(NameKind::Data) => format!("w.{}", ident(name)),
                Some(NameKind::EnumValue(e)) => format!("{e}::{name}"),
                _ => {
                    self.errors.push(format!("'{name}' is a part; use one of its properties"));
                    String::new()
                }
            },
            ExprKind::Member { object, member } => {
                if let Some(NameKind::EnumValue(e)) = self.types.name_of(expr) {
                    return format!("{e}::{}", member.name);
                }
                let TypeInfo::Record(entity) = self.types.type_of(object) else {
                    self.errors.push(format!("'{}' is not generated yet", print::expr(expr)));
                    return String::new();
                };
                let field = self.project.entity(&entity).and_then(|e| e.field(&member.name)).map(|f| f.ty.clone());
                let object = self.object(object);
                match field {
                    Some(FieldType::Ref { entity: target }) => {
                        format!("db.load::<{}>({object}.{})?", target.name, ident(&member.name))
                    }
                    Some(FieldType::List { .. }) => {
                        self.errors.push(format!("list fields like '{}' are not loaded yet", print::expr(expr)));
                        String::new()
                    }
                    _ => format!("{object}.{}", ident(&member.name)),
                }
            }
            _ => self.value(expr),
        }
    }

    /// The object of a member access: a record, borrowed or loaded.
    fn object(&mut self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Name { name } if self.is_ref(name) => ident(name),
            _ => self.place(expr),
        }
    }

    /// An expression as an owned value.
    fn value(&mut self, expr: &Expr) -> String {
        let ty = self.types.type_of(expr);
        match &expr.kind {
            ExprKind::Number { value } if is_decimal(&ty) => decimal(value),
            ExprKind::Number { value } => format!("{value}i64"),
            ExprKind::Str { value } => format!("String::from({value:?})"),
            ExprKind::Bool { value } => value.to_string(),
            ExprKind::Member { .. } if self.part_property(expr).is_some() => {
                let (part, property) = self.part_property(expr).unwrap_or_default();
                self.read_part(&part, property)
            }
            ExprKind::Name { .. } | ExprKind::Member { .. } => {
                let place = self.place(expr);
                let loaded = place.ends_with(")?");
                if is_copy(&ty) || loaded { place } else { format!("{place}.clone()") }
            }
            ExprKind::Unary { op: UnaryOp::Not, operand } => format!("!{}", self.value(operand)),
            ExprKind::Unary { op: UnaryOp::Neg, operand } => format!("-{}", self.value(operand)),
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right, &ty),
            ExprKind::In { value, options } => {
                let value_ty = self.types.type_of(value);
                let common = if is_decimal(&value_ty) || options.iter().any(|o| is_decimal(&self.types.type_of(o))) {
                    TypeInfo::Decimal
                } else {
                    value_ty.clone()
                };
                let v = coerce(self.value(value), &value_ty, &common);
                let opts: Vec<String> = options
                    .iter()
                    .map(|o| {
                        let t = self.types.type_of(o);
                        coerce(self.value(o), &t, &common)
                    })
                    .collect();
                format!("[{}].contains(&{v})", opts.join(", "))
            }
            ExprKind::Call { function, args } if self.types.name_of(expr) == Some(&NameKind::Handler) => {
                if !args.is_empty() {
                    self.errors.push(format!("handler '{}' takes no arguments", function.name));
                }
                let prefix = self.window.as_ref().map(|w| w.prefix.clone()).unwrap_or_default();
                format!("{prefix}__{}(db, w, ui)?", function.name)
            }
            ExprKind::Call { function, args } => self.call(&function.name, args),
            ExprKind::Find { entity, filter, order, var } => self.find(&var.name, &entity.name, filter.as_deref(), order),
        }
    }

    /// `find c in Entity where ... order by ...` as a database query. The
    /// condition becomes a `Cond` tree; the runtime writes the SQL.
    fn find(&mut self, var: &str, entity: &str, filter: Option<&Expr>, order: &[OrderKey]) -> String {
        let mut query = "Query::new()".to_string();
        if let Some(filter) = filter {
            let cond = self.cond(filter, var, entity);
            query.push_str(&format!(".filter({cond})"));
        }
        for key in order {
            let numeric = self.field_type(entity, &key.field.name).is_some_and(|t| is_stored_number(&t));
            let method = if numeric { "order_number" } else { "order" };
            query.push_str(&format!(".{method}({:?}, {})", self.column(entity, &key.field.name), key.descending));
        }
        format!("db.find::<{entity}>(&{query})?")
    }

    /// The column a field is stored in: its own name, or the existing
    /// column it is mapped to.
    fn column(&self, entity: &str, field: &str) -> String {
        self.project.entity(entity).map_or_else(|| field.to_string(), |e| e.column_of(field).to_string())
    }

    fn field_type(&self, entity: &str, field: &str) -> Option<FieldType> {
        self.project.entity(entity).and_then(|e| e.field(field)).map(|f| f.ty.clone())
    }

    /// A query condition as a `Cond`. The checker made sure each
    /// comparison has a field of `var` on one side and a value on the other.
    fn cond(&mut self, expr: &Expr, var: &str, entity: &str) -> String {
        let field_of = |e: &Expr| match &e.kind {
            ExprKind::Member { object, member } if matches!(&object.kind, ExprKind::Name { name } if name == var) => {
                Some(member.name.clone())
            }
            _ => None,
        };
        match &expr.kind {
            ExprKind::Binary { op: BinaryOp::And, left, right } => {
                format!("{}.and({})", self.cond(left, var, entity), self.cond(right, var, entity))
            }
            ExprKind::Binary { op: BinaryOp::Or, left, right } => {
                format!("{}.or({})", self.cond(left, var, entity), self.cond(right, var, entity))
            }
            ExprKind::Unary { op: UnaryOp::Not, operand } => format!("{}.negated()", self.cond(operand, var, entity)),
            ExprKind::Binary { op, left, right } => {
                // The field on the left: `5 < c.x` is `c.x > 5`.
                let (column, value, op) = match (field_of(left), field_of(right)) {
                    (Some(column), _) => (column, right, *op),
                    (None, Some(column)) => (column, left, flipped(*op)),
                    (None, None) => {
                        self.errors.push(format!("'{}' compares no field of {var}", print::expr(expr)));
                        return String::new();
                    }
                };
                let Some(ty) = self.field_type(entity, &column) else {
                    self.errors.push(format!("{entity} has no field '{column}'"));
                    return String::new();
                };
                if op == BinaryOp::Contains {
                    return format!("Cond::contains({:?}, &{})", self.column(entity, &column), self.text_ref(value));
                }
                let constructor = if is_stored_number(&ty) { "number" } else { "compare" };
                let value = self.sql_value(&ty, value);
                format!("Cond::{constructor}({:?}, Op::{}, {value})", self.column(entity, &column), op_name(op))
            }
            ExprKind::In { value, options } => {
                let Some(column) = field_of(value) else {
                    self.errors.push(format!("'{}' compares no field of {var}", print::expr(expr)));
                    return String::new();
                };
                let Some(ty) = self.field_type(entity, &column) else {
                    self.errors.push(format!("{entity} has no field '{column}'"));
                    return String::new();
                };
                let constructor = if is_stored_number(&ty) { "one_of_numbers" } else { "one_of" };
                let values: Vec<String> = options.iter().map(|o| self.sql_value(&ty, o)).collect();
                format!("Cond::{constructor}({:?}, vec![{}])", self.column(entity, &column), values.join(", "))
            }
            _ => match field_of(expr) {
                Some(column) => format!("Cond::is_true({:?})", self.column(entity, &column)),
                None => {
                    self.errors.push(format!("'{}' is not a condition on {var}", print::expr(expr)));
                    String::new()
                }
            },
        }
    }

    /// Text to borrow: a name or field is read in place, anything else is
    /// computed.
    fn text_ref(&mut self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Name { .. } | ExprKind::Member { .. } if self.part_property(expr).is_none() => self.place(expr),
            _ => self.value(expr),
        }
    }

    /// A value compared with a field of type `ty`, as the database stores it.
    fn sql_value(&mut self, ty: &FieldType, value: &Expr) -> String {
        let from = self.types.type_of(value);
        match ty {
            FieldType::Text { .. } | FieldType::Email | FieldType::Url => format!("sql::text(&{})", self.value(value)),
            FieldType::Integer | FieldType::Id => format!("sql::integer({})", self.value(value)),
            FieldType::Decimal { .. } | FieldType::Money => {
                format!("sql::decimal({})", coerce(self.value(value), &from, &TypeInfo::Decimal))
            }
            FieldType::Bool => format!("sql::boolean({})", self.value(value)),
            FieldType::Date => format!("sql::date({})", self.value(value)),
            FieldType::DateTime => format!("sql::date_time({})", self.value(value)),
            FieldType::Enum { .. } => format!("sql::name(&{})", self.value(value)),
            FieldType::Ref { .. } => {
                // A reference field holds the id already; a record has its own.
                let stored_ref = match &value.kind {
                    ExprKind::Member { object, member } => match self.types.type_of(object) {
                        TypeInfo::Record(owner) => {
                            matches!(self.field_type(&owner, &member.name), Some(FieldType::Ref { .. }))
                                .then(|| format!("{}.{}", self.object(object), ident(&member.name)))
                        }
                        _ => None,
                    },
                    _ => None,
                };
                match stored_ref {
                    Some(id) => format!("sql::reference({id})"),
                    None => format!("sql::reference(Some({}.id))", self.object(value)),
                }
            }
            FieldType::List { .. } => {
                self.errors.push("a list field cannot be compared in a query".into());
                String::new()
            }
        }
    }

    fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr, result: &TypeInfo) -> String {
        let lt = self.types.type_of(left);
        let rt = self.types.type_of(right);
        let symbol = match op {
            BinaryOp::Or => "||",
            BinaryOp::And => "&&",
            other => other.symbol(),
        };
        match op {
            BinaryOp::Contains => format!("contains(&{}, &{})", self.text_ref(left), self.text_ref(right)),
            BinaryOp::Add if *result == TypeInfo::Text => {
                format!("[{}.show(), {}.show()].concat()", self.value(left), self.value(right))
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                let l = coerce(self.value(left), &lt, result);
                let r = coerce(self.value(right), &rt, result);
                format!("({l} {symbol} {r})")
            }
            BinaryOp::And | BinaryOp::Or => format!("({} {symbol} {})", self.value(left), self.value(right)),
            _ => {
                let common = if is_decimal(&lt) || is_decimal(&rt) { TypeInfo::Decimal } else { lt.clone() };
                let l = coerce(self.value(left), &lt, &common);
                let r = coerce(self.value(right), &rt, &common);
                format!("({l} {symbol} {r})")
            }
        }
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> String {
        if name == "count" {
            let list = match args.first() {
                Some(arg) => match self.held(arg) {
                    Some((place, _)) => place,
                    None => format!("({})", self.value(arg)),
                },
                None => String::new(),
            };
            return format!("({list}.len() as i64)");
        }
        let Some(callee) = self.project.files.iter().flat_map(|f| &f.functions).find(|f| f.name.name == name) else {
            self.errors.push(format!("unknown function '{name}'"));
            return String::new();
        };
        let mut code = vec!["db".to_string()];
        for (arg, param) in args.iter().zip(&callee.params) {
            let expected = type_ref_info(self.project, &param.ty);
            let from = self.types.type_of(arg);
            let held = self.held(arg);
            code.push(match (&expected, callee.kind, held) {
                (TypeInfo::Record(_) | TypeInfo::List(_), _, Some((place, true))) => place,
                (TypeInfo::Record(_), FunctionKind::Action, Some((place, false))) => format!("&mut {place}"),
                (TypeInfo::Record(_) | TypeInfo::List(_), _, Some((place, false))) => format!("&{place}"),
                (TypeInfo::Record(_), FunctionKind::Action, None) => format!("&mut {}", self.value(arg)),
                (TypeInfo::Record(_) | TypeInfo::List(_), _, None) => format!("&{}", self.value(arg)),
                _ => coerce(self.value(arg), &from, &expected),
            });
        }
        format!("{}({})?", ident(name), code.join(", "))
    }
}

/// What a handler may read of a part.
#[derive(Debug, Clone)]
pub struct PartInfo {
    pub bind: Option<String>,
    pub caption: Option<String>,
    /// Whether the design makes the part read-only.
    pub readonly: bool,
}

/// The parts of the window or app a handler belongs to, by name.
#[derive(Debug, Default)]
pub struct WindowCtx {
    /// The start of its handlers' Rust names: `order_form` in
    /// `order_form__save_clicked`.
    pub prefix: String,
    pub parts: HashMap<String, PartInfo>,
    /// The window's data: name, entity and whether it is a list.
    pub data: Vec<(String, String, bool)>,
}

impl WindowCtx {
    fn widgets(&mut self, widgets: &[Widget]) {
        for widget in widgets {
            if let Some(name) = &widget.props.name {
                self.parts.insert(
                    name.clone(),
                    PartInfo {
                        bind: widget.props.bind.clone(),
                        caption: widget.props.caption.clone(),
                        readonly: widget.props.readonly,
                    },
                );
            }
            self.widgets(&widget.children);
            for page in &widget.pages {
                self.widgets(&page.children);
            }
        }
    }

    fn commands(&mut self, commands: &[Command]) {
        for command in commands {
            if let Some(name) = &command.props.name {
                self.parts.insert(
                    name.clone(),
                    PartInfo {
                        bind: None,
                        caption: command.props.caption.clone(),
                        readonly: false,
                    },
                );
            }
            self.commands(&command.items);
        }
    }

    fn frame(&mut self, frame: &Frame) {
        if let Some(menu) = &frame.menu {
            self.commands(&menu.items);
        }
        if let Some(toolbar) = &frame.toolbar {
            self.commands(&toolbar.items);
        }
        if let Some(ribbon) = &frame.ribbon {
            for group in ribbon.tabs.iter().flat_map(|t| &t.groups) {
                self.commands(&group.items);
            }
        }
        for sidebar in &frame.sidebars {
            self.widgets(&sidebar.widgets);
        }
        if let Some(footer) = &frame.footer {
            self.widgets(&footer.widgets);
        }
    }
}

/// A window or the app, as `main.rs` dispatches to it.
#[derive(Debug, Clone)]
pub struct Owner {
    pub name: String,
    /// The Rust struct holding its data: `CustomerFormWindow`, `ShopApp`.
    pub struct_name: String,
    pub module: String,
    /// Handlers by Clarity name, with their Rust function names.
    pub handlers: Vec<(String, String)>,
    pub is_app: bool,
    /// Its own events (open, close, return, start) with their handlers.
    pub events: Vec<(&'static str, String)>,
}

/// The struct and handlers of every window and the app in `file`.
pub fn emit_windows(
    project: &Project,
    file: &SpecFile,
    types: &FileTypes,
    module: &str,
) -> Result<(String, Vec<Owner>), Vec<String>> {
    let mut out = String::new();
    let mut owners = Vec::new();
    let mut errors = Vec::new();
    let pages = file.pages.iter().map(|p| (&p.name.name, &p.data, Some(p), None));
    let apps = file.apps.iter().map(|a| (&a.name.name, &a.data, None, Some(a)));
    for (name, data, page, app) in pages.chain(apps) {
        let struct_name = if app.is_some() { format!("{name}App") } else { format!("{name}Window") };
        let mut ctx = WindowCtx {
            prefix: super::names::snake(name),
            ..WindowCtx::default()
        };
        let mut events = Vec::new();
        if let Some(page) = page {
            ctx.widgets(&page.widgets);
            ctx.frame(&page.frame);
            events.extend(page.handlers.iter().map(|h| (h.event.keyword(), h.handler.clone())));
        }
        if let Some(app) = app {
            ctx.frame(&app.frame);
            events.extend(app.handlers.iter().map(|h| (h.event.keyword(), h.handler.clone())));
        }
        ctx.data = data.iter().map(|d| (d.name.name.clone(), d.entity.name.clone(), d.list)).collect();

        let fields: String = data
            .iter()
            .map(|d| {
                let ty = if d.list { format!("Vec<{}>", d.entity.name) } else { d.entity.name.clone() };
                format!("    pub {}: {ty},\n", ident(&d.name.name))
            })
            .collect();
        let loads: String = data
            .iter()
            .map(|d| {
                let value = if d.list {
                    format!("db.all::<{}>()?", d.entity.name)
                } else {
                    format!("{}::default()", d.entity.name)
                };
                format!("            {}: {value},\n", ident(&d.name.name))
            })
            .collect();
        out.push_str(&format!(
            "/// The data of {name}, as its handlers see it.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct {struct_name} {{
{fields}}}

impl {struct_name} {{
    /// The data when it opens: every record of a list, a new record otherwise.
    pub fn load(db: &Db) -> Result<Self> {{
        Ok({struct_name} {{
{loads}        }})
    }}
}}

"
        ));

        let mut handlers = Vec::new();
        for handler in file.code.iter().filter(|b| b.name.name == *name).flat_map(|b| &b.handlers) {
            let function = format!("{}__{}", super::names::snake(name), handler.name.name);
            let mut emitter = Emitter {
                project,
                types,
                decl: None,
                window: Some(std::mem::take(&mut ctx)),
                refs: vec![HashSet::new()],
                errors: Vec::new(),
            };
            let mut body = String::new();
            emitter.block(&handler.body, 1, &mut body);
            ctx = emitter.window.take().unwrap_or_default();
            if emitter.errors.is_empty() {
                out.push_str(&format!(
                    "pub fn {function}(db: &Db, w: &mut {struct_name}, ui: &mut Ui) -> Result<()> {{\n{body}    Ok(())\n}}\n\n"
                ));
                handlers.push((handler.name.name.clone(), function));
            } else {
                errors.extend(emitter.errors.into_iter().map(|e| format!("handler '{}' of {name}: {e}", handler.name.name)));
            }
        }
        owners.push(Owner {
            name: name.clone(),
            struct_name,
            module: module.to_string(),
            handlers,
            is_app: app.is_some(),
            events,
        });
    }
    if errors.is_empty() { Ok((out, owners)) } else { Err(errors) }
}

/// Decimals and money: stored as text, compared and ordered as numbers.
fn is_stored_number(ty: &FieldType) -> bool {
    matches!(ty, FieldType::Decimal { .. } | FieldType::Money)
}

/// The same comparison with its sides swapped.
fn flipped(op: BinaryOp) -> BinaryOp {
    match op {
        BinaryOp::Lt => BinaryOp::Gt,
        BinaryOp::LtEq => BinaryOp::GtEq,
        BinaryOp::Gt => BinaryOp::Lt,
        BinaryOp::GtEq => BinaryOp::LtEq,
        other => other,
    }
}

/// The runtime's name for a comparison.
fn op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Eq => "Eq",
        BinaryOp::NotEq => "NotEq",
        BinaryOp::Lt => "Lt",
        BinaryOp::LtEq => "LtEq",
        BinaryOp::Gt => "Gt",
        _ => "GtEq",
    }
}
