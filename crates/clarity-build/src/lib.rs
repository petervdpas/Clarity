//! Generate, Build and Run of a Clarity project, as the IDE's buttons and the
//! `clarity gen`, `clarity build` and `clarity run` commands do them
//! (docs/ledger/build-tools.md): the project is generated into its output
//! folder and compiled with the stack's own toolchain (cargo for Rust, dotnet
//! for C#, go, CMake). Engines whose source changed are built first; Run
//! migrates the program's databases before it starts it. What a step says
//! goes to `say`, the IDE's Output panel or the terminal.

pub mod engines;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// Where a build's lines go: the IDE's Output panel, or the terminal.
pub type Say<'a> = &'a (dyn Fn(String) + Sync);

/// What the IDE asks a person to do to generate the project.
pub const GENERATE_IN_THE_IDE: &str = "press Generate first";
/// What the terminal asks.
pub const GENERATE_IN_THE_TERMINAL: &str = "run clarity gen first";

/// Whether a web application Run starts can listen at `address`, the
/// project's port: no other program may listen there already.
pub fn can_serve_at(address: &str) -> Result<(), String> {
    match std::net::TcpListener::bind(address) {
        Ok(_) => Ok(()),
        Err(_) => {
            let port = address.rsplit(':').next().unwrap_or(address);
            Err(format!("another program listens on port {port}; stop it, or set another port on the Project tab"))
        }
    }
}

/// A generated program: where it is, how to build it, and what its last
/// build was made from.
pub struct Generated {
    pub out: PathBuf,
    /// The project's data folder, which Run names in CLARITY_DATA so the program works on the project's data.
    pub data: Option<PathBuf>,
    /// The program's databases, which Run migrates with clarity-migrate before it starts the program.
    pub databases: Vec<clarity_migrate::Database>,
    pub toolchain: clarity_gen::Toolchain,
    pub stack: clarity_project::Stack,
    /// The mark of its generated files, which a build records.
    pub fingerprint: String,
    pub release: bool,
    /// The last build, while it was made from these same files and its
    /// program is there: Run starts it without building again.
    pub built: Option<(clarity_gen::Toolchain, bool)>,
    /// Whether the build links Clarity.UI's library from the folder
    /// CLARITY_UI names, which Clarity fetches when the person names none.
    pub library: bool,
}

/// Generates the project: transpiles it into its source tree, the code of
/// each of its programs in its stack's language and UI framework, written
/// into `out`, else the folder its `output` setting names; built nothing
/// yet. It builds and runs with the stack's own tools, in Clarity or outside
/// it. The one way to generate: the IDE's Generate, `clarity gen` and the
/// agents' generate tool all come here.
pub fn generate(file: &Path, out: Option<&Path>) -> Result<Source, String> {
    if !file.is_file() {
        return Err(format!("{} is not a project file; give one like shop.clarity", file.display()));
    }
    let dir = clarity_project::module_dir(file);
    // Each web API's module follows its description and picks (manual/code.web-apis.md).
    clarity_project::refresh_apis(file)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    // Each native part's header follows its declaration, and a new part gets a starter (manual/design.custom-parts.md).
    let parts = clarity_gen::parts::write_part_folders(&dir, &project).map_err(|err| format!("cannot write the parts' folders: {err}"))?;
    let mut source = transpile(file, out)?;
    source.parts = parts;
    source.written = clarity_gen::write_programs_listed(&source.out, &source.programs, source.settings.stack.backend, &source.settings.keeps).map_err(|err| format!("cannot write {}: {err}", source.out.display()))?;
    // With `include migrate`, Clarity's data migration tools beside the programs (docs/ledger/migrations.md).
    let built = clarity_project::build_settings(&dir, &source.settings);
    source.tools = clarity_gen::write_migration_tools(&source.out, &project, &built, &clarity_gen::clarity_programs_folder())?;
    source.written.written.extend(source.tools.iter().cloned());
    Ok(source)
}

