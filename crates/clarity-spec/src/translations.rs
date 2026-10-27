//! Translations by key (design section 1.5, I1c). Design and code name every
//! text people read by its key, `@saveCaption`, with its slots filled by
//! name in code: `@orderApproved(number = order.number)`. A file per
//! language in the module folder, `translations/nl.clarity`, gives each key
//! its value:
//!
//! ```text
//! translation nl
//!     saveCaption "Opslaan"
//!     orderApproved "Bestelling {number} is goedgekeurd"
//! ```
//!
//! Every language file has the same keys. The default language, the
//! project's own (`language "en"`), is such a file too and gives every key a
//! value; another language may leave one empty, which shows the default's
//! until it is translated. A text without letters, such as a calculator's
//! `7`, may be written as it is.

use std::collections::BTreeMap;
use std::path::Path;

use crate::code::{Expr, ExprKind, Stmt, StmtKind, each_expr_in};
use crate::diagnostic::Diagnostic;
use crate::formats::language_problem;
use crate::lexer::{TokenKind, lex, quote};
use crate::model::{Command, DataDecl, DataKind, FieldType, Frame, Literal, Project, Widget};
use crate::span::Span;

/// The folder of translation files in a project's module folder.
pub const TRANSLATIONS_DIR: &str = "translations";

/// The renderer's own words, by key, with their value in English: a bool as
/// it shows in a table or a list, the buttons of Clarity's dialogs, and
/// closing a message.
pub const RENDERER_KEYS: [(&str, &str); 5] = [("yes", "Yes"), ("no", "No"), ("ok", "OK"), ("cancel", "Cancel"), ("dismiss", "Dismiss")];

/// Clarity's words on the buttons of a wizard, the `steps` part.
pub const STEP_KEYS: [(&str, &str); 3] = [("back", "Back"), ("next", "Next"), ("finish", "Finish")];
/// Clarity's words heading the pickers of the click targets `language` and `theme`.
pub const LANGUAGE_PICKER_KEYS: [(&str, &str); 1] = [("languagePicker", "Language")];
pub const THEME_PICKER_KEYS: [(&str, &str); 1] = [("themePicker", "Theme")];
/// Clarity's words of a file part bound to a file field: its empty state,
/// its buttons, and what it says when a copy was saved.
pub const FILE_KEYS: [(&str, &str); 6] = [
    ("noFile", "No file"),
    ("chooseFile", "Choose a file"),
    ("openFile", "Open"),
    ("saveFileCopy", "Save a copy"),
    ("removeFile", "Remove the file"),
    ("fileCopySaved", "A copy is in your Downloads folder"),
];
/// Clarity's words of a tags part bound to a list field: the place a new
/// item is typed or picked, and the button that removes one.
pub const TAGS_KEYS: [(&str, &str); 2] = [("addTag", "Add"), ("removeTag", "Remove")];
/// Clarity's words of the bar under a table shown a page at a time: its
/// buttons to the first, the previous, the next and the last page.
pub const PAGE_KEYS: [(&str, &str); 4] = [("firstPage", "First page"), ("previousPage", "Previous page"), ("nextPage", "Next page"), ("lastPage", "Last page")];
/// Clarity's words of a tree's toggle, which shows or hides the rows under a row.
pub const TREE_KEYS: [(&str, &str); 2] = [("expandRow", "Expand"), ("collapseRow", "Collapse")];
/// What an image part bound to a picture shows of its own, by key: in its
/// empty state, and on its buttons to load a picture and to remove it.
pub const PICTURE_KEYS: [(&str, &str); 3] = [("noPicture", "No picture"), ("loadPicture", "Load a picture"), ("removePicture", "Remove the picture")];

/// Clarity's words of a command-line program (DESIGN 5.20): the headings of
/// its help, and what it says when a command line is wrong or an `ask`
/// has nobody to answer it. The runtimes fill the slots.
pub const CLI_KEYS: [(&str, &str); 20] = [
    ("cliUsage", "Usage"),
    ("cliCommands", "Commands"),
    ("cliOptions", "Options"),
    ("cliCommandWord", "command"),
    ("cliOptionsWord", "options"),
    ("cliHelpOption", "Shows this help"),
    ("cliDefault", "default {value}"),
    ("cliRequiredMark", "required"),
    ("cliUnknownCommand", "There is no command {command}"),
    ("cliUnknownOption", "There is no option {option}"),
    ("cliMissingArgument", "{argument} is missing"),
    ("cliTooManyArguments", "There are more values than arguments"),
    ("cliNeedsValue", "{option} needs a value"),
    ("cliWrongValue", "{name} cannot be {value}"),
    ("cliTooSmall", "{name} is at least {min}"),
    ("cliTooLarge", "{name} is at most {max}"),
    ("cliRequired", "{option} is required"),
    ("cliWrongAnswer", "{value} does not fit; try again"),
    ("cliNoTerminal", "There is nobody to answer: {question}"),
    ("cliChooseNumber", "Type a number from 1 to {count}"),
];

