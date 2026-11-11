//! A data source that is a file in a format (docs/FORMATS-PLAN.md): every
//! stack's program, desktop and command line, opens it after connecting,
//! with its format, options, the entity's table and key and the columns it
//! maps, and carries Clarity.Formats and SQLite for it.

use clarity_project::Backend;

const LOGINS: &str = "module logins\nsource logins\n\nentity Login\n    site: text(200), column \"Login URL\"\n    visits: integer\n    paid: bool\n";
const MAIN: &str = "module main\nuse logins\n\nentity Note\n    text: text(200)\n\npage Logins: window\n    data logins: list<Login>\n    table name list at 1,1 span 4 bind logins\n";

fn files(backend: Backend, cli: bool) -> Vec<clarity_gen::OutFile> {
    let sources = [clarity_spec::SourceRef::sql("main"), clarity_spec::SourceRef { external: true, ..clarity_spec::SourceRef::sql("logins") }];
    let step = "migration \"Notes\"\n    add entity Note\n        text: text(200)\n";
    let project = clarity_spec::load_in(clarity_spec::converted([("erasure/main.clarity", MAIN), ("erasure/logins.clarity", LOGINS)]).iter().map(|(p, t)| (p.as_str(), t.as_str())), [("migrations/main/0001.clarity", step)], &sources);
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let target = if cli { "cli" } else { "desktop" };
    let mut settings = clarity_project::parse(&format!("project Erasure\n    stack rust {target}\n    datasource main sqlite \"data/erasure.db\"\n    datasource logins csv \"imports/logins.csv\" external\n        separator \";\"\n")).unwrap();
    settings.stack.backend = backend;
    if cli { clarity_gen::generate_cli(&project, &settings) } else { clarity_gen::generate(&project, &settings) }.unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"))
}

fn all_text(files: &[clarity_gen::OutFile]) -> String {
    files.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n")
}

#[test]
fn every_stack_opens_a_file_source_after_connecting() {
    let wanted = [
        (Backend::Rust, "db.add_file(clarity_runtime::FileSource { name: \"logins\", format: \"csv\", path: \"imports/logins.csv\", options: r#\"{\"separator\":\";\"}\"#"),
        (Backend::Go, "db.AddFile(clarity.FileSource{Name: \"logins\", Format: \"csv\", Path: \"imports/logins.csv\""),
        (Backend::C, "clarity_db_add_file(db, &(ClarityFileSource){.name = \"logins\", .format = \"csv\", .path = \"imports/logins.csv\""),
        (Backend::Cpp, "add_file(clarity::FileSource{\"logins\", \"csv\", \"imports/logins.csv\""),
        (Backend::Csharp, "Clarity.Formats.FileSources.AddFile(db, new Clarity.Formats.FileSource(\"logins\", \"csv\", \"imports/logins.csv\""),
    ];
    for (backend, call) in wanted {
        for cli in [false, true] {
            let text = all_text(&files(backend, cli));
            assert!(text.contains(call), "{backend:?} cli {cli}: no {call}");
            assert!(text.contains("login") && text.contains("Login URL"), "{backend:?} cli {cli}: table and columns");
            // The file source is not one of the databases connected to.
            assert!(!text.contains("\"imports/logins.csv\")"), "{backend:?} cli {cli}: the file is connected to as a database");
        }
    }
}

#[test]
fn a_program_with_a_file_source_carries_formats_and_sqlite() {
    let rust = all_text(&files(Backend::Rust, false));
    assert!(rust.contains("\"formats\"") && rust.contains("\"sqlite\""), "the Rust program's features");
    let csharp = all_text(&files(Backend::Csharp, true));
    assert!(csharp.contains("Clarity.Formats") && csharp.contains("Sqlite.Use();"), "the C# program's package and driver");
    let go = files(Backend::Go, true);
    assert!(go.iter().any(|f| f.path.ends_with("formats.go")), "the Go program's formats.go");
    for backend in [Backend::C, Backend::Cpp] {
        let text = all_text(&files(backend, true));
        assert!(text.contains("clarity_use_formats("), "{backend:?}: links Clarity.Formats");
        assert!(text.contains("formats/file_source."), "{backend:?}: carries the file source");
    }
}