/// The project transpiled in memory, nothing written: its output folder (`out`, else its own), settings and programs.
fn transpile(file: &Path, out: Option<&Path>) -> Result<Source, String> {
    let dir = clarity_project::module_dir(file);
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let loaded = clarity_project::load(file);
    let shown = file.file_name().unwrap_or_default().to_string_lossy().to_string();
    let settings = match (loaded.settings, loaded.problems.first()) {
        (Some(settings), None) => settings,
        (_, Some(problem)) => return Err(format!("{shown}: {problem}")),
        (None, None) => return Err(format!("{shown} cannot be read")),
    };
    let built = clarity_project::build_settings(&dir, &settings);
    // A project with errors is not generated: the checker's errors say where, then what generating found.
    let mut programs = clarity_gen::programs(&project, &built).map_err(|errors| {
        let mut why: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.to_string()).collect();
        why.extend(errors);
        why.join("\n")
    })?;
    // A program served to a browser carries Clarity.UI's browser files (round B of manual/start.how-it-works.md).
    if clarity_gen::web_browser::needs_library(&programs) {
        clarity_gen::web_browser::with_library(&mut programs, &browser_folder()?)?;
    }
    let out = out.map_or_else(|| clarity_project::out_dir(file, &settings), Path::to_path_buf);
    let formats = project.uses_formats();
    // The databases the app owns with their steps, which Run migrates first (the app does not, docs/ledger/migrations.md).
    let databases = clarity_gen::migrated_databases(&project, &built).map_err(|errors| errors.join("; "))?;
    Ok(Source { out, settings, programs, shown, written: clarity_gen::Written::default(), parts: Vec::new(), tools: Vec::new(), formats, databases })
}

/// A project's source tree as it was written: its output folder, its settings and programs.
pub struct Source {
    pub out: PathBuf,
    pub settings: clarity_project::Settings,
    pub programs: Vec<clarity_gen::Program>,
    /// The project file's name, as messages name it.
    shown: String,
    /// What Generate wrote and removed in the output folder; nothing for a transpile alone.
    pub written: clarity_gen::Written,
    /// The part folders' files Generate wrote (manual/design.custom-parts.md).
    pub parts: Vec<PathBuf>,
    /// Clarity's data migration tools it put beside the programs, with `include migrate`.
    pub tools: Vec<String>,
    /// Whether the code reads or writes a format, so a Go, C or C++ program of any target links Clarity.Formats.
    pub formats: bool,
    /// The databases the app owns, with every step, as clarity-migrate migrates them.
    pub databases: Vec<clarity_gen::MigratedDatabase>,
}

impl Source {
    /// What generating did, a line each, as the terminal and the agents read it.
    pub fn lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self.parts.iter().map(|path| format!("wrote {}", path.display())).collect();
        lines.extend(self.tools.iter().map(|name| format!("wrote {}", self.out.join(name).display())));
        for program in &self.programs {
            lines.push(format!("{} file(s) in {}", program.files.len(), clarity_project::target_dir(&self.out, program.target).display()));
        }
        lines.push(format!("{} changed", self.written.written.len()));
        lines
    }
}

/// What Build and Run build: the source Generate wrote, which must be the
/// project as it is now; they generate nothing themselves. A project not
/// generated, or changed since, is refused with `generate`, what to do
/// ([`GENERATE_IN_THE_IDE`], [`GENERATE_IN_THE_TERMINAL`]).
pub fn generate_out(file: &Path, release: bool, cli: bool, generate: &str) -> Result<Generated, String> {
    let Source { out, settings, programs, shown, formats, databases, .. } = transpile(file, None)?;
    // Each program is in its own folder of the output; one without windows has only the command line.
    let cli = cli || !settings.stack.has_ui();
    if cli && !settings.stack.has(clarity_project::Target::Cli) {
        return Err(format!("{shown} has no command-line program; add the cli target to the stack"));
    }
    let target = if cli { clarity_project::Target::Cli } else { clarity_gen::windowed_target(&settings) };
    let toolchain_of = |release: bool| if cli { clarity_gen::cli_toolchain(&settings, release) } else { clarity_gen::toolchain(&settings, release) };
    let toolchain = toolchain_of(release).ok_or_else(|| format!("{shown}: Clarity cannot build this stack yet"))?;
    let out = clarity_project::target_dir(&out, target);
    let fingerprint = programs.iter().find(|p| p.target == target).map(clarity_gen::fingerprint).unwrap_or_default();
    match clarity_gen::last_generated(&out) {
        None => return Err(format!("{shown} is not generated yet: {generate}")),
        Some(generated) if generated != fingerprint => return Err(format!("{shown} changed since it was generated: {generate}")),
        Some(_) => {}
    }
    let built = clarity_gen::last_build(&out, &fingerprint)
        .and_then(|release| toolchain_of(release).map(|t| (t, release)))
        .filter(|(t, _)| out.join(&t.program).exists());
    // Clarity.UI's folder: a desktop program draws with it, and a program whose code uses a format links Clarity.Formats from it.
    let library = (!cli && clarity_gen::uses_clarity_ui(&settings)) || (formats && matches!(settings.stack.backend, clarity_project::Backend::Go | clarity_project::Backend::C | clarity_project::Backend::Cpp));
    let data = clarity_project::data_folder(&clarity_project::module_dir(file), &settings);
    let databases = placed(file, &settings, databases);
    Ok(Generated { out, data, toolchain, stack: settings.stack, fingerprint, release, built, library, databases })
}