/// Clarity's words of a password input: its buttons that show what is
/// typed, hide it again, and copy it.
pub const SECRET_KEYS: [(&str, &str); 3] = [("showSecret", "Show"), ("hideSecret", "Hide"), ("copySecret", "Copy")];

/// Clarity's words of the keys window (DESIGN 5.28), where a person sets
/// the keys of the application's commands. The runtimes fill the slots.
pub const KEYS_WINDOW_KEYS: [(&str, &str); 15] = [
    ("keysTitle", "Keys"),
    ("keysEverywhere", "Everywhere"),
    ("keysChange", "Change"),
    ("keysReset", "Reset"),
    ("keysResetAll", "Reset all"),
    ("keysClose", "Close"),
    ("keysPress", "Press the new key; Escape stops"),
    ("keysNone", "None"),
    ("keysTaken", "{key} is the key of {command}. Swap them?"),
    ("keysSwap", "Swap"),
    ("keysAlso", "{key} is also the key of {command}, elsewhere; here {mine} goes first"),
    ("keysTyping", "{key} cannot be used: a key without Ctrl or Alt is typing"),
    ("keysTab", "{key} cannot be used: Tab moves between fields"),
    ("keysEnter", "{key} cannot be used: Enter confirms"),
    ("keysEscape", "{key} cannot be used: Escape cancels"),
];

/// Clarity's words for why the databases did not open (DESIGN 5.27): the
/// window that keeps the connection says it, and the app's `on unconnected`
/// gets it. The runtimes fill the slot with their own message.
pub const UNCONNECTED_KEYS: [(&str, &str); 2] = [("unconnectedNotKept", "The database connection is not set yet"), ("unconnectedFailed", "The database did not open: {problem}")];

/// Why an engine beside the screen stopped, which its canvas shows in place
/// of its drawing (DESIGN 5.12): the host's own message in `{problem}`.
pub const SCREEN_ENGINE_KEYS: [(&str, &str); 1] = [("screenEngineFailed", "The engine stopped: {problem}")];

/// Every list of Clarity's own words, with their English.
pub fn own_words() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
    RENDERER_KEYS
        .iter()
        .chain(&PICTURE_KEYS)
        .chain(&STEP_KEYS)
        .chain(&FILE_KEYS)
        .chain(&TAGS_KEYS)
        .chain(&PAGE_KEYS)
        .chain(&TREE_KEYS)
        .chain(&LANGUAGE_PICKER_KEYS)
        .chain(&THEME_PICKER_KEYS)
        .chain(&CLI_KEYS)
        .chain(&UNCONNECTED_KEYS)
        .chain(&SECRET_KEYS)
        .chain(&KEYS_WINDOW_KEYS)
        .chain(&SCREEN_ENGINE_KEYS)
}

/// The key a text names, without its `@`; none for a text written as it is.
pub fn key_of(text: &str) -> Option<&str> {
    text.strip_prefix('@')
}

/// Whether `text` has letters outside its slots. A text without, such as a
/// calculator's `7` or `+`, reads the same in every language.
pub fn has_letters(text: &str) -> bool {
    let mut outside_slots = true;
    text.chars().any(|c| {
        match c {
            '{' => outside_slots = false,
            '}' => outside_slots = true,
            _ => {}
        }
        outside_slots && c.is_alphabetic()
    })
}

/// A key the project uses, and where: `window Card`, `entity Customer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyUse {
    pub key: String,
    pub places: Vec<String>,
}

/// One use of a key, where it is written.
struct Use {
    key: String,
    place: String,
    path: String,
    span: Span,
    /// The slots code gives values to; none in the design, which gives none.
    args: Option<Vec<String>>,
}

