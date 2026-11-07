//! Each window's view model (DESIGN 7.9): the window's data, the state of its
//! named parts (`PartState`), and its handlers as methods, run as commands.
//! Clarity.UI draws the window from its markup (`Windows/*.cxaml`, written
//! by `web_windows`) over this view model.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use clarity_spec::Project;
use clarity_spec::code::check::FileTypes;
use clarity_spec::frame::{ShownFrame, shown};
use clarity_spec::model::{App, Command, DataDecl, DataKind, Entity, Field, FieldType, Handler, Literal, Page, PageKind, Widget};
use clarity_spec::widgets::{Event, WidgetKind};

use super::HEADER;
use super::logic::{self, PartInfo, WindowCtx};
use super::names::{backing, command_method, member, on, pascal, property, string};
use super::texts::Texts;
use crate::OutFile;

/// The built-in click targets that are no window: `quit` ends the
/// application, `close` closes the window; `language` and `theme` pick in
/// place, which the screen does (Clarity.UI).
const QUIT: &str = "quit";
const CLOSE: &str = "close";
const KEYS: &str = "keys";
const UNDO: &str = "undo";
const REDO: &str = "redo";


/// The view model of `page`.
pub fn emit(project: &Project, page: &Page, types: &FileTypes, namespace: &str, texts: &Texts) -> Result<Vec<OutFile>, Vec<String>> {
    let app = project.apps().next();
    let frame = shown(app, page);
    let window = Window::new(project, page, app, frame, texts);
    let name = &page.name.name;
    let view_model = window.view_model(types, namespace)?;
    Ok(vec![text(&format!("Windows/{name}ViewModel.cs"), view_model)])
}

fn text(path: &str, body: String) -> OutFile {
    OutFile {
        path: path.to_string(),
        text: body,
        bytes: None,
    }
}

/// Every part of a list of parts, nested ones included.
fn all_parts(widgets: &[Widget]) -> Vec<&Widget> {
    let mut out = Vec::new();
    for w in widgets {
        out.push(w);
        out.extend(all_parts(&w.children));
        for page in &w.pages {
            out.extend(all_parts(&page.children));
        }
    }
    out
}

fn all_commands(commands: &[Command]) -> Vec<&Command> {
    let mut out = Vec::new();
    for c in commands {
        out.push(c);
        out.extend(all_commands(&c.items));
    }
    out
}

/// The property of the generated Icons class that holds an icon: `triangle-alert` is `TriangleAlert`.
pub(super) fn icon_member(name: &str) -> String {
    pascal(name)
}

/// The field a first text field shows for a record in a dropdown.
pub(super) fn display(entity: &Entity) -> String {
    entity
        .fields
        .iter()
        .find(|f| matches!(f.ty, FieldType::Text { .. } | FieldType::Email))
        .map_or_else(|| "r.Id.ToString()".to_string(), |f| format!("r.{}", property(&entity.name.name, &f.name.name)))
}

/// The view model's list of the values a typed dropdown's field has: `CustomerCityValues`.
fn typed_values(entity: &Entity, field: &Field) -> String {
    format!("{}{}Values", entity.name.name, pascal(&field.name.name))
}

/// The view model's records a search dropdown picks from, with the fields it shows: `CustomerNameCityChoices`.
fn searched_choices(entity: &Entity, show: &[String]) -> String {
    format!("{}{}Choices", entity.name.name, show.iter().map(|f| pascal(f)).collect::<String>())
}

struct Window<'a> {
    project: &'a Project,
    page: &'a Page,
    app: Option<&'a App>,
    frame: ShownFrame<'a>,
    /// The window's records and lists with the app's: name, entity and whether it is a list.
    data: Vec<(String, String, bool)>,
    /// The window's plain values with the app's: `data shown: decimal`.
    values: Vec<&'a DataDecl>,
    /// Every part the window shows: its grid, nested parts, the sidebars and the footer.
    parts: Vec<&'a Widget>,
    commands: Vec<&'a Command>,
    texts: &'a Texts,
    /// The calls with values its parts wire, each with the command that
    /// makes it: `digit(7)` with `Digit7`.
    calls: Vec<(&'a Handler, String)>,
    /// The members of its parts and data (`RedInk` for a part `redInk`), which a handler's command steps around.
    members: BTreeSet<String>,
}

