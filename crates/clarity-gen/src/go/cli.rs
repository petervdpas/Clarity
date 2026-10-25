//! The Go command-line program (DESIGN 5.20): a module of its own in
//! `out/cli`, on the runtime module without Wails, with the project's
//! records, actions and commands, and a main that reads the command line
//! and runs the command it names.

use clarity_project::Settings;
use clarity_spec::code::check::{self, TypeInfo};
use clarity_spec::model::{FieldType, Project};

use super::names::{pascal, string};
use super::{ENGINE_MODULES, RUNTIME_MODULE, SERVERS, Server, go_file, go_mod, go_sum, logic, model, text};
use crate::OutFile;
use crate::commands::{CommandSpec, ParamSpec, Said};

fn said(text: &Option<Said>) -> String {
    match text {
        Some(Said::Key(key)) => format!("clarity.CliHelp(clarity.KeyText({}))", string(key)),
        Some(Said::Plain(text)) => format!("clarity.CliHelp(clarity.PlainText({}))", string(text)),
        None => "nil".to_string(),
    }
}

fn some(text: &Option<String>) -> String {
    text.as_ref().map_or("nil".to_string(), |t| format!("clarity.CliDefault({})", string(t)))
}

fn param(p: &ParamSpec) -> String {
    let mut fields = vec![format!("Name: {}", string(&p.name)), format!("Kind: {}", string(p.kind))];
    if !p.values.is_empty() {
        fields.push(format!("Values: []string{{{}}}", p.values.iter().map(|v| string(v)).collect::<Vec<_>>().join(", ")));
    }
    for (name, value) in [("Default", &p.default), ("Min", &p.min), ("Max", &p.max)] {
        if value.is_some() {
            fields.push(format!("{name}: {}", some(value)));
        }
    }
    if p.required {
        fields.push("Required: true".to_string());
    }
    if p.help.is_some() {
        fields.push(format!("Help: {}", said(&p.help)));
    }
    format!("{{{}}}", fields.join(", "))
}

fn command(c: &CommandSpec) -> String {
    let line: Vec<String> = c.line.iter().map(|w| string(w)).collect();
    let arguments: Vec<String> = c.arguments.iter().map(param).collect();
    let options: Vec<String> = c.options.iter().map(param).collect();
    format!(
        "\t\t\t{{\n\t\t\t\tLine:      []string{{{}}},\n\t\t\t\tHelp:      {},\n\t\t\t\tArguments: []clarity.CliParam{{{}}},\n\t\t\t\tOptions:   []clarity.CliParam{{{}}},\n\t\t\t}},\n",
        line.join(", "),
        said(&c.help),
        arguments.join(", "),
        options.join(", ")
    )
}

/// A value of the command line as the type code holds it.
fn typed(p: &ParamSpec, at: usize) -> String {
    let value = format!("values[{at}]");
    match &p.ty {
        TypeInfo::Integer | TypeInfo::Duration => format!("clarity.CliInteger({value})"),
        TypeInfo::Decimal | TypeInfo::Money => format!("clarity.CliDecimal({value})"),
        TypeInfo::Date => format!("clarity.CliDate({value})"),
        TypeInfo::DateTime => format!("clarity.CliDateTime({value})"),
        TypeInfo::Bool => format!("clarity.CliBool({value})"),
        TypeInfo::Enum(name) => format!("{name}(clarity.CliText({value}))"),
        _ => format!("clarity.CliText({value})"),
    }
}

