//! `clarity-migrate [databases.json] [--check]`: migrates each database the
//! file lists (`clarity_migrate::Database`, as the generator writes them;
//! without a file, `clarity-migrate.json` beside the program, as an app made
//! with Clarity carries it), or with `--check` only says how far each one is.
//! It exits with 1 when a database could not be migrated, or with `--check`
//! when one is behind or ahead. It speaks English, as Clarity's tools do.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("usage: clarity-migrate [databases.json] [--check]\n  without a file, clarity-migrate.json beside this program");
        return ExitCode::SUCCESS;
    }
    let file = match args.iter().find(|a| !a.starts_with("--")) {
        Some(given) => std::path::PathBuf::from(given),
        None => match clarity_migrate::beside() {
            Ok(path) => path,
            Err(why) => {
                eprintln!("clarity-migrate: {why}");
                return ExitCode::from(2);
            }
        },
    };
    let databases = match clarity_migrate::read(&file) {
        Ok(databases) => databases,
        Err(why) => {
            eprintln!("clarity-migrate: {why}");
            return ExitCode::from(2);
        }
    };
    let mut failed = false;
    for database in &databases {
        if check {
            match clarity_migrate::state(database) {
                Ok((had, wanted)) if had == wanted => println!("{} is up to date ({had} steps)", database.source),
                Ok((had, wanted)) => {
                    println!("{} has {had} of {wanted} steps", database.source);
                    failed = true;
                }
                Err(why) => {
                    eprintln!("clarity-migrate: {why}");
                    failed = true;
                }
            }
            continue;
        }
        match clarity_migrate::migrate(database) {
            Ok(clarity_migrate::Outcome::UpToDate) => println!("{} is up to date", database.source),
            Ok(clarity_migrate::Outcome::Migrated(n)) => println!("{} migrated by {n} step{}", database.source, if n == 1 { "" } else { "s" }),
            Err(why) => {
                eprintln!("clarity-migrate: {why}");
                failed = true;
            }
        }
    }
    if failed { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
