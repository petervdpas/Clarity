//! A Rust desktop application on Clarity.UI (DESIGN 7.9, G8a): `stack rust
//! desktop`. The program (the runtime's `Program`, implemented once) is the
//! same one the web server answers with: the windows and app frames as
//! `.cxaml`, the themes as resolved JSON and the model travel in the program,
//! and main starts `clarity_ui::desktop`.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

const SHOP: &str = "module shop

entity Customer
    name: text(80), required

app Shop
    title \"Shop\"
    main CustomerList

theme Night
    mode dark
    color accent \"#e0a030\"

page CustomerList: window
    title \"Customers\"
    data customers: list<Customer>
    table name grid at 1,1 span 12 rows 4 bind customers columns name
    button \"Add\" name addButton at 1,5 span 2 on click: addClicked

code CustomerList
    handler addClicked
        message \"Added\"
";

const NL: &str = "translations nl
    \"Customers\" \"Klanten\"
";

fn settings(ui: Ui) -> Settings {
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Rust;
    settings.stack.ui = ui;
    settings
}

fn files_of(ui: Ui) -> Vec<OutFile> {
    let mut project = load_keyed([("shop.clarity", SHOP)], &[("translations/nl.clarity", NL)], "en");
    clarity_spec::themes::use_themes(&mut project, &["Night"]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    generate(&project, &settings(ui)).unwrap_or_else(|errors| panic!("{errors:#?}"))
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn a_clarity_ui_project_has_no_tauri() {
    let files = files_of(Ui::Clarity);
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    for path in ["build.rs", "tauri.conf.json", "src/desktop.rs", "ui/index.html", "icons/icon.ico"] {
        assert!(!paths.contains(&path), "{path} in {paths:?}");
    }
    for path in ["Cargo.toml", "README.md", "src/main.rs", "src/model.json", "src/modules.rs", "src/modules/shop.rs", "src/schema.rs"] {
        assert!(paths.contains(&path), "no {path} in {paths:?}");
    }
    for f in &files {
        assert!(!f.text.contains("tauri"), "{} names Tauri:\n{}", f.path, f.text);
    }
}

#[test]
fn the_manifest_names_the_runtime_and_clarity_ui() {
    let files = files_of(Ui::Clarity);
    let cargo = file(&files, "Cargo.toml");
    has(cargo, &format!("clarity-runtime = {{ version = \"{}\", default-features = false, features = [\"sqlite\"] }}", clarity_gen::rust_runtime_version()));
    has(cargo, &format!("clarity-ui = \"{}\"", clarity_gen::rust_ui_version()));
    has(cargo, "\n[workspace]\n");
    // The README says how to build it, and how to build on the crates in Clarity's repository.
    let readme = file(&files, "README.md");
    has(readme, "cargo build --release");
    has(readme, "clarity-ui = { path = \"<Clarity>/runtimes/rust-ui\" }");
    assert!(!readme.contains("WebKitGTK"), "{readme}");
}

#[test]
fn the_windows_frames_themes_and_model_travel_in_the_program() {
    let files = files_of(Ui::Clarity);
    has(file(&files, "windows/CustomerList.cxaml"), "<Window ");
    has(file(&files, "apps/Shop.cxaml"), "<App ");
    has(file(&files, "themes/night.light.json"), "\"accent\"");
    has(file(&files, "themes/night.dark.json"), "\"accent\"");
    let main = file(&files, "src/main.rs");
    has(main, "    windows: &[(\"CustomerList\", include_str!(\"../windows/CustomerList.cxaml\"))],\n");
    has(main, "    apps: &[(\"Shop\", include_str!(\"../apps/Shop.cxaml\"))],\n");
    has(main, "    themes: &[(\"night.light\", include_str!(\"../themes/night.light.json\")), (\"night.dark\", include_str!(\"../themes/night.dark.json\"))],\n");
    // The model, whose translations Clarity.UI loads, and the main window's size.
    has(main, "    model: MODEL,\n");
    has(main, "    width: ");
}

#[test]
fn main_starts_the_program_on_clarity_ui() {
    let files = files_of(Ui::Clarity);
    let main = file(&files, "src/main.rs");
    // The program, as on the shared screen: the runtime's Program, implemented once.
    has(main, "impl Program for App {");
    has(main, "    fn dispatch_event(&self, state: WindowState, handler: &str) -> Answer<Reply> {");
    // Started as the Tauri main starts it: the translations, then the databases.
    has(main, "    clarity_runtime::i18n::load(MODEL);\n");
    has(main, "    let app = App(Mutex::new(db));\n");
    has(main, "const FILES: clarity_ui::AppFiles = clarity_ui::AppFiles {\n    windows: ");
    has(main, "    title: \"Shop\",\n");
    has(main, "fn main() -> std::process::ExitCode {\n");
    has(main, "    clarity_ui::desktop::run(app, FILES)\n}");
    // Nothing of the web server or Tauri's commands.
    assert!(!main.contains("fn served(") && !main.contains("#[tauri::command]"), "{main}");
}

/// Rust's webview desktop (Tauri) is gone, as Avalonia from C#: a Rust
/// program with windows draws with Clarity.UI.
#[test]
fn the_webview_desktop_is_gone() {
    let project = load_keyed([("shop.clarity", SHOP)], &[], "en");
    let errors = generate(&project, &settings(Ui::Webview)).unwrap_err();
    assert_eq!(errors, ["Rust draws its windows with Clarity.UI only; write stack rust desktop"]);
}

/// `stack rust web` serves Clarity.UI's browser files and answers their
/// wire (round B of DESIGN 7.9); beside the desktop target it is the same
/// program, serving with `--serve`.
#[test]
fn the_web_program_serves_clarity_ui_in_the_browser() {
    let project = load_keyed([("shop.clarity", SHOP)], &[], "en");
    let mut web = settings(Ui::Clarity);
    web.stack.targets = vec![clarity_project::Target::Web];
    let files = generate(&project, &web).unwrap_or_else(|errors| panic!("{errors:#?}"));
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"browser/index.html") && paths.contains(&"browser/clarity/files.json") && paths.contains(&"src/facts.rs"), "{paths:?}");
    assert!(!file(&files, "Cargo.toml").contains("clarity-ui"));
    has(file(&files, "src/main.rs"), "clarity_runtime::wire::answer(&app, command, args)");
    has(file(&files, "README.md"), "Rust with Clarity.UI");

    // With the desktop target too, one program: the desktop's, serving with --serve.
    web.stack.targets = vec![clarity_project::Target::Desktop, clarity_project::Target::Web];
    let files = generate(&project, &web).unwrap_or_else(|errors| panic!("{errors:#?}"));
    has(file(&files, "src/main.rs"), "arg == \"--serve\"");
}

