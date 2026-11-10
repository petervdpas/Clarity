//! Structural tools (docs/AGENT-TOOLS-PLAN.md): agents change the project's
//! data model, windows, views, parts, app and frame through these, never by
//! writing a `.cxaml` file. Each tool names things by name (`Customer`,
//! `customerTable`), lists what is allowed in its schema, turns the names
//! into the places a designer edit wants (`places`), and makes the edit the
//! designer makes (`crate::edits::apply`): checked, written, one Undo.
//!
//! - `model`: entities, enums, their fields and values, a module's uses.
//! - `windows`: windows and views, their parts, grids, sizes, tabs and data.
//! - `frame`: the app, its main window and splash, menus, toolbars,
//!   ribbons, sidebars, events and handlers.

pub mod frame;
pub mod model;
pub mod places;
pub mod windows;

use clarity_agent::ToolDef;
use serde_json::{Value, json};

use crate::ProjectTools;

/// Every structural tool.
pub fn definitions() -> Vec<ToolDef> {
    [model::definitions(), windows::definitions(), frame::definitions()].concat()
}

/// The tool `name`, when it is a structural one: its answer.
pub fn call(tools: &mut ProjectTools, name: &str, input: &Value) -> Option<Result<String, String>> {
    model::call(tools, name, input).or_else(|| windows::call(tools, name, input)).or_else(|| frame::call(tools, name, input))
}

/// A tool's definition.
pub(crate) fn tool(name: &str, description: &str, input_schema: Value) -> ToolDef {
    ToolDef { name: name.into(), description: description.into(), input_schema }
}

/// An object schema with these properties, these required.
pub(crate) fn schema(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required })
}
