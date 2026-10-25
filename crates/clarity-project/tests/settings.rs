use std::path::Path;

use clarity_project::{
    Auth, AuthKind, Backend, DataSource, FileStore, Settings, SourceKind, Stack, Target, Ui, catalog, load, module_dir, out_dir_in, Folders,
    parse, starter, target_dir, title, validate, write,
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
        openapi: None,
        picks: Vec::new(),
        pick_schemas: Vec::new(),
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
        themes: Vec::new(),
        filestores: Vec::new(),
        out: "out".into(),
        keeps: Vec::new(),
    }
}

/// A path Clarity writes for another tool to read (generated code, a cargo
/// `--config`, CC and CXX) is spelled with `/` on every system: Windows takes
/// it everywhere, and a backslash is an escape in TOML, C, Go and C#.
#[test]
fn a_path_written_for_a_tool_has_forward_slashes() {
    let path = Path::new("C:/").join("Users").join("me").join("shop.db");
    assert_eq!(clarity_project::slashed(&path), "C:/Users/me/shop.db");
    // The settings a program is built with carry their folders that way.
    let mut settings = shop();
    settings.filestores = vec![clarity_project::FileStore { name: "photos".into(), location: "data/photos".into() }];
    let dir = std::path::absolute("/projects/shop").unwrap();
    let built = clarity_project::build_settings(&dir, &settings);
    assert_eq!(built.filestores[0].location, clarity_project::slashed(&dir.join("data").join("photos")));
    assert!(!built.filestores[0].location.contains(std::path::MAIN_SEPARATOR) || std::path::MAIN_SEPARATOR == '/');
    assert!(!built.datasources[0].url.contains(std::path::MAIN_SEPARATOR) || std::path::MAIN_SEPARATOR == '/');
}