/// Each of the project's databases as clarity-migrate migrates it for Run and
/// `clarity data migrate`: an SQLite one where the program opens it when Run
/// names the project's data folder, backed up into its folder in the
/// project's backups folder (`clarity_project::backups::folder_of`), or not at
/// all when the project takes no backups; a server's as the program connects to it.
fn placed(file: &Path, settings: &clarity_project::Settings, databases: Vec<clarity_gen::MigratedDatabase>) -> Vec<clarity_migrate::Database> {
    let dir = clarity_project::module_dir(file);
    let data = clarity_project::data_folder(&dir, settings);
    databases
        .into_iter()
        .map(|d| {
            let backups = if d.dialect == "sqlite" { clarity_project::backups::folder_of(&dir, settings, &d.source).map(|f| f.display().to_string()) } else { None };
            clarity_migrate::Database { source: d.source, dialect: d.dialect, location: d.location, steps: d.steps, backups }
        })
        .map(|d| match &data {
            Some(folder) => clarity_migrate::in_data_folder(&d, folder),
            None => d,
        })
        .collect()
}

/// The project's databases as Run migrates them (`placed`), for
/// `clarity data migrate`; a project with errors is refused, its errors first.
pub fn databases(file: &Path) -> Result<Vec<clarity_migrate::Database>, String> {
    let dir = clarity_project::module_dir(file);
    let loaded = clarity_project::load(file);
    let shown = file.file_name().unwrap_or_default().to_string_lossy().to_string();
    let settings = match (loaded.settings, loaded.problems.first()) {
        (Some(settings), None) => settings,
        (_, Some(problem)) => return Err(format!("{shown}: {problem}")),
        (None, None) => return Err(format!("{shown} cannot be read")),
    };
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    if project.has_errors() {
        let mut why: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.to_string()).collect();
        why.push("the project has errors; fix them before migrating".to_string());
        return Err(why.join("\n"));
    }
    let built = clarity_project::build_settings(&dir, &settings);
    let databases = clarity_gen::migrated_databases(&project, &built).map_err(|errors| errors.join("; "))?;
    Ok(placed(file, &settings, databases))
}

/// Where a program served to a browser alone listens when Run starts it: the
/// project's port; none for a desktop or command-line program.
pub fn serves(file: &Path, stack: &clarity_project::Stack, cli: bool) -> Option<String> {
    (!cli && stack.has(clarity_project::Target::Web) && !stack.has(clarity_project::Target::Desktop)).then(|| clarity_project::load(file).settings.map(|s| clarity_project::serve_address(&s))).flatten()
}

/// Why the generated project in `out` cannot be built on this machine: the
/// tools of its stack that are missing; none when they are all there.
pub fn cannot_build(tools: &clarity_gen::tools::StackTools, out: &Path) -> Option<String> {
    (!tools.missing.is_empty()).then(|| {
        format!(
            "the project is generated in {}; to build {} here, install {} (the manual's page Setting up the build tools says how)",
            out.display(),
            clarity_gen::tools::stack_name(tools.backend, tools.ui),
            tools.missing.join(", and ")
        )
    })
}

/// Whether this machine has the tools to build `generated`; what to install when it has not.
pub fn ready(generated: &Generated) -> Result<(), String> {
    let tools = clarity_gen::tools::check(generated.stack.backend, generated.stack.ui, &clarity_gen::tools::System);
    match cannot_build(&tools, &generated.out) {
        Some(why) => Err(why),
        None => Ok(()),
    }
}

