//! The agent panel: a conversation with a model that knows Clarity and works
//! on the open project through tools. It reads the project, writes modules
//! and the project file, checks them, makes a new project, and shows a
//! window. Its changes are written at once and checked; one Undo takes a
//! whole turn back. The model is reached through a connection of Clarity's
//! vault, so its key never sits in a file.

use crate::fail_safe::Held;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::time::Duration;

use clarity_agent::{Agent, Event, Message, ToolDef, ToolResult, Tools};
use clarity_project::Stack;
use clarity_spec::Severity;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use super::{FileChange, HistoryState, WatchState, is_spec_file, open_project_of, record, write_atomically};

/// The language reference, which the model reads by section.
const LANGUAGE: &str = include_str!("../../../docs/LANGUAGE.md");
/// Clarity's design document: its aims, its stacks and its roadmap. Too long
/// for every message, so the model reads the sections it needs.
const DESIGN: &str = include_str!("../../../docs/DESIGN.md");

/// The headings (`##` and `###`) of a Markdown document with their levels
/// and lines, leaving out what is in code blocks.
fn headings(doc: &str) -> Vec<(usize, usize, &str)> {
    let mut code = false;
    let mut found = Vec::new();
    for (i, line) in doc.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            code = !code;
        } else if !code {
            let level = line.chars().take_while(|c| *c == '#').count();
            if (2..=3).contains(&level) && line[level..].starts_with(' ') {
                found.push((i, level, line));
            }
        }
    }
    found
}

/// A section of a Markdown document (`name` says which in messages):
/// without `wanted`, the contents; with it, the section whose number or
/// title it is (or whose title holds it), up to the next section of its level.
fn section(doc: &str, name: &str, wanted: &str) -> Result<String, String> {
    let all = headings(doc);
    let wanted = wanted.trim().to_lowercase();
    if wanted.is_empty() {
        return Ok(format!("The sections of the {name}; ask for one by its number or title.\n\n{}", contents(doc)));
    }
    let title = |line: &str| line.trim_start_matches('#').trim().to_lowercase();
    let number = |line: &str| title(line).split_whitespace().next().unwrap_or_default().trim_end_matches('.').to_string();
    let found = all
        .iter()
        .position(|(_, _, line)| number(line) == wanted)
        .or_else(|| all.iter().position(|(_, _, line)| title(line).contains(&wanted)))
        .ok_or_else(|| format!("the {name} has no section '{wanted}'; ask without a section for the contents"))?;
    let (start, level, _) = all[found];
    let end = all[found + 1..].iter().find(|(_, l, _)| *l <= level).map_or(usize::MAX, |(i, _, _)| *i);
    let lines: Vec<&str> = doc.lines().skip(start).take(end.saturating_sub(start)).collect();
    Ok(lines.join("\n").trim_end().to_string())
}

/// The headings of a document, one per line.
fn contents(doc: &str) -> String {
    headings(doc).iter().map(|(_, _, line)| *line).collect::<Vec<_>>().join("\n")
}

/// The manual tool: a topic's page, or the list of topics.
fn manual_topic(id: &str) -> Result<String, String> {
    if id.trim().is_empty() {
        return Ok(crate::manual::index());
    }
    crate::manual::topic(id).map(|t| t.text.to_string()).ok_or_else(|| format!("the manual has no topic '{}'; these are its topics:\n{}", id.trim(), crate::manual::index()))
}

/// How many steps (model answers) one turn may take.
const MAX_STEPS: usize = 40;

