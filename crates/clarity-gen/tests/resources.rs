//! Resources: every stack carries the files of the project's `resources`
//! folder. A window's `image source "logo.png"` shows a picture of it (X2),
//! and `markdown source "rules.md"` a Markdown file, in the language shown.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};
use clarity_spec::resources::{self, ResourceFile};
use clarity_spec::{Project, load_sources};

const HOME: &str = "module shop

entity Customer
    name: text(80)

page Home: window
    data notes: text
    image name image1 at 1,1 span 3 rows 3 source \"logo.png\"
    markdown name licenseText at 1,4 span 6 rows 3 source \"license.md\"
    markdown @gameRules name rulesText at 1,7 span 6 rows 3
    image @bannerPicture name banner at 1,10 span 6 rows 2
    markdown name notesText at 1,12 span 6 rows 3 bind notes
";

const LOGO: [u8; 4] = [0x89, b'P', b'N', b'G'];

/// The resources, and the keys that point each language at one of them.
fn project() -> Project {
    let mut project = load_sources([("shop.clarity", HOME)]);
    project.translations.insert("en".into(), Default::default());
    project.translations.insert("nl".into(), Default::default());
    for (language, rules, banner) in [("en", "rules-en.md", "banner-en.png"), ("nl", "rules-nl.md", "banner-nl.png")] {
        let keys = project.resource_keys.entry(language.into()).or_default();
        keys.insert("gameRules".into(), rules.into());
        keys.insert("bannerPicture".into(), banner.into());
    }
    let file = |name: &str, bytes: &[u8]| ResourceFile { name: name.into(), bytes: bytes.to_vec() };
    resources::attach(
        &mut project,
        vec![
            file("logo.png", &LOGO),
            file("banner-en.png", &LOGO),
            file("banner-nl.png", &LOGO),
            file("license.md", b"# License"),
            file("rules-en.md", b"# Rules"),
            file("rules-nl.md", b"# Regels"),
        ],
    );
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    project
}

fn files(backend: Backend, ui: Ui) -> Vec<OutFile> {
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    generate(&project(), &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

/// The program of `backend` on the shared web screen: Rust's is its web
/// program, until Clarity.UI draws in the browser (G8d).
fn web_files(backend: Backend) -> Vec<OutFile> {
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = clarity_project::usual_ui(backend);
    if backend == Backend::Rust {
        settings.stack.targets = vec![clarity_project::Target::Web];
    }
    generate(&project(), &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn bytes<'a>(files: &'a [OutFile], path: &str) -> &'a [u8] {
    let file = files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}"));
    file.bytes.as_deref().unwrap_or_else(|| panic!("{path} is not binary"))
}

fn text<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

/// The web stacks put the resources next to the screen, which shows
/// `resources/logo.png` from its own folder; the Markdown travels in the
/// model, and a key that points at a resource is, in each language, the
/// Markdown of its file or the name of its picture.
#[test]
fn the_web_stacks_carry_the_resources_with_the_renderer() {
    for (backend, model) in [(Backend::Rust, "src/model.json"), (Backend::Go, "model.json")] {
        let files = web_files(backend);
        assert_eq!(bytes(&files, "ui/resources/logo.png"), LOGO, "{backend:?}");
        assert_eq!(bytes(&files, "ui/resources/rules-nl.md"), b"# Regels", "{backend:?}");
        let model = text(&files, model);
        assert!(model.contains("\"license.md\":\"# License\""), "{backend:?}: {model}");
        assert!(model.contains("\"nl\":{\"bannerPicture\":\"banner-nl.png\",\"gameRules\":\"# Regels\"}"), "{backend:?}: {model}");
    }
}

/// C# embeds every resource under its name; a key's Markdown and pictures
/// come through the texts, in the language shown.
#[test]
fn csharp_embeds_the_resources() {
    let files = files(Backend::Csharp, Ui::Clarity);
    assert_eq!(bytes(&files, "resources/logo.png"), LOGO);
    assert_eq!(bytes(&files, "resources/rules-nl.md"), b"# Regels");
    assert!(text(&files, "Shop.csproj").contains("<EmbeddedResource Include=\"resources/**\" LogicalName=\"resources/%(RecursiveDir)%(Filename)%(Extension)\" />"), "{}", text(&files, "Shop.csproj"));
    assert!(text(&files, "Texts.nl.resx").contains("<data name=\"GameRules\" xml:space=\"preserve\">\n    <value># Regels</value>"), "{}", text(&files, "Texts.nl.resx"));
}

/// A Rust program on Clarity.UI carries each resource in the program, by
/// its name, for its parts to show (`image source`, `markdown source`, sounds).
#[test]
fn rust_on_clarity_ui_embeds_the_resources() {
    let files = files(Backend::Rust, Ui::Clarity);
    assert_eq!(bytes(&files, "resources/logo.png"), LOGO);
    let main = text(&files, "src/main.rs");
    assert!(main.contains("    resources: &[(\"banner-en.png\", include_bytes!(\"../resources/banner-en.png\")), "), "{main}");
    assert!(main.contains("(\"logo.png\", include_bytes!(\"../resources/logo.png\"))"), "{main}");
    assert!(main.contains("(\"rules-nl.md\", include_bytes!(\"../resources/rules-nl.md\"))"), "{main}");
}

/// A project without resources carries no folder for them.
#[test]
fn no_resources_no_folder() {
    let project = load_sources([("shop.clarity", "module shop\n\nentity Customer\n    name: text(80)\n\npage Home: window\n    label \"Hi\" name label1 at 1,1 span 2\n")]);
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Clarity;
    let files = generate(&project, &settings).unwrap();
    assert!(!text(&files, "Shop.csproj").contains("Include=\"resources/**\""));
}

/// The one-file page of the C and C++ webview carries its pictures as data
/// URLs, each with its own kind: an SVG as an SVG, which a browser shows only so.
#[test]
fn the_one_file_page_carries_an_svg_as_an_svg() {
    let source = "module art\n\napp Art\n    main Gallery\n\npage Gallery: window\n    image name logo at 1,1 span 3 rows 3 source \"logo.svg\"\n";
    let mut project = clarity_spec::load_sources([("art.clarity", source)]);
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"><circle cx=\"5\" cy=\"5\" r=\"4\"/></svg>".to_vec();
    clarity_spec::resources::attach(&mut project, vec![clarity_spec::resources::ResourceFile { name: "logo.svg".into(), bytes: svg }]);
    let page = clarity_gen::web::single_page(&project);
    assert!(page.contains("\"logo.svg\":\"data:image/svg+xml;base64,"), "{}", &page[page.find("clarityResources").unwrap_or(0)..][..200]);
}