/// Windows answers `canonicalize` with a verbatim path; `plain` takes the
/// prefix off where the path does not need it, so messages and build tools
/// see an ordinary one. Every other system hands the path back as it is.
#[test]
fn a_canonical_path_is_plain_again() {
    let here = std::env::current_dir().unwrap().canonicalize().unwrap();
    let plain = clarity_project::plain(here.clone());
    assert!(!plain.to_string_lossy().starts_with(r"\\?\"), "{}", plain.display());
    // It is still the same folder, whatever its spelling.
    assert_eq!(plain.canonicalize().unwrap(), here);
    // A path that is not verbatim is its own answer.
    let ordinary = std::path::PathBuf::from("shop/out");
    assert_eq!(clarity_project::plain(ordinary.clone()), ordinary);
}

#[test]
fn reads_a_project_file() {
    assert_eq!(parse(SHOP).unwrap(), shop());
}

/// The folders the output keeps are the project's own, beside the ones the
/// stack's build tools make; they are read, written back and checked.
#[test]
fn the_output_keeps_the_folders_the_project_names() {
    let text = SHOP.replace("    stack csharp avalonia desktop", "    stack csharp avalonia desktop\n    output keeps \"vendor\", \"notes\"");
    let settings = parse(&text).unwrap();
    assert_eq!(settings.keeps, ["vendor", "notes"]);
    assert!(validate(&settings).is_empty(), "{:?}", validate(&settings));
    // It is written back as it was read, so a round trip changes nothing.
    let written = write(Some(&text), &settings).unwrap();
    assert!(written.contains("    output keeps \"vendor\", \"notes\"\n"), "{written}");
    assert_eq!(parse(&written).unwrap(), settings);
    // A project that keeps nothing grows no line.
    let plain = parse(SHOP).unwrap();
    assert!(plain.keeps.is_empty());
    assert!(!write(Some(SHOP), &plain).unwrap().contains("output keeps"));
    // A path is not the name of a folder in the output.
    let bad = SHOP.replace("    stack csharp avalonia desktop", "    stack csharp avalonia desktop\n    output keeps \"a/b\"");
    assert!(parse(&bad).unwrap_err().contains("is a path"), "{}", parse(&bad).unwrap_err());
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
    assert_eq!(err, "line 3: unknown setting 'frontend'; the settings are title, stack, output, language, currency, theme, datasource and filestore");
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
        ["data source 'main': the connection contains a password; put it in an environment variable (env:NAME), the keychain (keychain:NAME) or a secret setting (setting:name)"]
    );
    let problems = problems_of(|s| s.datasources[0].url = "Server=db;Database=shop;Password=hunter2".into());
    assert_eq!(problems.len(), 1, "{problems:?}");
    // A connection without a password is fine.
    assert!(problems_of(|s| s.datasources[0].url = "postgres://localhost/shop".into()).is_empty());
    let problems = problems_of(|s| s.datasources[1].auth.as_mut().unwrap().secret = "abc123".into());
    assert_eq!(
        problems,
        ["data source 'payments': the secret must be a reference (env:NAME, keychain:NAME or setting:name), not the secret itself"]
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
    assert_eq!(out_dir_in(file, &shop(), &Folders::default()), Path::new("/work/examples/shop/out"));
    let mut absolute = shop();
    absolute.out = "/builds/shop".into();
    assert_eq!(out_dir_in(file, &absolute, &Folders::default()), Path::new("/builds/shop"));
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
    // Where builds write the app, with this computer's folders.
    assert_eq!(found.app_folder, clarity_project::out_dir(&file, &shop()).display().to_string());

    std::fs::write(&file, "project Shop\n    stak rust\n").unwrap();
    let broken = load(&file);
    assert_eq!(broken.settings, None);
    assert_eq!(broken.problems.len(), 1);
    assert_eq!(broken.app_folder, "");

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
        // The line of the opening fence: its language, as `clarity`.
        let block = block.split_once('\n').map_or("", |(_, code)| code);
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

#[test]
fn the_project_file_names_its_theme() {
    let text = SHOP.replace("    currency EUR\n", "    currency EUR\n    theme Shop NightOwl\n");
    let settings = parse(&text).unwrap();
    assert_eq!(settings.themes, ["Shop", "NightOwl"]);
    let written = write(Some(&text), &settings).unwrap();
    assert_eq!(written, text);
    let mut fresh = starter("Shop", "shop");
    fresh.themes = vec!["Dark".into()];
    assert!(write(None, &fresh).unwrap().contains("    output \"out\"\n    theme Dark\n"), "{}", write(None, &fresh).unwrap());
}

#[test]
fn loading_hands_the_theme_to_the_project() {
    let root = std::env::temp_dir().join(format!("clarity-theme-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("shop")).unwrap();
    std::fs::write(root.join("shop.clarity"), "project Shop\n    stack rust web desktop\n    theme Plain\n").unwrap();
    std::fs::write(root.join("shop/shop.clarity"), "module shop\n\ntheme Plain\n    radius 2\n").unwrap();
    let project = clarity_project::load_modules(&root.join("shop")).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(project.themes, ["Plain"]);
    assert!(project.diagnostics.iter().all(|d| !d.message.contains("theme")), "{:#?}", project.diagnostics);
}

/// A file store keeps the pictures of image fields as files: a folder, or
/// `env:NAME` for one that differs per machine.
#[test]
fn reads_and_writes_file_stores() {
    let text = format!("{SHOP}    filestore photos \"pictures\"\n    filestore scans \"env:SCAN_FOLDER\"\n");
    let settings = parse(&text).unwrap();
    assert_eq!(
        settings.filestores,
        [FileStore { name: "photos".into(), location: "pictures".into() }, FileStore { name: "scans".into(), location: "env:SCAN_FOLDER".into() }]
    );
    assert_eq!(write(Some(&text), &settings).unwrap(), text);
    let fresh = write(None, &settings).unwrap();
    assert!(fresh.contains("    filestore photos \"pictures\"\n    filestore scans \"env:SCAN_FOLDER\"\n"), "{fresh}");
    assert!(parse(&format!("{SHOP}    filestore photos\n")).unwrap_err().contains("filestore photos \"pictures\""));
}

#[test]
fn file_store_names_are_identifiers_and_unique() {
    let store = |name: &str| FileStore { name: name.into(), location: "pictures".into() };
    assert_eq!(problems_of(|s| s.filestores = vec![store("photos"), store("photos")]), ["there are two file stores named 'photos'"]);
    assert_eq!(problems_of(|s| s.filestores = vec![store("my photos")]), ["'my photos' is not a valid file store name"]);
    assert_eq!(problems_of(|s| s.filestores = vec![FileStore { location: "".into(), ..store("photos") }]), ["file store 'photos' needs a folder"]);
}

/// A relative file store is in the project's folder, as a SQLite file is,
/// whichever folder the application runs in; `env:` stays as written.
#[test]
fn a_file_store_is_found_from_the_project_folder() {
    let mut settings = shop();
    settings.filestores = vec![
        FileStore { name: "photos".into(), location: "data/photos".into() },
        FileStore { name: "scans".into(), location: "env:SCAN_FOLDER".into() },
    ];
    // A folder this system calls absolute, and paths compared as paths: the
    // separator between them is the system's own.
    let dir = std::path::absolute("/projects/shop").unwrap();
    let built = clarity_project::build_settings(&dir, &settings);
    assert_eq!(Path::new(&built.filestores[0].location), dir.join("data/photos"));
    assert_eq!(built.filestores[1].location, "env:SCAN_FOLDER");
}

#[test]
fn a_command_line_program_needs_no_ui() {
    let settings = parse("project Ledger\n    stack go cli\n").unwrap();
    assert_eq!(settings.stack.backend, Backend::Go);
    assert_eq!(settings.stack.targets, [Target::Cli]);
    assert!(!settings.stack.has_ui());
    assert_eq!(validate(&settings), Vec::<String>::new());
    let text = write(None, &settings).unwrap();
    assert!(text.contains("    stack go cli\n"), "{text}");
    assert_eq!(parse(&text).unwrap(), settings);
}

#[test]
fn a_command_line_program_beside_the_desktop_one() {
    let text = SHOP.replace("stack csharp avalonia desktop", "stack csharp avalonia desktop cli");
    let settings = parse(&text).unwrap();
    assert_eq!(settings.stack.ui, Ui::Avalonia);
    assert_eq!(settings.stack.targets, [Target::Desktop, Target::Cli]);
    assert!(settings.stack.has_ui());
    assert_eq!(write(Some(&text), &settings).unwrap(), text);
}

#[test]
fn a_window_needs_its_ui_named() {
    assert_eq!(
        parse("project Shop\n    stack go cli desktop\n").unwrap_err(),
        "line 2: a program with a window names its UI before the targets, like: stack go web desktop cli"
    );
    assert!(catalog().targets.iter().any(|t| t.id == Target::Cli && t.label == "Command line"));
}

#[test]
fn each_target_has_its_own_output_folder() {
    let out = Path::new("/work/examples/shop/out");
    assert_eq!(target_dir(out, Target::Desktop), Path::new("/work/examples/shop/out/desktop"));
    assert_eq!(target_dir(out, Target::Cli), Path::new("/work/examples/shop/out/cli"));
}

/// A web API's key may be a secret setting of the person (DESIGN 5.27),
/// which the system's protected store keeps.
#[test]
fn a_web_api_signs_with_a_secret_setting() {
    let text = "project Shop\n    stack go web desktop\n    datasource shop rest \"https://shop.example.com/api\"\n        auth bearer \"setting:shopToken\"\n";
    let settings = parse(text).unwrap();
    assert_eq!(validate(&settings), Vec::<String>::new());
    let bare = text.replace("\"setting:shopToken\"", "\"setting:\"");
    assert!(validate(&parse(&bare).unwrap()).iter().any(|p| p.contains("must be a reference")));
}

/// A server database's connection may be a secret setting too (DESIGN 5.27):
/// the person keeps it, with its password, in the system's protected store.
/// A SQLite file is no secret.
#[test]
fn a_database_connects_with_a_secret_setting() {
    assert!(problems_of(|s| s.datasources[0].url = "setting:shopDb".into()).is_empty());
    let problems = problems_of(|s| s.datasources[0].url = "setting:".into());
    assert_eq!(problems, ["data source 'main': the connection names no variable, keychain entry or setting after its prefix"]);
    let problems = problems_of(|s| {
        s.datasources[0].kind = SourceKind::Sqlite;
        s.datasources[0].url = "setting:shopDb".into();
    });
    assert_eq!(problems, ["data source 'main': a SQLite source is a file path, like data/shop.db"]);
    let refs = clarity_project::source_refs(&{
        let mut s = shop();
        s.datasources[0].url = "setting:shopDb".into();
        s
    });
    assert_eq!(refs[0].address, "setting:shopDb");
}
