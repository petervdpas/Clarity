//! The tools Clarity's agents work with on a project: reading, writing and
//! checking its files, the manual and the references, web APIs, migration
//! steps, diagrams, new projects, generating and reading the generated code.
//! They design and generate only: none of them builds, runs or touches a
//! database. The IDE's agent adds its own tools (build, backups, Preview) on
//! top; the MCP server offers these as they are (docs/MCP-PLAN.md).

pub mod diagrams;
pub mod edits;
pub mod files;
pub mod generate;
pub mod manual;
pub mod new_project;
pub mod parts;
pub mod steps;
pub mod structural;
pub mod turn;

use std::path::{Path, PathBuf};

use clarity_agent::{ToolDef, ToolResult, Tools};
use clarity_project::Stack;
use clarity_spec::Severity;
use serde_json::{Value, json};

use files::{is_spec_file, write_atomically};


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

/// How many texts each language other than the default still lacks (empty,
/// or the default's own text), as `nl 3, de 1`; none when every one has its own.
fn untranslated(project: &clarity_spec::Project) -> Option<String> {
    let counts: Vec<String> = clarity_spec::translations::left_to_translate(project)
        .into_iter()
        .filter(|(_, left)| !left.is_empty())
        .map(|(language, left)| format!("{language} {}", left.len()))
        .collect();
    (!counts.is_empty()).then(|| counts.join(", "))
}

/// The first class a class diagram names as `module.Class` (on a `class`
/// line or in a relation), which Mermaid and Clarity take as no class.
fn dotted_class(text: &str) -> Option<String> {
    let relation = |line: &str| ["--", ".."].iter().any(|arrow| line.contains(arrow));
    text.lines().map(str::trim).filter(|l| !l.starts_with("%%") && (l.starts_with("class ") || relation(l))).find_map(|line| {
        line.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.')).find_map(|word| {
            let (module, class) = word.split_once('.')?;
            let ident = |w: &str| w.chars().next().is_some_and(|c| c.is_alphabetic());
            (ident(module) && class.chars().next().is_some_and(char::is_uppercase)).then(|| word.to_string())
        })
    })
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

/// What the model is told before the conversation: how to work, the
/// manual's topics and the reference's sections. The pages themselves are
/// read on demand, so every message stays small.
/// `host` holds the lines of the agent's own place: the IDE's build,
/// backups and Preview, or what the MCP server offers.
pub fn instructions(host: &[&str]) -> String {
    format!(
        "You are an agent working with Clarity, a development environment where applications are written in the Clarity \
language and designed visually; Clarity transpiles them into real applications (Rust, Go, C#, C and C++ stacks). You \
work on the project that is open in Clarity, through your tools, while the person watches the designer follow your \
changes. You design and generate; building and running is the person's.

How you work:
- Start a task by looking at the project (project_overview), and read a file before you change it.
- README.md holds the project's goal: work toward it; update it when the goal or plan changes.
- Take a request for an application, or a feature of one, in this order. Read the manual topics it touches. Find \
what Clarity does itself: the parts tool lists every built-in part with its properties and events, the manual what \
code can do; use those. Reach for a library or a custom part only for what nothing built in does, \
and tell the person why. When the request needs data, design it in a diagram first (below). Then write the windows \
and their code and fix what each write's check reports. \
Finish by saying what works and what Clarity cannot do yet; never invent syntax or a part to fill a gap.
- Before you write Clarity new to this conversation, check its exact syntax with the reference tool.
- A project is one project file (path `project`), modules (`.clarity` files: app, entities, enums, themes, code) \
and windows and views, one `.cxaml` each in design/ (`design/Main.cxaml`, `<Window class=\"todo.Main\">`), which \
you read but never write.
- Change the data model, windows, views, parts, the app and its frame with the structural tools, which take \
names and check each change. Write code and texts with write_file. Fix every error a change reports, and finish only when check reports no errors and nothing left to translate.
- Give every part a name, and name parts and handlers in camelCase: letters and digits only, starting with a small \
letter, such as `newGame`, `statusText` and `newGameClicked`. Name a part after what it is for, never `button1` or \
`label2`: an input after its field (`name`, `vatNumber`), its label and help after the input (`nameLabel`, \
`nameHelp`), a button after what it does in its window (`saveOrder`, `openAlbum`, `newCustomer`), a container after \
what it holds (`identityPanel`, `albumTabs`). A part's name is not a word code uses, such as `save` or `close`. \
Every other name is camelCase too (`withinCredit`, `isPlaying`), never snake_case; entities, enums and their values, \
windows and the app are PascalCase.
- Every text people read is a key: `title @customerCardTitle`, `button @saveCaption name saveOrder ...`, \
`message @savedName(name = customer.name)`, `label @customerNameLabel` on every field. Give each key its text in the \
default language's file, `translations/en.clarity` (read it first; lines like `saveCaption \"Save\"`), and the same \
key in every other language file, translated; missing_translations lists what is left. Name a key in camelCase \
after its part and role (`identityPanelCaption`, `nameLabel`); parts that say the same share one (`saveCaption`).
- Long texts (rules, help) are Markdown files of the resources, `resources/rules-en.md`, one per language. A markdown part shows one by a key (`markdown @gameRules`), and each \
language file points the key at its file: `gameRules resource \"rules-nl.md\"`. A file that is the same in every \
language is shown by its source (`markdown source \"license.md\"`). Pictures you cannot write; use those the \
overview lists (`image source \"logo.png\"`).
- A custom part (`part Arena`) is a part an outside library draws; a library (`library Words`) gives code functions \
from a package. Both are written in the project's own language: C or C++ (`native`, `cmake`), Rust (`crate`), Go \
(`go`), C# (`nuget`, `dll`); never another stack's. Read the manual topics design.custom-parts and code.libraries \
first. The overview lists their folders and files: read and write those files by their paths (like \
parts/Arena/src/lib.rs). The file Clarity writes from the declaration (src/clarity.rs, clarity.go, the clarity \
folder's header, the .g.cs half) is read only: change the declaration and the file follows. Read it first and \
write exactly the functions it names, with its names and types. In C#, \
`libraries/Player/Player.g.cs` declares `namespace ClarityLibraries;` and `public static partial class Player` with \
a `public static partial` method per function in PascalCase (`isPlaying` is `IsPlaying`, an `integer` is `long`); \
`Player.cs` repeats the namespace, the class and each method as `public static partial`, with its body. Go writes each function in PascalCase in the library's package, Rust as the snake_case methods of the \
trait in src/clarity.rs, C and C++ as the header declares them. Build one on its own with the build tool's part to \
see its compiler's errors. A sound (`sound name rattle source \"roll.wav\"`) plays sound and music files of the \
resources, tones and melodies (`music.play(path)`, `on end`, design.sound); it has no place in the window, and \
music needs no library of its own; use the sound files the overview lists.
- Lay windows out to the full width: inputs and tables reach the last column, labels left of their inputs, buttons \
together at the bottom right.
- Data is designed in the project's one ERD first. When asked for new or changed data (\"an order system with \
customers and invoices\"), write the change into the ERD in diagrams/ (read it first, keep all it holds); with none \
yet, make diagrams/data.mermaid from diagram_view, the whole Data model. Write it with write_diagram, say in a few lines what it proposes, and stop: the person \
adjusts it in Clarity. Apply it with apply_diagram only when the person asks, reading diagram_proposals first since \
they may have changed it. Diagrams propose and the data dictionary decides; never apply a removal unless the person \
asks for that one. A data change is done only when the windows show it: once applied, put each new field on the \
forms of its entity with place_field, say which lists could show it as a column, then show_window. Write a diagram as the reference's section 13 \
(Diagrams) says, every field with its modifiers, and keep its `%% clarity:` lines.
- Entity changes go into a migration step by themselves. Rename an entity, enum or field with the rename tool: a name \
written over is a drop and an add, which loses the data.
- Read generated code with read_generated and never edit it: fix the project, or say what in Clarity needs fixing.
- When asked what Clarity can do or plans, read the manual or the design tool (Stacks, Roadmap) rather than guess; \
never promise what is not built.
- Read only what the task needs: every page you read is sent again with each later message.
- You execute nothing yourself.\n{host}- Say briefly what you did when you finish. Write plain, short sentences.
- Write in Clarity's house style, in what you say and in what you write into the project (comments, texts, Markdown): \
no em-dash and no en-dash (use a period, a comma, a colon or parentheses; a plain hyphen for compounds; for a range \
write \"1 to 9\"). No filler such as \"it is worth noting\", \"moreover\" or \"seamless\", no praise of the question, \
no recap at the end of a short answer.

Clarity's manual (read a topic with the manual tool):
{manual}
The language reference's sections (read one with the reference tool):
{reference}",
        host = host.iter().map(|line| format!("- {line}\n")).collect::<String>(),
        manual = crate::manual::index(),
        reference = contents(LANGUAGE),
    )
}

/// One change of one file during a turn: its text before (none for a new
/// file) and after (none when the turn removed it, as a removed window's file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub file: PathBuf,
    pub before: Option<String>,
    pub after: Option<String>,
}