/// What the model is told before the conversation: how to work, the
/// manual's topics and the reference's sections. The pages themselves are
/// read on demand, so every message stays small.
fn instructions() -> String {
    format!(
        "You are the agent inside Clarity, a development environment where applications are written in the Clarity \
language and designed visually; Clarity transpiles them into real applications (Rust, Go, C#, C and C++ stacks). You \
work on the project that is open in Clarity, through your tools, while the person watches the designer follow your \
changes.

How you work:
- Start a task by looking at the project (project_overview), and read a file before you change it.
- Before you write Clarity you have not written in this conversation, read the manual topics the task needs with the \
manual tool, and the exact syntax with the reference tool. Do not invent syntax. When unsure of a part's properties, \
use the parts tool.
- A project is one project file (read and write it as the path `project`) and modules: `.clarity` files in the \
project's folder, written as paths like `todo.clarity`.
- Write whole files with write_file. Every write is checked at once: fix every error it reports before you go on, \
and finish only when check reports no errors.
- Give every part a name, and name parts and handlers in camelCase: letters and digits only, starting with a small \
letter, such as `newGame`, `statusText` and `newGameClicked`. Name a part after what it is for, never `button1` or \
`label2`: an input after its field (`name`, `vatNumber`), its label and help after the input (`nameLabel`, \
`nameHelp`), a button after what it does in its window (`saveOrder`, `openAlbum`, `newCustomer`), a container after \
what it holds (`identityPanel`, `albumTabs`). A part's name is not a word code uses, such as `save` or `close`. \
Fields, data and functions stay snake_case.
- Every text people read is a key: `title @customerCardTitle`, `button @saveCaption name saveOrder ...`, \
`message @savedName(name = customer.name)`, `label @customerNameLabel` on every field. Give each key its text in the \
default language's file, `translations/en.clarity` (read it first; lines like `saveCaption \"Save\"`), and the same \
key in every other language file, empty (`saveCaption \"\"`) when you do not translate it. Name a key in camelCase \
after where it is: the part and what the text is to it (`identityPanelCaption`, `nameLabel`). Parts that say the same \
thing share one key, such as `saveCaption` for every Save button.
- Long texts, such as rules or help, are Markdown files of the resources, written with write_file as \
`resources/rules-en.md`, one per language. A markdown part shows one by a key (`markdown @gameRules`), and each \
language file points the key at its file: `gameRules resource \"rules-nl.md\"`. A file that is the same in every \
language is shown by its source (`markdown source \"license.md\"`). Pictures you cannot write; use those the \
overview lists (`image source \"logo.png\"`, or a key pointing each language at one).
- A custom part (`part Arena`) is a part an outside library draws; a library (`library Words`) gives code functions \
from a package. Both are written in the project's own language: C or C++ (`native`, `cmake`), Rust (`crate`), Go \
(`go`), C# (`nuget`, `dll`); never another stack's. Read the manual topics design.custom-parts and code.libraries \
first. The overview lists their folders and files: read and write those files by their paths (like \
parts/Arena/src/lib.rs). The file Clarity writes from the declaration (src/clarity.rs, clarity.go, the clarity \
folder's header, the .g.cs half) is read only: change the declaration and the file follows. Build one on its own \
with the build tool's part to see its compiler's errors. A sound (`sound name rattle source \"roll.wav\"`, see \
design.sound) plays sound files of the resources, tones and melodies; it has no place in the window. Sound files you \
cannot write; use those the overview lists.
- Lay windows out to use the full width of their grid: inputs and tables reach the last column, labels sit left of \
their inputs, buttons sit together at the bottom right.
- Data is designed in a diagram first. When asked for new or changed data (\"an order system with customers and \
invoices\"), read the Data model with diagram_view, then sketch the change as a Mermaid ERD (or a class diagram for \
enums) with write_diagram, open it with show_diagram, say in a few lines what it proposes, and stop: the person \
adjusts it in Clarity. Apply it with apply_diagram only when the person asks, reading diagram_proposals first since \
they may have changed it. Diagrams propose and the data dictionary decides; never apply a removal unless the person \
asks for that one. In an ERD write Clarity's types (`text(80)`, `money`, `email`, an enum's name), `UK` for unique, \
and other modifiers or a decimal's full type in the comment (`\"required\"`, `\"decimal(10, 2), computed\"`). \
In a class diagram put each class in the namespace of its module (`namespace music {{`), the module's functions on a \
`<<module>>` class of the module's name outside the namespaces; the designer draws each module as a frame around its \
classes. Draw what a module's functions use (the engines they call, the entities they take, find or make) as a uses \
arrow from the module's class (`music ..> Images`); it is drawn only and Apply ignores it.
- After adding or changing entities, write a migration step with migrate, so the database follows.
- When a window is done, show it with show_window so the person sees it.
- When the person mentions a failed build or run, or errors in the output, read them with build_output; never ask \
for the output to be pasted. Read the generated code it points at with read_generated. The generated code is \
not edited: fix the project, or say what in Clarity needs fixing.
- When asked what Clarity can do, which stacks support something, or what is planned, read the manual or the design \
document (design tool, its Stacks and Roadmap sections) rather than guess; never promise what is not built.
- Read only what the task needs: every page you read is sent again with each later message.
- You execute nothing yourself. To run or build the application use the build tool, which may ask the person \
first; use it only when the task needs it, and when they deny it, do not ask again in this turn. Without a build \
tool (Clarity's settings turned it off), ask the person to press Run or Build, then read build_output.
- Say briefly what you did when you finish. Write plain, short sentences.
- Write in Clarity's house style, in what you say and in what you write into the project (comments, texts, Markdown): \
no em-dash and no en-dash (use a period, a comma, a colon or parentheses; a plain hyphen for compounds; for a range \
write \"1 to 9\"). No filler such as \"it is worth noting\", \"moreover\" or \"seamless\", no praise of the question, \
no recap at the end of a short answer.

Clarity's manual (read a topic with the manual tool):
{manual}
The language reference's sections (read one with the reference tool):
{reference}",
        manual = crate::manual::index(),
        reference = contents(LANGUAGE),
    )
}

/// One change of one file during a turn: its text before (none for a new file) and after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub file: PathBuf,
    pub before: Option<String>,
    pub after: String,
}

/// Whether the agent may run or build the application, as Clarity's
/// settings (Agent) say: ask every time, allow without asking, or never.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Runs {
    #[default]
    Ask,
    Allow,
    Never,
}

/// What the agent does only after the person allows it: running or building
/// the application. The agent executes nothing else.
pub trait Execute {
    /// Asks the person, in the panel, whether the agent may do `what`; true when they allow it.
    fn ask(&mut self, what: &str) -> bool;
    /// Generates and builds the project (with `run`, starts it too), as the
    /// toolbar does, and waits for the end; gives the build's output.
    fn build(&mut self, file: &Path, run: bool) -> Result<String, String>;
    /// Builds the custom part or library `name` of the project in `dir` on
    /// its own, in its folder, and waits for the end; gives the output.
    fn build_part(&mut self, dir: &Path, name: &str) -> Result<String, String>;
}

/// The tools, on one project: the project file and its folder of modules.
pub struct ProjectTools {
    project: Option<(PathBuf, PathBuf)>,
    /// What was written during the turn, oldest first.
    pub written: Vec<Written>,
    /// A project made during the turn, to open when it ends.
    pub made: Option<PathBuf>,
    /// The window to show in Preview, when a tool asked for one.
    pub shown: Option<String>,
    /// The diagram to open, by its file name, when a tool asked for one.
    pub shown_diagram: Option<String>,
    /// The folder new projects go in by default.
    projects_folder: PathBuf,
    /// The Output panel as the turn started: the last build's lines and how it ended.
    pub output: String,
    /// How the build tool asks the person and builds; without it, nothing runs.
    pub execute: Option<Box<dyn Execute>>,
    /// Whether the build tool asks first, runs without asking, or is not there.
    pub runs: Runs,
}

impl ProjectTools {
    pub fn new(project: Option<(PathBuf, PathBuf)>, projects_folder: PathBuf) -> ProjectTools {
        ProjectTools { project, written: Vec::new(), made: None, shown: None, shown_diagram: None, projects_folder, output: crate::build::Output::default().text(), execute: None, runs: Runs::Ask }
    }

    fn open(&self) -> Result<&(PathBuf, PathBuf), String> {
        self.project.as_ref().ok_or_else(|| "no project is open; make one with new_project".to_string())
    }

    /// The file a tool's path names: `project`, or a `.clarity` file in the project's folder.
    fn file_of(&self, path: &str) -> Result<PathBuf, String> {
        let (file, dir) = self.open()?;
        let path = path.trim();
        if path == "project" {
            return Ok(file.clone());
        }
        // A Markdown file of the resources: `resources/rules-en.md`, a plain name.
        if let Some(name) = path.strip_prefix("resources/") {
            let plain = !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\']);
            if plain && clarity_spec::resources::is_markdown(name) {
                return Ok(dir.join(clarity_spec::resources::RESOURCES_DIR).join(name));
            }
            return Err(format!("'{path}' is not a Markdown file of the resources: write one like resources/rules-en.md; pictures are added in Clarity"));
        }
        // A written diagram: `diagrams/orders.mermaid`.
        if let Some(name) = path.strip_prefix("diagrams/") {
            return super::diagrams::diagram_file(dir, name);
        }
        // A file of a custom part's or a library's folder: `parts/Arena/src/lib.rs`.
        if let Some(found) = self.part_file(path)? {
            return Ok(found);
        }
        let relative = Path::new(path);
        let fine = !path.is_empty()
            && relative.is_relative()
            && relative.components().all(|c| matches!(c, std::path::Component::Normal(_)))
            && is_spec_file(relative);
        if !fine {
            return Err(format!("'{path}' is not a module of the project: use a path like todo.clarity inside the project's folder, or `project`"));
        }
        Ok(dir.join(relative))
    }

    /// The part folders of the project: its custom parts and libraries.
    fn part_folders(&self) -> Result<Vec<super::parts::PartFolder>, String> {
        let (_, dir) = self.open()?;
        let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read the project: {err}"))?;
        Ok(super::parts::part_folders(dir, &project))
    }

    /// A file inside a custom part's or a library's folder, when `path` names
    /// one: never outside it, and never in its build output.
    fn part_file(&self, path: &str) -> Result<Option<PathBuf>, String> {
        let (_, dir) = self.open()?;
        for folder in self.part_folders()? {
            let Some(inside) = path.strip_prefix(&format!("{}/", folder.folder)) else { continue };
            let pieces: Vec<&str> = inside.split('/').collect();
            if inside.is_empty() || pieces.iter().any(|p| p.is_empty() || *p == "." || *p == ".." || p.starts_with('.')) || inside.contains('\\') {
                return Err(format!("'{path}' is not a file of {} {}: write a path like {}/src/lib.rs", folder.kind, folder.name, folder.folder));
            }
            if pieces.len() > 1 && super::parts::BUILD_FOLDERS.contains(&pieces[0]) {
                return Err(format!("'{path}' is in the build's output of {} {}, which is not written by hand", folder.kind, folder.name));
            }
            return Ok(Some(dir.join(&folder.folder).join(inside)));
        }
        Ok(None)
    }

    /// The file Clarity writes in a part's or library's folder, when `path` is it.
    fn clarity_s_file(&self, path: &str) -> Result<bool, String> {
        Ok(self.part_folders()?.iter().any(|f| f.own.as_ref().is_some_and(|own| path == format!("{}/{own}", f.folder))))
    }

    /// The project's problems, as `file:line:col: error: message` lines.
    fn problems(&self) -> Result<String, String> {
        let (_, dir) = self.open()?;
        let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read the project: {err}"))?;
        let lines: Vec<String> = project
            .diagnostics
            .iter()
            .take(60)
            .map(|d| {
                let file = Path::new(&d.path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "project".into());
                let severity = if d.severity == Severity::Error { "error" } else { "warning" };
                format!("{file}:{}:{}: {severity}: {}", d.span.line, d.span.col, d.message)
            })
            .collect();
        let errors = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
        let warnings = project.diagnostics.len() - errors;
        Ok(match (errors, warnings) {
            (0, 0) => "No problems.".into(),
            (0, n) => format!("No errors; {n} warning(s):\n{}", lines.join("\n")),
            (e, _) => format!("{e} error(s):\n{}", lines.join("\n")),
        })
    }

    fn overview(&self) -> Result<String, String> {
        let (file, dir) = self.open()?;
        let mut out = format!("Project file ({}):\n{}\n\nModules in {}:\n", file.display(), std::fs::read_to_string(file).unwrap_or_default(), dir.display());
        let mut modules: Vec<PathBuf> = Vec::new();
        collect(dir, dir, &mut modules);
        modules.sort();
        for m in &modules {
            out.push_str(&format!("- {}\n", m.display()));
        }
        let resources = clarity_spec::resources::resource_files(dir).ok().flatten().unwrap_or_default();
        if !resources.is_empty() {
            out.push_str("\nResources (resources/, the files the application carries):\n");
            for r in &resources {
                out.push_str(&format!("- resources/{}\n", r.name));
            }
        }
        let folders = self.part_folders()?;
        if !folders.is_empty() {
            out.push_str("\nCustom parts and libraries (their folders, in the project's language; read and write their files by these paths):\n");
            for f in &folders {
                out.push_str(&format!("{} {}: {}, in {}\n", f.kind, f.name, f.language, f.folder));
                for entry in super::parts::tree(&dir.join(&f.folder), f.own.as_deref()).into_iter().filter(|e| !e.folder) {
                    let mark = if entry.own { " (Clarity's, read only)" } else { "" };
                    out.push_str(&format!("- {}/{}{mark}\n", f.folder, entry.path));
                }
            }
        }
        let diagrams = clarity_spec::diagrams::diagram_files(dir).unwrap_or_default();
        if !diagrams.is_empty() {
            out.push_str("\nDiagrams (diagrams/, design that proposes changes; read one with read_file):\n");
            for d in &diagrams {
                out.push_str(&format!("- diagrams/{}\n", d.name));
            }
        }
        out.push_str(&format!("\nProblems:\n{}", self.problems()?));
        Ok(out)
    }

    fn read(&self, path: &str) -> Result<String, String> {
        let file = self.file_of(path)?;
        std::fs::read_to_string(&file).map_err(|_| format!("{path} does not exist yet"))
    }

    fn write(&mut self, path: &str, text: &str) -> Result<String, String> {
        if self.clarity_s_file(path.trim())? {
            return Err(format!("{path} is Clarity's: Clarity writes it from the declaration, so change the declaration instead"));
        }
        let file = self.file_of(path)?;
        let before = std::fs::read_to_string(&file).ok();
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
        }
        let text = if text.ends_with('\n') { text.to_string() } else { format!("{text}\n") };
        write_atomically(&file, &text)?;
        self.written.push(Written { file, before, after: text });
        Ok(format!("Wrote {path}. {}", self.problems()?))
    }

    /// What a web API's description offers, and what the project picks
    /// (DESIGN 5.23), as the API... panel shows it.
    fn api(&self, source: &str) -> Result<String, String> {
        let (file, _) = self.open()?;
        let listing = clarity_project::api_listing(file, source)?;
        let Some(description) = &listing.file else {
            return Err(format!("data source '{source}' names no OpenAPI description; add openapi \"file.yaml\" under it, a file of the apis folder"));
        };
        let settings = clarity_project::load(file).settings.ok_or("the project file cannot be read")?;
        let paging = settings.datasources.iter().find(|s| s.name == source).and_then(|s| s.paging.as_ref());
        let mark = |picked: bool| if picked { "[x]" } else { "[ ]" };
        let mut out = format!("{description} ({} {})", listing.title, listing.version);
        match paging {
            Some(p) => out.push_str(&format!("; the source pages: {}\n", clarity_project::paging_line(p))),
            None => out.push_str("; the source does not page\n"),
        }
        out.push_str("Operations, [x] picked:\n");
        for o in &listing.operations {
            let summary = o.summary.as_ref().map(|s| format!(": {s}")).unwrap_or_default();
            out.push_str(&format!("{} {} {} {}{summary}\n", mark(o.picked), o.id, o.method.to_uppercase(), o.path));
        }
        out.push_str("Schemas, [x] picked on their own, (needed) by a picked operation:\n");
        for s in &listing.schemas {
            out.push_str(&format!("{} {}{}\n", mark(s.picked), s.name, if s.needed { " (needed)" } else { "" }));
        }
        Ok(out)
    }

    /// Picks what the application uses of a web API's description: the
    /// operations and the schemas on their own, in place of what was picked,
    /// written into the project file; Clarity writes the API's module again.
    fn pick_api(&mut self, input: &Value) -> Result<String, String> {
        let source = input["source"].as_str().unwrap_or_default().to_string();
        let names = |key: &str| -> Vec<String> { input[key].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default() };
        let (operations, schemas) = (names("operations"), names("schemas"));
        let (file, _) = self.open()?.clone();
        let listing = clarity_project::api_listing(&file, &source)?;
        let description = listing.file.clone().ok_or_else(|| format!("data source '{source}' names no OpenAPI description"))?;
        let all: Vec<&str> = listing.operations.iter().map(|o| o.id.as_str()).collect();
        if let Some(missing) = operations.iter().find(|o| !all.contains(&o.as_str())) {
            return Err(format!("{description} has no operation {missing}; it has {}", listed(&all)));
        }
        let all: Vec<&str> = listing.schemas.iter().map(|s| s.name.as_str()).collect();
        if let Some(missing) = schemas.iter().find(|s| !all.contains(&s.as_str())) {
            return Err(format!("{description} has no schema {missing}; it has {}", listed(&all)));
        }
        let old = std::fs::read_to_string(&file).map_err(|err| format!("cannot read the project file: {err}"))?;
        let mut settings = clarity_project::parse(&old)?;
        let found = settings.datasources.iter_mut().find(|s| s.name == source).ok_or_else(|| format!("there is no data source '{source}'"))?;
        found.picks = operations;
        found.pick_schemas = schemas;
        let text = clarity_project::write(Some(&old), &settings)?;
        self.write("project", &text)?;
        let warnings = clarity_project::refresh_apis(&file)?;
        let mut out = format!("Picked; Clarity wrote apis/{source}.clarity. ");
        for w in warnings {
            out.push_str(&format!("Warning: {w}. "));
        }
        out.push_str(&self.problems()?);
        Ok(out)
    }

    /// The Data model: an ERD Clarity writes from the data dictionary.
    fn diagram_view(&self) -> Result<String, String> {
        let (_, dir) = self.open()?;
        let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read the project: {err}"))?;
        Ok(clarity_spec::diagrams::views::data_model(&project).text)
    }

    /// A written diagram's text, by its name (`billing` or `billing.mermaid`).
    fn diagram_text(&self, name: &str) -> Result<(PathBuf, String), String> {
        let (_, dir) = self.open()?;
        let file = super::diagrams::diagram_file(dir, name)?;
        let text = std::fs::read_to_string(&file).map_err(|_| format!("diagrams/{} does not exist", file.file_name().unwrap_or_default().to_string_lossy()))?;
        Ok((file, text))
    }

    /// The diagram's problems and what it proposes, numbered for apply_diagram.
    fn proposals_text(&self, text: &str) -> Result<String, String> {
        let (_, dir) = self.open()?;
        let (_, problems) = clarity_spec::diagrams::parse(text);
        let mut out = String::new();
        if !problems.is_empty() {
            out.push_str("Problems of the diagram:\n");
            for p in &problems {
                let severity = if p.severity == Severity::Error { "error" } else { "warning" };
                out.push_str(&format!("{}:{}: {severity}: {}\n", p.span.line, p.span.col, p.message));
            }
        }
        let proposals = super::diagrams::proposals_of(dir, text)?;
        if proposals.is_empty() {
            out.push_str("The diagram matches the project: it proposes nothing.");
        } else {
            out.push_str("It proposes:\n");
            out.push_str(&super::diagrams::proposal_lines(&proposals));
        }
        Ok(out)
    }

    /// Writes a diagram into the diagrams folder, and says what it proposes.
    fn write_diagram(&mut self, name: &str, text: &str) -> Result<String, String> {
        let (_, dir) = self.open()?.clone();
        let file = super::diagrams::diagram_file(&dir, name)?;
        let before = std::fs::read_to_string(&file).ok();
        std::fs::create_dir_all(dir.join(clarity_spec::diagrams::DIAGRAMS_DIR)).map_err(|err| format!("cannot make the diagrams folder: {err}"))?;
        let text = if text.ends_with('\n') { text.to_string() } else { format!("{text}\n") };
        write_atomically(&file, &text)?;
        let shown = file.file_name().unwrap_or_default().to_string_lossy().to_string();
        self.written.push(Written { file, before, after: text.clone() });
        Ok(format!("Wrote diagrams/{shown}. {}", self.proposals_text(&text)?))
    }

    /// Applies the proposals of a written diagram at `numbers` (from 1), as Apply does.
    fn apply_diagram(&mut self, name: &str, numbers: &[usize]) -> Result<String, String> {
        let (_, dir) = self.open()?.clone();
        let (_, text) = self.diagram_text(name)?;
        if numbers.is_empty() {
            return Err("name the proposals to apply by their numbers, as diagram_proposals lists them".into());
        }
        let count = super::diagrams::proposals_of(&dir, &text)?.len();
        if let Some(wrong) = numbers.iter().find(|n| **n == 0 || **n > count) {
            return Err(format!("there is no proposal {wrong}: the diagram has {count}"));
        }
        let ticked: Vec<usize> = numbers.iter().map(|n| n - 1).collect();
        let changes = super::diagrams::apply_text(&dir, &text, &ticked)?;
        // Apply writes files; it takes none away.
        for change in changes {
            if let Some(after) = change.after {
                self.written.push(Written { file: change.file, before: change.before, after });
            }
        }
        Ok(format!("Applied {} proposals. {}", numbers.len(), self.problems()?))
    }

    /// Writes what the data dictionary changed since the last migration step
    /// as the next step, as the Migrate button does.
    fn migrate(&mut self, input: &Value) -> Result<String, String> {
        let (file, dir) = self.open()?.clone();
        let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read the project: {err}"))?;
        if project.diagnostics.iter().any(|d| d.severity == Severity::Error) {
            return Err("fix the errors first: a step is written from a project that checks clean".into());
        }
        let first = clarity_project::load(&file).settings.and_then(|s| s.datasources.first().map(|d| d.name.clone())).unwrap_or_else(|| "main".into());
        let source = input["source"].as_str().filter(|s| !s.trim().is_empty()).map(String::from).unwrap_or(first);
        let renames = clarity_spec::migrate::proposed_renames(&project, &source);
        let ops = clarity_spec::migrate::pending(&project, &source, &renames);
        if ops.is_empty() {
            return Ok(format!("Nothing to migrate in {source}: every change is in a step already."));
        }
        let title = input["title"].as_str().filter(|t| !t.trim().is_empty()).unwrap_or("Changes by the agent");
        let path = clarity_spec::write_step(&dir, &source, title.trim(), &ops).map_err(|err| format!("cannot write the step: {err}"))?;
        let text = std::fs::read_to_string(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
        self.written.push(Written { file: path.clone(), before: None, after: text.clone() });
        Ok(format!("Wrote the step {}:\n{text}\n{}", path.display(), self.problems()?))
    }

    /// The folder the project is generated into, as its `output` setting says.
    fn out(&self) -> Result<PathBuf, String> {
        let (file, _) = self.open()?;
        let loaded = clarity_project::load(file);
        let settings = loaded.settings.ok_or_else(|| format!("the project file cannot be read: {}", loaded.problems.join("; ")))?;
        let out = clarity_project::out_dir(file, &settings);
        if !out.is_dir() {
            return Err(format!("the project is not generated yet ({} does not exist); Run or Build generates it", out.display()));
        }
        Ok(out)
    }

    /// The generated project, read only: without a path, its files; with one,
    /// that file. The toolchain's own folders (bin, obj, target) are left out
    /// of the list but can be read by name.
    fn generated(&self, path: &str) -> Result<String, String> {
        let out = self.out()?;
        let path = path.trim();
        if path.is_empty() {
            let (file, _) = self.open()?;
            // The build tools' own folders, and the ones the project keeps.
            let settings = clarity_project::load(file).settings;
            let mut skipped: Vec<&str> = Vec::new();
            if let Some(settings) = &settings {
                skipped.extend(clarity_gen::build_output(settings.stack.backend).iter().copied());
                skipped.extend(settings.keeps.iter().map(String::as_str));
            }
            let mut files = Vec::new();
            collect_generated(&out, &out, &skipped, &mut files);
            files.sort();
            // One shape of path on every system, as the agent is asked to give it back.
            let list: Vec<String> = files.iter().take(500).map(|f| format!("- {}", clarity_project::slashed(f))).collect();
            return Ok(format!("The generated project in {} ({} files; read one by its path):\n{}", out.display(), files.len(), list.join("\n")));
        }
        let relative = Path::new(path);
        if !relative.is_relative() || !relative.components().all(|c| matches!(c, std::path::Component::Normal(_))) {
            return Err(format!("'{path}' is not inside the generated project: use a path like desktop/Modules/Todo.cs, as the list gives it"));
        }
        let bytes = std::fs::read(out.join(relative)).map_err(|_| format!("the generated project has no file {path}"))?;
        let text = String::from_utf8(bytes).map_err(|_| format!("{path} is not a text file"))?;
        Ok(match text.char_indices().nth(GENERATED_LIMIT) {
            Some((cut, _)) => format!("{}\n[the first {GENERATED_LIMIT} characters of {path}; the rest is left out]", &text[..cut]),
            None => text,
        })
    }

    /// Runs or builds the application, after the person allows it.
    fn build(&mut self, input: &Value) -> Result<String, String> {
        let run = input["run"].as_bool().unwrap_or(false);
        let (file, dir) = self.open()?.clone();
        if self.runs == Runs::Never {
            return Err("running and building are turned off in Clarity's settings (Agent); ask the person to press Run or Build".into());
        }
        // A custom part or a library, built on its own in its folder.
        if let Some(part) = input["part"].as_str().map(str::trim).filter(|p| !p.is_empty()) {
            let folder = self.part_folders()?.into_iter().find(|f| f.name == part).ok_or_else(|| format!("there is no custom part or library {part} with a folder"))?;
            let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read the project: {err}"))?;
            if !super::parts::builds_alone(&project, part) {
                return Err(format!("{} {part} is built with the program: build the application instead", folder.kind));
            }
            let execute = self.execute.as_mut().ok_or("running and building are not available here")?;
            let what = format!("Build {} {part} on its own, in {}", folder.kind, folder.folder);
            if self.runs == Runs::Ask && !execute.ask(&what) {
                return Ok("The person denied this, so nothing ran. Do not ask again in this turn; say what they can do instead.".into());
            }
            let output = execute.build_part(&dir, part)?;
            self.output = output.clone();
            return Ok(output);
        }
        let execute = self.execute.as_mut().ok_or("running and building are not available here")?;
        let what = if run { "Run the application: generate it, build it and start it" } else { "Build the application: generate it and build it" };
        if self.runs == Runs::Ask && !execute.ask(what) {
            return Ok("The person denied this, so nothing ran. Do not ask again in this turn; say what they can do instead.".into());
        }
        let output = execute.build(&file, run)?;
        self.output = output.clone();
        Ok(output)
    }

    fn new_project(&mut self, input: &Value) -> Result<String, String> {
        let name = input["name"].as_str().unwrap_or_default().trim();
        if name.is_empty() {
            return Err("a project needs a name, like todo".into());
        }
        let folder = input["folder"].as_str().map(PathBuf::from).unwrap_or_else(|| self.projects_folder.clone());
        let stack: Stack = match input.get("stack") {
            Some(stack) if !stack.is_null() => serde_json::from_value(stack.clone()).map_err(|err| format!("the stack does not fit: {err}"))?,
            _ => clarity_project::starter("App", "app").stack,
        };
        let file = super::write_new_project(&folder.join(name), stack)?;
        let dir = clarity_project::module_dir(&file);
        self.project = Some((file.clone(), dir));
        self.made = Some(file.clone());
        Ok(format!("Made the project {}; it is the open project now.\n\n{}", file.display(), self.overview()?))
    }
}

/// Names as a sentence: "a, b and c".
fn listed(names: &[&str]) -> String {
    match names.split_last() {
        Some((last, [])) => last.to_string(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => "nothing".to_string(),
    }
}

/// How much of one generated file the agent reads.
const GENERATED_LIMIT: usize = 60_000;

/// Every file under `folder`, relative to `root`, leaving out the names in
/// `skipped` at the top (the toolchain's output).
fn collect_generated(root: &Path, folder: &Path, skipped: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        // The toolchain's own folders, in each program's folder (`out/desktop`, `out/cli`).
        let in_program = folder.parent() == Some(root);
        if (folder == root || in_program) && skipped.iter().any(|s| entry.file_name() == *s) {
            continue;
        }
        if path.is_dir() {
            collect_generated(root, &path, skipped, out);
        } else {
            out.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
}

/// Every `.clarity` file under `folder`, relative to `root`.
fn collect(root: &Path, folder: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out);
        } else if is_spec_file(&path) {
            out.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
}

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required })
}