const ORDERS: &str = "module orders

enum Status
    open
    paid

entity Customer
    name: text(80), required

entity Order
    total: money, required, min 0
    status: Status
    customer: ref Customer
    color: color

page OrderList: window
    title \"Orders\"
    data orders: list<Order>
    data order: Order
    data note: text
    table name grid at 1,1 span 12 rows 4 bind orders columns total
";

fn orders_files() -> Vec<OutFile> {
    let project = load_keyed([("orders.clarity", ORDERS)], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    generate(&project, &settings(Ui::Clarity)).unwrap_or_else(|errors| panic!("{errors:#?}"))
}

/// What Clarity.UI's parts read beside the values (the C# Fields.cs): each
/// field's facts, the types of each window's values, each enum's values, as
/// one static table the program gives through `Program::facts`.
#[test]
fn the_program_carries_the_facts_of_its_fields() {
    let files = orders_files();
    let facts = file(&files, "src/facts.rs");
    has(facts, "pub static FACTS: Facts = Facts {\n");
    has(facts, "(\"OrderList\", &[(\"orders\", \"list<Order>\"), (\"order\", \"Order\"), (\"note\", \"text\")])");
    has(facts, "(\"Order\", &[(\"total\", \"money\"), (\"status\", \"Status\"), (\"customer\", \"ref<Customer>\"), (\"color\", \"color\")])");
    has(facts, "(\"Order.total\", FieldFacts { label: \"@orderTotalLabel\", format: Some(FieldFormat { kind: FormatKind::Money, scale: None, format: None }), required: true, min: Some(\"0\"), max: None, refers: None, parent: None, color: false })");
    has(facts, "(\"Order.customer\", FieldFacts { label: \"@orderCustomerLabel\", format: None, required: false, min: None, max: None, refers: Some(\"Customer\"), parent: None, color: false })");
    has(facts, "color: true })");
    has(facts, "(\"Status\", &[\"open\", \"paid\"])");
    let main = file(&files, "src/main.rs");
    has(main, "mod facts;\n");
    has(main, "    fn facts_json(&self) -> Value {\n        facts::FACTS.to_json()\n    }\n");
}


mod everything_project;

/// Every part of every window of the every-part project is written as an
/// element Clarity.UI for Rust reads as a kind of its catalog
/// (runtimes/rust-ui/catalog/part-defaults.json): `<Textbox>` is `textbox`,
/// `<TreeTable>` is `treeTable`. A tag it does not know would show as a part
/// that is missing.
#[test]
fn every_part_is_written_as_a_kind_clarity_ui_reads() {
    let catalog: serde_json::Map<String, serde_json::Value> = serde_json::from_str(include_str!("../../../runtimes/rust-ui/catalog/part-defaults.json")).unwrap();
    let (file, project, settings) = everything_project::prepare("rust-clarity-tags", "rust desktop cli");
    let files = generate(&project, &settings).unwrap_or_else(|errors| panic!("{errors:#?}"));
    let mut seen = std::collections::BTreeSet::new();
    for window in files.iter().filter(|f| f.path.starts_with("windows/") && f.path.ends_with(".cxaml")) {
        // A part is an element with a place in its grid: `at="1,2"`.
        for element in window.text.split('<').skip(1).filter(|e| !e.starts_with('/') && e.contains(" at=\"")) {
            let tag: String = element.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            let mut chars = tag.chars();
            let kind: String = chars.next().map(|c| c.to_lowercase().chain(chars).collect()).unwrap_or_default();
            assert!(catalog.contains_key(&kind), "<{tag}> in {} is no kind of Clarity.UI's catalog", window.path);
            seen.insert(kind);
        }
    }
    // The project has every kind of part.
    assert!(seen.len() > 40, "{seen:?}");
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}