/// Everything the project shows: the keys it uses, and the texts it writes
/// as they are where people read them.
#[derive(Default)]
struct Shown {
    uses: Vec<Use>,
    /// A text with letters written as it is: what it says, and where.
    literals: Vec<(String, String, Span)>,
    /// What shows a text people read without one: why, and where.
    unnamed: Vec<(String, String, Span)>,
    path: String,
    place: String,
}

impl Shown {
    /// A text in the design: a key, or a text as written.
    fn text(&mut self, text: &str, span: Span) {
        match key_of(text) {
            Some(key) => self.uses.push(Use { key: key.to_string(), place: self.place.clone(), path: self.path.clone(), span, args: None }),
            None if has_letters(text) => self.literals.push((text.to_string(), self.path.clone(), span)),
            None => {}
        }
    }

    fn some(&mut self, text: &Option<String>, span: Span) {
        if let Some(text) = text {
            self.text(text, span);
        }
    }

    fn widgets(&mut self, widgets: &[Widget]) {
        for widget in widgets {
            let p = &widget.props;
            self.some(&p.caption, widget.span);
            self.some(&p.placeholder, widget.span);
            self.some(&p.tooltip, widget.span);
            // A custom part's text properties are shown by its library.
            for value in p.values.values() {
                if let crate::model::Literal::Str(text) = value {
                    self.text(text, widget.span);
                }
            }
            if let Some(menu) = &widget.menu {
                self.commands(&menu.items);
            }
            self.widgets(&widget.children);
            for page in &widget.pages {
                self.some(&page.title, page.span);
                self.widgets(&page.children);
            }
        }
    }

    fn commands(&mut self, commands: &[Command]) {
        for command in commands {
            self.some(&command.props.caption, command.span);
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
            for tab in &ribbon.tabs {
                self.text(&tab.title, tab.span);
                for group in &tab.groups {
                    self.text(&group.title, group.span);
                    self.commands(&group.items);
                }
            }
        }
        for sidebar in &frame.sidebars {
            self.widgets(&sidebar.widgets);
        }
        if let Some(footer) = &frame.footer {
            self.some(&footer.status, footer.status_span.unwrap_or(footer.span));
            self.widgets(&footer.widgets);
        }
    }

    /// The text a value starts with may name a key: `data note: text, default @noteStart`.
    fn data(&mut self, data: &[DataDecl]) {
        for decl in data {
            if let DataKind::Value { default: Some(Literal::Str(text)), .. } = &decl.of
                && let Some(key) = key_of(text)
            {
                self.uses.push(Use { key: key.to_string(), place: self.place.clone(), path: self.path.clone(), span: decl.span, args: None });
            }
        }
    }

    /// A text outside statements that people read: a key, or a text as written.
    fn expr(&mut self, expr: Option<&Expr>) {
        let Some(expr) = expr else { return };
        if let ExprKind::Key { key, args } = &expr.kind {
            self.uses.push(Use {
                key: key.name.clone(),
                place: self.place.clone(),
                path: self.path.clone(),
                span: key.span,
                args: Some(args.iter().map(|a| a.name.name.clone()).collect()),
            });
        }
        self.written(expr);
    }

    /// Every key code names, with the slots it fills, and the texts it
    /// shows as written.
    fn code(&mut self, body: &[Stmt]) {
        let mut asked = Vec::new();
        each_expr_in(body, &mut |expr| {
            // A command's `ask` that gives its answer: its question and buttons.
            if let ExprKind::Ask { question, form } = &expr.kind {
                asked.push(question.as_ref().clone());
                if let crate::code::AskForm::Confirm { yes, no } = form.as_ref() {
                    asked.extend(yes.iter().chain(no).cloned());
                }
            }
            if let ExprKind::Key { key, args } = &expr.kind {
                self.uses.push(Use {
                    key: key.name.clone(),
                    place: self.place.clone(),
                    path: self.path.clone(),
                    span: key.span,
                    args: Some(args.iter().map(|a| a.name.name.clone()).collect()),
                });
            }
        });
        for text in &asked {
            self.written(text);
        }
        self.shown_as_written(body);
    }

