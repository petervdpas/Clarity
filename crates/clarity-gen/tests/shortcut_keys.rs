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
fn avalonia_has_the_keys_window_and_the_persons_keys() {
    let files = files(Backend::Csharp, Ui::Avalonia, true);
    let catalog = file(&files, "KeysCatalog.cs");
    assert!(catalog.contains("new KeyLayerInfo(\"\", null, [new KeyCommandInfo(\"keysItem\", Text.Of(\"keysItem\"), []), new KeyCommandInfo(\"quitItem\", Text.Of(\"quitItem\"), [\"Ctrl+Q\"])])"), "{catalog}");
    assert!(catalog.contains("new KeyLayerInfo(\"Card\", Text.Of(\"cardTitle\"), [new KeyCommandInfo(\"Card.saveButton\", Text.Of(\"saveCaption\"), [\"Ctrl+S\"])])"), "{catalog}");
    let app = file(&files, "App.axaml.cs");
    assert!(app.contains("PersonKeys.Start(\"Shop\");"), "{app}");
    assert!(app.contains("KeysWindow.Catalog = KeysCatalog.Layers;"), "{app}");
    let view = file(&files, "Windows/CardView.axaml");
    assert!(view.contains("c:Keys.Id=\"keysItem\""), "{view}");
    assert!(view.contains("c:Keys.Id=\"Card.saveButton\""), "{view}");
    let model = file(&files, "Windows/CardViewModel.cs");
    assert!(model.contains("void Keys_() => ShowKeys();"), "{model}");
}

#[test]
fn without_keys_editable_the_design_keys_stay() {
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let project = clarity_spec::load_in([("shop.clarity", SHOP)], [], &clarity_project::source_refs(&settings));
    // `-> keys` needs the project file's say.
    let refused = generate(&project, &settings).unwrap_err();
    assert_eq!(refused, ["keysItem clicks to keys, the keys window, which opens when the project file says keys editable"]);
    let plain = SHOP.replace("            item @keysItem name keysItem on click -> keys\n", "");
    let project = clarity_spec::load_in([("shop.clarity", plain.as_str())], [], &clarity_project::source_refs(&settings));
    let files = generate(&project, &settings).unwrap();
    assert!(!files.iter().any(|f| f.path == "KeysCatalog.cs"));
    assert!(!file(&files, "Windows/CardView.axaml").contains("c:Keys.Id="));
    assert!(!file(&files, "App.axaml.cs").contains("PersonKeys"));
}

#[test]
fn the_web_stacks_carry_the_catalog_in_their_model() {
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp] {
        let code: String = files(backend, Ui::Web, true).iter().map(|f| f.text.clone()).collect();
        assert!(code.contains("Card.saveButton"), "{backend:?} carries the catalog");
    }
}
