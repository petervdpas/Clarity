use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use clarity_spec::{Preview, Severity};

#[derive(Parser)]
#[command(name = "clarity", version, about = "Clarity development environment")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Work with spec files.
    Spec {
        #[command(subcommand)]
        command: SpecCommand,
    },
    /// Transpile a project into the stack its project file names.
    Gen {
        /// The project file, like shop.clarity; its modules are in shop/.
        project: PathBuf,
        /// Where the application goes; the project's output folder by default.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Write the file each custom part and library follows from its
    /// declaration, and a starter into a folder that has none (DESIGN 5.19, 5.21).
    Parts {
        /// The project file, like shop.clarity.
        project: PathBuf,
    },
    /// Write the changes in the data dictionary that no migration covers
    /// yet as the next step in the project's migrations folder.
    Migrate {
        /// The project file, like shop.clarity.
        project: PathBuf,
        /// Only list the pending changes.
        #[arg(long)]
        check: bool,
        /// The title of the step; "Step N" by default.
        #[arg(long)]
        title: Option<String>,
        /// Treat proposed renames as a removal and an addition.
        #[arg(long)]
        no_renames: bool,
        /// Write the step even when changes can lose data.
        #[arg(long)]
        allow_data_loss: bool,
    },
    /// Take ownership of an external data source: its tables as they are
    /// become a baseline step, and from then on Clarity migrates them.
    Own {
        /// The project file, like shop.clarity.
        project: PathBuf,
        /// The external data source, as the project file names it.
        source: String,
    },
    /// Import tables of an external SQLite data source as a module of
    /// entities mapped onto them.
    Import {
        /// The project file, like shop.clarity.
        project: PathBuf,
        /// The external data source, as the project file names it.
        source: String,
        /// The module to write; the source's name by default.
        #[arg(long)]
        module: Option<String>,
        /// The tables to import, comma separated; all that can be by default.
        #[arg(long, value_delimiter = ',')]
        tables: Vec<String>,
        /// Only list the tables.
        #[arg(long)]
        list: bool,
    },
    /// Look for the build tools of every stack, and say what to install.
    Tools,
}

#[derive(Subcommand)]
enum SpecCommand {
    /// Parse and check the modules of a project.
    Check {
        /// The project file, or the folder with its modules.
        dir: PathBuf,
    },
    /// Name every part that has none, and make part and handler names
    /// camelCase, in every module of a project: what the checker asks of
    /// projects written before names were required.
    Names {
        /// The project file, or the folder with its modules.
        dir: PathBuf,
    },
    /// Make every text people read a key: in the modules, a key in place of
    /// each text, and a file per language giving every key its value, the
    /// texts' own in the default language: what the checker asks of projects
    /// written before texts had keys.
    Keys {
        /// The project file, or the folder with its modules.
        dir: PathBuf,
    },
    /// List, per language, the keys its file gives no value yet, written as
    /// lines to fill in.
    Texts {
        /// The project file, or the folder with its modules.
        dir: PathBuf,
    },
    /// Print the checked model as JSON.
    Model {
        /// The project file, or the folder with its modules.
        dir: PathBuf,
        /// Include sample rows per entity, as used by the preview.
        #[arg(long)]
        samples: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Spec { command } => match command {
            SpecCommand::Check { dir } => check(&dir),
            SpecCommand::Model { dir, samples } => model(&dir, samples),
            SpecCommand::Names { dir } => names(&dir),
            SpecCommand::Texts { dir } => texts(&dir),
            SpecCommand::Keys { dir } => keys(&dir),
        },
        Command::Gen { project, out } => generate(&project, out.as_deref()),
        Command::Parts { project } => parts(&project),
        Command::Migrate {
            project,
            check,
            title,
            no_renames,
            allow_data_loss,
        } => migrate(&project, check, title, no_renames, allow_data_loss),
        Command::Import {
            project,
            source,
            module,
            tables,
            list,
        } => import(&project, &source, module, tables, list),
        Command::Own { project, source } => own(&project, &source),
        Command::Tools => {
            print!("{}", clarity_gen::tools::report(&clarity_gen::tools::check_all(&clarity_gen::tools::System)));
            ExitCode::SUCCESS
        }
    }
}


