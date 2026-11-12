//! A sidebar on every stack: the renderer docks it, lets the person drag,
//! fold and fill it, and sends that; code reads and sets `collapsed` and
//! `maximized` (conformance/sidebars.json).

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};
use clarity_spec::load_converted;

const SHOP: &str = "module shop

page Main: window
    data shown: text
    sidebar left name nav width 280 min 200 max 480 resizable collapsible collapsed maximizable grid 4 columns
        text name navText at 1,1 span 4 bind shown
    button \"Menu\" name menuButton at 1,1 span 2 on click: menuClick

code Main
    handler menuClick
        if nav.collapsed
            nav.collapsed = false
        nav.maximized = not nav.maximized
";

fn files(backend: Backend, ui: Ui) -> Vec<OutFile> {
    let project = load_converted([("shop.clarity", SHOP)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn text<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn code_reads_and_sets_a_sidebar_on_every_stack() {
    let rust = files(Backend::Rust, Ui::Clarity);
    has(text(&rust, "src/modules/shop.rs"), "if ui.collapsed(\"nav\", true) {");
    has(text(&rust, "src/modules/shop.rs"), "ui.set_maximized(\"nav\", !ui.maximized(\"nav\"));");
    let go = files(Backend::Go, Ui::Clarity);
    has(text(&go, "shop_module.go"), "if ui.Collapsed(\"nav\", true) {");
    has(text(&go, "shop_module.go"), "ui.SetMaximized(\"nav\", !ui.Maximized(\"nav\"))");
    let c = files(Backend::C, Ui::Clarity);
    has(text(&c, "src/modules/shop.c"), "if (clarity_ui_collapsed(ui, \"nav\", true)) {");
    has(text(&c, "src/modules/shop.c"), "clarity_ui_set_maximized(ui, \"nav\", !clarity_ui_maximized(ui, \"nav\"));");
    let cpp = files(Backend::Cpp, Ui::Clarity);
    has(text(&cpp, "src/modules/shop.cpp"), "if (ui.collapsed(\"nav\", true)) {");
    has(text(&cpp, "src/modules/shop.cpp"), "ui.set_maximized(\"nav\", !ui.maximized(\"nav\"));");
}

/// C# keeps a sidebar's folding and filling in its part's state, in the window's view model.
#[test]
fn csharp_keeps_a_sidebar_s_state() {
    let files = files(Backend::Csharp, Ui::Clarity);
    let vm = text(&files, "Windows/MainViewModel.cs");
    has(vm, "public PartState Nav { get; } = new(\"nav\") { IsCollapsed = true };");
    has(vm, "if (Nav.IsCollapsed)");
    has(vm, "Nav.IsMaximized = !Nav.IsMaximized;");
}

/// The designer's markup marks a sidebar and the footer with their keys, as
/// it marks parts: the designer finds their grids by them, so a part in a
/// sidebar or the footer is moved and resized as any.
#[test]
fn the_designer_finds_a_sidebar_s_and_the_footer_s_grid() {
    use clarity_gen::preview::{Shown, files_json};
    let text = SHOP.replace("    button \"Menu\"", "    footer\n        status \"Ready\"\n        text name footText at 1,1 span 3 bind shown\n    button \"Menu\"");
    let project = clarity_spec::keyed::load_keyed([("shop.clarity", text.as_str())], &[], "en");
    let files: serde_json::Value = serde_json::from_str(&files_json(&project, "Shop", &Shown::Window("Main".into()), true, &Default::default())).unwrap();
    let markup = files["windows"]["Main"].as_str().expect("the window's markup");
    let line = |element: &str| markup.lines().find(|l| l.trim_start().starts_with(element)).unwrap_or_else(|| panic!("no {element} in {markup}")).to_string();
    assert!(line("<Sidebar").contains("designItem=\""), "{markup}");
    assert!(line("<Footer").contains("designItem=\""), "{markup}");
}
