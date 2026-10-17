use std::path::Path;

use clarity_project::{
    Auth, AuthKind, Backend, DataSource, Settings, SourceKind, Stack, Target, Ui, catalog, load, module_dir, out_dir,
    parse, starter, title, validate, write,
};

const SHOP: &str = r#"// The shop example.
project Shop
    stack csharp avalonia desktop   // C# with Avalonia
    language "nl-NL"
    currency EUR
    datasource main postgres "env:SHOP_DB_URL"

    // The payment provider.
    datasource payments rest "https://api.example.com/v2"
        auth bearer "keychain:payments"
"#;

fn source(name: &str, kind: SourceKind, url: &str) -> DataSource {
    DataSource {
        name: name.into(),
        kind,
        url: url.into(),
        auth: None,
        external: false,
        readonly: false,
    }
}

fn shop() -> Settings {
    Settings {
        name: "Shop".into(),
        title: None,
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
        language: Some("nl-NL".into()),
        currency: Some("EUR".into()),
        out: "out".into(),
    }
}

#[test]
fn reads_a_project_file() {
    assert_eq!(parse(SHOP).unwrap(), shop());
}

#[test]
fn reads_a_title_and_several_targets() {
    let text = SHOP.replace("    stack csharp avalonia desktop", "    title \"The shop\"\n    stack csharp web desktop web");
    let settings = parse(&text).unwrap();
    assert_eq!(settings.title.as_deref(), Some("The shop"));
    assert_eq!(settings.stack.ui, Ui::Web);
    assert_eq!(settings.stack.targets, [Target::Desktop, Target::Web]);
    assert_eq!(title(&settings), "The shop");
    assert_eq!(title(&shop()), "Shop");
}

#[test]
fn names_the_expected_values_for_an_unknown_one() {
    let err = parse(&SHOP.replace("stack csharp", "stack java")).unwrap_err();
    assert_eq!(err, "line 3: unknown backend 'java'; the backends are rust, go, csharp, c and cpp");
    let err = parse(&SHOP.replace("rest \"https", "graphql \"https")).unwrap_err();
    assert_eq!(err, "line 9: unknown data source kind 'graphql'; the kinds are sqlite, postgres, mysql, mssql and rest");
}

#[test]
fn refuses_unknown_settings_and_a_wrong_shape() {
    let err = parse(&SHOP.replace("    stack", "    frontend svelte\n    stack")).unwrap_err();
    assert_eq!(err, "line 3: unknown setting 'frontend'; the settings are title, stack, output, language, currency and datasource");
    assert_eq!(parse("module shop\n").unwrap_err(), "line 1: a project file starts with 'project Name'");
    assert_eq!(
        parse("project Shop\n    datasource main sqlite \"x.db\"\n").unwrap_err(),
        "the project needs a 'stack' line, like: stack rust web desktop"
    );
    assert_eq!(
        parse(&SHOP.replace("    datasource main", "    stack rust web desktop\n    datasource main")).unwrap_err(),
        "line 6: 'stack' is set twice"
    );
    assert_eq!(
        parse(&SHOP.replace("        auth bearer", "    auth bearer")).unwrap_err(),
        "line 10: 'auth' belongs under a datasource line, indented"
    );
    assert!(parse("project Shop\n    stack rust web\n").unwrap_err().contains("at least one target"));
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
        s.name = "My shop".into();
        s.stack.targets.clear();
    });
    assert_eq!(
        problems,
        [
            "'My shop' is not a valid project name; use a name like MyShop, and a title for the text people see",
            "the project needs at least one target"
        ]
    );
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
    let text = write(None, &starter("MyShop", "my_shop")).unwrap();
    assert_eq!(
        text,
        "project MyShop
    stack rust web desktop
    output \"out\"
    datasource main sqlite \"data/my_shop.db\"
"
    );
    assert_eq!(parse(&text).unwrap(), starter("MyShop", "my_shop"));
}

