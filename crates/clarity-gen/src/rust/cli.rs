//! The Rust command-line program (DESIGN 5.20): a crate of its own in
//! `out/cli`, on the runtime crate without Tauri, with the project's
//! records, actions and commands, and a main that reads the command line
//! and runs the command it names.

use clarity_project::Settings;
use clarity_spec::code::check;
use clarity_spec::model::{FieldType, Project};

use super::names::{ident, snake};
use super::{ALLOW, HEADER, logic, model, text};
use crate::OutFile;
use crate::commands::{CommandSpec, ParamSpec, Said};

fn said(text: &Option<Said>) -> String {
    match text {
        Some(Said::Key(key)) => format!("Some(Text::key({key:?}).show())"),
        Some(Said::Plain(text)) => format!("Some({text:?}.to_string())"),
        None => "None".to_string(),
    }
}

fn kind(word: &str) -> &'static str {
    match word {
        "integer" => "Integer",
        "decimal" => "Decimal",
        "money" => "Money",
        "date" => "Date",
        "datetime" => "Datetime",
        "duration" => "Duration",
        "enum" => "Enum",
        "bool" => "Bool",
        _ => "Text",
    }
}

fn some(text: &Option<String>) -> String {
    text.as_ref().map_or("None".to_string(), |t| format!("Some({t:?}.to_string())"))
}

fn param(p: &ParamSpec) -> String {
    let values: Vec<String> = p.values.iter().map(|v| format!("{v:?}.to_string()")).collect();
    format!(
        "Param {{ name: {:?}.to_string(), kind: Kind::{}, values: vec![{}], default: {}, min: {}, max: {}, required: {}, help: {} }}",
        p.name,
        kind(p.kind),
        values.join(", "),
        some(&p.default),
        some(&p.min),
        some(&p.max),
        p.required,
        said(&p.help)
    )
}

fn command(c: &CommandSpec) -> String {
    let line: Vec<String> = c.line.iter().map(|w| format!("{w:?}.into()")).collect();
    let arguments: Vec<String> = c.arguments.iter().map(param).collect();
    let options: Vec<String> = c.options.iter().map(param).collect();
    format!(
        "        Command {{\n            line: vec![{}],\n            help: {},\n            arguments: vec![{}],\n            options: vec![{}],\n        }},\n",
        line.join(", "),
        said(&c.help),
        arguments.join(", "),
        options.join(", ")
    )
}

