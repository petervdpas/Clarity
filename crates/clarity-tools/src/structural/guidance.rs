//! The app's guidance for AI agents (docs/ledger/app-mcp.md), through tools:
//! `guidance` reads its sections with what the app offers for each, and
//! `set_guidance` writes one section by its heading. Agents fill
//! `agent/guidance.md` only through these, as the IDE's wizard does, on the
//! same format (`clarity_spec::guidance`), so what one writes the other
//! reads back.

use clarity_agent::ToolDef;
use clarity_spec::guidance::{self, SECTIONS};
use serde_json::{Value, json};

use super::{schema, tool};
use crate::{ProjectTools, Written};

pub fn definitions() -> Vec<ToolDef> {
    let keys: Vec<&str> = SECTIONS.iter().map(|s| s.key).collect();
    vec![
        tool(
            "guidance",
            "Reads the app's guidance for AI agents (agent/guidance.md), which its MCP service gives to agents when they connect: each section with what it asks and what it says now, and what the app offers to name in it (its commands for agents, entities and fields, enums). Names written in `backticks` must be the app's.",
            schema(json!({}), &[]),
        ),
        tool(
            "set_guidance",
            "Writes one section of the app's guidance for AI agents, in Markdown, in place of what it said; the file starts from its template when the app has none. Write what the app's declarations cannot say: what it is for, what its words mean, how work flows, rules no check enforces.",
            schema(json!({ "section": { "type": "string", "enum": keys }, "text": { "type": "string", "description": "The section's Markdown, without its heading" } }), &["section", "text"]),
        ),
    ]
}

/// The tool `name`, when it is one of this file's: its answer.
pub fn call(tools: &mut ProjectTools, name: &str, input: &Value) -> Option<Result<String, String>> {
    let result = match name {
        "guidance" => read(tools),
        "set_guidance" => write(tools, input),
        _ => return None,
    };
    Some(result)
}

/// The project's name, its folder, and its model.
fn project(tools: &ProjectTools) -> Result<(String, std::path::PathBuf, clarity_spec::Project), String> {
    let (file, dir) = tools.open()?.clone();
    let name = clarity_project::load(&file).settings.map(|s| clarity_project::title(&s)).unwrap_or_else(|| "The app".into());
    let model = super::places::model_project(&dir)?;
    Ok((name, dir, model))
}

fn read(tools: &ProjectTools) -> Result<String, String> {
    let (name, dir, model) = project(tools)?;
    let text = std::fs::read_to_string(dir.join(guidance::FILE)).ok();
    let said = guidance::sections(text.as_deref().unwrap_or(""));
    let mut out = match &text {
        Some(_) => format!("{} for {name}:\n", guidance::FILE),
        None => format!("{name} has no {} yet; set_guidance starts it from its template.\n", guidance::FILE),
    };
    for (section, (_, body)) in SECTIONS.iter().zip(&said) {
        let now = if body.is_empty() { "(empty)".to_string() } else { body.clone() };
        out.push_str(&format!("\n## {} (section {})\nAsks: {}\nSays: {now}\n", section.heading, section.key, section.asks));
    }
    let commands: Vec<String> = model.files.iter().flat_map(|f| &f.commands).filter(|c| c.agent).map(|c| format!("`{}`", c.line())).collect();
    let entities: Vec<String> = model
        .entities()
        .filter(|e| !e.settings)
        .chain(model.records())
        .map(|e| format!("`{}` ({})", e.name.name, e.fields.iter().map(|f| format!("`{}.{}`", e.name.name, f.name.name)).collect::<Vec<_>>().join(", ")))
        .collect();
    let enums: Vec<String> = model.enums().map(|e| format!("`{}` ({})", e.name.name, e.variants.iter().map(|v| format!("`{}`", v.name.name)).collect::<Vec<_>>().join(", "))).collect();
    let listed = |items: &[String]| if items.is_empty() { "none".to_string() } else { items.join("; ") };
    out.push_str(&format!("\nThe app offers to name:\n- commands for agents: {}\n- entities: {}\n- enums: {}\n", listed(&commands), listed(&entities), listed(&enums)));
    Ok(out)
}

fn write(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let section = input.get("section").and_then(Value::as_str).ok_or("give section")?.trim().to_string();
    let text = input.get("text").and_then(Value::as_str).ok_or("give text")?;
    let (name, dir, _) = project(tools)?;
    let file = dir.join(guidance::FILE);
    let before = std::fs::read_to_string(&file).ok();
    let after = guidance::saved(before.as_deref(), &name, &section, text)?;
    if before.as_deref() == Some(after.as_str()) {
        return Ok(format!("The section {section} of {} says that already.", guidance::FILE));
    }
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
    }
    crate::files::write_atomically(&file, &after)?;
    tools.written.push(Written { file, before, after: Some(after) });
    // What the checker says of the names in backticks, as of every change.
    Ok(format!("Wrote the section {section} of {}. {}", guidance::FILE, tools.problems()?))
}

#[cfg(test)]
mod tests {
    use clarity_agent::Tools;
    use serde_json::json;

    use crate::ProjectTools;

    fn project(name: &str) -> (std::path::PathBuf, std::path::PathBuf, ProjectTools) {
        let root = std::env::temp_dir().join(format!("clarity-guidance-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        let tools = ProjectTools::new(Some((file, dir.clone())), root.clone());
        (root, dir, tools)
    }

    #[test]
    fn agents_fill_the_guidance_a_section_at_a_time() {
        let (root, dir, mut tools) = project("sections");
        let wrote = tools.call("write_file", &json!({"path": "todo.clarity", "text": "module todo\n\nentity Task\n    title: text(80)\n    done: bool\n"}));
        assert!(!wrote.is_error, "{}", wrote.text);
        let read = tools.call("guidance", &json!({}));
        assert!(!read.is_error && read.text.contains("no agent/guidance.md yet") && read.text.contains("Asks: What the app is for"), "{}", read.text);
        assert!(read.text.contains("`Task.title`"), "what the app offers to name: {}", read.text);

        let wrote = tools.call("set_guidance", &json!({"section": "purpose", "text": "Keeps a list of tasks."}));
        assert!(!wrote.is_error, "{}", wrote.text);
        let text = std::fs::read_to_string(dir.join("agent/guidance.md")).unwrap();
        assert!(text.starts_with("# Todo for agents\n"), "started from the template: {text}");
        assert_eq!(clarity_spec::guidance::sections(&text)[0].1, "Keeps a list of tasks.");
        let read = tools.call("guidance", &json!({}));
        assert!(read.text.contains("Says: Keeps a list of tasks."), "{}", read.text);

        let unknown = tools.call("set_guidance", &json!({"section": "story", "text": "x"}));
        assert!(unknown.is_error && unknown.text.contains("purpose, words, flow, rules"), "{}", unknown.text);
        let stale = tools.call("set_guidance", &json!({"section": "words", "text": "A `Chore` is a task."}));
        assert!(stale.text.contains("`Chore`"), "the checker's word on a name the app does not have: {}", stale.text);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn write_file_leaves_the_guidance_to_its_tools() {
        let (root, _, mut tools) = project("refused");
        let refused = tools.call("write_file", &json!({"path": "agent/guidance.md", "text": "# x\n"}));
        assert!(refused.is_error && refused.text.contains("set_guidance"), "{}", refused.text);
        let _ = std::fs::remove_dir_all(root);
    }
}