impl Tools for ProjectTools {
    fn definitions(&self) -> Vec<ToolDef> {
        let tool = |name: &str, description: &str, input_schema: Value| ToolDef { name: name.into(), description: description.into(), input_schema };
        let mut tools = vec![
            tool("project_overview", "The open project: its project file, its modules and its problems.", schema(json!({}), &[])),
            tool(
                "read_file",
                "Reads a module (a path like todo.clarity, relative to the project's folder), the project file (`project`) or a Markdown file of the resources (resources/rules-en.md).",
                schema(json!({ "path": { "type": "string" } }), &["path"]),
            ),
            tool(
                "write_file",
                "Writes a whole module (a path like todo.clarity), the project file (`project`) or a Markdown file of the resources (resources/rules-en.md), new or replacing it. Gives the project's problems after the write.",
                schema(json!({ "path": { "type": "string" }, "text": { "type": "string", "description": "The whole file." } }), &["path", "text"]),
            ),
            tool("check", "Checks the whole project and gives its problems.", schema(json!({}), &[])),
            tool(
                "api",
                "What a web API data source's OpenAPI description offers: its operations and schemas, which the project picks, and how the source pages. Read it before you pick.",
                schema(json!({ "source": { "type": "string", "description": "The data source, like forms." } }), &["source"]),
            ),
            tool(
                "pick_api",
                "Picks what the application uses of a web API's description, in place of what was picked: each operation becomes an action and the schemas it takes and gives entities, which Clarity writes into apis/<source>.clarity. Gives the project's problems after.",
                schema(
                    json!({
                        "source": { "type": "string", "description": "The data source, like forms." },
                        "operations": { "type": "array", "items": { "type": "string" }, "description": "Operation ids, as api lists them." },
                        "schemas": { "type": "array", "items": { "type": "string" }, "description": "Optional: schemas the application uses on their own." }
                    }),
                    &["source", "operations"],
                ),
            ),
            tool(
                "migrate",
                "Writes the changes of the data dictionary since the last migration step as the next step, so the database follows. Use it after adding or changing entities, when check warns that they are not in a migration yet.",
                schema(json!({ "title": { "type": "string", "description": "What the step does, like \"Tasks\"." }, "source": { "type": "string", "description": "The data source; the first one by default." } }), &[]),
            ),
            tool("parts", "The parts a window can hold, with their properties, sizes and events, as JSON.", schema(json!({}), &[])),
            tool(
                "manual",
                "Reads a topic of Clarity's manual by its id (like code.queries), as listed in your instructions. Read the topics a task needs before you write: how to write handlers, queries, windows, data and the rest.",
                schema(json!({ "topic": { "type": "string", "description": "A topic id, like code.queries." } }), &["topic"]),
            ),
            tool(
                "reference",
                "Reads a section of the Clarity language reference, the exact syntax of every construct. Ask by a section's number (like 6) or a word of its title (like queries). Without a section it gives the contents.",
                schema(json!({ "section": { "type": "string", "description": "A section's number or a word of its title." } }), &[]),
            ),
            tool(
                "design",
                "Reads Clarity's design document: what Clarity aims at, which stacks there are and what each supports, and the roadmap of what is done and what is planned. Without a section it gives the contents.",
                schema(json!({ "section": { "type": "string", "description": "A section's number (like 19 or 1.2) or a word of its title (like roadmap)." } }), &[]),
            ),
            tool(
                "new_project",
                "Makes a new project with its first module, an app and a main window, and opens it. Use it when the person asks for a new application.",
                schema(
                    json!({
                        "name": { "type": "string", "description": "The project's file name, like todo." },
                        "folder": { "type": "string", "description": "The folder to make it in; the folder of the open project by default." },
                        "stack": { "type": "object", "description": "Optional: { backend: rust|go|csharp|c|cpp, ui: web|avalonia, targets: [desktop, cli] }." }
                    }),
                    &["name"],
                ),
            ),
            tool(
                "read_generated",
                "Reads the application Clarity generated from the project (in the folder the project's output setting names), read only. Each program has its folder there: desktop for the windowed application, cli for the command-line program. Without a path it lists the generated files; with one (like desktop/Modules/Todo.cs) it gives that file. Use it with build_output to see why a build failed.",
                schema(json!({ "path": { "type": "string", "description": "A path inside the generated project, as the list gives it." } }), &[]),
            ),
            tool(
                "build_output",
                "Reads Clarity's Output panel: the last Run or Build of the application (or of an engine), how it ended, and the toolchain's lines, such as compiler errors in the generated code.",
                schema(json!({}), &[]),
            ),
            tool("show_window", "Shows a window of the project in Clarity's Preview.", schema(json!({ "name": { "type": "string" } }), &["name"])),
            tool(
                "diagram_view",
                "The project's Data model as a Mermaid ERD that Clarity writes from the data dictionary: every entity, its fields and relationships. Read it before you sketch a change to the data.",
                schema(json!({}), &[]),
            ),
            tool(
                "write_diagram",
                "Writes a Mermaid ERD or class diagram into the project's diagrams folder (diagrams/<name>.mermaid), new or replacing it. It changes nothing else: it gives the diagram's problems and the numbered changes it proposes for the project.",
                schema(
                    json!({ "name": { "type": "string", "description": "The diagram's name, like billing." }, "text": { "type": "string", "description": "The whole Mermaid text." } }),
                    &["name", "text"],
                ),
            ),
            tool(
                "diagram_proposals",
                "What a written diagram proposes for the project now, numbered, with its problems. Use it after the person changed the diagram.",
                schema(json!({ "name": { "type": "string", "description": "The diagram's name, like billing." } }), &["name"]),
            ),
            tool(
                "apply_diagram",
                "Makes the proposals of a written diagram that you name by their numbers: entities, fields, refs, lists and enums, with label keys in every language file. Only when the person asked for it.",
                schema(
                    json!({
                        "name": { "type": "string", "description": "The diagram's name, like billing." },
                        "proposals": { "type": "array", "items": { "type": "integer" }, "description": "The numbers of the proposals to apply, as diagram_proposals lists them." }
                    }),
                    &["name", "proposals"],
                ),
            ),
            tool("show_diagram", "Opens a written diagram in Clarity, drawn with what it proposes marked.", schema(json!({ "name": { "type": "string" } }), &["name"])),
        ];
        // Clarity's settings can turn running off; then the agent has no such tool.
        if self.runs != Runs::Never {
            tools.push(tool(
                "build",
                "Runs the application (run: true: generate, build and start it, as the Run button does) or builds it (run: false, as the Build button does). With part, builds that custom part or library on its own in its folder (C and C++ with CMake, Rust with Cargo, Go with go build), to see its compiler's errors. The person may be asked first and may deny it. Gives the build's output.",
                schema(
                    json!({
                        "run": { "type": "boolean", "description": "true to start the application after building it." },
                        "part": { "type": "string", "description": "Optional: a custom part or library to build on its own, like Arena." }
                    }),
                    &[],
                ),
            ));
        }
        tools
    }

