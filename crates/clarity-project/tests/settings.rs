use clarity_project::{
    Auth, AuthKind, Backend, DataSource, PROJECT_FILE, Settings, SourceKind, Stack, Target, Ui, catalog, load, parse,
    starter, validate, write,
};

const SHOP: &str = r#"# The shop example.
[project]
name = "Shop"

[stack]
backend = "csharp"   # C# with Avalonia
ui = "avalonia"
targets = ["desktop"]

[[datasource]]
name = "main"
kind = "postgres"
url = "env:SHOP_DB_URL"

[[datasource]]
name = "payments"   # the payment provider
kind = "rest"
url = "https://api.example.com/v2"
auth = { kind = "bearer", secret = "keychain:payments" }
"#;

fn source(name: &str, kind: SourceKind, url: &str) -> DataSource {
    DataSource {
        name: name.into(),
        kind,
        url: url.into(),
        auth: None,
    }
}

fn shop() -> Settings {
    Settings {
        name: "Shop".into(),
        stack: Stack {
            backend: Backend::Csharp,
            ui: Ui::Avalonia,
            targets: vec![Target::Desktop],
        },
        datasources: vec![
            source("main", SourceKind::Postgres, "env:SHOP_DB_URL"),
            DataSource {
                auth: Some(Auth {
                    kind: AuthKind::Bearer,
                    secret: "keychain:payments".into(),
                }),
                ..source("payments", SourceKind::Rest, "https://api.example.com/v2")
            },
        ],
    }
}

#[test]
fn reads_a_project_file() {
    assert_eq!(parse(SHOP).unwrap(), shop());
}

#[test]
fn names_the_expected_values_for_an_unknown_one() {
    let err = parse(&SHOP.replace("\"csharp\"", "\"java\"")).unwrap_err();
    assert!(err.contains("java") && err.contains("csharp"), "{err}");
}

#[test]
fn refuses_unknown_keys() {
    let err = parse(&SHOP.replace("ui = ", "frontend = \"svelte\"\nui = ")).unwrap_err();
    assert!(err.contains("frontend"), "{err}");
}

#[test]
fn a_valid_project_has_no_problems() {
    assert_eq!(validate(&shop()), Vec::<String>::new());
    assert_eq!(validate(&starter("Shop", "shop")), Vec::<String>::new());
}

fn problems_of(change: impl FnOnce(&mut Settings)) -> Vec<String> {
    let mut settings = shop();
    change(&mut settings);
    validate(&settings)
}

#[test]
fn a_native_toolkit_needs_its_backend() {
    let problems = problems_of(|s| s.stack.backend = Backend::Rust);
    assert_eq!(problems, ["Avalonia is a UI for the C# backend, not for Rust"]);
}

#[test]
fn needs_a_name_and_a_target() {
    let problems = problems_of(|s| {
        s.name = " ".into();
        s.stack.targets.clear();
    });
    assert_eq!(problems, ["the project needs a name", "the project needs at least one target"]);
    let problems = problems_of(|s| s.stack.targets = vec![Target::Web, Target::Web]);
    assert_eq!(problems, ["the target web is listed twice"]);
}

#[test]
fn data_source_names_are_identifiers_and_unique() {
    let problems = problems_of(|s| s.datasources[1].name = "main".into());
    assert_eq!(problems, ["there are two data sources named 'main'"]);
    let problems = problems_of(|s| s.datasources[1].name = "pay ments".into());
    assert_eq!(problems, ["'pay ments' is not a valid data source name"]);
}

#[test]
fn secrets_stay_out_of_the_project_file() {
    let problems = problems_of(|s| s.datasources[0].url = "postgres://shop:hunter2@localhost/shop".into());
    assert_eq!(
        problems,
        ["data source 'main': the connection contains a password; put it in an environment variable (env:NAME) or the keychain (keychain:NAME)"]
    );
    let problems = problems_of(|s| s.datasources[0].url = "Server=db;Database=shop;Password=hunter2".into());
    assert_eq!(problems.len(), 1, "{problems:?}");
    // A connection without a password is fine.
    assert!(problems_of(|s| s.datasources[0].url = "postgres://localhost/shop".into()).is_empty());
    let problems = problems_of(|s| s.datasources[1].auth.as_mut().unwrap().secret = "abc123".into());
    assert_eq!(
        problems,
        ["data source 'payments': the secret must be a reference (env:NAME or keychain:NAME), not the secret itself"]
    );
}

