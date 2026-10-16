//! Starter templates for new windows: "form for an entity" and "browse for
//! an entity". A template produces an ordinary window made of parts on the
//! snap grid, which is then designed like any other window.

use serde::Deserialize;
use ts_rs::TS;

use crate::lexer::quote;
use crate::model::{DEFAULT_PAGE_COLUMNS, Entity, FieldType, Project, WidgetProps, default_label};
use crate::widgets::{self, WidgetKind, new_props, widget_line};

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
            let filter: Vec<&str> = entity
                .fields
                .iter()
                .find(|f| f.ty.is_textual())
                .map(|f| vec![f.name.name.as_str()])
                .unwrap_or_default();
            Ok(browse_page(name, &title, entity, &columns, &filter))
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
fn plural(name: &str) -> String {
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

/// A button named after its caption: `Save` is `save`.
fn button(caption: &str, col: u32, row: u32) -> String {
    let mut props: WidgetProps = new_props(WidgetKind::Button, col, row);
    props.caption = Some(caption.to_string());
    props.name = Some(caption.to_lowercase());
    widget_line(&props)
}

fn form_page(name: &str, title: &str, entity: &Entity) -> String {
    const LABEL: u32 = 3;
    let data = data_name(&entity.name.name);
    let mut lines = Vec::new();
    let columns = DEFAULT_PAGE_COLUMNS;
    let mut row = 1;
    // A caption label and a bound input per field, one field per row.
    for field in entity.fields.iter().filter(|f| !matches!(f.ty, FieldType::List { .. })) {
        let mut caption = new_props(WidgetKind::Label, 1, row);
        caption.caption = Some(field.label.clone());
        caption.width = LABEL;
        lines.push(widget_line(&caption));
        let kind = widgets::widget_for(&field.ty);
        let mut input = new_props(kind, 1 + LABEL, row);
        input.width = input_width(kind, &field.ty, columns - LABEL);
        input.caption = None;
        // Inputs are named after their field, for the code behind.
        input.name = Some(field.name.name.clone());
        input.bind = Some(format!("{data}.{}", field.name.name));
        lines.push(widget_line(&input));
        row += 1;
    }
    let row = row + 1;
    lines.push(button("Save", columns - 3, row));
    lines.push(button("Close", columns - 1, row));
    format!(
        "page {name}: window\n    title {}\n    data {data}: {}\n\n{}",
        quote(title),
        entity.name.name,
        body(&lines)
    )
}

fn browse_page(
    name: &str,
    title: &str,
    entity: &Entity,
    columns: &[&str],
    filter: &[&str],
) -> String {
    let data = plural(&data_name(&entity.name.name));
    let grid = DEFAULT_PAGE_COLUMNS;
    let mut lines = Vec::new();
    // Filters, two to a row: a label and an input that the code behind reads.
    let mut row = 1;
    let mut placed = 0;
    for field in filter.iter().filter_map(|f| entity.field(f)) {
        let col = 1 + (placed % 2) * 6;
        row = 1 + placed / 2;
        let mut caption = new_props(WidgetKind::Label, col, row);
        caption.caption = Some(field.label.clone());
        caption.width = 2;
        lines.push(widget_line(&caption));
        let mut input = new_props(WidgetKind::Textbox, col + 2, row);
        input.width = 4;
        input.name = Some(format!("{}_filter", field.name.name));
        lines.push(widget_line(&input));
        placed += 1;
    }
    let table_row = if placed > 0 { row + 1 } else { 1 };
    let mut table = new_props(WidgetKind::Table, 1, table_row);
    table.width = grid;
    table.height = TABLE_ROWS;
    table.bind = Some(data.clone());
    table.name = Some(format!("{data}_table"));
    table.columns = columns.iter().map(|c| c.to_string()).collect();
    lines.push(widget_line(&table));
    let buttons = table_row + TABLE_ROWS;
    lines.push(button("New", grid - 3, buttons));
    lines.push(button("Open", grid - 1, buttons));
    format!(
        "page {name}: window\n    title {}\n    data {data}: list<{}>\n    on return: {RETURN_HANDLER}\n\n{}",
        quote(title),
        entity.name.name,
        body(&lines)
    )
}

/// The handler a browse window gets for its `on return` event.
const RETURN_HANDLER: &str = "window_returned";

/// The code behind a template starts with, if any: a browse window reloads
/// its list when a window it opened (a form) has closed.
pub fn code_from_template(project: &Project, name: &str, template: &PageTemplate) -> Result<Option<String>, String> {
    match template {
        PageTemplate::Browse { entity } => {
            let entity = find_entity(project, entity)?;
            let data = plural(&data_name(&entity.name.name));
            Ok(Some(format!("code {name}\n    handler {RETURN_HANDLER}\n        reload {data}")))
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