pub fn generate(project: &Project, settings: &Settings) -> Result<Vec<OutFile>, Vec<String>> {
    let module = super::module_name(settings);
    let project = &crate::with_secret_settings(project, &crate::settings_program(settings));
    let (databases, owned) = crate::databases(project, settings)?;
    let databases = crate::with_secret_locations(databases, &crate::settings_program(settings));
    let types = check::types(&project.files);
    let mut errors = Vec::new();
    let (servers, unused): (Vec<&Server>, Vec<&Server>) = SERVERS.iter().partition(|s| databases.iter().any(|(_, sql, _)| *sql == s.sql));
    let mut left_out: Vec<&str> = unused.iter().flat_map(|s| s.modules.iter().copied()).filter(|m| !servers.iter().any(|s| s.modules.contains(m))).collect();
    let engines = crate::engines(project);
    // The protected store's modules only for secret settings (DESIGN 5.27).
    if project.secret_settings().is_empty() {
        left_out.extend(super::SECRET_MODULES);
    }
    if engines.is_empty() {
        left_out.extend(ENGINE_MODULES);
    }
    let mut files = vec![text("texts.json", crate::commands::texts_json(project))];
    for file in &project.files {
        let Some(m) = &file.module else { continue };
        let file_types = types.get(&file.path).cloned().unwrap_or_default();
        let mut body = model::emit(project, file);
        match logic::emit(project, file, &file_types) {
            Ok(functions) => body.push_str(&functions),
            Err(mut problems) => errors.append(&mut problems),
        }
        match logic::emit_commands(project, file, &file_types) {
            Ok(commands) => body.push_str(&commands),
            Err(mut problems) => errors.append(&mut problems),
        }
        files.push(text(&format!("{}_module.go", super::names::module(&m.name)), go_file(&body)));
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    files.push(text("schema.go", super::schema_go(project, &owned)?));
    if !engines.is_empty() {
        files.extend(crate::engine_files(project, "engines"));
        files.push(text("engines.go", super::engines_go(project)));
    }
    files.push(text("main.go", main_go(project, settings, &databases, &servers)));
    let mut direct = vec![RUNTIME_MODULE];
    if files.iter().any(|f| f.path.ends_with(".go") && f.text.contains("\t\"github.com/shopspring/decimal\"\n")) {
        direct.push("github.com/shopspring/decimal");
    }
    files.push(text("go.mod", go_mod(&module, &left_out, &direct)));
    files.push(text("go.sum", go_sum(&left_out)));
    let name = crate::commands::program_name(settings);
    files.push(text(
        "README.md",
        format!(
            "# {title}: the command line\n\nGenerated by Clarity from the project {project}: the command-line program `{name}`, in Go. Do not edit these files; change the project in Clarity and generate it again.\n\n## Build and run\n\nIn this folder, build the program with\n\n    go build -trimpath -ldflags=\"-s -w\" -o bin/{name} .\n\nRun `bin/{name} --help` to see its commands.\n",
            title = clarity_project::title(settings),
            project = settings.name,
        ),
    ));
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn main_go(project: &Project, settings: &Settings, databases: &[(String, crate::ddl::Sql, String)], servers: &[&Server]) -> String {
    let commands = crate::commands::commands(project);
    let mut imports: Vec<(String, String)> = vec![(format!("{RUNTIME_MODULE}/clarity"), String::new())];
    imports.extend(servers.iter().map(|s| (format!("{RUNTIME_MODULE}/{}", s.package), "_ ".to_string())));
    // Secret settings (DESIGN 5.27): read and kept through the system's protected store.
    let secret_names = crate::secret_names(project);
    let secrets_list = format!("[]string{{{}}}", secret_names.iter().map(|n| string(n)).collect::<Vec<_>>().join(", "));
    if !secret_names.is_empty() {
        imports.push((format!("{RUNTIME_MODULE}/secrets"), String::new()));
    }
    imports.sort();
    let imports: String = imports.iter().map(|(path, name)| format!("\t{name}\"{path}\"\n")).collect();
    let sources = databases.iter().map(|(name, sql, at)| format!("{{Name: {}, Dialect: {}, At: {}}}", string(name), super::dialect(*sql), string(at))).collect::<Vec<_>>().join(", ");
    let stores = settings.filestores.iter().map(|s| format!("{}: {}", string(&s.name), string(&s.location))).collect::<Vec<_>>().join(", ");
    let languages: Vec<String> = project.translations.keys().map(|l| string(l)).collect();
    let base = project.locale.language.clone().unwrap_or_else(|| "en".to_string());
    let listed: String = commands.iter().map(command).collect();
    let kept = project.files.iter().find_map(|f| f.entities.iter().find(|e| e.settings));
    let (data, start, keep) = match kept {
        Some(entity) => {
            let language = entity
                .fields
                .iter()
                .find(|f| f.ty == FieldType::Language)
                .map(|f| format!("\tif data.Settings.{field} != \"\" {{\n\t\tlanguage := data.Settings.{field}\n\t\tclarity.SetLanguage(&language)\n\t}}\n", field = pascal(&f.name.name)))
                .unwrap_or_default();
            (
                "// CommandData is what every command has as data of its own: the settings (DESIGN 5.17).\ntype CommandData struct {\n\tSettings *Settings\n}\n",
                if secret_names.is_empty() {
                    format!("\tsettings := clarity.CliKeptSettings(clarity.ReadSettings(program), NewSettings())\n\tbefore := settings\n\tdata := &CommandData{{Settings: &settings}}\n{language}")
                } else {
                    format!("\t// The secret settings are kept in the system's protected store (DESIGN 5.27).\n\tclarity.Secrets = secrets.System{{}}\n\tsettings := clarity.CliKeptSettings(clarity.ReadSettingsWith(program, {secrets_list}), NewSettings())\n\tbefore := settings\n\tdata := &CommandData{{Settings: &settings}}\n{language}")
                },
                if secret_names.is_empty() {
                    "\t// A command that changed a setting keeps it, as a window does.\n\tclarity.CliKeepSettings(program, before, *data.Settings)\n".to_string()
                } else {
                    format!("\t// A command that changed a setting keeps it, as a window does; its secrets go to the protected store.\n\tclarity.CliKeepSettingsWith(program, before, *data.Settings, {secrets_list})\n")
                },
            )
        }
        None => ("// CommandData is what every command has as data of its own.\ntype CommandData struct{}\n", "\tdata := &CommandData{}\n".to_string(), String::new()),
    };
    let mut dispatch = String::new();
    for (i, c) in commands.iter().enumerate() {
        let values: Vec<String> = c.arguments.iter().chain(&c.options).enumerate().map(|(at, p)| typed(p, at)).collect();
        let args: String = values.iter().map(|v| format!(", {v}")).collect();
        dispatch.push_str(&format!("\t\tcase {i}:\n\t\t\treturn {}(db, cli, data{args})\n", logic::command_function(c.decl)));
    }
    format!(
        "{}\npackage main\n\nimport (\n\t_ \"embed\"\n\t\"fmt\"\n\t\"os\"\n\n{imports})\n\n// texts are the texts the program shows, in every language the project has.\n//\n//go:embed texts.json\nvar texts []byte\n\n// program is the desktop program's name, which names the folder the\n// person's settings are kept in: both programs share them.\nconst program = {program}\n\n// databases is the database of each data source: its name, its kind and where it is.\nvar databases = []clarity.Source{{{sources}}}\n\n// stores is the folder of each file store.\nvar stores = map[string]string{{{stores}}}\n\n{data}\n// cliProgram is the program and its commands, as the command line reads them.\nfunc cliProgram() clarity.CliProgram {{\n\treturn clarity.CliProgram{{\n\t\tName: {name},\n\t\tHelp: {help},\n\t\tCommands: []clarity.CliCommand{{\n{listed}\t\t}},\n\t}}\n}}\n\n// openDatabases opens every database and applies the migration steps it has not had yet.\nfunc openDatabases() (*clarity.Db, error) {{\n\tdb, err := clarity.Connect(databases...)\n\tif err != nil {{\n\t\treturn nil, err\n\t}}\n\tfor _, s := range Sources {{\n\t\tif _, err := db.MigrateSource(s.Source, s.Steps); err != nil {{\n\t\t\tdb.Close()\n\t\t\treturn nil, err\n\t\t}}\n\t}}\n\tfor name, at := range stores {{\n\t\tif err := db.AddStore(name, at); err != nil {{\n\t\t\tdb.Close()\n\t\t\treturn nil, err\n\t\t}}\n\t}}\n\treturn db, nil\n}}\n\nfunc main() {{\n\tclarity.LoadTranslations(texts)\n\tlocale := \"\"\n\tfor _, name := range []string{{\"LC_ALL\", \"LC_MESSAGES\", \"LANG\"}} {{\n\t\tif locale = os.Getenv(name); locale != \"\" {{\n\t\t\tbreak\n\t\t}}\n\t}}\n\tclarity.SetLanguage(clarity.CliPickLanguage(locale, []string{{{languages}}}, {base}))\n{start}\tdb, err := openDatabases()\n\tif err != nil {{\n\t\tfmt.Fprintln(os.Stderr, err)\n\t\tos.Exit(1)\n\t}}\n\tcode := clarity.RunCliProgram(cliProgram(), func(cli *clarity.CliSession, command int, values []*string) (int, error) {{\n\t\tswitch command {{\n{dispatch}\t\t}}\n\t\treturn 2, nil\n\t}})\n{keep}\tdb.Close()\n\tos.Exit(code)\n}}\n",
        super::HEADER,
        program = string(&crate::settings_program(settings)),
        name = string(&crate::commands::program_name(settings)),
        help = said(&crate::commands::program_help(project)),
        languages = languages.join(", "),
        base = string(&base),
    )
}
