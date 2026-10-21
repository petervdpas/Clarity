//! Functions and actions as Rust functions. Types come from the checker's
//! table, so the emitter never decides a type on its own: it only chooses
//! how to write it in Rust (borrowing records, widening integers, loading
//! referenced records through the database).

use std::collections::{HashMap, HashSet};

use clarity_spec::Project;
use clarity_spec::code::check::{FileTypes, NameKind, TypeInfo};
use clarity_spec::code::{
    BinaryOp, Expr, ExprKind, FunctionDecl, FunctionKind, MessageLevel, OrderKey, Stmt, StmtKind, TextPart, TypeRef, TypeRefKind,
    UnaryOp, path_text, template,
};
use clarity_spec::model::{Command, DataKind, FieldType, Frame, SpecFile, Widget};

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
        TypeRefKind::Duration => TypeInfo::Duration,
        TypeRefKind::Decimal => TypeInfo::Decimal,
        TypeRefKind::Money => TypeInfo::Money,
        TypeRefKind::Bool => TypeInfo::Bool,
        TypeRefKind::Date => TypeInfo::Date,
        TypeRefKind::DateTime => TypeInfo::DateTime,
        TypeRefKind::File => TypeInfo::File,
        TypeRefKind::Bytes => TypeInfo::Bytes,
        TypeRefKind::List { item } if item == "file" => TypeInfo::Files,
        TypeRefKind::Named { name } if project.entity(name).is_some() => TypeInfo::Record(name.clone()),
        TypeRefKind::Named { name } => TypeInfo::Enum(name.clone()),
        TypeRefKind::List { item } => TypeInfo::List(item.clone()),
    }
}

