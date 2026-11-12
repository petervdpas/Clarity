//! Starter templates for new windows: "form for an entity" and "browse for
//! an entity". A template produces an ordinary window made of parts on the
//! snap grid, which is then designed like any other window.

use serde::Deserialize;
use ts_rs::TS;

use crate::names::camel_case;
use crate::model::{DEFAULT_PAGE_COLUMNS, Entity, Field, FieldType, Project, WidgetProps, default_label};
use crate::model::Handler;

mod signin;
use crate::widgets::{self, Event, WidgetKind, new_props};

/// What a new window starts with.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum PageTemplate {
    /// An empty window, showing an empty view in its content.
    Blank,
    /// An empty view, which a window's content shows (DESIGN 5.33).
    View,
    /// The app's splash screen: its name and a status code sets while the app starts.
    Splash,
    /// A toolbox (docs/IMAGE-PLAN.md): a painting canvas's tools to start from, on a grid of 4.
    Toolbox,
    /// The sign-in window (docs/APP-CAPABILITIES-PLAN.md step 4): a name
    /// and a password, labeled, and a button that signs in with them.
    SignIn,
    /// A label and an input for every field of the entity, and Save and
    /// Close buttons.
    Form { entity: String },
    /// A filter for the first text field, a table of the entity, and New
    /// and Open buttons.
    Browse { entity: String },
    /// A dialog with a form of every setting of the project (DESIGN 5.17),
    /// labeled, and a Close button.
    Settings,
}

/// Columns a browse template shows, at most.
const BROWSE_COLUMNS: usize = 5;
/// Rows of the table in a browse window.
const TABLE_ROWS: u32 = 8;



fn find_entity<'a>(project: &'a Project, name: &str) -> Result<&'a Entity, String> {
    project
        .entity(name)
        .ok_or_else(|| format!("'{name}' is not an entity"))
}

/// `Customer` -> `customer`, `OrderLine` -> `orderLine`.
fn data_name(entity: &str) -> String {
    crate::names::camel_case(entity)
}

/// `customer` -> `customers`, `category` -> `categories`.
pub(crate) fn plural(name: &str) -> String {
    if let Some(stem) = name.strip_suffix('y') {
        format!("{stem}ies")
    } else if name.ends_with('s') || name.ends_with('x') {
        format!("{name}es")
    } else {
        format!("{name}s")
    }
}


/// Width of the input for a field, at most `room` columns.
fn input_width(kind: WidgetKind, ty: &FieldType, room: u32) -> u32 {
    let wide = match ty {
        FieldType::Text { max_length } => max_length.is_none_or(|n| n > 40),
        FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme => true,
        _ => false,
    };
    let width = if wide { 6 } else { widgets::default_size(kind).0 };
    width.min(room).max(1)
}

/// A button named after its caption: `Save` is `save`, `New order` is `newOrder`.
fn button(caption: &str, col: u32, row: u32) -> WidgetProps {
    let mut props: WidgetProps = new_props(WidgetKind::Button, col, row);
    props.caption = Some(caption.to_string());
    props.name = Some(camel_case(caption));
    props
}

/// Rows of a memo for text without a maximum length.
const MEMO_ROWS: u32 = 3;

/// Columns of the label in front of an input.
const LABEL: u32 = 3;