#[test]
fn each_kind_checks_its_location() {
    let problems = problems_of(|s| s.datasources[1].url = "api.example.com".into());
    assert_eq!(problems, ["data source 'payments': the base URL must start with http:// or https://"]);
    let problems = problems_of(|s| s.datasources[0].url = String::new());
    assert_eq!(problems, ["data source 'main': the connection is empty"]);
    let problems = problems_of(|s| {
        s.datasources[0].kind = SourceKind::Sqlite;
        s.datasources[0].url = "env:SHOP_DB".into();
    });
    assert_eq!(problems, ["data source 'main': a SQLite source is a file path, like data/shop.db"]);
    let problems = problems_of(|s| s.datasources[0].auth = Some(Auth { kind: AuthKind::Bearer, secret: "env:T".into() }));
    assert_eq!(problems, ["data source 'main': only web API sources have auth"]);
}

#[test]
fn writes_a_new_project_file() {
    let text = write(None, &starter("My shop", "my_shop")).unwrap();
    assert_eq!(
        text,
        r#"[project]
name = "My shop"

[stack]
backend = "rust"
ui = "web"
targets = ["desktop"]

[[datasource]]
name = "main"
kind = "sqlite"
url = "data/my_shop.db"
"#
    );
    assert_eq!(parse(&text).unwrap(), starter("My shop", "my_shop"));
}

#[test]
fn writing_keeps_comments_and_untouched_lines() {
    assert_eq!(write(Some(SHOP), &shop()).unwrap(), SHOP);
    let mut changed = shop();
    changed.stack.targets.push(Target::Web);
    changed.datasources[0].url = "env:SHOP_DATABASE".into();
    let text = write(Some(SHOP), &changed).unwrap();
    assert!(text.starts_with("# The shop example.\n"), "{text}");
    assert!(text.contains("backend = \"csharp\"   # C# with Avalonia"), "{text}");
    assert!(text.contains("name = \"payments\"   # the payment provider"), "{text}");
    assert!(text.contains("targets = [\"desktop\", \"web\"]"), "{text}");
    assert_eq!(parse(&text).unwrap(), changed);
}

#[test]
fn removes_reorders_and_adds_data_sources() {
    let mut changed = shop();
    changed.datasources.reverse();
    changed.datasources[0].auth = None;
    changed.datasources.push(source("archive", SourceKind::Sqlite, "data/archive.db"));
    let text = write(Some(SHOP), &changed).unwrap();
    assert_eq!(parse(&text).unwrap(), changed);
    assert!(text.contains("name = \"payments\"   # the payment provider"), "{text}");
    assert!(!text.contains("auth"), "{text}");

    changed.datasources.clear();
    let text = write(Some(SHOP), &changed).unwrap();
    assert!(!text.contains("datasource"), "{text}");
    assert_eq!(parse(&text).unwrap(), changed);
}

#[test]
fn loads_the_project_file_of_a_folder() {
    let dir = std::env::temp_dir().join(format!("clarity-project-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let missing = load(&dir);
    assert_eq!(missing.settings, None);
    assert_eq!(missing.problems, ["the project has no clarity.toml; create it on the Project tab"]);

    std::fs::write(dir.join(PROJECT_FILE), SHOP).unwrap();
    let found = load(&dir);
    assert_eq!(found.settings, Some(shop()));
    assert!(found.problems.is_empty());
    assert!(!found.version.is_empty());

    std::fs::write(dir.join(PROJECT_FILE), "[stack\n").unwrap();
    let broken = load(&dir);
    assert_eq!(broken.settings, None);
    assert_eq!(broken.problems.len(), 1);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_catalog_offers_every_backend_and_a_ui_for_each() {
    let catalog = catalog();
    assert_eq!(catalog.backends.len(), 5);
    assert!(catalog.backends.iter().all(|b| b.uis.contains(&Ui::Web)));
    let csharp = catalog.backends.iter().find(|b| b.id == Backend::Csharp).unwrap();
    assert_eq!(csharp.uis, [Ui::Web, Ui::Avalonia]);
    assert_eq!(catalog.sources.len(), 5);
}
