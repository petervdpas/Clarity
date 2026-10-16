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
}

#[derive(Subcommand)]
enum SpecCommand {
    /// Parse and check the modules of a project.
    Check {
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
        },
        Command::Gen { project, out } => generate(&project, out.as_deref()),
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
    let files = match clarity_gen::generate(&project, &settings) {
        Ok(files) => files,
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
    match clarity_gen::write_out(out, &files, &clarity_gen::BUILD_OUTPUT) {
        Ok(written) => {
            eprintln!("{} file(s) in {}, {written} changed", files.len(), out.display());
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
    clarity_spec::load_dir(&dir).map_err(|err| {
        eprintln!("clarity: cannot read {}: {err}", dir.display());
        ExitCode::from(2)
    })
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