/// The toolchain of `generated`, with Clarity.UI's library for a Rust, Go,
/// C or C++ desktop program (manual/start.how-it-works.md): the folder CLARITY_UI names, else the
/// one fetched from the library's release into the cache folder, fetched
/// now when it is not there yet.
/// With the runtimes the installed Clarity carries, for when the internet
/// does not answer (`with_carried`).
pub fn toolchain(generated: &Generated, say: Say) -> Result<clarity_gen::Toolchain, String> {
    let toolchain = with_carried(generated.toolchain.clone(), generated.stack.backend, &generated.out, say);
    if !generated.library {
        return Ok(toolchain);
    }
    let folder = library_folder(say)?;
    Ok(clarity_gen::with_clarity_ui(toolchain, generated.stack.backend, &folder))
}

/// `toolchain` with the runtimes the installed Clarity carries
/// (docs/ledger/offline.md): the internet first; only when the stack's
/// registry does not answer, its carried runtimes, unpacked into the cache
/// folder the first time, said in the build's output.
fn with_carried(toolchain: clarity_gen::Toolchain, backend: clarity_project::Backend, out: &Path, say: Say) -> clarity_gen::Toolchain {
    let (Some(carried), Some(registry)) = (clarity_gen::carried::carried(), clarity_gen::carried::registry(backend)) else { return toolchain };
    if answers(registry) {
        return toolchain;
    }
    let Some(cache) = clarity_gen::clarity_ui::cache_dir() else { return toolchain };
    say(format!("No answer from {registry}: building with the runtimes Clarity carries"));
    match clarity_gen::carried::unpacked(carried, backend, &cache) {
        Ok(folder) => {
            say(format!("The runtimes are in {}", folder.display()));
            clarity_gen::carried::offline(toolchain, backend, &folder, out)
        }
        Err(why) => {
            say(why);
            toolchain
        }
    }
}

/// Whether `url` answers at all within a few seconds.
pub fn answers(url: &str) -> bool {
    off_runtime(|| {
        let Ok(client) = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(5)).build() else { return false };
        client.get(url).send().is_ok()
    })
}

/// Runs `work` on a thread of its own: the blocking client panics on an
/// async runtime's threads, where the IDE's DevTools tab commands run.
fn off_runtime<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|s| s.spawn(work).join()).unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

/// The folder of Clarity.UI's library for a build: the one CLARITY_UI names,
/// the one Clarity carries, else fetched now into the cache folder.
pub fn library_folder(say: Say) -> Result<PathBuf, String> {
    let library = clarity_gen::clarity_ui::library();
    let folder = clarity_gen::clarity_ui::provide(library, clarity_gen::clarity_ui::RELEASES, &download, &|line| say(line))?;
    keep_openh264(&folder);
    Ok(folder)
}

/// Makes sure the library's folder holds Cisco's openh264, which every video
/// app carries (docs/ledger/video-camera.md): in the background, never holding a build
/// up, and quietly offline. A folder Clarity carries already has it; one
/// fetched before it did, or Clarity's own repository build, gets it from Cisco.
fn keep_openh264(folder: &Path) {
    let folder = folder.to_path_buf();
    std::thread::spawn(move || {
        if let Err(why) = clarity_gen::clarity_ui::ensure_openh264(&folder, std::env::consts::OS, std::env::consts::ARCH, &download) {
            eprintln!("openh264 for video apps: {why}");
        }
    });
}

/// The folder of Clarity.UI's browser files for a web program: the one
/// CLARITY_UI_BROWSER names, the one Clarity carries, else fetched now from
/// the library's release into the cache folder.
pub fn browser_folder() -> Result<PathBuf, String> {
    let library = clarity_gen::clarity_ui::browser_library();
    clarity_gen::clarity_ui::provide(library, clarity_gen::clarity_ui::RELEASES, &download, &|line| eprintln!("{line}"))
}

/// The largest download Clarity takes: an archive of Clarity.UI's library is some tens of MB.
const DOWNLOAD_LIMIT: u64 = 512 * 1024 * 1024;