#[test]
fn writing_keeps_comments_and_untouched_lines() {
    assert_eq!(write(Some(SHOP), &shop()).unwrap(), SHOP);
    let mut changed = shop();
    changed.stack.targets.push(Target::Web);
    changed.datasources[0].url = "env:SHOP_DATABASE".into();
    changed.title = Some("The \"best\" shop".into());
    let text = write(Some(SHOP), &changed).unwrap();
    assert!(text.starts_with("// The shop example.\nproject Shop\n    title \"The \\\"best\\\" shop\"\n"), "{text}");
    assert!(text.contains("    stack csharp avalonia desktop web\n"), "{text}");
    assert!(text.contains("    // The payment provider.\n    datasource payments rest"), "{text}");
    assert!(text.contains("    datasource main postgres \"env:SHOP_DATABASE\"\n"), "{text}");
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
    assert!(text.contains("    // The payment provider.\n    datasource payments rest"), "{text}");
    assert!(!text.contains("auth"), "{text}");

    changed.datasources.clear();
    let text = write(Some(SHOP), &changed).unwrap();
    assert!(!text.contains("datasource"), "{text}");
    assert_eq!(parse(&text).unwrap(), changed);
}

#[test]
fn the_output_folder_defaults_to_out_and_can_be_set() {
    // A file without an output line builds into `out`, and writing keeps it that way.
    assert_eq!(parse(SHOP).unwrap().out, "out");
    assert!(!write(Some(SHOP), &shop()).unwrap().contains("output"));

    let mut elsewhere = shop();
    elsewhere.out = "../builds/shop".into();
    let text = write(Some(SHOP), &elsewhere).unwrap();
    assert!(text.contains("    output \"../builds/shop\"\n"), "{text}");
    assert_eq!(parse(&text).unwrap(), elsewhere);
}

#[test]
fn the_output_folder_is_never_the_project_itself() {
    for out in ["", ".", "./", " "] {
        let problems = problems_of(|s| s.out = out.into());
        assert_eq!(problems, ["the output folder must be a folder of its own, like out"], "{out:?}");
    }
    assert!(problems_of(|s| s.out = "build/app".into()).is_empty());
    assert!(problems_of(|s| s.out = "/home/me/builds/shop".into()).is_empty());
}

#[test]
fn the_modules_and_output_live_in_the_folder_next_to_the_file() {
    let file = Path::new("/work/examples/shop.clarity");
    assert_eq!(module_dir(file), Path::new("/work/examples/shop"));
    assert_eq!(out_dir(file, &shop()), Path::new("/work/examples/shop/out"));
    let mut absolute = shop();
    absolute.out = "/builds/shop".into();
    assert_eq!(out_dir(file, &absolute), Path::new("/builds/shop"));
}