    /// A message, a refusal, a caption or an error written as a text.
    fn shown_as_written(&mut self, body: &[Stmt]) {
        for stmt in body {
            match &stmt.kind {
                StmtKind::Message { text, .. } | StmtKind::Refuse { text } | StmtKind::Print { value: text, .. } => self.written(text),
                StmtKind::Ask { question, form, .. } => {
                    if let Some(question) = question {
                        self.written(question);
                    }
                    if let crate::code::AskForm::Confirm { yes, no } = form.as_ref() {
                        for text in yes.iter().chain(no) {
                            self.written(text);
                        }
                    }
                }
                StmtKind::Assign { target, value } if is_shown_property(target) => self.written(value),
                StmtKind::If { branches, otherwise } => {
                    for branch in branches {
                        self.shown_as_written(&branch.body);
                    }
                    if let Some(body) = otherwise {
                        self.shown_as_written(body);
                    }
                }
                StmtKind::For { body, .. } => self.shown_as_written(body),
                _ => {}
            }
        }
    }

    fn written(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Str { value } if has_letters(value) => self.literals.push((value.clone(), self.path.clone(), expr.span)),
            ExprKind::Text { parts } => {
                let text = crate::code::template(parts);
                if has_letters(&text) {
                    self.literals.push((text, self.path.clone(), expr.span));
                }
            }
            _ => {}
        }
    }
}

/// `part.caption` or `part.error`: a property whose text is shown.
fn is_shown_property(target: &Expr) -> bool {
    matches!(&target.kind, ExprKind::Member { object, member }
        if matches!(object.kind, ExprKind::Name { .. }) && matches!(member.name.as_str(), "caption" | "error"))
}

/// Every text the project shows, by key or as written.
/// The keys the code of an engine written in Clarity names, in order and
/// once each: what it gives people to read, as its `says` would list them
/// once it is a WebAssembly file (DESIGN 5.12, N5b).
pub fn engine_keys(body: &crate::code::EngineBody) -> Vec<String> {
    let mut shown = Shown::default();
    for decl in &body.consts {
        shown.expr(Some(&decl.value));
    }
    for decl in &body.functions {
        shown.code(&decl.body);
    }
    let mut keys: Vec<String> = Vec::new();
    for found in shown.uses {
        if !keys.contains(&found.key) {
            keys.push(found.key);
        }
    }
    keys
}

