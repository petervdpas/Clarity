//! The tool list of an app's MCP service (docs/ledger/app-mcp.md), written
//! once for every stack: the commands with an `agent` line, each with its
//! name as typed (`customer-add`), what it does in the default language, its
//! inputs as a JSON schema, what it changes, whether who may run it is
//! limited, and its words and arguments for the command line a call becomes; and the instructions agents get when they connect, the app's
//! guidance with its data summed up. Each runtime serves this list as it is.

use serde_json::{Map, Value, json};

use clarity_spec::code::check::TypeInfo;
use clarity_spec::model::{FieldType, Project};

use crate::commands::{CommandSpec, ParamSpec, Said};

/// Whether the program offers commands to agents: one has an `agent` line.
pub fn offers_agents(project: &Project) -> bool {
    project.files.iter().any(|f| f.commands.iter().any(|c| c.agent))
}

/// `project` with Clarity's own settings `agentReadsWhileClosed` and
/// `agentChangesWhileClosed` starting as the project file says (`agents while
/// closed read change`, `settings`): every stack writes a setting's default
/// as it writes any other, and the person's choice is kept over it.
pub fn with_agent_defaults(project: &Project, settings: &clarity_project::Settings) -> Project {
    let mut out = project.clone();
    for field in out.files.iter_mut().flat_map(|f| f.entities.iter_mut()).filter(|e| e.settings).flat_map(|e| e.fields.iter_mut()) {
        let allowed = match field.name.name.as_str() {
            "agentReadsWhileClosed" => settings.agents_read_closed,
            "agentChangesWhileClosed" => settings.agents_change_closed,
            _ => continue,
        };
        if field.ty == clarity_spec::model::FieldType::Bool {
            field.modifiers.default = Some(clarity_spec::model::Literal::Bool(allowed));
        }
    }
    out
}

/// The tool list as the file a program carries (`tools.json`).
pub fn tool_list_file(project: &Project, settings: &clarity_project::Settings) -> String {
    serde_json::to_string_pretty(&tool_list(project, settings)).expect("a tool list is JSON") + "\n"
}

/// The tool list of the program `settings` makes from `project`.
pub fn tool_list(project: &Project, settings: &clarity_project::Settings) -> Value {
    let language = clarity_spec::translations::default_language(project);
    let texts = project.translations.get(&language);
    let shown = |said: &Option<Said>| -> Option<String> {
        match said.as_ref()? {
            Said::Key(key) => Some(texts.and_then(|t| t.get(key)).cloned().unwrap_or_else(|| key.clone())),
            Said::Plain(text) => Some(text.clone()),
        }
    };
    let tools: Vec<Value> = crate::commands::commands(project)
        .iter()
        .filter(|c| c.decl.agent)
        .map(|c| {
            json!({
                "name": c.line.join("-"),
                "description": shown(&c.help).unwrap_or_default(),
                "inputSchema": input_schema(c, &shown),
                "changes": clarity_spec::agent::changes(project, c.decl),
                "allow": c.decl.allow.is_some(),
                // How the program reads a call: as this command line (Clarity.AppMcp `cli`).
                "line": c.line,
                "arguments": c.arguments.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "name": crate::commands::program_name(settings),
        "instructions": instructions(project),
        // What agents may do while the app is closed, until the person sets it (`agents while closed`).
        "whileClosed": { "read": settings.agents_read_closed, "change": settings.agents_change_closed },
        // How many agents the running app serves at once (`agents at most`).
        "atMost": settings.agents_at_most.unwrap_or(clarity_project::AGENTS_AT_MOST),
        "tools": tools,
    })
}

/// The JSON schema of a command's arguments (required) and options.
fn input_schema(command: &CommandSpec, shown: &dyn Fn(&Option<Said>) -> Option<String>) -> Value {
    let mut properties = Map::new();
    for param in command.arguments.iter().chain(&command.options) {
        let mut property = match param.kind {
            "integer" => json!({ "type": "integer" }),
            "decimal" | "money" => json!({ "type": "number" }),
            "bool" => json!({ "type": "boolean" }),
            "date" => json!({ "type": "string", "format": "date" }),
            "datetime" => json!({ "type": "string", "format": "date-time" }),
            "duration" => json!({ "type": "string", "description": "3:45, 1:02:03 or seconds" }),
            "enum" => json!({ "type": "string", "enum": param.values }),
            _ => json!({ "type": "string" }),
        };
        let object = property.as_object_mut().expect("a schema is an object");
        let number = |text: &Option<String>| text.as_deref().and_then(|t| serde_json::from_str::<Value>(t).ok()).filter(Value::is_number);
        if let Some(min) = number(&param.min) {
            object.insert("minimum".into(), min);
        }
        if let Some(max) = number(&param.max) {
            object.insert("maximum".into(), max);
        }
        if let Some(default) = default_of(param) {
            object.insert("default".into(), default);
        }
        if let Some(help) = shown(&param.help) {
            object.insert("description".into(), Value::String(help));
        }
        properties.insert(param.name.clone(), property);
    }
    let required: Vec<&str> = command.arguments.iter().map(|a| a.name.as_str()).chain(command.options.iter().filter(|o| o.required).map(|o| o.name.as_str())).collect();
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

/// An option's default as the JSON value of its kind.
fn default_of(param: &ParamSpec) -> Option<Value> {
    let text = param.default.as_deref()?;
    Some(match param.ty {
        TypeInfo::Integer | TypeInfo::Decimal | TypeInfo::Money => serde_json::from_str::<Value>(text).ok().filter(Value::is_number)?,
        TypeInfo::Bool => Value::Bool(text == "true"),
        _ => Value::String(text.to_string()),
    })
}

/// What agents get when they connect: the guidance as the developer wrote
/// it, then the app's data, an entity a line with its fields and their types.
fn instructions(project: &Project) -> String {
    let mut out = String::new();
    if let Some(guidance) = project.guidance.as_deref().map(str::trim_end).filter(|g| !g.is_empty()) {
        out.push_str(guidance);
        out.push_str("\n\n");
    }
    out.push_str("## The app's data\n\n");
    for entity in project.entities().filter(|e| !e.settings) {
        let fields: Vec<String> = entity
            .fields
            .iter()
            .map(|f| {
                let ty = match &f.ty {
                    FieldType::Enum { name } => {
                        let values: Vec<&str> = project.enum_decl(&name.name).map(|e| e.variants.iter().map(|v| v.name.name.as_str()).collect()).unwrap_or_default();
                        format!("{}: {}", name.name, values.join(", "))
                    }
                    _ => f.type_text.split('(').next().unwrap_or(&f.type_text).trim().to_string(),
                };
                format!("{} ({ty})", f.name.name)
            })
            .collect();
        out.push_str(&format!("- {}: {}\n", entity.name.name, fields.join(", ")));
    }
    out
}