fn parts(project_file: &std::path::Path) -> ExitCode {
    let project = match load(project_file) {
        Ok(project) => project,
        Err(code) => return code,
    };
    let dir = if project_file.is_file() { clarity_project::module_dir(project_file) } else { project_file.to_path_buf() };
    match clarity_gen::parts::write_part_folders(&dir, &project) {
        Ok(written) => {
            for path in &written {
                eprintln!("wrote {}", path.display());
            }
            if written.is_empty() {
                eprintln!("every part's folder is up to date");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("clarity: cannot write the parts' folders: {err}");
            ExitCode::FAILURE
        }
    }
}

fn generate(project_file: &std::path::Path, out: Option<&std::path::Path>) -> ExitCode {
    if !project_file.is_file() {
        eprintln!("clarity: {} is not a project file; give one like shop.clarity", project_file.display());
        return ExitCode::from(2);
    }
    let project = match load(project_file) {
        Ok(project) => project,
        Err(code) => return code,
    };
    let file = clarity_project::load(project_file);
    let Some(settings) = file.settings.filter(|_| file.problems.is_empty()) else {
        for problem in &file.problems {
            eprintln!("{}: {problem}", project_file.display());
        }
        return ExitCode::FAILURE;
    };
    let out = out.map_or_else(|| clarity_project::out_dir(project_file, &settings), std::path::Path::to_path_buf);
    let out = out.as_path();
    let built = clarity_project::build_settings(&clarity_project::module_dir(project_file), &settings);
    let programs = match clarity_gen::programs(&project, &built) {
        Ok(programs) => programs,
        Err(errors) => {
            for diagnostic in project.diagnostics.iter().filter(|d| d.severity == Severity::Error) {
                eprintln!("{diagnostic}");
            }
            for error in errors {
                eprintln!("clarity: {error}");
            }
            return ExitCode::FAILURE;
        }
    };
    match clarity_gen::write_programs(out, &programs, settings.stack.backend, &settings.keeps) {
        Ok(written) => {
            for program in &programs {
                let dir = clarity_project::target_dir(out, program.target);
                eprintln!("{} file(s) in {}", program.files.len(), dir.display());
            }
            eprintln!("{written} changed");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("clarity: cannot write {}: {err}", out.display());
            ExitCode::FAILURE
        }
    }
}

/// Loads the modules of a project, given its project file or its folder.
fn load(path: &std::path::Path) -> Result<clarity_spec::Project, ExitCode> {
    let dir = if path.is_file() { clarity_project::module_dir(path) } else { path.to_path_buf() };
    clarity_project::load_modules(&dir).map_err(|err| {
        eprintln!("clarity: cannot read {}: {err}", dir.display());
        ExitCode::from(2)
    })
}

fn names(path: &std::path::Path) -> ExitCode {
    let dir = if path.is_file() { clarity_project::module_dir(path) } else { path.to_path_buf() };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        eprintln!("clarity: cannot read {}", dir.display());
        return ExitCode::from(2);
    };
    let mut files: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "clarity")).collect();
    files.sort();
    let mut changed = 0;
    let mut renames = Vec::new();
    for file in &files {
        let Ok(text) = std::fs::read_to_string(file) else { continue };
        let named = match clarity_spec::edit::apply(&text, &clarity_spec::edit::Edit::NameParts) {
            Ok(outcome) => outcome.text,
            Err(err) => {
                eprintln!("clarity: {}: {err}", file.display());
                continue;
            }
        };
        let (camel, renamed) = clarity_spec::names::camel_names_with(&named);
        renames.extend(renamed);
        if camel != text {
            if let Err(err) = std::fs::write(file, &camel) {
                eprintln!("clarity: cannot write {}: {err}", file.display());
                return ExitCode::FAILURE;
            }
            eprintln!("named {}", file.display());
            changed += 1;
        }
    }
    // The translations' texts follow the renamed slots of the module's texts.
    let translations = dir.join(clarity_spec::translations::TRANSLATIONS_DIR);
    if let Ok(entries) = std::fs::read_dir(&translations) {
        for path in entries.filter_map(Result::ok).map(|e| e.path()) {
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let renamed = clarity_spec::names::rename_slots(&text, &renames);
            if renamed != text && std::fs::write(&path, &renamed).is_ok() {
                eprintln!("renamed slots in {}", path.display());
            }
        }
    }
    eprintln!("{changed} of {} file(s) changed", files.len());
    ExitCode::SUCCESS
}