/// The bytes at `url`, over HTTPS: the network part of fetching Clarity.UI's library.
pub fn download(url: &str) -> Result<Vec<u8>, String> {
    off_runtime(|| {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .map_err(|err| format!("cannot download {url}: {err}"))?;
        let response = client.get(url).send().map_err(|err| format!("cannot download {url}: {err}"))?;
        if !response.status().is_success() {
            return Err(format!("cannot download {url}: {}", response.status()));
        }
        clarity_gen::clarity_ui::read_limited(response, DOWNLOAD_LIMIT)
    })
}

/// Builds `generated` with `toolchain`: the stack's build tool in its folder,
/// its output said line by line; then, for Go, the libraries beside the
/// program (as C's, C++'s and Rust's builds put them); and records what the
/// build was made from. Returns the program.
pub fn compile(generated: &Generated, toolchain: &clarity_gen::Toolchain, say: Say) -> Result<PathBuf, String> {
    let out = generated.out.as_path();
    build_with(out, toolchain, say)?;
    // Go's build has no step of its own after `go build`: the libraries go beside the program here,
    // as C's, C++'s and Rust's builds put them (on Windows the program finds them only there).
    if generated.library && generated.stack.backend == clarity_project::Backend::Go {
        let folder = library_folder(say)?;
        clarity_gen::libraries_beside(&folder, &out.join(&toolchain.program), std::env::consts::OS).map_err(|err| format!("cannot put the libraries beside the program: {err}"))?;
    }
    let _ = clarity_gen::record_build(out, &generated.fingerprint, generated.release);
    Ok(out.join(&toolchain.program))
}

/// Runs the stack's build tool in `out`, its output said line by line.
fn build_with(out: &Path, toolchain: &clarity_gen::Toolchain, say: Say) -> Result<(), String> {
    let mut command = clarity_project::background_command(clarity_gen::tools::Probe::find(&clarity_gen::tools::System, toolchain.tool).unwrap_or_else(|| toolchain.tool.into()));
    command
        .args(&toolchain.args)
        // The tool starts others (go runs gcc and pkg-config): they look where Clarity looks.
        .env("PATH", clarity_gen::tools::tool_path())
        // The C and C++ compilers chosen in the Environment settings, for CMake.
        .envs(clarity_gen::tools::tool_env())
        .envs(toolchain.env.iter().cloned())
        .current_dir(out)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // The command first, so the output says what runs where.
    say(format!("{} {} (in {})", toolchain.tool, toolchain.args.join(" "), out.display()));
    let mut child = command.spawn().map_err(|err| format!("cannot start {}: {err}; {}", toolchain.tool, toolchain.missing))?;
    // cargo reports on stderr, dotnet on stdout: both are said.
    let (stdout, stderr) = (child.stdout.take(), child.stderr.take());
    std::thread::scope(|s| {
        if let Some(stdout) = stdout {
            s.spawn(|| say_lines(stdout, say));
        }
        if let Some(stderr) = stderr {
            say_lines(stderr, say);
        }
    });
    match child.wait() {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err("the build failed; see the output".to_string()),
        Err(err) => Err(format!("{} stopped: {err}", toolchain.tool)),
    }
}

/// Says each line `from` gives.
pub fn say_lines(from: impl Read, say: Say) {
    use std::io::BufRead;
    for line in std::io::BufReader::new(from).lines().map_while(Result::ok) {
        say(line);
    }
}

/// Migrates the program's databases before Run starts it (the program does
/// not migrate, docs/ledger/migrations.md), with clarity-migrate.
pub fn migrate(generated: &Generated, say: Say) -> Result<(), String> {
    migrate_databases(&generated.databases, say)
}

/// Migrates each of `databases` with clarity-migrate, in order, stopping at
/// the first that cannot be: Run and `clarity data migrate` both come here.
pub fn migrate_databases(databases: &[clarity_migrate::Database], say: Say) -> Result<(), String> {
    for database in databases {
        match clarity_migrate::migrate(database) {
            Ok(clarity_migrate::Outcome::Migrated(n)) => say(format!("{} migrated by {n} step{}", database.source, if n == 1 { "" } else { "s" })),
            Ok(clarity_migrate::Outcome::UpToDate) => say(format!("{} is up to date", database.source)),
            Err(why) => return Err(format!("cannot migrate {}: {why}", database.source)),
        }
    }
    Ok(())
}