    fn call(&mut self, name: &str, input: &Value) -> ToolResult {
        let text = |key: &str| input[key].as_str().unwrap_or_default().to_string();
        let result = match name {
            "project_overview" => self.overview(),
            "read_file" => self.read(&text("path")),
            "write_file" => self.write(&text("path"), &text("text")),
            "check" => self.problems(),
            "api" => self.api(&text("source")),
            "pick_api" => self.pick_api(input),
            "parts" => serde_json::to_string(&clarity_spec::widgets::catalog()).map_err(|err| err.to_string()),
            "design" => section(DESIGN, "design document", &text("section")),
            "manual" => manual_topic(&text("topic")),
            "reference" => section(LANGUAGE, "language reference", &text("section")),
            "new_project" => self.new_project(input),
            "migrate" => self.migrate(input),
            "build_output" => Ok(self.output.clone()),
            "read_generated" => self.generated(&text("path")),
            "build" => self.build(input),
            "show_window" => {
                self.shown = Some(text("name"));
                Ok(format!("Showing {} in Preview.", text("name")))
            }
            "diagram_view" => self.diagram_view(),
            "write_diagram" => self.write_diagram(&text("name"), &text("text")),
            "diagram_proposals" => self.diagram_text(&text("name")).and_then(|(_, t)| self.proposals_text(&t)),
            "apply_diagram" => {
                let numbers: Vec<usize> = input["proposals"].as_array().map(|a| a.iter().filter_map(|n| n.as_u64()).map(|n| n as usize).collect()).unwrap_or_default();
                self.apply_diagram(&text("name"), &numbers)
            }
            "show_diagram" => self.diagram_text(&text("name")).map(|(file, _)| {
                let shown = file.file_name().unwrap_or_default().to_string_lossy().to_string();
                self.shown_diagram = Some(shown.clone());
                format!("Opening diagrams/{shown} in Clarity.")
            }),
            other => Err(format!("there is no tool {other}")),
        };
        match result {
            Ok(text) => ToolResult { text, is_error: false },
            Err(text) => ToolResult { text, is_error: true },
        }
    }
}