fn shown(project: &Project) -> Shown {
    let mut shown = Shown::default();
    for file in &project.files {
        shown.path = file.path.clone();
        for entity in &file.entities {
            shown.place = format!("entity {}", entity.name.name);
            for field in &entity.fields {
                match &field.modifiers.label {
                    Some(label) => shown.text(label, field.span),
                    None => shown.unnamed.push((
                        format!("field {} of {} shows its name as its label; give it a key: label @{}", field.name.name, entity.name.name, crate::names::camel_case(&format!("{} {}", entity.name.name, field.name.name))),
                        file.path.clone(),
                        field.span,
                    )),
                }
                shown.some(&field.modifiers.help, field.span);
            }
        }
        for decl in &file.enums {
            shown.place = format!("enum {}", decl.name.name);
            for variant in &decl.variants {
                match key_of(&variant.label) {
                    Some(_) => shown.text(&variant.label, variant.name.span),
                    None if has_letters(&variant.label) => shown.unnamed.push((
                        format!("{} of {} shows a text without a key; give it one: label @{}", variant.name.name, decl.name.name, crate::names::camel_case(&format!("{} {}", decl.name.name, variant.name.name))),
                        file.path.clone(),
                        variant.name.span,
                    )),
                    None => {}
                }
            }
        }
        for theme in &file.themes {
            shown.place = format!("theme {}", theme.name.name);
            match &theme.title {
                Some(title) => shown.text(title, theme.span),
                None if project.themes.contains(&theme.name.name) => shown.unnamed.push((
                        format!("theme {} is offered, so people read its name; give it a key: title @{}", theme.name.name, crate::names::camel_case(&format!("{} theme", theme.name.name))),
                    file.path.clone(),
                    theme.span,
                )),
                None => {}
            }
        }
        for app in &file.apps {
            shown.place = format!("app {}", app.name.name);
            shown.some(&app.title, app.span);
            shown.data(&app.data);
            shown.frame(&app.frame);
        }
        for page in &file.pages {
            shown.place = format!("window {}", page.name.name);
            shown.some(&page.title, page.span);
            shown.data(&page.data);
            shown.widgets(&page.widgets);
            shown.frame(&page.frame);
        }
        for block in &file.code {
            shown.place = format!("code {}", block.name.name);
            for handler in &block.handlers {
                shown.code(&handler.body);
            }
            for event in &block.events {
                shown.code(&event.body);
            }
        }
        for decl in &file.functions {
            shown.place = format!("{} {}", decl.kind.keyword(), decl.name.name);
            shown.code(&decl.body);
        }
        for cli in &file.clis {
            shown.place = format!("cli {}", cli.name.name);
            shown.expr(cli.help.as_ref());
        }
        for command in &file.commands {
            shown.place = format!("command {}", command.line());
            shown.expr(command.help.as_ref());
            for option in &command.options {
                shown.expr(option.help.as_ref());
            }
            shown.code(&command.body);
        }
        // A constant's keys are used; its plain text is not read where it is
        // written (a picture's name, a shape's op), so it is no literal.
        for decl in file.consts.iter().chain(file.engines.iter().filter_map(|e| e.body.as_ref()).flat_map(|b| &b.consts)) {
            let mut keys = Shown { place: format!("const {}", decl.name.name), path: shown.path.clone(), ..Shown::default() };
            keys.expr(Some(&decl.value));
            shown.uses.extend(keys.uses);
        }
        for engine in &file.engines {
            shown.place = format!("engine {}", engine.name.name);
            for text in &engine.says {
                shown.text(text, engine.span);
            }
            // An engine written in Clarity names keys in its code as functions do.
            for decl in engine.body.iter().flat_map(|b| &b.functions) {
                shown.place = format!("engine {}, function {}", engine.name.name, decl.name.name);
                shown.code(&decl.body);
            }
        }
    }
    // Clarity's own words.
    shown.place = "Clarity".to_string();
    shown.path = project.files.first().map(|f| f.path.clone()).unwrap_or_default();
    let bound_picture = project.files.iter().flat_map(|f| &f.pages).any(|p| any_widget(&p.widgets, &|w| {
        w.props.kind == crate::widgets::WidgetKind::Image && w.props.bind.is_some()
    }));
    let wizard = project.files.iter().flat_map(|f| &f.pages).any(|p| any_widget(&p.widgets, &|w| w.props.kind == crate::widgets::WidgetKind::Steps));
    let bound_file = project.files.iter().flat_map(|f| &f.pages).any(|p| any_widget(&p.widgets, &|w| {
        w.props.kind == crate::widgets::WidgetKind::File && w.props.bind.is_some()
    }));
    let paged = project.files.iter().flat_map(|f| &f.pages).any(|p| any_widget(&p.widgets, &|w| w.props.page.is_some()));
    let trees = project.files.iter().flat_map(|f| &f.pages).any(|p| any_widget(&p.widgets, &|w| matches!(w.props.kind, crate::widgets::WidgetKind::Tree | crate::widgets::WidgetKind::TreeTable)));
    let bound_tags = project.files.iter().flat_map(|f| &f.pages).any(|p| any_widget(&p.widgets, &|w| {
        w.props.kind == crate::widgets::WidgetKind::Tags && w.props.bind.is_some()
    }));
    // The pickers' headings, which also label the settings' own `language` and `theme`.
    let setting = |ty: FieldType| project.entities().any(|e| e.settings && e.fields.iter().any(|f| f.ty == ty));
    let languages = crate::frame::clicks_to(project, "language") || setting(FieldType::Language);
    let themes = crate::frame::clicks_to(project, "theme") || setting(FieldType::Theme);
    let commands = project.files.iter().any(|f| !f.commands.is_empty());
    let unconnected = !project.secret_connections().is_empty() || project.handles_unconnected();
    let keys_window = crate::frame::clicks_to(project, "keys");
    let screen_engines = project.files.iter().flat_map(|f| &f.engines).any(|e| e.beside);
    let passwords = project.files.iter().flat_map(|f| &f.pages).any(|p| any_widget(&p.widgets, &|w| w.props.kind == crate::widgets::WidgetKind::Password));
    let own = RENDERER_KEYS
        .iter()
        .chain(if bound_picture { PICTURE_KEYS.as_slice() } else { &[] })
        .chain(if wizard { STEP_KEYS.as_slice() } else { &[] })
        .chain(if bound_file { FILE_KEYS.as_slice() } else { &[] })
        .chain(if bound_tags { TAGS_KEYS.as_slice() } else { &[] })
        .chain(if paged { PAGE_KEYS.as_slice() } else { &[] })
        .chain(if trees { TREE_KEYS.as_slice() } else { &[] })
        .chain(if languages { LANGUAGE_PICKER_KEYS.as_slice() } else { &[] })
        .chain(if themes { THEME_PICKER_KEYS.as_slice() } else { &[] })
        .chain(if commands { CLI_KEYS.as_slice() } else { &[] })
        .chain(if unconnected { UNCONNECTED_KEYS.as_slice() } else { &[] })
        .chain(if passwords { SECRET_KEYS.as_slice() } else { &[] })
        .chain(if keys_window { KEYS_WINDOW_KEYS.as_slice() } else { &[] })
        .chain(if screen_engines { SCREEN_ENGINE_KEYS.as_slice() } else { &[] });
    // The runtimes fill the slots of Clarity's own words, the ones its English has.
    for (key, english) in own {
        shown.uses.push(Use { key: key.to_string(), place: "Clarity".into(), path: String::new(), span: Span::default(), args: Some(slots(english)) });
    }
    shown
}