#[test]
fn loads_a_project_file() {
    let dir = std::env::temp_dir().join(format!("clarity-project-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("shop.clarity");

    let missing = load(&file);
    assert_eq!(missing.settings, None);
    assert!(missing.problems[0].starts_with("cannot read"), "{:?}", missing.problems);

    std::fs::write(&file, SHOP).unwrap();
    let found = load(&file);
    assert_eq!(found.settings, Some(shop()));
    assert!(found.problems.is_empty());
    assert!(!found.version.is_empty());

    std::fs::write(&file, "project Shop\n    stak rust\n").unwrap();
    let broken = load(&file);
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

#[test]
fn picked_paths_inside_the_project_folder_become_relative() {
    use clarity_project::project_path;
    let file = Path::new("/work/examples/shop.clarity");
    assert_eq!(project_path(file, Path::new("/work/examples/shop/out")), "out");
    assert_eq!(project_path(file, Path::new("/work/examples/shop/data/shop.db")), "data/shop.db");
    assert_eq!(project_path(file, Path::new("/builds/shop")), "/builds/shop");
    // The project folder itself stays a full path; validation then refuses it as output.
    assert_eq!(project_path(file, Path::new("/work/examples/shop")), "/work/examples/shop");
    assert_eq!(project_path(file, Path::new("/work/examples/shopping/x")), "/work/examples/shopping/x");
}

/// The project files in docs/LANGUAGE.md are read like real ones.
#[test]
fn the_language_reference_project_files_are_valid() {
    const DOC: &str = include_str!("../../../docs/LANGUAGE.md");
    let mut found = 0;
    for block in DOC.split("```").skip(1).step_by(2) {
        let block = block.trim_start_matches('\n');
        if !block.starts_with("project ") {
            continue;
        }
        let settings = parse(block).unwrap_or_else(|e| panic!("{e} in\n{block}"));
        assert!(validate(&settings).is_empty(), "{:?}", validate(&settings));
        found += 1;
    }
    assert_eq!(found, 2);
}

#[test]
fn a_data_source_can_be_external_and_read_only() {
    let text = "project Shop\n    stack rust web desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource erp sqlite \"legacy/erp.db\" external readonly\n";
    let settings = parse(text).unwrap();
    let erp = &settings.datasources[1];
    assert!(erp.external && erp.readonly);
    assert!(!settings.datasources[0].external);
    // Written back the same way.
    let written = write(Some(text), &settings).unwrap();
    assert_eq!(written, text);

    let twice = text.replace("external readonly", "external external");
    assert!(parse(&twice).unwrap_err().contains("'external' is written twice"));
    let unknown = text.replace("external readonly", "shared");
    assert!(parse(&unknown).unwrap_err().contains("unexpected 'shared'; a data source can be external or readonly"));

    // A web API is never Clarity's own.
    let mut web = settings.clone();
    web.datasources[1].kind = SourceKind::Rest;
    assert!(validate(&web).iter().any(|p| p.contains("a web API is always external")), "{:?}", validate(&web));
}

#[test]
fn reads_and_checks_the_language_and_currency() {
    let settings = parse(SHOP).unwrap();
    assert_eq!(settings.language.as_deref(), Some("nl-NL"));
    assert_eq!(settings.currency.as_deref(), Some("EUR"));
    assert_eq!(
        parse(&SHOP.replace("\"nl-NL\"", "\"Dutch\"")).unwrap_err(),
        "line 4: 'Dutch' is not a language tag; write one like en, nl or nl-NL"
    );
    assert_eq!(
        parse(&SHOP.replace("currency EUR", "currency euro")).unwrap_err(),
        "line 5: 'euro' is not a currency code; write its ISO code, like EUR or USD"
    );
    for tag in ["en", "nl-NL", "sr-Latn-RS", "es-419"] {
        assert!(clarity_project::language_problem(tag).is_none(), "{tag}");
    }
    for tag in ["EN", "nl_NL", "nl-nl", "e", "nl-NL-x"] {
        assert!(clarity_project::language_problem(tag).is_some(), "{tag}");
    }
    let problems = problems_of(|s| s.currency = Some("eur".into()));
    assert_eq!(problems, ["'eur' is not a currency code; write its ISO code, like EUR or USD"]);
}

#[test]
fn writes_the_language_and_currency_after_the_output() {
    let mut settings = starter("Shop", "shop");
    settings.language = Some("en-GB".into());
    settings.currency = Some("GBP".into());
    let text = write(None, &settings).unwrap();
    assert!(text.contains("    output \"out\"\n    language \"en-GB\"\n    currency GBP\n    datasource main"), "{text}");
    assert_eq!(parse(&text).unwrap(), settings);
}

#[test]
fn loading_hands_the_language_and_currency_to_the_project() {
    let root = std::env::temp_dir().join(format!("clarity-locale-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("shop")).unwrap();
    std::fs::write(root.join("shop.clarity"), "project Shop\n    stack rust web desktop\n    language \"nl-NL\"\n    currency EUR\n").unwrap();
    std::fs::write(root.join("shop/shop.clarity"), "module shop\n").unwrap();
    let project = clarity_project::load_modules(&root.join("shop")).unwrap();
    assert_eq!(project.locale.language.as_deref(), Some("nl-NL"));
    assert_eq!(project.locale.currency.as_deref(), Some("EUR"));
    std::fs::remove_dir_all(&root).unwrap();
}
