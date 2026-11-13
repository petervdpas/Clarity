//! The tool list of an app, as clarity-gen writes it (`agent_tools.rs`):
//! the program's name, the instructions agents get, and each tool with its
//! description, inputs, what it changes and whether `allow when` limits it.

use serde_json::{Value, json};

use crate::grants::Kind;

#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    /// `{data, files, settings, net}`, each true when the command may change it.
    pub changes: Value,
    pub allow: bool,
    /// The command's words, as typed: `["customer", "add"]`.
    pub line: Vec<String>,
    /// The names of its arguments, in their order; the other inputs are options.
    pub arguments: Vec<String>,
}

impl Tool {
    /// Whether a call changes anything, so its grant is a change's.
    pub fn kind(&self) -> Kind {
        let changes = self.changes.as_object().is_some_and(|c| c.values().any(|v| v == &Value::Bool(true)));
        if changes { Kind::Change } else { Kind::Read }
    }

    /// The tool as `tools/list` gives it: whether it only reads, and whether
    /// it reaches outside the app.
    pub fn listed(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
            "annotations": {
                "readOnlyHint": self.kind() == Kind::Read,
                "openWorldHint": self.changes["net"] == Value::Bool(true),
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolList {
    pub name: String,
    pub instructions: String,
    pub tools: Vec<Tool>,
    /// What the project allows agents while the app is closed, until the
    /// person sets it (`agents while closed read change`): reads, changes.
    pub while_closed: (bool, bool),
    /// How many agents the running app serves at once (`agents at most`).
    pub at_most: usize,
}

impl ToolList {
    /// Whether agents may read and change while the app is closed: the
    /// person's settings `reads` and `changes` when set, else what the project allows.
    pub fn closed_allows(&self, reads: Option<bool>, changes: Option<bool>) -> (bool, bool) {
        (reads.unwrap_or(self.while_closed.0), changes.unwrap_or(self.while_closed.1))
    }
}

/// The texts of the list `key` of `value`.
fn words(value: &Value, key: &str) -> Vec<String> {
    value.get(key).and_then(Value::as_array).map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
}

impl ToolList {
    pub fn from_json(text: &str) -> Result<ToolList, String> {
        let value: Value = serde_json::from_str(text).map_err(|e| format!("the tool list is no JSON: {e}"))?;
        let word = |v: &Value, key: &str| v.get(key).and_then(Value::as_str).map(str::to_string);
        let mut tools = Vec::new();
        for tool in value.get("tools").and_then(Value::as_array).ok_or("the tool list has no tools")? {
            tools.push(Tool {
                name: word(tool, "name").ok_or("a tool has no name")?,
                description: word(tool, "description").unwrap_or_default(),
                input_schema: tool.get("inputSchema").cloned().unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
                changes: tool.get("changes").cloned().unwrap_or_else(|| json!({})),
                allow: tool.get("allow").and_then(Value::as_bool).unwrap_or(false),
                line: words(tool, "line"),
                arguments: words(tool, "arguments"),
            });
        }
        let closed = &value["whileClosed"];
        let allows = |key: &str| closed.get(key).and_then(Value::as_bool).unwrap_or(false);
        // 3 when the list does not say, as `agents at most` without the line.
        let at_most = value.get("atMost").and_then(Value::as_u64).filter(|n| *n > 0).map_or(3, |n| n as usize);
        Ok(ToolList { name: word(&value, "name").unwrap_or_default(), instructions: word(&value, "instructions").unwrap_or_default(), tools, while_closed: (allows("read"), allows("change")), at_most })
    }

    pub fn tool(&self, name: &str) -> Option<&Tool> {
        self.tools.iter().find(|t| t.name == name)
    }
}