/// The changes of a turn as one change for Undo: each file once, from its
/// text before the turn to its text after.
pub fn merged(written: &[Written]) -> Vec<Written> {
    let mut out: Vec<Written> = Vec::new();
    for w in written {
        match out.iter_mut().find(|o| o.file == w.file) {
            Some(o) => o.after = w.after.clone(),
            None => out.push(w.clone()),
        }
    }
    out
}

/// The conversation, and whether a turn runs.
#[derive(Default)]
pub struct AgentState {
    conversation: Mutex<Vec<Message>>,
    running: AtomicBool,
    stop: Arc<AtomicBool>,
    /// The question waiting for the person's answer, and where the answer goes.
    asking: Mutex<Option<(u32, Sender<bool>)>>,
    last_ask: AtomicU32,
}

/// The agent asks the person whether it may do something.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentAsk {
    pub id: u32,
    pub what: String,
}

/// The person's answer (or a stop, which denies).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentAnswered {
    pub id: u32,
    pub allowed: bool,
}

/// Asks in the panel and builds with the toolbar's Run and Build.
struct AppExecute {
    app: AppHandle,
    stop: Arc<AtomicBool>,
}

impl Execute for AppExecute {
    fn ask(&mut self, what: &str) -> bool {
        let state = self.app.state::<AgentState>();
        let id = state.last_ask.fetch_add(1, Ordering::SeqCst) + 1;
        let (send, answer) = std::sync::mpsc::channel();
        if let Ok(mut asking) = state.asking.held() {
            *asking = Some((id, send));
        }
        let _ = self.app.emit("agent-ask", AgentAsk { id, what: what.to_string() });
        // Waits for Allow or Deny; Stop denies.
        let allowed = loop {
            match answer.recv_timeout(Duration::from_millis(200)) {
                Ok(allowed) => break allowed,
                Err(RecvTimeoutError::Timeout) if !self.stop.load(Ordering::SeqCst) => {}
                Err(_) => break false,
            }
        };
        if let Ok(mut asking) = state.asking.held() {
            *asking = None;
        }
        let _ = self.app.emit("agent-answered", AgentAnswered { id, allowed });
        allowed
    }

    fn build(&mut self, file: &Path, run: bool) -> Result<String, String> {
        // As the toolbar: Run is a debug build that starts, Build a release build.
        crate::build::start(self.app.clone(), file.to_path_buf(), !run, run, None)?;
        Ok(self.finished())
    }

    fn build_part(&mut self, dir: &Path, name: &str) -> Result<String, String> {
        crate::parts::start_part_build(self.app.clone(), dir, name)?;
        Ok(self.finished())
    }
}

impl AppExecute {
    /// Waits for the build to end; gives the Output panel's text.
    fn finished(&self) -> String {
        let builds = self.app.state::<crate::build::BuildState>();
        while builds.busy.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(200));
        }
        builds.output.held().map(|o| o.text()).unwrap_or_default()
    }
}

/// The person's answer to the agent's question.
#[tauri::command]
pub fn agent_answer(id: u32, allowed: bool, agent: State<AgentState>) -> Result<(), String> {
    let asking = agent.asking.held().map_err(|_| "the question is not available".to_string())?;
    match asking.as_ref() {
        Some((asked, send)) if *asked == id => {
            let _ = send.send(allowed);
            Ok(())
        }
        _ => Err("the agent is no longer asking this".into()),
    }
}

/// Sent to the screen while a turn runs, and when it ends.
const AGENT_EVENT: &str = "agent-event";

/// What the screen does when a turn ends.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentEnded {
    /// The project made during the turn, to open.
    pub opened: Option<String>,
    /// The window to show in Preview.
    pub shown: Option<String>,
    /// The written diagram to open, by its file name.
    pub diagram: Option<String>,
    /// Whether files changed, so Undo can take the turn back.
    pub changed: bool,
}

/// The model of a connection: its kind, address, model, key and length limit.
fn model_of(app: &AppHandle, connection: &str) -> Result<clarity_agent::anthropic::Anthropic, String> {
    let vault = app.state::<super::vault::VaultState>();
    let field = |key: &str| super::vault::resolve(&vault, connection, key);
    let kind = field("kind")?;
    if !kind.is_empty() && !kind.eq_ignore_ascii_case("anthropic") {
        return Err(format!("the connection {connection} is of kind {kind}; Clarity's agent speaks to Anthropic models for now"));
    }
    let api_key = field("apikey")?;
    if api_key.is_empty() {
        return Err(format!("the connection {connection} has no apikey"));
    }
    let base_url = Some(field("baseUrl")?).filter(|u| !u.is_empty()).unwrap_or_else(|| "https://api.anthropic.com".into());
    let model = field("model")?;
    if model.is_empty() {
        return Err(format!("the connection {connection} names no model"));
    }
    let max_tokens = field("maxTokens")?.trim().parse().unwrap_or(16384);
    // How long the model keeps the conversation cached: 5m (the default) or 1h.
    let ttl = field("cacheTtl")?;
    let cache = clarity_agent::anthropic::Cache::from_field(&ttl).ok_or_else(|| format!("the connection {connection} has cacheTtl {ttl}; use 5m or 1h"))?;
    clarity_agent::anthropic::Anthropic::new(&base_url, &api_key, &model, max_tokens, cache)
}

/// The budget of a turn when the connection names none: tokens at full
/// price (cache reads are not counted).
const DEFAULT_BUDGET: u64 = 200_000;