/// The command of a call with values, named after it: `digit(7)` as
/// `Digit7`, `scale(0.5, "half")` as `Scale0_5Half`.
fn call_command(h: &Handler) -> String {
    use clarity_spec::model::Literal;
    let mut name = pascal(&h.handler);
    for value in &h.args {
        let part = match value {
            Literal::Number(n) => n.replace('-', "Minus").replace('.', "_"),
            Literal::Str(text) => {
                let words: String = text.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
                let word = pascal(&words);
                if word.is_empty() { "Text".into() } else { word }
            }
            Literal::Bool(b) => if *b { "True" } else { "False" }.into(),
            Literal::Ident(v) => pascal(v),
        };
        name.push_str(&part);
    }
    name
}

impl<'a> Window<'a> {
    fn new(project: &'a Project, page: &'a Page, app: Option<&'a App>, frame: ShownFrame<'a>, texts: &'a Texts) -> Self {
        let mut data: Vec<(String, String, bool)> = Vec::new();
        let mut values: Vec<&DataDecl> = Vec::new();
        for d in page.data.iter().chain(app.map(|a| a.data.as_slice()).unwrap_or_default()) {
            let seen = data.iter().any(|(n, _, _)| *n == d.name.name) || values.iter().any(|v| v.name.name == d.name.name);
            if seen {
                continue;
            }
            match d.entity() {
                Some(entity) => data.push((d.name.name.clone(), entity.name.clone(), d.is_list())),
                None => values.push(d),
            }
        }
        let mut parts = all_parts(&page.widgets);
        for sidebar in [frame.left, frame.right].into_iter().flatten() {
            parts.extend(all_parts(&sidebar.widgets));
        }
        if let Some(footer) = frame.footer {
            parts.extend(all_parts(&footer.widgets));
        }
        let mut commands = Vec::new();
        if let Some(menu) = frame.menu {
            commands.extend(all_commands(&menu.items));
        }
        if let Some(toolbar) = frame.toolbar {
            commands.extend(all_commands(&toolbar.items));
        }
        if let Some(ribbon) = frame.ribbon {
            for group in ribbon.tabs.iter().flat_map(|t| &t.groups) {
                commands.extend(all_commands(&group.items));
            }
        }
        // The items of the parts' context menus are commands as the frame's are.
        for menu in parts.iter().filter_map(|w| w.menu.as_ref()) {
            commands.extend(all_commands(&menu.items));
        }
        // The view model's members of its parts and data, which no command may be called.
        let mut members: BTreeSet<String> = parts.iter().filter_map(|w| w.props.name.as_deref()).map(member).collect();
        members.extend(commands.iter().filter_map(|c| c.props.name.as_deref()).map(member));
        members.extend(frame.menu.and_then(|m| m.name.as_deref()).map(member));
        members.extend(frame.toolbar.and_then(|t| t.name.as_deref()).map(member));
        if let Some(ribbon) = frame.ribbon {
            members.extend(ribbon.name.as_deref().map(member));
            members.extend(ribbon.tabs.iter().flat_map(|t| t.name.iter().chain(t.groups.iter().filter_map(|g| g.name.as_ref()))).map(|n| member(n)));
        }
        members.extend([frame.left, frame.right].into_iter().flatten().filter_map(|s| s.props.name.as_deref()).map(member));
        members.extend(data.iter().map(|(n, _, _)| member(n)));
        members.extend(values.iter().map(|d| member(&d.name.name)));
        // A command for each call with values, named after it; a name that
        // is taken already, by a handler or a part, gets a number.
        let mut taken: BTreeSet<String> = project.files.iter().flat_map(|f| &f.code).flat_map(|b| &b.handlers).map(|h| pascal(&h.name.name)).collect();
        taken.extend(members.iter().cloned());
        let mut calls = Vec::new();
        for h in crate::wired_calls(page) {
            let base = call_command(h);
            let mut name = base.clone();
            let mut n = 2;
            while !taken.insert(name.clone()) {
                name = format!("{base}{n}");
                n += 1;
            }
            calls.push((h, name));
        }
        Window {
            project,
            page,
            app,
            frame,
            data,
            values,
            parts,
            commands,
            texts,
            calls,
            members,
        }
    }

    /// A value of `entity.field`, written `value`, as the person reads it:
    /// an enum by its label, the rest in the person's format or the field's own.
    fn shown_text(&self, entity: &Entity, field: &str, value: &str) -> String {
        match entity.field(field) {
            Some(f) if matches!(f.ty, FieldType::Enum { .. }) => format!("Labels.Of({value})"),
            Some(f) => match super::field_format(f) {
                Some(format) => format!("ShownText.Of({value}, {format})"),
                None => format!("ShownText.Of({value})"),
            },
            None => format!("ShownText.Of({value})"),
        }
    }

