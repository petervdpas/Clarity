//! Translations (design section 1.5, I1c). Every text a person sees is in
//! the project already: labels, help texts, titles, captions, enum labels,
//! menu items, messages and refusals. Clarity collects them, so nobody
//! marks a text for translation. A file per language in the module folder,
//! `translations/nl.clarity`, gives each text in that language, found by
//! what the text says:
//!
//! ```text
//! translation nl
//!     "Customer card" "Klantkaart"
//!     "Saved {customer.name}" "{customer.name} opgeslagen"
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use crate::code::{Expr, ExprKind, Stmt, StmtKind, template};
use crate::diagnostic::Diagnostic;
use crate::formats::language_problem;
use crate::lexer::{TokenKind, lex, quote};
use crate::model::{Command, Frame, Project, Widget};
use crate::span::Span;

/// The folder of translation files in a project's module folder.
pub const TRANSLATIONS_DIR: &str = "translations";

/// The renderer's own words, which a translation can give too.
pub const RENDERER_TEXTS: [&str; 2] = ["Yes", "No"];

/// What an image part bound to a picture shows of its own: in its empty
/// state, and on its buttons to load a picture and to remove it.
pub const PICTURE_TEXTS: [&str; 3] = ["No picture", "Load a picture", "Remove the picture"];

/// A text people see, and where it is used: `window Card`, `entity Customer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Used {
    pub text: String,
    pub places: Vec<String>,
}

/// The texts of a project, each once, in the order they are first used.
#[derive(Default)]
struct Texts {
    used: Vec<Used>,
}

impl Texts {
    fn add(&mut self, text: &str, place: &str) {
        if text.trim().is_empty() {
            return;
        }
        match self.used.iter_mut().find(|u| u.text == text) {
            Some(used) if !used.places.iter().any(|p| p == place) => used.places.push(place.to_string()),
            Some(_) => {}
            None => self.used.push(Used {
                text: text.to_string(),
                places: vec![place.to_string()],
            }),
        }
    }

    fn some(&mut self, text: &Option<String>, place: &str) {
        if let Some(text) = text {
            self.add(text, place);
        }
    }

    fn widgets(&mut self, widgets: &[Widget], place: &str) {
        for widget in widgets {
            let p = &widget.props;
            self.some(&p.caption, place);
            self.some(&p.placeholder, place);
            self.some(&p.tooltip, place);
            self.widgets(&widget.children, place);
            for page in &widget.pages {
                self.some(&page.title, place);
                self.widgets(&page.children, place);
            }
        }
    }

    fn commands(&mut self, commands: &[Command], place: &str) {
        for command in commands {
            self.some(&command.props.caption, place);
            self.commands(&command.items, place);
        }
    }

    fn frame(&mut self, frame: &Frame, place: &str) {
        if let Some(menu) = &frame.menu {
            self.commands(&menu.items, place);
        }
        if let Some(toolbar) = &frame.toolbar {
            self.commands(&toolbar.items, place);
        }
        if let Some(ribbon) = &frame.ribbon {
            for tab in &ribbon.tabs {
                self.add(&tab.title, place);
                for group in &tab.groups {
                    self.add(&group.title, place);
                    self.commands(&group.items, place);
                }
            }
        }
        for sidebar in &frame.sidebars {
            self.widgets(&sidebar.widgets, place);
        }
        if let Some(footer) = &frame.footer {
            self.some(&footer.status, place);
            self.widgets(&footer.widgets, place);
        }
    }

    /// Text a statement shows: a message, a refusal, a caption or an error.
    fn statements(&mut self, body: &[Stmt], place: &str) {
        for stmt in body {
            match &stmt.kind {
                StmtKind::Message { text, .. } | StmtKind::Refuse { text } => self.shown(text, place),
                StmtKind::Assign { target, value } if is_shown_property(target) => self.shown(value, place),
                StmtKind::If { branches, otherwise } => {
                    for branch in branches {
                        self.statements(&branch.body, place);
                    }
                    if let Some(body) = otherwise {
                        self.statements(body, place);
                    }
                }
                StmtKind::For { body, .. } => self.statements(body, place),
                _ => {}
            }
        }
    }

    fn shown(&mut self, expr: &Expr, place: &str) {
        match &expr.kind {
            ExprKind::Str { value } => self.add(value, place),
            ExprKind::Text { parts } => self.add(&template(parts), place),
            _ => {}
        }
    }
}

