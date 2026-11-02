//! A Rust desktop application on Clarity.UI (DESIGN 7.9, G8a): `stack rust
//! desktop`. The program is the same as on the shared screen (the runtime's
//! `Program`, implemented once), without Tauri: the windows and app frames as
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

/// The webview stack is unchanged: Tauri, its commands thin calls into the program.
#[test]
fn the_webview_stack_keeps_tauri() {
    let files = files_of(Ui::Webview);
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    for path in ["build.rs", "tauri.conf.json", "src/desktop.rs", "ui/index.html", "ui/windows/CustomerList.cxaml"] {
        assert!(paths.contains(&path), "no {path} in {paths:?}");
    }
    assert!(!paths.iter().any(|p| p.starts_with("themes/") || p.starts_with("texts/")), "{paths:?}");
    has(file(&files, "Cargo.toml"), "tauri = ");
    assert!(!file(&files, "Cargo.toml").contains("clarity-ui"));
    has(file(&files, "src/desktop.rs"), "fn dispatch_event(app: tauri::State<App>, state: WindowState, handler: String) -> Answer<Reply> {\n    app.dispatch_event(state, &handler)\n}");
    has(file(&files, "src/main.rs"), "impl Program for App {");
}
