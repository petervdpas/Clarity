//! The person's keys in the generated application (DESIGN 5.28), where the
//! project file says `keys editable`: the keys window, the catalog of
//! commands, and each command known by its id.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};

const SHOP: &str = "module shop

app Shop
    main Card
    menu name mainMenu
        item @fileItem name fileItem
            item @keysItem name keysItem on click -> keys
            item @quitItem name quitItem key \"Ctrl+Q\" on click -> quit

page Card: window
    title @cardTitle
    toolbar name cardToolbar
        button @printCaption name printButton on click: saveClicked
    button @saveCaption name saveButton at 1,1 span 2 key \"Ctrl+S\" on click: saveClicked

code Card
    handler saveClicked
        message @savedText
";

fn files(backend: Backend, ui: Ui, editable: bool) -> Vec<OutFile> {
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    settings.keys_editable = editable;
    let project = clarity_spec::load_in([("shop.clarity", SHOP)], [], &clarity_project::source_refs(&settings));
    generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

#[test]
fn csharp_starts_the_persons_keys() {
    let files = files(Backend::Csharp, Ui::Clarity, true);
    let program = file(&files, "Program.cs");
    // On the desktop the library reads the keys window's commands from model.json.
    assert!(program.contains("PersonKeys.Start(\"shop\");\n"), "{program}");
    assert!(!program.contains("KeysCatalog"), "{program}");
    assert!(!files.iter().any(|f| f.path == "KeysCatalog.cs"));
    let model = file(&files, "model.json");
    assert!(model.contains("\"keysEditable\":true"), "the model says keys are editable");
}

#[test]
fn csharp_on_the_web_keeps_its_keys_catalog() {
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Clarity;
    settings.keys_editable = true;
    settings.stack.targets = vec![clarity_project::Target::Web];
    let project = clarity_spec::load_in([("shop.clarity", SHOP)], [], &clarity_project::source_refs(&settings));
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let catalog = file(&files, "App/KeysCatalog.cs");
    assert!(catalog.contains("using Clarity.Ui;\n"), "{catalog}");
    assert!(catalog.contains("new KeyLayerInfo(\"\", null, [new KeyCommandInfo(\"keysItem\", Text.Of(\"keysItem\"), []), new KeyCommandInfo(\"quitItem\", Text.Of(\"quitItem\"), [\"Ctrl+Q\"])])"), "{catalog}");
    // A toolbar command says where it lives, in Clarity's words.
    assert!(catalog.contains("new KeyLayerInfo(\"Card\", Text.Of(\"cardTitle\"), [new KeyCommandInfo(\"Card.printButton\", Text.Of(\"printCaption\"), [], [new Text(\"Toolbar\") { Key = \"keysToolbar\" }]), new KeyCommandInfo(\"Card.saveButton\", Text.Of(\"saveCaption\"), [\"Ctrl+S\"])])"), "{catalog}");
    let model = file(&files, "App/Windows/CardViewModel.cs");
    assert!(model.contains("void Keys_() => ShowKeys();"), "{model}");
}

#[test]
fn without_keys_editable_the_design_keys_stay() {
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Clarity;
    let project = clarity_spec::load_in([("shop.clarity", SHOP)], [], &clarity_project::source_refs(&settings));
    // `-> keys` needs the project file's say.
    let refused = generate(&project, &settings).unwrap_err();
    assert_eq!(refused, ["keysItem clicks to keys, the keys window, which opens when the project file says keys editable"]);
    let plain = SHOP.replace("            item @keysItem name keysItem on click -> keys\n", "");
    let project = clarity_spec::load_in([("shop.clarity", plain.as_str())], [], &clarity_project::source_refs(&settings));
    let files = generate(&project, &settings).unwrap();
    assert!(!file(&files, "Program.cs").contains("PersonKeys"));
    assert!(!files.iter().any(|f| f.path == "KeysCatalog.cs"));
}

#[test]
fn the_web_stacks_carry_the_catalog_in_their_model() {
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp] {
        let code: String = files(backend, clarity_project::usual_ui(backend), true).iter().map(|f| f.text.clone()).collect();
        assert!(code.contains("Card.saveButton"), "{backend:?} carries the catalog");
    }
}
