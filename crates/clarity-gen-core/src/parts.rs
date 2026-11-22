//! Custom parts in the output (manual/design.custom-parts.md), the same for every
//! stack: what code reads of a part, and the files of Clarity's and the
//! starter that declaring a part writes into its folder. Each stack writes
//! its own part files (c/parts.rs, cpp/parts.rs, go/parts.rs, rust/parts.rs,
//! csharp/parts.rs).

use clarity_spec::code::{PartDecl, PartLibrary, TypeRefKind};
use clarity_spec::Project;

/// The folder of the output the parts are built in.
pub const FOLDER: &str = "parts";

/// The project's native parts, in the order they are declared.
pub fn native_parts(project: &Project) -> Vec<&PartDecl> {
    project.files.iter().flat_map(|f| &f.parts).filter(|p| matches!(p.library, Some(PartLibrary::Native { .. }))).collect()
}

/// The kind under which a sound part keeps what code reads of it
/// (docs/ledger/audio.md): lower case, so no declared part has it.
pub const SOUND_KIND: &str = "sound";

/// The kind under which a video part keeps what code reads of it (docs/ledger/video-camera.md).
pub const VIDEO_KIND: &str = "video";

/// The kind under which a pdfReport keeps the data its report takes, before the report's name: `report:Invoice`.
pub const REPORT_KIND: &str = "report:";

/// The kind under which a text box or memo keeps where the person types: its caret and selected text (Writer gap 21).
pub const TYPING_KIND: &str = "typing";

/// What a part keeps in its values for code to read, as the generators
/// carry it: a custom part's kind and the values on its line, or a sound's.
pub fn kept_of(props: &clarity_spec::model::WidgetProps) -> Option<(String, std::collections::BTreeMap<String, clarity_spec::model::Literal>)> {
    match props.kind {
        clarity_spec::widgets::WidgetKind::Sound => Some((SOUND_KIND.to_string(), Default::default())),
        clarity_spec::widgets::WidgetKind::Video => Some((VIDEO_KIND.to_string(), Default::default())),
        clarity_spec::widgets::WidgetKind::PdfReport => props.report.as_ref().map(|r| (format!("{REPORT_KIND}{r}"), Default::default())),
        clarity_spec::widgets::WidgetKind::Textbox | clarity_spec::widgets::WidgetKind::Memo | clarity_spec::widgets::WidgetKind::Markdown => Some((TYPING_KIND.to_string(), Default::default())),
        _ => props.part.clone().map(|kind| (kind, props.values.clone())),
    }
}

/// Whether a part of `kind` keeps `property` in its values: a custom part's own, or what a sound says.
pub fn keeps(project: &Project, kind: &str, property: &str) -> bool {
    if kind == SOUND_KIND {
        return clarity_spec::sound::read(property).is_some();
    }
    if kind == VIDEO_KIND {
        return clarity_spec::video::read(property).is_some();
    }
    if kind == TYPING_KIND {
        return clarity_spec::typing::read(property).is_some();
    }
    if let Some(report) = kind.strip_prefix(REPORT_KIND) {
        return clarity_spec::reports::report_data(project, report).is_some_and(|d| d.iter().any(|(n, _)| n == property));
    }
    declaration(project, kind).is_some_and(|d| d.properties.iter().any(|p| p.name.name == property))
}

/// The declaration of the custom part of kind `kind`.
pub fn declaration<'a>(project: &'a Project, kind: &str) -> Option<&'a PartDecl> {
    project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == kind)
}

/// A custom part's property as code reads it when no handler set it: the
/// value written on the part's line, else the declared default, else the
/// empty value of its type; as JSON, which each runtime reads as the type.
pub fn fallback_json(project: &Project, kind: &str, written: &std::collections::BTreeMap<String, clarity_spec::model::Literal>, property: &str) -> String {
    use clarity_spec::code::ExprKind;
    use clarity_spec::model::Literal;
    if kind == SOUND_KIND {
        return clarity_spec::sound::read_fallback(property).to_string();
    }
    if kind == VIDEO_KIND {
        return clarity_spec::video::read_fallback(property).to_string();
    }
    if kind == TYPING_KIND {
        return clarity_spec::typing::read_fallback(property).to_string();
    }
    let Some(declared) = declaration(project, kind).and_then(|d| d.properties.iter().find(|p| p.name.name == property)) else {
        return "null".into();
    };
    let whole = matches!(declared.ty.kind, TypeRefKind::Integer | TypeRefKind::Duration);
    let number = |n: &str| if whole { n.to_string() } else { serde_json::Value::String(n.to_string()).to_string() };
    let text = |t: &str| serde_json::Value::String(t.to_string()).to_string();
    if let Some(value) = written.get(property) {
        return match value {
            Literal::Number(n) => number(n),
            Literal::Str(s) | Literal::Ident(s) => text(s),
            Literal::Bool(b) => b.to_string(),
        };
    }
    if let Some(default) = &declared.default {
        match &default.kind {
            ExprKind::Number { value } => return number(value),
            ExprKind::Unary { operand, .. } if matches!(&operand.kind, ExprKind::Number { .. }) => {
                if let ExprKind::Number { value } = &operand.kind {
                    return number(&format!("-{value}"));
                }
            }
            ExprKind::Str { value } => return text(value),
            ExprKind::Bool { value } => return value.to_string(),
            ExprKind::Name { name } => return text(name),
            _ => {}
        }
    }
    match declared.ty.kind {
        TypeRefKind::Integer | TypeRefKind::Duration => "0".into(),
        TypeRefKind::Decimal | TypeRefKind::Money => text("0"),
        TypeRefKind::Bool => "false".into(),
        TypeRefKind::Date | TypeRefKind::DateTime => "null".into(),
        _ => text(""),
    }
}

/// The handlers of `page` wired to a custom part's tells: they run with the
/// tell's values, as the handler an answer goes to does.
pub fn told_handlers(page: Option<&clarity_spec::model::Page>) -> Vec<String> {
    fn walk(widgets: &[clarity_spec::model::Widget], found: &mut Vec<String>) {
        for widget in widgets {
            for tell in &widget.props.tells {
                if !found.contains(&tell.handler) {
                    found.push(tell.handler.clone());
                }
            }
            walk(&widget.children, found);
            for page in &widget.pages {
                walk(&page.children, found);
            }
        }
    }
    let mut found = Vec::new();
    if let Some(page) = page {
        walk(&page.widgets, &mut found);
        for sidebar in &page.frame.sidebars {
            walk(&sidebar.widgets, &mut found);
        }
    }
    found
}

