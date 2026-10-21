//! A ribbon on every stack: the renderer opens its tabs and folds it, and
//! sends that; code reads and sets `collapsed` and the open `tab`, and hides
//! tabs and groups by name (conformance/ribbons.json).

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};


const SHOP: &str = "module shop

page Main: window
    ribbon name mainRibbon collapsible collapsed
        tab \"Home\" name homeTab
            group \"Clipboard\" name clipboardGroup
                button \"Paste\" name pasteButton large on click: pasteClicked
        tab \"View\" name viewTab
            group \"Zoom\" name zoomGroup
                button \"Zoom in\" name zoomInButton on click: pasteClicked
    text name mainText at 1,1 span 6

code Main
    handler pasteClicked
        mainRibbon.collapsed = false
        if mainRibbon.tab == 1
            mainRibbon.tab = 2
        clipboardGroup.visible = false
";

fn files(backend: Backend, ui: Ui) -> Vec<OutFile> {
    let project = clarity_spec::keyed::load_keyed([("shop.clarity", SHOP)], &[], "en");
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
fn code_reads_and_sets_a_ribbon_on_every_stack() {
    let rust = files(Backend::Rust, Ui::Web);
    let code = text(&rust, "src/modules/shop.rs");
    has(code, "ui.set_collapsed(\"mainRibbon\", false);");
    has(code, "if (ui.tab(\"mainRibbon\") == 1i64) {");
    has(code, "ui.set_tab(\"mainRibbon\", 2i64);");
    has(code, "ui.set_visible(\"clipboardGroup\", false);");
    let go = files(Backend::Go, Ui::Web);
    has(text(&go, "shop_module.go"), "ui.SetTab(\"mainRibbon\", 2)");
    let c = files(Backend::C, Ui::Web);
    has(text(&c, "src/modules/shop.c"), "clarity_ui_set_tab(ui, \"mainRibbon\", 2);");
    let cpp = files(Backend::Cpp, Ui::Web);
    has(text(&cpp, "src/modules/shop.cpp"), "ui.set_tab(\"mainRibbon\", int64_t{2});");
}

/// Avalonia's ribbon is a tab control whose open tab, folding and the
/// visibility of its tabs and groups follow the parts' state.
#[test]
fn avalonia_ribbon_follows_its_state() {
    let files = files(Backend::Csharp, Ui::Avalonia);
    let view = text(&files, "Windows/MainView.axaml");
    has(view, "<c:RibbonView x:Name=\"mainRibbon\" DockPanel.Dock=\"Top\" Classes=\"ribbon\" Collapsible=\"True\" IsCollapsed=\"{Binding MainRibbon.IsCollapsed}\" Tab=\"{Binding MainRibbon.Tab}\">");
    has(view, "<TabItem x:Name=\"homeTab\" Header=\"{x:Static local:Texts.HomeTab}\" IsVisible=\"{Binding HomeTab.IsVisible}\">");
    has(view, "IsVisible=\"{Binding ClipboardGroup.IsVisible}\"");
    let vm = text(&files, "Windows/MainViewModel.cs");
    has(vm, "public PartState MainRibbon { get; } = new(\"mainRibbon\") { IsCollapsed = true };");
    has(vm, "public PartState HomeTab { get; } = new(\"homeTab\");");
    has(vm, "MainRibbon.Tab = 2L;");
}