fn check(dir: &std::path::Path) -> ExitCode {
    let project = match load(dir) {
        Ok(project) => project,
        Err(code) => return code,
    };
    if project.files.is_empty() {
        eprintln!("clarity: no .clarity files in {}", dir.display());
        return ExitCode::from(2);
    }
    for diagnostic in &project.diagnostics {
        eprintln!("{diagnostic}");
    }
    let errors = project
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warnings = project.diagnostics.len() - errors;
    eprintln!(
        "{} file(s), {errors} error(s), {warnings} warning(s)",
        project.files.len()
    );
    if errors > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn keys(path: &std::path::Path) -> ExitCode {
    let project = match load(path) {
        Ok(project) => project,
        Err(code) => return code,
    };
    let dir = if path.is_file() { clarity_project::module_dir(path) } else { path.to_path_buf() };
    let read = |folder: &std::path::Path| -> Vec<(String, String)> {
        let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };
        let mut files: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "clarity")).collect();
        files.sort();
        files.into_iter().filter_map(|p| std::fs::read_to_string(&p).ok().map(|t| (p.display().to_string(), t))).collect()
    };
    let modules = read(&dir);
    let folder = dir.join(clarity_spec::translations::TRANSLATIONS_DIR);
    let languages = read(&folder);
    let default = clarity_spec::translations::default_language(&project);
    let made = clarity_spec::keyed::make_keys(
        &modules.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect::<Vec<_>>(),
        &languages.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect::<Vec<_>>(),
        &default,
    );
    for ((path, before), (_, after)) in modules.iter().zip(&made.modules) {
        if before != after {
            if let Err(err) = std::fs::write(path, after) {
                eprintln!("clarity: cannot write {path}: {err}");
                return ExitCode::FAILURE;
            }
            eprintln!("keyed {path}");
        }
    }
    if let Err(err) = std::fs::create_dir_all(&folder) {
        eprintln!("clarity: cannot make {}: {err}", folder.display());
        return ExitCode::FAILURE;
    }
    for language in made.languages.keys() {
        let file = folder.join(format!("{language}.clarity"));
        if let Err(err) = std::fs::write(&file, made.language_file(language)) {
            eprintln!("clarity: cannot write {}: {err}", file.display());
            return ExitCode::FAILURE;
        }
        eprintln!("wrote {}", file.display());
    }
    ExitCode::SUCCESS
}

fn texts(dir: &std::path::Path) -> ExitCode {
    let project = match load(dir) {
        Ok(project) => project,
        Err(code) => return code,
    };
    use clarity_spec::translations::{default_language, keys};
    let used = keys(&project);
    let default = default_language(&project);
    let defaults = project.translations.get(&default);
    for (language, values) in &project.translations {
        let empty: Vec<_> = used.iter().filter(|u| values.get(&u.key).is_none_or(|v| v.is_empty())).collect();
        println!("// {language}{}: {} of {} keys have no value", if *language == default { " (default)" } else { "" }, empty.len(), used.len());
        for u in empty {
            let said = defaults.and_then(|d| d.get(&u.key)).filter(|v| !v.is_empty()).map(|v| format!(" {}", clarity_spec::lexer::quote(v))).unwrap_or_default();
            println!("    {} \"\"  // {}{said}", u.key, u.places.join(", "));
        }
    }
    if project.translations.is_empty() {
        println!("// {} keys, and no language file yet; add translations/{default}.clarity", used.len());
        for u in &used {
            println!("    {} \"\"  // {}", u.key, u.places.join(", "));
        }
    }
    ExitCode::SUCCESS
}

fn model(dir: &std::path::Path, samples: bool) -> ExitCode {
    let project = match load(dir) {
        Ok(project) => project,
        Err(code) => return code,
    };
    let json = if samples {
        serde_json::to_string_pretty(&Preview::new(project))
    } else {
        serde_json::to_string_pretty(&project)
    };
    match json {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("clarity: {err}");
            ExitCode::FAILURE
        }
    }
}

