//! Functions, actions and data events as static methods of `Logic`. Types
//! come from the checker's table, so the emitter never decides a type on
//! its own: it only chooses how to write it in C#. C# widens integers to
//! decimals by itself, and records are objects, so a record goes into an
//! action as it is and the action changes it in place.

use clarity_spec::Project;
use clarity_spec::code::check::{FileTypes, NameKind, TypeInfo};
use clarity_spec::code::{
    BinaryOp, Expr, ExprKind, FunctionDecl, OrderKey, Stmt, StmtKind, TextPart, TypeRef, TypeRefKind, UnaryOp,
    path_text, template,
};
use clarity_spec::model::FieldType;

use super::names::{camel, pascal, property, string, value_type};
use crate::print;

/// The methods of `file`'s functions, actions and data events.
pub fn emit(project: &Project, file: &clarity_spec::model::SpecFile, types: &FileTypes) -> Result<String, Vec<String>> {
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
            decl,
            errors: Vec::new(),
        };
        let code = emitter.function();
        if emitter.errors.is_empty() {
            out.push_str(&code);
        } else {
            errors.extend(emitter.errors.into_iter().map(|e| format!("{} '{}': {e}", decl.kind.keyword(), decl.name.name)));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    if out.is_empty() {
        return Ok(out);
    }
    Ok(format!("public static partial class Logic\n{{\n{}}}\n\n", out.trim_end_matches('\n').to_string() + "\n"))
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

fn is_decimal(ty: &TypeInfo) -> bool {
    matches!(ty, TypeInfo::Decimal | TypeInfo::Money)
}

/// Decimals and money: stored as text in SQLite, compared and ordered as numbers.
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

struct Emitter<'a> {
    project: &'a Project,
    types: &'a FileTypes,
    /// The function, action or data event being written.
    decl: &'a FunctionDecl,
    errors: Vec<String>,
}