/// Values of these types are copied; the others are cloned when read.
fn is_copy(ty: &TypeInfo) -> bool {
    !matches!(ty, TypeInfo::Text | TypeInfo::Record(_) | TypeInfo::List(_) | TypeInfo::Image | TypeInfo::File | TypeInfo::Files | TypeInfo::Bytes)
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

impl<'a> Emitter<'a> {
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
                    let value = if matches!(property, "caption" | "error") { self.shown(value) } else { coerce(self.value(value), &from, &to) };
                    out.push_str(&format!("{pad}ui.set_{property}({part:?}, {value});\n"));
                    return;
                }
                // A record given to a reference field is stored as its id.
                if let ExprKind::Member { object, member } = &target.kind
                    && let TypeInfo::Record(owner) = self.types.type_of(object)
                    && matches!(self.project.entity(&owner).and_then(|e| e.field(&member.name)).map(|f| &f.ty), Some(FieldType::Ref { .. }))
                {
                    let object = self.object(object);
                    let stored = if matches!(value.kind, ExprKind::Empty) { "None".to_string() } else { format!("Some({}.id)", self.object(value)) };
                    out.push_str(&format!("{pad}{object}.{} = {stored};\n", ident(&member.name)));
                    return;
                }
                // A file as a picture: linked in the field's own store, else copied.
                if from == TypeInfo::File
                    && let ExprKind::Member { object, member } = &target.kind
                    && let TypeInfo::Record(owner) = self.types.type_of(object)
                {
                    let store = self.project.entity(&owner).and_then(|e| e.field(&member.name)).and_then(|f| f.modifiers.store.clone());
                    let store = store.map_or("None".to_string(), |s| format!("Some({s:?})"));
                    let place = self.place(target);
                    let file = self.object(value);
                    out.push_str(&format!("{pad}{place} = Some(db.picture_from(&{file}, {store})?);\n"));
                    return;
                }
                let place = self.place(target);
                let value = coerce(self.value(value), &from, &to);
                out.push_str(&format!("{pad}{place} = {value};\n"));
                if let ExprKind::Name { name } = &target.kind {
                    out.push_str(&self.deselect(name, &pad));
                }
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
            StmtKind::Write { text, store, path } => {
                // Bytes are written as they are, text as UTF-8.
                let write = if self.types.type_of(text) == TypeInfo::Bytes { "write_bytes" } else { "write_text" };
                let path = self.text_ref(path);
                let text = self.text_ref(text);
                out.push_str(&format!("{pad}db.{write}({:?}, &{path}, &{text})?;\n", store.name));
            }
            StmtKind::For { var, list, body } => {
                let source = match self.held(list) {
                    Some((place, _)) => place,
                    None => format!("({})", self.value(list)),
                };
                // A body that changes, saves or deletes its record works on the
                // record in the list, as on the other stacks.
                let each = if changes(body, &var.name) { "iter_mut" } else { "iter" };
                out.push_str(&format!("{pad}for {} in {source}.{each}() {{\n", ident(&var.name)));
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
                let record = self.record_mut(value);
                out.push_str(&format!("{pad}db.save({record})?;\n"));
            }
            StmtKind::Delete { value } => {
                let record = self.record_mut(value);
                out.push_str(&format!("{pad}db.delete({record})?;\n"));
            }
            StmtKind::Refuse { text } => {
                let text = self.shown(text);
                out.push_str(&format!("{pad}return Err(Error::Refused({text}));\n"));
            }
            StmtKind::Message { .. } | StmtKind::Open { .. } | StmtKind::Close if self.window.is_none() => {
                self.errors.push("messages and windows belong in handlers".into());
            }
            // A shape on a canvas, as conformance/drawing.json sends it.
            StmtKind::Draw { canvas, shape, args } => {
                use clarity_spec::code::check::ShapeValue;
                let mut fields = vec![format!("\"op\": {:?}", shape.name)];
                for (key, kind, given) in crate::shape_values(&shape.name, args) {
                    let value = match (kind, given) {
                        (ShapeValue::Number, Ok(arg)) => match self.types.type_of(arg) {
                            TypeInfo::Decimal | TypeInfo::Money => format!("clarity_runtime::units({})", self.value(arg)),
                            _ => format!("({}) as f64", self.value(arg)),
                        },
                        (ShapeValue::Number, Err(default)) => format!("{default}.0"),
                        (ShapeValue::Color, Ok(arg)) => self.value(arg),
                        (ShapeValue::Color, Err(default)) => format!("{default:?}"),
                        (ShapeValue::Text, Ok(arg)) => format!("serde_json::to_value({}).unwrap_or_default()", self.shown(arg)),
                        (ShapeValue::Text, Err(default)) => format!("{default:?}"),
                    };
                    fields.push(format!("\"{key}\": {value}"));
                }
                out.push_str(&format!("{pad}ui.draw({:?}, serde_json::json!({{{}}}));\n", canvas.name, fields.join(", ")));
            }
            StmtKind::Message { level, text } => {
                let level = match level {
                    MessageLevel::Info => "Info",
                    MessageLevel::Warning => "Warning",
                    MessageLevel::Error => "Error",
                };
                let text = self.shown(text);
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
                        out.push_str(&self.deselect(&name, &pad));
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

    /// Text the screen shows (a message, a refusal, a caption, an error) as
    /// a runtime `Text`: the text as written with a typed value per slot,
    /// so the screen fills the slots in the person's format.
    fn shown(&mut self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Str { value } => format!("Text::plain({value:?})"),
            ExprKind::Text { parts } => {
                let mut code = format!("Text::new({:?})", template(parts));
                for part in parts {
                    if let TextPart::Slot { value } = part {
                        let slot = self.slot(value);
                        code.push_str(&format!(".with({:?}, {slot})", path_text(value)));
                    }
                }
                code
            }
            _ if self.types.type_of(expr) == TypeInfo::Text => format!("Text::plain({})", self.value(expr)),
            _ => format!("Text::plain({}.show())", self.value(expr)),
        }
    }

    /// The value of a slot with what the screen needs to show it: its type,
    /// and the format and decimals of the field it comes from.
    fn slot(&mut self, expr: &Expr) -> String {
        let value = self.value(expr);
        let field = self.field_of(expr);
        let format = match field.and_then(|f| f.modifiers.format.as_deref()) {
            Some(name) => format!("Some({name:?})"),
            None => "None".to_string(),
        };
        let scale = match field.map(|f| &f.ty) {
            Some(FieldType::Decimal { scale, .. }) => format!("Some({scale})"),
            _ => "None".to_string(),
        };
        match self.types.type_of(expr) {
            TypeInfo::Text => format!("Slot::text({value})"),
            TypeInfo::Integer => format!("Slot::integer({value}, {format})"),
            TypeInfo::Duration => format!("Slot::duration({value})"),
            TypeInfo::Decimal => format!("Slot::decimal({value}, {scale}, {format})"),
            TypeInfo::Money => format!("Slot::money({value}, {format})"),
            TypeInfo::Date => format!("Slot::date({value}, {format})"),
            TypeInfo::DateTime => format!("Slot::date_time({value}, {format})"),
            TypeInfo::Bool => format!("Slot::boolean({value})"),
            TypeInfo::Enum(name) => format!("Slot::name(&{value}, {name:?})"),
            other => {
                self.errors.push(format!("a slot cannot show {other:?}"));
                String::new()
            }
        }
    }

    /// The dictionary field a member path such as `order.total` reads.
    fn field_of(&self, expr: &Expr) -> Option<&'a clarity_spec::model::Field> {
        let ExprKind::Member { object, member } = &expr.kind else { return None };
        let TypeInfo::Record(entity) = self.types.type_of(object) else { return None };
        self.project.entity(&entity)?.field(&member.name)
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
            "checked" => "checked",
            "error" => "error",
            "readonly" => "readonly",
            "folded" => "folded",
            "collapsed" => "collapsed",
            "maximized" => "maximized",
            "tab" => "tab",
            "selected" => "selected",
            "x" => "x",
            "y" => "y",
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
            ("folded", Some(info)) => format!("ui.folded({part:?}, {})", info.folded),
            ("collapsed", Some(info)) => format!("ui.collapsed({part:?}, {})", info.collapsed),
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

    /// The record `save` or `delete` works on: a name holding it, or the
    /// record a value is (a table's selected row, as the other stacks save
    /// it), which lives until the statement ends.
    fn record_mut(&mut self, value: &Expr) -> String {
        match self.held(value) {
            Some((place, true)) => place,
            Some((place, false)) => format!("&mut {place}"),
            None => format!("&mut {}", self.value(value)),
        }
    }

    /// After a handler replaced the rows of `data` (`reload`, or a new
    /// list): the tables and lists showing them lose their selected row.
    fn deselect(&self, data: &str, pad: &str) -> String {
        let parts = self.window.as_ref().and_then(|w| w.showing.get(data)).into_iter().flatten();
        parts.map(|part| format!("{pad}ui.deselect({part:?});\n")).collect()
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
            // A part property inside a path: `grid.selected` in `grid.selected.number`.
            ExprKind::Member { .. } if self.part_property(expr).is_some() => {
                let (part, property) = self.part_property(expr).unwrap_or_default();
                self.read_part(&part, property)
            }
            ExprKind::Member { object, member } => {
                if let Some(NameKind::EnumValue(e)) = self.types.name_of(expr) {
                    return format!("{e}::{}", member.name);
                }
                // A file's details have the same names in the runtime.
                if self.types.type_of(object) == TypeInfo::File {
                    let object = self.object(object);
                    return format!("{object}.{}", member.name);
                }
                if self.types.type_of(object) == TypeInfo::Bytes {
                    let object = self.object(object);
                    return format!("({object}.len() as i64)");
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
            // The checker allows text with slots only where it is shown.
            ExprKind::Text { .. } => {
                self.errors.push("text with slots is text to show".into());
                String::new()
            }
            ExprKind::Bool { value } => value.to_string(),
            ExprKind::Empty => "None".to_string(),
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
                let prefix = self.window.as_ref().map(|w| w.prefix.clone()).unwrap_or_default();
                let params = self.window.as_ref().and_then(|w| w.params.get(&function.name).cloned()).unwrap_or_default();
                let mut code = String::new();
                for (arg, param) in args.iter().zip(&params) {
                    let expected = type_ref_info(self.project, param);
                    let from = self.types.type_of(arg);
                    code.push_str(&format!(", {}", coerce(self.value(arg), &from, &expected)));
                }
                format!("{prefix}__{}(db, w, ui{code})?", super::names::snake(&function.name))
            }
            ExprKind::Call { function, args } => self.call(&function.name, args),
            // A function of an engine: values to JSON and back (DESIGN 5.11).
            ExprKind::EngineCall { engine, function, args } => {
                let returns = self
                    .project
                    .files
                    .iter()
                    .flat_map(|f| &f.engines)
                    .find(|e| e.name.name == engine.name)
                    .and_then(|e| e.functions.iter().find(|f| f.name.name == function.name))
                    .and_then(|f| f.returns.as_ref())
                    .map(|r| value_type(&type_ref_info(self.project, r)))
                    .unwrap_or_else(|| "serde_json::Value".into());
                // Bytes cross as base64 text; other values as serde writes them.
                let values: Vec<String> = args
                    .iter()
                    .map(|a| match self.types.type_of(a) {
                        TypeInfo::Bytes => format!("clarity_runtime::engine::bytes_value(&{})", self.value(a)),
                        _ => format!("clarity_runtime::engine::to_value(&{})?", self.value(a)),
                    })
                    .collect();
                let from = if returns == "Vec<u8>" { "bytes_from".to_string() } else { format!("from_value::<{returns}>") };
                // An engine with state per window is called in the handler's own window.
                let per_window = self.project.files.iter().flat_map(|f| &f.engines).any(|e| e.name.name == engine.name && e.state == clarity_spec::code::EngineState::Window);
                let call = if per_window { "call_in(ui.instance(), " } else { "call(" };
                format!(
                    "clarity_runtime::engine::{from}({e:?}, {f:?}, crate::engines::{}()?.{call}{f:?}, vec![{}])?)?",
                    super::names::snake(&engine.name),
                    values.join(", "),
                    e = engine.name,
                    f = function.name
                )
            }
            ExprKind::Find { entity, filter, order, var } => self.find(&var.name, &entity.name, filter.as_deref(), order),
            ExprKind::Files { var, store, filter, order } => self.files(&var.name, &store.name, filter.as_deref(), order),
            ExprKind::ReadText { file } => format!("db.read_text(&{})?", self.object(file)),
            ExprKind::ReadBytes { file } => format!("db.read_bytes(&{})?", self.object(file)),
            ExprKind::New { entity } => format!("{}::default()", entity.name),
        }
    }

    /// `files f in store where ... order by ...`: the files of a store,
    /// filtered and sorted in memory.
    fn files(&mut self, var: &str, store: &str, filter: Option<&Expr>, order: &[OrderKey]) -> String {
        let mut list = format!("db.files({store:?})?.into_iter()");
        if let Some(filter) = filter {
            self.refs.push(HashSet::from([var.to_string()]));
            let cond = self.value(filter);
            self.refs.pop();
            list.push_str(&format!(".filter(|{}| {cond})", ident(var)));
        }
        list.push_str(".collect::<Vec<FileEntry>>()");
        if order.is_empty() {
            return list;
        }
        let keys: Vec<String> = order
            .iter()
            .map(|k| {
                let (a, b) = if k.descending { ("b", "a") } else { ("a", "b") };
                format!("{a}.{m}.cmp(&{b}.{m})", m = k.field.name)
            })
            .collect();
        let mut compare = keys[0].clone();
        for key in &keys[1..] {
            compare = format!("{compare}.then_with(|| {key})");
        }
        format!("{{ let mut found = {list}; found.sort_by(|a, b| {compare}); found }}")
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
            let Some(path) = crate::query::order_path(self.project, entity, key) else {
                self.errors.push(format!("{entity} cannot be ordered by '{}'", key.field.name));
                continue;
            };
            let numeric = self.field_type(&path.entity, &path.field).is_some_and(|t| is_stored_number(&t));
            let column = self.column(&path.entity, &path.field);
            if path.hops.is_empty() {
                let method = if numeric { "order_number" } else { "order" };
                query.push_str(&format!(".{method}({column:?}, {})", key.descending));
            } else {
                let hops: Vec<String> = path.hops.iter().map(|h| format!("Hop::new({:?}, {:?}, {:?})", h.column, h.table, h.key)).collect();
                query.push_str(&format!(".order_through(&[{}], {column:?}, {}, {numeric})", hops.join(", "), key.descending));
            }
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
        let project = self.project;
        let path_of = |e: &Expr| crate::query::field_path(project, entity, var, e);
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
                let (path, value, op) = match (path_of(left), path_of(right)) {
                    (Some(path), _) => (path, right, *op),
                    (None, Some(path)) => (path, left, flipped(*op)),
                    (None, None) => {
                        self.errors.push(format!("'{}' compares no field of {var}", print::expr(expr)));
                        return String::new();
                    }
                };
                let Some(ty) = self.field_type(&path.entity, &path.field) else {
                    self.errors.push(format!("{} has no field '{}'", path.entity, path.field));
                    return String::new();
                };
                let column = self.column(&path.entity, &path.field);
                let own = if matches!(value.kind, ExprKind::Empty) {
                    let test = format!("Cond::is_none({column:?})");
                    if op == BinaryOp::Eq { test } else { format!("{test}.negated()") }
                } else if op == BinaryOp::Contains {
                    format!("Cond::contains({column:?}, &{})", self.text_ref(value))
                } else {
                    let constructor = if is_stored_number(&ty) { "number" } else { "compare" };
                    let value = self.sql_value(&ty, value);
                    format!("Cond::{constructor}({column:?}, Op::{}, {value})", op_name(op))
                };
                through(&path.hops, own)
            }
            ExprKind::In { value, options } => {
                let Some(path) = path_of(value) else {
                    self.errors.push(format!("'{}' compares no field of {var}", print::expr(expr)));
                    return String::new();
                };
                let Some(ty) = self.field_type(&path.entity, &path.field) else {
                    self.errors.push(format!("{} has no field '{}'", path.entity, path.field));
                    return String::new();
                };
                let constructor = if is_stored_number(&ty) { "one_of_numbers" } else { "one_of" };
                let values: Vec<String> = options.iter().map(|o| self.sql_value(&ty, o)).collect();
                let own = format!("Cond::{constructor}({:?}, vec![{}])", self.column(&path.entity, &path.field), values.join(", "));
                through(&path.hops, own)
            }
            _ => match path_of(expr) {
                Some(path) => through(&path.hops, format!("Cond::is_true({:?})", self.column(&path.entity, &path.field))),
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
            FieldType::Integer | FieldType::Id | FieldType::Duration => format!("sql::integer({})", self.value(value)),
            FieldType::Decimal { .. } | FieldType::Money => {
                format!("sql::decimal({})", coerce(self.value(value), &from, &TypeInfo::Decimal))
            }
            FieldType::Bool => format!("sql::boolean({})", self.value(value)),
            FieldType::Date => format!("sql::date({})", self.value(value)),
            FieldType::DateTime => format!("sql::date_time({})", self.value(value)),
            FieldType::Enum { .. } => format!("sql::name(&{})", self.value(value)),
            // The checker allows only `== none` and `!= none`, handled before.
            FieldType::Image => {
                self.errors.push("a picture is compared with none only".into());
                String::new()
            }
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

    /// Whether a date or reference field is empty (`x == none`); a record
    /// that is no field is none until it is stored.
    fn is_none(&mut self, expr: &Expr) -> String {
        if let ExprKind::Member { object, member } = &expr.kind
            && let TypeInfo::Record(owner) = self.types.type_of(object)
            && let Some(ty) = self.field_type(&owner, &member.name)
            && matches!(ty, FieldType::Ref { .. } | FieldType::Date | FieldType::DateTime)
        {
            return format!("{}.{}.is_none()", self.object(object), ident(&member.name));
        }
        if matches!(self.types.type_of(expr), TypeInfo::Record(_)) {
            return format!("({}.id == 0)", self.object(expr));
        }
        format!("{}.is_none()", self.place(expr))
    }

    fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr, result: &TypeInfo) -> String {
        if matches!(op, BinaryOp::Eq | BinaryOp::NotEq) {
            let other = match (&left.kind, &right.kind) {
                (_, ExprKind::Empty) => Some(left),
                (ExprKind::Empty, _) => Some(right),
                _ => None,
            };
            if let Some(other) = other {
                let test = self.is_none(other);
                return if op == BinaryOp::Eq { test } else { format!("!{test}") };
            }
        }
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
        use clarity_spec::code::check::{NOW, TODAY};
        // Dates are optional values in records and locals.
        if name == TODAY || name == NOW {
            return format!("Some(clarity_runtime::{name}())");
        }
        // round(value, places), as every runtime rounds (conformance/rounding.json).
        // sin(degrees), cos(degrees), as every runtime gives them (conformance/drawing.json).
        if name == clarity_spec::code::check::SIN || name == clarity_spec::code::check::COS {
            let arg = args.first();
            let value = match arg.map(|a| self.types.type_of(a)) {
                Some(TypeInfo::Decimal | TypeInfo::Money) => self.value(arg.unwrap()),
                Some(_) => format!("Decimal::from({})", self.value(arg.unwrap())),
                None => String::new(),
            };
            return format!("clarity_runtime::{name}({value})");
        }
        if name == clarity_spec::code::check::ROUND {
            let values: Vec<String> = args.iter().map(|a| self.value(a)).collect();
            return format!("round({})", values.join(", "));
        }
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
#[derive(Debug, Clone, Default)]
pub struct PartInfo {
    pub bind: Option<String>,
    pub caption: Option<String>,
    /// Whether the design makes the part read-only.
    pub readonly: bool,
    /// Whether the design starts a foldable part folded.
    pub folded: bool,
    /// Whether the design starts a collapsible sidebar folded to its button.
    pub collapsed: bool,
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
    /// The named tables and lists showing each list data (`crate::rows_shown`).
    pub showing: std::collections::BTreeMap<String, Vec<String>>,
    /// The parameter types of each handler that takes values, by name.
    pub params: HashMap<String, Vec<TypeRef>>,
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
                        folded: widget.props.folded,
                        collapsed: false,
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
                        folded: false,
                        collapsed: false,
                    },
                );
            }
            self.commands(&command.items);
        }
    }

    fn frame(&mut self, frame: &Frame) {
        if let Some(menu) = &frame.menu {
            if let Some(name) = &menu.name {
                self.parts.insert(name.clone(), PartInfo::default());
            }
            self.commands(&menu.items);
        }
        if let Some(toolbar) = &frame.toolbar {
            if let Some(name) = &toolbar.name {
                self.parts.insert(name.clone(), PartInfo::default());
            }
            self.commands(&toolbar.items);
        }
        if let Some(ribbon) = &frame.ribbon {
            if let Some(name) = &ribbon.name {
                self.parts.insert(name.clone(), PartInfo { collapsed: ribbon.collapsed, ..PartInfo::default() });
            }
            for tab in &ribbon.tabs {
                for name in tab.name.iter().chain(tab.groups.iter().filter_map(|g| g.name.as_ref())) {
                    self.parts.insert(name.clone(), PartInfo::default());
                }
            }
            for group in ribbon.tabs.iter().flat_map(|t| &t.groups) {
                self.commands(&group.items);
            }
        }
        for sidebar in &frame.sidebars {
            if let Some(name) = &sidebar.props.name {
                self.parts.insert(name.clone(), PartInfo { collapsed: sidebar.props.collapsed, ..PartInfo::default() });
            }
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
    /// The handlers called by name (those taking no values), with their Rust function names.
    pub handlers: Vec<(String, String)>,
    /// The calls with values its parts wire: as written (`digit(7)`), the
    /// Rust function, and the values as Rust.
    pub calls: Vec<(String, String, Vec<String>)>,
    pub is_app: bool,
    /// Its own events (open, close, return, start) with their handlers.
    pub events: Vec<(&'static str, String)>,
    /// The canvases its draw handlers draw, with their Rust functions.
    pub draws: Vec<(String, String)>,
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
        ctx.data = data.iter().filter_map(|d| d.entity().map(|e| (d.name.name.clone(), e.name.clone(), d.is_list()))).collect();
        ctx.showing = crate::rows_shown(page);
        let written: Vec<&clarity_spec::code::HandlerDecl> = file.code.iter().filter(|b| b.name.name == *name).flat_map(|b| &b.handlers).collect();
        ctx.params = written.iter().map(|h| (h.name.name.clone(), h.params.iter().map(|p| p.ty.clone()).collect())).collect();
        // Each call with values the parts wire, with its values as Rust.
        let calls: Vec<(String, String, Vec<String>)> = page
            .map(crate::wired_calls)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|h| {
                let decl = written.iter().find(|d| d.name.name == h.handler)?;
                let values = h.args.iter().zip(&decl.params).map(|(v, p)| literal_in_main(project, &p.ty, v)).collect();
                Some((clarity_spec::widgets::handler_call(h), format!("{}__{}", super::names::snake(name), super::names::snake(&h.handler)), values))
            })
            .collect();

        let fields: String = data
            .iter()
            .map(|d| {
                let ty = match &d.of {
                    DataKind::Record { entity } => entity.name.clone(),
                    DataKind::List { entity } => format!("Vec<{}>", entity.name),
                    DataKind::Value { ty, .. } => value_type(&type_ref_info(project, ty)),
                };
                format!("    pub {}: {ty},\n", ident(&d.name.name))
            })
            .collect();
        let loads: String = data
            .iter()
            .map(|d| {
                let value = match &d.of {
                    DataKind::Record { entity } => format!("{}::default()", entity.name),
                    DataKind::List { entity } => format!("db.all::<{}>()?", entity.name),
                    DataKind::Value { ty, default: Some(value) } => literal(project, ty, value),
                    DataKind::Value { default: None, .. } => "Default::default()".into(),
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
    /// The data when it opens: every record of a list, a new record otherwise, each value at its default.
    pub fn load(db: &Db) -> Result<Self> {{
        Ok({struct_name} {{
{loads}        }})
    }}
}}

"
        ));

        let mut handlers = Vec::new();
        for handler in file.code.iter().filter(|b| b.name.name == *name).flat_map(|b| &b.handlers) {
            let function = format!("{}__{}", super::names::snake(name), super::names::snake(&handler.name.name));
            // The values a click gives it, typed.
            let values: String = handler
                .params
                .iter()
                .map(|p| format!(", {}: {}", ident(&p.name.name), value_type(&type_ref_info(project, &p.ty))))
                .collect();
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
                    "pub fn {function}(db: &Db, w: &mut {struct_name}, ui: &mut Ui{values}) -> Result<()> {{\n{body}    Ok(())\n}}\n\n"
                ));
                // One that takes values is called with them, through the calls.
                if handler.params.is_empty() {
                    handlers.push((handler.name.name.clone(), function));
                }
            } else {
                errors.extend(emitter.errors.into_iter().map(|e| format!("handler '{}' of {name}: {e}", handler.name.name)));
            }
        }
        let draws = page
            .map(crate::canvas_draws)
            .unwrap_or_default()
            .into_iter()
            .map(|(canvas, handler)| (canvas, format!("{}__{}", super::names::snake(name), super::names::snake(&handler))))
            .collect();
        owners.push(Owner {
            name: name.clone(),
            struct_name,
            module: module.to_string(),
            handlers,
            calls,
            is_app: app.is_some(),
            events,
            draws,
        });
    }
    if errors.is_empty() { Ok((out, owners)) } else { Err(errors) }
}

