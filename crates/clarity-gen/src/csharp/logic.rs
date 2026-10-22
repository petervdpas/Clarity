//! Functions, actions and data events as static methods of `Logic`, and
//! handlers as methods of a window's ViewModel. Types come from the
//! checker's table, so the emitter never decides a type on its own: it only
//! chooses how to write it in C#. C# widens integers to decimals by itself,
//! and records are objects, so a record goes into an action as it is and the
//! action changes it in place.

use std::collections::HashMap;

use clarity_spec::Project;
use clarity_spec::code::check::{FileTypes, NameKind, TypeInfo};
use clarity_spec::code::{
    AskForm, BinaryOp, EngineBody, EngineDecl, EngineState, Expr, ExprKind, FunctionDecl, HandlerDecl, MessageLevel, OrderKey, Stmt, StmtKind, TextPart, TypeRef,
    UnaryOp, path_text, template,
};
use clarity_spec::model::FieldType;

use super::names::{camel, function as function_name, member, pascal, property, string, value_type};
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
        let mut emitter = Emitter::new(project, types, Some(decl), None);
        let code = emitter.function();
        if emitter.errors.is_empty() {
            out.push_str(&code);
        } else {
            errors.extend(emitter.errors.into_iter().map(|e| format!("{} '{}': {e}", decl.kind.keyword(), decl.name.name)));
        }
    }
    let mut engines = String::new();
    for engine in &file.engines {
        if let Some(body) = &engine.body {
            match emit_engine(project, types, engine, body) {
                Ok(code) => engines.push_str(&code),
                Err(mut problems) => errors.append(&mut problems),
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    if out.is_empty() {
        return Ok(engines);
    }
    Ok(format!("public static partial class Logic\n{{\n{}}}\n\n{engines}", out.trim_end_matches('\n').to_string() + "\n"))
}

/// The C# class of an engine written in Clarity: `BoardEngine`.
pub(crate) fn engine_class(engine: &str) -> String {
    format!("{}Engine", pascal(engine))
}

/// An engine written in Clarity as a static class: what it keeps, the kept
/// values of each instance (a window, or 0 for the app), and per function
/// an entry that finds the instance's values and an inner one that works on
/// them. A call works on a copy that is kept only when the call succeeds,
/// so a call that fails or refuses changes nothing.
fn emit_engine(project: &Project, types: &FileTypes, engine: &EngineDecl, body: &EngineBody) -> Result<String, Vec<String>> {
    let name = &engine.name.name;
    let class = engine_class(name);
    let mut errors = Vec::new();
    let mut fields = String::new();
    let mut starts = Vec::new();
    let mut copies = Vec::new();
    for keep in &body.keeps {
        let info = type_ref_info(project, &keep.ty);
        let field = pascal(&keep.name.name);
        let ty = value_type(&info);
        fields.push_str(&format!("        public {ty} {field} = default!;\n"));
        let mut emitter = Emitter::new(project, types, None, None);
        emitter.engine = Some(engine);
        let value = emitter.value(&keep.value);
        if emitter.errors.is_empty() {
            starts.push(format!("{field} = {value}"));
        } else {
            errors.extend(emitter.errors.into_iter().map(|e| format!("engine {name} keeps {}: {e}", keep.name.name)));
        }
        copies.push(if matches!(info, TypeInfo::Values(_)) { format!("{field} = new {ty}({field})") } else { format!("{field} = {field}") });
    }
    let stored = engine.state != EngineState::None;
    let mut out = format!(
        "/// <summary>Engine {name}, written in Clarity.</summary>\npublic static class {class}\n{{\n    /// <summary>What the engine keeps between calls.</summary>\n    public sealed class Kept\n    {{\n{fields}\n        /// <summary>What a new instance starts with.</summary>\n        public static Kept New(Db db) => new() {{ {} }};\n\n        public Kept Copy() => new() {{ {} }};\n    }}\n\n",
        starts.join(", "),
        copies.join(", ")
    );
    if stored {
        out.push_str(
            "    static readonly Dictionary<long, Kept> All = new();\n\n    // A closed window's values go.\n    static ",
        );
        out.push_str(&format!("{class}() => OpenWindows.Closed += Close;\n\n"));
        out.push_str("    /// <summary>Drops what instance <paramref name=\"instance\"/> (a window, or 0 for the app) kept.</summary>\n    public static void Close(long instance)\n    {\n        lock (All)\n            All.Remove(instance);\n    }\n\n");
    }
    for decl in &body.functions {
        let mut emitter = Emitter::new(project, types, Some(decl), None);
        emitter.engine = Some(engine);
        let inner = emitter.function();
        if !emitter.errors.is_empty() {
            errors.extend(emitter.errors.into_iter().map(|e| format!("function '{}' of engine {name}: {e}", decl.name.name)));
            continue;
        }
        let function = pascal(&decl.name.name);
        let params: String = decl.params.iter().map(|p| format!(", {} {}", value_type(&type_ref_info(project, &p.ty)), camel(&p.name.name))).collect();
        let names: String = decl.params.iter().map(|p| format!(", {}", camel(&p.name.name))).collect();
        let ret = decl.returns.as_ref().map(|r| value_type(&type_ref_info(project, r))).unwrap_or_else(|| "void".into());
        let (give, gives) = if ret == "void" { ("", "") } else { ("var answer = ", "            return answer;\n") };
        let run = if stored {
            format!(
                "        lock (All)\n        {{\n            var kept = All.TryGetValue(instance, out var found) ? found.Copy() : Kept.New(db);\n            {give}{function}With(db, kept{names});\n            All[instance] = kept;\n{gives}        }}\n"
            )
        } else {
            let give = if ret == "void" { "" } else { "return " };
            format!("        {give}{function}With(db, Kept.New(db){names});\n")
        };
        out.push_str(&format!(
            "    /// <summary>{name}.{} in code, for instance <paramref name=\"instance\"/> (a window, or 0 for the app).</summary>\n    public static {ret} {function}(Db db, long instance{params})\n    {{\n{run}    }}\n\n",
            decl.name.name
        ));
        out.push_str(&inner);
    }
    out = out.trim_end_matches('\n').to_string() + "\n}\n\n";
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// What a handler may read of a part.
#[derive(Debug, Clone, Default)]
pub struct PartInfo {
    /// The data a table or list shows, for its selected row.
    pub bind: Option<String>,
}

/// The window or app a handler belongs to.
#[derive(Debug, Clone, Default)]
pub struct WindowCtx {
    pub parts: HashMap<String, PartInfo>,
    /// The window's data: name, entity and whether it is a list.
    pub data: Vec<(String, String, bool)>,
}

/// The body of a handler, as the method `On{Name}` of the ViewModel.
pub fn handler(project: &Project, types: &FileTypes, ctx: &WindowCtx, decl: &HandlerDecl) -> Result<String, Vec<String>> {
    let mut emitter = Emitter::new(project, types, None, Some(ctx));
    let mut body = String::new();
    emitter.block(&decl.body, 2, &mut body);
    if !emitter.errors.is_empty() {
        return Err(emitter.errors);
    }
    // The values a click gives it, typed.
    let params: Vec<String> = decl.params.iter().map(|p| format!("{} {}", value_type(&type_ref_info(project, &p.ty)), camel(&p.name.name))).collect();
    Ok(format!("    void On{}({})\n    {{\n{body}    }}\n", pascal(&decl.name.name), params.join(", ")))
}

/// A value written in the design as C#, of the parameter's type.
pub fn literal(project: &Project, ty: &TypeRef, value: &clarity_spec::model::Literal) -> String {
    use clarity_spec::model::Literal;
    match (type_ref_info(project, ty), value) {
        (TypeInfo::Decimal | TypeInfo::Money, Literal::Number(n)) => format!("{n}m"),
        (_, Literal::Number(n)) => format!("{n}L"),
        (_, Literal::Str(text)) => string(text),
        (_, Literal::Bool(b)) => b.to_string(),
        (TypeInfo::Enum(e), Literal::Ident(v)) => format!("{e}.{v}"),
        (_, Literal::Ident(v)) => v.clone(),
    }
}

pub(crate) use crate::type_ref_info;

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

/// A part property a handler reads or sets, as its `PartState` property.
fn part_property(property: &str) -> Option<&'static str> {
    Some(match property {
        "visible" => "IsVisible",
        "enabled" => "IsEnabled",
        "readonly" => "IsReadOnly",
        "folded" => "IsFolded",
        "collapsed" => "IsCollapsed",
        "maximized" => "IsMaximized",
        "tab" => "Tab",
        "caption" => "Caption",
        "text" => "Text",
        "checked" => "IsChecked",
        "error" => "Error",
        "selected" => "SelectedItem",
        "x" => "X",
        "y" => "Y",
        _ => return None,
    })
}

struct Emitter<'a> {
    project: &'a Project,
    types: &'a FileTypes,
    /// The function, action or data event being written; None for a handler.
    decl: Option<&'a FunctionDecl>,
    /// The window of a handler.
    window: Option<&'a WindowCtx>,
    /// The database: a parameter of `Logic`, a property of a ViewModel.
    db: &'static str,
    /// The engine written in Clarity whose function or keep value this is.
    engine: Option<&'a EngineDecl>,
    errors: Vec<String>,
}

impl<'a> Emitter<'a> {
    fn new(project: &'a Project, types: &'a FileTypes, decl: Option<&'a FunctionDecl>, window: Option<&'a WindowCtx>) -> Self {
        Emitter {
            project,
            types,
            decl,
            window,
            db: if window.is_some() { "Db" } else { "db" },
            engine: None,
            errors: Vec::new(),
        }
    }

    fn returns(&self) -> Option<TypeInfo> {
        self.decl.and_then(|d| d.returns.as_ref()).map(|r| type_ref_info(self.project, r))
    }

    fn function(&mut self) -> String {
        let decl = self.decl.expect("a function or action");
        let mut params = vec!["Db db".to_string()];
        let name = match self.engine {
            Some(_) => {
                params.push("Kept kept".to_string());
                format!("{}With", pascal(&decl.name.name))
            }
            None => function_name(self.project, &decl.name.name),
        };
        for param in &decl.params {
            let info = type_ref_info(self.project, &param.ty);
            params.push(format!("{} {}", value_type(&info), camel(&param.name.name)));
        }
        let ret = self.returns().map(|r| value_type(&r)).unwrap_or_else(|| "void".into());
        let mut out = format!("    public static {ret} {name}({})\n    {{\n", params.join(", "));
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
        let db = self.db;
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
                if let Some((part, property)) = self.part_property(target) {
                    // A caption or error is kept as written with its slots, and shown in the language and format of the person.
                    if matches!(property, "Caption" | "Error") {
                        let value = self.shown(value);
                        out.push_str(&format!("{pad}{part}.{property}Text = {value};\n"));
                    } else {
                        let value = self.value(value);
                        out.push_str(&format!("{pad}{part}.{property} = {value};\n"));
                    }
                    return;
                }
                // A record given to a reference field is stored as its id.
                if self.is_reference_field(target) {
                    let stored = self.stored_reference(target);
                    let record = if matches!(value.kind, ExprKind::Empty) { "null".to_string() } else { format!("{}.Id", self.value(value)) };
                    out.push_str(&format!("{pad}{stored} = {record};\n"));
                    return;
                }
                // A file as a picture: linked in the field's own store, else copied.
                if self.types.type_of(value) == TypeInfo::File
                    && let ExprKind::Member { object, member } = &target.kind
                    && let TypeInfo::Record(owner) = self.types.type_of(object)
                {
                    let store = self.project.entity(&owner).and_then(|e| e.field(&member.name)).and_then(|f| f.modifiers.store.clone());
                    let store = store.map_or("null".to_string(), |s| string(&s));
                    let place = self.place(target);
                    let file = self.value(value);
                    out.push_str(&format!("{pad}{place} = {db}.PictureFrom({file}, {store});\n"));
                    return;
                }
                // A list position, counted from 1.
                if let ExprKind::Index { list, index } = &target.kind {
                    let (list, at, value) = (self.list_ref(list), self.value(index), self.value(value));
                    out.push_str(&format!("{pad}Lists.Set({list}, {at}, {value});\n"));
                    return;
                }
                let place = self.place(target);
                // A text value shows its text in the person's language and format, as a label does.
                let value = if matches!(target.kind, ExprKind::Name { .. })
                    && matches!(self.types.name_of(target), Some(NameKind::Data))
                    && self.types.type_of(target) == TypeInfo::Text
                {
                    format!("Locale.Show({})", self.shown(value))
                } else {
                    self.value(value)
                };
                out.push_str(&format!("{pad}{place} = {value};\n"));
            }
            StmtKind::Write { text, store, path } => {
                // Bytes are written as they are, text as UTF-8.
                let write = if self.types.type_of(text) == TypeInfo::Bytes { "WriteBytes" } else { "WriteText" };
                let (path, text) = (self.value(path), self.value(text));
                out.push_str(&format!("{pad}{db}.{write}({}, {path}, {text});\n", string(&store.name)));
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
                out.push_str(&format!("{pad}{db}.Save({record});\n"));
            }
            StmtKind::Delete { value } => {
                let record = self.value(value);
                out.push_str(&format!("{pad}{db}.Delete({record});\n"));
            }
            StmtKind::Refuse { text } => {
                let text = self.shown(text);
                out.push_str(&format!("{pad}throw new RefusedException({text});\n"));
            }
            StmtKind::Message { .. } | StmtKind::Ask { .. } | StmtKind::Open { .. } | StmtKind::Close | StmtKind::CloseWith { .. } | StmtKind::Reload { .. } if self.window.is_none() => {
                self.errors.push("messages, windows and window data belong in handlers".into());
            }
            // A shape on a canvas: the runtime's Shape (conformance/drawing.json).
            StmtKind::Draw { canvas, shape, args } => {
                use clarity_spec::code::check::ShapeValue;
                let record = match shape.name.as_str() {
                    "clear" => "Clear",
                    "line" => "Line",
                    "rect" => "Rect",
                    "circle" => "Circle",
                    "ring" => "Ring",
                    _ => "Words",
                };
                let mut values = Vec::new();
                for (_, kind, given) in crate::shape_values(&shape.name, args) {
                    values.push(match (kind, given) {
                        (ShapeValue::Number, Ok(arg)) => match self.types.type_of(arg) {
                            TypeInfo::Decimal | TypeInfo::Money => format!("Numbers.Units({})", self.value(arg)),
                            _ => format!("(double)({})", self.value(arg)),
                        },
                        (ShapeValue::Number, Err(default)) => default.to_string(),
                        (ShapeValue::Color, Ok(arg)) => self.value(arg),
                        (ShapeValue::Color | ShapeValue::Text, Err(default)) => string(default),
                        (ShapeValue::Text, Ok(arg)) => self.shown(arg),
                    });
                }
                out.push_str(&format!("{pad}{}.Draw(new Shape.{record}({}));\n", member(&canvas.name), values.join(", ")));
            }
            StmtKind::Message { level, text } => {
                let level = match level {
                    MessageLevel::Info => "Info",
                    MessageLevel::Warning => "Warning",
                    MessageLevel::Error => "Error",
                };
                let text = self.shown(text);
                out.push_str(&format!("{pad}Say(Level.{level}, {text});\n"));
            }
            StmtKind::Ask { question, form, then, args } => {
                // The values written for the handler, as they are when it asks.
                let mut passed = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    let local = format!("asked{}_{i}", stmt.span.start);
                    out.push_str(&format!("{pad}var {local} = {};\n", self.value(arg)));
                    passed.push(local);
                }
                let method = format!("On{}", pascal(&then.name));
                // A dialog window opens as `open` opens it; what it closes with is the answer, of the type it gives.
                if let AskForm::Window { window, with } = form.as_ref() {
                    let init: Vec<String> = with.iter().map(|a| format!("{} = {}", pascal(&a.name.name), self.value(&a.value))).collect();
                    let init = if init.is_empty() { String::new() } else { format!(" {{ {} }}", init.join(", ")) };
                    let gives = self.project.files.iter().flat_map(|f| &f.pages).find(|p| p.name.name == window.name).and_then(|p| p.gives.as_ref());
                    let (word, of) = gives.map_or(("text", None), |g| crate::ask_type(self.project, g));
                    let answer = match word {
                        "record" | "enum" => format!("({})answer!", of.unwrap_or_default()),
                        "integer" | "duration" => "(long)answer!".to_string(),
                        "decimal" | "money" => "(decimal)answer!".to_string(),
                        "date" => "(DateOnly?)answer".to_string(),
                        "datetime" => "(DateTime?)answer".to_string(),
                        _ => "(string)answer!".to_string(),
                    };
                    let mut values = passed.clone();
                    values.push(answer);
                    out.push_str(&format!("{pad}Ask(new {}ViewModel({db}){init}, answer => {method}({}));\n", window.name, values.join(", ")));
                    return;
                }
                let question = question.as_ref().map_or("Text.Plain(\"\")".to_string(), |q| self.shown(q));
                let (form, answered) = match form.as_ref() {
                    AskForm::Confirm { yes, no } => {
                        let yes = yes.as_ref().map_or("null".to_string(), |t| self.shown(t));
                        let no = no.as_ref().map_or("null".to_string(), |t| self.shown(t));
                        (format!("new ConfirmForm({yes}, {no})"), format!("_ => {method}({})", passed.join(", ")))
                    }
                    AskForm::Value { ty, default, from } => {
                        let (word, of) = crate::ask_type(self.project, ty);
                        let format = match word {
                            "integer" => "new FieldFormat(FormatKind.Integer)",
                            "decimal" => "new FieldFormat(FormatKind.Decimal)",
                            "money" => "new FieldFormat(FormatKind.Money)",
                            "date" => "new FieldFormat(FormatKind.Date)",
                            "datetime" => "new FieldFormat(FormatKind.DateTime)",
                            "duration" => "new FieldFormat(FormatKind.Duration)",
                            _ => "null",
                        };
                        let of = of.unwrap_or_default();
                        // An enum's values, or the records to choose from by their ids.
                        let choices = match word {
                            "enum" => format!("Choices.{of}"),
                            "record" => {
                                let records = from.as_ref().map_or(format!("{db}.All<{of}>()"), |list| self.value(list));
                                let label = self.project.entity(&of).map_or("r.Id.ToString()".to_string(), super::windows::display);
                                format!("{records}.Select(r => new Choice(r.Id, {label})).ToList()")
                            }
                            _ => "null".to_string(),
                        };
                        let start = default.as_ref().map_or("null".to_string(), |d| format!("(object?)({})", self.value(d)));
                        let answer = match word {
                            "record" => format!("{db}.Get<{of}>((long)answer!)"),
                            "enum" => format!("({of})answer!"),
                            "integer" | "duration" => "(long)answer!".to_string(),
                            "decimal" | "money" => "(decimal)answer!".to_string(),
                            "date" => "(DateOnly?)answer".to_string(),
                            "datetime" => "(DateTime?)answer".to_string(),
                            _ => "(string)answer!".to_string(),
                        };
                        let mut values = passed.clone();
                        values.push(answer);
                        (format!("new ValueForm({format}, {choices}, {start})"), format!("answer => {method}({})", values.join(", ")))
                    }
                    AskForm::Window { .. } => unreachable!("a dialog window is asked above"),
                };
                out.push_str(&format!("{pad}Ask({question}, {form}, {answered});\n"));
            }
            StmtKind::Open { window, with } => {
                let args: Vec<String> = with.iter().map(|arg| format!("{} = {}", pascal(&arg.name.name), self.value(&arg.value))).collect();
                let init = if args.is_empty() { String::new() } else { format!(" {{ {} }}", args.join(", ")) };
                out.push_str(&format!("{pad}Open(new {}ViewModel({db}){init});\n", window.name));
            }
            StmtKind::Close => out.push_str(&format!("{pad}Close();\n")),
            StmtKind::CloseWith { value } => out.push_str(&format!("{pad}CloseWith({});\n", self.value(value))),
            StmtKind::Reload { data } => {
                let found = self.window.and_then(|w| w.data.iter().find(|(n, _, _)| *n == data.name)).cloned();
                match found {
                    Some((name, entity, true)) => out.push_str(&format!("{pad}{} = {db}.All<{entity}>();\n", member(&name))),
                    Some((name, entity, false)) => {
                        let prop = member(&name);
                        out.push_str(&format!("{pad}if ({prop}.Id != 0)\n{pad}    {prop} = {db}.Get<{entity}>({prop}.Id);\n"));
                    }
                    None => self.errors.push(format!("there is no data '{}' to reload", data.name)),
                }
            }
        }
    }

    /// Text the screen shows (a message, a refusal) as a runtime `Text`: the
    /// text as written with a typed value per slot, so the screen fills the
    /// slots in the person's format.
    fn shown(&mut self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Str { value } => format!("Text.Plain({})", string(value)),
            // A key, whose value the language shown gives, with its slots by name.
            ExprKind::Key { key, args } => {
                let mut code = format!("Text.Of({})", string(&key.name));
                for arg in args {
                    let slot = self.slot(&arg.value);
                    code.push_str(&format!(".With({}, {slot})", string(&arg.name.name)));
                }
                code
            }
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
            TypeInfo::Duration => format!("Slot.OfDuration({value})"),
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
    fn field_of(&self, expr: &Expr) -> Option<&'a clarity_spec::model::Field> {
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

    /// A part property such as `grid.selected` or `name_input.error`: the
    /// part's `PartState` and its property, when `expr` is one.
    fn part_property(&self, expr: &Expr) -> Option<(String, &'static str)> {
        let ExprKind::Member { object, member: prop } = &expr.kind else { return None };
        let ExprKind::Name { name } = &object.kind else { return None };
        if self.types.name_of(object) != Some(&NameKind::Part) {
            return None;
        }
        Some((member(name), part_property(&prop.name)?))
    }

    /// Reading a part property in a handler; the selected row is a record.
    fn read_part(&mut self, expr: &Expr) -> String {
        let (part, property) = self.part_property(expr).unwrap_or_default();
        if property != "SelectedItem" {
            return format!("{part}.{property}");
        }
        let ExprKind::Member { object, .. } = &expr.kind else { return String::new() };
        let ExprKind::Name { name } = &object.kind else { return String::new() };
        let entity = self
            .window
            .and_then(|w| w.parts.get(name))
            .and_then(|p| p.bind.as_ref())
            .and_then(|bind| self.window.and_then(|w| w.data.iter().find(|(n, _, _)| n == bind)))
            .map(|(_, entity, _)| entity.clone());
        match entity {
            Some(entity) => format!("Selected<{entity}>({part})"),
            None => {
                self.errors.push(format!("'{name}' is not bound to a list"));
                String::new()
            }
        }
    }

    /// An expression as a place: what can be assigned.
    fn place(&mut self, expr: &Expr) -> String {
        let db = self.db;
        match &expr.kind {
            ExprKind::Name { name } => match self.types.name_of(expr) {
                Some(NameKind::Local) => camel(name),
                Some(NameKind::Kept) => format!("kept.{}", pascal(name)),
                Some(NameKind::Data) if self.window.is_some() => member(name),
                Some(NameKind::EnumValue(e)) => format!("{e}.{name}"),
                _ => {
                    self.errors.push(format!("'{name}' is a part; use one of its properties"));
                    String::new()
                }
            },
            // A part property inside a path: `grid.selected` in `grid.selected.number`.
            ExprKind::Member { .. } if self.part_property(expr).is_some() => self.read_part(expr),
            ExprKind::Member { object, member: field } => {
                if let Some(NameKind::EnumValue(e)) = self.types.name_of(expr) {
                    return format!("{e}.{}", field.name);
                }
                // A file's details: Name, Path, Extension, Size, Modified.
                if self.types.type_of(object) == TypeInfo::File {
                    let object = self.place(object);
                    return format!("{object}.{}", pascal(&field.name));
                }
                if self.types.type_of(object) == TypeInfo::Bytes {
                    let object = self.place(object);
                    return format!("{object}.LongLength");
                }
                let TypeInfo::Record(entity) = self.types.type_of(object) else {
                    self.errors.push(format!("'{}' is not generated yet", print::expr(expr)));
                    return String::new();
                };
                let object = self.place(object);
                // Every record has an id, read as the record's own `Id`.
                let id = field.name == clarity_spec::code::check::RECORD_ID && self.field_type(&entity, &field.name).is_none();
                let prop = if id { "Id".to_string() } else { property(&entity, &field.name) };
                match self.field_type(&entity, &field.name) {
                    Some(FieldType::Ref { entity: target }) => format!("{db}.Load<{}>({object}.{prop})", target.name),
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
            ExprKind::ListOf { items } => {
                let items: Vec<String> = items.iter().map(|i| self.value(i)).collect();
                if items.is_empty() { format!("new {}()", value_type(&ty)) } else { format!("new {} {{ {} }}", value_type(&ty), items.join(", ")) }
            }
            ExprKind::Index { list, index } => {
                let (list, at) = (self.list_ref(list), self.value(index));
                format!("Lists.At({list}, {at})")
            }
            // A list of values is a value: what reads it gets a copy of its own.
            ExprKind::Name { .. } | ExprKind::Member { .. } if matches!(ty, TypeInfo::Values(_)) && self.part_property(expr).is_none() => {
                format!("new {}({})", value_type(&ty), self.place(expr))
            }
            ExprKind::Number { value } if is_decimal(&ty) => format!("{value}m"),
            ExprKind::Number { value } => format!("{value}L"),
            ExprKind::Str { value } => string(value),
            // A text with slots in code is filled plainly; a key takes its value
            // in the language shown, then its values in the person's format.
            ExprKind::Text { .. } => format!("{}.Show()", self.shown(expr)),
            ExprKind::Key { .. } => format!("Locale.Show({})", self.shown(expr)),
            ExprKind::Bool { value } => value.to_string(),
            ExprKind::Empty => "null".to_string(),
            ExprKind::Member { .. } if self.part_property(expr).is_some() => self.read_part(expr),
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
            ExprKind::Call { function, args } if self.types.name_of(expr) == Some(&NameKind::Handler) => {
                let values: Vec<String> = args.iter().map(|a| self.value(a)).collect();
                format!("On{}({})", pascal(&function.name), values.join(", "))
            }
            ExprKind::Call { function, args } => self.call(&function.name, args),
            ExprKind::Find { entity, filter, order, var } => self.find(&var.name, &entity.name, filter.as_deref(), order),
            ExprKind::Files { var, store, filter, order } => self.files(&var.name, &store.name, filter.as_deref(), order),
            ExprKind::ReadText { file } => {
                let db = self.db;
                format!("{db}.ReadText({})", self.value(file))
            }
            ExprKind::ReadBytes { file } => {
                let db = self.db;
                format!("{db}.ReadBytes({})", self.value(file))
            }
            ExprKind::EngineCall { engine, function, args } if crate::engine(self.project, &engine.name).is_some_and(|e| e.body.is_some()) => {
                self.written_call(&engine.name, &function.name, args)
            }
            // A function of an engine, through the runtime's host (DESIGN 5.11).
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
                    .unwrap_or_else(|| "object".into());
                // An engine with state per window is called in the handler's own window.
                let per_window = self.project.files.iter().flat_map(|f| &f.engines).any(|e| e.name.name == engine.name && e.state == clarity_spec::code::EngineState::Window);
                let window = per_window.then(|| "Instance".to_string());
                let values: Vec<String> = window.into_iter().chain(std::iter::once(string(&function.name))).chain(args.iter().map(|a| self.value(a))).collect();
                let call = if per_window { "CallIn" } else { "Call" };
                let called = format!("Engines.{}.{call}<{returns}>({})", pascal(&engine.name), values.join(", "));
                // The text an engine gives is in the person's language, as its `says` lists it.
                if self.types.type_of(expr) == TypeInfo::Text { format!("Locale.Translate({called})") } else { called }
            }
            ExprKind::New { entity } => format!("new {}()", entity.name),
        }
    }

    /// Whether a date or reference field is empty (`x == none`); a record
    /// that is no field is none until it is stored.
    fn is_none(&mut self, expr: &Expr) -> String {
        if self.is_reference_field(expr) {
            return format!("({} == null)", self.stored_reference(expr));
        }
        if matches!(self.types.type_of(expr), TypeInfo::Record(_)) {
            return format!("({}.Id == 0)", self.value(expr));
        }
        format!("({} == null)", self.value(expr))
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

    /// A list as it is, for a position or a count: a name or field read in
    /// place, anything else computed.
    fn list_ref(&mut self, list: &Expr) -> String {
        match &list.kind {
            ExprKind::Name { .. } | ExprKind::Member { .. } if self.part_property(list).is_none() => self.place(list),
            _ => self.value(list),
        }
    }

    /// A function of an engine written in Clarity, on the instance of the
    /// handler's window (state per window) or the app's (0).
    fn written_call(&mut self, engine: &str, function: &str, args: &[Expr]) -> String {
        let Some(decl) = crate::engine(self.project, engine) else { return String::new() };
        let instance = match decl.state {
            EngineState::Window if self.window.is_some() => "Instance",
            EngineState::Window => {
                self.errors.push(format!("engine {engine} keeps state per window; call it from a handler"));
                ""
            }
            _ => "0",
        };
        let mut code = vec![self.db.to_string(), instance.to_string()];
        code.extend(args.iter().map(|a| self.value(a)));
        format!("{}.{}({})", engine_class(engine), pascal(function), code.join(", "))
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> String {
        use clarity_spec::code::check::{NOW, TODAY};
        if name == TODAY {
            return "Clock.Today()".into();
        }
        if name == NOW {
            return "Clock.Now()".into();
        }
        // round(value, places), as every runtime rounds (conformance/rounding.json).
        // sin(degrees), cos(degrees), as every runtime gives them (conformance/drawing.json).
        if name == clarity_spec::code::check::SIN || name == clarity_spec::code::check::COS {
            let value = args.first().map(|a| self.value(a)).unwrap_or_default();
            let function = if name == clarity_spec::code::check::SIN { "Sin" } else { "Cos" };
            return format!("Numbers.{function}({value})");
        }
        if name == clarity_spec::code::check::ROUND {
            let values: Vec<String> = args.iter().map(|a| self.value(a)).collect();
            return format!("Numbers.Round({})", values.join(", "));
        }
        if name == clarity_spec::code::check::RANDOM && !self.project.files.iter().flat_map(|f| &f.functions).any(|f| f.name.name == name) {
            let max = args.first().map(|a| self.value(a)).unwrap_or_default();
            return format!("Splitmix.Random({max})");
        }
        // An engine's own function, on the same kept values.
        if let Some(engine) = self.engine
            && engine.body.as_ref().is_some_and(|b| b.functions.iter().any(|f| f.name.name == name))
        {
            let mut code = vec![self.db.to_string(), "kept".to_string()];
            code.extend(args.iter().map(|a| self.value(a)));
            return format!("{}With({})", pascal(name), code.join(", "));
        }
        if name == "count" {
            let list = args.first().map(|a| self.list_ref(a)).unwrap_or_default();
            let simple = list.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '@');
            return if simple { format!("(long){list}.Count") } else { format!("(long)({list}).Count") };
        }
        if !self.project.files.iter().flat_map(|f| &f.functions).any(|f| f.name.name == name) {
            self.errors.push(format!("unknown function '{name}'"));
            return String::new();
        }
        let mut code = vec![self.db.to_string()];
        code.extend(args.iter().map(|a| self.value(a)));
        // A ViewModel reaches the functions through `Logic`.
        let owner = if self.window.is_some() { "Logic." } else { "" };
        format!("{owner}{}({})", function_name(self.project, name), code.join(", "))
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
            let Some(path) = crate::query::order_path(self.project, entity, key) else {
                self.errors.push(format!("{entity} cannot be ordered by '{}'", key.field.name));
                continue;
            };
            let numeric = self.field_type(&path.entity, &path.field).is_some_and(|t| is_stored_number(&t));
            let column = string(&self.column(&path.entity, &path.field));
            if path.hops.is_empty() {
                let method = if numeric { "OrderNumber" } else { "Order" };
                query.push_str(&format!(".{method}({column}, {})", key.descending));
            } else {
                let hops: Vec<String> =
                    path.hops.iter().map(|h| format!("new Hop({}, {}, {})", string(&h.column), string(&h.table), string(&h.key))).collect();
                query.push_str(&format!(".OrderThrough([{}], {column}, {}, {numeric})", hops.join(", "), key.descending));
            }
        }
        format!("{}.Find<{entity}>({query})", self.db)
    }

    /// `files f in store where ... order by ...`: the files of a store,
    /// filtered and sorted in memory, text by its ordinal order as on every stack.
    fn files(&mut self, var: &str, store: &str, filter: Option<&Expr>, order: &[OrderKey]) -> String {
        let mut code = format!("{}.Files({})", self.db, string(store));
        let name = camel(var);
        if let Some(filter) = filter {
            let cond = self.value(filter);
            code.push_str(&format!(".Where({name} => {cond})"));
        }
        for (i, key) in order.iter().enumerate() {
            let method = match (i, key.descending) {
                (0, false) => "OrderBy",
                (0, true) => "OrderByDescending",
                (_, false) => "ThenBy",
                (_, true) => "ThenByDescending",
            };
            let text = matches!(key.field.name.as_str(), "name" | "path" | "extension");
            let comparer = if text { ", StringComparer.Ordinal" } else { "" };
            code.push_str(&format!(".{method}({name} => {name}.{}{comparer})", pascal(&key.field.name)));
        }
        code.push_str(".ToList()");
        code
    }

    /// A query condition as a `Cond`. The checker made sure each comparison
    /// has a field of `var` on one side and a value on the other.
    fn cond(&mut self, expr: &Expr, var: &str, entity: &str) -> String {
        let project = self.project;
        let path_of = |e: &Expr| crate::query::field_path(project, entity, var, e);
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
                let name = string(&self.column(&path.entity, &path.field));
                let own = if matches!(value.kind, ExprKind::Empty) {
                    let test = format!("Cond.IsNone({name})");
                    if op == BinaryOp::Eq { test } else { format!("{test}.Negated()") }
                } else if op == BinaryOp::Contains {
                    format!("Cond.Contains({name}, {})", self.value(value))
                } else {
                    let constructor = if is_stored_number(&ty) { "Number" } else { "Compare" };
                    let value = self.sql_value(&ty, value);
                    format!("Cond.{constructor}({name}, Op.{}, {value})", op_name(op))
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
                let constructor = if is_stored_number(&ty) { "OneOfNumbers" } else { "OneOf" };
                let values: Vec<String> = options.iter().map(|o| self.sql_value(&ty, o)).collect();
                let own = format!("Cond.{constructor}({}, [{}])", string(&self.column(&path.entity, &path.field)), values.join(", "));
                through(&path.hops, own)
            }
            _ => match path_of(expr) {
                Some(path) => through(&path.hops, format!("Cond.IsTrue({})", string(&self.column(&path.entity, &path.field)))),
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

/// A condition on the record the steps lead to, as one on the query's own
/// record: each step a `Cond.Through`, the first one outermost.
fn through(hops: &[crate::query::Hop], cond: String) -> String {
    hops.iter().rev().fold(cond, |inner, h| {
        format!("Cond.Through({}, {}, {}, {inner})", string(&h.column), string(&h.table), string(&h.key))
    })
}
