//! Each window as an Avalonia view (a `UserControl` in AXAML) with its
//! ViewModel and code-behind, shown in the application's one desktop window
//! (the runtime's `Shell`); only a dialog gets a desktop window of its own. The
//! grid becomes a `Grid` with a `*` column per grid column and the shared
//! row height (`clarity_spec::layout`), rows that hold a growing part as `*`
//! rows; each part binds to the window's data and, when it has a name, to
//! its `PartState`; handlers become ViewModel methods, run as commands.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;

use clarity_spec::Project;
use clarity_spec::code::check::FileTypes;
use clarity_spec::frame::{ShownFrame, shown};
use clarity_spec::layout::{CONTENT_PADDING, GAP, POPOVER_COLUMN, ROW_HEIGHT};
use clarity_spec::model::{App, Command, CommandKind, DataDecl, DataKind, Entity, Field, FieldType, Handler, ListItem, Literal, Page, PageKind, Widget, WidgetProps};
use clarity_spec::widgets::{Event, WidgetKind};
use clarity_spec::widgets;

use super::HEADER;
use super::logic::{self, PartInfo, WindowCtx};
use super::names::{backing, control, member, pascal, property, string};
use super::texts::{Texts, plain_attr};
use super::themes::style_class;
use crate::OutFile;

/// The built-in click targets that are no window: `quit` ends the
/// application, `close` closes the window, `language` and `theme` pick in
/// place (the runtime's `Pickers`).
const QUIT: &str = "quit";
const CLOSE: &str = "close";
const LANGUAGE: &str = "language";
const THEME: &str = "theme";

/// The width of a dialog's view, in pixels; its window fits it.
const DIALOG_WIDTH: u32 = 560;

/// The AXAML, code-behind and ViewModel of `page`.
pub fn emit(project: &Project, page: &Page, types: &FileTypes, namespace: &str, texts: &Texts) -> Result<Vec<OutFile>, Vec<String>> {
    let app = project.apps().next();
    let frame = shown(app, page);
    let window = Window::new(project, page, app, frame, texts);
    let name = &page.name.name;
    let view_model = window.view_model(types, namespace)?;
    Ok(vec![
        text(&format!("Windows/{name}View.axaml"), window.axaml(namespace)),
        text(&format!("Windows/{name}View.axaml.cs"), window.code_behind(namespace)),
        text(&format!("Windows/{name}ViewModel.cs"), view_model),
    ])
}

fn text(path: &str, body: String) -> OutFile {
    OutFile {
        path: path.to_string(),
        text: body,
        bytes: None,
    }
}

/// Text as an AXAML attribute value that is no text of the project (a key).
fn attr(text: &str) -> String {
    plain_attr(text)
}