/// `part.caption` or `part.error`: a property whose text is shown.
fn is_shown_property(target: &Expr) -> bool {
    matches!(&target.kind, ExprKind::Member { object, member }
        if matches!(object.kind, ExprKind::Name { .. }) && matches!(member.name.as_str(), "caption" | "error"))
}

/// Every text a person sees in the project, each once, with where it is
/// used, in the order the project first uses them; the renderer's own
/// words last.
pub fn texts(project: &Project) -> Vec<Used> {
    let mut texts = Texts::default();
    for file in &project.files {
        for entity in &file.entities {
            let place = format!("entity {}", entity.name.name);
            for field in &entity.fields {
                texts.add(&field.label, &place);
                texts.some(&field.modifiers.help, &place);
            }
        }
        for decl in &file.enums {
            let place = format!("enum {}", decl.name.name);
            for variant in &decl.variants {
                texts.add(&variant.label, &place);
            }
        }
        for theme in &file.themes {
            texts.add(&theme.label, &format!("theme {}", theme.name.name));
        }
        for app in &file.apps {
            let place = format!("app {}", app.name.name);
            texts.some(&app.title, &place);
            texts.frame(&app.frame, &place);
        }
        for page in &file.pages {
            let place = format!("window {}", page.name.name);
            texts.some(&page.title, &place);
            texts.widgets(&page.widgets, &place);
            texts.frame(&page.frame, &place);
        }
        for block in &file.code {
            let place = format!("code {}", block.name.name);
            for handler in &block.handlers {
                texts.statements(&handler.body, &place);
            }
            for event in &block.events {
                texts.statements(&event.body, &place);
            }
        }
        for decl in &file.functions {
            let place = format!("{} {}", decl.kind.keyword(), decl.name.name);
            texts.statements(&decl.body, &place);
        }
    }
    let bound_picture = project.files.iter().flat_map(|f| &f.pages).any(|p| any_widget(&p.widgets, &|w| {
        w.props.kind == crate::widgets::WidgetKind::Image && w.props.bind.is_some()
    }));
    if bound_picture {
        for text in PICTURE_TEXTS {
            texts.add(text, "Clarity");
        }
    }
    for text in RENDERER_TEXTS {
        texts.add(text, "Clarity");
    }
    texts.used
}

/// One line of a translation file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub text: String,
    pub translation: String,
    pub span: Span,
}

/// A translation file: its language and its entries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Translation {
    pub language: String,
    pub entries: Vec<Entry>,
}

/// Reads a translation file.
pub fn parse(text: &str) -> (Translation, Vec<Diagnostic>) {
    let mut translation = Translation::default();
    let mut diagnostics = Vec::new();
    let (lines, lexed) = lex(text);
    diagnostics.extend(lexed);
    let mut lines = lines.into_iter();
    match lines.next() {
        Some(head) if head.indent == 0 && head.tokens.first().is_some_and(|t| t.text == "translation") => {
            // A language tag such as nl-NL lexes as several tokens.
            translation.language = text[head.tokens[1..].first().map_or(head.span.end, |t| t.span.start) as usize..head.span.end as usize]
                .trim()
                .to_string();
            if let Some(problem) = language_problem(&translation.language) {
                diagnostics.push(Diagnostic::error(problem, head.span));
            }
        }
        Some(head) => {
            diagnostics.push(Diagnostic::error("a translation file starts with 'translation' and its language", head.span));
            return (translation, diagnostics);
        }
        None => return (translation, diagnostics),
    }
    for line in lines {
        match line.tokens.as_slice() {
            [from, to] if from.kind == TokenKind::Str && to.kind == TokenKind::Str => translation.entries.push(Entry {
                text: from.text.clone(),
                translation: to.text.clone(),
                span: line.span,
            }),
            _ => diagnostics.push(Diagnostic::error(
                "write a text and its translation, each in quotes: \"Save\" \"Opslaan\"",
                line.span,
            )),
        }
    }
    (translation, diagnostics)
}

/// The slots of a text: `{customer.name}`, without the braces of `{{`.
fn slots(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find('{') {
        let after = &rest[at + 1..];
        if let Some(stripped) = after.strip_prefix('{') {
            rest = stripped;
            continue;
        }
        let Some(end) = after.find('}') else { break };
        found.push(after[..end].trim().to_string());
        rest = &after[end + 1..];
    }
    found
}