/// A C++ program without the desktop's files still fetches Clarity.UI's
/// folder, where Clarity.Formats lies, when CLARITY_UI names none.
#[test]
fn a_cpp_program_without_the_desktop_fetches_the_folder_of_clarity_formats() {
    for cli in [false, true] {
        let cpp = files(Backend::Cpp, cli);
        assert!(cpp.iter().any(|f| f.path == "clarity-runtime/clarity_ui.cmake"), "cli {cli}: clarity_ui.cmake");
        assert!(cpp.iter().any(|f| f.path == "CMakeLists.txt" && f.text.contains("set(CLARITY_UI_VERSION ")), "cli {cli}: the version it fetches");
    }
}

/// A C program without the desktop's files fetches it too.
#[test]
fn a_c_program_without_the_desktop_fetches_the_folder_of_clarity_formats() {
    for cli in [false, true] {
        let c = files(Backend::C, cli);
        assert!(c.iter().any(|f| f.path == "clarity-runtime/clarity_ui.cmake"), "cli {cli}: clarity_ui.cmake");
        assert!(c.iter().any(|f| f.path == "CMakeLists.txt" && f.text.contains("set(CLARITY_UI_VERSION ")), "cli {cli}: the version it fetches");
    }
}

const FEED: &str = "module feed\nsource feed\n\nentity Order\n    number: text(20), label @orderNumber\n    customer: text(100), label @orderCustomer\n    lines: list<Line>, label @orderLines\n\nentity Line\n    order: ref Order, label @lineOrder\n    product: text(80), label @lineProduct\n\nmapping OrderFeed of Order from xml\n    at \"shop/orders/order\"\n    number at \"@number\"\n    customer at \"customer/name\"\n    lines of Line at \"lines/line\"\n        order is \"..\"\n        product at \"@product\"\n    check \"orders.xsd\"\n    indent true\n\nfunction ordersIn(text: text) -> list<Order>\n    return read list<Order> from text using OrderFeed\n\nfunction written(orders: list<Order>) -> text\n    return text of orders using OrderFeed\n";

/// The options object a mapping becomes, as every stack passes it to
/// Clarity.Formats; `check` is left out until the library takes schemas.
const FEED_OPTIONS: &str = r#"{"at":"shop/orders/order","fields":{"number":"@number","customer":"customer/name"},"nested":{"lines":{"at":"lines/line","fields":{"product":"@product"},"parent":"order"}},"indent":true}"#;

fn feed_files(backend: Backend) -> Vec<clarity_gen::OutFile> {
    let sources = [clarity_spec::SourceRef::sql("main"), clarity_spec::SourceRef { external: true, mapping: Some("OrderFeed".into()), ..clarity_spec::SourceRef::sql("feed") }];
    let main = "module main\nuse feed\n\npage Orders: window\n    data orders: list<Order>\n    table name list at 1,1 span 4 bind orders\n";
    let project = clarity_spec::load_in(clarity_spec::converted([("shop/main.clarity", main), ("shop/feed.clarity", FEED)]).iter().map(|(p, t)| (p.as_str(), t.as_str())), [], &sources);
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.iter().all(|e| e.contains("label") || e.contains("translation")), "{errors:#?}");
    let mut settings = clarity_project::parse("project Shop\n    stack rust desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource feed file \"imports/orders.xml\" external using OrderFeed\n").unwrap();
    settings.stack.backend = backend;
    clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"))
}

#[test]
fn a_mapping_is_the_options_object_on_every_stack() {
    let escaped = FEED_OPTIONS.replace('"', "\\\"");
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp, Backend::Csharp] {
        let text = all_text(&feed_files(backend));
        // Read and written in code, and the file source laid over by it: the format from the mapping.
        assert!(text.matches(escaped.as_str()).count() + text.matches(FEED_OPTIONS).count() >= 3, "{backend:?}: the mapping's options object in code and on the file source");
        assert!(text.contains("\"imports/orders.xml\""), "{backend:?}: the file source");
    }
}

