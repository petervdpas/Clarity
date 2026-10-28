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

fn bytes<'a>(files: &'a [OutFile], path: &str) -> &'a [u8] {
    let file = files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}"));
    file.bytes.as_deref().unwrap_or_else(|| panic!("{path} is not binary"))
}

fn text<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

/// The web stacks put the resources next to the renderer, which shows
/// `resources/logo.png` from its own folder; the Markdown travels in the
/// model, and a key that points at a resource is, in each language, the
/// Markdown of its file or the name of its picture.
#[test]
fn the_web_stacks_carry_the_resources_with_the_renderer() {
    for (backend, model) in [(Backend::Rust, "src/model.json"), (Backend::Go, "model.json")] {
        let files = files(backend, Ui::Web);
        assert_eq!(bytes(&files, "ui/resources/logo.png"), LOGO, "{backend:?}");
        assert_eq!(bytes(&files, "ui/resources/rules-nl.md"), b"# Regels", "{backend:?}");
        let model = text(&files, model);
        assert!(model.contains("\"license.md\":\"# License\""), "{backend:?}: {model}");
        assert!(model.contains("\"nl\":{\"bannerPicture\":\"banner-nl.png\",\"gameRules\":\"# Regels\"}"), "{backend:?}: {model}");
    }
}

/// Avalonia embeds every resource under its name. A Markdown file shows
/// with the runtime's MarkdownView; a key's Markdown and pictures come
/// through the texts, in the language shown.
#[test]
fn avalonia_embeds_the_resources() {
    let files = files(Backend::Csharp, Ui::Avalonia);
    assert_eq!(bytes(&files, "resources/logo.png"), LOGO);
    assert_eq!(bytes(&files, "resources/rules-nl.md"), b"# Regels");
    assert!(text(&files, "Shop.csproj").contains("<AvaloniaResource Include=\"resources/**\" />"), "{}", text(&files, "Shop.csproj"));
    assert!(text(&files, "Texts.nl.resx").contains("<data name=\"GameRules\" xml:space=\"preserve\">\n    <value># Regels</value>"), "{}", text(&files, "Texts.nl.resx"));
    let view = text(&files, "Windows/HomeView.axaml");
    // A picture of the resources by name, SVG or bitmap alike.
    assert!(view.contains("<c:ResourceImage x:Name=\"image1\"") && view.contains("Picture=\"logo.png\" Stretch=\"Uniform\""), "{view}");
    assert!(view.contains("<c:MarkdownView x:Name=\"licenseText\" Grid.Column=\"0\" Grid.Row=\"3\" Grid.ColumnSpan=\"6\" Grid.RowSpan=\"3\" Source=\"license.md\" "), "{view}");
    assert!(view.contains("<c:MarkdownView x:Name=\"rulesText\" Grid.Column=\"0\" Grid.Row=\"6\" Grid.ColumnSpan=\"6\" Grid.RowSpan=\"3\" Markdown=\"{x:Static local:Texts.GameRules}\" "), "{view}");
    assert!(view.contains("<Image x:Name=\"banner\" Grid.Column=\"0\" Grid.Row=\"9\" Grid.ColumnSpan=\"6\" Grid.RowSpan=\"2\" Source=\"{Binding Source={x:Static local:Texts.BannerPicture}, Converter={x:Static c:ResourcePictures.Named}}\" Stretch=\"Uniform\""), "{view}");
    assert!(view.contains("<c:MarkdownView x:Name=\"notesText\" Grid.Column=\"0\" Grid.Row=\"11\" Grid.ColumnSpan=\"6\" Grid.RowSpan=\"3\" Markdown=\"{Binding Notes}\" "), "{view}");
    // The pictures come from Clarity.Avalonia's ResourcePictures, which reads SVG too.
    assert!(!text(&files, "Shown.cs").contains("new Bitmap("), "{}", text(&files, "Shown.cs"));
}

/// A project without resources carries no folder for them.
#[test]
fn no_resources_no_folder() {
    let project = load_sources([("shop.clarity", "module shop\n\nentity Customer\n    name: text(80)\n\npage Home: window\n    label \"Hi\" name label1 at 1,1 span 2\n")]);
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let files = generate(&project, &settings).unwrap();
    assert!(!text(&files, "Shop.csproj").contains("AvaloniaResource Include=\"resources"));
}

/// A link of a markdown part opens in the browser through the shell; the
/// Rust stack's shell is generated, and opens only web and mail addresses.
#[test]
fn the_rust_shell_opens_links() {
    let files = files(Backend::Rust, Ui::Web);
    // The shell's commands are in the desktop module.
    let main = text(&files, "src/desktop.rs");
    assert!(main.contains("fn open_link(address: String) -> std::result::Result<(), String> {"), "{main}");
    assert!(main.contains("[\"http://\", \"https://\", \"mailto:\"]"), "{main}");
    assert!(main.contains("            open_link,\n"), "{main}");
}