impl Emitter<'_> {
    fn returns(&self) -> Option<TypeInfo> {
        self.decl.returns.as_ref().map(|r| type_ref_info(self.project, r))
    }

    fn function(&mut self) -> String {
        let decl = self.decl;
        let mut params = vec!["Db db".to_string()];
        for param in &decl.params {
            let info = type_ref_info(self.project, &param.ty);
            params.push(format!("{} {}", value_type(&info), camel(&param.name.name)));
        }
        let ret = self.returns().map(|r| value_type(&r)).unwrap_or_else(|| "void".into());
        let mut out = format!("    public static {ret} {}({})\n    {{\n", pascal(&decl.name.name), params.join(", "));
        for condition in &decl.requires {
            let holds = self.value(condition);
            out.push_str(&format!("        Contracts.Require({holds}, {});\n", string(&print::expr(condition))));
        }
        if decl.ensures.is_empty() {
            self.block(&decl.body, 2, &mut out);
        } else {
            // The body runs as a local function, so every return gives the
            // result the ensures check.
            out.push_str(&format!("        {ret} Body()\n        {{\n"));
            self.block(&decl.body, 3, &mut out);
            out.push_str("        }\n        var result = Body();\n");
            for condition in &decl.ensures {
                let holds = self.value(condition);
                out.push_str(&format!("        Contracts.Ensure({holds}, {});\n", string(&print::expr(condition))));
            }
            out.push_str("        return result;\n");
        }
        out.push_str("    }\n\n");
        out
    }

    fn block(&mut self, body: &[Stmt], depth: usize, out: &mut String) {
        for stmt in body {
            self.stmt(stmt, depth, out);
        }
    }

    /// A block in braces, as C# writes the body of an `if` or `foreach`.
    fn braced(&mut self, body: &[Stmt], depth: usize, out: &mut String) {
        let pad = "    ".repeat(depth);
        out.push_str(&format!("{pad}{{\n"));
        self.block(body, depth + 1, out);
        out.push_str(&format!("{pad}}}\n"));
    }

    fn stmt(&mut self, stmt: &Stmt, depth: usize, out: &mut String) {
        let pad = "    ".repeat(depth);
        match &stmt.kind {
            StmtKind::Let { name, ty, value } => {
                let value = self.value(value);
                let ty = match ty {
                    Some(declared) => value_type(&type_ref_info(self.project, declared)),
                    None => "var".into(),
                };
                out.push_str(&format!("{pad}{ty} {} = {value};\n", camel(&name.name)));
            }
            StmtKind::Assign { target, value } => {
                let place = self.place(target);
                let value = self.value(value);
                out.push_str(&format!("{pad}{place} = {value};\n"));
            }
            StmtKind::If { branches, otherwise } => {
                for (i, branch) in branches.iter().enumerate() {
                    let keyword = if i == 0 { "if" } else { "else if" };
                    let condition = self.value(&branch.condition);
                    out.push_str(&format!("{pad}{keyword} ({condition})\n"));
                    self.braced(&branch.body, depth, out);
                }
                if let Some(body) = otherwise {
                    out.push_str(&format!("{pad}else\n"));
                    self.braced(body, depth, out);
                }
            }
            StmtKind::For { var, list, body } => {
                let list = self.value(list);
                out.push_str(&format!("{pad}foreach (var {} in {list})\n", camel(&var.name)));
                self.braced(body, depth, out);
            }
            StmtKind::Return { value: Some(value) } => {
                let value = self.value(value);
                out.push_str(&format!("{pad}return {value};\n"));
            }
            StmtKind::Return { value: None } => out.push_str(&format!("{pad}return;\n")),
            StmtKind::Call { call } => {
                let code = self.value(call);
                if matches!(self.types.type_of(call), TypeInfo::Unknown) {
                    out.push_str(&format!("{pad}{code};\n"));
                } else {
                    out.push_str(&format!("{pad}_ = {code};\n"));
                }
            }
            StmtKind::Save { value } => {
                let record = self.value(value);
                out.push_str(&format!("{pad}db.Save({record});\n"));
            }
            StmtKind::Delete { value } => {
                let record = self.value(value);
                out.push_str(&format!("{pad}db.Delete({record});\n"));
            }
            StmtKind::Refuse { text } => {
                let text = self.shown(text);
                out.push_str(&format!("{pad}throw new RefusedException({text});\n"));
            }
            StmtKind::Message { .. } | StmtKind::Open { .. } | StmtKind::Close | StmtKind::Reload { .. } => {
                self.errors.push("messages, windows and window data belong in handlers".into());
            }
        }
    }

    /// Text the screen shows (a message, a refusal, a caption, an error) as
    /// a runtime `Text`: the text as written with a typed value per slot,
    /// so the screen fills the slots in the person's format.
    fn shown(&mut self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Str { value } => format!("Text.Plain({})", string(value)),
            ExprKind::Text { parts } => {
                let mut code = format!("new Text({})", string(&template(parts)));
                for part in parts {
                    if let TextPart::Slot { value } = part {
                        let slot = self.slot(value);
                        code.push_str(&format!(".With({}, {slot})", string(&path_text(value))));
                    }
                }
                code
            }
            _ => {
                let value = self.value(expr);
                format!("Text.Plain({})", self.show(value, &self.types.type_of(expr)))
            }
        }
    }

    /// A value as text, the way `"text" + value` shows it.
    fn show(&self, code: String, ty: &TypeInfo) -> String {
        match ty {
            TypeInfo::Text => code,
            TypeInfo::Enum(_) => format!("Labels.Of({code})"),
            _ => format!("Show.Of({code})"),
        }
    }

    /// The value of a slot with what the screen needs to show it: its type,
    /// and the format and decimals of the field it comes from.
    fn slot(&mut self, expr: &Expr) -> String {
        let value = self.value(expr);
        let field = self.field_of(expr);
        let format = match field.and_then(|f| f.modifiers.format.as_deref()) {
            Some(name) => string(name),
            None => "null".to_string(),
        };
        let scale = match field.map(|f| &f.ty) {
            Some(FieldType::Decimal { scale, .. }) => scale.to_string(),
            _ => "null".to_string(),
        };
        match self.types.type_of(expr) {
            TypeInfo::Text => format!("Slot.OfText({value})"),
            TypeInfo::Integer => format!("Slot.OfInteger({value}, {format})"),
            TypeInfo::Decimal => format!("Slot.OfDecimal({value}, {scale}, {format})"),
            TypeInfo::Money => format!("Slot.OfMoney({value}, {format})"),
            TypeInfo::Date => format!("Slot.OfDate({value}, {format})"),
            TypeInfo::DateTime => format!("Slot.OfDateTime({value}, {format})"),
            TypeInfo::Bool => format!("Slot.OfBoolean({value})"),
            TypeInfo::Enum(name) => format!("Slot.OfName({value}, {})", string(&name)),
            other => {
                self.errors.push(format!("a slot cannot show {other:?}"));
                String::new()
            }
        }
    }

    /// The dictionary field a member path such as `order.total` reads.
    fn field_of(&self, expr: &Expr) -> Option<&clarity_spec::model::Field> {
        let ExprKind::Member { object, member } = &expr.kind else { return None };
        let TypeInfo::Record(entity) = self.types.type_of(object) else { return None };
        self.project.entity(&entity)?.field(&member.name)
    }

    fn field_type(&self, entity: &str, field: &str) -> Option<FieldType> {
        self.project.entity(entity).and_then(|e| e.field(field)).map(|f| f.ty.clone())
    }

    /// The column a field is stored in: its own name, or the existing
    /// column it is mapped to.
    fn column(&self, entity: &str, field: &str) -> String {
        self.project.entity(entity).map_or_else(|| field.to_string(), |e| e.column_of(field).to_string())
    }

    /// An expression as a place: what can be assigned.
    fn place(&mut self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Name { name } => match self.types.name_of(expr) {
                Some(NameKind::Local) => camel(name),
                Some(NameKind::EnumValue(e)) => format!("{e}.{name}"),
                _ => {
                    self.errors.push(format!("'{name}' belongs to a window; windows are generated for C# next"));
                    String::new()
                }
            },
            ExprKind::Member { object, member } => {
                if let Some(NameKind::EnumValue(e)) = self.types.name_of(expr) {
                    return format!("{e}.{}", member.name);
                }
                let TypeInfo::Record(entity) = self.types.type_of(object) else {
                    self.errors.push(format!("'{}' belongs to a window; windows are generated for C# next", print::expr(expr)));
                    return String::new();
                };
                let object = self.place(object);
                let prop = property(&entity, &member.name);
                match self.field_type(&entity, &member.name) {
                    Some(FieldType::Ref { entity: target }) => format!("db.Load<{}>({object}.{prop})", target.name),
                    Some(FieldType::List { .. }) => {
                        self.errors.push(format!("list fields like '{}' are not loaded yet", print::expr(expr)));
                        String::new()
                    }
                    _ => format!("{object}.{prop}"),
                }
            }
            _ => self.value(expr),
        }
    }

    /// An expression as a value.
    fn value(&mut self, expr: &Expr) -> String {
        let ty = self.types.type_of(expr);
        match &expr.kind {
            ExprKind::Number { value } if is_decimal(&ty) => format!("{value}m"),
            ExprKind::Number { value } => format!("{value}L"),
            ExprKind::Str { value } => string(value),
            // The checker allows text with slots only where it is shown.
            ExprKind::Text { .. } => {
                self.errors.push("text with slots is text to show".into());
                String::new()
            }
            ExprKind::Bool { value } => value.to_string(),
            ExprKind::Name { .. } | ExprKind::Member { .. } => self.place(expr),
            ExprKind::Unary { op: UnaryOp::Not, operand } => format!("!{}", self.value(operand)),
            ExprKind::Unary { op: UnaryOp::Neg, operand } => format!("-{}", self.value(operand)),
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right, &ty),
            ExprKind::In { value, options } => {
                let value_ty = self.types.type_of(value);
                let common = if is_decimal(&value_ty) || options.iter().any(|o| is_decimal(&self.types.type_of(o))) {
                    TypeInfo::Decimal
                } else {
                    value_ty
                };
                let v = self.value(value);
                let opts: Vec<String> = options.iter().map(|o| self.value(o)).collect();
                format!("new {}[] {{ {} }}.Contains({v})", value_type(&common), opts.join(", "))
            }
            ExprKind::Call { function, .. } if self.types.name_of(expr) == Some(&NameKind::Handler) => {
                self.errors.push(format!("handler '{}' belongs to a window; windows are generated for C# next", function.name));
                String::new()
            }
            ExprKind::Call { function, args } => self.call(&function.name, args),
            ExprKind::Find { entity, filter, order, var } => self.find(&var.name, &entity.name, filter.as_deref(), order),
        }
    }

    fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr, result: &TypeInfo) -> String {
        let symbol = match op {
            BinaryOp::Or => "||",
            BinaryOp::And => "&&",
            other => other.symbol(),
        };
        match op {
            BinaryOp::Contains => format!("Texts.Contains({}, {})", self.value(left), self.value(right)),
            BinaryOp::Add if *result == TypeInfo::Text => {
                let (lt, rt) = (self.types.type_of(left), self.types.type_of(right));
                let l = self.value(left);
                let r = self.value(right);
                format!("({} + {})", self.show(l, &lt), self.show(r, &rt))
            }
            _ => format!("({} {symbol} {})", self.value(left), self.value(right)),
        }
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> String {
        if name == "count" {
            let list = args.first().map(|a| self.value(a)).unwrap_or_default();
            let simple = list.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '@');
            return if simple { format!("(long){list}.Count") } else { format!("(long)({list}).Count") };
        }
        if !self.project.files.iter().flat_map(|f| &f.functions).any(|f| f.name.name == name) {
            self.errors.push(format!("unknown function '{name}'"));
            return String::new();
        }
        let mut code = vec!["db".to_string()];
        code.extend(args.iter().map(|a| self.value(a)));
        format!("{}({})", pascal(name), code.join(", "))
    }

    /// `find c in Entity where ... order by ...` as a database query. The
    /// condition becomes a `Cond` tree; the runtime writes the SQL.
    fn find(&mut self, var: &str, entity: &str, filter: Option<&Expr>, order: &[OrderKey]) -> String {
        let mut query = "new Query()".to_string();
        if let Some(filter) = filter {
            let cond = self.cond(filter, var, entity);
            query.push_str(&format!(".Filter({cond})"));
        }
        for key in order {
            let numeric = self.field_type(entity, &key.field.name).is_some_and(|t| is_stored_number(&t));
            let method = if numeric { "OrderNumber" } else { "Order" };
            query.push_str(&format!(".{method}({}, {})", string(&self.column(entity, &key.field.name)), key.descending));
        }
        format!("db.Find<{entity}>({query})")
    }

    /// A query condition as a `Cond`. The checker made sure each comparison
    /// has a field of `var` on one side and a value on the other.
    fn cond(&mut self, expr: &Expr, var: &str, entity: &str) -> String {
        let field_of = |e: &Expr| match &e.kind {
            ExprKind::Member { object, member } if matches!(&object.kind, ExprKind::Name { name } if name == var) => {
                Some(member.name.clone())
            }
            _ => None,
        };
        match &expr.kind {
            ExprKind::Binary { op: BinaryOp::And, left, right } => {
                format!("{}.And({})", self.cond(left, var, entity), self.cond(right, var, entity))
            }
            ExprKind::Binary { op: BinaryOp::Or, left, right } => {
                format!("{}.Or({})", self.cond(left, var, entity), self.cond(right, var, entity))
            }
            ExprKind::Unary { op: UnaryOp::Not, operand } => format!("{}.Negated()", self.cond(operand, var, entity)),
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
                let name = string(&self.column(entity, &column));
                if op == BinaryOp::Contains {
                    return format!("Cond.Contains({name}, {})", self.value(value));
                }
                let constructor = if is_stored_number(&ty) { "Number" } else { "Compare" };
                let value = self.sql_value(&ty, value);
                format!("Cond.{constructor}({name}, Op.{}, {value})", op_name(op))
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
                let constructor = if is_stored_number(&ty) { "OneOfNumbers" } else { "OneOf" };
                let values: Vec<String> = options.iter().map(|o| self.sql_value(&ty, o)).collect();
                format!("Cond.{constructor}({}, [{}])", string(&self.column(entity, &column)), values.join(", "))
            }
            _ => match field_of(expr) {
                Some(column) => format!("Cond.IsTrue({})", string(&self.column(entity, &column))),
                None => {
                    self.errors.push(format!("'{}' is not a condition on {var}", print::expr(expr)));
                    String::new()
                }
            },
        }
    }

    /// A value compared with a field of type `ty`, as the runtime takes it.
    fn sql_value(&mut self, ty: &FieldType, value: &Expr) -> String {
        let from = self.types.type_of(value);
        match ty {
            FieldType::Decimal { .. } | FieldType::Money if from == TypeInfo::Integer => format!("(decimal){}", self.value(value)),
            FieldType::Enum { .. } => format!("Sql.Name({})", self.value(value)),
            FieldType::Ref { .. } => match from {
                // A reference field holds the id already; a record has its own.
                TypeInfo::Record(_) if self.is_reference_field(value) => self.stored_reference(value),
                TypeInfo::Record(_) => format!("{}.Id", self.value(value)),
                _ => self.value(value),
            },
            FieldType::List { .. } => {
                self.errors.push("a list field cannot be compared in a query".into());
                String::new()
            }
            _ => self.value(value),
        }
    }

    /// Whether `expr` reads a reference field, such as `order.customer`.
    fn is_reference_field(&self, expr: &Expr) -> bool {
        let ExprKind::Member { object, member } = &expr.kind else { return false };
        let TypeInfo::Record(owner) = self.types.type_of(object) else { return false };
        matches!(self.field_type(&owner, &member.name), Some(FieldType::Ref { .. }))
    }

    /// The id a reference field holds, without loading the record.
    fn stored_reference(&mut self, expr: &Expr) -> String {
        let ExprKind::Member { object, member } = &expr.kind else { return String::new() };
        let TypeInfo::Record(owner) = self.types.type_of(object) else { return String::new() };
        format!("{}.{}", self.place(object), property(&owner, &member.name))
    }
}