#[test]
fn a_file_source_names_a_mapping_the_project_declares() {
    let sources = [clarity_spec::SourceRef::sql("main"), clarity_spec::SourceRef { external: true, mapping: Some("OrderFed".into()), ..clarity_spec::SourceRef::sql("feed") }];
    let project = clarity_spec::load_in(clarity_spec::converted([("shop/feed.clarity", FEED)]).iter().map(|(p, t)| (p.as_str(), t.as_str())), [], &sources);
    assert!(project.diagnostics.iter().any(|d| d.message == "data source feed is laid over by mapping OrderFed, which no module declares" && d.span.line == 2), "{:#?}", project.diagnostics);
    let other = FEED.replace("source feed\n", "");
    let project = clarity_spec::load_in(clarity_spec::converted([("shop/feed.clarity", other.as_str())]).iter().map(|(p, t)| (p.as_str(), t.as_str())), [], &[clarity_spec::SourceRef::sql("main"), clarity_spec::SourceRef { external: true, mapping: Some("OrderFeed".into()), ..clarity_spec::SourceRef::sql("feed") }]);
    assert!(project.diagnostics.iter().any(|d| d.message == "data source feed is laid over by mapping OrderFeed, so Order lives in it: write 'source feed' in the module of Order"), "{:#?}", project.diagnostics);
}

/// The records field among the file source's fields: the inner entity's
/// kind with its own table and key, where the runtime keeps its rows.
const FEED_LINES: [&str; 6] = [r#""name":"lines""#, r#""kind":"records""#, r#""entity":"Line""#, r#""fields":[{"kind":"text","name":"product"}]"#, r#""table":"line""#, r#""key":"id""#];

#[test]
fn a_file_source_with_records_inside_passes_their_entity_table_and_key() {
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp, Backend::Csharp] {
        let text = all_text(&feed_files(backend));
        for part in FEED_LINES {
            let escaped = part.replace('"', "\\\"");
            assert!(text.contains(part) || text.contains(escaped.as_str()), "{backend:?}: {part} in the records field among the fields");
        }
    }
}

#[test]
fn the_entity_of_records_inside_lives_in_the_file_source() {
    let feed = FEED.replace("entity Line\n", "module lines\n\nentity Line\n");
    let (feed, lines) = feed.split_once("module lines\n").unwrap();
    let lines = format!("module lines\nuse feed\n{}", lines);
    let feed = feed.replace("source feed\n", "source feed\nuse lines\n");
    let sources = [clarity_spec::SourceRef::sql("main"), clarity_spec::SourceRef { external: true, mapping: Some("OrderFeed".into()), ..clarity_spec::SourceRef::sql("feed") }];
    let project = clarity_spec::load_in(clarity_spec::converted([("shop/feed.clarity", feed.as_str()), ("shop/lines.clarity", lines.as_str())]).iter().map(|(p, t)| (p.as_str(), t.as_str())), [], &sources);
    assert!(project.diagnostics.iter().any(|d| d.message == "data source feed is laid over by mapping OrderFeed, so Line lives in it: write 'source feed' in the module of Line"), "{:#?}", project.diagnostics);
}

/// Records inside a file source's mapping name the record they sit in
/// (`order is ".."`): a record made in code would otherwise belong to no
/// record of the file, and writing it back would lose it.
#[test]
fn records_inside_a_file_source_name_the_record_they_sit_in() {
    let feed = FEED.replace("        order is \"..\"\n", "");
    let sources = [clarity_spec::SourceRef::sql("main"), clarity_spec::SourceRef { external: true, mapping: Some("OrderFeed".into()), ..clarity_spec::SourceRef::sql("feed") }];
    let project = clarity_spec::load_in(clarity_spec::converted([("shop/feed.clarity", feed.as_str())]).iter().map(|(p, t)| (p.as_str(), t.as_str())), [], &sources);
    assert!(
        project.diagnostics.iter().any(|d| d.message == "data source feed keeps the Line records inside each Order, so mapping OrderFeed names the Order a Line sits in: write 'order is \"..\"' under 'lines of Line', with order: ref Order on Line"),
        "{:#?}",
        project.diagnostics
    );
}