/// The parts that show one field at a cell, with `room` columns to the
/// edge of the container: a label for its input when there is room for
/// both, the input named `input_name`, and a help text when the field has
/// one and there is room left. The input comes first.
pub fn field_parts(
    field: &Field,
    data: &str,
    input_name: &str,
    col: u32,
    row: u32,
    room: u32,
) -> Vec<WidgetProps> {
    let kind = match field.ty {
        FieldType::Text { max_length: None } => WidgetKind::Memo,
        _ => widgets::widget_for(&field.ty),
    };
    let labelled = room > LABEL + 1;
    let at = if labelled { col + LABEL } else { col };
    let room = room - (at - col);
    let mut input = new_props(kind, at, row);
    input.width = input_width(kind, &field.ty, room);
    if kind == WidgetKind::Memo {
        input.height = MEMO_ROWS;
    }
    input.caption = None;
    let input_name = camel_case(input_name);
    input.name = Some(input_name.clone());
    input.bind = Some(format!("{data}.{}", field.name.name));
    let mut parts = vec![input.clone()];
    if labelled {
        let mut label = new_props(WidgetKind::Label, col, row);
        label.caption = Some(field.label.clone());
        label.width = LABEL;
        label.name = Some(format!("{input_name}Label"));
        label.for_part = Some(input_name.clone());
        parts.push(label);
    }
    let left = room - input.width;
    if field.modifiers.help.is_some() && left >= 2 {
        let mut help = new_props(WidgetKind::Help, at + input.width, row);
        help.width = left;
        help.name = Some(format!("{input_name}Help"));
        help.for_part = Some(input_name.clone());
        parts.push(help);
    }
    parts
}


/// The field a browse window filters on: the first text field.
fn browse_filter(entity: &Entity) -> Option<&Field> {
    entity.fields.iter().find(|f| f.ty.is_textual())
}

/// The handler that runs when the filter changes.
const FILTER_HANDLER: &str = "filterChanged";


/// The handler a browse window gets for its `on return` event.
const RETURN_HANDLER: &str = "windowReturned";

