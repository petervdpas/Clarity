//! Clarity's migration tool (docs/ledger/migrations.md, Design): it applies the
//! steps a database has not had, as conformance/migrations.json gives them
//! for every database kind, and refuses a database ahead of them.

use clarity_migrate::{Database, Outcome, migrate, state};

fn cases() -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/migrations.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The steps of `kind` in the cases: each a list of statements.
fn steps(kind: &str) -> Vec<Vec<String>> {
    serde_json::from_value(cases()["steps"][kind].clone()).unwrap()
}

/// A fresh SQLite file of its own for `test`.
fn fresh(test: &str) -> String {
    let folder = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("clarity-migrate").join(test);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder.join("main.db").display().to_string()
}

fn sqlite(test: &str, steps: Vec<Vec<String>>) -> Database {
    Database { source: "main".into(), dialect: "sqlite".into(), location: fresh(test), steps, backups: None }
}

#[test]
fn a_new_database_gets_every_step_once() {
    let database = sqlite("every_step", steps("sqlite"));
    assert_eq!(migrate(&database), Ok(Outcome::Migrated(2)));
    assert_eq!(state(&database), Ok((2, 2)));
    assert_eq!(migrate(&database), Ok(Outcome::UpToDate), "a database with every step is left as it is");
}

#[test]
fn a_database_behind_gets_only_the_steps_it_lacks() {
    let all = steps("sqlite");
    let first = Database { steps: all[..1].to_vec(), ..sqlite("behind", Vec::new()) };
    assert_eq!(migrate(&first), Ok(Outcome::Migrated(1)));
    let later = Database { steps: all, ..first.clone() };
    assert_eq!(state(&later), Ok((1, 2)));
    assert_eq!(migrate(&later), Ok(Outcome::Migrated(1)));
}

#[test]
fn a_database_ahead_of_the_app_is_refused_and_left_as_it_is() {
    let all = steps("sqlite");
    let newer = sqlite("ahead", all.clone());
    migrate(&newer).unwrap();
    let older = Database { steps: all[..1].to_vec(), ..newer.clone() };
    let refused = migrate(&older).unwrap_err();
    assert!(refused.contains("more than the 1 this app was made with"), "{refused}");
    assert_eq!(state(&newer), Ok((2, 2)), "nothing changed");
}

#[test]
fn a_database_kind_clarity_does_not_know_is_said() {
    let odd = Database { dialect: "oracle".into(), ..sqlite("odd", Vec::new()) };
    assert!(migrate(&odd).unwrap_err().contains("there is no database kind 'oracle'"));
}

/// The server databases, where a test server is named (CLARITY_TEST_POSTGRES,
/// CLARITY_TEST_MYSQL, CLARITY_TEST_MSSQL), as the runtimes' server tests do.
#[test]
fn a_server_database_gets_its_steps_where_a_test_server_is_named() {
    for (variable, kind) in [("CLARITY_TEST_POSTGRES", "postgres"), ("CLARITY_TEST_MYSQL", "mysql"), ("CLARITY_TEST_MSSQL", "mssql")] {
        let Some(at) = std::env::var(variable).ok().filter(|v| !v.is_empty()) else { continue };
        let database = Database { source: "main".into(), dialect: kind.into(), location: at, steps: steps(kind), backups: None };
        let (had, wanted) = state(&database).unwrap();
        assert!(had <= wanted, "{kind}: the test database is ahead of the cases");
        migrate(&database).unwrap();
        assert_eq!(state(&database), Ok((wanted, wanted)), "{kind}");
    }
}

/// What the generator gives for a project is what the tool reads.
#[test]
fn the_generator_s_databases_are_what_the_tool_migrates() {
    let project = clarity_spec::load_converted([("shop.clarity", "module shop\n\nentity Customer\n    name: text\n    email: email\n")]);
    let location = fresh("generated");
    let settings = clarity_project::parse(&format!("project Shop\n    stack rust desktop\n    datasource main sqlite \"{location}\"\n")).unwrap();
    let written = clarity_gen::migrated_databases(&project, &settings).unwrap();
    let databases: Vec<Database> = serde_json::from_str(&serde_json::to_string(&written).unwrap()).unwrap();
    assert_eq!(databases.len(), 1);
    assert_eq!((databases[0].source.as_str(), databases[0].dialect.as_str(), databases[0].location.as_str()), ("main", "sqlite", location.as_str()));
    assert_eq!(migrate(&databases[0]), Ok(Outcome::Migrated(databases[0].steps.len())));
}

