//! Fixed pictures (X2): every stack carries the files of the project's
//! `images` folder, and a window's `image source "logo.png"` shows one.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};
use clarity_spec::images::{self, ProjectImage};
use clarity_spec::{Project, load_sources};

const HOME: &str = "module shop

entity Customer
    name: text(80)

page Home: window
    image at 1,1 span 3 rows 3 source \"logo.png\"
";

const LOGO: [u8; 4] = [0x89, b'P', b'N', b'G'];

fn project() -> Project {
    let mut project = load_sources([("shop.clarity", HOME)]);
    images::attach(&mut project, vec![ProjectImage { name: "logo.png".into(), bytes: LOGO.to_vec() }]);
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
/// `images/logo.png` from its own folder.
#[test]
fn the_web_stacks_carry_the_pictures_with_the_renderer() {
    for backend in [Backend::Rust, Backend::Go] {
        let files = files(backend, Ui::Web);
        assert_eq!(bytes(&files, "ui/images/logo.png"), LOGO, "{backend:?}");
    }
}

/// Avalonia embeds them as resources and shows them with an Image.
#[test]
fn avalonia_embeds_the_pictures() {
    let files = files(Backend::Csharp, Ui::Avalonia);
    assert_eq!(bytes(&files, "images/logo.png"), LOGO);
    assert!(text(&files, "Shop.csproj").contains("<AvaloniaResource Include=\"images/**\" />"), "{}", text(&files, "Shop.csproj"));
    let view = text(&files, "Windows/HomeView.axaml");
    assert!(view.contains("Source=\"/images/logo.png\" Stretch=\"Uniform\""), "{view}");
}

/// A project without pictures carries no folder for them.
#[test]
fn no_pictures_no_folder() {
    let project = load_sources([("shop.clarity", "module shop\n\nentity Customer\n    name: text(80)\n\npage Home: window\n    label \"Hi\" at 1,1 span 2\n")]);
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let files = generate(&project, &settings).unwrap();
    assert!(!text(&files, "Shop.csproj").contains("AvaloniaResource Include=\"images"));
}