pub fn generate(project: &Project, settings: &Settings) -> Result<Vec<OutFile>, Vec<String>> {
    let name = crate::commands::program_name(settings);
    let (databases, owned) = crate::databases(project, settings)?;
    let mut features: Vec<&str> = databases.iter().map(|(_, sql, _)| sql.feature()).collect();
    if !crate::engines(project).is_empty() {
        features.push("engines");
    }
    // HTTP calls from code (DESIGN 5.23): the runtime's client.
    if project.calls_out() {
        features.push("http");
    }
    features.sort();
    features.dedup();
    let types = check::types(&project.files);
    let mut errors = Vec::new();
    let features = features.iter().map(|f| format!("{f:?}")).collect::<Vec<_>>().join(", ");
    let mut files = vec![
        text(
            "Cargo.toml",
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nclarity-runtime = {{ version = \"{}\", default-features = false, features = [{features}] }}\nserde = {{ version = \"1\", features = [\"derive\"] }}\nserde_json = \"1\"\n\n# A project of its own, not part of a workspace around it.\n[workspace]\n",
                super::runtime_version()
            ),
        ),
        text("src/texts.json", crate::commands::texts_json(project)),
    ];
    let mut modules = Vec::new();
    for file in &project.files {
        let Some(module) = &file.module else { continue };
        let module_name = snake(&module.name);
        let file_types = types.get(&file.path).cloned().unwrap_or_default();
        let mut code = format!("{HEADER}\nuse clarity_runtime::cli::Session;\nuse clarity_runtime::sql;\nuse clarity_runtime::ui::*;\nuse clarity_runtime::*;\n");
        for used in &file.uses {
            code.push_str(&format!("use super::{}::*;\n", snake(&used.name)));
        }
        code.push('\n');
        code.push_str(&model::emit(project, file));
        match logic::emit(project, file, &file_types) {
            Ok(functions) => code.push_str(&functions),
            Err(mut problems) => errors.append(&mut problems),
        }
        match logic::emit_commands(project, file, &file_types) {
            Ok(commands) => code.push_str(&commands),
            Err(mut problems) => errors.append(&mut problems),
        }
        files.push(text(&format!("src/modules/{module_name}.rs"), code.trim_end().to_string() + "\n"));
        modules.push(module_name);
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let declared: String = modules.iter().map(|m| format!("pub mod {m};\n")).collect();
    files.push(text("src/modules.rs", format!("{HEADER}\n{declared}")));
    files.push(text("src/schema.rs", super::schema_rs(project, &owned)?));
    files.extend(crate::engine_files(project, "engines"));
    if !crate::engines(project).is_empty() {
        files.push(text("src/engines.rs", super::engines_rs(project)));
    }
    files.push(text("src/main.rs", main_rs(project, settings, &databases)));
    files.push(text("README.md", readme(settings, &name)));
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn main_rs(project: &Project, settings: &Settings, databases: &[(String, crate::ddl::Sql, String)]) -> String {
    let commands = crate::commands::commands(project);
    let databases = databases.iter().map(|(name, sql, at)| format!("({name:?}, {}, {at:?})", sql.runtime())).collect::<Vec<_>>().join(", ");
    let stores: String = settings.filestores.iter().map(|s| format!("({:?}, {:?})", s.name, s.location)).collect::<Vec<_>>().join(", ");
    let languages: Vec<String> = project.translations.keys().map(|l| format!("{l:?}.to_string()")).collect();
    let base = project.locale.language.clone().unwrap_or_else(|| "en".to_string());
    let listed: String = commands.iter().map(command).collect();
    // The settings, which a command reads and sets as a window does.
    let kept = project.files.iter().find_map(|f| f.entities.iter().find(|e| e.settings).map(|e| (f, e)));
    let (data, start, keep, language) = match kept {
        Some((file, entity)) => {
            let module = file.module.as_ref().map(|m| snake(&m.name)).unwrap_or_default();
            let language = entity
                .fields
                .iter()
                .find(|f| f.ty == FieldType::Language)
                .map(|f| format!("    if !data.settings.{field}.is_empty() {{\n        clarity_runtime::i18n::set_language(Some(&data.settings.{field}));\n    }}\n", field = ident(&f.name.name)))
                .unwrap_or_default();
            (
                format!("/// What every command has as data of its own: the settings (DESIGN 5.17).\npub struct CommandData {{\n    pub settings: modules::{module}::Settings,\n}}\n\n/// The settings kept, each one that still fits; the others have their default.\nfn kept_settings() -> modules::{module}::Settings {{\n    let kept = clarity_runtime::settings::read(PROGRAM);\n    let mut values = serde_json::to_value(modules::{module}::Settings::default()).unwrap_or_default();\n    for (name, value) in kept {{\n        let mut tried = values.clone();\n        tried[&name] = value;\n        if serde_json::from_value::<modules::{module}::Settings>(tried.clone()).is_ok() {{\n            values = tried;\n        }}\n    }}\n    serde_json::from_value(values).unwrap_or_default()\n}}\n"),
                "    let mut data = CommandData { settings: kept_settings() };\n    let before = serde_json::to_value(&data.settings).unwrap_or_default();\n".to_string(),
                "    // A command that changed a setting keeps it, as a window does.\n    let after = serde_json::to_value(&data.settings).unwrap_or_default();\n    if after != before && let serde_json::Value::Object(mut values) = after {\n        values.remove(\"id\");\n        if let Err(err) = clarity_runtime::settings::keep(PROGRAM, &values) {\n            eprintln!(\"{err}\");\n        }\n    }\n".to_string(),
                language,
            )
        }
        None => ("/// What every command has as data of its own.\npub struct CommandData {}\n".to_string(), "    let mut data = CommandData {};\n".to_string(), String::new(), String::new()),
    };
    let mut dispatch = String::new();
    for (i, c) in commands.iter().enumerate() {
        let values: Vec<String> = c
            .arguments
            .iter()
            .chain(&c.options)
            .enumerate()
            .map(|(at, p)| format!("typed::<{}>(Kind::{}, values[{at}].as_deref())", super::names::value_type(&p.ty), kind(p.kind)))
            .collect();
        let args: String = values.iter().map(|v| format!(", {v}")).collect();
        dispatch.push_str(&format!("        {i} => modules::{}::{}(&db, cli, &mut data{args}),\n", snake(&c.module), logic::command_function(c.decl)));
    }
    let engines_mod = if crate::engines(project).is_empty() { "" } else { "mod engines;\n" };
    format!(
        r#"{HEADER}{ALLOW}
mod schema;
mod modules;
{engines_mod}
use clarity_runtime::*;
use clarity_runtime::cli::{{Command, Kind, Param, Program, typed}};
use clarity_runtime::ui::Text;

/// The texts the program shows, in every language the project has.
const TEXTS: &str = include_str!("texts.json");
/// The name the desktop program has, which names the folder the person's
/// settings are kept in: both programs share them.
const PROGRAM: &str = {program:?};
/// The database of each data source: its kind, and its file or connection
/// (`env:NAME` reads it from that environment variable when the program starts).
const DATABASES: &[(&str, Dialect, &str)] = &[{databases}];
/// The file store of each name (`env:NAME` reads it from that environment variable).
const STORES: &[(&str, &str)] = &[{stores}];

{data}
/// The program and its commands, as the command line reads them.
fn program() -> Program {{
    Program {{
        name: {name:?}.to_string(),
        help: {help},
        commands: vec![
{listed}        ],
    }}
}}

/// The language texts show in: the system's, when the project has it.
fn pick_language() {{
    let locale = ["LC_ALL", "LC_MESSAGES", "LANG"].iter().find_map(|v| std::env::var(v).ok().filter(|l| !l.is_empty())).unwrap_or_default();
    let available: Vec<String> = vec![{languages}];
    clarity_runtime::i18n::set_language(clarity_runtime::cli::pick_language(&locale, &available, {base:?}).as_deref());
}}

fn main() -> std::process::ExitCode {{
    clarity_runtime::i18n::load(TEXTS);
    pick_language();
{start}{language}    let mut db = match Db::connect(DATABASES) {{
        Ok(db) => db,
        Err(err) => {{
            eprintln!("{{err}}");
            return std::process::ExitCode::from(1);
        }}
    }};
    for (source, steps) in schema::SOURCES {{
        if let Err(err) = db.migrate_source(source, steps) {{
            eprintln!("{{err}}");
            return std::process::ExitCode::from(1);
        }}
    }}
    for (name, at) in STORES {{
        if let Err(err) = db.add_store(name, at) {{
            eprintln!("{{err}}");
            return std::process::ExitCode::from(1);
        }}
    }}
    let code = clarity_runtime::cli::run_program(&program(), |cli, command, values| match command {{
{dispatch}        _ => Ok(2),
    }});
{keep}    code
}}
"#,
        program = super::crate_name(settings),
        name = crate::commands::program_name(settings),
        help = said(&crate::commands::program_help(project)),
        languages = languages.join(", "),
    )
}

fn readme(settings: &Settings, name: &str) -> String {
    format!(
        "# {title}: the command line\n\nGenerated by Clarity from the project {project}: the command-line program `{name}`, in Rust. Do not edit these files; change the project in Clarity and generate it again.\n\n## Build and run\n\nIn this folder, build the program with\n\n    cargo build --release\n\nwhich writes `target/release/{name}`. Run `{name} --help` to see its commands.\n",
        title = clarity_project::title(settings),
        project = settings.name,
    )
}