/// The code behind a template starts with, if any. A browse window finds
/// its records with the filter when it changes, and again when a window it
/// opened (a form) has closed; without a filter it reloads its list.
pub fn code_from_template(project: &Project, name: &str, template: &PageTemplate) -> Result<Option<String>, String> {
    match template {
        PageTemplate::Browse { entity } => {
            let entity = find_entity(project, entity)?;
            let data = plural(&data_name(&entity.name.name));
            let returned = format!(
                "code {name}\n    handler {RETURN_HANDLER}\n        // A window opened from here may have changed the records.\n"
            );
            let Some(field) = browse_filter(entity) else {
                return Ok(Some(format!("{returned}        reload {data}")));
            };
            let var: String = data.chars().take(1).collect();
            let field = &field.name.name;
            let filter = camel_case(&format!("{field}_filter"));
            Ok(Some(format!(
                "{returned}        {FILTER_HANDLER}()\n\n    handler {FILTER_HANDLER}\n        {data} = find {var} in {} where {var}.{field} contains {filter}.text order by {var}.{field}",
                entity.name.name
            )))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_data_after_the_entity() {
        assert_eq!(data_name("Customer"), "customer");
        assert_eq!(data_name("OrderLine"), "orderLine");
        assert_eq!(plural("customer"), "customers");
        assert_eq!(plural("category"), "categories");
        assert_eq!(plural("address"), "addresses");
    }
}

// ---------------------------------------------------------------------------
// New windows, views and the app, as what their files hold

/// What a new window, view, splash, toolbox or app starts as: each page
/// (or the app) for its own file in `design/`, and the code blocks it starts
/// with, for its module's file.
#[derive(Debug, Clone, PartialEq)]
pub struct Made {
    pub designed: Vec<crate::design_edit::Designed>,
    pub code: Vec<String>,
}

/// A new page of `kind` with nothing on it yet.
fn page_of(name: &str, kind: crate::model::PageKind, title: Option<String>) -> crate::model::Page {
    use crate::model::{Frame, Ident, Page};
    Page {
        name: Ident { name: name.to_string(), span: crate::span::Span::default() },
        kind,
        title,
        size: None,
        style: None,
        data: Vec::new(),
        columns: DEFAULT_PAGE_COLUMNS,
        cell: None,
        widgets: Vec::new(),
        frame: Frame::default(),
        handlers: Vec::new(),
        gives: None,
        tells: Vec::new(),
        allow: None,
        allow_condition: None,
        span: crate::span::Span::default(),
    }
}

fn widget_of(props: WidgetProps) -> crate::model::Widget {
    crate::model::Widget { props, children: Vec::new(), pages: Vec::new(), menu: None, levels: Vec::new(), selects: Vec::new(), span: crate::span::Span::default() }
}

/// Data of a window: a record or a list of records of `entity`.
fn data_of(name: &str, entity: &str, list: bool) -> crate::model::DataDecl {
    use crate::model::{DataDecl, DataKind, Ident};
    let entity = Ident { name: entity.to_string(), span: crate::span::Span::default() };
    let of = if list { DataKind::List { entity } } else { DataKind::Record { entity } };
    DataDecl { name: Ident { name: name.to_string(), span: crate::span::Span::default() }, of, made: false, span: crate::span::Span::default() }
}

/// Whether `name` is free for a new window, view or app of the project.
fn free_name(project: &Project, name: &str) -> Result<(), String> {
    if !widgets::is_identifier(name) {
        return Err(format!("'{name}' is not a valid window name"));
    }
    let taken = project.pages().any(|p| p.name.name == name) || project.apps().any(|a| a.name.name == name) || project.entities().any(|e| e.name.name == name) || project.files.iter().any(|f| f.enums.iter().any(|e| e.name.name == name));
    if taken {
        return Err(format!("'{name}' is already declared in this project"));
    }
    Ok(())
}

/// A new window, view, splash or toolbox made from `template`.
pub fn new_window(project: &Project, name: &str, template: &PageTemplate) -> Result<Made, String> {
    use crate::design_edit::Designed;
    use crate::model::PageKind;
    free_name(project, name)?;
    let title = default_label(name);
    let alone = |page| Ok(Made { designed: vec![Designed::Page(page)], code: Vec::new() });
    match template {
        PageTemplate::View => alone(page_of(name, PageKind::View, Some(title))),
        PageTemplate::Splash => {
            // Sized as a splash is: its name and a status that code sets as the app starts.
            let mut page = page_of(name, PageKind::Splash, None);
            page.size = Some(crate::model::WindowSize { width: 480, height: 280 });
            let key = camel_case(name);
            let mut heading = new_props(WidgetKind::Heading, 1, 2);
            heading.caption = Some(format!("@{key}Title"));
            heading.name = Some("titleHeading".into());
            heading.width = DEFAULT_PAGE_COLUMNS;
            let mut status = new_props(WidgetKind::Label, 1, 5);
            status.caption = Some(format!("@{key}Starting"));
            status.name = Some("status".into());
            status.width = DEFAULT_PAGE_COLUMNS;
            page.widgets = vec![widget_of(heading), widget_of(status)];
            alone(page)
        }
        PageTemplate::Toolbox => alone(toolbox_page(name)),
        PageTemplate::SignIn => signin::signin_window(project, name, &title),
        PageTemplate::Blank => {
            free_name(project, &format!("{name}View"))?;
            Ok(in_content_of(page_of(name, PageKind::Window, Some(title)), None))
        }
        PageTemplate::Form { entity } => {
            free_name(project, &format!("{name}View"))?;
            let entity = find_entity(project, entity)?;
            Ok(in_content_of(form_model(name, &title, entity), None))
        }
        PageTemplate::Browse { entity } => {
            free_name(project, &format!("{name}View"))?;
            let found = find_entity(project, entity)?;
            let columns: Vec<&str> = found.fields.iter().filter(|f| !matches!(f.ty, FieldType::List { .. })).take(BROWSE_COLUMNS).map(|f| f.name.name.as_str()).collect();
            let code = code_from_template(project, name, template)?;
            Ok(in_content_of(browse_model(name, &title, found, &columns), code.as_deref()))
        }
        // A dialog holds its parts itself.
        PageTemplate::Settings => {
            let settings = project.entities().find(|e| e.settings).ok_or("the project has no settings; write a 'settings' block first")?;
            alone(settings_model(name, &title, settings))
        }
    }
}

/// A new page of `kind`, titled after its name.
pub fn new_page(project: &Project, name: &str, kind: crate::model::PageKind) -> Result<Made, String> {
    free_name(project, name)?;
    Ok(Made { designed: vec![crate::design_edit::Designed::Page(page_of(name, kind, Some(default_label(name))))], code: Vec::new() })
}

/// A new app, titled after its name; a project has one.
pub fn new_app(project: &Project, name: &str) -> Result<Made, String> {
    if !widgets::is_identifier(name) {
        return Err(format!("'{name}' is not a valid app name"));
    }
    if project.apps().next().is_some() {
        return Err("this project already has an app".into());
    }
    free_name(project, name)?;
    let app = crate::model::App {
        name: crate::model::Ident { name: name.to_string(), span: crate::span::Span::default() },
        title: Some(default_label(name)),
        main: None,
        splash: None,
        splash_for: None,
        data: Vec::new(),
        frame: crate::model::Frame::default(),
        handlers: Vec::new(),
        span: crate::span::Span::default(),
    };
    Ok(Made { designed: vec![crate::design_edit::Designed::App(app)], code: Vec::new() })
}

/// A painting canvas's tools to start from, on a grid of 4 (docs/IMAGE-PLAN.md).
fn toolbox_page(name: &str) -> crate::model::Page {
    let mut page = page_of(name, crate::model::PageKind::Toolbox, None);
    page.columns = 4;
    let tool = |name: &str, does: &str, col: u32, row: u32| {
        let mut props = new_props(WidgetKind::Tool, col, row);
        props.name = Some(name.to_string());
        props.does = Some(does.to_string());
        widget_of(props)
    };
    let group = |name: &str, col: u32, tools: Vec<crate::model::Widget>| {
        let mut props = new_props(WidgetKind::ToolGroup, col, 1);
        props.name = Some(name.to_string());
        let mut w = widget_of(props);
        w.children = tools;
        w
    };
    let wide = |kind: WidgetKind, name: &str, row: u32, width: u32| {
        let mut props = new_props(kind, 1, row);
        props.name = Some(name.to_string());
        props.width = width;
        widget_of(props)
    };
    page.widgets = vec![
        group("selectGroup", 1, vec![tool("selectTool", "select", 1, 1), tool("ellipseSelectTool", "ellipseSelect", 1, 1), tool("lassoTool", "lasso", 1, 1)]),
        group("drawGroup", 2, vec![tool("penTool", "pen", 1, 1), tool("pencilTool", "pencil", 1, 1), tool("sprayTool", "spray", 1, 1)]),
        tool("eraserTool", "eraser", 3, 1),
        tool("fillTool", "fill", 4, 1),
        wide(WidgetKind::ToolSize, "sizeOption", 2, 4),
        wide(WidgetKind::ToolOpacity, "opacityOption", 3, 4),
        wide(WidgetKind::ToolColors, "colorOption", 4, 2),
    ];
    page
}

/// A form for `entity`: an input with its label for every field in a form
/// bound to the record, then Save and Close.
fn form_model(name: &str, title: &str, entity: &Entity) -> crate::model::Page {
    let data = data_name(&entity.name.name);
    let columns = DEFAULT_PAGE_COLUMNS;
    let mut page = page_of(name, crate::model::PageKind::Window, Some(title.to_string()));
    page.data.push(data_of(&data, &entity.name.name, false));
    let mut fields = Vec::new();
    let mut row = 1;
    for field in entity.fields.iter().filter(|f| !matches!(f.ty, FieldType::List { .. })) {
        // Inputs are named after their field, for the code behind.
        let mut parts = field_parts(field, &data, &field.name.name, 1, row, columns);
        row += parts[0].height;
        parts.sort_by_key(|p| p.col);
        fields.extend(parts.into_iter().map(widget_of));
    }
    let mut form = new_props(WidgetKind::Form, 1, 1);
    form.name = Some(camel_case(&format!("{data}_form")));
    form.width = columns;
    form.height = (row - 1).max(1);
    form.bind = Some(data.clone());
    let mut form = widget_of(form);
    form.children = fields;
    // Enter saves and Escape closes.
    let row = row + 1;
    let mut save = button("Save", columns - 3, row);
    save.default_button = true;
    let mut close = button("Close", columns - 1, row);
    close.cancel = true;
    page.widgets = vec![form, widget_of(save), widget_of(close)];
    page
}

/// A browse window for `entity`: a filter on its first text field, a table
/// of its records, and New and Open.
fn browse_model(name: &str, title: &str, entity: &Entity, columns: &[&str]) -> crate::model::Page {
    let data = plural(&data_name(&entity.name.name));
    let grid = DEFAULT_PAGE_COLUMNS;
    let mut page = page_of(name, crate::model::PageKind::Window, Some(title.to_string()));
    page.data.push(data_of(&data, &entity.name.name, true));
    page.handlers.push(Handler { event: Event::Return, handler: RETURN_HANDLER.to_string(), args: Vec::new() });
    let filter = browse_filter(entity);
    if let Some(field) = filter {
        let input_name = camel_case(&format!("{}_filter", field.name.name));
        let mut caption = new_props(WidgetKind::Label, 1, 1);
        caption.caption = Some(field.label.clone());
        caption.width = 2;
        caption.name = Some(format!("{input_name}Label"));
        caption.for_part = Some(input_name.clone());
        let mut input = new_props(WidgetKind::Textbox, 3, 1);
        input.width = 4;
        input.name = Some(input_name);
        input.handlers.push(Handler { event: Event::Change, handler: FILTER_HANDLER.to_string(), args: Vec::new() });
        page.widgets.push(widget_of(caption));
        page.widgets.push(widget_of(input));
    }
    let table_row = if filter.is_some() { 2 } else { 1 };
    let mut table = new_props(WidgetKind::Table, 1, table_row);
    table.width = grid;
    table.height = TABLE_ROWS;
    table.bind = Some(data.clone());
    table.name = Some(camel_case(&format!("{data}_table")));
    table.columns = columns.iter().map(|c| c.to_string()).collect();
    page.widgets.push(widget_of(table));
    let buttons = table_row + TABLE_ROWS;
    page.widgets.push(widget_of(button("New", grid - 3, buttons)));
    page.widgets.push(widget_of(button("Open", grid - 1, buttons)));
    page
}

/// A settings dialog: a form bound to `settings` with a label column, an
/// input for each setting (a spin box for a whole number between two ends,
/// a check box that labels itself for yes or no), and a Close button.
fn settings_model(name: &str, title: &str, settings: &Entity) -> crate::model::Page {
    use crate::parser::SETTINGS_DATA;
    let columns = DEFAULT_PAGE_COLUMNS;
    let mut page = page_of(name, crate::model::PageKind::Dialog, Some(title.to_string()));
    let mut inputs = Vec::new();
    for (i, field) in settings.fields.iter().enumerate() {
        let row = i as u32 + 1;
        let kind = match &field.ty {
            FieldType::Integer if field.modifiers.min.is_some() && field.modifiers.max.is_some() => WidgetKind::Spinbox,
            other => widgets::widget_for(other),
        };
        let mut input = new_props(kind, LABEL + 1, row);
        input.name = Some(camel_case(&format!("{} box", field.name.name)));
        input.bind = Some(format!("{SETTINGS_DATA}.{}", field.name.name));
        input.width = input_width(kind, &field.ty, columns - LABEL);
        if matches!(kind, WidgetKind::Checkbox | WidgetKind::Switch) {
            input.caption = Some(field.modifiers.label.clone().unwrap_or_else(|| default_label(&field.name.name)));
            input.width = 6.min(columns - LABEL);
        }
        inputs.push(widget_of(input));
    }
    let mut form = new_props(WidgetKind::Form, 1, 1);
    form.name = Some(camel_case(&format!("{SETTINGS_DATA} form")));
    form.width = columns;
    form.height = (settings.fields.len() as u32).max(1);
    form.bind = Some(SETTINGS_DATA.to_string());
    form.labels = Some(LABEL);
    let mut close = button("Close", columns - 1, form.height + 2);
    close.name = Some("closeButton".to_string());
    close.cancel = true;
    close.on_click = Some("close".to_string());
    let mut form = widget_of(form);
    form.children = inputs;
    page.widgets = vec![form, widget_of(close)];
    page
}

/// A new window as every window is built (DESIGN 5.33): window, content,
/// view. The window keeps its title, size, look, events, frame and
/// slide-outs and shows the view `<name>View` in its content `main`; the
/// view holds its data, grid and parts, and the handlers nothing of the
/// window wires go with them into the view's code.
fn in_content_of(page: crate::model::Page, code: Option<&str>) -> Made {
    use crate::design_edit::Designed;
    use crate::model::{PageKind, WidgetPage};
    let name = page.name.name.clone();
    let view_name = format!("{name}View");
    let columns = page.columns;
    let (slideouts, parts): (Vec<_>, Vec<_>) = page.widgets.iter().cloned().partition(|w| w.props.kind == WidgetKind::Slideout);
    let rows = parts.iter().filter(|w| !widgets::off_grid(w.props.kind)).map(|w| w.props.row + w.props.height - 1).max().unwrap_or(1);
    let mut view = page_of(&view_name, PageKind::View, None);
    view.data = page.data.clone();
    view.columns = columns;
    view.widgets = parts;
    let mut content = new_props(WidgetKind::Content, 1, 1);
    content.name = Some("main".into());
    content.width = columns;
    content.height = rows;
    let mut content = widget_of(content);
    content.pages.push(WidgetPage { title: None, columns: None, view: Some(view_name.clone()), tells: Vec::new(), color: None, children: Vec::new(), span: crate::span::Span::default() });
    let mut window = page;
    window.data = Vec::new();
    window.widgets = std::iter::once(content).chain(slideouts).collect();
    // The handlers the window wires stay its own; the rest go with the view.
    let mut wired: Vec<String> = window.handlers.iter().map(|h| h.handler.clone()).collect();
    fn of_parts(widgets: &[crate::model::Widget], out: &mut Vec<String>) {
        for w in widgets {
            out.extend(w.props.handlers.iter().map(|h| h.handler.clone()));
            of_parts(&w.children, out);
        }
    }
    of_parts(&window.widgets[1..], &mut wired);
    let code = code.map(|code| split_code(code, &name, &view_name, &wired)).unwrap_or_default();
    Made { designed: vec![Designed::Page(window), Designed::Page(view)], code }
}

/// `code`, a block of the window `name`, split: the handlers in `wired`
/// stay the window's, the others go to the code of `view`.
fn split_code(code: &str, name: &str, view: &str, wired: &[String]) -> Vec<String> {
    let mut handlers: Vec<Vec<&str>> = Vec::new();
    let mut comments: Vec<&str> = Vec::new();
    for line in code.lines().skip(1) {
        if line.trim_start().starts_with("//") && line.starts_with("    ") && !line.starts_with("     ") {
            comments.push(line);
        } else if line.starts_with("    handler ") {
            let mut h = std::mem::take(&mut comments);
            h.push(line);
            handlers.push(h);
        } else if let Some(last) = handlers.last_mut() {
            last.push(line);
        }
    }
    let name_of = |h: &Vec<&str>| h.iter().find(|l| l.starts_with("    handler ")).map(|l| l["    handler ".len()..].chars().take_while(|c| c.is_alphanumeric()).collect::<String>()).unwrap_or_default();
    let (stay, go): (Vec<_>, Vec<_>) = handlers.into_iter().partition(|h| wired.contains(&name_of(h)));
    let block = |owner: &str, hs: &[Vec<&str>]| format!("code {owner}\n{}", hs.iter().map(|h| h.join("\n").trim_end().to_string()).collect::<Vec<_>>().join("\n\n"));
    let mut out = Vec::new();
    if !stay.is_empty() {
        out.push(block(name, &stay));
    }
    if !go.is_empty() {
        out.push(block(view, &go));
    }
    out
}