fn migrate(project_file: &std::path::Path, check: bool, title: Option<String>, no_renames: bool, allow_data_loss: bool) -> ExitCode {
    use clarity_spec::migrate;
    let project = match load(project_file) {
        Ok(project) => project,
        Err(code) => return code,
    };
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    if !errors.is_empty() {
        for diagnostic in errors {
            eprintln!("{diagnostic}");
        }
        eprintln!("clarity: fix the errors before migrating");
        return ExitCode::FAILURE;
    }
    let dir = if project_file.is_file() { clarity_project::module_dir(project_file) } else { project_file.to_path_buf() };
    let mut pending_any = false;
    // Each SQL data source has its own steps.
    for source in project.sources.iter().filter(|s| s.sql).map(|s| s.name.as_str()) {
        let renames = if no_renames { Vec::new() } else { migrate::proposed_renames(&project, source) };
        let ops = migrate::pending(&project, source, &renames);
        if ops.is_empty() {
            continue;
        }
        pending_any = true;
        eprintln!("Changes in data source {source} not in a migration yet:");
        for op in &ops {
            let mark = if op.loses_data() { "  (can lose data)" } else { "" };
            for (i, line) in migrate::op_text(op).lines().enumerate() {
                eprintln!("  {line}{}", if i == 0 { mark } else { "" });
            }
        }
        if !renames.is_empty() {
            eprintln!("Renames are proposed; use --no-renames to remove and add instead.");
        }
        if check {
            continue;
        }
        if ops.iter().any(migrate::Op::loses_data) && !allow_data_loss {
            eprintln!("clarity: some changes can lose data; check them and run again with --allow-data-loss");
            return ExitCode::FAILURE;
        }
        let count = project.migrations.as_ref().and_then(|m| m.get(source)).map_or(0, Vec::len);
        let title = title.clone().unwrap_or_else(|| format!("Step {}", count + 1));
        match clarity_spec::write_step(&dir, source, &title, &ops) {
            Ok(path) => eprintln!("Wrote {}", path.display()),
            Err(err) => {
                eprintln!("clarity: cannot write the step: {err}");
                return ExitCode::FAILURE;
            }
        }
    }
    if !pending_any {
        eprintln!("The migrations cover the data dictionary; nothing to migrate.");
    }
    ExitCode::SUCCESS
}

fn import(project_file: &std::path::Path, source: &str, module: Option<String>, tables: Vec<String>, list: bool) -> ExitCode {
    let dir = clarity_project::module_dir(project_file);
    let available = match clarity_project::source_tables(&dir, source) {
        Ok(tables) => tables,
        Err(err) => {
            eprintln!("clarity: {err}");
            return ExitCode::FAILURE;
        }
    };
    if list {
        for table in &available {
            match &table.not_importable {
                None => eprintln!("  {} ({} columns)", table.name, table.columns),
                Some(reason) => eprintln!("  {} (not importable: {reason})", table.name),
            }
        }
        return ExitCode::SUCCESS;
    }
    let tables = if tables.is_empty() {
        available.iter().filter(|t| t.not_importable.is_none()).map(|t| t.name.clone()).collect()
    } else {
        tables
    };
    let module = module.unwrap_or_else(|| source.to_string());
    match clarity_project::import_tables(&dir, source, &module, &tables) {
        Ok(done) => {
            eprintln!("Wrote {}", done.path.display());
            for (table, reason) in &done.skipped {
                eprintln!("  {table} is left out: {reason}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("clarity: {err}");
            ExitCode::FAILURE
        }
    }
}

fn own(project_file: &std::path::Path, source: &str) -> ExitCode {
    match clarity_project::take_ownership(&clarity_project::module_dir(project_file), source) {
        Ok(owned) => {
            eprintln!("Wrote {} and removed 'external' from {source} in {}.", owned.step.display(), owned.project_file.display());
            eprintln!("Clarity now migrates {source}; the baseline step changes nothing in the database it was taken from.");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("clarity: {err}");
            ExitCode::FAILURE
        }
    }
}
