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
}

#[derive(Subcommand)]
enum SpecCommand {
    /// Parse and check all spec files in a directory.
    Check {
        /// Directory with .clarity files.
        #[arg(default_value = "spec")]
        dir: PathBuf,
    },
    /// Print the checked model as JSON.
    Model {
        #[arg(default_value = "spec")]
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
    }
}

fn load(dir: &std::path::Path) -> Result<clarity_spec::Project, ExitCode> {
    clarity_spec::load_dir(dir).map_err(|err| {
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