/// A connection's `turnBudget`: tokens like `250000`, `80k` or `1.5M`, the
/// default when empty, or `none` for no budget.
fn turn_budget(text: &str) -> Result<Option<u64>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(Some(DEFAULT_BUDGET));
    }
    if text.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let (number, scale) = match text.chars().last() {
        Some('k' | 'K') => (&text[..text.len() - 1], 1_000.0),
        Some('m' | 'M') => (&text[..text.len() - 1], 1_000_000.0),
        _ => (text, 1.0),
    };
    number
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| *n > 0.0)
        .map(|n| Some((n * scale).round() as u64))
        .ok_or_else(|| format!("turnBudget {text} is no number of tokens; write it like 200k, 250000 or none"))
}

/// Starts a turn: the person's words go to the model, which works on the
/// open project. Returns at once; the turn reports through events.
#[tauri::command]
pub fn agent_send(text: String, connection: String, runs: Runs, app: AppHandle, agent: State<AgentState>) -> Result<(), String> {
    // The key is read before the turn starts, so a locked vault asks for its password first.
    let mut model = model_of(&app, &connection)?;
    let budget = {
        let vault = app.state::<super::vault::VaultState>();
        turn_budget(&super::vault::resolve(&vault, &connection, "turnBudget")?).map_err(|err| format!("the connection {connection}: {err}"))?
    };
    if agent.running.swap(true, Ordering::SeqCst) {
        return Err("the agent is still working; stop it first".into());
    }
    agent.stop.store(false, Ordering::SeqCst);
    let stop = agent.stop.clone();
    let project = open_project_of(&app.state::<WatchState>()).ok();
    // New projects go to the projects folder of Clarity settings, else beside the open one.
    let projects_folder = clarity_project::Folders::current()
        .projects
        .or_else(|| project.as_ref().and_then(|(file, _)| file.parent().map(Path::to_path_buf)))
        .unwrap_or_else(|| std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("Clarity"));
    std::thread::spawn(move || {
        let state = app.state::<AgentState>();
        let mut tools = ProjectTools::new(project, projects_folder);
        tools.execute = Some(Box::new(AppExecute { app: app.clone(), stop: stop.clone() }));
        tools.runs = runs;
        if let Ok(output) = app.state::<crate::build::BuildState>().output.held() {
            tools.output = output.text();
        }
        let mut conversation = state.conversation.held().map(|c| c.clone()).unwrap_or_default();
        let agent = Agent { system: instructions(), max_steps: MAX_STEPS, budget };
        let _ = agent.run(&mut model, &mut tools, &mut conversation, &text, &stop, &mut |event| {
            let _ = app.emit(AGENT_EVENT, &event);
        });
        if let Ok(mut kept) = state.conversation.held() {
            *kept = conversation;
        }
        let changes: Vec<FileChange> =
            merged(&tools.written).into_iter().map(|w| FileChange { file: w.file, before: w.before, after: Some(w.after) }).collect();
        let changed = !changes.is_empty();
        // A project made during the turn opens with a history of its own.
        if tools.made.is_none() && changed {
            let _ = record(&app.state::<HistoryState>(), changes);
        }
        state.running.store(false, Ordering::SeqCst);
        let ended = AgentEnded { opened: tools.made.map(|f| f.display().to_string()), shown: tools.shown, diagram: tools.shown_diagram, changed };
        let _ = app.emit("agent-ended", &ended);
    });
    Ok(())
}

/// Stops the turn after the step it is in.
#[tauri::command]
pub fn agent_stop(agent: State<AgentState>) {
    agent.stop.store(true, Ordering::SeqCst);
}

/// Forgets the conversation, to start a new one.
#[tauri::command]
pub fn agent_forget(agent: State<AgentState>) -> Result<(), String> {
    if agent.running.load(Ordering::SeqCst) {
        return Err("the agent is still working; stop it first".into());
    }
    agent.conversation.held().map_err(|_| "the conversation is not available".to_string())?.clear();
    Ok(())
}

