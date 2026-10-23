//! What every stack's command-line program is made from (DESIGN 5.20):
//! the program's name and help, and its commands with their arguments and
//! options in the words conformance/command-line.json uses, which each
//! generator writes in its own language.

use clarity_spec::code::check::TypeInfo;
use clarity_spec::code::{CommandDecl, Expr, ExprKind, TypeRef, UnaryOp, hyphenated};
use clarity_spec::model::Project;

/// A text people read: a key, whose value the program shows in the
/// language shown, or a text as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Said {
    Key(String),
    Plain(String),
}

fn said(expr: Option<&Expr>) -> Option<Said> {
    match &expr?.kind {
        ExprKind::Key { key, .. } => Some(Said::Key(key.name.clone())),
        ExprKind::Str { value } => Some(Said::Plain(value.clone())),
        ExprKind::Text { parts } => Some(Said::Plain(clarity_spec::code::template(parts))),
        _ => None,
    }
}

/// An argument or option as the runtime's parser takes it.
#[derive(Debug, Clone)]
pub(crate) struct ParamSpec {
    /// As typed: `call-after`.
    pub name: String,
    /// The kind's word: text, integer, decimal, money, date, datetime,
    /// duration, enum or bool.
    pub kind: &'static str,
    /// The type code holds it as.
    pub ty: TypeInfo,
    /// The values of an enum.
    pub values: Vec<String>,
    pub default: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub required: bool,
    pub help: Option<Said>,
}

/// A command, with the module it is declared in.
#[derive(Debug, Clone)]
pub(crate) struct CommandSpec<'a> {
    pub decl: &'a CommandDecl,
    pub module: String,
    /// The words typed: `["customer", "add"]`.
    pub line: Vec<String>,
    pub help: Option<Said>,
    pub arguments: Vec<ParamSpec>,
    pub options: Vec<ParamSpec>,
}

/// A value written in Clarity as the command line writes it: `1000`, `-5`,
/// `true`, an enum value by its name.
pub(crate) fn literal_text(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Number { value } => Some(value.clone()),
        ExprKind::Str { value } => Some(value.clone()),
        ExprKind::Bool { value } => Some(value.to_string()),
        ExprKind::Name { name } => Some(name.clone()),
        ExprKind::Member { member, .. } => Some(member.name.clone()),
        ExprKind::Unary { op: UnaryOp::Neg, operand } => literal_text(operand).map(|v| format!("-{v}")),
        _ => None,
    }
}

fn kind_of(project: &Project, ty: &TypeRef) -> (&'static str, TypeInfo, Vec<String>) {
    let info = crate::type_ref_info(project, ty);
    let (kind, values) = match &info {
        TypeInfo::Integer => ("integer", Vec::new()),
        TypeInfo::Decimal => ("decimal", Vec::new()),
        TypeInfo::Money => ("money", Vec::new()),
        TypeInfo::Date => ("date", Vec::new()),
        TypeInfo::DateTime => ("datetime", Vec::new()),
        TypeInfo::Duration => ("duration", Vec::new()),
        TypeInfo::Bool => ("bool", Vec::new()),
        TypeInfo::Enum(name) => ("enum", project.enum_decl(name).map(|e| e.variants.iter().map(|v| v.name.name.clone()).collect()).unwrap_or_default()),
        _ => ("text", Vec::new()),
    };
    (kind, info, values)
}

/// The program's name, as typed: the project's name in lower case with
/// hyphens (`MyShop` is `my-shop`).
pub(crate) fn program_name(settings: &clarity_project::Settings) -> String {
    hyphenated(&settings.name)
}

/// The program's description, from its `cli` block.
pub(crate) fn program_help(project: &Project) -> Option<Said> {
    project.files.iter().flat_map(|f| &f.clis).next().and_then(|c| said(c.help.as_ref()))
}

/// Every command of the project, in the order the modules declare them.
pub(crate) fn commands(project: &Project) -> Vec<CommandSpec<'_>> {
    let mut out = Vec::new();
    for file in &project.files {
        let Some(module) = &file.module else { continue };
        for decl in &file.commands {
            let arguments = decl
                .params
                .iter()
                .map(|p| {
                    let (kind, ty, values) = kind_of(project, &p.ty);
                    ParamSpec { name: hyphenated(&p.name.name), kind, ty, values, default: None, min: None, max: None, required: false, help: None }
                })
                .collect();
            let options = decl
                .options
                .iter()
                .map(|o| {
                    let (kind, ty, values) = kind_of(project, &o.ty);
                    ParamSpec {
                        name: hyphenated(&o.name.name),
                        kind,
                        ty,
                        values,
                        default: o.default.as_ref().and_then(literal_text),
                        min: o.min.as_ref().and_then(literal_text),
                        max: o.max.as_ref().and_then(literal_text),
                        required: o.required,
                        help: said(o.help.as_ref()),
                    }
                })
                .collect();
            out.push(CommandSpec {
                decl,
                module: module.name.clone(),
                line: decl.line().split(' ').map(str::to_string).collect(),
                help: said(decl.help.as_ref()),
                arguments,
                options,
            });
        }
    }
    out
}

/// The translations a program shows its texts in, and the language the
/// project is written in, as the runtimes' i18n reads a model.
pub(crate) fn texts_json(project: &Project) -> String {
    serde_json::json!({ "project": { "translations": project.translations, "locale": project.locale } }).to_string() + "\n"
}
