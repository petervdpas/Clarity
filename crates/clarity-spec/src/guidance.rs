//! An app's guidance for AI agents (docs/ledger/app-mcp.md): `agent/guidance.md`
//! in the project's folder, Markdown in fixed sections, one file in the
//! project's language. The app's MCP service gives it to agents when they
//! connect. The wizard and the agents' tools read and write it a section at
//! a time, by its heading, so what one writes the other reads back; the
//! checker keeps the names written in backticks true to the app.

use crate::diagnostic::Diagnostic;
use crate::model::Project;
use crate::span::Span;

/// Where the guidance is, in the project's folder.
pub const FILE: &str = "agent/guidance.md";
/// The folder that holds it.
pub const DIR: &str = "agent";

/// A section of the guidance: its key for the tools, its heading in the
/// file, and what the wizard and the tools ask for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    pub key: &'static str,
    pub heading: &'static str,
    pub asks: &'static str,
}

/// The sections, in their order in the file.
pub const SECTIONS: [Section; 4] = [
    Section { key: "purpose", heading: "What the app is for", asks: "What the app is for and who uses it, in a sentence or two." },
    Section { key: "words", heading: "The app's words", asks: "What the app's words mean, and which data each one is: its entities, fields and enums." },
    Section { key: "flow", heading: "How work flows", asks: "The order things happen in, and which command does each step." },
    Section { key: "rules", heading: "Rules the app does not check", asks: "What an agent should do or leave alone that no check of the app enforces." },
];

/// A new guidance for the app `app`: its title and the sections, empty.
pub fn template(app: &str) -> String {
    let mut out = format!("# {app} for agents\n");
    for section in SECTIONS {
        out.push_str(&format!("\n## {}\n", section.heading));
    }
    out
}

/// What each section says, by its key, in the order of `SECTIONS`; empty
/// for a section the file does not have.
pub fn sections(text: &str) -> Vec<(&'static str, String)> {
    let lines: Vec<&str> = text.lines().collect();
    SECTIONS
        .iter()
        .map(|s| (s.key, body_of(&lines, s.heading).map(|(from, to)| lines[from..to].join("\n").trim().to_string()).unwrap_or_default()))
        .collect()
}

/// The lines under `heading`, up to the next heading, as a range of `lines`.
fn body_of(lines: &[&str], heading: &str) -> Option<(usize, usize)> {
    let at = lines.iter().position(|l| l.strip_prefix("## ").map(str::trim) == Some(heading))?;
    let end = lines[at + 1..].iter().position(|l| is_heading(l)).map_or(lines.len(), |i| at + 1 + i);
    Some((at + 1, end))
}

fn is_heading(line: &str) -> bool {
    line.starts_with("# ") || line.starts_with("## ")
}

/// A section as the IDE's Agents part shows it: its key, heading, what to write in it, and what it says.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct SectionShown {
    pub key: String,
    pub heading: String,
    pub asks: String,
    pub body: String,
}

/// Every section of the guidance `text` (none: no file yet), in order.
pub fn shown(text: Option<&str>) -> Vec<SectionShown> {
    let bodies = sections(text.unwrap_or(""));
    SECTIONS
        .iter()
        .zip(bodies)
        .map(|(s, (_, body))| SectionShown { key: s.key.into(), heading: s.heading.into(), asks: s.asks.into(), body })
        .collect()
}

/// The guidance after saving the section `key`: `current`'s, or a new file
/// from the template for the app `app` when there is none.
pub fn saved(current: Option<&str>, app: &str, key: &str, body: &str) -> Result<String, String> {
    match current {
        Some(text) => with_section(text, key, body),
        None => with_section(&template(app), key, body),
    }
}

/// `text` with the section `key` saying `body`: in place of what it said,
/// or as a new section in its place among the others. The title, the other
/// sections and headings of the developer's own stay as they are.
pub fn with_section(text: &str, key: &str, body: &str) -> Result<String, String> {
    let Some(index) = SECTIONS.iter().position(|s| s.key == key) else {
        let keys: Vec<&str> = SECTIONS.iter().map(|s| s.key).collect();
        return Err(format!("there is no section {key}; the sections are {}", keys.join(", ")));
    };
    let section = SECTIONS[index];
    let lines: Vec<&str> = text.lines().collect();
    let body = body.trim();
    let written: Vec<String> = if body.is_empty() { vec![String::new()] } else { vec![String::new(), body.to_string(), String::new()] };
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    match body_of(&lines, section.heading) {
        Some((from, to)) => {
            out.splice(from..to, written);
        }
        None => {
            // Before the first section of ours that comes after it, else at the end.
            let next = SECTIONS[index + 1..].iter().find_map(|s| lines.iter().position(|l| l.strip_prefix("## ").map(str::trim) == Some(s.heading)));
            let mut block = vec![format!("## {}", section.heading)];
            block.extend(written);
            match next {
                Some(at) => {
                    out.splice(at..at, block);
                }
                None => {
                    if out.last().is_some_and(|l| !l.trim().is_empty()) {
                        out.push(String::new());
                    }
                    out.extend(block);
                }
            }
        }
    }
    let mut joined = out.join("\n");
    while joined.ends_with("\n\n") || joined.ends_with('\n') {
        joined.pop();
    }
    joined.push('\n');
    Ok(joined)
}

/// The names written in backticks, with where each starts in `text`.
pub fn names(text: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut rest = text;
    let mut offset = 0;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        let name = after[..close].trim();
        if !name.is_empty() && !name.contains('\n') {
            out.push((name.to_string(), offset + open));
        }
        let used = open + 1 + close + 1;
        rest = &rest[used..];
        offset += used;
    }
    out
}

/// The names of the app a guidance may write in backticks: commands as
/// typed (`customer add`), entities and records, their fields
/// (`Customer.name`), enums and their values.
fn known(project: &Project) -> Vec<String> {
    let mut out = Vec::new();
    for entity in project.entities().chain(project.records()) {
        out.push(entity.name.name.clone());
        out.extend(entity.fields.iter().map(|f| format!("{}.{}", entity.name.name, f.name.name)));
    }
    for decl in project.enums() {
        out.push(decl.name.name.clone());
        out.extend(decl.variants.iter().map(|v| v.name.name.clone()));
        out.extend(decl.variants.iter().map(|v| format!("{}.{}", decl.name.name, v.name.name)));
    }
    for command in project.files.iter().flat_map(|f| &f.commands) {
        out.push(command.line());
        out.push(command.line().replace(' ', "-"));
    }
    out
}

/// What the checker says of the guidance `text` (none when the project has
/// no `agent/guidance.md`), at `path`: a name in backticks the app does not
/// have, and an app offering commands to agents without guidance.
pub fn check(project: &Project, text: Option<&str>, path: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    match text {
        Some(text) => {
            let known = known(project);
            for (name, _) in names(text) {
                if !known.contains(&name) {
                    out.push(
                        Diagnostic::warning(
                            format!("the guidance names `{name}`, which is no command, entity, field or enum of the app; write other words without backticks"),
                            Span::default(),
                        )
                        .in_file(path),
                    );
                }
            }
        }
        None => {
            for command in project.files.iter().flat_map(|f| f.commands.iter().map(move |c| (f, c))).filter(|(_, c)| c.agent) {
                let (file, command) = command;
                out.push(
                    Diagnostic::warning(
                        format!("command {} is for agents, and the app has no {FILE} to tell them how it is used; the guidance wizard or the guidance tool starts one", command.line()),
                        command.name.span,
                    )
                    .in_file(&file.path),
                );
            }
        }
    }
    out
}