/// Reads the translation files `(path, text)` of a project, checks them
/// against the texts it shows, and hands them to the project for the
/// renderer.
pub fn attach<'a>(project: &mut Project, files: impl IntoIterator<Item = (&'a str, &'a str)>) {
    let used: Vec<String> = texts(project).into_iter().map(|u| u.text).collect();
    for (path, text) in files {
        let (translation, mut found) = parse(text);
        let named = Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if !translation.language.is_empty() && named != translation.language {
            found.push(Diagnostic::error(
                format!("this file is for language '{named}' (its name), but it says '{}'", translation.language),
                Span::default(),
            ));
        }
        let mut map = BTreeMap::new();
        for entry in &translation.entries {
            let quoted = quote(&entry.text);
            if map.insert(entry.text.clone(), entry.translation.clone()).is_some() {
                found.push(Diagnostic::error(format!("{quoted} is translated twice"), entry.span));
                continue;
            }
            if !used.contains(&entry.text) {
                found.push(Diagnostic::warning(
                    format!("{quoted} is no longer shown anywhere; its translation can go"),
                    entry.span,
                ));
            }
            let (from, to) = (slots(&entry.text), slots(&entry.translation));
            for slot in from.iter().filter(|s| !to.contains(s)) {
                found.push(Diagnostic::error(
                    format!("the translation of {quoted} must keep the slot {{{slot}}}"),
                    entry.span,
                ));
            }
            for slot in to.iter().filter(|s| !from.contains(s)) {
                found.push(Diagnostic::error(
                    format!("the translation of {quoted} has a slot {{{slot}}} that the text does not have"),
                    entry.span,
                ));
            }
        }
        project.diagnostics.extend(found.into_iter().map(|d| d.in_file(path)));
        project.translations.insert(named, map);
    }
}

/// The text of translation file `existing` (or a new one for `language`)
/// with `text` translated as `translation`: a changed line keeps its place
/// and the lines around it, a new one goes at the end, and an empty
/// translation removes the line.
pub fn set(existing: Option<&str>, language: &str, text: &str, translation: &str) -> String {
    let line = format!("    {} {}", quote(text), quote(translation));
    let source = existing.map_or_else(|| format!("translation {language}\n"), str::to_string);
    let (parsed, _) = parse(&source);
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();
    match parsed.entries.iter().find(|e| e.text == text) {
        Some(entry) => {
            let index = entry.span.line as usize - 1;
            if translation.is_empty() {
                lines.remove(index);
            } else {
                lines[index] = line;
            }
        }
        None if translation.is_empty() => {}
        None => lines.push(line),
    }
    lines.join("\n") + "\n"
}

/// One text in the Translations table: where it is used, and its
/// translation per language, in the order of `TranslationTable::languages`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct TranslationRow {
    pub text: String,
    pub places: Vec<String>,
    pub translations: Vec<Option<String>>,
}

/// Every text a person sees with its translations, for the Project tab.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct TranslationTable {
    /// The languages the project has translation files for.
    pub languages: Vec<String>,
    pub rows: Vec<TranslationRow>,
    /// How many texts each language does not translate yet.
    pub missing: Vec<usize>,
}

/// The Translations table of a project.
pub fn table(project: &Project) -> TranslationTable {
    let languages: Vec<String> = project.translations.keys().cloned().collect();
    let rows: Vec<TranslationRow> = texts(project)
        .into_iter()
        .map(|used| TranslationRow {
            translations: languages
                .iter()
                .map(|l| project.translations[l].get(&used.text).filter(|t| !t.is_empty()).cloned())
                .collect(),
            text: used.text,
            places: used.places,
        })
        .collect();
    let missing = (0..languages.len()).map(|i| rows.iter().filter(|r| r.translations[i].is_none()).count()).collect();
    TranslationTable { languages, rows, missing }
}

/// Whether any part in `widgets`, inside containers too, is one `test` picks.
fn any_widget(widgets: &[Widget], test: &impl Fn(&Widget) -> bool) -> bool {
    widgets.iter().any(|w| test(w) || any_widget(&w.children, test) || w.pages.iter().any(|p| any_widget(&p.children, test)))
}
