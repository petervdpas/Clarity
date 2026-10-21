//! A foldable panel or form on every stack: the web renderer folds it by its
//! header and sends whether it is folded; the code reads and sets
//! `part.folded`, starting from the design (conformance/folding.json).

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};
use clarity_spec::load_sources;

const SHOP: &str = "module shop

entity Customer
    name: text(40)

page Main: window
    data customer: Customer
    panel \"Details\" name details at 1,1 span 6 rows 4 foldable folded
        text name nameText at 1,1 span 4 bind customer.name
    button \"More\" name more at 1,5 span 2 on click: moreClick

code Main
    handler moreClick
        if details.folded
            details.folded = false
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
fn code_reads_and_sets_folded_from_the_design_on_every_stack() {
    let rust = files(Backend::Rust, Ui::Web);
    has(text(&rust, "src/modules/shop.rs"), "if ui.folded(\"details\", true) {");
    has(text(&rust, "src/modules/shop.rs"), "ui.set_folded(\"details\", false);");
    let go = files(Backend::Go, Ui::Web);
    has(text(&go, "shop_module.go"), "if ui.Folded(\"details\", true) {");
    has(text(&go, "shop_module.go"), "ui.SetFolded(\"details\", false)");
    let c = files(Backend::C, Ui::Web);
    has(text(&c, "src/modules/shop.c"), "if (clarity_ui_folded(ui, \"details\", true)) {");
    has(text(&c, "src/modules/shop.c"), "clarity_ui_set_folded(ui, \"details\", false);");
    let cpp = files(Backend::Cpp, Ui::Web);
    has(text(&cpp, "src/modules/shop.cpp"), "if (ui.folded(\"details\", true)) {");
    has(text(&cpp, "src/modules/shop.cpp"), "ui.set_folded(\"details\", false);");
}

/// Avalonia folds with an Expander, open when the part is not folded; the
/// window starts it as designed.
#[test]
fn avalonia_folds_with_an_expander() {
    let files = files(Backend::Csharp, Ui::Avalonia);
    let view = text(&files, "Windows/MainView.axaml");
    has(view, "<Expander x:Name=\"details\" Grid.Column=\"0\" Grid.Row=\"0\" Grid.ColumnSpan=\"6\" Grid.RowSpan=\"4\" Header=\"{x:Static local:Texts.Details}\" IsExpanded=\"{Binding Details.IsOpen}\"");
    let vm = text(&files, "Windows/MainViewModel.cs");
    has(vm, "public PartState Details { get; } = new(\"details\") { IsFolded = true };");
    has(vm, "if (Details.IsFolded)");
    has(vm, "Details.IsFolded = false;");
}