/// Every key the project uses, each once, in the order the project first
/// uses them, with where; Clarity's own words last.
pub fn keys(project: &Project) -> Vec<KeyUse> {
    let mut keys: Vec<KeyUse> = Vec::new();
    for found in shown(project).uses {
        match keys.iter_mut().find(|k| k.key == found.key) {
            Some(k) if !k.places.contains(&found.place) => k.places.push(found.place),
            Some(_) => {}
            None => keys.push(KeyUse { key: found.key, places: vec![found.place] }),
        }
    }
    keys
}

/// The value of a key in a language: a text, or a file of the resources
/// folder it points at (`gameRules resource "rules-nl.md"`), such as a
/// Markdown file a markdown part shows or a picture an image shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Text(String),
    Resource(String),
}

/// One line of a translation file: a key and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub value: Value,
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
            [key, value] if key.kind == TokenKind::Ident && value.kind == TokenKind::Str => {
                translation.entries.push(Entry { key: key.text.clone(), value: Value::Text(value.text.clone()), span: line.span })
            }
            [key, word, file] if key.kind == TokenKind::Ident && word.text == "resource" && file.kind == TokenKind::Str => {
                translation.entries.push(Entry { key: key.text.clone(), value: Value::Resource(file.text.clone()), span: line.span })
            }
            _ => diagnostics.push(Diagnostic::error(
                "write a key and its text in quotes, saveCaption \"Save\", or the resource it points at, gameRules resource \"rules.md\"",
                line.span,
            )),
        }
    }
    (translation, diagnostics)
}