/// Beside an app: the program reads `clarity-migrate.json` beside it, as the
/// generator writes it for the settings the app is built with, and finds the
/// SQLite database where the app does (here CLARITY_DATA, as Clarity's Run names it).
#[test]
fn the_program_migrates_the_app_s_databases_from_the_file_beside_it() {
    assert_eq!(clarity_gen::MIGRATE_FILE, clarity_migrate::MIGRATE_FILE);
    let folder = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("clarity-migrate").join("beside");
    let _ = std::fs::remove_dir_all(&folder);
    let (app, data) = (folder.join("app"), folder.join("data"));
    std::fs::create_dir_all(&app).unwrap();
    std::fs::create_dir_all(&data).unwrap();
    let project = clarity_spec::load_converted([("shop.clarity", "module shop\n\nentity Customer\n    name: text\n")]);
    let settings = clarity_project::parse("project Shop\n    stack go desktop\n    datasource main sqlite \"data/shop.db\"\n").unwrap();
    let built = clarity_project::build_settings(&folder, &settings);
    let file = clarity_gen::migrate_file(&project, &built).unwrap();
    assert!(file.text.contains("\"location\": \"data:shop/shop.db\""), "{}", file.text);
    std::fs::write(app.join(&file.path), &file.text).unwrap();
    let program = app.join(format!("clarity-migrate{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(env!("CARGO_BIN_EXE_clarity-migrate"), &program).unwrap();
    let run = |args: &[&str]| std::process::Command::new(&program).args(args).env("CLARITY_DATA", &data).output().unwrap();
    let checked = run(&["--check"]);
    assert!(!checked.status.success() && String::from_utf8_lossy(&checked.stdout).contains("main has 0 of 1 steps"), "{checked:?}");
    let migrated = run(&[]);
    assert!(migrated.status.success() && String::from_utf8_lossy(&migrated.stdout).contains("main migrated by 1 step"), "{migrated:?}");
    assert!(data.join("shop.db").is_file(), "the database is where the app finds it");
    let again = run(&["--check"]);
    assert!(again.status.success() && String::from_utf8_lossy(&again.stdout).contains("main is up to date (1 steps)"), "{again:?}");
}

/// The installed app's `clarity-migrate.json` names each SQLite database's
/// backups folder: the project's `backups` line with the data source's name,
/// in the folder the database opens in; none without the line, so whoever
/// installs the app migrates without a backup as the project does.
#[test]
fn the_file_beside_the_app_names_the_backups_folder_of_the_project() {
    let project = clarity_spec::load_converted([("shop.clarity", "module shop\n\nentity Customer\n    name: text\n")]);
    let folder = std::path::Path::new("/projects/shop");
    let with = clarity_project::parse("project Shop\n    stack go desktop\n    backups \"data/backups\"\n    datasource main sqlite \"data/shop.db\"\n").unwrap();
    let file = clarity_gen::migrate_file(&project, &clarity_project::build_settings(folder, &with)).unwrap();
    let databases: Vec<Database> = serde_json::from_str(&file.text).unwrap();
    assert_eq!(databases[0].backups.as_deref(), Some("data/backups/main"), "{}", file.text);
    let without = clarity_project::parse("project Shop\n    stack go desktop\n    datasource main sqlite \"data/shop.db\"\n").unwrap();
    let file = clarity_gen::migrate_file(&project, &clarity_project::build_settings(folder, &without)).unwrap();
    assert!(!file.text.contains("backups"), "{}", file.text);
}

/// Run in Clarity: the database the app opens with CLARITY_DATA naming the project's data folder.
#[test]
fn a_database_is_found_in_the_data_folder_as_the_app_finds_it_there() {
    let kept = Database { source: "main".into(), dialect: "sqlite".into(), location: "data:shop/shop.db".into(), steps: Vec::new(), backups: None };
    let folder = std::path::Path::new("/projects/shop/data");
    assert_eq!(clarity_migrate::in_data_folder(&kept, folder).location, folder.join("shop.db").display().to_string());
    let server = Database { dialect: "postgres".into(), location: "host=db".into(), ..kept };
    assert_eq!(clarity_migrate::in_data_folder(&server, folder), server);
}

/// One SQLite database with its first step: customers that refer to each other.
const CUSTOMERS: &str = "CREATE TABLE customer (id INTEGER PRIMARY KEY, name TEXT NOT NULL, referred_by INTEGER REFERENCES customer (id))";

fn customers(test: &str, more: &[&str]) -> Database {
    let mut steps = vec![vec![CUSTOMERS.to_string()]];
    steps.extend(more.iter().map(|s| vec![s.to_string()]));
    sqlite(test, steps)
}

fn sql(database: &Database, query: &str) -> Vec<Vec<clarity_runtime::SqlValue>> {
    clarity_runtime::Db::open(std::path::Path::new(&database.location)).unwrap().rows("", query, &[]).unwrap()
}

#[test]
fn a_failed_step_changes_nothing() {
    let first = customers("failed", &[]);
    migrate(&first).unwrap();
    let broken = Database { steps: customers("failed", &["ALTER TABLE customer ADD COLUMN a TEXT; THIS IS NOT SQL;"]).steps, ..first.clone() };
    assert!(migrate(&broken).is_err());
    assert_eq!(state(&broken), Ok((1, 2)));
    assert!(clarity_runtime::Db::open(std::path::Path::new(&first.location)).unwrap().batch("", "SELECT a FROM customer").is_err(), "the column of the failed step is not there");
}

#[test]
fn a_step_can_build_a_referenced_table_again() {
    let first = customers("rebuild", &[]);
    migrate(&first).unwrap();
    clarity_runtime::Db::open(std::path::Path::new(&first.location)).unwrap().batch("", "INSERT INTO customer (id, name) VALUES (1, 'Ada'); INSERT INTO customer (id, name, referred_by) VALUES (2, 'Grace', 1);").unwrap();
    let rebuild = "CREATE TABLE customer__new (id INTEGER PRIMARY KEY, name TEXT NOT NULL, referred_by INTEGER REFERENCES customer (id));
        INSERT INTO customer__new (id, name, referred_by) SELECT id, name, referred_by FROM customer;
        DROP TABLE customer;
        ALTER TABLE customer__new RENAME TO customer;";
    let later = Database { steps: vec![vec![CUSTOMERS.into()], vec![rebuild.into()]], ..first.clone() };
    assert_eq!(migrate(&later), Ok(Outcome::Migrated(1)));
    assert_eq!(sql(&later, "SELECT referred_by FROM customer WHERE id = 2"), vec![vec![clarity_runtime::SqlValue::Integer(1)]]);
}

#[test]
fn a_step_that_breaks_references_changes_nothing() {
    let first = customers("dangling", &[]);
    migrate(&first).unwrap();
    clarity_runtime::Db::open(std::path::Path::new(&first.location)).unwrap().batch("", "INSERT INTO customer (id, name) VALUES (1, 'Grace')").unwrap();
    let dangling = Database { steps: vec![vec![CUSTOMERS.into()], vec!["UPDATE customer SET referred_by = 99".into()]], ..first.clone() };
    let refused = migrate(&dangling).unwrap_err();
    assert!(refused.contains("leaves 1 reference(s) to records that do not exist"), "{refused}");
    assert_eq!(state(&dangling), Ok((1, 2)));
    assert_eq!(sql(&dangling, "SELECT referred_by FROM customer"), vec![vec![clarity_runtime::SqlValue::Null]]);
}

/// A database with a schema, then a second step to apply.
fn behind(test: &str, backups: Option<&str>) -> (Database, std::path::PathBuf) {
    let first = Database { steps: vec![vec!["CREATE TABLE item (id INTEGER PRIMARY KEY, name TEXT NOT NULL DEFAULT '')".into()]], backups: backups.map(String::from), ..sqlite(test, Vec::new()) };
    let dir = std::path::Path::new(&first.location).parent().unwrap().to_path_buf();
    migrate(&first).unwrap();
    clarity_runtime::Db::open(std::path::Path::new(&first.location)).unwrap().batch("", "INSERT INTO item (name) VALUES ('kept')").unwrap();
    let mut steps = first.steps.clone();
    steps.push(vec!["ALTER TABLE item ADD COLUMN price INTEGER NOT NULL DEFAULT 0".into()]);
    (Database { steps, ..first }, dir)
}

/// The project's backups folder (`backups "data/backups"`) holds a folder per
/// data source; clarity-migrate backs up into the one it is given, a relative
/// one in the database's own folder (docs/ledger/backups.md).
#[test]
fn a_database_with_a_schema_is_backed_up_into_the_folder_given_before_new_steps_and_a_new_one_is_not() {
    let (both, dir) = behind("backup", Some("backups/main"));
    assert!(!dir.join("backups").exists(), "a new database needs no backup");
    assert_eq!(migrate(&both), Ok(Outcome::Migrated(1)));
    let backups: Vec<_> = std::fs::read_dir(dir.join("backups/main")).unwrap().flatten().map(|e| e.path()).collect();
    assert_eq!(1, backups.len());
    let name = backups[0].file_name().unwrap().to_string_lossy().to_string();
    assert!(name.starts_with("main-") && name.ends_with("-before-migration-0002.db"), "{name}");
    // The backup holds the data and the schema as they were before step 2.
    let copy = clarity_runtime::Db::open(&backups[0]).unwrap();
    assert_eq!(1, copy.schema_version().unwrap());
    assert!(copy.batch("", "SELECT price FROM item").is_err(), "step 2 is not in the backup");
    drop(copy);
    // Nothing to apply: no backup.
    assert_eq!(migrate(&both), Ok(Outcome::UpToDate));
    assert_eq!(1, std::fs::read_dir(dir.join("backups/main")).unwrap().count());
    // A folder written out in full is used as it is.
    let elsewhere = dir.join("elsewhere");
    let (full, _) = behind("backup_full", Some(&elsewhere.display().to_string()));
    assert_eq!(migrate(&full), Ok(Outcome::Migrated(1)));
    assert_eq!(1, std::fs::read_dir(&elsewhere).unwrap().count());
}

/// A project without a backups folder takes no backups: clarity-migrate migrates without one.
#[test]
fn a_database_without_a_backups_folder_is_migrated_without_a_backup() {
    let (both, dir) = behind("no_backup", None);
    assert_eq!(migrate(&both), Ok(Outcome::Migrated(1)));
    let left: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|f| f != "main.db").collect();
    assert!(left.is_empty(), "nothing but the database: {left:?}");
}
