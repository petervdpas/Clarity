//! Each window as an Avalonia view (a `UserControl` in AXAML) with its
//! ViewModel and code-behind, shown in the application's one desktop window
//! (the runtime's `Shell`); only a dialog gets a desktop window of its own. The
//! grid becomes a `Grid` with a `*` column per grid column and the shared
//! row height (`clarity_spec::layout`), rows that hold a growing part as `*`
//! rows; each part binds to the window's data and, when it has a name, to
//! its `PartState`; handlers become ViewModel methods, run as commands.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use clarity_spec::Project;
use clarity_spec::code::check::FileTypes;
use clarity_spec::frame::{ShownFrame, shown};
use clarity_spec::layout::{CONTENT_PADDING, GAP, ROW_HEIGHT, SIDEBAR_COLUMN};
use clarity_spec::model::{App, Command, CommandKind, Entity, Field, FieldType, Page, PageKind, Widget};
use clarity_spec::widgets::{Event, WidgetKind};
use clarity_spec::widgets;

use super::HEADER;
use super::logic::{self, PartInfo, WindowCtx};
use super::names::{backing, control, member, pascal, property, string};
use crate::OutFile;

/// The built-in click targets that are no window: `quit` and `close` work;
/// `language` and `theme` come with translations and themes for C#.
const QUIT: &str = "quit";
const CLOSE: &str = "close";

/// The width of a dialog's view, in pixels; its window fits it.
const DIALOG_WIDTH: u32 = 560;