/// The slots of a text: `{name}`, without the braces of `{{`.
pub fn slots(text: &str) -> Vec<String> {
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

/// The language the project is written in: its texts' default.
pub fn default_language(project: &Project) -> String {
    project.locale.language.clone().unwrap_or_else(|| "en".to_string())
}

/// Reads the translation files `(path, text)` of a project, checks them and
/// the keys the project uses against each other, and hands them to the
/// project for the renderer.
pub fn attach<'a>(project: &mut Project, files: impl IntoIterator<Item = (&'a str, &'a str)>) {
    let default = default_language(project);
    let mut found = Vec::new();
    // Each file's entries, by language, with its path.
    let mut read: Vec<(String, String, Translation)> = Vec::new();
    for (path, text) in files {
        let (translation, diagnostics) = parse(text);
        found.extend(diagnostics.into_iter().map(|d| d.in_file(path)));
        let named = Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if !translation.language.is_empty() && named != translation.language {
            found.push(
                Diagnostic::error(format!("this file is for language '{named}' (its name), but it says '{}'", translation.language), Span::default())
                    .in_file(path),
            );
        }
        let mut map = BTreeMap::new();
        let mut resources = BTreeMap::new();
        for entry in &translation.entries {
            if !crate::names::is_camel_case(&entry.key) {
                found.push(Diagnostic::error(format!("key '{}' is not camelCase, as names are", entry.key), entry.span).in_file(path));
            }
            let twice = match &entry.value {
                Value::Text(text) => map.insert(entry.key.clone(), text.clone()).is_some() || resources.contains_key(&entry.key),
                Value::Resource(file) => resources.insert(entry.key.clone(), file.clone()).is_some() || map.contains_key(&entry.key),
            };
            if twice {
                found.push(Diagnostic::error(format!("key '{}' is in this file twice", entry.key), entry.span).in_file(path));
            }
        }
        project.translations.insert(named.clone(), map);
        if !resources.is_empty() {
            project.resource_keys.insert(named.clone(), resources);
        }
        read.push((named, path.to_string(), translation));
    }

    let shown = shown(project);
    for (text, path, span) in &shown.literals {
        found.push(Diagnostic::error(format!("{} is read by people, so it is a key: write @key, with its text in the language files", quote(text)), *span).in_file(path));
    }
    for (problem, path, span) in &shown.unnamed {
        found.push(Diagnostic::error(problem.clone(), *span).in_file(path));
    }

    let defaults = read.iter().find(|(language, _, _)| *language == default);
    if defaults.is_none() && !shown.uses.iter().all(|u| u.place == "Clarity") {
        let at = shown.uses.iter().find(|u| u.place != "Clarity").expect("a use outside Clarity");
        found.push(Diagnostic::error(format!("the default language {default} has no file; add translations/{default}.clarity"), at.span).in_file(&at.path));
    }
    let values = project.translations.get(&default).cloned().unwrap_or_default();
    if defaults.is_some() {
        // Every key used has a value in the default language, and gets the
        // slots that value has.
        for u in &shown.uses {
            let place = |d: Diagnostic| if u.path.is_empty() { d.in_file(&defaults.map(|(_, p, _)| p.clone()).unwrap_or_default()) } else { d.in_file(&u.path) };
            // A key that points at a resource has its value: the resources check the file.
            if project.resource_keys.get(&default).is_some_and(|r| r.contains_key(&u.key)) {
                continue;
            }
            match values.get(&u.key).filter(|v| !v.trim().is_empty()) {
                // Clarity knows its own words: until the language file has one, it shows Clarity's English.
                None if u.place == "Clarity" => {
                    let english = own_words().find(|(k, _)| *k == u.key).map_or("", |(_, v)| *v);
                    found.push(place(Diagnostic::warning(
                        format!("Clarity's word '{}' is not in {default} yet; it shows \"{english}\" until it is", u.key),
                        u.span,
                    )));
                }
                None => found.push(place(Diagnostic::error(format!("key '{}' has no value in {default}, the default language", u.key), u.span))),
                Some(value) => {
                    let wanted = slots(value);
                    let given = u.args.clone().unwrap_or_default();
                    for slot in wanted.iter().filter(|s| !given.contains(s)) {
                        found.push(place(Diagnostic::error(format!("@{} needs a value for {{{slot}}}", u.key), u.span)));
                    }
                    for slot in given.iter().filter(|s| !wanted.contains(s)) {
                        found.push(place(Diagnostic::error(format!("@{} has no slot {{{slot}}}", u.key), u.span)));
                    }
                }
            }
        }
    }
    if let Some((_, default_path, default_file)) = defaults {
        // A key nobody uses can go.
        for entry in &default_file.entries {
            if !shown.uses.iter().any(|u| u.key == entry.key) {
                found.push(Diagnostic::warning(format!("key '{}' is not used anywhere; it can go", entry.key), entry.span).in_file(default_path));
            }
        }
        // Every other language has the same keys, and keeps the slots of each value.
        for (language, path, translation) in read.iter().filter(|(l, _, _)| *l != default) {
            for entry in &default_file.entries {
                if !translation.entries.iter().any(|e| e.key == entry.key) {
                    found.push(Diagnostic::error(format!("key '{}' is missing in {language}; every language file has the same keys", entry.key), Span::default()).in_file(path));
                }
            }
            for entry in &translation.entries {
                if !default_file.entries.iter().any(|e| e.key == entry.key) {
                    found.push(Diagnostic::error(format!("key '{}' is in {language} but not in {default}, the default language", entry.key), entry.span).in_file(path));
                    continue;
                }
                // Slots are for texts, in both languages.
                let (Some(value), Value::Text(text)) = (values.get(&entry.key), &entry.value) else { continue };
                if text.is_empty() {
                    continue;
                }
                let (from, to) = (slots(value), slots(text));
                for slot in from.iter().filter(|s| !to.contains(s)) {
                    found.push(Diagnostic::error(format!("{} in {language} must keep the slot {{{slot}}}", entry.key), entry.span).in_file(path));
                }
                for slot in to.iter().filter(|s| !from.contains(s)) {
                    found.push(Diagnostic::error(format!("{} in {language} has a slot {{{slot}}} that {default} does not have", entry.key), entry.span).in_file(path));
                }
            }
        }
    }
    project.diagnostics.extend(found);
}

