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
    markdown name rulesText at 1,4 span 6 rows 6 source \"rules.md\"
    markdown name notesText at 1,10 span 6 rows 3 bind notes
";

const LOGO: [u8; 4] = [0x89, b'P', b'N', b'G'];

fn project() -> Project {
    let mut project = load_sources([("shop.clarity", HOME)]);
    project.translations.insert("en".into(), Default::default());
    project.translations.insert("nl".into(), Default::default());
    resources::attach(
        &mut project,
        vec![
            ResourceFile { name: "logo.png".into(), bytes: LOGO.to_vec() },
            ResourceFile { name: "rules.md".into(), bytes: b"# Rules".to_vec() },
            ResourceFile { name: "rules.nl.md".into(), bytes: b"# Regels".to_vec() },
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

/// The web stacks put the pictures next to the renderer, which shows
/// `resources/logo.png` from its own folder; the Markdown travels in the
/// model the renderer reads, every language of it.
#[test]
fn the_web_stacks_carry_the_resources_with_the_renderer() {
    for (backend, model) in [(Backend::Rust, "src/model.json"), (Backend::Go, "model.json")] {
        let files = files(backend, Ui::Web);
        assert_eq!(bytes(&files, "ui/resources/logo.png"), LOGO, "{backend:?}");
        let model = text(&files, model);
        assert!(model.contains("\"markdown\":{\"rules.md\":{\"en\":\"# Rules\",\"nl\":\"# Regels\"}}"), "{backend:?}: {model}");
    }
}

/// Avalonia embeds them as resources, shows a picture with an Image, and a
/// Markdown file with the runtime's MarkdownView, which picks the file of
/// the language shown.
#[test]
fn avalonia_embeds_the_resources() {
    let files = files(Backend::Csharp, Ui::Avalonia);
    assert_eq!(bytes(&files, "resources/logo.png"), LOGO);
    assert_eq!(bytes(&files, "resources/rules.md"), b"# Rules");
    assert_eq!(bytes(&files, "resources/rules.nl.md"), b"# Regels");
    assert!(text(&files, "Shop.csproj").contains("<AvaloniaResource Include=\"resources/**\" />"), "{}", text(&files, "Shop.csproj"));
    let view = text(&files, "Windows/HomeView.axaml");
    assert!(view.contains("Source=\"/resources/logo.png\" Stretch=\"Uniform\""), "{view}");
    assert!(
        view.contains("<c:MarkdownView x:Name=\"rulesText\" Grid.Column=\"0\" Grid.Row=\"3\" Grid.ColumnSpan=\"6\" Grid.RowSpan=\"6\" Source=\"rules.md\" IsEnabled=\"{Binding RulesText.IsEnabled}\" IsVisible=\"{Binding RulesText.IsVisible}\" />"),
        "{view}"
    );
    assert!(view.contains("<c:MarkdownView x:Name=\"notesText\" Grid.Column=\"0\" Grid.Row=\"9\" Grid.ColumnSpan=\"6\" Grid.RowSpan=\"3\" Markdown=\"{Binding Notes}\" "), "{view}");
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
    let main = text(&files, "src/main.rs");
    assert!(main.contains("fn open_link(address: String) -> Result<(), String> {"), "{main}");
    assert!(main.contains("[\"http://\", \"https://\", \"mailto:\"]"), "{main}");
    assert!(main.contains("            open_link,\n"), "{main}");
}
