use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use clarity_spec::{Preview, Severity};

mod users;

/// The release's version, which the installer's build names (CLARITY_VERSION, the
/// IDE's version); the workspace's when built from source.
const VERSION: &str = match option_env!("CLARITY_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

#[derive(Parser)]
#[command(name = "clarity", version = VERSION, about = "Clarity development environment")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a project, and bring an older one up to what the checker asks
    ///
    /// See manual/project.run-build.md.
    Spec {
        #[command(subcommand)]
        command: SpecCommand,
    },
    /// Generate a project's program for its stack
    ///
    /// Transpiles the project into the stack its project file names. See manual/project.run-build.md.
    Gen {
        /// The project file, like shop.clarity; its modules are in shop/.
        project: PathBuf,
        /// Where the application goes; the project's output folder by default.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Build a generated project's program with its stack's own tools
    ///
    /// Builds what `clarity gen` wrote, as the IDE's Build does: the engines whose source changed,
    /// then a release program, with Clarity.UI's library where it draws with it. See manual/project.run-build.md.
    Build {
        /// The project file, like shop.clarity; its modules are in shop/.
        project: PathBuf,
        /// A debug program instead of a release one.
        #[arg(long)]
        debug: bool,
        /// The command-line program, of a project that has windows too.
        #[arg(long)]
        cli: bool,
    },
    /// Build a generated project's program when it changed, and run it
    ///
    /// Does what the IDE's Run does: builds a debug program when it changed since the last build, migrates the
    /// program's databases, and starts it here, on the project's data. The words after `--` go to
    /// the command-line program. See manual/project.run-build.md.
    Run {
        /// The project file, like shop.clarity; its modules are in shop/.
        project: PathBuf,
        /// A release program instead of a debug one.
        #[arg(long)]
        release: bool,
        /// The words the command-line program runs with, after `--`.
        #[arg(last = true)]
        words: Vec<String>,
    },
    /// Write the files of custom parts and libraries
    ///
    /// Writes the file each custom part and library follows from its declaration, and a starter
    /// into a folder that has none. See manual/design.custom-parts.md and manual/code.libraries.md.
    Parts {
        /// The project file, like shop.clarity.
        project: PathBuf,
    },
    /// Move windows and views into .cxaml files
    ///
    /// Moves each window and view out of the project's .clarity files into a .cxaml file of its
    /// own, in a folder named after its module. See manual/design.windows.md.
    ConvertPages {
        /// The project file, like shop.clarity.
        project: PathBuf,
    },
    /// Write the next migration step
    ///
    /// Writes the changes in the data dictionary that no migration covers yet as the next step
    /// in the project's migrations folder. See manual/data.migrations.md.
    Migrate {
        /// The project file, like shop.clarity.
        project: PathBuf,
        /// Only list the pending changes.
        #[arg(long)]
        check: bool,
        /// The title of the step; "Step N" by default.
        #[arg(long)]
        title: Option<String>,
        /// A rename the step records, so the data stays: `Customer=Client` for an entity,
        /// `Customer.name=fullName` for a field, `Kind.Private=Personal` for a value of an enum.
        /// Repeat it for more; nothing else is taken as a rename.
        #[arg(long = "rename", value_name = "FROM=TO")]
        renames: Vec<String>,
        /// Write the step even when changes can lose data.
        #[arg(long)]
        allow_data_loss: bool,
    },
    /// Take ownership of an external data source
    ///
    /// Its tables as they are become a baseline step, and from then on Clarity migrates them.
    /// See manual/data.sources.md.
    Own {
        /// The project file, like shop.clarity.
        project: PathBuf,
        /// The external data source, as the project file names it.
        source: String,
    },
    /// Import a data source's tables as entities
    ///
    /// Imports the tables of an external SQLite data source, or of a data source that is a file
    /// (CSV, JSON, XML), as a module of entities mapped onto them. See manual/data.sources.md.
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
        /// An XSD schema of an XML file source, for typed entities.
        #[arg(long)]
        schema: Option<PathBuf>,
    },
    /// Back up, list and restore the project's databases
    ///
    /// Backs up the project's SQLite databases, lists the backups and restores one; brings a
    /// database up to date. See manual/data.sources.md and manual/data.migrations.md.
    Data {
        #[command(subcommand)]
        command: DataCommand,
    },
    /// Manage the people of a project that signs people in
    ///
    /// Lists, adds and removes people, and gives a password, a token or recovery codes.
    /// See manual/project.signin.md.
    Users {
        /// The project file, like club.clarity.
        project: PathBuf,
        #[command(subcommand)]
        command: UsersCommand,
    },
    /// Look for the build tools of every stack
    ///
    /// Says which tools each stack has and what to install. See manual/setup.tools.md.
    Tools,
    /// Run the release checks, inside Clarity's repository
    ///
    /// The release checks of Clarity's repository that are no test; Clarity settings > DevTools >
    /// Release planning runs them.
    Release {
        #[command(subcommand)]
        command: ReleaseCommand,
    },
    /// Serve Clarity's tools to an AI agent over MCP
    ///
    /// Serves them on stdin and stdout, on the project open in the Clarity IDE: the agent designs
    /// and generates, and never builds or runs. Messages go to stderr. See manual/ai.mcp.md.
    Mcp {
        /// For a turn of the IDE's chat panel: the file where show_window and
        /// show_diagram write what to show, which the IDE reads when the turn ends.
        #[arg(long)]
        turn: Option<PathBuf>,
    },
    /// Rebuild the base libraries, inside Clarity's repository
    ///
    /// Rebuilds Clarity.UI, Clarity.Audio, Clarity.Video, Clarity.Image, Clarity.Formats,
    /// Clarity.Auth and Clarity.AppMcp from the source in Clarity's repository: every Go, C, C++
    /// and Rust desktop program Clarity builds then links them, so a change of Clarity.UI is
    /// tried in any project.
    Libraries,
}