/// Silences the unused-event warning of the TypeScript export.
#[allow(dead_code)]
fn exported(_: Event) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A project in a scratch folder: the shop example's project file and modules.
    fn project(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("clarity-agent-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = super::super::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        (root, file, dir)
    }

    /// An OpenAPI description in the project (DESIGN 5.23): the agent reads
    /// what it offers and picks what the application uses, as the API...
    /// panel does, through the project file, so one Undo takes it back.
    #[test]
    fn the_agent_reads_an_api_and_picks_what_it_uses() {
        let (root, file, dir) = project("api");
        let description = "openapi: 3.0.3\ninfo: { title: Forms, version: 1.0.0 }\npaths:\n  /templates:\n    get:\n      operationId: listTemplates\n      summary: Lists the templates\n      responses:\n        \"200\":\n          description: The templates\n          content:\n            application/json:\n              schema: { type: array, items: { $ref: \"#/components/schemas/Template\" } }\n    post:\n      operationId: createTemplate\n      responses: { \"201\": { description: Made } }\ncomponents:\n  schemas:\n    Template:\n      type: object\n      properties:\n        name: { type: string }\n";
        std::fs::create_dir_all(dir.join("apis")).unwrap();
        std::fs::write(dir.join("apis/forms.yaml"), description).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        std::fs::write(&file, format!("{text}    datasource forms rest \"https://forms.example.com/api\"\n        paging page \"page\" size \"per_page\" 50\n        openapi \"forms.yaml\"\n")).unwrap();
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());

        let listed = tools.call("api", &json!({"source": "forms"}));
        assert!(!listed.is_error, "{}", listed.text);
        for line in ["forms.yaml (Forms 1.0.0)", "paging page \"page\" size \"per_page\" 50", "[ ] listTemplates GET /templates: Lists the templates", "[ ] createTemplate POST /templates", "[ ] Template"] {
            assert!(listed.text.contains(line), "no {line:?} in\n{}", listed.text);
        }

        let picked = tools.call("pick_api", &json!({"source": "forms", "operations": ["listTemplates"]}));
        assert!(!picked.is_error, "{}", picked.text);
        assert!(std::fs::read_to_string(&file).unwrap().contains("        pick \"listTemplates\"\n"));
        let module = std::fs::read_to_string(dir.join("apis/forms.clarity")).unwrap();
        assert!(module.contains("action list_templates() -> list<Template>") && module.contains("every page"), "{module}");
        assert!(picked.text.contains("No problems."), "{}", picked.text);
        assert!(tools.call("api", &json!({"source": "forms"})).text.contains("[x] listTemplates"));
        // One change of the project file, for Undo.
        assert_eq!(tools.written.iter().filter(|w| w.file == file).count(), 1);

        let wrong = tools.call("pick_api", &json!({"source": "forms", "operations": ["listForms"]}));
        assert!(wrong.is_error && wrong.text.contains("forms.yaml has no operation listForms; it has listTemplates and createTemplate"), "{}", wrong.text);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_tools_read_write_and_check_the_project() {
        let (root, file, dir) = project("tools");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let overview = tools.call("project_overview", &json!({}));
        assert!(!overview.is_error);
        assert!(overview.text.contains("- todo.clarity") && overview.text.contains("No problems."), "{}", overview.text);

        let read = tools.call("read_file", &json!({"path": "todo.clarity"}));
        assert!(read.text.starts_with("module todo"), "{}", read.text);
        assert!(!tools.call("read_file", &json!({"path": "project"})).is_error);

        // A write is checked at once: the model sees what it broke.
        let broken = tools.call("write_file", &json!({"path": "tasks.clarity", "text": "module tasks\n\nentity Task\n    title: nonsense\n"}));
        assert!(!broken.is_error);
        assert!(broken.text.contains("error(s)") && broken.text.contains("tasks.clarity:"), "{}", broken.text);
        // A label is a key, whose text the default language's file gives.
        let en = tools.call("read_file", &json!({"path": "translations/en.clarity"}));
        assert!(en.text.starts_with("translation en"), "{}", en.text);
        let with_key = format!("{}\n    taskTitle \"Title\"\n", en.text.trim_end());
        assert!(!tools.call("write_file", &json!({"path": "translations/en.clarity", "text": with_key})).is_error);
        let fixed = tools.call("write_file", &json!({"path": "tasks.clarity", "text": "module tasks\n\nentity Task\n    title: text(120), required, label @taskTitle"}));
        assert!(fixed.text.contains("No errors") && fixed.text.contains("not in a migration yet"), "{}", fixed.text);
        // A migration step takes the warning away.
        let step = tools.call("migrate", &json!({"title": "Tasks"}));
        assert!(!step.is_error && step.text.contains("add entity Task") && step.text.ends_with("No problems."), "{}", step.text);
        assert_eq!(std::fs::read_to_string(dir.join("tasks.clarity")).unwrap(), "module tasks\n\nentity Task\n    title: text(120), required, label @taskTitle\n");

        // Paths stay inside the project.
        for path in ["../escape.clarity", "/etc/passwd", "notes.txt", ""] {
            assert!(tools.call("write_file", &json!({"path": path, "text": "x"})).is_error, "{path}");
        }
        assert!(tools.call("format_disk", &json!({})).is_error);

        // The turn is one change for Undo: each file from before the turn to after it.
        let one = merged(&tools.written);
        assert_eq!(one.len(), 3, "the module, the language file and the step");
        let module = one.iter().find(|c| c.file.ends_with("tasks.clarity")).unwrap();
        assert_eq!(module.before, None);
        assert!(module.after.contains("text(120)"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_agent_sketches_a_diagram_first_and_applies_it_when_asked() {
        let (root, file, dir) = project("diagrams");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let view = tools.call("diagram_view", &json!({}));
        assert!(!view.is_error && view.text.starts_with("erDiagram"), "{}", view.text);

        let sketch = "erDiagram\n    Customer {\n        text(80) name \"required\"\n    }\n    Invoice {\n        date issued_on \"required\"\n    }\n    Invoice }o..|| Customer : customer\n";
        let written = tools.call("write_diagram", &json!({"name": "billing", "text": sketch}));
        assert!(!written.is_error, "{}", written.text);
        assert!(written.text.contains("diagrams/billing.mermaid"), "{}", written.text);
        assert!(written.text.contains("1. add Customer: entity Customer"), "{}", written.text);
        assert!(written.text.contains("2. add Invoice: entity Invoice"), "{}", written.text);
        assert_eq!(std::fs::read_to_string(dir.join("diagrams/billing.mermaid")).unwrap(), sketch);
        // The person may change it; the agent reads it back.
        assert_eq!(tools.call("read_file", &json!({"path": "diagrams/billing.mermaid"})).text, sketch);
        assert!(tools.call("diagram_proposals", &json!({"name": "billing"})).text.contains("2. add Invoice"));

        let applied = tools.call("apply_diagram", &json!({"name": "billing", "proposals": [1, 2]}));
        assert!(!applied.is_error, "{}", applied.text);
        assert!(applied.text.contains("Applied 2 proposals"), "{}", applied.text);
        let module = std::fs::read_to_string(dir.join("todo.clarity")).unwrap();
        assert!(module.contains("entity Invoice\n    issued_on: date, required, label @invoiceIssuedOnLabel\n    customer: ref Customer"), "{module}");
        let en = std::fs::read_to_string(dir.join("translations/en.clarity")).unwrap();
        assert!(en.contains("invoiceIssuedOnLabel \"Issued on\""), "{en}");
        assert!(tools.call("diagram_proposals", &json!({"name": "billing"})).text.contains("matches the project"));

        // The diagram, the module and the language file are one change for Undo.
        let one = merged(&tools.written);
        assert!(one.iter().any(|c| c.file.ends_with("diagrams/billing.mermaid")));
        assert!(one.iter().any(|c| c.file.ends_with("todo.clarity")));
        assert!(one.iter().any(|c| c.file.ends_with("translations/en.clarity")));

        tools.call("show_diagram", &json!({"name": "billing"}));
        assert_eq!(tools.shown_diagram.as_deref(), Some("billing.mermaid"));
        for name in ["../escape", "a/b", ""] {
            assert!(tools.call("write_diagram", &json!({"name": name, "text": sketch})).is_error, "{name}");
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// What the agent says follows Clarity's house style: no dashes for
    /// asides, plain sentences. Its instructions say so, and are written so.
    #[test]
    fn the_instructions_follow_the_house_style() {
        let text = instructions();
        for dash in ['\u{2014}', '\u{2013}'] {
            assert!(!text.contains(dash), "the instructions hold {dash:?}");
        }
        assert!(text.contains("em-dash") && text.contains("en-dash"), "the instructions name the rule");
    }

    #[test]
    fn a_new_project_becomes_the_open_one() {
        let root = std::env::temp_dir().join(format!("clarity-agent-new-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut tools = ProjectTools::new(None, root.clone());
        assert!(tools.call("check", &json!({})).text.contains("no project is open"));
        let made = tools.call("new_project", &json!({"name": "todo"}));
        assert!(!made.is_error, "{}", made.text);
        assert_eq!(tools.made, Some(root.join("todo.clarity")));
        assert!(made.text.contains("No problems."), "{}", made.text);
        assert!(!tools.call("read_file", &json!({"path": "todo.clarity"})).is_error);
        tools.call("show_window", &json!({"name": "Main"}));
        assert_eq!(tools.shown.as_deref(), Some("Main"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// The agent executes nothing by itself: its tools read, write project
    /// files and check them. Only `build` runs something, and it asks the
    /// person first. Any other tool that runs something needs this list
    /// changed on purpose.
    #[test]
    fn only_the_build_tool_executes_and_it_asks_first() {
        let tools = ProjectTools::new(None, std::env::temp_dir());
        let mut names: Vec<String> = tools.definitions().into_iter().map(|t| t.name).collect();
        names.sort();
        let allowed = [
            // Reading.
            "api", "build_output", "check", "design", "diagram_proposals", "diagram_view", "manual", "parts", "project_overview", "read_file",
            "read_generated", "reference",
            // Writing project files, checked at once and undone with one Undo.
            "apply_diagram", "migrate", "new_project", "pick_api", "write_diagram", "write_file",
            // Showing a window in Preview or a diagram, which draw without running anything.
            "show_diagram", "show_window",
            // Running or building, only after the person allows it.
            "build",
        ];
        let mut allowed: Vec<String> = allowed.iter().map(|n| n.to_string()).collect();
        allowed.sort();
        assert_eq!(names, allowed);
    }

    /// Stands in for the person and the toolchain.
    struct Asked {
        allow: bool,
        asked: Vec<String>,
        built: Vec<bool>,
    }

    /// A project with a custom part in Rust, its crate written by Clarity's starter.
    fn with_part(name: &str) -> (PathBuf, PathBuf, PathBuf, ProjectTools) {
        let (root, file, dir) = project(name);
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let declared = tools.call("write_file", &json!({"path": "arena.clarity", "text": "module arena\n\npart Arena\n    crate \"parts/Arena\"\n    hears restart\n    size 4 x 3\n"}));
        assert!(!declared.is_error, "{}", declared.text);
        let project = clarity_project::load_modules(&dir).unwrap();
        clarity_gen::parts::write_part_folders(&dir, &project).unwrap();
        (root, file, dir, tools)
    }

    #[test]
    fn the_agent_writes_a_part_s_code_but_not_clarity_s_file() {
        let (root, _, dir, mut tools) = with_part("part-files");
        let lib = tools.call("read_file", &json!({"path": "parts/Arena/src/lib.rs"}));
        assert!(!lib.is_error && lib.text.contains("impl ArenaPart for Arena"), "{}", lib.text);
        let wrote = tools.call("write_file", &json!({"path": "parts/Arena/src/sound.rs", "text": "pub fn ping() {}"}));
        assert!(!wrote.is_error, "{}", wrote.text);
        assert_eq!(std::fs::read_to_string(dir.join("parts/Arena/src/sound.rs")).unwrap(), "pub fn ping() {}\n");
        // Clarity's own file is read, never written; build output and other folders are out of reach.
        assert!(!tools.call("read_file", &json!({"path": "parts/Arena/src/clarity.rs"})).is_error);
        let own = tools.call("write_file", &json!({"path": "parts/Arena/src/clarity.rs", "text": "x"}));
        assert!(own.is_error && own.text.contains("Clarity writes it from the declaration"), "{}", own.text);
        for path in ["parts/Arena/target/debug/x", "parts/Arena/../escape.rs", "parts/Other/x.rs", "parts/Arena"] {
            assert!(tools.call("write_file", &json!({"path": path, "text": "x"})).is_error, "{path}");
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_overview_lists_parts_and_libraries_with_their_files() {
        let (root, _, _, mut tools) = with_part("part-overview");
        let overview = tools.call("project_overview", &json!({})).text;
        assert!(overview.contains("Custom parts and libraries"), "{overview}");
        assert!(overview.contains("part Arena: Rust, in parts/Arena"), "{overview}");
        assert!(overview.contains("- parts/Arena/src/lib.rs") && overview.contains("- parts/Arena/src/clarity.rs (Clarity's, read only)"), "{overview}");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_build_tool_builds_a_part_on_its_own_after_asking() {
        let (root, _, _, mut tools) = with_part("part-build");
        let person = std::rc::Rc::new(std::cell::RefCell::new(Asked { allow: true, asked: vec![], built: vec![] }));
        tools.execute = Some(Box::new(person.clone()));
        let built = tools.call("build", &json!({"part": "Arena"}));
        assert!(!built.is_error, "{}", built.text);
        assert_eq!(person.borrow().asked, ["Build part Arena on its own, in parts/Arena"]);
        assert!(tools.call("build", &json!({"part": "Nothing"})).is_error);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_agent_is_told_of_parts_libraries_and_sound() {
        let told = instructions();
        assert!(told.contains("custom part") && told.contains("library") && told.contains("sound"), "{told}");
        for topic in ["design.custom-parts", "code.libraries", "design.sound"] {
            assert!(manual_topic(topic).is_ok(), "{topic}");
        }
    }

    impl Execute for std::rc::Rc<std::cell::RefCell<Asked>> {
        fn ask(&mut self, what: &str) -> bool {
            let mut asked = self.borrow_mut();
            asked.asked.push(what.to_string());
            asked.allow
        }

        fn build(&mut self, _file: &Path, run: bool) -> Result<String, String> {
            self.borrow_mut().built.push(run);
            Ok("The last build succeeded: running. Its output (1 line):\ndotnet build".into())
        }

        fn build_part(&mut self, _dir: &Path, name: &str) -> Result<String, String> {
            Ok(format!("The last build succeeded: {name} built."))
        }
    }

    #[test]
    fn the_build_tool_asks_the_person_first() {
        let (root, file, dir) = project("build");
        let person = std::rc::Rc::new(std::cell::RefCell::new(Asked { allow: false, asked: vec![], built: vec![] }));
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        // Without a way to ask, nothing runs.
        assert!(tools.call("build", &json!({"run": true})).is_error);

        tools.execute = Some(Box::new(person.clone()));
        let denied = tools.call("build", &json!({"run": true}));
        assert!(!denied.is_error && denied.text.contains("denied"), "{}", denied.text);
        assert_eq!(person.borrow().asked, ["Run the application: generate it, build it and start it"]);
        assert!(person.borrow().built.is_empty(), "a denied build does not run");

        person.borrow_mut().allow = true;
        let built = tools.call("build", &json!({"run": false}));
        assert!(!built.is_error && built.text.ends_with("dotnet build"), "{}", built.text);
        assert_eq!(person.borrow().asked[1], "Build the application: generate it and build it");
        assert_eq!(person.borrow().built, [false]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Clarity's settings say whether the agent asks, may run without asking, or never runs.
    #[test]
    fn the_settings_say_whether_the_agent_asks() {
        let (root, file, dir) = project("runs");
        let person = std::rc::Rc::new(std::cell::RefCell::new(Asked { allow: false, asked: vec![], built: vec![] }));
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        tools.execute = Some(Box::new(person.clone()));

        tools.runs = Runs::Allow;
        assert!(!tools.call("build", &json!({"run": true})).is_error);
        assert!(person.borrow().asked.is_empty(), "allowed in the settings: no question");
        assert_eq!(person.borrow().built, [true]);

        // Never: the agent is not even offered the tool, and a call does nothing.
        tools.runs = Runs::Never;
        assert!(!tools.definitions().iter().any(|t| t.name == "build"));
        let refused = tools.call("build", &json!({"run": true}));
        assert!(refused.is_error && refused.text.contains("settings"), "{}", refused.text);
        assert_eq!(person.borrow().built, [true]);
        assert_eq!(serde_json::from_value::<Runs>(json!("ask")).unwrap(), Runs::Ask);
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// The agent writes Markdown files of the resources too, and a key points
    /// each language at one; it cannot write pictures.
    #[test]
    fn the_agent_writes_markdown_resources() {
        let (root, file, dir) = project("resources");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let wrote = tools.call("write_file", &json!({"path": "resources/rules-en.md", "text": "# Rules"}));
        assert!(!wrote.is_error, "{}", wrote.text);
        assert_eq!(std::fs::read_to_string(dir.join("resources/rules-en.md")).unwrap(), "# Rules\n");
        assert_eq!(tools.call("read_file", &json!({"path": "resources/rules-en.md"})).text, "# Rules\n");
        assert!(tools.call("project_overview", &json!({})).text.contains("- resources/rules-en.md"));
        for path in ["resources/logo.png", "resources/../escape.md", "resources/a/b.md", "notes.md"] {
            assert!(tools.call("write_file", &json!({"path": path, "text": "x"})).is_error, "{path}");
        }
        let told = instructions();
        assert!(told.contains("resources/") && told.contains("resource \"rules-nl.md\""), "the instructions say nothing of resources");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_agent_reads_the_generated_project() {
        let (root, file, dir) = project("generated");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let none = tools.call("read_generated", &json!({}));
        assert!(none.is_error && none.text.contains("not generated yet"), "{}", none.text);

        // The project's own output folder, not always `out`.
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("output \"out\""), "{text}");
        std::fs::write(&file, text.replace("output \"out\"", "output \"build/app\"")).unwrap();
        let out = dir.join("build/app");
        // Each program in its own folder of the output (DESIGN 5.20).
        std::fs::create_dir_all(out.join("desktop/Modules")).unwrap();
        std::fs::create_dir_all(out.join("desktop/target/debug")).unwrap();
        std::fs::write(out.join("desktop/Modules/Todo.cs"), "Lists.At(xs, 1)").unwrap();
        std::fs::write(out.join("desktop/App.csproj"), "<Project />").unwrap();
        std::fs::write(out.join("desktop/target/debug/app"), "binary").unwrap();

        // The listing leaves the toolchain's own folders out; the stack here is the starter's (Rust, whose output is target).
        let listed = tools.call("read_generated", &json!({}));
        assert!(!listed.is_error, "{}", listed.text);
        assert!(listed.text.contains(&out.display().to_string()), "{}", listed.text);
        assert!(listed.text.contains("- desktop/Modules/Todo.cs") && listed.text.contains("- desktop/App.csproj"), "{}", listed.text);
        assert!(!listed.text.contains("target/"), "{}", listed.text);
        let read = tools.call("read_generated", &json!({"path": "desktop/Modules/Todo.cs"}));
        assert_eq!(read.text, "Lists.At(xs, 1)");
        // What the toolchain wrote can still be read when asked by name.
        assert_eq!(tools.call("read_generated", &json!({"path": "desktop/target/debug/app"})).text, "binary");
        // Only inside out, and only reading.
        for path in ["../todo.clarity", "/etc/passwd", "Modules/../../todo.clarity"] {
            assert!(tools.call("read_generated", &json!({"path": path})).is_error, "{path}");
        }
        assert!(tools.call("read_generated", &json!({"path": "Missing.cs"})).text.contains("no file Missing.cs"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_agent_reads_the_build_output() {
        let mut tools = ProjectTools::new(None, std::env::temp_dir());
        assert_eq!(tools.call("build_output", &json!({})).text, "Nothing was built or run since Clarity started.");
        tools.output = "The last build failed: the build failed. Its output (1 line):\nerror CS0103".into();
        let read = tools.call("build_output", &json!({}));
        assert!(!read.is_error && read.text.ends_with("error CS0103"), "{}", read.text);
        // The model is told to read it rather than ask for a paste.
        assert!(instructions().contains("build_output"));
    }

    const DOC: &str ="# Design\n\nIntro.\n\n## 1. Purpose\n\nWhy.\n\n### 1.2 Stacks\n\nRust and Go.\n\n```\n## not a heading\n```\n\n### 1.3 Data\n\nSQLite.\n\n## 19. Roadmap\n\nG5 is next.\n\n## 20. Open questions\n\nMany.\n";

    #[test]
    fn the_design_tool_gives_the_contents_and_a_section() {
        // Without a section: the headings, to pick from.
        let contents = section(DOC, "design document", "").unwrap();
        assert_eq!(contents.lines().filter(|l| l.starts_with('#')).collect::<Vec<_>>(), ["## 1. Purpose", "### 1.2 Stacks", "### 1.3 Data", "## 19. Roadmap", "## 20. Open questions"]);
        // By number or by title: the section up to the next one of its level.
        assert_eq!(section(DOC, "design document", "19").unwrap(), "## 19. Roadmap\n\nG5 is next.");
        assert_eq!(section(DOC, "design document", "roadmap").unwrap(), "## 19. Roadmap\n\nG5 is next.");
        assert_eq!(section(DOC, "design document", "1.2").unwrap(), "### 1.2 Stacks\n\nRust and Go.\n\n```\n## not a heading\n```");
        // A chapter holds its subsections.
        assert!(section(DOC, "design document", "1").unwrap().contains("### 1.3 Data"));
        assert!(section(DOC, "design document", "teleport").unwrap_err().contains("no section"));
        // The real document has its roadmap.
        assert!(section(DESIGN, "design document", "roadmap").unwrap().starts_with("## 19. Roadmap"));
    }

    #[test]
    fn the_model_is_told_the_manual_and_reads_it_on_demand() {
        let told = instructions();
        // Every topic of the manual, by id with its summary, to choose from.
        for topic in crate::manual::topics() {
            assert!(told.contains(&format!("- {}: ", topic.id)), "{} is not offered", topic.id);
        }
        // The reference's sections by title, not the reference itself.
        assert!(told.contains("Queries") && !told.contains("## 12. Grammar summary\n"));
        assert!(told.contains("manual tool") && told.contains("reference tool") && told.contains("write_file"));
        // How parts and keys are named, as the examples are: never a number, never a statement word.
        for rule in ["never `button1` or `label2`", "`nameLabel`", "`nameHelp`", "`saveOrder`", "not a word code uses, such as `save` or `close`", "`saveCaption`"] {
            assert!(told.contains(rule), "the instructions lack {rule}");
        }
        assert!(!told.contains("name save "), "the example names a part `save`");
        // Small enough for every message: the whole reference was some 18,000 tokens.
        assert!(told.len() < 16_000, "the instructions are {} bytes", told.len());
    }

    #[test]
    fn a_turn_budget_reads_as_tokens() {
        assert_eq!(turn_budget(""), Ok(Some(200_000)));
        assert_eq!(turn_budget("250000"), Ok(Some(250_000)));
        assert_eq!(turn_budget(" 80k "), Ok(Some(80_000)));
        assert_eq!(turn_budget("1.5M"), Ok(Some(1_500_000)));
        assert_eq!(turn_budget("none"), Ok(None));
        assert!(turn_budget("lots").unwrap_err().contains("lots"));
    }

    #[test]
    fn the_manual_and_the_reference_are_read_on_demand() {
        let mut tools = ProjectTools::new(None, std::env::temp_dir());
        let page = tools.call("manual", &json!({"topic": "code.queries"}));
        assert!(!page.is_error && page.text.starts_with("# "), "{}", page.text);
        let missing = tools.call("manual", &json!({"topic": "teleport"}));
        assert!(missing.is_error && missing.text.contains("code.queries"), "{}", missing.text);
        // Without a topic: the list again.
        assert!(tools.call("manual", &json!({})).text.contains("- start.overview: "));

        let section = tools.call("reference", &json!({"section": "Queries"}));
        assert!(!section.is_error && section.text.starts_with("#"), "{}", section.text);
        assert!(tools.call("reference", &json!({})).text.contains("12. Grammar summary"));
        assert!(tools.call("reference", &json!({"section": "teleport"})).text.contains("the language reference has no section"));
    }

}