/// `axaml` with `class` added to the classes of its first element.
fn with_class(axaml: &str, class: &str) -> String {
    let start = axaml.find('<').unwrap_or(0);
    let end = axaml.find('>').unwrap_or(axaml.len());
    let head = &axaml[start..end];
    match head.find(" Classes=\"") {
        Some(at) => {
            let insert = start + at + " Classes=\"".len();
            format!("{}{class} {}", &axaml[..insert], &axaml[insert..])
        }
        None => {
            let name_end = start + head.find([' ', '/', '>']).unwrap_or(head.len());
            format!("{} Classes=\"{class}\"{}", &axaml[..name_end], &axaml[name_end..])
        }
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

/// An icon drawn `size` pixels square from the generated Icons class, in
/// the foreground of what holds it (the style Path.icon in App.axaml).
fn icon_xaml(name: &str, size: u32) -> String {
    format!(
        "<Viewbox Width=\"{size}\" Height=\"{size}\"><Canvas Width=\"24\" Height=\"24\"><Path Classes=\"icon\" Data=\"{{x:Static local:Icons.{}}}\" /></Canvas></Viewbox>",
        icon_member(name)
    )
}

/// Whether a part takes extra height when the window grows (the web
/// renderer's `widgetGrows`).
fn grows(widget: &Widget) -> bool {
    let info = widgets::info(widget.props.kind);
    if !info.grows {
        return false;
    }
    if info.container.is_none() {
        return true;
    }
    widget.children.iter().chain(widget.pages.iter().flat_map(|p| &p.children)).any(grows)
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
    /// The control of each command that picks the language or theme, by the
    /// command's address: `picker1`.
    pickers: HashMap<usize, String>,
    /// The calls with values its parts wire, each with the command that
    /// makes it: `digit(7)` with `Digit7`.
    calls: Vec<(&'a Handler, String)>,
}

/// A shortcut as Avalonia names its keys: `Backspace` as `Back`, `Ctrl++`
/// as `Ctrl+OemPlus`, `7` as `D7`; modifiers and the other keys are named alike.
fn gesture(shortcut: &str) -> String {
    let (modifiers, key) = clarity_spec::widgets::shortcut_keys(shortcut);
    let key = match key {
        "Backspace" => "Back".to_string(),
        "+" | "=" => "OemPlus".to_string(),
        "-" => "OemMinus".to_string(),
        "." => "OemPeriod".to_string(),
        "," => "OemComma".to_string(),
        "*" => "Multiply".to_string(),
        "/" => "Divide".to_string(),
        digit if digit.len() == 1 && digit.chars().all(|c| c.is_ascii_digit()) => format!("D{digit}"),
        other => other.to_string(),
    };
    if modifiers.is_empty() { key } else { format!("{}+{key}", modifiers.join("+")) }
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

fn address(command: &Command) -> usize {
    std::ptr::from_ref(command) as usize
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
        let pickers = commands
            .iter()
            .filter(|c| matches!(c.props.on_click.as_deref(), Some(LANGUAGE | THEME)))
            .enumerate()
            .map(|(i, c)| (address(c), format!("picker{}", i + 1)))
            .collect();
        // A command for each call with values, named after it; a name that
        // is taken already gets a number.
        let mut taken: BTreeSet<String> = project.files.iter().flat_map(|f| &f.code).flat_map(|b| &b.handlers).map(|h| pascal(&h.name.name)).collect();
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
            pickers,
            calls,
        }
    }

    /// The command a part's handler runs: the handler's own, or, for a call
    /// with values, the command made for it.
    fn command(&self, h: &Handler) -> String {
        if h.args.is_empty() {
            return pascal(&h.handler);
        }
        self.calls.iter().find(|(c, _)| c.handler == h.handler && c.args == h.args).map(|(_, name)| name.clone()).unwrap_or_else(|| call_command(h))
    }

    /// A text of the project as an attribute value, in the language shown.
    fn text(&self, text: &str) -> String {
        self.texts.attr(text)
    }

    /// A value of a field with a format, shown in the person's format, else as the runtime shows it.
    /// Whether a date part is bound to a moment (a datetime field).
    fn binds_moment(&self, p: &WidgetProps) -> bool {
        p.bind.as_deref().and_then(|b| self.bound_field(b)).is_some_and(|(_, f)| f.ty == FieldType::DateTime)
    }

    /// A column of a table part: the field, what kind of cell holds it, and
    /// whether and how it is edited.
    fn table_column(&self, entity: &Entity, field: &Field, edit: bool, switch: bool, width: Option<u32>) -> String {
        let kind = match &field.ty {
            FieldType::Integer | FieldType::Decimal { .. } | FieldType::Money => "Number",
            FieldType::Bool => "Bool",
            FieldType::Enum { .. } | FieldType::Ref { .. } => "Choice",
            FieldType::Date => "Date",
            FieldType::DateTime => "Moment",
            FieldType::Color => "Color",
            _ => "Text",
        };
        let mut attributes = format!(
            "Field=\"{}\" Header=\"{}\" Kind=\"{kind}\"",
            property(&entity.name.name, &field.name.name),
            self.text(&field.label)
        );
        if let Some(width) = width {
            let _ = write!(attributes, " Width=\"{width}\"");
        }
        if !edit {
            let _ = write!(attributes, " Shown=\"{{x:Static {}}}\"", self.shown_converter(entity, &field.name.name));
            return format!("<c:TableColumn {attributes} />");
        }
        attributes.push_str(" Edit=\"True\"");
        match &field.ty {
            FieldType::Bool if switch => attributes.push_str(" Switch=\"True\""),
            FieldType::Enum { name } => {
                let _ = write!(attributes, " Choices=\"{{x:Static local:Choices.{}}}\"", name.name);
            }
            // The records it can point at, which the window reads.
            FieldType::Ref { entity: target } => {
                let _ = write!(attributes, " ChoicesFrom=\"{}Choices\"", target.name);
            }
            FieldType::Integer | FieldType::Decimal { .. } | FieldType::Money => {
                let _ = write!(attributes, " Number=\"{{x:Static local:Edit.{}{}}}\"", entity.name.name, pascal(&field.name.name));
            }
            _ => {}
        }
        format!("<c:TableColumn {attributes} />")
    }

    fn shown_converter(&self, entity: &Entity, field: &str) -> String {
        match entity.field(field).map(|f| &f.ty) {
            Some(FieldType::Integer | FieldType::Decimal { .. } | FieldType::Money | FieldType::Duration | FieldType::Date | FieldType::DateTime) => {
                format!("local:Shown.{}{}", entity.name.name, pascal(field))
            }
            _ => "local:Shown.Converter".to_string(),
        }
    }

    /// The enum of a plain value, as `pending` of `data pending: Operation`.
    fn value_enum(&self, data: &str) -> Option<String> {
        let ty = self.values.iter().find(|v| v.name.name == data)?.value()?;
        match logic::type_ref_info(self.project, ty) {
            clarity_spec::code::check::TypeInfo::Enum(e) => Some(e),
            _ => None,
        }
    }

    fn entity_of(&self, data: &str) -> Option<&'a Entity> {
        let (_, entity, _) = self.data.iter().find(|(n, _, _)| n == data)?;
        self.project.entity(entity)
    }

    /// The field a binding such as `order.number` reads, and its entity.
    /// A slider's or a spin box's range as attributes: its ends as written,
    /// else the bound field's own, its step, and whether it holds whole numbers.
    fn range_attrs(&self, p: &WidgetProps) -> String {
        let field = p.bind.as_deref().and_then(|b| self.bound_field(b)).map(|(_, f)| f);
        let range = widgets::number_range(p, field.map(|f| &f.modifiers));
        let mut attrs = String::new();
        if let Some(min) = &range.min {
            let _ = write!(attrs, " Min=\"{min}\"");
        }
        if let Some(max) = &range.max {
            let _ = write!(attrs, " Max=\"{max}\"");
        }
        let _ = write!(attrs, " Step=\"{}\"", range.step);
        if p.kind == WidgetKind::Slider {
            let whole = match field {
                Some(f) => f.ty == FieldType::Integer,
                None => [&range.min, &range.max].into_iter().flatten().chain([&range.step]).all(|n| !n.contains('.')),
            };
            let _ = write!(attrs, " Whole=\"{}\"", if whole { "True" } else { "False" });
        }
        attrs
    }

    fn bound_field(&self, bind: &str) -> Option<(&'a Entity, &'a Field)> {
        let (data, field) = bind.split_once('.')?;
        let entity = self.entity_of(data)?;
        Some((entity, entity.field(field)?))
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

    /// The command a click runs: a handler, or opening a window, in place or,
    /// where the design says so, in a desktop window of its own.
    fn click_command(&self, command: Option<String>, on_click: Option<&str>, new_window: bool) -> Option<String> {
        if let Some(command) = command {
            return Some(format!("{command}Command"));
        }
        match on_click? {
            // The commands Clarity makes itself have an underscore in their
            // names, which no part's name (camelCase) has, so they never clash.
            QUIT => Some("Quit_Command".into()),
            CLOSE => Some("CloseWindow_Command".into()),
            target if self.project.pages().any(|p| p.name.name == target) => {
                Some(format!("{}_{target}Command", if new_window { "OpenNew" } else { "Open" }))
            }
            // A popover of the window opens next to its part.
            target if self.popover(target).is_some() => Some(format!("Show_{target}Command")),
            _ => None,
        }
    }

    // -- AXAML ---------------------------------------------------------------

    fn axaml(&self, namespace: &str) -> String {
        let name = &self.page.name.name;
        // A dialog sizes its desktop window to its view.
        let width = if self.page.kind == PageKind::Dialog { format!(" Width=\"{DIALOG_WIDTH}\"") } else { String::new() };
        let mut out = String::new();
        // The menu and the toolbar show and take clicks while their part state says so.
        let bar = |name: &Option<String>, rest: &str| match name {
            Some(n) => {
                let part = member(n);
                format!(" x:Name=\"{}\"{rest} IsEnabled=\"{{Binding {part}.IsEnabled}}\" IsVisible=\"{{Binding {part}.IsVisible}}\"", attr(n))
            }
            None => rest.to_string(),
        };
        if let Some(menu) = self.frame.menu {
            let _ = writeln!(out, "      <Menu{}>", bar(&menu.name, " DockPanel.Dock=\"Top\""));
            for item in &menu.items {
                self.menu_item(item, 4, &mut out);
            }
            out.push_str("      </Menu>\n");
        }
        // The ribbon is the runtime's RibbonView: its open tab and folding follow
        // its part state, and each tab and group shows while its own is visible.
        if let Some(ribbon) = self.frame.ribbon {
            let mut attrs = String::new();
            if let Some(name) = &ribbon.name {
                let _ = write!(attrs, " x:Name=\"{}\"", attr(name));
            }
            attrs.push_str(" DockPanel.Dock=\"Top\" Classes=\"ribbon\"");
            if let Some(name) = &ribbon.name {
                let part = member(name);
                if ribbon.collapsible {
                    let _ = write!(attrs, " Collapsible=\"True\" IsCollapsed=\"{{Binding {part}.IsCollapsed}}\"");
                }
                let _ = write!(attrs, " Tab=\"{{Binding {part}.Tab}}\"");
            }
            let shown = |name: &Option<String>| match name {
                Some(n) => format!(" x:Name=\"{}\"", attr(n)),
                None => String::new(),
            };
            let visible = |name: &Option<String>| match name {
                Some(n) => format!(" IsVisible=\"{{Binding {}.IsVisible}}\"", member(n)),
                None => String::new(),
            };
            let _ = writeln!(out, "      <c:RibbonView{attrs}>");
            for tab in &ribbon.tabs {
                let _ = writeln!(out, "        <TabItem{} Header=\"{}\"{}>", shown(&tab.name), self.text(&tab.title), visible(&tab.name));
                out.push_str("          <StackPanel Orientation=\"Horizontal\" Spacing=\"16\">\n");
                for group in &tab.groups {
                    let _ = writeln!(out, "            <StackPanel{} Spacing=\"4\"{}>\n              <StackPanel Orientation=\"Horizontal\" Spacing=\"4\">", shown(&group.name), visible(&group.name));
                    for item in &group.items {
                        self.button(item, 8, &mut out);
                    }
                    let _ = writeln!(out, "              </StackPanel>\n              <TextBlock Classes=\"group\" Text=\"{}\" />\n            </StackPanel>", self.text(&group.title));
                }
                out.push_str("          </StackPanel>\n        </TabItem>\n");
            }
            out.push_str("      </c:RibbonView>\n");
        }
        if let Some(toolbar) = self.frame.toolbar {
            let _ = writeln!(out, "      <Border{}>\n        <StackPanel Orientation=\"Horizontal\" Spacing=\"4\">", bar(&toolbar.name, " DockPanel.Dock=\"Top\" Classes=\"toolbar\""));
            for item in &toolbar.items {
                self.button(item, 5, &mut out);
            }
            out.push_str("        </StackPanel>\n      </Border>\n");
        }
        if let Some(footer) = self.frame.footer {
            out.push_str("      <Border DockPanel.Dock=\"Bottom\" Classes=\"footer\">\n        <DockPanel>\n");
            if let Some(status) = &footer.status {
                let _ = writeln!(out, "          <TextBlock Classes=\"status\" Text=\"{}\" />", self.text(status));
            }
            self.grid(&footer.widgets, footer.columns, Some(1), false, 5, &mut out);
            out.push_str("        </DockPanel>\n      </Border>\n");
        }
        // A sidebar is the runtime's SidebarView: dragged, folded and filling the window as it allows.
        for (side, dock) in [(self.frame.left, "Left"), (self.frame.right, "Right")] {
            let Some(sidebar) = side else { continue };
            let p = &sidebar.props;
            let mut attrs = format!(" DockPanel.Dock=\"{dock}\" Side=\"{dock}\" Classes=\"sidebar\" SidebarWidth=\"{}\"", p.width);
            if let Some(min) = p.min {
                let _ = write!(attrs, " MinSidebarWidth=\"{min}\"");
            }
            if let Some(max) = p.max {
                let _ = write!(attrs, " MaxSidebarWidth=\"{max}\"");
            }
            for (on, what) in [(p.resizable, "Resizable"), (p.collapsible, "Collapsible"), (p.maximizable, "Maximizable")] {
                if on {
                    let _ = write!(attrs, " {what}=\"True\"");
                }
            }
            if let Some(name) = &p.name {
                let part = member(name);
                if p.collapsible {
                    let _ = write!(attrs, " IsCollapsed=\"{{Binding {part}.IsCollapsed}}\"");
                }
                if p.maximizable {
                    let _ = write!(attrs, " IsMaximized=\"{{Binding {part}.IsMaximized}}\"");
                }
                attrs = format!(" x:Name=\"{}\"{attrs}", attr(name));
            }
            let _ = writeln!(out, "      <c:SidebarView{attrs}>\n        <c:SidebarView.Content>");
            self.grid(&sidebar.widgets, p.columns, None, false, 5, &mut out);
            out.push_str("        </c:SidebarView.Content>\n      </c:SidebarView>\n");
        }
        // A window smaller than its design scrolls, unless rows grow to fit.
        let fluid = self.page.widgets.iter().any(grows);
        if fluid {
            self.grid(&self.page.widgets, self.page.columns, None, true, 3, &mut out);
        } else {
            out.push_str("      <ScrollViewer>\n");
            self.grid(&self.page.widgets, self.page.columns, None, true, 4, &mut out);
            out.push_str("      </ScrollViewer>\n");
        }
        // The parts above sit one level deeper than they end up.
        let body: String = out.lines().map(|l| format!("{}\n", l.strip_prefix("  ").unwrap_or(l))).collect();
        format!(
            "<!-- Generated by Clarity. Do not edit: change the project in Clarity instead. -->
<UserControl xmlns=\"https://github.com/avaloniaui\"
             xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\"
             xmlns:local=\"using:{namespace}\"
             xmlns:c=\"using:Clarity.Avalonia\"
             x:Class=\"{namespace}.{name}View\"
             x:DataType=\"local:{name}ViewModel\"{width}>
  <DockPanel>
{body}  </DockPanel>
</UserControl>
"
        )
    }

    fn menu_item(&self, item: &Command, depth: usize, out: &mut String) {
        let pad = "  ".repeat(depth);
        if item.props.kind == CommandKind::Separator {
            let _ = writeln!(out, "{pad}<Separator />");
            return;
        }
        let header = self.text(item.props.caption.as_deref().unwrap_or_default());
        // A named item hides and disables as its part state says, as every part does.
        let state = item.props.name.as_deref().map(|n| Self::state(n, false)).unwrap_or_default();
        if !item.items.is_empty() {
            let _ = writeln!(out, "{pad}<MenuItem Header=\"{header}\"{state}>");
            for child in &item.items {
                self.menu_item(child, depth + 1, out);
            }
            let _ = writeln!(out, "{pad}</MenuItem>");
            return;
        }
        let mut attrs = match self.pickers.get(&address(item)) {
            Some(picker) => format!("x:Name=\"{picker}\" Header=\"{header}\""),
            None => format!("Header=\"{header}\""),
        };
        if let Some(key) = &item.props.shortcut {
            let _ = write!(attrs, " InputGesture=\"{key}\" HotKey=\"{key}\"", key = attr(&gesture(key)));
        }
        if let Some(command) = self.click_command(item.props.handler.as_deref().map(pascal), item.props.on_click.as_deref(), item.props.new_window) {
            let _ = write!(attrs, " Command=\"{{Binding {command}}}\"");
        }
        attrs.push_str(&state);
        match &item.props.icon {
            Some(icon) => {
                let _ = writeln!(out, "{pad}<MenuItem {attrs}>\n{pad}  <MenuItem.Icon>{}</MenuItem.Icon>\n{pad}</MenuItem>", icon_xaml(icon, 16));
            }
            None => {
                let _ = writeln!(out, "{pad}<MenuItem {attrs} />");
            }
        }
    }

    /// A toolbar or ribbon button.
    fn button(&self, item: &Command, depth: usize, out: &mut String) {
        let pad = "  ".repeat(depth);
        if item.props.kind == CommandKind::Separator {
            let _ = writeln!(out, "{pad}<Separator Classes=\"vertical\" />");
            return;
        }
        let content = self.text(item.props.caption.as_deref().unwrap_or_default());
        let mut attrs = match self.pickers.get(&address(item)) {
            Some(picker) => format!("x:Name=\"{picker}\" Content=\"{content}\""),
            None => format!("Content=\"{content}\""),
        };
        if let Some(command) = self.click_command(item.props.handler.as_deref().map(pascal), item.props.on_click.as_deref(), item.props.new_window) {
            let _ = write!(attrs, " Command=\"{{Binding {command}}}\"");
        }
        let Some(icon) = &item.props.icon else {
            let _ = writeln!(out, "{pad}<Button {attrs} />");
            return;
        };
        // An icon beside the caption; a large ribbon button has a bigger one above it.
        let attrs = attrs.replacen(&format!(" Content=\"{content}\""), "", 1).replacen(&format!("Content=\"{content}\" "), "", 1).replacen(&format!("Content=\"{content}\""), "", 1);
        let (orientation, size) = if item.props.large { ("Vertical", 24) } else { ("Horizontal", 16) };
        let space = if attrs.is_empty() { String::new() } else { format!(" {attrs}") };
        let _ = writeln!(out, "{pad}<Button{space}>");
        let _ = writeln!(out, "{pad}  <StackPanel Orientation=\"{orientation}\" Spacing=\"4\" HorizontalAlignment=\"Center\">");
        let _ = writeln!(out, "{pad}    {}", icon_xaml(icon, size));
        let _ = writeln!(out, "{pad}    <TextBlock Text=\"{content}\" HorizontalAlignment=\"Center\" VerticalAlignment=\"Center\" />");
        let _ = writeln!(out, "{pad}  </StackPanel>\n{pad}</Button>");
    }

    /// A grid of parts: `columns` shared columns; rows of the shared height,
    /// or, for the window's own grid, `*` rows where a part grows.
    fn grid(&self, widgets: &[Widget], columns: u32, fixed_rows: Option<u32>, window: bool, depth: usize, out: &mut String) {
        let pad = "  ".repeat(depth);
        let bottom = widgets.iter().map(|w| w.props.row + w.props.height - 1).max().unwrap_or(0);
        let rows = fixed_rows.unwrap_or(bottom.max(1));
        let mut growing = BTreeSet::new();
        if fixed_rows.is_none() {
            for w in widgets.iter().filter(|w| grows(w)) {
                growing.extend(w.props.row..w.props.row + w.props.height);
            }
        }
        let column_definitions = vec!["*"; columns.max(1) as usize].join(",");
        let margin = if window { format!(" Margin=\"{CONTENT_PADDING}\"") } else { String::new() };
        let _ = writeln!(out, "{pad}<Grid ColumnDefinitions=\"{column_definitions}\" ColumnSpacing=\"{GAP}\" RowSpacing=\"{GAP}\"{margin}>");
        let _ = writeln!(out, "{pad}  <Grid.RowDefinitions>");
        for row in 1..=rows {
            if growing.contains(&row) {
                let _ = writeln!(out, "{pad}    <RowDefinition Height=\"*\" MinHeight=\"{ROW_HEIGHT}\" />");
            } else {
                let _ = writeln!(out, "{pad}    <RowDefinition Height=\"{ROW_HEIGHT}\" />");
            }
        }
        let _ = writeln!(out, "{pad}  </Grid.RowDefinitions>");
        for widget in widgets {
            self.widget(widget, depth + 1, out);
        }
        let _ = writeln!(out, "{pad}</Grid>");
    }

    /// The bindings a named part has to its state.
    fn state(name: &str, read_only: bool) -> String {
        let part = member(name);
        let mut out = format!(" IsEnabled=\"{{Binding {part}.IsEnabled}}\" IsVisible=\"{{Binding {part}.IsVisible}}\"");
        if read_only {
            let _ = write!(out, " IsReadOnly=\"{{Binding {part}.IsReadOnly}}\"");
        }
        out
    }

    /// What a badge, a link or a busy indicator says: what it binds, shown
    /// in the person's format; else its caption, which code can set.
    fn shown_text(&self, p: &clarity_spec::model::WidgetProps, bound: Option<&str>, named: Option<&str>, caption: &str) -> String {
        match (bound, p.bind.as_deref().and_then(|b| self.bound_field(b))) {
            (Some(path), Some((entity, field))) => format!("{{Binding {path}, Converter={{x:Static {}}}}}", self.shown_converter(entity, &field.name.name)),
            (Some(path), None) => format!("{{Binding {path}, Converter={{x:Static local:Shown.Converter}}}}"),
            _ => match named {
                Some(n) if p.caption.is_some() => format!("{{Binding {}.Caption}}", member(n)),
                _ => self.text(caption),
            },
        }
    }

    /// A popover of the window, by name.
    fn popover(&self, name: &str) -> Option<&'a Widget> {
        self.page.widgets.iter().find(|w| w.props.kind == WidgetKind::Popover && w.props.name.as_deref() == Some(name))
    }

    /// A part, with the class of its style, whose values come from the theme shown.
    fn widget(&self, widget: &Widget, depth: usize, out: &mut String) {
        let mut own = String::new();
        // A split button's menu is its own dropdown; any other part's is a context menu around it.
        let context = widget.menu.as_ref().filter(|_| !widget.props.split);
        let depth = if context.is_some() { depth + 1 } else { depth };
        self.part_axaml(widget, depth, &mut own);
        if let Some(style) = widget.props.style.as_deref() {
            own = with_class(&own, &style_class(style));
        }
        let Some(menu) = context else {
            out.push_str(&own);
            return;
        };
        // A part with a context menu stands in a panel that has it, in the
        // part's place: a right click anywhere on the part opens it.
        let p = &widget.props;
        let mut place = format!("Grid.Column=\"{}\" Grid.Row=\"{}\"", p.col - 1, p.row - 1);
        if p.width > 1 {
            let _ = write!(place, " Grid.ColumnSpan=\"{}\"", p.width);
        }
        if p.height > 1 {
            let _ = write!(place, " Grid.RowSpan=\"{}\"", p.height);
        }
        let pad = "  ".repeat(depth - 1);
        let visible = p.name.as_deref().map(|n| format!(" IsVisible=\"{{Binding {}.IsVisible}}\"", member(n))).unwrap_or_default();
        let named = menu.name.as_deref().map(|n| format!(" IsVisible=\"{{Binding {}.IsVisible}}\" IsEnabled=\"{{Binding {}.IsEnabled}}\"", member(n), member(n))).unwrap_or_default();
        let _ = writeln!(out, "{pad}<Panel {place}{visible}>\n{pad}  <Panel.ContextMenu>\n{pad}    <ContextMenu{named}>");
        for item in &menu.items {
            self.menu_item(item, depth + 2, out);
        }
        let _ = writeln!(out, "{pad}    </ContextMenu>\n{pad}  </Panel.ContextMenu>");
        out.push_str(&own.replacen(&format!(" {place}"), "", 1).replacen(&place, "", 1));
        let _ = writeln!(out, "{pad}</Panel>");
    }

    fn part_axaml(&self, widget: &Widget, depth: usize, out: &mut String) {
        let pad = "  ".repeat(depth);
        let p = &widget.props;
        let mut at = String::new();
        if let Some(name) = &p.name {
            let _ = write!(at, "x:Name=\"{}\" ", control(name));
        }
        let _ = write!(at, "Grid.Column=\"{}\" Grid.Row=\"{}\"", p.col - 1, p.row - 1);
        if p.width > 1 {
            let _ = write!(at, " Grid.ColumnSpan=\"{}\"", p.width);
        }
        if p.height > 1 {
            let _ = write!(at, " Grid.RowSpan=\"{}\"", p.height);
        }
        let named = p.name.as_deref();
        let text_state = named.map(|n| Self::state(n, true)).unwrap_or_default();
        let plain_state = named.map(|n| Self::state(n, false)).unwrap_or_default();
        let mut extra = String::new();
        if let Some(tip) = &p.tooltip {
            let _ = write!(extra, " ToolTip.Tip=\"{}\"", self.text(tip));
        }
        if let Some(order) = p.tab_order {
            let _ = write!(extra, " TabIndex=\"{order}\"");
        }
        let bound = p.bind.as_deref().map(|b| self.path(b));
        let shown = |path: &str| format!("{{Binding {path}, Converter={{x:Static local:Shown.Converter}}}}");
        let caption_text = p.caption.clone().unwrap_or_default();
        match p.kind {
            WidgetKind::Label => {
                let required = p
                    .for_part
                    .as_deref()
                    .and_then(|t| self.part(t))
                    .and_then(|w| w.props.bind.as_deref())
                    .and_then(|b| self.bound_field(b))
                    .is_some_and(|(_, f)| f.modifiers.required);
                let text = match named {
                    Some(n) => format!("{{Binding {}.Caption}}", member(n)),
                    None => self.text(&caption_text),
                };
                if required {
                    // The mark of a required input stays beside the translated label.
                    let _ = writeln!(out, "{pad}<TextBlock {at} Classes=\"label\"{extra}><Run Text=\"{text}\" /><Run Text=\" *\" /></TextBlock>");
                } else {
                    let _ = writeln!(out, "{pad}<TextBlock {at} Classes=\"label\" Text=\"{text}\"{extra} />");
                }
            }
            WidgetKind::Help => {
                let target = p.for_part.as_deref().filter(|t| self.part(t).is_some_and(|w| w.props.name.is_some()));
                match target {
                    Some(t) => {
                        let part = member(t);
                        let _ = writeln!(
                            out,
                            "{pad}<TextBlock {at} Classes=\"help\" Classes.error=\"{{Binding {part}.HasError}}\" Text=\"{{Binding {part}.Shown}}\"{extra} />"
                        );
                    }
                    None => {
                        let _ = writeln!(out, "{pad}<TextBlock {at} Classes=\"help\" Text=\"{}\"{extra} />", self.text(&caption_text));
                    }
                }
            }
            // A button that stays pressed: Avalonia's ToggleButton, bound as a checkbox is.
            WidgetKind::Toggle => {
                let value = match (bound.as_deref(), named) {
                    (Some(b), _) => format!(" IsChecked=\"{{Binding {b}}}\""),
                    (None, Some(n)) => format!(" IsChecked=\"{{Binding {}.IsChecked}}\"", member(n)),
                    (None, None) => String::new(),
                };
                let text = match named {
                    Some(n) if p.caption.is_some() => format!("{{Binding {}.Caption}}", member(n)),
                    _ => self.text(&caption_text),
                };
                let read_only = if p.readonly { " IsEnabled=\"False\"" } else { "" };
                let tip = if p.caption.is_none() && p.tooltip.is_none() { named.map(|n| format!(" ToolTip.Tip=\"{}\"", attr(n))).unwrap_or_default() } else { String::new() };
                let inside = match (&p.icon, p.caption.is_some()) {
                    (Some(icon), true) => format!("<StackPanel Orientation=\"Horizontal\" Spacing=\"6\" HorizontalAlignment=\"Center\">{}<TextBlock Text=\"{text}\" VerticalAlignment=\"Center\" /></StackPanel>", icon_xaml(icon, 16)),
                    (Some(icon), false) => icon_xaml(icon, 16),
                    (None, _) => format!("<TextBlock Text=\"{text}\" />"),
                };
                let _ = writeln!(out, "{pad}<ToggleButton {at} Classes=\"toggle\"{value}{read_only}{tip}{plain_state}{extra}>{inside}</ToggleButton>");
            }
            // Buttons and toggles joined in one row: their own grid, without the space between columns.
            WidgetKind::Buttons => {
                let _ = writeln!(out, "{pad}<Border {at} Classes=\"buttons\"{plain_state}{extra}>");
                let mut inner = String::new();
                self.grid(&widget.children, p.width, Some(1), false, depth + 1, &mut inner);
                out.push_str(&inner.replacen(&format!("ColumnSpacing=\"{GAP}\""), "ColumnSpacing=\"0\"", 1));
                let _ = writeln!(out, "{pad}</Border>");
            }
            // A line between parts, level unless vertical.
            WidgetKind::Separator => {
                let class = if p.orientation == Some(clarity_spec::model::Orientation::Vertical) { "separator vertical" } else { "separator" };
                let _ = writeln!(out, "{pad}<Border {at} Classes=\"{class}\"{plain_state}{extra} />");
            }
            // A short text or number in a pill: what it binds, or its caption.
            WidgetKind::Badge => {
                let text = self.shown_text(p, bound.as_deref(), named, &caption_text);
                let _ = writeln!(out, "{pad}<Border {at} Classes=\"badge\"{plain_state}{extra}><TextBlock Text=\"{text}\" /></Border>");
            }
            // A link: Avalonia's HyperlinkButton, opening its address, what it binds, or running a handler.
            WidgetKind::Link => {
                let text = self.shown_text(p, bound.as_deref(), named, &caption_text);
                let mut attrs = format!("Content=\"{text}\"");
                if let Some(to) = &p.to {
                    let _ = write!(attrs, " NavigateUri=\"{}\"", attr(to));
                } else if let Some(path) = bound.as_deref() {
                    let _ = write!(attrs, " NavigateUri=\"{{Binding {path}, Converter={{x:Static local:Shown.Address}}}}\"");
                } else if let Some(command) = self.click_command(Self::handler(widget, Event::Click).map(|h| self.command(h)), p.on_click.as_deref(), p.new_window) {
                    let _ = write!(attrs, " Command=\"{{Binding {command}}}\"");
                }
                let _ = writeln!(out, "{pad}<HyperlinkButton {at} {attrs}{plain_state}{extra} />");
            }
            // A bar for a number, from 0 to its max.
            WidgetKind::Progress => {
                let value = bound.as_deref().map(|b| format!(" Value=\"{{Binding {b}}}\"")).unwrap_or_default();
                let max = p.max.as_deref().unwrap_or("100");
                let _ = writeln!(out, "{pad}<ProgressBar {at} Classes=\"progress\" Minimum=\"0\" Maximum=\"{max}\"{value}{plain_state}{extra} />");
            }
            // Busy: Avalonia's running bar with what it is busy with.
            WidgetKind::Busy => {
                let text = self.shown_text(p, None, named, &caption_text);
                let _ = writeln!(
                    out,
                    "{pad}<StackPanel {at} Classes=\"busy\" Orientation=\"Horizontal\"{plain_state}{extra}><ProgressBar IsIndeterminate=\"True\" /><TextBlock Text=\"{text}\" /></StackPanel>"
                );
            }
            WidgetKind::Heading => {
                let _ = writeln!(out, "{pad}<TextBlock {at} Classes=\"heading\" Text=\"{}\"{plain_state}{extra} />", self.text(&caption_text));
            }
            // A number in seven-segment digits, drawn by the runtime.
            WidgetKind::Display => {
                let value = bound.as_deref().map(|b| format!(" Value=\"{{Binding {b}}}\"")).unwrap_or_default();
                let digits = p.digits.unwrap_or(clarity_spec::widgets::DEFAULT_DIGITS);
                let _ = writeln!(out, "{pad}<c:SegmentDisplay {at}{value} Digits=\"{digits}\"{plain_state}{extra} />");
            }
            WidgetKind::Text => {
                let text = match (bound.as_deref(), p.bind.as_deref().and_then(|b| self.bound_field(b))) {
                    (Some(path), Some((entity, field))) => {
                        format!("{{Binding {path}, Converter={{x:Static {}}}}}", self.shown_converter(entity, &field.name.name))
                    }
                    (Some(path), None) => shown(path),
                    _ => String::new(),
                };
                if p.height > 1 {
                    // Over several rows: lines to read, wrapped, scrolling when longer than the rows.
                    let place = at.split_once("Grid.").map(|(_, rest)| format!("Grid.{rest}")).unwrap_or_default();
                    let name = named.map(|n| format!("x:Name=\"{}\" ", control(n))).unwrap_or_default();
                    let visible = named.map(|n| format!(" IsVisible=\"{{Binding {}.IsVisible}}\"", member(n))).unwrap_or_default();
                    let enabled = named.map(|n| format!(" IsEnabled=\"{{Binding {}.IsEnabled}}\"", member(n))).unwrap_or_default();
                    let _ = writeln!(out, "{pad}<ScrollViewer {place} HorizontalScrollBarVisibility=\"Disabled\" VerticalScrollBarVisibility=\"Auto\"{visible}>");
                    let _ = writeln!(out, "{pad}  <TextBlock {name}Classes=\"value lines\" Text=\"{text}\"{enabled}{extra} />");
                    let _ = writeln!(out, "{pad}</ScrollViewer>");
                } else {
                    let _ = writeln!(out, "{pad}<TextBlock {at} Classes=\"value\" Text=\"{text}\"{plain_state}{extra} />");
                }
            }
            // Markdown, drawn by the runtime: a file of the resources folder in
            // the language shown, or a text it binds.
            WidgetKind::Markdown => {
                let content = match (p.source.as_deref(), bound.as_deref(), p.caption.as_deref()) {
                    (Some(source), _, _) => format!(" Source=\"{}\"", attr(source)),
                    (None, Some(path), _) => format!(" Markdown=\"{{Binding {path}}}\""),
                    // A key: the Markdown of the file it points at in the language shown, through the texts.
                    (None, None, Some(key)) => format!(" Markdown=\"{}\"", self.text(key)),
                    (None, None, None) => String::new(),
                };
                let _ = writeln!(out, "{pad}<c:MarkdownView {at}{content}{plain_state}{extra} />");
            }
            // Clarity's own inputs (Clarity.Avalonia), which tell when the person
            // changed their value and only then, the same as on the web.
            // A color (U1c): Clarity's ColorInput, a swatch with the shared palette and the color to type.
            WidgetKind::Color => {
                let value = match (&bound, named) {
                    (Some(path), _) => format!(" Value=\"{{Binding {path}}}\""),
                    (None, Some(n)) => format!(" Value=\"{{Binding {}.Text}}\"", member(n)),
                    (None, None) => String::new(),
                };
                let read_only = if p.readonly { " IsReadOnly=\"True\"" } else { "" };
                let _ = writeln!(out, "{pad}<c:ColorInput {at}{value}{read_only}{plain_state}{extra} />");
            }
            // A number between its lowest and highest value (U1c), Clarity's SliderInput.
            WidgetKind::Slider => {
                let value = match (&bound, named) {
                    (Some(path), _) => format!(" Value=\"{{Binding {path}}}\""),
                    (None, Some(n)) => format!(" Value=\"{{Binding {}.Text}}\"", member(n)),
                    (None, None) => String::new(),
                };
                let read_only = if p.readonly { " IsReadOnly=\"True\"" } else { "" };
                let _ = writeln!(out, "{pad}<c:SliderInput {at}{}{value}{read_only}{plain_state}{extra} />", self.range_attrs(p));
            }
            WidgetKind::Textbox | WidgetKind::Password | WidgetKind::Memo | WidgetKind::Number | WidgetKind::Spinbox => {
                let number = p
                    .bind
                    .as_deref()
                    .and_then(|b| self.bound_field(b))
                    .filter(|(_, f)| matches!(f.ty, FieldType::Integer | FieldType::Decimal { .. } | FieldType::Money | FieldType::Duration));
                let class = match p.kind {
                    WidgetKind::Memo => " Classes=\"memo\" AcceptsReturn=\"True\" TextWrapping=\"Wrap\"",
                    WidgetKind::Number | WidgetKind::Spinbox if number.is_none() => " Classes=\"number\"",
                    WidgetKind::Password => " PasswordChar=\"*\"",
                    _ => "",
                };
                let (control, value) = match (&bound, named, number) {
                    // A number shows and reads in the person's format.
                    (Some(path), _, Some((entity, field))) => (
                        "c:NumberInput",
                        format!(" Format=\"{{x:Static local:Edit.{}{}}}\" Value=\"{{Binding {path}}}\"", entity.name.name, pascal(&field.name.name)),
                    ),
                    (Some(path), _, None) => ("c:TextInput", format!(" Value=\"{{Binding {path}}}\"")),
                    // Unbound, a named input holds its own text.
                    (None, Some(n), _) => ("c:TextInput", format!(" Value=\"{{Binding {}.Text}}\"", member(n))),
                    (None, None, _) => ("c:TextInput", String::new()),
                };
                let placeholder = p.placeholder.as_deref().map(|t| format!(" PlaceholderText=\"{}\"", self.text(t))).unwrap_or_default();
                let read_only = if p.readonly && named.is_none() { " IsReadOnly=\"True\"" } else { "" };
                // A spin box is a number input with a range and arrows.
                let range = if p.kind == WidgetKind::Spinbox { self.range_attrs(p) } else { String::new() };
                let _ = writeln!(out, "{pad}<{control} {at}{class}{range}{value}{placeholder}{read_only}{text_state}{extra} />");
            }
            WidgetKind::Checkbox | WidgetKind::Switch => {
                // A switch's Content is a header above it; its caption goes beside it, as on the web.
                let (control, label) = if p.kind == WidgetKind::Switch {
                    ("c:SwitchInput", format!("OnContent=\"{text}\" OffContent=\"{text}\"", text = self.text(&caption_text)))
                } else {
                    ("c:CheckInput", format!("Content=\"{}\"", self.text(&caption_text)))
                };
                // Unbound, a named checkbox holds its own state.
                let value = match (bound.as_deref(), named) {
                    (Some(b), _) => format!(" IsChecked=\"{{Binding {b}}}\""),
                    (None, Some(n)) => format!(" IsChecked=\"{{Binding {}.IsChecked}}\"", member(n)),
                    (None, None) => String::new(),
                };
                let read_only = if p.readonly { " IsEnabled=\"False\"" } else { "" };
                let _ = writeln!(out, "{pad}<{control} {at} {label}{value}{read_only}{plain_state}{extra} />");
            }
            // Any text, the values the field has offered (U1c).
            WidgetKind::Dropdown if p.typed => {
                let values = p.bind.as_deref().and_then(|b| self.bound_field(b)).map(|(e, f)| format!(" Values=\"{{Binding {}}}\"", typed_values(e, f))).unwrap_or_default();
                let value = bound.as_deref().map(|b| format!(" Value=\"{{Binding {b}}}\"")).unwrap_or_default();
                let read_only = if p.readonly { " IsReadOnly=\"True\"" } else { "" };
                let _ = writeln!(out, "{pad}<c:TypedInput {at}{values}{value}{read_only}{plain_state}{extra} />");
            }
            // A record among many, filtered as it is typed, with the fields it shows (U1c).
            WidgetKind::Dropdown if p.search => {
                let choices = match p.bind.as_deref().and_then(|b| self.bound_field(b)).map(|(_, f)| &f.ty) {
                    Some(FieldType::Ref { entity }) => self.project.entity(&entity.name).map(|t| format!(" Choices=\"{{Binding {}}}\"", searched_choices(t, &p.show))).unwrap_or_default(),
                    _ => String::new(),
                };
                let value = bound.as_deref().map(|b| format!(" Value=\"{{Binding {b}}}\"")).unwrap_or_default();
                let read_only = if p.readonly { " IsReadOnly=\"True\"" } else { "" };
                let _ = writeln!(out, "{pad}<c:LookupInput {at}{choices}{value}{read_only}{plain_state}{extra} />");
            }
            WidgetKind::Dropdown | WidgetKind::Radio => {
                // An enum's values (never an empty one), or the records a reference can point at after an empty one.
                let choices = match p.bind.as_deref().and_then(|b| self.bound_field(b)).map(|(_, f)| &f.ty) {
                    Some(FieldType::Enum { name }) => format!(" Choices=\"{{x:Static local:Choices.{}}}\"", name.name),
                    // A plain value of an enum: `data pending: Operation`.
                    None if let Some(e) = p.bind.as_deref().and_then(|b| self.value_enum(b)) => format!(" Choices=\"{{x:Static local:Choices.{e}}}\""),
                    Some(FieldType::Ref { entity }) => format!(" Choices=\"{{Binding {}Choices}}\"", entity.name),
                    // The settings' own: the project's languages, the offered themes.
                    Some(FieldType::Language) => " Choices=\"{x:Static c:Choice.LanguagesOffered}\"".to_string(),
                    Some(FieldType::Theme) => " Choices=\"{x:Static c:Choice.ThemesOffered}\"".to_string(),
                    _ => String::new(),
                };
                let value = bound.as_deref().map(|b| format!(" Value=\"{{Binding {b}}}\"")).unwrap_or_default();
                let control = if p.kind == WidgetKind::Radio { "c:RadioInput" } else { "c:ChoiceInput" };
                let vertical = if p.kind == WidgetKind::Radio && p.orientation == Some(clarity_spec::model::Orientation::Vertical) { " Vertical=\"True\"" } else { "" };
                let _ = writeln!(out, "{pad}<{control} {at}{choices}{value}{vertical}{plain_state}{extra} />");
            }
            WidgetKind::Date if self.binds_moment(p) => {
                // A moment is picked as a day and a time of day, as on the web.
                let value = bound.as_deref().map(|b| format!(" Value=\"{{Binding {b}}}\"")).unwrap_or_default();
                let _ = writeln!(out, "{pad}<c:DateTimeInput {at}{value} HorizontalAlignment=\"Stretch\"{plain_state}{extra} />");
            }
            WidgetKind::Date => {
                let value = bound.as_deref().map(|b| format!(" Value=\"{{Binding {b}}}\"")).unwrap_or_default();
                let _ = writeln!(out, "{pad}<c:DateInput {at}{value}{plain_state}{extra} />");
            }
            WidgetKind::Button => {
                let content = match named {
                    Some(n) => format!("{{Binding {}.Caption}}", member(n)),
                    None => self.text(&caption_text),
                };
                let mut attrs = format!("Content=\"{content}\"");
                if let Some(command) = self.click_command(Self::handler(widget, Event::Click).map(|h| self.command(h)), p.on_click.as_deref(), p.new_window) {
                    let _ = write!(attrs, " Command=\"{{Binding {command}}}\"");
                }
                if p.default_button {
                    attrs.push_str(" IsDefault=\"True\"");
                }
                if p.cancel {
                    attrs.push_str(" IsCancel=\"True\"");
                }
                if let Some(key) = &p.shortcut {
                    // A character is what is typed, from the keyboard or the number pad.
                    if key.chars().count() == 1 {
                        let _ = write!(attrs, " c:Keys.Typed=\"{}\"", attr(key));
                    } else {
                        let _ = write!(attrs, " HotKey=\"{}\"", attr(&gesture(key)));
                    }
                }
                if p.split {
                    // A split button: Avalonia's SplitButton, its menu the flyout behind the arrow.
                    let label = match &p.icon {
                        Some(icon) => format!(
                            "\n{pad}  <StackPanel Orientation=\"Horizontal\" Spacing=\"6\">{}<TextBlock Text=\"{content}\" VerticalAlignment=\"Center\" /></StackPanel>",
                            icon_xaml(icon, 16)
                        ),
                        None => format!("\n{pad}  <TextBlock Text=\"{content}\" />"),
                    };
                    let attrs = attrs.replacen(&format!("Content=\"{content}\""), "", 1);
                    let attrs = attrs.trim_start();
                    let _ = writeln!(out, "{pad}<SplitButton {at} {attrs}{plain_state}{extra}>{label}");
                    let _ = writeln!(out, "{pad}  <SplitButton.Flyout>\n{pad}    <MenuFlyout Placement=\"BottomEdgeAlignedLeft\">");
                    for item in widget.menu.iter().flat_map(|m| &m.items) {
                        self.menu_item(item, depth + 3, out);
                    }
                    let _ = writeln!(out, "{pad}    </MenuFlyout>\n{pad}  </SplitButton.Flyout>\n{pad}</SplitButton>");
                    return;
                }
                match &p.icon {
                    // An icon beside the caption.
                    Some(icon) => {
                        let attrs = attrs.replacen(&format!("Content=\"{content}\""), "", 1);
                        let attrs = attrs.trim_start();
                        let _ = writeln!(out, "{pad}<Button {at} {attrs}{plain_state}{extra}>");
                        let _ = writeln!(out, "{pad}  <StackPanel Orientation=\"Horizontal\" Spacing=\"6\" HorizontalAlignment=\"Center\">");
                        let _ = writeln!(out, "{pad}    {}", icon_xaml(icon, 16));
                        let _ = writeln!(out, "{pad}    <TextBlock Text=\"{content}\" VerticalAlignment=\"Center\" />");
                        let _ = writeln!(out, "{pad}  </StackPanel>\n{pad}</Button>");
                    }
                    None => {
                        let _ = writeln!(out, "{pad}<Button {at} {attrs}{plain_state}{extra} />");
                    }
                }
            }
            // An icon of its own, centered in its cells.
            WidgetKind::Icon => {
                let icon = p.icon.as_deref().unwrap_or("circle");
                let _ = writeln!(
                    out,
                    "{pad}<Border {at} Classes=\"icon\"{plain_state}{extra}>{}</Border>",
                    icon_xaml(icon, 20).replacen("<Viewbox ", "<Viewbox HorizontalAlignment=\"Center\" VerticalAlignment=\"Center\" ", 1)
                );
            }
            // A foldable panel or form is an Expander: open while the part is not folded.
            WidgetKind::Form | WidgetKind::Panel if p.foldable && named.is_some() => {
                let class = if p.kind == WidgetKind::Form { "form" } else { "panel" };
                let header = p.caption.as_ref().map(|t| format!(" Header=\"{}\"", self.text(t))).unwrap_or_default();
                let part = member(named.unwrap_or_default());
                let _ = writeln!(out, "{pad}<Expander {at}{header} IsExpanded=\"{{Binding {part}.IsOpen}}\" Classes=\"{class}\"{plain_state}{extra}>");
                self.grid(&widget.children, p.width, None, false, depth + 1, out);
                let _ = writeln!(out, "{pad}</Expander>");
            }
            WidgetKind::Form | WidgetKind::Panel => {
                let _ = writeln!(out, "{pad}<Border {at} Classes=\"{}\"{plain_state}{extra}>", if p.kind == WidgetKind::Form { "form" } else { "panel" });
                let _ = writeln!(out, "{pad}  <DockPanel>");
                if let Some(title) = &p.caption {
                    let _ = writeln!(out, "{pad}    <TextBlock DockPanel.Dock=\"Top\" Classes=\"legend\" Text=\"{}\" />", self.text(title));
                }
                self.grid(&widget.children, p.width, None, false, depth + 2, out);
                let _ = writeln!(out, "{pad}  </DockPanel>\n{pad}</Border>");
            }
            WidgetKind::Tabs => {
                let _ = writeln!(out, "{pad}<TabControl {at} Classes=\"tabs\"{plain_state}{extra}>");
                for tab in &widget.pages {
                    let _ = writeln!(out, "{pad}  <TabItem Header=\"{}\">", self.text(tab.title.as_deref().unwrap_or_default()));
                    self.grid(&tab.children, p.width, None, false, depth + 2, out);
                    let _ = writeln!(out, "{pad}  </TabItem>");
                }
                let _ = writeln!(out, "{pad}</TabControl>");
            }
            // A popover is Avalonia's Popup, open while its part state is visible
            // and closed by a click outside it, next to the part in its `for`.
            WidgetKind::Popover => {
                let open = named.map(|n| format!(" IsOpen=\"{{Binding {}.IsVisible, Mode=TwoWay}}\"", member(n))).unwrap_or_default();
                let target = p.for_part.as_deref().map(|t| format!(" PlacementTarget=\"{{Binding ElementName={}}}\"", control(t))).unwrap_or_default();
                let name = named.map(|n| format!(" x:Name=\"{}\"", control(n))).unwrap_or_default();
                // Its grid's width, with the padding and border of Border.popover around it.
                let width = POPOVER_COLUMN * p.width + GAP * (p.width - 1) + 2 * 8 + 2;
                let _ = writeln!(out, "{pad}<Popup{name}{open}{target} Placement=\"BottomEdgeAlignedLeft\" IsLightDismissEnabled=\"True\">");
                let _ = writeln!(out, "{pad}  <Border Classes=\"popover\" Width=\"{width}\">");
                self.grid(&widget.children, p.width, Some(p.height), false, depth + 2, out);
                let _ = writeln!(out, "{pad}  </Border>\n{pad}</Popup>");
            }
            // An accordion is the runtime's AccordionView: its open section follows the part's state.
            WidgetKind::Accordion => {
                let open = named.map(|n| format!(" Open=\"{{Binding {}.OpenSection}}\"", member(n))).unwrap_or_default();
                let _ = writeln!(out, "{pad}<c:AccordionView {at}{open}{plain_state}{extra}>");
                for page in &widget.pages {
                    let _ = writeln!(out, "{pad}  <c:SectionItem Header=\"{}\">", self.text(page.title.as_deref().unwrap_or_default()));
                    self.grid(&page.children, p.width, None, false, depth + 2, out);
                    let _ = writeln!(out, "{pad}  </c:SectionItem>");
                }
                let _ = writeln!(out, "{pad}</c:AccordionView>");
            }
            // A scroll box: its own grid in a ScrollViewer, so parts below its rows scroll into view.
            WidgetKind::Scroll => {
                let _ = writeln!(out, "{pad}<ScrollViewer {at} Classes=\"scroll\" HorizontalScrollBarVisibility=\"Disabled\" VerticalScrollBarVisibility=\"Auto\"{plain_state}{extra}>");
                self.grid(&widget.children, p.width, None, false, depth + 1, out);
                let _ = writeln!(out, "{pad}</ScrollViewer>");
            }
            // A wizard is the runtime's StepsView: its step follows the part's state.
            WidgetKind::Steps => {
                let step = named.map(|n| format!(" Step=\"{{Binding {}.Step}}\"", member(n))).unwrap_or_default();
                let _ = writeln!(out, "{pad}<c:StepsView {at}{step}{plain_state}{extra}>");
                for page in &widget.pages {
                    let _ = writeln!(out, "{pad}  <c:StepItem Header=\"{}\">", self.text(page.title.as_deref().unwrap_or_default()));
                    self.grid(&page.children, p.width, None, false, depth + 2, out);
                    let _ = writeln!(out, "{pad}  </c:StepItem>");
                }
                let _ = writeln!(out, "{pad}</c:StepsView>");
            }
            WidgetKind::Splitter => {
                let vertical = p.orientation == Some(clarity_spec::model::Orientation::Vertical);
                let definitions = vec!["*"; widget.pages.len().max(1)].join(",Auto,");
                let axis = if vertical { "RowDefinitions" } else { "ColumnDefinitions" };
                let _ = writeln!(out, "{pad}<Grid {at} {axis}=\"{definitions}\"{plain_state}{extra}>");
                for (i, pane) in widget.pages.iter().enumerate() {
                    let position = if vertical { "Grid.Row" } else { "Grid.Column" };
                    if i > 0 {
                        let resize = if vertical { "Rows" } else { "Columns" };
                        let _ = writeln!(out, "{pad}  <GridSplitter {position}=\"{}\" ResizeDirection=\"{resize}\" />", 2 * i - 1);
                    }
                    let _ = writeln!(out, "{pad}  <Border {position}=\"{}\">", 2 * i);
                    self.grid(&pane.children, p.width, None, false, depth + 2, out);
                    let _ = writeln!(out, "{pad}  </Border>");
                }
                let _ = writeln!(out, "{pad}</Grid>");
            }
            WidgetKind::Table => {
                // Clarity's own table part: it owns how cells behave, the same as on the web.
                let entity = p.bind.as_deref().and_then(|b| self.entity_of(b));
                let selected = named.map(|n| format!(" SelectedItem=\"{{Binding {}.SelectedItem}}\"", member(n))).unwrap_or_default();
                let items = bound.as_deref().map(|b| format!(" ItemsSource=\"{{Binding {b}}}\"")).unwrap_or_default();
                // Resizing, sorting and the first order: the table's own, kept per window and table.
                let mut options = String::new();
                if p.resizable {
                    options.push_str(" Resizable=\"True\"");
                }
                if p.sortable {
                    options.push_str(" Sortable=\"True\"");
                }
                if let (Some(order), Some(entity)) = (&p.order, entity) {
                    let _ = write!(options, " OrderBy=\"{}\"", property(&entity.name.name, &order.field));
                    if order.desc {
                        options.push_str(" OrderDescending=\"True\"");
                    }
                }
                if p.resizable && let Some(name) = named {
                    let _ = write!(options, " Remember=\"{}.{name}\"", self.page.name.name);
                }
                let _ = writeln!(out, "{pad}<c:ClarityTable {at}{items}{selected}{options}{plain_state}{extra}>");
                let _ = writeln!(out, "{pad}  <c:ClarityTable.Columns>");
                if let Some(entity) = entity {
                    for column in &p.columns {
                        let Some(field) = entity.field(column) else { continue };
                        let width = p.widths.get(column).copied();
                        let _ = writeln!(out, "{pad}    {}", self.table_column(entity, field, p.edit.contains(column), p.switches.contains(column), width));
                    }
                }
                let _ = writeln!(out, "{pad}  </c:ClarityTable.Columns>\n{pad}</c:ClarityTable>");
            }
            WidgetKind::List => {
                let entity = p.bind.as_deref().and_then(|b| self.entity_of(b));
                let selected = named.map(|n| format!(" SelectedItem=\"{{Binding {}.SelectedItem}}\"", member(n))).unwrap_or_default();
                let items = bound.as_deref().map(|b| format!(" ItemsSource=\"{{Binding {b}}}\"")).unwrap_or_default();
                let _ = writeln!(out, "{pad}<ListBox {at}{items}{selected}{plain_state}{extra}>");
                if let Some(entity) = entity {
                    let _ = writeln!(out, "{pad}  <ListBox.ItemTemplate>\n{pad}    <DataTemplate x:DataType=\"local:{}\">", entity.name.name);
                    let _ = writeln!(out, "{pad}      <StackPanel Orientation=\"Horizontal\" Spacing=\"8\">");
                    for (i, field) in p.show.iter().enumerate() {
                        if i > 0 {
                            let _ = writeln!(out, "{pad}        <TextBlock Text=\"|\" />");
                        }
                        let _ = writeln!(
                            out,
                            "{pad}        <TextBlock Text=\"{{Binding {}, Converter={{x:Static {}}}}}\" />",
                            property(&entity.name.name, field),
                            self.shown_converter(entity, field)
                        );
                    }
                    let _ = writeln!(out, "{pad}      </StackPanel>\n{pad}    </DataTemplate>\n{pad}  </ListBox.ItemTemplate>");
                }
                let _ = writeln!(out, "{pad}</ListBox>");
            }
            // A record's file, which the view model fetches to open it or save a copy.
            // A list field's items as chips (U1c); an enum's are picked from its values.
            WidgetKind::Tags => {
                let field = p.bind.as_deref().and_then(|b| self.bound_field(b)).map(|(_, f)| &f.ty);
                let (item, choices) = match field {
                    Some(FieldType::Values { item: ListItem::Enum { name } }) => ("Choice", format!(" Choices=\"{{x:Static local:Choices.{}}}\"", name.name)),
                    Some(FieldType::Values { item: ListItem::Integer }) => ("Integer", String::new()),
                    Some(FieldType::Values { item: ListItem::Decimal }) => ("Decimal", String::new()),
                    Some(FieldType::Values { item: ListItem::Date }) => ("Date", String::new()),
                    _ => ("Text", String::new()),
                };
                let value = bound.as_deref().map(|b| format!(" Value=\"{{Binding {b}}}\"")).unwrap_or_default();
                let read_only = if p.readonly { " IsReadOnly=\"True\"" } else { "" };
                let _ = writeln!(
                    out,
                    "{pad}<c:TagsInput {at} Item=\"{item}\"{choices}{value} AddText=\"{add}\" RemoveText=\"{remove}\"{read_only}{plain_state}{extra} />",
                    // Clarity's own words, by key.
                    add = self.text("@addTag"),
                    remove = self.text("@removeTag"),
                );
            }
            WidgetKind::File => {
                let (data, field) = p.bind.as_deref().and_then(|b| b.split_once('.')).unwrap_or_default();
                let path = bound.as_deref().unwrap_or_default();
                let read_only = if p.readonly && named.is_none() { " IsReadOnly=\"True\"" } else { "" };
                let in_store = self.bound_field(p.bind.as_deref().unwrap_or_default()).is_some_and(|(_, f)| f.modifiers.store.is_some());
                let fetch_file = if in_store { format!(" FetchFile=\"{{Binding {}}}\"", picture_file(data, field)) } else { String::new() };
                let _ = writeln!(
                    out,
                    "{pad}<c:FileInput {at} File=\"{{Binding {path}}}\" Record=\"{{Binding {record}}}\" Fetch=\"{{Binding {fetch}}}\"{fetch_file} EmptyText=\"{empty}\" ChooseText=\"{choose}\" OpenText=\"{open}\" SaveText=\"{save}\" RemoveText=\"{remove}\"{read_only}{plain_state}{extra} />",
                    record = member(data),
                    fetch = picture_fetch(data, field),
                    // Clarity's own words, by key.
                    empty = self.text("@noFile"),
                    choose = self.text("@chooseFile"),
                    open = self.text("@openFile"),
                    save = self.text("@saveFileCopy"),
                    remove = self.text("@removeFile"),
                );
            }
            // A record's picture, which the view model fetches when it is stored.
            WidgetKind::Image if p.bind.as_deref().is_some_and(|b| b.contains('.')) => {
                let (data, field) = p.bind.as_deref().and_then(|b| b.split_once('.')).unwrap_or_default();
                let path = bound.as_deref().unwrap_or_default();
                let read_only = if p.readonly && named.is_none() { " IsReadOnly=\"True\"" } else { "" };
                // A field kept in a store shows a file linked before the record is saved.
                let in_store = self.bound_field(p.bind.as_deref().unwrap_or_default()).is_some_and(|(_, f)| f.modifiers.store.is_some());
                let fetch_file = if in_store { format!(" FetchFile=\"{{Binding {}}}\"", picture_file(data, field)) } else { String::new() };
                let _ = writeln!(
                    out,
                    "{pad}<c:PictureBox {at} Picture=\"{{Binding {path}}}\" Record=\"{{Binding {record}}}\" Fetch=\"{{Binding {fetch}}}\"{fetch_file} EmptyText=\"{empty}\" LoadText=\"{load}\" RemoveText=\"{remove}\"{read_only}{text_state}{extra} />",
                    record = member(data),
                    fetch = picture_fetch(data, field),
                    // Clarity's own words, by key.
                    empty = self.text("@noPicture"),
                    load = self.text("@loadPicture"),
                    remove = self.text("@removePicture"),
                );
            }
            // A key: the picture it points at in the language shown, by its name in the texts.
            WidgetKind::Image if p.source.is_none() && p.bind.is_none() && p.caption.is_some() => {
                let key = self.text(p.caption.as_deref().unwrap_or_default());
                let named = key.trim_start_matches('{').trim_end_matches('}');
                let _ = writeln!(
                    out,
                    "{pad}<Image {at} Source=\"{{Binding Source={{{named}}}, Converter={{x:Static local:Shown.Picture}}}}\" Stretch=\"Uniform\"{plain_state}{extra} />"
                );
            }
            WidgetKind::Image if p.source.is_some() => {
                let source = p.source.as_deref().unwrap_or_default();
                let _ = writeln!(out, "{pad}<Image {at} Source=\"/resources/{}\" Stretch=\"Uniform\"{plain_state}{extra} />", attr(source));
            }
            // A canvas drawn by its draw handler, through the runtime.
            WidgetKind::Canvas if named.is_some() && Self::handler(widget, Event::Draw).is_some() => {
                let (width, height) = p.units.map_or(clarity_spec::widgets::DEFAULT_UNITS, |u| (u.width, u.height));
                let name = member(named.unwrap_or_default());
                let click = match Self::handler(widget, Event::Click) {
                    Some(_) => format!(" Command=\"{{Binding ClickAt_{name}Command}}\""),
                    None => String::new(),
                };
                let _ = writeln!(
                    out,
                    "{pad}<c:CanvasView {at} Drawing=\"{{Binding {name}.Drawing}}\" UnitsWidth=\"{width}\" UnitsHeight=\"{height}\"{click}{plain_state}{extra} />",
                );
            }
            WidgetKind::Image | WidgetKind::Canvas => {
                let what = if p.kind == WidgetKind::Image { "Image" } else { "Canvas" };
                let source = p.source.as_deref().or(p.renderer.as_deref()).unwrap_or_default();
                let _ = writeln!(
                    out,
                    "{pad}<Border {at} Classes=\"placeholder\"{plain_state}{extra}>\n{pad}  <TextBlock Text=\"{}\" />\n{pad}</Border>",
                    attr(format!("{what} {source}").trim())
                );
            }
        }
    }

    // -- Code-behind ---------------------------------------------------------

    /// The events a control has no command for, wired to the ViewModel's commands.
    fn wiring(&self) -> Vec<String> {
        let mut out = Vec::new();
        for widget in &self.parts {
            let Some(name) = &widget.props.name else { continue };
            let control = control(name);
            // A file part says where a copy went, in Clarity's own words.
            if widget.props.kind == WidgetKind::File {
                out.push(format!("{control}.CopySaved += _ => Vm.Say(Level.Info, Clarity.Runtime.Text.Of(\"fileCopySaved\"));"));
            }
            for h in &widget.props.handlers {
                let run = format!("Vm.{}Command.Execute(null)", self.command(h));
                let kind = widget.props.kind;
                // Every input of the library tells when the person changed its value.
                let input = matches!(
                    kind,
                    WidgetKind::Textbox
                        | WidgetKind::Password
                        | WidgetKind::Memo
                        | WidgetKind::Number
                        | WidgetKind::Spinbox
                        | WidgetKind::Slider
                        | WidgetKind::Color
                        | WidgetKind::File
                        | WidgetKind::Tags
                        | WidgetKind::Checkbox
                        | WidgetKind::Switch
                        | WidgetKind::Dropdown
                        | WidgetKind::Radio
                        | WidgetKind::Date
                );
                let line = match h.event {
                    Event::Click => continue,
                    // A toggle tells its change when the person presses it, not when code sets it.
                    Event::Change if kind == WidgetKind::Toggle => format!("{control}.Click += (_, _) => {run};"),
                    // An accordion tells when the person opened or folded a section.
                    Event::Change if kind == WidgetKind::Accordion => format!("{control}.Changed += () => {run};"),
                    Event::Change if input => format!("{control}.ValueChanged += (_, _) => {run};"),
                    // The table chooses an edited row first, so the handler finds it.
                    Event::Change if kind == WidgetKind::Table => format!("{control}.Edited += _ => {run};"),
                    Event::Select if kind == WidgetKind::Table => format!("{control}.Selected += (_, _) => {run};"),
                    Event::Open if kind == WidgetKind::Table => format!("{control}.Opened += (_, _) => {run};"),
                    Event::Change | Event::Select => format!("{control}.SelectionChanged += (_, _) => {run};"),
                    Event::Open => format!("{control}.DoubleTapped += (_, _) => {run};"),
                    Event::Enter => format!("{control}.GotFocus += (_, _) => {run};"),
                    Event::Leave => format!("{control}.LostFocus += (_, _) => {run};"),
                    Event::Submit => format!("Events.OnEnter({control}, () => {run});"),
                    // A wizard's Next asks whether the handler refused, so it keeps the step.
                    Event::Next => format!("{control}.Next = Vm.Next_{};", member(name)),
                    Event::Finish => format!("{control}.Finished += () => {run};"),
                    _ => continue,
                };
                out.push(line);
            }
        }
        out
    }

    fn code_behind(&self, namespace: &str) -> String {
        let name = &self.page.name.name;
        let mut wiring: String = self.wiring().iter().map(|l| format!("        {l}\n")).collect();
        // The language and theme pickers, in the order of the frame.
        for command in &self.commands {
            if let Some(picker) = self.pickers.get(&address(command)) {
                let kind = if command.props.on_click.as_deref() == Some(LANGUAGE) { "Language" } else { "Theme" };
                let _ = writeln!(wiring, "        Pickers.{kind}({picker});");
            }
        }
        format!(
            "{HEADER}
using Avalonia.Controls;
using Clarity.Avalonia;

namespace {namespace};

public partial class {name}View : UserControl
{{
    public {name}View()
    {{
        InitializeComponent();
{wiring}    }}

    {name}ViewModel Vm => ({name}ViewModel)DataContext!;
}}
"
        )
    }

    // -- ViewModel -----------------------------------------------------------

    fn view_model(&self, types: &FileTypes, namespace: &str) -> Result<String, Vec<String>> {
        let name = &self.page.name.name;
        let mut errors = Vec::new();
        let mut ctx = WindowCtx {
            data: self.data.clone(),
            ..WindowCtx::default()
        };
        for widget in &self.parts {
            if let Some(n) = &widget.props.name {
                ctx.parts.insert(n.clone(), PartInfo { bind: widget.props.bind.clone() });
            }
        }
        for command in &self.commands {
            if let Some(n) = &command.props.name {
                ctx.parts.insert(n.clone(), PartInfo::default());
            }
        }

        // The constructor loads every list and the records of references.
        let mut constructor = String::new();
        let mut properties = String::new();
        for (data, entity, list) in &self.data {
            let prop = member(data);
            let field = backing(data);
            if *list {
                let _ = writeln!(constructor, "        {field} = Db.All<{entity}>();");
                let _ = write!(
                    properties,
                    "    public List<{entity}> {prop} {{ get => {field}; set => SetProperty(ref {field}, value); }}\n    List<{entity}> {field};\n\n"
                );
            } else if self.project.entity(entity).is_some_and(|e| e.settings) {
                // Every window shows the same settings.
                let _ = write!(properties, "    public {entity} {prop} => {entity}.Current;\n\n");
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
                (None, _) => "default".into(),
            };
            let (prop, field) = (member(&d.name.name), backing(&d.name.name));
            let _ = write!(properties, "    public {ty_name} {prop} {{ get => {field}; set => SetProperty(ref {field}, value); }}\n    {ty_name} {field} = {start};\n\n");
        }
        // The picture each picture field shows when the record has one stored.
        let mut fetches: BTreeMap<String, (&str, String)> = BTreeMap::new();
        for widget in self.parts.iter().filter(|w| matches!(w.props.kind, WidgetKind::Image | WidgetKind::File)) {
            let Some((data, field)) = widget.props.bind.as_deref().and_then(|b| b.split_once('.')) else { continue };
            let Some(entity) = self.entity_of(data) else { continue };
            fetches.insert(
                picture_fetch(data, field),
                ("byte[]?", format!("() => Db.PictureOf<{}>({}.Id, {})", entity.name.name, member(data), string(entity.column_of(field)))),
            );
            if let Some(store) = entity.field(field).and_then(|f| f.modifiers.store.as_deref()) {
                fetches.insert(picture_file(data, field), ("string, byte[]?", format!("path => Db.StoreFile({}, path)", string(store))));
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
            let _ = write!(properties, "    public Choice[] {entity}Choices {{ get; }}\n\n");
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
                    let _ = write!(properties, "    public List<string> {name} {{ get; }}\n\n");
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
                    .map(|f| format!("{}.TextOf(r.{})", self.shown_converter(target, f).trim_start_matches("local:"), property(&target.name.name, f)))
                    .collect();
                let read = format!("        {name} = Choice.Of(Db.All<{}>(), r => {}, r => [{}]);", target.name.name, display(target), cells.join(", "));
                if lists.insert(name.clone(), read).is_none() {
                    let _ = write!(properties, "    public Choice[] {name} {{ get; }}\n\n");
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
            // A foldable part starts as designed; a popover starts closed.
            let starts = if widget.props.kind == WidgetKind::Popover {
                " { IsVisible = false }"
            } else if widget.props.foldable && widget.props.folded {
                " { IsFolded = true }"
            } else {
                ""
            };
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
            let method = pascal(&decl.name.name);
            match logic::handler(self.project, types, &ctx, decl) {
                // One that takes values runs through the commands of its calls.
                Ok(body) if !decl.params.is_empty() => {
                    let _ = writeln!(handlers, "{body}");
                }
                Ok(body) => {
                    let _ = write!(handlers, "    [RelayCommand]\n    void {method}() => Handle(On{method});\n\n{body}\n");
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
            let _ = write!(events, "    protected override void {hook}() => On{}();\n\n", pascal(&h.handler));
        }
        // The canvases, drawn anew by their draw handlers after every handler.
        let draws = crate::canvas_draws(self.page);
        if !draws.is_empty() {
            let body: String = draws
                .iter()
                .map(|(canvas, handler)| format!("        {c}.BeginDrawing();\n        On{}();\n        {c}.EndDrawing();\n", pascal(handler), c = member(canvas)))
                .collect();
            let _ = write!(events, "    protected override void Redraw()\n    {{\n{body}    }}\n\n");
        }

        // A click on a canvas: where it was, then its handler.
        for widget in self.parts.iter().filter(|w| w.props.kind == WidgetKind::Canvas) {
            let (Some(canvas), Some(click)) = (&widget.props.name, Self::handler(widget, Event::Click)) else { continue };
            let _ = write!(
                events,
                "    [RelayCommand]\n    void ClickAt_{c}(Pointer at) => Handle(() =>\n    {{\n        {c}.Pointer = at;\n        On{}();\n    }});\n\n",
                pascal(&click.handler),
                c = member(canvas)
            );
        }

        // A wizard's Next: its handler, saying whether it refused.
        for widget in self.parts.iter().filter(|w| w.props.kind == WidgetKind::Steps) {
            let (Some(wizard), Some(next)) = (&widget.props.name, Self::handler(widget, Event::Next)) else { continue };
            let Some(decl) = own.iter().find(|d| d.name.name == next.handler) else { continue };
            let values: Vec<String> = next.args.iter().zip(&decl.params).map(|(v, p)| logic::literal(self.project, &p.ty, v)).collect();
            let _ = write!(events, "    public bool Next_{}() => Handle(() => On{}({}));\n\n", member(wizard), pascal(&next.handler), values.join(", "));
        }

        // Windows opened by a click, and the built-in targets.
        let mut opens = BTreeSet::new();
        let mut quit = false;
        let mut close = false;
        let targets = self
            .parts
            .iter()
            .filter_map(|w| w.props.on_click.as_deref().map(|t| (t, w.props.new_window)))
            .chain(self.commands.iter().filter_map(|c| c.props.on_click.as_deref().map(|t| (t, c.props.new_window))));
        for (target, new_window) in targets {
            match target {
                QUIT => quit = true,
                CLOSE => close = true,
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
        // The calls with values its parts wire, a command each.
        for (call, command) in &self.calls {
            let Some(decl) = own.iter().find(|d| d.name.name == call.handler) else { continue };
            let values: Vec<String> = call.args.iter().zip(&decl.params).map(|(v, p)| logic::literal(self.project, &p.ty, v)).collect();
            let _ = write!(clicks, "    [RelayCommand]\n    void {command}() => Handle(() => On{}({}));\n\n", pascal(&call.handler), values.join(", "));
        }
        if quit {
            clicks.push_str("    [RelayCommand]\n    void Quit_() => Handle(QuitApplication);\n\n");
        }
        if close {
            clicks.push_str("    [RelayCommand]\n    void CloseWindow_() => Handle(Close);\n\n");
        }
        // The popovers a click opens.
        for popover in self.page.widgets.iter().filter(|w| w.props.kind == WidgetKind::Popover) {
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
            identity.push_str(&format!("\n    public override global::Avalonia.Size? WindowSize => new({}, {});\n", size.width, size.height));
        }
        let body = format!("{identity}\n{properties}{parts}\n{events}{clicks}{handlers}");
        Ok(format!(
            "{HEADER}
using Clarity.Avalonia;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;

namespace {namespace};

public sealed partial class {name}ViewModel : WindowViewModel
{{
    public {name}ViewModel(Db db) : base(db)
    {{
{constructor}    }}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_style_class_joins_the_classes_of_a_part() {
        assert_eq!(with_class("  <Button x:Name=\"save\" />\n", "primary"), "  <Button Classes=\"primary\" x:Name=\"save\" />\n");
        assert_eq!(with_class("<TextBlock Classes=\"label\" Text=\"a\" />", "loud"), "<TextBlock Classes=\"loud label\" Text=\"a\" />");
    }

    #[test]
    fn attributes() {
        assert_eq!(attr("Tom & \"Jerry\""), "Tom &amp; &quot;Jerry&quot;");
        assert_eq!(attr("{not a binding}"), "{}{not a binding}");
    }
}