/// The AXAML, code-behind and ViewModel of `page`.
pub fn emit(project: &Project, page: &Page, types: &FileTypes, namespace: &str) -> Result<Vec<OutFile>, Vec<String>> {
    let app = project.apps().next();
    let frame = shown(app, page);
    let window = Window::new(project, page, app, frame);
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

/// Text as an AXAML attribute value: escaped, and a leading brace kept as text.
fn attr(text: &str) -> String {
    let escaped = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
    if escaped.starts_with('{') { format!("{{}}{escaped}") } else { escaped }
}

/// A menu or button caption: `_` marks an access key in Avalonia.
fn caption(text: &str) -> String {
    attr(&text.replace('_', "__"))
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
fn display(entity: &Entity) -> String {
    entity
        .fields
        .iter()
        .find(|f| matches!(f.ty, FieldType::Text { .. } | FieldType::Email))
        .map_or_else(|| "r.Id.ToString()".to_string(), |f| format!("r.{}", property(&entity.name.name, &f.name.name)))
}

struct Window<'a> {
    project: &'a Project,
    page: &'a Page,
    app: Option<&'a App>,
    frame: ShownFrame<'a>,
    /// The window's data with the app's: name, entity and whether it is a list.
    data: Vec<(String, String, bool)>,
    /// Every part the window shows: its grid, nested parts, the sidebars and the footer.
    parts: Vec<&'a Widget>,
    commands: Vec<&'a Command>,
}

impl<'a> Window<'a> {
    fn new(project: &'a Project, page: &'a Page, app: Option<&'a App>, frame: ShownFrame<'a>) -> Self {
        let mut data: Vec<(String, String, bool)> = Vec::new();
        for d in page.data.iter().chain(app.map(|a| a.data.as_slice()).unwrap_or_default()) {
            if !data.iter().any(|(n, _, _)| *n == d.name.name) {
                data.push((d.name.name.clone(), d.entity.name.clone(), d.list));
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
        Window {
            project,
            page,
            app,
            frame,
            data,
            parts,
            commands,
        }
    }

    fn entity_of(&self, data: &str) -> Option<&'a Entity> {
        let (_, entity, _) = self.data.iter().find(|(n, _, _)| n == data)?;
        self.project.entity(entity)
    }

    /// The field a binding such as `order.number` reads, and its entity.
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
    fn handler(widget: &Widget, event: Event) -> Option<&str> {
        widget.props.handlers.iter().find(|h| h.event == event).map(|h| h.handler.as_str())
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
    fn click_command(&self, handler: Option<&str>, on_click: Option<&str>, new_window: bool) -> Option<String> {
        if let Some(handler) = handler {
            return Some(format!("{}Command", pascal(handler)));
        }
        match on_click? {
            QUIT => Some("QuitCommand".into()),
            CLOSE => Some("CloseWindowCommand".into()),
            target if self.project.pages().any(|p| p.name.name == target) => {
                Some(format!("Open{target}{}Command", if new_window { "InNewWindow" } else { "" }))
            }
            _ => None,
        }
    }

    // -- AXAML ---------------------------------------------------------------

    fn axaml(&self, namespace: &str) -> String {
        let name = &self.page.name.name;
        // A dialog sizes its desktop window to its view.
        let width = if self.page.kind == PageKind::Dialog { format!(" Width=\"{DIALOG_WIDTH}\"") } else { String::new() };
        let mut out = String::new();
        if let Some(menu) = self.frame.menu {
            out.push_str("      <Menu DockPanel.Dock=\"Top\">\n");
            for item in &menu.items {
                self.menu_item(item, 4, &mut out);
            }
            out.push_str("      </Menu>\n");
        }
        if let Some(ribbon) = self.frame.ribbon {
            out.push_str("      <TabControl DockPanel.Dock=\"Top\" Classes=\"ribbon\">\n");
            for tab in &ribbon.tabs {
                let _ = writeln!(out, "        <TabItem Header=\"{}\">", caption(&tab.title));
                out.push_str("          <StackPanel Orientation=\"Horizontal\" Spacing=\"16\">\n");
                for group in &tab.groups {
                    out.push_str("            <StackPanel Spacing=\"4\">\n              <StackPanel Orientation=\"Horizontal\" Spacing=\"4\">\n");
                    for item in &group.items {
                        self.button(item, 8, &mut out);
                    }
                    let _ = writeln!(out, "              </StackPanel>\n              <TextBlock Classes=\"group\" Text=\"{}\" />\n            </StackPanel>", attr(&group.title));
                }
                out.push_str("          </StackPanel>\n        </TabItem>\n");
            }
            out.push_str("      </TabControl>\n");
        }
        if let Some(toolbar) = self.frame.toolbar {
            out.push_str("      <Border DockPanel.Dock=\"Top\" Classes=\"toolbar\">\n        <StackPanel Orientation=\"Horizontal\" Spacing=\"4\">\n");
            for item in &toolbar.items {
                self.button(item, 5, &mut out);
            }
            out.push_str("        </StackPanel>\n      </Border>\n");
        }
        if let Some(footer) = self.frame.footer {
            out.push_str("      <Border DockPanel.Dock=\"Bottom\" Classes=\"footer\">\n        <DockPanel>\n");
            if let Some(status) = &footer.status {
                let _ = writeln!(out, "          <TextBlock Classes=\"status\" Text=\"{}\" />", attr(status));
            }
            self.grid(&footer.widgets, footer.columns, Some(1), false, 5, &mut out);
            out.push_str("        </DockPanel>\n      </Border>\n");
        }
        for (side, dock) in [(self.frame.left, "Left"), (self.frame.right, "Right")] {
            let Some(sidebar) = side else { continue };
            let _ = writeln!(
                out,
                "      <Border DockPanel.Dock=\"{dock}\" Classes=\"sidebar\" Width=\"{}\">",
                sidebar.width * SIDEBAR_COLUMN
            );
            self.grid(&sidebar.widgets, sidebar.width, None, false, 4, &mut out);
            out.push_str("      </Border>\n");
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
        let header = caption(item.props.caption.as_deref().unwrap_or_default());
        if !item.items.is_empty() {
            let _ = writeln!(out, "{pad}<MenuItem Header=\"{header}\">");
            for child in &item.items {
                self.menu_item(child, depth + 1, out);
            }
            let _ = writeln!(out, "{pad}</MenuItem>");
            return;
        }
        let mut attrs = format!("Header=\"{header}\"");
        if let Some(key) = &item.props.shortcut {
            let _ = write!(attrs, " InputGesture=\"{key}\" HotKey=\"{key}\"", key = attr(key));
        }
        if let Some(command) = self.click_command(item.props.handler.as_deref(), item.props.on_click.as_deref(), item.props.new_window) {
            let _ = write!(attrs, " Command=\"{{Binding {command}}}\"");
        }
        let _ = writeln!(out, "{pad}<MenuItem {attrs} />");
    }

    /// A toolbar or ribbon button.
    fn button(&self, item: &Command, depth: usize, out: &mut String) {
        let pad = "  ".repeat(depth);
        if item.props.kind == CommandKind::Separator {
            let _ = writeln!(out, "{pad}<Separator Classes=\"vertical\" />");
            return;
        }
        let mut attrs = format!("Content=\"{}\"", caption(item.props.caption.as_deref().unwrap_or_default()));
        if let Some(command) = self.click_command(item.props.handler.as_deref(), item.props.on_click.as_deref(), item.props.new_window) {
            let _ = write!(attrs, " Command=\"{{Binding {command}}}\"");
        }
        let _ = writeln!(out, "{pad}<Button {attrs} />");
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

    fn widget(&self, widget: &Widget, depth: usize, out: &mut String) {
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
            let _ = write!(extra, " ToolTip.Tip=\"{}\"", attr(tip));
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
                    None if required => attr(&format!("{caption_text} *")),
                    None => attr(&caption_text),
                };
                let _ = writeln!(out, "{pad}<TextBlock {at} Classes=\"label\" Text=\"{text}\"{extra} />");
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
                        let _ = writeln!(out, "{pad}<TextBlock {at} Classes=\"help\" Text=\"{}\"{extra} />", attr(&caption_text));
                    }
                }
            }
            WidgetKind::Heading => {
                let _ = writeln!(out, "{pad}<TextBlock {at} Classes=\"heading\" Text=\"{}\"{plain_state}{extra} />", attr(&caption_text));
            }
            WidgetKind::Text => {
                let text = bound.as_deref().map(shown).unwrap_or_default();
                let _ = writeln!(out, "{pad}<TextBlock {at} Classes=\"value\" Text=\"{text}\"{plain_state}{extra} />");
            }
            WidgetKind::Textbox | WidgetKind::Password | WidgetKind::Memo | WidgetKind::Number => {
                let class = match p.kind {
                    WidgetKind::Memo => " Classes=\"memo\" AcceptsReturn=\"True\" TextWrapping=\"Wrap\"",
                    WidgetKind::Number => " Classes=\"number\"",
                    WidgetKind::Password => " PasswordChar=\"*\"",
                    _ => "",
                };
                let value = match (&bound, named) {
                    (Some(path), _) => format!(" Text=\"{{Binding {path}}}\""),
                    (None, Some(n)) => format!(" Text=\"{{Binding {}.Text}}\"", member(n)),
                    (None, None) => String::new(),
                };
                let placeholder = p.placeholder.as_deref().map(|t| format!(" PlaceholderText=\"{}\"", attr(t))).unwrap_or_default();
                let read_only = if p.readonly && named.is_none() { " IsReadOnly=\"True\"" } else { "" };
                let _ = writeln!(out, "{pad}<TextBox {at}{class}{value}{placeholder}{read_only}{text_state}{extra} />");
            }
            WidgetKind::Checkbox | WidgetKind::Switch => {
                // A switch's Content is a header above it; its caption goes beside it, as on the web.
                let (control, label) = if p.kind == WidgetKind::Switch {
                    ("ToggleSwitch", format!("OnContent=\"{text}\" OffContent=\"{text}\"", text = attr(&caption_text)))
                } else {
                    ("CheckBox", format!("Content=\"{}\"", attr(&caption_text)))
                };
                let value = bound.as_deref().map(|b| format!(" IsChecked=\"{{Binding {b}}}\"")).unwrap_or_default();
                let read_only = if p.readonly { " IsEnabled=\"False\"" } else { "" };
                let _ = writeln!(out, "{pad}<{control} {at} {label}{value}{read_only}{plain_state}{extra} />");
            }
            WidgetKind::Dropdown => {
                let field = p.bind.as_deref().and_then(|b| self.bound_field(b)).map(|(_, f)| &f.ty);
                let items = match field {
                    Some(FieldType::Enum { name }) => format!("{{x:Static local:Choices.{}}}", name.name),
                    Some(FieldType::Ref { entity }) => format!("{{Binding {}Choices}}", entity.name),
                    _ => String::new(),
                };
                let value = bound.as_deref().map(|b| format!(" SelectedValue=\"{{Binding {b}}}\"")).unwrap_or_default();
                let _ = writeln!(
                    out,
                    "{pad}<ComboBox {at} ItemsSource=\"{items}\" DisplayMemberBinding=\"{{Binding Label, x:DataType=c:Choice}}\" SelectedValueBinding=\"{{Binding Value, x:DataType=c:Choice}}\"{value} HorizontalAlignment=\"Stretch\"{plain_state}{extra} />"
                );
            }
            WidgetKind::Radio => {
                let orientation = match p.orientation {
                    Some(clarity_spec::model::Orientation::Vertical) => "Vertical",
                    _ => "Horizontal",
                };
                let _ = writeln!(out, "{pad}<StackPanel {at} Orientation=\"{orientation}\" Spacing=\"12\"{plain_state}{extra}>");
                if let (Some(path), Some(FieldType::Enum { name })) = (&bound, p.bind.as_deref().and_then(|b| self.bound_field(b)).map(|(_, f)| &f.ty)) {
                    for variant in self.project.enum_decl(&name.name).map(|e| e.variants.as_slice()).unwrap_or_default() {
                        let _ = writeln!(
                            out,
                            "{pad}  <RadioButton Content=\"{}\" IsChecked=\"{{Binding {path}, Converter={{x:Static c:Converters.Equal}}, ConverterParameter={{x:Static local:{}.{}}}}}\" />",
                            attr(&variant.label),
                            name.name,
                            variant.name.name
                        );
                    }
                }
                let _ = writeln!(out, "{pad}</StackPanel>");
            }
            WidgetKind::Date => {
                let date_only = p.bind.as_deref().and_then(|b| self.bound_field(b)).is_some_and(|(_, f)| f.ty == FieldType::Date);
                let value = match &bound {
                    Some(b) if date_only => format!(" SelectedDate=\"{{Binding {b}, Converter={{x:Static c:Converters.Date}}}}\""),
                    Some(b) => format!(" SelectedDate=\"{{Binding {b}}}\""),
                    None => String::new(),
                };
                let _ = writeln!(out, "{pad}<CalendarDatePicker {at}{value} HorizontalAlignment=\"Stretch\"{plain_state}{extra} />");
            }
            WidgetKind::Button => {
                let content = match named {
                    Some(n) => format!("{{Binding {}.Caption}}", member(n)),
                    None => caption(&caption_text),
                };
                let mut attrs = format!("Content=\"{content}\"");
                if let Some(command) = self.click_command(Self::handler(widget, Event::Click), p.on_click.as_deref(), p.new_window) {
                    let _ = write!(attrs, " Command=\"{{Binding {command}}}\"");
                }
                if p.default_button {
                    attrs.push_str(" IsDefault=\"True\"");
                }
                if p.cancel {
                    attrs.push_str(" IsCancel=\"True\"");
                }
                if let Some(key) = &p.shortcut {
                    let _ = write!(attrs, " HotKey=\"{}\"", attr(key));
                }
                let _ = writeln!(out, "{pad}<Button {at} {attrs}{plain_state}{extra} />");
            }
            WidgetKind::Form | WidgetKind::Panel => {
                let _ = writeln!(out, "{pad}<Border {at} Classes=\"{}\"{plain_state}{extra}>", if p.kind == WidgetKind::Form { "form" } else { "panel" });
                let _ = writeln!(out, "{pad}  <DockPanel>");
                if let Some(title) = &p.caption {
                    let _ = writeln!(out, "{pad}    <TextBlock DockPanel.Dock=\"Top\" Classes=\"legend\" Text=\"{}\" />", attr(title));
                }
                self.grid(&widget.children, p.width, None, false, depth + 2, out);
                let _ = writeln!(out, "{pad}  </DockPanel>\n{pad}</Border>");
            }
            WidgetKind::Tabs => {
                let _ = writeln!(out, "{pad}<TabControl {at}{plain_state}{extra}>");
                for tab in &widget.pages {
                    let _ = writeln!(out, "{pad}  <TabItem Header=\"{}\">", caption(tab.title.as_deref().unwrap_or_default()));
                    self.grid(&tab.children, p.width, None, false, depth + 2, out);
                    let _ = writeln!(out, "{pad}  </TabItem>");
                }
                let _ = writeln!(out, "{pad}</TabControl>");
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
                let entity = p.bind.as_deref().and_then(|b| self.entity_of(b));
                let selected = named.map(|n| format!(" SelectedItem=\"{{Binding {}.SelectedItem}}\"", member(n))).unwrap_or_default();
                let items = bound.as_deref().map(|b| format!(" ItemsSource=\"{{Binding {b}}}\"")).unwrap_or_default();
                let _ = writeln!(
                    out,
                    "{pad}<DataGrid {at}{items}{selected} AutoGenerateColumns=\"False\" IsReadOnly=\"True\" GridLinesVisibility=\"Horizontal\"{plain_state}{extra}>"
                );
                let _ = writeln!(out, "{pad}  <DataGrid.Columns>");
                if let Some(entity) = entity {
                    for column in &p.columns {
                        let Some(field) = entity.field(column) else { continue };
                        let path = property(&entity.name.name, column);
                        let _ = writeln!(
                            out,
                            "{pad}    <DataGridTextColumn Header=\"{}\" Binding=\"{}\" x:DataType=\"local:{}\" />",
                            attr(&field.label),
                            shown(&path),
                            entity.name.name
                        );
                    }
                }
                let _ = writeln!(out, "{pad}  </DataGrid.Columns>\n{pad}</DataGrid>");
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
                        let _ = writeln!(out, "{pad}        <TextBlock Text=\"{}\" />", shown(&property(&entity.name.name, field)));
                    }
                    let _ = writeln!(out, "{pad}      </StackPanel>\n{pad}    </DataTemplate>\n{pad}  </ListBox.ItemTemplate>");
                }
                let _ = writeln!(out, "{pad}</ListBox>");
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
            for h in &widget.props.handlers {
                let run = format!("Vm.{}Command.Execute(null)", pascal(&h.handler));
                let kind = widget.props.kind;
                let text = matches!(kind, WidgetKind::Textbox | WidgetKind::Password | WidgetKind::Memo | WidgetKind::Number);
                let line = match h.event {
                    Event::Click => continue,
                    Event::Change if text => format!("Events.OnCommit({control}, () => {run});"),
                    Event::Change if matches!(kind, WidgetKind::Checkbox | WidgetKind::Switch) => format!("{control}.IsCheckedChanged += (_, _) => {run};"),
                    Event::Change if kind == WidgetKind::Date => format!("{control}.SelectedDateChanged += (_, _) => {run};"),
                    Event::Change if kind == WidgetKind::Radio => format!("Events.OnChoice({control}, () => {run});"),
                    Event::Change | Event::Select => format!("{control}.SelectionChanged += (_, _) => {run};"),
                    Event::Open => format!("{control}.DoubleTapped += (_, _) => {run};"),
                    Event::Enter => format!("{control}.GotFocus += (_, _) => {run};"),
                    Event::Leave => format!("{control}.LostFocus += (_, _) => {run};"),
                    Event::Submit => format!("Events.OnEnter({control}, () => {run});"),
                    _ => continue,
                };
                out.push(line);
            }
        }
        out
    }

    fn code_behind(&self, namespace: &str) -> String {
        let name = &self.page.name.name;
        let wiring: String = self.wiring().iter().map(|l| format!("        {l}\n")).collect();
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
            } else {
                let _ = write!(
                    properties,
                    "    public {entity} {prop} {{ get => {field}; set => SetProperty(ref {field}, value); }}\n    {entity} {field} = new();\n\n"
                );
            }
        }
        let mut choices: BTreeMap<String, String> = BTreeMap::new();
        for widget in self.parts.iter().filter(|w| w.props.kind == WidgetKind::Dropdown) {
            let Some((_, field)) = widget.props.bind.as_deref().and_then(|b| self.bound_field(b)) else { continue };
            if let FieldType::Ref { entity } = &field.ty
                && let Some(target) = self.project.entity(&entity.name)
            {
                choices.insert(entity.name.clone(), display(target));
            }
        }
        for (entity, shown) in &choices {
            let _ = writeln!(constructor, "        {entity}Choices = Choice.Of(Db.All<{entity}>(), r => {shown});");
            let _ = write!(properties, "    public Choice[] {entity}Choices {{ get; }}\n\n");
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
                && matches!(widget.props.kind, WidgetKind::Button | WidgetKind::Label | WidgetKind::Checkbox | WidgetKind::Switch | WidgetKind::Heading)
            {
                args.push(format!("caption: {}", string(c)));
            }
            if widget.props.readonly {
                args.push("readOnly: true".into());
            }
            let help = self.help_of(n);
            if !help.is_empty() {
                args.push(format!("help: {}", string(&help)));
            }
            let _ = writeln!(parts, "    public PartState {} {{ get; }} = new({});", member(n), args.join(", "));
        }
        for command in &self.commands {
            let Some(n) = &command.props.name else { continue };
            if seen.insert(n.clone()) {
                let caption = command.props.caption.as_deref().map(|c| format!(", caption: {}", string(c))).unwrap_or_default();
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
                    "    [RelayCommand]\n    void Open{target}InNewWindow() => Handle(() => Open(new {target}ViewModel(Db), newWindow: true));\n\n"
                );
            } else {
                let _ = write!(clicks, "    [RelayCommand]\n    void Open{target}() => Handle(() => Open(new {target}ViewModel(Db)));\n\n");
            }
        }
        if quit {
            clicks.push_str("    [RelayCommand]\n    void Quit() => Handle(QuitApplication);\n\n");
        }
        if close {
            clicks.push_str("    [RelayCommand]\n    void CloseWindow() => Handle(Close);\n\n");
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let title = self.page.title.clone().unwrap_or_else(|| name.clone());
        let mut identity = format!("    public override string Title => {};\n", string(&title));
        if self.page.kind == PageKind::Dialog {
            identity.push_str("\n    public override bool IsDialog => true;\n");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes() {
        assert_eq!(attr("Tom & \"Jerry\""), "Tom &amp; &quot;Jerry&quot;");
        assert_eq!(attr("{not a binding}"), "{}{not a binding}");
        assert_eq!(caption("Save_all"), "Save__all");
    }
}
