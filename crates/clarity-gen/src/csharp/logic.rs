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
    AskForm, BinaryOp, Expr, ExprKind, FunctionDecl, HandlerDecl, MessageLevel, OrderKey, Stmt, StmtKind, TextPart, TypeRef,
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
    if !errors.is_empty() {
        return Err(errors);
    }
    if out.is_empty() {
        return Ok(String::new());
    }
    Ok(format!("public static partial class Logic\n{{\n{}}}\n\n", out.trim_end_matches('\n').to_string() + "\n"))
}

/// The commands of a module (DESIGN 5.20): each a method of `Logic`
/// taking the database, the session it prints and asks through, and its
/// arguments and options, and giving its exit code.
pub fn emit_commands(project: &Project, file: &clarity_spec::model::SpecFile, types: &FileTypes) -> Result<String, Vec<String>> {
    let mut out = String::new();
    let mut errors = Vec::new();
    for command in &file.commands {
        let mut emitter = Emitter::new(project, types, None, None);
        emitter.command = true;
        let mut params = vec!["Db db".to_string(), "CliSession cli".to_string()];
        for (name, ty) in command.params.iter().map(|p| (&p.name, &p.ty)).chain(command.options.iter().map(|o| (&o.name, &o.ty))) {
            params.push(format!("{} {}", value_type(&type_ref_info(project, ty)), camel(&name.name)));
        }
        let mut body = String::new();
        emitter.block(&command.body, 2, &mut body);
        // C# refuses code after a return, so the command's own last return stays the last.
        if !body.trim_end().lines().last().is_some_and(|l| l.trim_start().starts_with("return ")) {
            body.push_str("        return 0;\n");
        }
        if emitter.errors.is_empty() {
            out.push_str(&format!("    /// <summary><c>{}</c></summary>\n    public static int {}({})\n    {{\n{body}    }}\n\n", command.line(), command_method(command), params.join(", ")));
        } else {
            errors.extend(emitter.errors.into_iter().map(|e| format!("command '{}': {e}", command.line())));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    if out.is_empty() {
        return Ok(String::new());
    }
    // Beside the functions and actions, which commands call by name.
    Ok(format!("public static partial class Logic\n{{\n{}}}\n\n", out.trim_end_matches('\n').to_string() + "\n"))
}

/// The C# method of a command: `CommandCustomerAdd`.
pub fn command_method(command: &clarity_spec::code::CommandDecl) -> String {
    let group = command.group.as_ref().map(|g| pascal(&g.name)).unwrap_or_default();
    format!("Command{group}{}", pascal(&command.name.name))
}

/// What a handler may read of a part.
#[derive(Debug, Clone, Default)]
pub struct PartInfo {
    /// The data a table or list shows, for its selected row.
    pub bind: Option<String>,
    /// A custom part (DESIGN 5.19): its kind and the values written on its line.
    pub custom: Option<(String, std::collections::BTreeMap<String, clarity_spec::model::Literal>)>,
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
        "step" => "Step",
        "open" => "OpenSection",
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
    /// Whether this is a command of the command-line program (DESIGN 5.20):
    /// it prints, asks and says through `cli`, and gives its exit code.
    command: bool,
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
            command: false,
            errors: Vec::new(),
        }
    }

    fn returns(&self) -> Option<TypeInfo> {
        self.decl.and_then(|d| d.returns.as_ref()).map(|r| type_ref_info(self.project, r))
    }

    fn function(&mut self) -> String {
        let decl = self.decl.expect("a function or action");
        let mut params = vec!["Db db".to_string()];
        let name = function_name(self.project, &decl.name.name);
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
                if let Some((part, property, _, _)) = self.custom_property(target) {
                    let value = self.value(value);
                    out.push_str(&format!("{pad}{part}.SetValue({}, {value});\n", string(&property)));
                    return;
                }
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
                    // A file field keeps any file, under its name; an image only a picture.
                    let made = if self.types.type_of(target) == TypeInfo::RecordFile { "RecordFileFrom" } else { "PictureFrom" };
                    out.push_str(&format!("{pad}{place} = {db}.{made}({file}, {store});\n"));
                    return;
                }
                // Another record's picture or file: copied, its bytes read when it is only stored.
                if let Some(copy) = crate::copied(self.project, |e| self.types.type_of(e), target, value) {
                    let made = if copy.file { "CopiedFile" } else { "CopiedPicture" };
                    let store = |s: Option<String>| s.map_or("null".to_string(), |s| string(&s));
                    let place = self.place(target);
                    let object = self.value(copy.object);
                    out.push_str(&format!(
                        "{pad}{place} = {db}.{made}<{}>({object}.{}, {object}.Id, {}, {}, {});\n",
                        copy.entity,
                        property(&copy.entity, &copy.field),
                        string(&copy.column),
                        store(copy.from),
                        store(copy.to)
                    ));
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
            StmtKind::Return { value: None } if self.command => out.push_str(&format!("{pad}return 0;\n")),
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
            // A command says it on standard error.
            StmtKind::Message { text, .. } if self.command => {
                let text = self.shown(text);
                out.push_str(&format!("{pad}cli.Message({text});\n"));
            }
            StmtKind::Message { .. } | StmtKind::Ask { .. } | StmtKind::Open { .. } | StmtKind::Close | StmtKind::CloseWith { .. } | StmtKind::Reload { .. } if self.window.is_none() => {
                self.errors.push("messages, windows and window data belong in handlers".into());
            }
            // A shape on a canvas: the runtime's Shape (conformance/drawing.json).
            // `generate` refuses a project with custom parts until CP1 builds them.
            // A hear of a custom part (DESIGN 5.19), which the part hears when it shows.
            // An engine's statement; engines are compiled to WebAssembly before generating (N5b).
            StmtKind::Tell { .. } => self.errors.push("'tell' is for an engine beside the screen".into()),
            StmtKind::Hear { part, hear, args } => {
                let values: String = args.iter().map(|a| format!(", {}", self.value(a))).collect();
                out.push_str(&format!("{pad}{}.Hear({}{values});\n", member(&part.name), string(&hear.name)));
            }
            StmtKind::Draw { canvas, shape, args } => {
                use clarity_spec::code::check::ShapeValue;
                let record = match shape.name.as_str() {
                    "clear" => "Clear",
                    "line" => "Line",
                    "rect" => "Rect",
                    "circle" => "Circle",
                    "ring" => "Ring",
                    "image" => "Image",
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
                        (ShapeValue::Color | ShapeValue::Picture, Ok(arg)) => self.value(arg),
                        (ShapeValue::Color | ShapeValue::Picture | ShapeValue::Text, Err(default)) => string(default),
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
            // Only commands print, exit and ask for an answer; they are generated apart.
            StmtKind::Print { value, columns } if columns.is_empty() => {
                let text = self.shown(value);
                out.push_str(&format!("{pad}cli.Print({text});\n"));
            }
            StmtKind::Print { value, columns } => {
                let code = self.print_table(value, columns);
                out.push_str(&format!("{pad}{code}\n"));
            }
            StmtKind::Exit { code } => out.push_str(&format!("{pad}return (int)({});\n", self.value(code))),
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

    /// `print list columns a, b`: a heading per field (its label), then the
    /// records' values as the command line writes them.
    fn print_table(&mut self, list: &Expr, columns: &[clarity_spec::model::Ident]) -> String {
        let TypeInfo::List(entity) = self.types.type_of(list) else {
            self.errors.push("'columns' prints a list of records".into());
            return String::new();
        };
        let Some(decl) = self.project.entity(&entity) else { return String::new() };
        let (mut headings, mut right, mut cells) = (Vec::new(), Vec::new(), Vec::new());
        for column in columns {
            let found = decl.field(&column.name);
            let label = found.and_then(|f| f.modifiers.label.clone()).unwrap_or_else(|| clarity_spec::model::default_label(&column.name));
            headings.push(match clarity_spec::translations::key_of(&label) {
                Some(key) => format!("Text.Of({})", string(key)),
                None => format!("Text.Plain({})", string(&label)),
            });
            let ty = found.map(|f| &f.ty);
            right.push(matches!(ty, Some(FieldType::Integer | FieldType::Decimal { .. } | FieldType::Money | FieldType::Duration) | None).to_string());
            let at = if column.name == "id" { "r.Id".to_string() } else { format!("r.{}", property(&entity, &column.name)) };
            cells.push(match ty {
                Some(FieldType::Bool) => format!("CliSession.YesNo({at})"),
                _ => format!("Cli.TextOf({at})"),
            });
        }
        let records = self.value(list);
        format!(
            "cli.PrintTable([{}], [{}], [.. {records}.Select(r => (IReadOnlyList<string>)[{}])]);",
            headings.join(", "),
            right.join(", "),
            cells.join(", ")
        )
    }

    /// `ask` in a command: it waits on the terminal and gives the answer.
    fn ask_value(&mut self, question: &Expr, form: &AskForm) -> String {
        let question = self.shown(question);
        match form {
            AskForm::Confirm { yes, no } => {
                let yes = yes.as_ref().map_or("null".to_string(), |t| self.shown(t));
                let no = no.as_ref().map_or("null".to_string(), |t| self.shown(t));
                format!("cli.Confirm({question}, {yes}, {no})")
            }
            AskForm::Value { ty, from: Some(list), .. } => {
                let entity = match type_ref_info(self.project, ty) {
                    TypeInfo::Record(e) => e,
                    _ => String::new(),
                };
                let shown = self
                    .project
                    .entity(&entity)
                    .and_then(|e| e.fields.iter().find(|f| matches!(f.ty, FieldType::Text { .. })))
                    .map_or("Id".to_string(), |f| property(&entity, &f.name.name));
                let list = self.value(list);
                format!("Cli.Pick({list}, all => cli.Choose({question}, [.. all.Select(r => Cli.TextOf(r.{shown}))]))")
            }
            AskForm::Value { ty, default, from: None } => {
                let info = type_ref_info(self.project, ty);
                let fallback = default.as_ref().map_or("null".to_string(), |d| format!("Cli.TextOf({})", self.value(d)));
                let asked = |kind: &str, values: &str| format!("cli.Ask({question}, CliKind.{kind}, {values}, {fallback})");
                match &info {
                    TypeInfo::Integer => format!("Cli.Long({})", asked("Integer", "[]")),
                    TypeInfo::Duration => format!("Cli.Long({})", asked("Duration", "[]")),
                    TypeInfo::Decimal => format!("Cli.Decimal({})", asked("Decimal", "[]")),
                    TypeInfo::Money => format!("Cli.Decimal({})", asked("Money", "[]")),
                    TypeInfo::Date => format!("Cli.Date({})", asked("Date", "[]")),
                    TypeInfo::DateTime => format!("Cli.Moment({})", asked("DateTime", "[]")),
                    TypeInfo::Enum(name) => format!("Cli.Enum<{name}>({})", asked("Enum", &format!("Enum.GetNames<{name}>()"))),
                    _ => format!("Cli.Text({})", asked("Text", "[]")),
                }
            }
            AskForm::Window { .. } => {
                self.errors.push("a command asks no window".into());
                String::new()
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

    /// A custom part's own property (DESIGN 5.19): the part's `PartState`,
    /// the property, the part's kind and the values written on its line.
    fn custom_property(&self, expr: &Expr) -> Option<(String, String, String, std::collections::BTreeMap<String, clarity_spec::model::Literal>)> {
        let ExprKind::Member { object, member: prop } = &expr.kind else { return None };
        let ExprKind::Name { name } = &object.kind else { return None };
        if self.types.name_of(object) != Some(&NameKind::Part) {
            return None;
        }
        let (kind, written) = self.window?.parts.get(name)?.custom.clone()?;
        let declared = crate::parts::declaration(self.project, &kind)?;
        declared.properties.iter().any(|p| p.name.name == prop.name).then(|| (member(name), prop.name.clone(), kind, written))
    }

    /// Reading a custom part's property: as written or set, else as declared.
    fn read_custom(&mut self, expr: &Expr) -> String {
        let (part, property, kind, written) = self.custom_property(expr).unwrap_or_default();
        let fallback = crate::parts::fallback_json(self.project, &kind, &written, &property);
        let ty = value_type(&self.types.type_of(expr));
        format!("{part}.ValueAs<{ty}>({}, {})", string(&property), string(&fallback))
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
                Some(NameKind::Data) if self.window.is_some() => member(name),
                // A command has the settings as a window does: the one shared object.
                Some(NameKind::Data) if self.command => format!("{}.Current", pascal(name)),
                Some(NameKind::EnumValue(e)) => format!("{e}.{name}"),
                _ => {
                    self.errors.push(format!("'{name}' is a part; use one of its properties"));
                    String::new()
                }
            },
            // A part property inside a path: `grid.selected` in `grid.selected.number`.
            ExprKind::Member { .. } if self.custom_property(expr).is_some() => self.read_custom(expr),
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
                // A reply's Status, Ok and Text.
                if self.types.type_of(object) == TypeInfo::HttpReply {
                    let object = self.place(object);
                    return format!("{object}.{}", pascal(&field.name));
                }
                // A record's file: its name, and the extension of its name.
                if self.types.type_of(object) == TypeInfo::RecordFile {
                    let object = self.place(object);
                    let read = if field.name == "extension" { "ExtensionOf" } else { "NameOf" };
                    return format!("RecordFiles.{read}({object})");
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
                    Some(FieldType::List { .. }) if !self.project.in_web_api(&entity) => {
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
    /// The headers or query pairs of an http call, as an array; a pair with
    /// `when` is spread in only while its condition holds.
    fn http_pairs(&mut self, pairs: &[clarity_spec::code::HttpPair]) -> String {
        let items: Vec<String> = pairs
            .iter()
            .map(|p| {
                let pair = format!("({}, {})", string(&p.name.name), self.value(&p.value));
                match &p.when {
                    Some(when) => format!(
                        ".. ({} ? new (string Name, string Value)[] {{ {pair} }} : System.Array.Empty<(string Name, string Value)>())",
                        self.value(when)
                    ),
                    None => pair,
                }
            })
            .collect();
        format!("[{}]", items.join(", "))
    }

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
            ExprKind::Ask { question, form } => self.ask_value(question, form),
            ExprKind::Number { value } if is_decimal(&ty) => format!("{value}m"),
            ExprKind::Number { value } => format!("{value}L"),
            ExprKind::Str { value } => string(value),
            // A text with slots in code is filled plainly; a key takes its value
            // in the language shown, then its values in the person's format.
            ExprKind::Text { .. } => format!("{}.Show()", self.shown(expr)),
            ExprKind::Key { .. } => format!("Locale.Show({})", self.shown(expr)),
            ExprKind::Bool { value } => value.to_string(),
            ExprKind::Empty => "null".to_string(),
            ExprKind::Member { .. } if self.custom_property(expr).is_some() => self.read_custom(expr),
            ExprKind::Member { .. } if self.part_property(expr).is_some() => self.read_part(expr),
            ExprKind::Name { .. } | ExprKind::Member { .. } => self.place(expr),
            ExprKind::Unary { op: UnaryOp::Not, operand } => format!("!{}", self.value(operand)),
            ExprKind::Unary { op: UnaryOp::Neg, operand } => format!("-{}", self.value(operand)),
            // A list of values grows by a list of its items (DESIGN 5.26).
            ExprKind::Binary { op: BinaryOp::Add, left, right } if matches!(ty, TypeInfo::Values(_)) => {
                format!("System.Linq.Enumerable.ToList(System.Linq.Enumerable.Concat({}, {}))", self.value(left), self.value(right))
            }
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
            ExprKind::Call { function, args, .. } if self.types.name_of(expr) == Some(&NameKind::Handler) => {
                let values: Vec<String> = args.iter().map(|a| self.value(a)).collect();
                format!("On{}({})", pascal(&function.name), values.join(", "))
            }
            ExprKind::Call { function, args, .. } => self.call(&function.name, args),
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
            // JSON (DESIGN 5.23): a value as Clarity's JSON, and records read from a service's.
            ExprKind::JsonOf { value } => format!("JsonValues.Of({})", self.value(value)),
            ExprKind::ReadJson { ty, text, at } => {
                let Some((entity, many)) = crate::json_fields::target(ty) else {
                    self.errors.push(format!("'{}' is not generated yet", print::expr(expr)));
                    return String::new();
                };
                let Some(decl) = self.project.entity(entity) else { return String::new() };
                let fields: Vec<String> = crate::json_fields::fields(self.project, decl).iter().map(|(name, kind)| format!("new JsonField({}, {})", string(name), cs_json_kind(kind))).collect();
                let text = self.value(text);
                let at = match at {
                    Some(at) => self.value(at),
                    None => "null".to_string(),
                };
                let read = if many { "ReadList" } else { "Read" };
                format!("JsonValues.{read}({text}, {at}, {}, [{}], new {entity}())", string(entity), fields.join(", "))
            }
            // A call to a web API (DESIGN 5.23): it never fails, its reply says how it went.
            ExprKind::Http { method, url, from, pages, body, headers, query } => {
                let url = self.value(url);
                let headers = self.http_pairs(headers);
                if from.is_some() || !query.is_empty() {
                    // A path of a web API data source (DESIGN 5.23): its address and auth come from the project file.
                    let (base, auth) = from.as_ref().and_then(|f| crate::web_source(self.project, &f.name)).unwrap_or(("", None));
                    let query = self.http_pairs(query);
                    let body = body.as_ref().map_or("null".to_string(), |b| self.value(b));
                    let (kind, secret) = auth.map_or(("null".to_string(), "null".to_string()), |(k, s)| (string(k), string(s)));
                    // Every page, as the source's paging says (DESIGN 5.23).
                    if let Some(p) = from.as_ref().filter(|_| *pages).and_then(|f| crate::web_paging(self.project, &f.name)) {
                        return format!(
                            "Http.Pages({}, {url}, {query}, {headers}, {kind}, {secret}, new HttpPaging({}, {}, {}, {}, {}, {}))",
                            string(base),
                            string(crate::paging_word(p.kind)),
                            string(&p.param),
                            string(&p.size_param),
                            p.size,
                            string(&p.next),
                            string(&p.items)
                        );
                    }
                    return format!("Http.At({}, {}, {url}, {query}, {body}, {headers}, {kind}, {secret})", string(method.verb()), string(base));
                }
                let call = pascal(method.word());
                match body {
                    Some(body) => format!("Http.{call}({url}, {}, {headers})", self.value(body)),
                    None => format!("Http.{call}({url}, {headers})"),
                }
            }
            // A function of a library, through its adapter (DESIGN 5.21).
            ExprKind::EngineCall { engine, function, args } if crate::libraries::library(self.project, &engine.name).is_some() => {
                let values: Vec<String> = args.iter().map(|a| self.value(a)).collect();
                format!("{}.{}.{}({})", crate::libraries::CSHARP_NAMESPACE, engine.name, pascal(&function.name), values.join(", "))
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
            // Whether a list holds an item.
            BinaryOp::Contains if matches!(self.types.type_of(left), TypeInfo::Values(_)) => format!("{}.Contains({})", self.value(left), self.value(right)),
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
            ExprKind::Binary { op: BinaryOp::Matches, .. } => {
                let Some((search, text)) = crate::query::search(project, entity, var, expr) else {
                    self.errors.push(format!("'{}' looks for words in no record of {var}", print::expr(expr)));
                    return String::new();
                };
                let columns: Vec<String> = search.columns.iter().map(|c| string(c)).collect();
                format!("Cond.Matches({}, {}, [{}], {})", string(&search.table), string(&search.key), columns.join(", "), self.value(text))
            }
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
                } else if let FieldType::Values { item } = &ty {
                    // A list field holds the item.
                    format!("Cond.Holds({name}, {})", self.sql_value(&item.field_type(), value))
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
            FieldType::List { .. } | FieldType::File => {
                self.errors.push("a list or a file field cannot be compared in a query".into());
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

/// What a field takes from JSON, as the C# runtime names it.
fn cs_json_kind(kind: &crate::json_fields::Kind) -> String {
    use crate::json_fields::Kind;
    match kind {
        Kind::Enum(values) => format!("new JsonKind(\"enum\", [{}])", values.iter().map(|v| string(v)).collect::<Vec<_>>().join(", ")),
        Kind::Values(item) => format!("new JsonKind(\"values\", null, {})", cs_json_kind(item)),
        // A record held inside (DESIGN 5.25): its entity's fields, over a new record of it.
        Kind::Record(nested) | Kind::Records(nested) => format!(
            "JsonKind.{}({}, [{}], new {}())",
            if matches!(kind, Kind::Record(_)) { "Record" } else { "Records" },
            string(&nested.entity),
            nested.fields.iter().map(|(name, kind)| format!("new JsonField({}, {})", string(name), cs_json_kind(kind))).collect::<Vec<_>>().join(", "),
            nested.entity
        ),
        other => format!("new JsonKind(\"{}\")", other.word()),
    }
}