    fn entity_of(&self, data: &str) -> Option<&'a Entity> {
        let (_, entity, _) = self.data.iter().find(|(n, _, _)| n == data)?;
        self.project.entity(entity)
    }

    fn bound_field(&self, bind: &str) -> Option<(&'a Entity, &'a Field)> {
        let (data, field) = bind.split_once('.')?;
        let entity = self.entity_of(data)?;
        Some((entity, entity.field(field)?))
    }

    /// `Field = ("order", "customer_id")` for an input bound to a required
    /// field: the data that holds the record and the field's column.
    fn required_field(&self, widget: &Widget) -> Option<String> {
        let bind = widget.props.bind.as_deref()?;
        let (data, field) = bind.split_once('.')?;
        let entity = self.entity_of(data)?;
        let column = entity.column_of(entity.field(field)?.name.name.as_str());
        crate::required_fields(self.project, entity)
            .iter()
            .any(|(c, _)| *c == column)
            .then(|| format!("Field = ({}, {})", string(data), string(&column)))
    }

    /// A binding as the AXAML path: `order.number` as `Order.Number`.
    fn path(&self, bind: &str) -> String {
        match bind.split_once('.') {
            Some((data, field)) => {
                let entity = self.entity_of(data).map(|e| e.name.name.clone()).unwrap_or_default();
                format!("{}.{}", member(data), property(&entity, field))
            }
            None => member(bind),
        }
    }

    /// The part a label or help text is for, by name.
    fn part(&self, name: &str) -> Option<&'a Widget> {
        self.parts.iter().copied().find(|w| w.props.name.as_deref() == Some(name))
    }

    /// The handler of `widget` for `event`.
    fn handler(widget: &Widget, event: Event) -> Option<&Handler> {
        widget.props.handlers.iter().find(|h| h.event == event)
    }

    /// The help a named input shows while it has no error: a help part's
    /// caption, or its field's help.
    fn help_of(&self, input: &str) -> String {
        let help = self.parts.iter().find(|w| w.props.kind == WidgetKind::Help && w.props.for_part.as_deref() == Some(input));
        if let Some(caption) = help.and_then(|h| h.props.caption.as_ref()) {
            return caption.clone();
        }
        self.part(input)
            .and_then(|w| w.props.bind.as_deref())
            .and_then(|b| self.bound_field(b))
            .and_then(|(_, f)| f.modifiers.help.clone())
            .unwrap_or_default()
    }

    /// Where a part's edits are undone (DESIGN 5.29): its form or panel, or itself; none without undo.
    fn undo_scope(&self, p: &clarity_spec::model::WidgetProps) -> Option<String> {
        p.undo_scope.clone()
    }

    fn view_model(&self, types: &FileTypes, namespace: &str) -> Result<String, Vec<String>> {
        let name = &self.page.name.name;
        let mut errors = Vec::new();
        let mut ctx = WindowCtx {
            data: self.data.clone(),
            ..WindowCtx::default()
        };
        for widget in &self.parts {
            if let Some(n) = &widget.props.name {
                let custom = crate::parts::kept_of(&widget.props);
                // A canvas of objects chooses a record of its list, as a table of its bound list does.
                let bind = widget.props.objects.as_ref().map(|o| o.list.clone()).or_else(|| widget.props.bind.clone());
                ctx.parts.insert(n.clone(), PartInfo { bind, custom, mode: widget.props.mode });
            }
        }
        for command in &self.commands {
            if let Some(n) = &command.props.name {
                ctx.parts.insert(n.clone(), PartInfo::default());
            }
        }

        // The constructor loads every list and the records of references.
        let mut constructor = String::new();
        // A text value that starts at a key shows again when the language changes.
        let mut said = String::new();
        let mut properties = String::new();
        for (data, entity, list) in &self.data {
            let prop = member(data);
            let field = backing(data);
            if *list {
                // A list of records starts empty: code builds it (DESIGN 5.12).
                if !self.project.entity(entity).is_some_and(|e| e.record) {
                    let _ = writeln!(constructor, "        {field} = Db.All<{entity}>();");
                }
                let _ = write!(
                    properties,
                    "    public List<{entity}> {prop} {{ get => {field}; set => SetProperty(ref {field}, value); }}\n    List<{entity}> {field} = [];\n\n"
                );
            } else if self.project.entity(entity).is_some_and(|e| e.settings) {
                // Every window shows the same settings.
                let _ = write!(properties, "    public {entity} {prop} => {entity}.Current;\n\n");
                // A part bound to a setting changes it: its edit takes it along for Ctrl+Z (DESIGN 5.29).
                let _ = write!(properties, "    protected override ISettings? AppSettings => {entity}.Current;\n\n");
            } else {
                let _ = write!(
                    properties,
                    "    public {entity} {prop} {{ get => {field}; set => SetProperty(ref {field}, value); }}\n    {entity} {field} = new();\n\n"
                );
            }
        }
        // A plain value starts at its default, or empty.
        for d in &self.values {
            let DataKind::Value { ty, default } = &d.of else { continue };
            let info = logic::type_ref_info(self.project, ty);
            let ty_name = super::names::value_type(&info);
            let start = match (default, &info) {
                // A text starts in the person's language, as a label shows.
                (Some(Literal::Str(text)), _) => self.texts.code(text),
                (Some(value), _) => logic::literal(self.project, ty, value),
                (None, clarity_spec::code::check::TypeInfo::Text) => "\"\"".into(),
                // A list of values starts empty (DESIGN 5.26).
                (None, clarity_spec::code::check::TypeInfo::Values(_)) => "[]".into(),
                (None, _) => "default".into(),
            };
            let (prop, field) = (member(&d.name.name), backing(&d.name.name));
            if let Some(Literal::Str(text)) = default
                && let Some(key) = self.texts.key(text)
            {
                let _ = writeln!(said, "        ShowText(nameof({prop}), Text.Of({}), shown => {prop} = shown);", string(&key));
            }
            let _ = write!(properties, "    public {ty_name} {prop} {{ get => {field}; set => SetProperty(ref {field}, value); }}\n    {ty_name} {field} = {start};\n\n");
        }
        // The picture each picture field shows when the record has one stored.
        let mut fetches: BTreeMap<String, (&str, String)> = BTreeMap::new();
        // A painting canvas paints on its field's picture (DESIGN 5.31).
        let paints = |w: &&&Widget| w.props.kind == WidgetKind::Canvas && w.props.mode == Some(clarity_spec::model::CanvasMode::Paint);
        for widget in self.parts.iter().filter(|w| matches!(w.props.kind, WidgetKind::Image | WidgetKind::File) || paints(w)) {
            let Some((data, field)) = widget.props.bind.as_deref().and_then(|b| b.split_once('.')) else { continue };
            let Some(entity) = self.entity_of(data) else { continue };
            // Through Fetched, as every picture of the database comes.
            let fetcher = picture_fetch(data, field);
            fetches.insert(
                fetcher.clone(),
                ("Task<byte[]?>", format!("() => Fetched(nameof({fetcher}), () => Db.PictureOf<{}>({}.Id, {}))", entity.name.name, member(data), string(&entity.column_of(field)))),
            );
            if let Some(store) = entity.field(field).and_then(|f| f.modifiers.store.as_deref()) {
                let fetcher = picture_file(data, field);
                fetches.insert(fetcher.clone(), ("string, Task<byte[]?>", format!("path => Fetched(nameof({fetcher}), () => Db.StoreFile({}, path), path)", string(store))));
            }
        }
        for (name, (ty, fetch)) in &fetches {
            let _ = write!(properties, "    public Func<{ty}> {name} => {fetch};\n\n");
        }
        let mut choices: BTreeMap<String, String> = BTreeMap::new();
        let mut referenced: Vec<&FieldType> = Vec::new();
        for widget in &self.parts {
            let p = &widget.props;
            match p.kind {
                WidgetKind::Dropdown => referenced.extend(p.bind.as_deref().and_then(|b| self.bound_field(b)).map(|(_, f)| &f.ty)),
                // A chart by a reference shows the texts of the records it names.
                WidgetKind::Chart => {
                    let Some(entity) = p.bind.as_deref().and_then(|b| self.entity_of(b)) else { continue };
                    referenced.extend(p.by.as_deref().and_then(|by| entity.field(by)).map(|f| &f.ty));
                }
                // A reference edited in a table cell picks from the same choices.
                WidgetKind::Table => {
                    let Some(entity) = p.bind.as_deref().and_then(|b| self.entity_of(b)) else { continue };
                    referenced.extend(p.edit.iter().filter_map(|c| entity.field(c)).map(|f| &f.ty));
                }
                _ => {}
            }
        }
        for ty in referenced {
            if let FieldType::Ref { entity } = ty
                && let Some(target) = self.project.entity(&entity.name)
            {
                choices.insert(entity.name.clone(), display(target));
            }
        }
        for (entity, shown) in &choices {
            let _ = writeln!(constructor, "        {entity}Choices = Choice.Of(Db.All<{entity}>(), r => {shown});");
            let _ = write!(properties, "    public Choice[] {entity}Choices {{ get; }} = [];\n\n");
        }
        // The dropdowns' options (U1c): the values a typed one's field has,
        // and the records a search picks from with the fields it shows.
        let mut lists: BTreeMap<String, String> = BTreeMap::new();
        for p in self.parts.iter().map(|w| &w.props).filter(|p| p.kind == WidgetKind::Dropdown) {
            let Some((entity, field)) = p.bind.as_deref().and_then(|b| self.bound_field(b)) else { continue };
            if p.typed {
                let name = typed_values(entity, field);
                let read = format!("        {name} = [.. Db.All<{}>().Select(r => r.{})];", entity.name.name, property(&entity.name.name, &field.name.name));
                if lists.insert(name.clone(), read).is_none() {
                    let _ = write!(properties, "    public List<string> {name} {{ get; }} = [];\n\n");
                }
            }
            // Without fields to show, a search picks from the reference's own choices.
            if let (true, FieldType::Ref { entity: target }) = (p.search, &field.ty)
                && !p.show.is_empty()
                && let Some(target) = self.project.entity(&target.name)
            {
                let name = searched_choices(target, &p.show);
                let cells: Vec<String> = p
                    .show
                    .iter()
                    .map(|f| self.shown_text(target, f, &format!("r.{}", property(&target.name.name, f))))
                    .collect();
                let read = format!("        {name} = Choice.Of(Db.All<{}>(), r => {}, r => [{}]);", target.name.name, display(target), cells.join(", "));
                if lists.insert(name.clone(), read).is_none() {
                    let _ = write!(properties, "    public Choice[] {name} {{ get; }} = [];\n\n");
                }
            }
        }
        for read in lists.values() {
            let _ = writeln!(constructor, "{read}");
        }

        // A state per named part.
        let mut parts = String::new();
        let mut seen = BTreeSet::new();
        for widget in &self.parts {
            let Some(n) = &widget.props.name else { continue };
            if !seen.insert(n.clone()) {
                continue;
            }
            let mut args = vec![string(n)];
            if let Some(c) = &widget.props.caption
                && matches!(
                    widget.props.kind,
                    WidgetKind::Button
                        | WidgetKind::Label
                        | WidgetKind::Checkbox
                        | WidgetKind::Switch
                        | WidgetKind::Toggle
                        | WidgetKind::Heading
                        | WidgetKind::Badge
                        | WidgetKind::Link
                        | WidgetKind::Busy
                )
            {
                args.push(format!("caption: {}", Texts::kept(c)));
            }
            if widget.props.readonly {
                args.push("readOnly: true".into());
            }
            let help = self.help_of(n);
            if !help.is_empty() {
                args.push(format!("help: {}", Texts::kept(&help)));
            }
            // A foldable part starts as designed; a popover and a slide-out start closed.
            let starts = if matches!(widget.props.kind, WidgetKind::Popover | WidgetKind::Slideout) {
                " { IsVisible = false }"
            } else if widget.props.foldable && widget.props.folded {
                " { IsFolded = true }"
            } else {
                ""
            }
            .to_string();
            // A canvas's grid, snap and background start as its line says; code can change them.
            let starts = if widget.props.kind == WidgetKind::Canvas {
                let mut set = Vec::new();
                if let Some(grid) = widget.props.grid {
                    set.push(format!("Grid = {grid}"));
                }
                if widget.props.snap {
                    set.push("Snap = true".to_string());
                }
                if let Some(background) = &widget.props.background {
                    set.push(format!("Background = {}", string(background)));
                }
                if let Some(size) = widget.props.dot_size {
                    set.push(format!("DotSize = {size}"));
                }
                if let Some(color) = &widget.props.dot_color {
                    set.push(format!("DotColor = {}", string(color)));
                }
                // A painting or vector canvas's tool, brush and ink (DESIGN 5.31).
                let tool = crate::canvas_tool(&widget.props);
                if !tool.is_empty() {
                    set.push(format!("Tool = {}", string(&tool)));
                }
                if let Some(brush) = widget.props.brush {
                    set.push(format!("Brush = {brush}"));
                }
                if let Some(ink) = &widget.props.ink {
                    set.push(format!("Ink = {}", string(ink)));
                }
                if set.is_empty() { starts } else { format!(" {{ {} }}", set.join(", ")) }
            } else {
                starts
            };
            // An input bound to a required field shows why a save refused it (conformance/required.json).
            let starts = match self.required_field(widget) {
                Some(field) if starts.is_empty() => format!(" {{ {field} }}"),
                Some(field) => format!("{}, {field} }}", starts.trim_end_matches('}').trim_end()),
                None => starts,
            };
            // A custom part starts with the values written on its line.
            if let Some(declared) = widget.props.part.as_deref().and_then(|k| crate::parts::declaration(self.project, k)) {
                let mut with = String::new();
                for (property, value) in &widget.props.values {
                    let Some(ty) = declared.properties.iter().find(|q| q.name.name == *property).map(|q| &q.ty) else { continue };
                    let code = match value {
                        clarity_spec::model::Literal::Str(text) if text.starts_with('@') => format!("Locale.Translate({})", string(&text[1..])),
                        other => logic::literal(self.project, ty, other),
                    };
                    let _ = write!(with, ".With({}, {code})", string(property));
                }
                let _ = writeln!(parts, "    public PartState {} {{ get; }} = new PartState({}){with};", member(n), args.join(", "));
                continue;
            }
            let _ = writeln!(parts, "    public PartState {} {{ get; }} = new({}){starts};", member(n), args.join(", "));
        }
        // The menu and the toolbar have a state each.
        for name in self.frame.menu.and_then(|m| m.name.as_ref()).into_iter().chain(self.frame.toolbar.and_then(|t| t.name.as_ref())) {
            if seen.insert(name.clone()) {
                let _ = writeln!(parts, "    public PartState {} {{ get; }} = new({});", member(name), string(name));
            }
        }
        // The ribbon starts folded as designed; its tabs and groups have a state each.
        if let Some(ribbon) = self.frame.ribbon {
            if let Some(n) = &ribbon.name
                && seen.insert(n.clone())
            {
                let starts = if ribbon.collapsible && ribbon.collapsed { " { IsCollapsed = true }" } else { "" };
                let _ = writeln!(parts, "    public PartState {} {{ get; }} = new({}){starts};", member(n), string(n));
            }
            for name in ribbon.tabs.iter().flat_map(|t| t.name.iter().chain(t.groups.iter().filter_map(|g| g.name.as_ref()))) {
                if seen.insert(name.clone()) {
                    let _ = writeln!(parts, "    public PartState {} {{ get; }} = new({});", member(name), string(name));
                }
            }
        }
        // A sidebar starts folded as designed.
        for sidebar in [self.frame.left, self.frame.right].into_iter().flatten() {
            let Some(n) = &sidebar.props.name else { continue };
            if seen.insert(n.clone()) {
                let starts = if sidebar.props.collapsible && sidebar.props.collapsed { " { IsCollapsed = true }" } else { "" };
                let _ = writeln!(parts, "    public PartState {} {{ get; }} = new({}){starts};", member(n), string(n));
            }
        }
        for command in &self.commands {
            let Some(n) = &command.props.name else { continue };
            if seen.insert(n.clone()) {
                let caption = command.props.caption.as_deref().map(|c| format!(", caption: {}", Texts::kept(c))).unwrap_or_default();
                let _ = writeln!(parts, "    public PartState {} {{ get; }} = new({}{caption});", member(n), string(n));
            }
        }

        // Handlers: the window's own, and the app's for its frame.
        let mut handlers = String::new();
        let mut own: Vec<&clarity_spec::code::HandlerDecl> = Vec::new();
        for block in self.project.files.iter().flat_map(|f| &f.code) {
            let ours = block.name.name == *name || self.app.is_some_and(|a| a.name.name == block.name.name);
            if ours {
                own.extend(&block.handlers);
            }
        }
        for decl in &own {
            let method = command_method(&decl.name.name, &self.members);
            match logic::handler(self.project, types, &ctx, decl) {
                // One that takes values runs through the commands of its calls.
                Ok(body) if !decl.params.is_empty() => {
                    let _ = writeln!(handlers, "{body}");
                }
                Ok(body) => {
                    let _ = write!(handlers, "    [RelayCommand]\n    void {method}() => Handle({});\n\n{body}\n", on(&decl.name.name));
                }
                Err(problems) => errors.extend(problems.into_iter().map(|e| format!("handler '{}' of {name}: {e}", decl.name.name))),
            }
        }

        // The window's own events.
        let mut events = String::new();
        for h in &self.page.handlers {
            let hook = match h.event {
                Event::Open => "OnOpen",
                Event::Close => "OnClose",
                Event::Return => "OnReturn",
                _ => continue,
            };
            let _ = write!(events, "    protected override void {hook}() => {}();\n\n", on(&h.handler));
        }
        // The canvases, drawn anew by their draw handlers after every handler.
        let draws = crate::canvas_draws(self.page);
        if !draws.is_empty() {
            let body: String = draws
                .iter()
                .map(|(canvas, handler)| format!("        {c}.BeginDrawing();\n        {}();\n        {c}.EndDrawing();\n", on(handler), c = member(canvas)))
                .collect();
            let _ = write!(events, "    protected override void Redraw()\n    {{\n{body}    }}\n\n");
        }

        // A click, a drag, a drop, a bend, a new line or a box resized on a canvas: where it was, then its handler.
        for widget in self.parts.iter().filter(|w| w.props.kind == WidgetKind::Canvas) {
            let Some(canvas) = &widget.props.name else { continue };
            let scope = self.undo_scope(&widget.props);
            for (event, command) in [(Event::Click, "ClickAt"), (Event::Drag, "DragAt"), (Event::Drop, "DropAt"), (Event::Bend, "BendAt"), (Event::Connect, "ConnectAt"), (Event::Resize, "ResizeAt")] {
                let Some(handler) = Self::handler(widget, event) else { continue };
                let run = format!("Handle(() =>\n    {{\n        {c}.Pointer = at;\n        {}();\n    }}, [at])", on(&handler.handler), c = member(canvas));
                // A drag, a drop, a bend, a new line and a resize are edits Ctrl+Z takes back (DESIGN 5.29); a drag starts one its drop ends.
                let run = match (&scope, event) {
                    (Some(scope), Event::Drag | Event::Drop | Event::Bend | Event::Connect | Event::Resize | Event::Click) => {
                        format!("EditOf(\"{canvas}\", \"{scope}\", ends: {}, () => {run})", event != Event::Drag)
                    }
                    _ => run,
                };
                let _ = write!(events, "    [RelayCommand]\n    void {command}_{c}(Pointer at) => {run};\n\n", c = member(canvas));
            }
        }

        // An action on a canvas with a mode (DESIGN 5.31): the runtime applies it, then `on change`, as one edit.
        for widget in self.parts.iter().filter(|w| w.props.kind == WidgetKind::Canvas && w.props.mode.is_some()) {
            let Some(canvas) = &widget.props.name else { continue };
            let p = &widget.props;
            let scope = self.undo_scope(p).map_or("null".to_string(), |s| string(&s));
            let mut args = format!("new CanvasArgs({})", string(p.mode.map(|m| m.keyword()).unwrap_or_default()));
            let mut set = vec![format!("Grid = {c}.Grid", c = member(canvas)), format!("Snap = {c}.Snap", c = member(canvas))];
            if let Some(objects) = &p.objects {
                let entity = self.entity_of(&objects.list).map(|e| e.name.name.clone()).unwrap_or_default();
                set.push(format!("List = {}", string(&member(&objects.list))));
                set.push(format!("At = [{}, {}]", string(&property(&entity, &objects.x)), string(&property(&entity, &objects.y))));
            }
            if let Some(bind) = &p.bind {
                set.push(format!("Path = {}", string(&self.path(bind))));
            }
            if let Some(units) = p.units {
                set.push(format!("Units = [{}, {}]", units.width, units.height));
            }
            if p.readonly {
                set.push("Readonly = true".to_string());
            }
            let _ = write!(args, " {{ {} }}", set.join(", "));
            let changed = Self::handler(widget, Event::Change).map_or("null".to_string(), |h| on(&h.handler));
            let _ = write!(
                events,
                "    [RelayCommand]\n    void CanvasAt_{c}(System.Text.Json.Nodes.JsonObject action) => CanvasEvent({c}, {scope}, {args}, action, {changed});\n\n",
                c = member(canvas)
            );
        }

        // A wizard's Next: its handler, saying whether it refused.
        for widget in self.parts.iter().filter(|w| w.props.kind == WidgetKind::Steps) {
            let (Some(wizard), Some(next)) = (&widget.props.name, Self::handler(widget, Event::Next)) else { continue };
            let Some(decl) = own.iter().find(|d| d.name.name == next.handler) else { continue };
            let values: Vec<String> = next.args.iter().zip(&decl.params).map(|(v, p)| logic::literal(self.project, &p.ty, v)).collect();
            let _ = write!(events, "    public bool Next_{}() => Handle(() => {}({}));\n\n", member(wizard), on(&next.handler), values.join(", "));
        }

        // Windows opened by a click, and the built-in targets.
        let mut opens = BTreeSet::new();
        let mut quit = false;
        let mut close = false;
        let mut keys = false;
        let mut undo = false;
        let targets = self
            .parts
            .iter()
            .filter_map(|w| w.props.on_click.as_deref().map(|t| (t, w.props.new_window)))
            .chain(self.commands.iter().filter_map(|c| c.props.on_click.as_deref().map(|t| (t, c.props.new_window))));
        for (target, new_window) in targets {
            match target {
                QUIT => quit = true,
                CLOSE => close = true,
                KEYS => keys = true,
                UNDO | REDO => undo = true,
                t if self.project.pages().any(|p| p.name.name == t) => {
                    opens.insert((t.to_string(), new_window));
                }
                _ => {}
            }
        }
        let mut clicks = String::new();
        for (target, new_window) in &opens {
            if *new_window {
                let _ = write!(
                    clicks,
                    "    [RelayCommand]\n    void OpenNew_{target}() => Handle(() => Open(new {target}ViewModel(Db), newWindow: true));\n\n"
                );
            } else {
                let _ = write!(clicks, "    [RelayCommand]\n    void Open_{target}() => Handle(() => Open(new {target}ViewModel(Db)));\n\n");
            }
        }
        // What each custom part tells, run by the handler wired to it with the tell's values.
        for widget in self.parts.iter().filter(|w| !w.props.tells.is_empty()) {
            let Some(name) = &widget.props.name else { continue };
            let mut cases = String::new();
            for wired in &widget.props.tells {
                let Some(decl) = own.iter().find(|d| d.name.name == wired.handler) else { continue };
                let values: Vec<String> = decl
                    .params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| format!("PartValues.Read<{}>(values, {i})", super::names::value_type(&crate::type_ref_info(self.project, &p.ty))))
                    .collect();
                // With what it told, as the handler's values.
                let _ = write!(cases, "            case {}:\n                Handle(() => {}({}), [tell, values]);\n                break;\n", string(&wired.tell), on(&wired.handler), values.join(", "));
            }
            let _ = write!(clicks, "    /// <summary>What custom part {name} told.</summary>\n    public void Told_{}(string tell, System.Text.Json.JsonElement[] values)\n    {{\n        switch (tell)\n        {{\n{cases}        }}\n    }}\n\n", member(name));
        }
        // The calls with values its parts wire, a command each, and its Run_,
        // which the screen runs to hear whether the handler refused.
        for (call, command) in &self.calls {
            let Some(decl) = own.iter().find(|d| d.name.name == call.handler) else { continue };
            let values: Vec<String> = call.args.iter().zip(&decl.params).map(|(v, p)| logic::literal(self.project, &p.ty, v)).collect();
            let _ = write!(
                clicks,
                "    [RelayCommand]\n    void {command}() => Run_{command}();\n\n    public bool Run_{command}() => Handle(() => {}({}), command: nameof({command}));\n\n",
                on(&call.handler),
                values.join(", ")
            );
        }
        if quit {
            clicks.push_str("    [RelayCommand]\n    void Quit_() => Handle(QuitApplication);\n\n");
        }
        if keys {
            clicks.push_str("    [RelayCommand]\n    void Keys_() => ShowKeys();\n\n");
        }
        if undo {
            clicks.push_str("    [RelayCommand]\n    void Undo_() => UndoFocused(true);\n\n    [RelayCommand]\n    void Redo_() => UndoFocused(false);\n\n");
        }
        if close {
            clicks.push_str("    [RelayCommand]\n    void CloseWindow_() => Handle(Close);\n\n");
        }
        // The popovers and slide-outs a click opens.
        for popover in self.page.widgets.iter().filter(|w| matches!(w.props.kind, WidgetKind::Popover | WidgetKind::Slideout)) {
            let Some(n) = &popover.props.name else { continue };
            let _ = write!(clicks, "    [RelayCommand]\n    void Show_{n}() => {}.IsVisible = true;\n\n", member(n));
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let title = self.page.title.clone().unwrap_or_else(|| name.clone());
        let mut identity = format!("    public override string Title => {};\n", self.texts.code(&title));
        if self.page.kind == PageKind::Dialog {
            identity.push_str("\n    public override bool IsDialog => true;\n");
        }
        if let Some(size) = self.page.size {
            identity.push_str(&format!("\n    public override Extent? WindowSize => new({}, {});\n", size.width, size.height));
        }
        let body = format!("{identity}\n{properties}{parts}\n{events}{clicks}{handlers}");
        Ok(format!(
            "{HEADER}
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;

namespace {namespace};

public sealed partial class {name}ViewModel : WindowViewModel
{{
    public {name}ViewModel(Db? db) : base(db)
    {{
{said}{constructor}    }}

{}
}}
",
            body.trim_end()
        ))
    }
}

/// The view model member that reads a file of the store of `data.field`.
fn picture_file(data: &str, field: &str) -> String {
    format!("{}{}File", member(data), pascal(field))
}

/// The view model member that fetches the stored picture of `data.field`.
fn picture_fetch(data: &str, field: &str) -> String {
    format!("{}{}Picture", member(data), pascal(field))
}
