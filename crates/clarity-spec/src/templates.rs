//! Starter templates for new windows: "form for an entity" and "browse for
//! an entity". A template produces an ordinary window made of parts on the
//! snap grid, which is then designed like any other window.

use serde::Deserialize;
use ts_rs::TS;

use crate::lexer::quote;
use crate::names::camel_case;
use crate::model::{DEFAULT_PAGE_COLUMNS, Entity, Field, FieldType, Project, WidgetProps, default_label};
use crate::model::Handler;
use crate::widgets::{self, Event, WidgetKind, new_props, widget_line};

/// What a new window starts with.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum PageTemplate {
    /// An empty window.
    Blank,
    /// A label and an input for every field of the entity, and Save and
    /// Close buttons.
    Form { entity: String },
    /// A filter for the first text field, a table of the entity, and New
    /// and Open buttons.
    Browse { entity: String },
}

/// Columns a browse template shows, at most.
const BROWSE_COLUMNS: usize = 5;
/// Rows of the table in a browse window.
const TABLE_ROWS: u32 = 8;

/// The text of a new page made from a template, without indentation.
pub fn page_from_template(
    project: &Project,
    name: &str,
    template: &PageTemplate,
) -> Result<String, String> {
    let title = default_label(name);
    match template {
        PageTemplate::Blank => Ok(format!("page {name}: window\n    title {}", quote(&title))),
        PageTemplate::Form { entity } => {
            let entity = find_entity(project, entity)?;
            Ok(form_page(name, &title, entity))
        }
        PageTemplate::Browse { entity } => {
            let entity = find_entity(project, entity)?;
            let columns: Vec<&str> = entity
                .fields
                .iter()
                .filter(|f| !matches!(f.ty, FieldType::List { .. }))
                .take(BROWSE_COLUMNS)
                .map(|f| f.name.name.as_str())
                .collect();
            Ok(browse_page(name, &title, entity, &columns))
        }
    }
}

fn find_entity<'a>(project: &'a Project, name: &str) -> Result<&'a Entity, String> {
    project
        .entity(name)
        .ok_or_else(|| format!("'{name}' is not an entity"))
}

/// `Customer` -> `customer`, `OrderLine` -> `order_line`.
fn data_name(entity: &str) -> String {
    let mut out = String::new();
    for (i, c) in entity.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
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

/// The widget lines of a window body, indented one level.
fn body(lines: &[String]) -> String {
    lines.iter().map(|l| format!("    {l}")).collect::<Vec<_>>().join("\n")
}

/// Width of the input for a field, at most `room` columns.
fn input_width(kind: WidgetKind, ty: &FieldType, room: u32) -> u32 {
    let wide = match ty {
        FieldType::Text { max_length } => max_length.is_none_or(|n| n > 40),
        FieldType::Email | FieldType::Url => true,
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

fn form_page(name: &str, title: &str, entity: &Entity) -> String {
    let data = data_name(&entity.name.name);
    let columns = DEFAULT_PAGE_COLUMNS;
    let mut fields = Vec::new();
    let mut row = 1;
    for field in entity.fields.iter().filter(|f| !matches!(f.ty, FieldType::List { .. })) {
        // Inputs are named after their field, for the code behind.
        let mut parts = field_parts(field, &data, &field.name.name, 1, row, columns);
        row += parts[0].height;
        parts.sort_by_key(|p| p.col);
        fields.extend(parts.iter().map(widget_line));
    }
    let mut form = new_props(WidgetKind::Form, 1, 1);
    form.name = Some(camel_case(&format!("{data}_form")));
    form.width = columns;
    form.height = (row - 1).max(1);
    form.bind = Some(data.clone());
    let mut lines = vec![widget_line(&form)];
    lines.extend(fields.iter().map(|l| format!("    {l}")));
    // Enter saves and Escape closes.
    let row = row + 1;
    let mut save = button("Save", columns - 3, row);
    save.default_button = true;
    let mut close = button("Close", columns - 1, row);
    close.cancel = true;
    lines.push(widget_line(&save));
    lines.push(widget_line(&close));
    format!(
        "page {name}: window\n    title {}\n    data {data}: {}\n\n{}",
        quote(title),
        entity.name.name,
        body(&lines)
    )
}

/// The field a browse window filters on: the first text field.
fn browse_filter(entity: &Entity) -> Option<&Field> {
    entity.fields.iter().find(|f| f.ty.is_textual())
}

/// The handler that runs when the filter changes.
const FILTER_HANDLER: &str = "filterChanged";

fn browse_page(name: &str, title: &str, entity: &Entity, columns: &[&str]) -> String {
    let data = plural(&data_name(&entity.name.name));
    let grid = DEFAULT_PAGE_COLUMNS;
    let mut lines = Vec::new();
    // The filter: a label and a text box whose handler finds the records.
    let filter = browse_filter(entity);
    if let Some(field) = filter {
        let input_name = camel_case(&format!("{}_filter", field.name.name));
        let mut caption = new_props(WidgetKind::Label, 1, 1);
        caption.caption = Some(field.label.clone());
        caption.width = 2;
        caption.name = Some(format!("{input_name}Label"));
        caption.for_part = Some(input_name.clone());
        lines.push(widget_line(&caption));
        let mut input = new_props(WidgetKind::Textbox, 3, 1);
        input.width = 4;
        input.name = Some(input_name);
        input.handlers.push(Handler {
            event: Event::Change,
            handler: FILTER_HANDLER.to_string(),
            args: Vec::new(),
        });
        lines.push(widget_line(&input));
    }
    let table_row = if filter.is_some() { 2 } else { 1 };
    let mut table = new_props(WidgetKind::Table, 1, table_row);
    table.width = grid;
    table.height = TABLE_ROWS;
    table.bind = Some(data.clone());
    table.name = Some(camel_case(&format!("{data}_table")));
    table.columns = columns.iter().map(|c| c.to_string()).collect();
    lines.push(widget_line(&table));
    let buttons = table_row + TABLE_ROWS;
    lines.push(widget_line(&button("New", grid - 3, buttons)));
    lines.push(widget_line(&button("Open", grid - 1, buttons)));
    format!(
        "page {name}: window\n    title {}\n    data {data}: list<{}>\n    on return: {RETURN_HANDLER}\n\n{}",
        quote(title),
        entity.name.name,
        body(&lines)
    )
}

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
        assert_eq!(data_name("OrderLine"), "order_line");
        assert_eq!(plural("customer"), "customers");
        assert_eq!(plural("category"), "categories");
        assert_eq!(plural("address"), "addresses");
    }
}