#[derive(Subcommand)]
enum ReleaseCommand {
    /// Make target/fresh-checkout as the checks on GitHub have it
    ///
    /// The last commit without tags or history, the uncommitted changes over it, nothing git
    /// ignores. Needs no network.
    FreshCheckout,
    /// Fail when the checks left files in the tree since the fresh checkout.
    CleanTree,
    /// Build every test of the fresh checkout once
    ///
    /// As the checks on GitHub do, noting each test program.
    BuildTests,
    /// Run a test program the build made
    ///
    /// Named `<package>:<target>` (`--lib` for a library's own tests), its arguments after `--`.
    RunTest {
        key: String,
        #[arg(last = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum UsersCommand {
    /// List the people, by the field they are found by.
    List,
    /// Add a person; with a password strategy, asks their password.
    Add {
        name: String,
        /// Read the password from standard input instead of asking it.
        #[arg(long)]
        stdin: bool,
    },
    /// Give a person a new password, asked without showing it.
    Password {
        name: String,
        /// Read the password from standard input instead of asking it.
        #[arg(long)]
        stdin: bool,
    },
    /// Give a person a new token, shown once.
    Token { name: String },
    /// Give a person new recovery codes, shown once.
    Codes { name: String },
    /// Remove a person.
    Remove { name: String },
}

#[derive(Subcommand)]
enum DataCommand {
    /// Take a backup of a data source's database, into its folder in the project's backups folder.
    Backup {
        /// The project file, like shop.clarity.
        project: PathBuf,
        /// The data source; the first SQLite one the project owns by default.
        #[arg(long)]
        source: Option<String>,
    },
    /// List the backups of a data source's database, the newest first.
    Backups {
        /// The project file, like shop.clarity.
        project: PathBuf,
        /// The data source; the first SQLite one the project owns by default.
        #[arg(long)]
        source: Option<String>,
    },
    /// Create the databases or bring them up to date
    ///
    /// With Clarity's migration tool (clarity-migrate): every database the project owns, SQLite,
    /// PostgreSQL, MySQL or SQL Server, gets the steps it lacks (an SQLite database that changes is
    /// backed up first when the project has a backups folder); one ahead of the project's steps is
    /// refused, and the first that fails stops the rest, as Run does. See manual/data.migrations.md.
    Migrate {
        /// The project file, like shop.clarity.
        project: PathBuf,
        /// Only this data source; every one the project owns by default.
        #[arg(long)]
        source: Option<String>,
    },
    /// Put a backup in place of the database, after backing up what is there now.
    Restore {
        /// The project file, like shop.clarity.
        project: PathBuf,
        /// The backup's file name, as `clarity data backups` lists it; a path is refused.
        backup: String,
        /// The data source; the first SQLite one the project owns by default.
        #[arg(long)]
        source: Option<String>,
    },
}

#[derive(Subcommand)]
enum SpecCommand {
    /// Parse and check the modules of a project.
    Check {
        /// The project file, or the folder with its modules.
        dir: PathBuf,
    },
    /// Name unnamed parts, and make part and handler names camelCase
    ///
    /// In every window, view and app of a project, in a module or in its own .cxaml, with the
    /// code and the translations' slots that name them: what the checker asks of projects
    /// written before names were required. Refused when the project would gain an error.
    Names {
        /// The project file, or the folder with its modules.
        dir: PathBuf,
    },
    /// Make every text people read a key
    ///
    /// In the modules and the windows', views' and app's .cxaml files, a key in place of each
    /// text, added to the file of every language: the texts' own in the default language. The
    /// keys the project has stay. What the checker asks of projects written before texts had
    /// keys. See manual/languages.md.
    Keys {
        /// The project file, or the folder with its modules.
        dir: PathBuf,
    },
    /// List the keys each language has no text for yet
    ///
    /// A key its file lacks or leaves empty, written as lines to fill in. See manual/languages.md.
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
    /// Write each window and app as markup for the web screen
    ///
    /// `windows/Main.cxaml` and `apps/Shop.cxaml` in a folder. See manual/start.how-it-works.md.
    Windows {
        /// The project file, or the folder with its modules.
        dir: PathBuf,
        /// The folder the window files go into.
        out: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    // Installed beside the IDE, the tool takes the library and browser files its package carries.
    if let Some(dir) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(std::path::Path::to_path_buf)) {
        let resources = clarity_gen::clarity_ui::installed_resources(&dir, std::env::consts::OS);
        clarity_gen::clarity_ui::set_bundled(resources.join("clarity-ui"));
        clarity_gen::clarity_ui::set_bundled_browser(resources.join("clarity-ui-browser"));
    }
    // A cDdev's swaps (Clarity settings, a run from the repository), which the IDE's builds use too.
    // An installed Clarity always builds with the release.
    if cfg!(debug_assertions) {
        clarity_gen::tools::environment::use_libraries(&clarity_gen::tools::environment::Environment::current());
    }
    match cli.command {
        Command::Spec { command } => match command {
            SpecCommand::Check { dir } => check(&dir),
            SpecCommand::Model { dir, samples } => model(&dir, samples),
            SpecCommand::Windows { dir, out } => windows(&dir, &out),
            SpecCommand::Names { dir } => names(&dir),
            SpecCommand::Texts { dir } => texts(&dir),
            SpecCommand::Keys { dir } => keys(&dir),
        },
        Command::Gen { project, out } => generate(&project, out.as_deref()),
        Command::Build { project, debug, cli } => build_or_run(&project, !debug, cli, None),
        Command::Run { project, release, words } => build_or_run(&project, release, !words.is_empty(), Some(words)),
        Command::Parts { project } => parts(&project),
        Command::ConvertPages { project } => convert_pages(&project),
        Command::Migrate {
            project,
            check,
            title,
            renames,
            allow_data_loss,
        } => migrate(&project, check, title, &renames, allow_data_loss),
        Command::Import {
            project,
            source,
            module,
            tables,
            list,
            schema,
        } => import(&project, &source, module, tables, list, schema),
        Command::Own { project, source } => own(&project, &source),
        Command::Data { command } => data(command),
        Command::Users { project, command } => {
            let (action, stdin) = match command {
                UsersCommand::List => (users::Action::List, false),
                UsersCommand::Add { name, stdin } => (users::Action::Add(name), stdin),
                UsersCommand::Password { name, stdin } => (users::Action::Password(name), stdin),
                UsersCommand::Token { name } => (users::Action::Token(name), false),
                UsersCommand::Codes { name } => (users::Action::Codes(name), false),
                UsersCommand::Remove { name } => (users::Action::Remove(name), false),
            };
            let mut password = move || {
                if stdin {
                    let mut line = String::new();
                    std::io::stdin().read_line(&mut line).ok()?;
                    let line = line.trim_end_matches(['\r', '\n']).to_string();
                    (!line.is_empty()).then_some(line)
                } else {
                    clarity_runtime::cli::read_password("Password: ")
                }
            };
            match users::run(&project, action, &mut password, &mut std::io::stdout(), &mut std::io::stderr()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(why) => {
                    eprintln!("clarity: {why}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Libraries => {
            let here = std::env::current_dir().unwrap_or_default();
            let Some(repo) = clarity_gen::repository_of(&here) else {
                eprintln!("clarity: {} is not inside Clarity's repository: run `clarity libraries` there", here.display());
                return ExitCode::FAILURE;
            };
            match clarity_gen::rebuild_base_libraries(&repo) {
                Ok(folder) => {
                    println!("The base libraries are rebuilt from {}: the programs Clarity builds link them from {}.", repo.display(), folder.display());
                    ExitCode::SUCCESS
                }
                Err(why) => {
                    eprintln!("clarity: {why}");
                    ExitCode::FAILURE
                }
            }
        }
        // Only the protocol goes to stdout.
        Command::Mcp { turn } => match clarity_mcp::run(turn) {
            Ok(()) => ExitCode::SUCCESS,
            Err(why) => {
                eprintln!("clarity: {why}");
                ExitCode::FAILURE
            }
        },
        Command::Release { command } => {
            let here = std::env::current_dir().unwrap_or_default();
            let Some(repo) = clarity_gen::repository_of(&here) else {
                eprintln!("clarity: {} is not inside Clarity's repository: run `clarity release` there", here.display());
                return ExitCode::from(2);
            };
            let dir = clarity_devtools::release::fresh_checkout_dir(&repo);
            let done = match command {
                ReleaseCommand::FreshCheckout => clarity_devtools::release::fresh_checkout(&repo, &dir).map(|()| format!("a fresh checkout in {}", dir.display())),
                ReleaseCommand::CleanTree => clarity_devtools::release::clean_tree(&repo, &dir).map(|()| "the checks left the tree clean".to_string()),
                ReleaseCommand::BuildTests => clarity_devtools::release::build_tests(&repo),
                ReleaseCommand::RunTest { key, args } => {
                    return match clarity_devtools::release::run_test(&repo, &key, &args) {
                        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
                        Err(e) => {
                            eprintln!("clarity: {e}");
                            ExitCode::from(1)
                        }
                    };
                }
            };
            match done {
                Ok(said) => {
                    println!("{said}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("clarity: {e}");
                    ExitCode::FAILURE
                }
            }
        }
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
    // The one way to generate, as the IDE's Generate and the agents' generate tool do (crates/clarity-build).
    match clarity_build::generate(project_file, out) {
        Ok(source) => {
            source.lines().iter().for_each(|line| eprintln!("{line}"));
            ExitCode::SUCCESS
        }
        Err(why) => {
            why.lines().for_each(|line| eprintln!("clarity: {line}"));
            ExitCode::from(if project_file.is_file() { 1 } else { 2 })
        }
    }
}

/// `clarity build` and (with `words`) `clarity run`: what the IDE's Build
/// and Run do (crates/clarity-build), saying each step on the terminal; Run
/// starts the program here and ends with its exit code.
fn build_or_run(project_file: &std::path::Path, release: bool, cli: bool, words: Option<Vec<String>>) -> ExitCode {
    if !project_file.is_file() {
        eprintln!("clarity: {} is not a project file; give one like shop.clarity", project_file.display());
        return ExitCode::from(2);
    }
    // Absolute, so the program is found from its own folder, where it starts.
    let project_file = &std::path::absolute(project_file).unwrap_or_else(|_| project_file.to_path_buf());
    let say = |line: String| eprintln!("{line}");
    let fail = |why: String| {
        eprintln!("clarity: {why}");
        ExitCode::FAILURE
    };
    let generated = match clarity_build::generate_out(project_file, release, cli, clarity_build::GENERATE_IN_THE_TERMINAL) {
        Ok(generated) => generated,
        Err(why) => return fail(why),
    };
    if let Err(why) = clarity_build::ready(&generated) {
        return fail(why);
    }
    let run = words.is_some();
    // The engines whose source changed first, so the program has them as written; their problems are in cargo's lines.
    let engines = match clarity_build::engines::build_changed(&clarity_project::module_dir(project_file), &say, &|_| {}) {
        Ok(built) => built,
        Err(why) => return fail(why),
    };
    // Run starts the last build when nothing changed since, as the IDE does.
    let binary = match &generated.built {
        Some((toolchain, _)) if run && !engines => generated.out.join(&toolchain.program),
        _ => match clarity_build::toolchain(&generated, &say).and_then(|toolchain| clarity_build::compile(&generated, &toolchain, &say)) {
            Ok(binary) => binary,
            Err(why) => return fail(why),
        },
    };
    let Some(words) = words else {
        eprintln!("built {}", binary.display());
        return ExitCode::SUCCESS;
    };
    // The program does not migrate (docs/ledger/migrations.md): Run migrates its databases first.
    if let Err(why) = clarity_build::migrate(&generated, &say) {
        return fail(why);
    }
    // A program served to a browser alone listens at the project's port, which no other program may hold.
    let address = clarity_build::serves(project_file, &generated.stack, cli);
    if let Some(address) = &address
        && let Err(why) = clarity_build::can_serve_at(address)
    {
        return fail(why);
    }
    eprintln!("{} (in {})", binary.display(), generated.out.display());
    // In the person's terminal: what the program writes and reads is theirs.
    let mut command = clarity_project::terminal_command(&binary);
    command.args(&words).current_dir(&generated.out);
    // The program works on the project's data, not the person's (conformance/data-folder.json).
    if let Some(data) = &generated.data {
        command.env("CLARITY_DATA", data);
    }
    if let Some(address) = &address {
        eprintln!("serving at http://{address}");
        command.args(["--address", address]);
    }
    match command.status() {
        Ok(status) => status.code().map_or(ExitCode::FAILURE, |code| ExitCode::from(code.clamp(0, 255) as u8)),
        Err(why) => fail(format!("cannot start {}: {why}", binary.display())),
    }
}

/// Reads and checks a project, given its project file or its folder, the
/// one way every front end does (clarity_project::check).
fn load(path: &std::path::Path) -> Result<clarity_spec::Project, ExitCode> {
    match clarity_project::check(path) {
        Ok(checked) => {
            checked.notes.iter().for_each(|note| eprintln!("clarity: {note}"));
            Ok(checked.project)
        }
        Err(why) => {
            eprintln!("clarity: {why}");
            Err(ExitCode::from(2))
        }
    }
}

fn names(path: &std::path::Path) -> ExitCode {
    let dir = if path.is_file() { clarity_project::module_dir(path) } else { path.to_path_buf() };
    // The one way the agents' name_parts and camel_names take too (clarity_tools::naming).
    let mut written: Vec<PathBuf> = Vec::new();
    for naming in [clarity_tools::naming::name_parts, clarity_tools::naming::camel_names] {
        match naming(&dir, None) {
            Ok(changes) => written.extend(changes.into_iter().map(|c| c.file).filter(|f| !written.contains(f)).collect::<Vec<_>>()),
            Err(why) => {
                eprintln!("clarity: {why}");
                return ExitCode::FAILURE;
            }
        }
    }
    for file in &written {
        eprintln!("named {}", file.display());
    }
    eprintln!("{} file(s) changed", written.len());
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
    let dir = if path.is_file() { clarity_project::module_dir(path) } else { path.to_path_buf() };
    // The one way the IDE's Make texts keys takes too (clarity_tools::texts_keys).
    match clarity_tools::texts_keys::make_texts_keys(&dir) {
        Ok(changes) => {
            for change in &changes {
                eprintln!("wrote {}", change.file.display());
            }
            eprintln!("{} file(s) changed", changes.len());
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("clarity: {why}");
            ExitCode::FAILURE
        }
    }
}

fn texts(dir: &std::path::Path) -> ExitCode {
    let project = match load(dir) {
        Ok(project) => project,
        Err(code) => return code,
    };
    use clarity_spec::translations::{default_language, keys, missing};
    // A key is missing in a language when its file lacks it or leaves it empty: the one
    // rule the IDE's languages table and the agents' missing_translations count by.
    let used = keys(&project).len();
    let default = default_language(&project);
    for (language, lacking) in missing(&project) {
        println!("// {language}{}: {} of {used} keys missing", if language == default { " (default)" } else { "" }, lacking.len());
        for m in lacking {
            let said = m.default_text.as_deref().map(|v| format!(" {}", clarity_spec::lexer::quote(v))).unwrap_or_default();
            println!("    {} \"\"  // {}{said}", m.key, m.places.join(", "));
        }
    }
    if project.translations.is_empty() {
        println!("// {used} keys, and no language file yet; add translations/{default}.clarity");
        for u in keys(&project) {
            println!("    {} \"\"  // {}", u.key, u.places.join(", "));
        }
    }
    ExitCode::SUCCESS
}

fn windows(dir: &std::path::Path, out: &std::path::Path) -> ExitCode {
    let project = match load(dir) {
        Ok(project) => project,
        Err(code) => return code,
    };
    for file in clarity_gen::web_windows::window_files(&project).into_iter().chain(clarity_gen::web_windows::app_files(&project)) {
        let path = out.join(file.path.trim_start_matches("ui/"));
        if let Err(err) = std::fs::create_dir_all(path.parent().unwrap_or(out)).and_then(|()| std::fs::write(&path, &file.text)) {
            eprintln!("clarity: cannot write {}: {err}", out.display());
            return ExitCode::FAILURE;
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

/// Moves every page block into its `.cxaml` file (clarity_tools::pages_out,
/// the one way the IDE's Move pages into design takes too): refused, moving
/// nothing, when a `.cxaml` is there already or the project would gain an error.
fn convert_pages(project_file: &std::path::Path) -> ExitCode {
    let dir = if project_file.is_file() { clarity_project::module_dir(project_file) } else { project_file.to_path_buf() };
    match clarity_tools::pages_out::move_pages_into_design(&dir) {
        Ok(changes) => {
            let pages = changes.iter().filter(|c| c.before.is_none()).count();
            eprintln!("{pages} window(s), view(s) and app(s) moved into .cxaml files");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("clarity: {why}");
            ExitCode::FAILURE
        }
    }
}

fn migrate(project_file: &std::path::Path, check: bool, title: Option<String>, renames: &[String], allow_data_loss: bool) -> ExitCode {
    use clarity_spec::migrate;
    use clarity_tools::steps;
    let project = match load(project_file) {
        Ok(project) => project,
        Err(code) => return code,
    };
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    for diagnostic in &errors {
        eprintln!("{diagnostic}");
    }
    let renames = match renames.iter().map(|r| named_rename(&project, r)).collect::<Result<Vec<_>, _>>() {
        Ok(renames) => renames,
        Err(why) => {
            eprintln!("clarity: {why}");
            return ExitCode::from(2);
        }
    };
    let dir = if project_file.is_file() { clarity_project::module_dir(project_file) } else { project_file.to_path_buf() };
    let pending = match steps::pending(&project, None, &renames) {
        Ok(pending) => pending,
        Err(why) => {
            eprintln!("clarity: {why}");
            return ExitCode::FAILURE;
        }
    };
    for p in pending.iter().filter(|p| !p.ops.is_empty()) {
        eprintln!("Changes in data source {} not in a migration yet:", p.source);
        for op in &p.ops {
            let mark = if op.loses_data() { "  (can lose data)" } else { "" };
            for (i, line) in migrate::op_text(op).lines().enumerate() {
                eprintln!("  {line}{}", if i == 0 { mark } else { "" });
            }
        }
        let suggested: Vec<&migrate::Rename> = p.proposed.iter().filter(|r| !renames.contains(r)).collect();
        if !suggested.is_empty() {
            eprintln!("Clarity can take these as renames, so the data stays; name each one you mean with --rename:");
            for r in suggested {
                let from = match &r.entity {
                    Some(entity) => format!("{entity}.{}", r.from),
                    None => r.from.clone(),
                };
                eprintln!("  --rename {from}={}", r.to);
            }
        }
    }
    if check {
        if pending.iter().all(|p| p.ops.is_empty()) {
            eprintln!("{}", steps::NOTHING_TO_MIGRATE);
        }
        return ExitCode::SUCCESS;
    }
    let request = steps::StepRequest { source: None, title: title.as_deref(), renames: &renames, allow_data_loss };
    match steps::write_steps(&dir, &request) {
        Ok(migrated) => {
            for change in &migrated.written {
                eprintln!("Wrote {}", change.file.display());
            }
            if migrated.written.is_empty() {
                eprintln!("{}", migrated.said);
            }
            ExitCode::SUCCESS
        }
        Err(why) => {
            let hint = if why.contains("can lose data") { " (--allow-data-loss)" } else { "" };
            eprintln!("clarity: {why}{hint}");
            ExitCode::FAILURE
        }
    }
}

/// A rename named on the command line (`--rename FROM=TO`): `Customer` an
/// entity, `Customer.name` a field, `Kind.Private` a value of the enum Kind.
fn named_rename(project: &clarity_spec::Project, text: &str) -> Result<clarity_spec::migrate::Rename, String> {
    let (from, to) = text.split_once('=').map(|(f, t)| (f.trim(), t.trim())).filter(|(f, t)| !f.is_empty() && !t.is_empty()).ok_or_else(|| format!("--rename {text}: write it as FROM=TO, like Customer.name=fullName"))?;
    Ok(match from.split_once('.') {
        Some((owner, name)) if project.enums().any(|e| e.name.name == owner) => clarity_spec::migrate::Rename::value(owner, name, to),
        Some((owner, name)) => clarity_spec::migrate::Rename { entity: Some(owner.into()), from: name.into(), to: to.into(), value: false },
        None => clarity_spec::migrate::Rename { entity: None, from: from.into(), to: to.into(), value: false },
    })
}

fn import(project_file: &std::path::Path, source: &str, module: Option<String>, tables: Vec<String>, list: bool, schema: Option<PathBuf>) -> ExitCode {
    let dir = clarity_project::module_dir(project_file);
    // A schema named on the command line is found from where it runs.
    let schema = schema.map(|s| std::path::absolute(&s).unwrap_or(s));
    let available = match clarity_project::source_tables(&dir, source, schema.as_deref()) {
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
    match clarity_project::import_tables(&dir, source, &module, &tables, schema.as_deref()) {
        Ok(done) => {
            eprintln!("Wrote {}", done.path.display());
            if let Some((file, _, _)) = &done.project_file {
                eprintln!("Set the at of {source} in {}", file.display());
            }
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

/// `clarity data migrate`: the databases the project owns created or brought up to date with
/// Clarity's migration tool (clarity-migrate), as Run does before it starts the program
/// (`clarity_build::databases`, `clarity_build::migrate_databases`): each SQLite one where Run
/// opens it, stopping at the first that fails.
fn migrate_data(project_file: &std::path::Path, source: Option<&str>) -> ExitCode {
    let project_file = &std::path::absolute(project_file).unwrap_or_else(|_| project_file.to_path_buf());
    let databases = match clarity_build::databases(project_file) {
        Ok(databases) => databases,
        Err(why) => {
            eprintln!("clarity: {why}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(name) = source
        && !databases.iter().any(|d| d.source == name)
    {
        eprintln!("clarity: the project owns no database of data source {name}");
        return ExitCode::FAILURE;
    }
    let chosen: Vec<clarity_migrate::Database> = databases.into_iter().filter(|d| source.is_none_or(|name| d.source == name)).collect();
    match clarity_build::migrate_databases(&chosen, &|line: String| eprintln!("{line}")) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("clarity: {why}");
            ExitCode::FAILURE
        }
    }
}

/// Backups of a project's databases: take one, list them, restore one by its
/// file name, with the one version the IDE and its agent use (`clarity_project::backups`).
fn data(command: DataCommand) -> ExitCode {
    let done = match command {
        DataCommand::Migrate { project, source } => return migrate_data(&project, source.as_deref()),
        DataCommand::Backup { project, source } => {
            clarity_project::backups::take(&clarity_project::module_dir(&project), source.as_deref()).map(|(found, b)| eprintln!("Backed up {} to {}", found.source, b.path.display()))
        }
        DataCommand::Backups { project, source } => clarity_project::backups::list(&clarity_project::module_dir(&project), source.as_deref()).map(|(found, all)| {
            if all.is_empty() {
                eprintln!("{} has no backups yet in {}; clarity data backup takes one.", found.source, found.folder.display());
            }
            for b in all {
                println!("{}  {:<24} {:>10}  {}", b.when, b.why, size(b.bytes), b.file);
            }
        }),
        DataCommand::Restore { project, backup, source } => {
            clarity_project::backups::restore(&clarity_project::module_dir(&project), source.as_deref(), &backup).map(|(found, before)| {
                eprintln!("Restored {} from {backup}.", found.source);
                if before.file != backup {
                    eprintln!("What it replaced is kept as {}.", before.file);
                }
            })
        }
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("clarity: {err}");
            ExitCode::FAILURE
        }
    }
}

/// A file's size as people read it: `840 B`, `12.3 kB`, `4.1 MB`.
fn size(bytes: u64) -> String {
    match bytes {
        b if b < 1000 => format!("{b} B"),
        b if b < 1_000_000 => format!("{:.1} kB", b as f64 / 1000.0),
        b => format!("{:.1} MB", b as f64 / 1_000_000.0),
    }
}