/// The tools, on one project: the project file and its folder of modules.
pub struct ProjectTools {
    project: Option<(PathBuf, PathBuf)>,
    /// What was written during the turn, oldest first.
    pub written: Vec<Written>,
    /// A project made during the turn, to open when it ends.
    pub made: Option<PathBuf>,
    /// The folder new projects go in by default.
    projects_folder: PathBuf,
}

impl ProjectTools {
    pub fn new(project: Option<(PathBuf, PathBuf)>, projects_folder: PathBuf) -> ProjectTools {
        ProjectTools { project, written: Vec::new(), made: None, projects_folder }
    }

    pub fn open(&self) -> Result<&(PathBuf, PathBuf), String> {
        self.project.as_ref().ok_or_else(|| "no project is open; make one with new_project".to_string())
    }

    /// The file a tool's path names: `project`, or a `.clarity` file in the project's folder.
    fn file_of(&self, path: &str) -> Result<PathBuf, String> {
        let (file, dir) = self.open()?;
        let path = path.trim();
        if path == "project" {
            return Ok(file.clone());
        }
        // The project's description: its design goal and guide.
        if path == clarity_spec::README_FILE {
            return Ok(dir.join(clarity_spec::README_FILE));
        }
        // A Markdown file of the resources: `resources/rules-en.md`, a plain name.
        if let Some(name) = path.strip_prefix("resources/") {
            let plain = !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\']);
            if plain && clarity_spec::resources::is_markdown(name) {
                return Ok(dir.join(clarity_spec::resources::RESOURCES_DIR).join(name));
            }
            return Err(format!("'{path}' is not a Markdown file of the resources: write one like resources/rules-en.md; pictures are added in Clarity"));
        }
        // A window or view: `design/Main.cxaml`, a plain name.
        if let Some(name) = path.strip_prefix(&format!("{}/", clarity_spec::cxaml::DESIGN_DIR)) {
            let plain = !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\']);
            if plain && clarity_spec::cxaml::is_page_file(name) {
                return Ok(clarity_spec::cxaml::design_folder(dir).join(name));
            }
            return Err(format!("'{path}' is not a window or view: write one like design/Main.cxaml"));
        }
        // A written diagram: `diagrams/orders.mermaid`.
        if let Some(name) = path.strip_prefix("diagrams/") {
            return crate::diagrams::diagram_file(dir, name);
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
    pub fn part_folders(&self) -> Result<Vec<crate::parts::PartFolder>, String> {
        let (_, dir) = self.open()?;
        let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read the project: {err}"))?;
        Ok(crate::parts::part_folders(dir, &project))
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
            if pieces.len() > 1 && crate::parts::BUILD_FOLDERS.contains(&pieces[0]) {
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

    /// Puts what the data model changed into the agent's migration step
    /// (crate::steps): the sentence that says so, or nothing.
    fn follow(&mut self, renames: &[clarity_spec::migrate::Rename]) -> Result<String, String> {
        let (file, dir) = self.open()?.clone();
        let followed = crate::steps::follow(&file, &dir, renames)?;
        self.written.extend(followed.written);
        Ok(if followed.said.is_empty() { String::new() } else { format!("{}\n", followed.said) })
    }

    /// Renames an entity, an enum or a field of an entity or record across
    /// the project, through the checked model; the step records the rename,
    /// so a stored table or column keeps its data.
    fn rename(&mut self, input: &Value) -> Result<String, String> {
        use clarity_spec::rename::{RenameKind, rename, rename_field};
        let (_, dir) = self.open()?.clone();
        let text = |key: &str| input[key].as_str().unwrap_or_default().trim().to_string();
        let (kind, from, to) = (text("kind"), text("from"), text("to"));
        let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read the project: {err}"))?;
        if project.diagnostics.iter().any(|d| d.severity == Severity::Error) {
            return Err("fix the errors first: a rename follows the checked model".into());
        }
        let at = |path: &str| if Path::new(path).is_absolute() { PathBuf::from(path) } else { dir.join(path) };
        let source = |path: &str| std::fs::read_to_string(at(path)).ok();
        let (changed, renamed, what) = match kind.as_str() {
            "entity" => (rename(&project, source, RenameKind::Entity, &from, &to)?, Some(clarity_spec::migrate::Rename { entity: None, from: from.clone(), to: to.clone() }), format!("entity {from}")),
            "enum" => (rename(&project, source, RenameKind::Enum, &from, &to)?, None, format!("enum {from}")),
            "field" => {
                let entity = text("entity");
                if entity.is_empty() {
                    return Err("a field is renamed in its entity: give entity, like Customer".into());
                }
                let renamed = Some(clarity_spec::migrate::Rename { entity: Some(entity.clone()), from: from.clone(), to: to.clone() });
                (rename_field(&project, source, &entity, &from, &to)?, renamed, format!("field {entity}.{from}"))
            }
            "window" | "view" => {
                let page = project.pages().find(|p| p.name.name == from).ok_or_else(|| format!("there is no window or view {from}"))?;
                let is_view = page.kind == clarity_spec::model::PageKind::View;
                if (kind == "view") != is_view {
                    let (is, not) = if is_view { ("view", "window") } else { ("window", "view") };
                    return Err(format!("{from} is a {is}, not a {not}: give kind {is}"));
                }
                (rename(&project, source, RenameKind::Page, &from, &to)?, None, format!("{kind} {from}"))
            }
            _ => return Err("kind is entity, enum, field, window or view".into()),
        };
        if changed.is_empty() {
            return Ok(format!("Nothing to rename: {what} is called {to} already."));
        }
        let mut paths = Vec::new();
        for file in changed {
            let path = at(&file.path);
            let before = std::fs::read_to_string(&path).ok();
            write_atomically(&path, &file.text)?;
            paths.push(clarity_project::slashed(path.strip_prefix(&dir).unwrap_or(&path)));
            self.written.push(Written { file: path, before, after: Some(file.text) });
            // A window's file takes its new name: the old one goes, kept for Undo.
            if let Some(was) = file.moved_from {
                let was = at(&was);
                let before = std::fs::read_to_string(&was).ok();
                std::fs::remove_file(&was).map_err(|err| format!("cannot remove {}: {err}", was.display()))?;
                self.written.push(Written { file: was, before, after: None });
            }
        }
        let step = self.follow(&renamed.into_iter().collect::<Vec<_>>())?;
        Ok(format!("Renamed {what} to {to} in {}. {step}{}", listed(&paths.iter().map(String::as_str).collect::<Vec<_>>()), self.problems()?))
    }

    /// What a structural tool tells after its edit (crate::structural): the
    /// edit's files kept for Undo, a change of the entities put into the
    /// agent's migration step, and the files and problems as the agent reads them.
    pub(crate) fn applied(&mut self, what: &str, applied: crate::edits::Applied) -> Result<String, String> {
        let (_, dir) = self.open()?.clone();
        let mut names = Vec::new();
        for change in applied.changes {
            names.push(clarity_project::slashed(change.file.strip_prefix(&dir).unwrap_or(&change.file)));
            self.written.push(Written { file: change.file, before: change.before, after: change.after });
        }
        if names.is_empty() {
            return Ok(format!("{what}: nothing changed. {}", self.problems()?));
        }
        let step = self.follow(&[])?;
        Ok(format!("{what}. Changed {}. {step}{}", listed(&names.iter().map(String::as_str).collect::<Vec<_>>()), self.problems()?))
    }

    /// The project's problems, as `file:line:col: error: message` lines.
    pub fn problems(&self) -> Result<String, String> {
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
        let said = match (errors, warnings) {
            (0, 0) => "No problems.".to_string(),
            (0, n) => format!("No errors; {n} warning(s):\n{}", lines.join("\n")),
            (e, _) => format!("{e} error(s):\n{}", lines.join("\n")),
        };
        // Every check says what is left to translate, so no agent leaves a language showing another's texts.
        Ok(match untranslated(&project) {
            Some(left) => format!("{said}\nNot translated yet (empty, or still the default's text): {left}. Run missing_translations and translate them before you finish; keep a text the same only where the language says it the same way."),
            None => said,
        })
    }

    /// Per language other than the default, the keys still empty or the same
    /// as the default's text: what is left to translate.
    pub fn missing_translations(&self) -> Result<String, String> {
        let (_, dir) = self.open()?;
        let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read the project: {err}"))?;
        let default = clarity_spec::translations::default_language(&project);
        if !project.translations.contains_key(&default) {
            return Ok(format!("The project has no {default} language file."));
        }
        let mut out = Vec::new();
        for (language, left) in clarity_spec::translations::left_to_translate(&project) {
            let left: Vec<String> = left.iter().map(|(key, text)| format!("    {key} {text:?}")).collect();
            if !left.is_empty() {
                out.push(format!("{language} ({} to translate; the {default} text after each key):\n{}", left.len(), left.join("\n")));
            }
        }
        Ok(if out.is_empty() { "Every language has its own text for every key.".into() } else { out.join("\n\n") })
    }

    pub fn overview(&self) -> Result<String, String> {
        let (file, dir) = self.open()?;
        let mut out = match std::fs::read_to_string(dir.join(clarity_spec::README_FILE)) {
            Ok(readme) => format!("README.md (the project's design goal and guide; work toward it, update it when the goal changes):\n{readme}\n\n"),
            Err(_) => "README.md: none yet. The project has no written design goal; offer to write one (path README.md) once the goal is clear.\n\n".to_string(),
        };
        out.push_str(&format!("Project file ({}):\n{}\n\nModules in {}:\n", file.display(), std::fs::read_to_string(file).unwrap_or_default(), dir.display()));
        let mut modules: Vec<PathBuf> = Vec::new();
        collect(dir, dir, &mut modules);
        modules.sort();
        for m in &modules {
            out.push_str(&format!("- {}\n", m.display()));
        }
        let mut pages: Vec<String> = std::fs::read_dir(clarity_spec::cxaml::design_folder(dir))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| clarity_spec::cxaml::is_page_file(n))
            .collect();
        pages.sort();
        if !pages.is_empty() {
            out.push_str("\nWindows and views (design/, one .cxaml file each):\n");
            for p in &pages {
                out.push_str(&format!("- design/{p}\n"));
            }
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
                for entry in crate::parts::tree(&dir.join(&f.folder), f.own.as_deref()).into_iter().filter(|e| !e.folder) {
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
        // The guidance keeps its sections, which the wizard reads back: only its tools write it.
        if path.trim().trim_start_matches("./") == clarity_spec::guidance::FILE {
            return Err(format!("{path} is the app's guidance for agents: write it a section at a time with set_guidance, and read it with guidance"));
        }
        // Windows, views and the app change only through the structural tools (docs/AGENT-TOOLS-PLAN.md).
        if clarity_spec::cxaml::is_page_file(path.trim()) {
            let names: Vec<String> = crate::structural::windows::definitions().into_iter().chain(crate::structural::frame::definitions()).map(|t| t.name).collect();
            return Err(format!(
                "{path} is a window, view or app: change it with the structural tools, which check every change and keep the markup right: {}",
                names.join(", ")
            ));
        }
        let file = self.file_of(path)?;
        let before = std::fs::read_to_string(&file).ok();
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
        }
        let text = if text.ends_with('\n') { text.to_string() } else { format!("{text}\n") };
        write_atomically(&file, &text)?;
        self.written.push(Written { file, before, after: Some(text) });
        let step = self.follow(&[])?;
        Ok(format!("Wrote {path}. {step}{}", self.problems()?))
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
        let mut out = format!("Picked; Clarity wrote apis/{source}.clarity. {}", self.follow(&[])?);
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
    pub fn diagram_text(&self, name: &str) -> Result<(PathBuf, String), String> {
        let (_, dir) = self.open()?;
        let file = crate::diagrams::diagram_file(dir, name)?;
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
        let proposals = crate::diagrams::proposals_of(dir, text)?;
        if proposals.is_empty() {
            out.push_str("The diagram matches the project: it proposes nothing.");
        } else {
            out.push_str("It proposes:\n");
            out.push_str(&crate::diagrams::proposal_lines(&proposals));
        }
        Ok(out)
    }

    /// Writes a diagram into the diagrams folder, and says what it proposes.
    fn write_diagram(&mut self, name: &str, text: &str) -> Result<String, String> {
        let (_, dir) = self.open()?.clone();
        let file = crate::diagrams::diagram_file(&dir, name)?;
        let before = std::fs::read_to_string(&file).ok();
        // A project has one ERD: data changes go into it, and a second is refused, naming it.
        if clarity_spec::diagrams::kind_of(text) == Some(clarity_spec::diagrams::DiagramKind::Erd) {
            let shown = file.file_name().unwrap_or_default().to_string_lossy().to_string();
            let erds: Vec<String> = clarity_spec::diagrams::diagram_files(&dir)
                .unwrap_or_default()
                .into_iter()
                .filter(|d| d.kind == Some(clarity_spec::diagrams::DiagramKind::Erd) && d.name != shown)
                .map(|d| format!("diagrams/{}", d.name))
                .collect();
            if !erds.is_empty() {
                return Err(format!("the project has its ERD already ({}): write the change into it, keeping what it holds", erds.join(", ")));
            }
        }
        if clarity_spec::diagrams::kind_of(text) == Some(clarity_spec::diagrams::DiagramKind::Class)
            && let Some(dotted) = dotted_class(text)
        {
            let own = dotted.rsplit('.').next().unwrap_or_default();
            return Err(format!("{dotted} is no class: name a class by its own name, in relations too ({own}), and put it in its module's namespace (namespace {} {{ class {own} }})", dotted.split('.').next().unwrap_or_default()));
        }
        std::fs::create_dir_all(dir.join(clarity_spec::diagrams::DIAGRAMS_DIR)).map_err(|err| format!("cannot make the diagrams folder: {err}"))?;
        let text = if text.ends_with('\n') { text.to_string() } else { format!("{text}\n") };
        write_atomically(&file, &text)?;
        let shown = file.file_name().unwrap_or_default().to_string_lossy().to_string();
        self.written.push(Written { file, before, after: Some(text.clone()) });
        Ok(format!("Wrote diagrams/{shown}. {}", self.proposals_text(&text)?))
    }

    /// Applies the proposals of a written diagram at `numbers` (from 1), as Apply does.
    fn apply_diagram(&mut self, name: &str, numbers: &[usize]) -> Result<String, String> {
        let (_, dir) = self.open()?.clone();
        let (_, text) = self.diagram_text(name)?;
        if numbers.is_empty() {
            return Err("name the proposals to apply by their numbers, as diagram_proposals lists them".into());
        }
        let count = crate::diagrams::proposals_of(&dir, &text)?.len();
        if let Some(wrong) = numbers.iter().find(|n| **n == 0 || **n > count) {
            return Err(format!("there is no proposal {wrong}: the diagram has {count}"));
        }
        let ticked: Vec<usize> = numbers.iter().map(|n| n - 1).collect();
        let changes = crate::diagrams::apply_text(&dir, &text, &ticked)?;
        // Apply writes files; it takes none away.
        for change in changes {
            if let Some(after) = change.after {
                self.written.push(Written { file: change.file, before: change.before, after: Some(after) });
            }
        }
        let step = self.follow(&[])?;
        Ok(format!("Applied {} proposals. {step}{}", numbers.len(), self.problems()?))
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
        // Never a guessed rename: the agent renames with the rename tool, whose step records it.
        let ops = clarity_spec::migrate::pending(&project, &source, &[]);
        if ops.is_empty() {
            return Ok(format!("Nothing to migrate in {source}: every change is in a step already."));
        }
        let title = input["title"].as_str().filter(|t| !t.trim().is_empty()).unwrap_or(crate::steps::AGENT_STEP);
        let path = clarity_spec::write_step(&dir, &source, title.trim(), &ops).map_err(|err| format!("cannot write the step: {err}"))?;
        let text = std::fs::read_to_string(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
        self.written.push(Written { file: path.clone(), before: None, after: Some(text.clone()) });
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
        let file = crate::new_project::write_new_project(&folder.join(name), stack)?;
        let dir = clarity_project::module_dir(&file);
        self.project = Some((file.clone(), dir));
        self.made = Some(file.clone());
        // Only what was made: whether and when it opens is the host's to say.
        Ok(format!("Made the project {}.\n\n{}", file.display(), self.overview()?))
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
        let tools = vec![
            tool("project_overview", "The open project: its project file, its modules and its problems.", schema(json!({}), &[])),
            tool(
                "read_file",
                "Reads a module (a path like todo.clarity, relative to the project's folder), the project file (`project`) or a Markdown file of the resources (resources/rules-en.md).",
                schema(json!({ "path": { "type": "string" } }), &["path"]),
            ),
            tool(
                "write_file",
                "Writes a whole module (a path like todo.clarity), the project file (`project`) or a Markdown file of the resources (resources/rules-en.md), new or replacing it. Windows, views and the app (.cxaml) are changed with the structural tools, never written. Gives the project's problems after the write.",
                schema(json!({ "path": { "type": "string" }, "text": { "type": "string", "description": "The whole file." } }), &["path", "text"]),
            ),
            tool("check", "Checks the whole project and gives its problems.", schema(json!({}), &[])),
            tool(
                "missing_translations",
                "Lists, per language, the keys whose text is still empty or the same as the default language's, with the default text: what to translate. Write the translations into translations/<language>.clarity with write_file.",
                schema(json!({}), &[]),
            ),
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
            tool(
                "rename",
                "Renames an entity, an enum, a field of an entity or record, a window or a view, everywhere the project names it: modules, windows, views and translations; a window's or view's file takes the new name. The migration step records a data rename, so a stored table or column keeps its data. Rename with this tool, never by writing the new name over the old one.",
                schema(
                    json!({
                        "kind": { "type": "string", "enum": ["entity", "enum", "field", "window", "view"], "description": "What is renamed; window covers dialogs and splash pages too." },
                        "entity": { "type": "string", "description": "For a field: the entity or record it belongs to, like Customer." },
                        "from": { "type": "string", "description": "The name now." },
                        "to": { "type": "string", "description": "The new name." }
                    }),
                    &["kind", "from", "to"],
                ),
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
                        "stack": { "type": "object", "description": "Optional: { backend: rust|go|csharp|c|cpp, ui: webview|clarity, targets: [desktop, cli] }." }
                    }),
                    &["name"],
                ),
            ),
            tool(
                "generate",
                "Generates the application's source tree for the project's stack into its output folder, as `clarity gen` does: each program in its folder (desktop, cli, web). It writes code and builds nothing; the person builds and runs. The project must check without errors. Read the result with read_generated.",
                schema(json!({}), &[]),
            ),
            tool(
                "read_generated",
                "Reads the application Clarity generated from the project (in the folder the project's output setting names), read only. Each program has its folder there: desktop for the windowed application, cli for the command-line program. Without a path it lists the generated files; with one (like desktop/Modules/Todo.cs) it gives that file. Use it with build_output to see why a build failed.",
                schema(json!({ "path": { "type": "string", "description": "A path inside the generated project, as the list gives it." } }), &[]),
            ),
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
        ];
        tools.into_iter().chain(crate::structural::definitions()).collect()
    }

    fn call(&mut self, name: &str, input: &Value) -> ToolResult {
        let text = |key: &str| input[key].as_str().unwrap_or_default().to_string();
        let result = match name {
            "project_overview" => self.overview(),
            "read_file" => self.read(&text("path")),
            "write_file" => self.write(&text("path"), &text("text")),
            "check" => self.problems(),
            "missing_translations" => self.missing_translations(),
            "api" => self.api(&text("source")),
            "pick_api" => self.pick_api(input),
            "parts" => serde_json::to_string(&clarity_spec::widgets::catalog()).map_err(|err| err.to_string()),
            "design" => section(DESIGN, "design document", &text("section")),
            "manual" => manual_topic(&text("topic")),
            "reference" => section(LANGUAGE, "language reference", &text("section")),
            "new_project" => self.new_project(input),
            "migrate" => self.migrate(input),
            "rename" => self.rename(input),
            "read_generated" => self.generated(&text("path")),
            "generate" => self.open().and_then(|(file, _)| crate::generate::generate(file, None).map(|lines| lines.join("\n")).map_err(|why| why.join("\n"))),
            "diagram_view" => self.diagram_view(),
            "write_diagram" => self.write_diagram(&text("name"), &text("text")),
            "diagram_proposals" => self.diagram_text(&text("name")).and_then(|(_, t)| self.proposals_text(&t)),
            "apply_diagram" => {
                let numbers: Vec<usize> = input["proposals"].as_array().map(|a| a.iter().filter_map(|n| n.as_u64()).map(|n| n as usize).collect()).unwrap_or_default();
                self.apply_diagram(&text("name"), &numbers)
            }
            other => match crate::structural::call(self, other, input) {
                Some(result) => result,
                None => Err(format!("there is no tool {other}")),
            },
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A project in a scratch folder: the shop example's project file and modules.
    fn project(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("clarity-tools-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
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
        assert!(module.contains("action listTemplates() -> list<Template>") && module.contains("every page"), "{module}");
        assert!(picked.text.contains("No problems."), "{}", picked.text);
        assert!(tools.call("api", &json!({"source": "forms"})).text.contains("[x] listTemplates"));
        // One change of the project file, for Undo.
        assert_eq!(tools.written.iter().filter(|w| w.file == file).count(), 1);

        let wrong = tools.call("pick_api", &json!({"source": "forms", "operations": ["listForms"]}));
        assert!(wrong.is_error && wrong.text.contains("forms.yaml has no operation listForms; it has listTemplates and createTemplate"), "{}", wrong.text);
        let _ = std::fs::remove_dir_all(root);
    }

    /// A window or view is renamed by name: its file takes the new name, and
    /// every place that names it follows.
    #[test]
    fn a_window_or_view_is_renamed_with_its_file() {
        let (root, file, dir) = project("rename-page");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let wrong = tools.call("rename", &json!({"kind": "view", "from": "Main", "to": "Home"}));
        assert!(wrong.is_error && wrong.text.contains("Main is a window, not a view"), "{}", wrong.text);
        let renamed = tools.call("rename", &json!({"kind": "window", "from": "Main", "to": "Home"}));
        assert!(!renamed.is_error && renamed.text.contains("Renamed window Main to Home"), "{}", renamed.text);
        assert!(!dir.join("design/Main.cxaml").exists() && dir.join("design/Home.cxaml").exists());
        assert!(std::fs::read_to_string(dir.join("design/Todo.cxaml")).unwrap().contains("main=\"Home\""));
        // Undo takes the old file back: the move is a removal and a new file.
        assert!(tools.written.iter().any(|w| w.file == dir.join("design/Main.cxaml") && w.after.is_none()));
        let view = tools.call("rename", &json!({"kind": "view", "from": "MainView", "to": "TaskView"}));
        assert!(!view.is_error && view.text.contains("No problems"), "{}", view.text);
        assert!(std::fs::read_to_string(dir.join("design/Home.cxaml")).unwrap().contains("TaskView"));
        let _ = std::fs::remove_dir_all(root);
    }

    /// The agent's model changes go into its migration step by themselves
    /// (docs/MCP-PLAN.md): folded while no database has applied the step, a
    /// new step after; a rename only as the rename tool asked, never guessed.
    #[test]
    fn model_changes_go_into_the_agent_s_steps() {
        let (root, file, dir) = project("steps");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let steps = || -> Vec<String> {
            let mut names: Vec<String> = std::fs::read_dir(dir.join("migrations/main")).map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
            names.sort();
            names
        };
        let latest = || std::fs::read_to_string(dir.join("migrations/main").join(steps().last().unwrap())).unwrap();
        let before = steps().len();
        let en = tools.call("read_file", &json!({"path": "translations/en.clarity"})).text;
        let keys = format!("{}\n    taskTitle \"Title\"\n    taskDone \"Done\"\n    fieldRequired \"{{field}} is required\"\n", en.trim_end());
        assert!(!tools.call("write_file", &json!({"path": "translations/en.clarity", "text": keys})).is_error);

        // A new entity: a new step.
        let task = "module tasks\n\nentity Task\n    title: text(120), required, label @taskTitle\n";
        let wrote = tools.call("write_file", &json!({"path": "tasks.clarity", "text": task}));
        assert!(wrote.text.contains("Wrote the step") && wrote.text.contains("No errors"), "{}", wrote.text);
        assert_eq!(steps().len(), before + 1);
        assert!(latest().contains("add entity Task") && latest().contains(crate::steps::AGENT_STEP), "{}", latest());

        // No database has applied it: a rename folds in, and the step adds the entity as it is now.
        let renamed = tools.call("rename", &json!({"kind": "field", "entity": "Task", "from": "title", "to": "name"}));
        assert!(!renamed.is_error && renamed.text.contains("Folded into the step"), "{}", renamed.text);
        assert_eq!(steps().len(), before + 1, "still one step");
        assert!(latest().contains("name") && !latest().contains("title:") && !latest().contains("rename"), "{}", latest());
        assert!(std::fs::read_to_string(dir.join("tasks.clarity")).unwrap().contains("    name: text(120)"));

        // A database applied every step: the next rename is a step of its own, which keeps the column's data.
        let settings = clarity_project::load(&file).settings.unwrap();
        let db = clarity_project::sqlite_path(&dir, &settings.datasources[0]);
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        rusqlite::Connection::open(&db).unwrap().execute_batch(&format!("CREATE TABLE clarity_schema (version INTEGER NOT NULL); INSERT INTO clarity_schema VALUES ({});", steps().len())).unwrap();
        let again = tools.call("rename", &json!({"kind": "field", "entity": "Task", "from": "name", "to": "label"}));
        assert!(!again.is_error && again.text.contains("Wrote the step"), "{}", again.text);
        assert_eq!(steps().len(), before + 2);
        assert!(latest().contains("rename field Task.name to label"), "{}", latest());

        // A name written over is not guessed to be a rename: a drop and an add.
        rusqlite::Connection::open(&db).unwrap().execute_batch(&format!("INSERT INTO clarity_schema VALUES ({});", steps().len())).unwrap();
        let task = "module tasks\n\nentity Task\n    caption: text(120), required, label @taskTitle\n";
        let over = tools.call("write_file", &json!({"path": "tasks.clarity", "text": task}));
        assert!(over.text.contains("Wrote the step"), "{}", over.text);
        assert!(!latest().contains("rename") && latest().contains("caption") && latest().contains("label"), "{}", latest());

        // A project with errors gets no step until it checks clean.
        let count = steps().len();
        let broken = tools.call("write_file", &json!({"path": "tasks.clarity", "text": "module tasks\n\nentity Task\n    caption: nonsense\n    done: bool, label @taskDone\n"}));
        assert!(broken.text.contains("error(s)") && !broken.text.contains("step"), "{}", broken.text);
        assert_eq!(steps().len(), count);
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
        // A label is a key, whose text the default language's file gives; a
        // required field also needs Clarity's word for saying so.
        let en = tools.call("read_file", &json!({"path": "translations/en.clarity"}));
        assert!(en.text.starts_with("translation en"), "{}", en.text);
        let with_key = format!("{}\n    taskTitle \"Title\"\n    fieldRequired \"{{field}} is required\"\n", en.text.trim_end());
        assert!(!tools.call("write_file", &json!({"path": "translations/en.clarity", "text": with_key})).is_error);
        let fixed = tools.call("write_file", &json!({"path": "tasks.clarity", "text": "module tasks\n\nentity Task\n    title: text(120), required, label @taskTitle"}));
        // The new entity goes into the agent's migration step at once, so nothing warns that it is in none.
        assert!(fixed.text.contains("Wrote the step migrations/main/") && fixed.text.contains("add entity Task") && fixed.text.ends_with("No problems."), "{}", fixed.text);
        let step = tools.call("migrate", &json!({"title": "Tasks"}));
        assert!(!step.is_error && step.text.starts_with("Nothing to migrate"), "{}", step.text);
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
        assert!(module.after.as_deref().is_some_and(|a| a.contains("text(120)")));
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
        assert!(module.contains("entity Invoice\n    issuedOn: date, required, label @invoiceIssuedOnLabel\n    customer: ref Customer"), "{module}");
        let en = std::fs::read_to_string(dir.join("translations/en.clarity")).unwrap();
        assert!(en.contains("invoiceIssuedOnLabel \"Issued on\""), "{en}");
        assert!(tools.call("diagram_proposals", &json!({"name": "billing"})).text.contains("matches the project"));

        // The diagram, the module and the language file are one change for Undo.
        let one = merged(&tools.written);
        assert!(one.iter().any(|c| c.file.ends_with("diagrams/billing.mermaid")));
        assert!(one.iter().any(|c| c.file.ends_with("todo.clarity")));
        assert!(one.iter().any(|c| c.file.ends_with("translations/en.clarity")));

        for name in ["../escape", "a/b", ""] {
            assert!(tools.call("write_diagram", &json!({"name": name, "text": sketch})).is_error, "{name}");
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A project has one ERD: the agent writes data changes into it, and
    /// makes it only when there is none. A second ERD is refused, naming the
    /// one there is; other kinds of diagram are not limited.
    #[test]
    fn a_project_has_one_erd() {
        let (root, file, dir) = project("one-erd");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let sketch = "erDiagram\n    Customer {\n        text(80) name \"required\"\n    }\n";
        assert!(!tools.call("write_diagram", &json!({"name": "data", "text": sketch})).is_error);
        let changed = format!("{sketch}    Invoice {{\n        date issued_on\n    }}\n");
        assert!(!tools.call("write_diagram", &json!({"name": "data", "text": changed})).is_error, "the ERD itself changes");
        let second = tools.call("write_diagram", &json!({"name": "invoices", "text": sketch}));
        assert!(second.is_error && second.text.contains("diagrams/data.mermaid"), "{}", second.text);
        assert!(!dir.join("diagrams/invoices.mermaid").exists());
        let class = "classDiagram\n    class Customer\n";
        assert!(!tools.call("write_diagram", &json!({"name": "classes", "text": class})).is_error, "another kind is free");
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A class is named by its own name in a class diagram, relations too:
    /// `sales.Order` is no class, and drew an empty second box beside the
    /// real one, so it is refused, naming how to write it.
    #[test]
    fn a_class_diagram_names_classes_by_their_own_names() {
        let (root, file, dir) = project("class-names");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let dotted = "classDiagram\n    namespace todo {\n        class Task {\n            +title: text(80)\n        }\n    }\n    class todo.Task\n";
        let refused = tools.call("write_diagram", &json!({"name": "classes", "text": dotted}));
        assert!(refused.is_error && refused.text.contains("todo.Task") && refused.text.contains("Task"), "{}", refused.text);
        assert!(!dir.join("diagrams/classes.mermaid").exists());
        let relation = "classDiagram\n    class Task\n    class List\n    todo.Task \"*\" --> \"1\" List : list\n";
        assert!(tools.call("write_diagram", &json!({"name": "classes", "text": relation})).is_error, "in a relation too");
        let plain = "classDiagram\n    namespace todo {\n        class Task {\n            +title: text(80)\n        }\n    }\n";
        assert!(!tools.call("write_diagram", &json!({"name": "classes", "text": plain})).is_error);
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// What is left to translate, per language: empty, or the same as the default's text.
    #[test]
    fn missing_translations_lists_what_is_left_to_translate() {
        let (root, file, dir) = project("missing");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let en = std::fs::read_to_string(dir.join("translations/en.clarity")).unwrap();
        std::fs::write(dir.join("translations/nl.clarity"), en.replacen("translation en", "translation nl", 1)).unwrap();
        let left = tools.call("missing_translations", &json!({}));
        assert!(!left.is_error && left.text.starts_with("nl ("), "{}", left.text);
        // Every check says so, until it is done.
        assert!(tools.call("check", &json!({})).text.contains("still the default's text): nl "), "the check is silent about it");
        std::fs::remove_file(dir.join("translations/nl.clarity")).unwrap();
        assert_eq!(tools.call("missing_translations", &json!({})).text, "Every language has its own text for every key.");
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// What the agent says follows Clarity's house style: no dashes for
    /// asides, plain sentences. Its instructions say so, and are written so.
    #[test]
    fn the_instructions_follow_the_house_style() {
        let text = instructions(&[]);
        for dash in ['\u{2014}', '\u{2013}'] {
            assert!(!text.contains(dash), "the instructions hold {dash:?}");
        }
        assert!(text.contains("em-dash") && text.contains("en-dash"), "the instructions name the rule");
    }

    #[test]
    fn a_new_project_becomes_the_open_one() {
        let root = std::env::temp_dir().join(format!("clarity-tools-new-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut tools = ProjectTools::new(None, root.clone());
        assert!(tools.call("check", &json!({})).text.contains("no project is open"));
        let made = tools.call("new_project", &json!({"name": "todo"}));
        assert!(!made.is_error, "{}", made.text);
        assert_eq!(tools.made, Some(root.join("todo.clarity")));
        assert!(made.text.contains("No problems."), "{}", made.text);
        assert!(!tools.call("read_file", &json!({"path": "todo.clarity"})).is_error);
        std::fs::remove_dir_all(&root).unwrap();
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
    fn the_agent_is_told_of_parts_libraries_and_sound() {
        let told = instructions(&[]);
        assert!(told.contains("custom part") && told.contains("library") && told.contains("sound"), "{told}");
        // Music plays through the sound part, not a library the agent writes.
        assert!(told.contains("music.play(path)") && told.contains("on end") && told.contains("needs no library of its own"), "{told}");
        for topic in ["design.custom-parts", "code.libraries", "design.sound"] {
            assert!(manual_topic(topic).is_ok(), "{topic}");
        }
    }

    /// The project's README.md is its design goal: the overview shows it
    /// first, and the agent reads and writes it.
    #[test]
    fn the_agent_reads_the_readme_first_and_keeps_it() {
        let (root, file, dir) = project("readme");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        assert!(tools.call("project_overview", &json!({})).text.starts_with("README.md: none yet"));
        let wrote = tools.call("write_file", &json!({"path": "README.md", "text": "# Goal\n\nTrack tasks."}));
        assert!(!wrote.is_error, "{}", wrote.text);
        assert_eq!(std::fs::read_to_string(dir.join("README.md")).unwrap(), "# Goal\n\nTrack tasks.\n");
        let overview = tools.call("project_overview", &json!({})).text;
        assert!(overview.starts_with("README.md (the project's design goal"), "{overview}");
        assert!(overview.contains("Track tasks."), "{overview}");
        assert!(instructions(&[]).contains("README.md"), "the instructions say nothing of the README");
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
        let told = instructions(&[]);
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
        let told = instructions(&[]);
        // Every topic of the manual, by id with its summary, to choose from.
        for topic in crate::manual::topics() {
            assert!(told.contains(&format!("- {}: ", topic.id)), "{} is not offered", topic.id);
        }
        // The reference's sections by title, not the reference itself.
        assert!(told.contains("Queries") && told.contains("## 13. Diagrams") && !told.contains("A class name is written once"));
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

    /// The shared tools design and generate only: they read, write project
    /// files and check them. None builds, runs, backs up or restores; the
    /// IDE adds those itself, and the MCP server never has them. A tool that
    /// runs something needs this list changed on purpose.
    #[test]
    fn the_shared_tools_design_and_generate_only() {
        let tools = ProjectTools::new(None, std::env::temp_dir());
        let mut names: Vec<String> = tools.definitions().into_iter().map(|t| t.name).collect();
        names.sort();
        let mut allowed: Vec<String> = [
            // Reading.
            "api", "check", "design", "diagram_proposals", "diagram_view", "manual", "missing_translations", "parts", "project_overview", "read_file", "read_generated",
            "reference",
            // Writing project files, checked at once and undone with one Undo.
            "apply_diagram", "migrate", "new_project", "pick_api", "rename", "write_diagram", "write_file",
            // Writing the generated source tree for the stack, as `clarity gen` does; it builds nothing.
            "generate",
        ]
        .iter()
        .map(|n| n.to_string())
        // The structural tools: each makes one checked designer edit (crate::edits::apply) and runs nothing.
        .chain(crate::structural::definitions().into_iter().map(|t| t.name))
        .collect();
        allowed.sort();
        assert_eq!(names, allowed);
    }

    /// Windows and views are `.cxaml` files in the design folder:
    /// the agent reads them by their paths and the overview lists them, but
    /// changes them only with the structural tools (docs/AGENT-TOOLS-PLAN.md,
    /// "add and change only through tools"): writing one is refused, naming the tools.
    #[test]
    fn the_agent_reads_windows_and_views_and_changes_them_with_tools() {
        let (root, file, dir) = project("cxaml");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let overview = tools.call("project_overview", &json!({})).text;
        assert!(overview.contains("- design/Main.cxaml"), "{overview}");
        let main = tools.call("read_file", &json!({"path": "design/Main.cxaml"}));
        assert!(!main.is_error && main.text.contains("<Window class=\"todo.Main\""), "{}", main.text);

        let view = "<View class=\"todo.Tasks\">\n</View>\n";
        let refused = tools.call("write_file", &json!({"path": "design/Tasks.cxaml", "text": view}));
        assert!(refused.is_error && refused.text.contains("add_view") && refused.text.contains("change_part"), "{}", refused.text);
        assert!(!dir.join("design/Tasks.cxaml").exists(), "nothing written");
        let main_before = std::fs::read_to_string(dir.join("design/Main.cxaml")).unwrap();
        assert!(tools.call("write_file", &json!({"path": "design/Main.cxaml", "text": view})).is_error);
        assert_eq!(std::fs::read_to_string(dir.join("design/Main.cxaml")).unwrap(), main_before);
        let told = instructions(&[]);
        assert!(told.contains("design/Main.cxaml") && told.contains("<Window class="), "the instructions say nothing of .cxaml");
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// The agent generates the project's source tree for its stack, as
    /// `clarity gen` does: generating writes code, it builds and runs nothing.
    #[test]
    fn the_agent_generates_the_project_and_reads_what_it_wrote() {
        let (root, file, dir) = project("generate");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let made = tools.call("generate", &json!({}));
        assert!(!made.is_error, "{}", made.text);
        assert!(made.text.contains("file(s) in") && made.text.contains("desktop"), "{}", made.text);
        let listed = tools.call("read_generated", &json!({}));
        assert!(!listed.is_error && listed.text.contains("- desktop/"), "{}", listed.text);
        // The generated tree is no project file: Undo leaves it alone.
        assert!(tools.written.is_empty());

        // A project with errors is not generated; the agent fixes them first.
        assert!(!tools.call("write_file", &json!({"path": "broken.clarity", "text": "module broken\n\nentity Task\n    title: nonsense\n"})).is_error);
        let refused = tools.call("generate", &json!({}));
        assert!(refused.is_error && refused.text.contains("broken.clarity"), "{}", refused.text);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