/// The text of translation file `existing` (or a new one for `language`)
/// with `key` given `value`: a changed line keeps its place and the lines
/// around it, a new key goes at the end. An empty value stays, as "": every
/// language file has the same keys.
pub fn set(existing: Option<&str>, language: &str, key: &str, value: &str) -> String {
    set_value(existing, language, key, &Value::Text(value.to_string()))
}

/// As `set`, for a text or a resource the key points at.
pub fn set_value(existing: Option<&str>, language: &str, key: &str, value: &Value) -> String {
    let line = match value {
        Value::Text(text) => format!("    {key} {}", quote(text)),
        Value::Resource(file) => format!("    {key} resource {}", quote(file)),
    };
    let source = existing.map_or_else(|| format!("translation {language}\n"), str::to_string);
    let (parsed, _) = parse(&source);
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();
    match parsed.entries.iter().find(|e| e.key == key) {
        Some(entry) => lines[entry.span.line as usize - 1] = line,
        None => lines.push(line),
    }
    lines.join("\n") + "\n"
}

/// One key in the languages table: where it is used, and its value per
/// language, in the order of `TranslationTable::languages`; none where a
/// language leaves it empty.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct TranslationRow {
    pub key: String,
    pub places: Vec<String>,
    /// What kind of value the key takes, by the parts that show it.
    pub takes: Takes,
    /// Its text per language; none where a language leaves it empty or points it at a file.
    pub values: Vec<Option<String>>,
    /// The file of the resources each language points it at, if any.
    pub resources: Vec<Option<String>>,
}

/// What a key's value is in every language: a text, or a file of the
/// resources, as the part that shows it needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Takes {
    Text,
    /// A Markdown file, for a markdown part.
    Markdown,
    /// A picture, for an image.
    Picture,
}

/// What each key a markdown or an image shows takes; every other key takes text.
fn takes(project: &Project) -> BTreeMap<String, Takes> {
    fn walk(widgets: &[Widget], found: &mut BTreeMap<String, Takes>) {
        for w in widgets {
            let kind = match w.props.kind {
                crate::widgets::WidgetKind::Markdown => Some(Takes::Markdown),
                crate::widgets::WidgetKind::Image => Some(Takes::Picture),
                _ => None,
            };
            if let (Some(kind), Some(key)) = (kind, w.props.caption.as_deref().and_then(key_of)) {
                found.insert(key.to_string(), kind);
            }
            walk(&w.children, found);
            for page in &w.pages {
                walk(&page.children, found);
            }
        }
    }
    let mut found = BTreeMap::new();
    for page in project.files.iter().flat_map(|f| &f.pages) {
        walk(&page.widgets, &mut found);
    }
    found
}

/// Every key the project uses with its value per language, for i18n.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct TranslationTable {
    /// The default language: the project's own, which gives every key a value.
    pub default: String,
    /// The languages the project has files for, the default first.
    pub languages: Vec<String>,
    pub rows: Vec<TranslationRow>,
    /// How many keys each language leaves empty.
    pub missing: Vec<usize>,
}

/// The languages table of a project.
pub fn table(project: &Project) -> TranslationTable {
    let default = default_language(project);
    let mut languages: Vec<String> = project.translations.keys().filter(|l| **l != default).cloned().collect();
    if project.translations.contains_key(&default) {
        languages.insert(0, default.clone());
    }
    let kinds = takes(project);
    let rows: Vec<TranslationRow> = keys(project)
        .into_iter()
        .map(|used| TranslationRow {
            values: languages.iter().map(|l| project.translations[l].get(&used.key).filter(|v| !v.is_empty()).cloned()).collect(),
            resources: languages.iter().map(|l| project.resource_keys.get(l).and_then(|r| r.get(&used.key)).cloned()).collect(),
            takes: kinds.get(&used.key).copied().unwrap_or(Takes::Text),
            key: used.key,
            places: used.places,
        })
        .collect();
    let missing = (0..languages.len()).map(|i| rows.iter().filter(|r| r.values[i].is_none() && r.resources[i].is_none()).count()).collect();
    TranslationTable { default, languages, rows, missing }
}

/// Whether any part in `widgets`, inside containers too, is one `test` picks.
fn any_widget(widgets: &[Widget], test: &impl Fn(&Widget) -> bool) -> bool {
    widgets.iter().any(|w| test(w) || any_widget(&w.children, test) || w.pages.iter().any(|p| any_widget(&p.children, test)))
}