/// A value written in the design as Rust, of the parameter's type.
fn literal(project: &Project, ty: &TypeRef, value: &clarity_spec::model::Literal) -> String {
    use clarity_spec::model::Literal;
    match (type_ref_info(project, ty), value) {
        (TypeInfo::Decimal | TypeInfo::Money, Literal::Number(n)) => decimal(n),
        (_, Literal::Number(n)) => format!("{n}i64"),
        (_, Literal::Str(text)) => format!("String::from({text:?})"),
        (_, Literal::Bool(b)) => b.to_string(),
        (TypeInfo::Enum(e), Literal::Ident(v)) => format!("{e}::{v}"),
        (_, Literal::Ident(v)) => v.clone(),
    }
}

/// A value written in the design as main.rs writes it, which reaches the
/// modules' types by their paths: `modules::sales::OrderStatus::Approved`.
fn literal_in_main(project: &Project, ty: &TypeRef, value: &clarity_spec::model::Literal) -> String {
    use clarity_spec::model::Literal;
    match (type_ref_info(project, ty), value) {
        (TypeInfo::Decimal | TypeInfo::Money, Literal::Number(n)) => format!("clarity_runtime::{}", decimal(n)),
        (TypeInfo::Enum(e), Literal::Ident(v)) => {
            let module = project.files.iter().find(|f| f.enums.iter().any(|d| d.name.name == e)).and_then(|f| f.module.as_ref());
            match module {
                Some(m) => format!("modules::{}::{e}::{v}", super::names::snake(&m.name)),
                None => format!("{e}::{v}"),
            }
        }
        _ => literal(project, ty, value),
    }
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

/// A condition on the record the steps lead to, as one on the query's own
/// record: each step a `Cond::through`, the first one outermost.
fn through(hops: &[crate::query::Hop], cond: String) -> String {
    hops.iter()
        .rev()
        .fold(cond, |inner, h| format!("Cond::through({:?}, {:?}, {:?}, {inner})", h.column, h.table, h.key))
}

/// Whether `body` sets a field of `var`, or saves or deletes it.
fn changes(body: &[Stmt], var: &str) -> bool {
    fn root(expr: &Expr) -> Option<&str> {
        match &expr.kind {
            ExprKind::Name { name } => Some(name),
            ExprKind::Member { object, .. } => root(object),
            _ => None,
        }
    }
    body.iter().any(|stmt| match &stmt.kind {
        StmtKind::Assign { target, .. } => matches!(&target.kind, ExprKind::Member { .. }) && root(target) == Some(var),
        StmtKind::Save { value } | StmtKind::Delete { value } => root(value) == Some(var),
        StmtKind::If { branches, otherwise } => {
            branches.iter().any(|b| changes(&b.body, var)) || otherwise.as_deref().is_some_and(|o| changes(o, var))
        }
        // An inner loop of its own name hides this one.
        StmtKind::For { var: inner, body, .. } => inner.name != var && changes(body, var),
        _ => false,
    })
}
