//! A sidebar on every stack: the renderer docks it, lets the person drag,
//! fold and fill it, and sends that; code reads and sets `collapsed` and
//! `maximized` (conformance/sidebars.json).

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};
use clarity_spec::load_sources;

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
    let project = load_sources([("shop.clarity", SHOP)]);
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
    let rust = files(Backend::Rust, Ui::Web);
    has(text(&rust, "src/modules/shop.rs"), "if ui.collapsed(\"nav\", true) {");
    has(text(&rust, "src/modules/shop.rs"), "ui.set_maximized(\"nav\", !ui.maximized(\"nav\"));");
    let go = files(Backend::Go, Ui::Web);
    has(text(&go, "shop_module.go"), "if ui.Collapsed(\"nav\", true) {");
    has(text(&go, "shop_module.go"), "ui.SetMaximized(\"nav\", !ui.Maximized(\"nav\"))");
    let c = files(Backend::C, Ui::Web);
    has(text(&c, "src/modules/shop.c"), "if (clarity_ui_collapsed(ui, \"nav\", true)) {");
    has(text(&c, "src/modules/shop.c"), "clarity_ui_set_maximized(ui, \"nav\", !clarity_ui_maximized(ui, \"nav\"));");
    let cpp = files(Backend::Cpp, Ui::Web);
    has(text(&cpp, "src/modules/shop.cpp"), "if (ui.collapsed(\"nav\", true)) {");
    has(text(&cpp, "src/modules/shop.cpp"), "ui.set_maximized(\"nav\", !ui.maximized(\"nav\"));");
}

/// Avalonia docks the runtime's SidebarView, its folding and filling bound to the part's state.
#[test]
fn avalonia_docks_a_sidebar_view() {
    let files = files(Backend::Csharp, Ui::Avalonia);
    let view = text(&files, "Windows/MainView.axaml");
    has(
        view,
        "<c:SidebarView x:Name=\"nav\" DockPanel.Dock=\"Left\" Side=\"Left\" Classes=\"sidebar\" SidebarWidth=\"280\" MinSidebarWidth=\"200\" MaxSidebarWidth=\"480\" Resizable=\"True\" Collapsible=\"True\" Maximizable=\"True\" IsCollapsed=\"{Binding Nav.IsCollapsed}\" IsMaximized=\"{Binding Nav.IsMaximized}\">",
    );
    has(view, "<c:SidebarView.Content>");
    let vm = text(&files, "Windows/MainViewModel.cs");
    has(vm, "public PartState Nav { get; } = new(\"nav\") { IsCollapsed = true };");
    has(vm, "if (Nav.IsCollapsed)");
    has(vm, "Nav.IsMaximized = !Nav.IsMaximized;");
}
