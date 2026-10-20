//! Fixed pictures (X2): `image source "logo.png"` shows a file of the
//! project's `images` folder, which every generated application carries.

use clarity_spec::images::{self, ProjectImage};
use clarity_spec::{Project, Severity, load_sources};

const HOME: &str = "module shop

page Home: window
    image name image1 at 1,1 span 3 rows 3 source \"logo.png\"
";

fn picture(name: &str) -> ProjectImage {
    ProjectImage { name: name.to_string(), bytes: vec![0x89, b'P', b'N', b'G'] }
}

fn with_images(source: &str, names: &[&str]) -> Project {
    let mut project = load_sources([("shop.clarity", source)]);
    images::attach(&mut project, names.iter().map(|n| picture(n)).collect());
    project
}

fn errors(project: &Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

#[test]
fn a_picture_of_the_images_folder_checks_clean() {
    let project = with_images(HOME, &["banner.jpg", "logo.png"]);
    assert_eq!(errors(&project), Vec::<String>::new());
    let names: Vec<&str> = project.images.as_ref().unwrap().iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["banner.jpg", "logo.png"]);
}

#[test]
fn a_picture_that_is_not_in_the_folder_is_an_error() {
    let project = with_images(HOME, &["banner.jpg"]);
    assert_eq!(errors(&project), ["image 'logo.png' is not in the images folder; it has banner.jpg"]);
    let empty = with_images(HOME, &[]);
    assert_eq!(errors(&empty), ["image 'logo.png' is not in the images folder, which has no pictures yet"]);
}

#[test]
fn a_picture_must_be_one_every_stack_can_show() {
    let project = with_images(&HOME.replace("logo.png", "logo.svg"), &["logo.svg"]);
    assert_eq!(errors(&project), ["'logo.svg' is not a picture every stack can show; use a .png, .jpg, .gif, .bmp or .webp file"]);
}

/// Pictures inside containers count too.
#[test]
fn pictures_in_containers_are_checked() {
    let source = "module shop\n\npage Home: window\n    tabs name tabs1 at 1,1 span 6 rows 6\n        tab \"Cover\"\n            image name image1 at 1,1 span 3 rows 3 source \"cover.jpg\"\n";
    assert_eq!(errors(&with_images(source, &["logo.png"])), ["image 'cover.jpg' is not in the images folder; it has logo.png"]);
}

/// A project loaded without its folder (from memory) is not checked.
#[test]
fn without_the_folder_nothing_is_checked() {
    let project = load_sources([("shop.clarity", HOME)]);
    assert!(project.images.is_none());
    assert_eq!(errors(&project), Vec::<String>::new());
}

#[test]
fn a_project_folder_brings_its_pictures() {
    let dir = std::env::temp_dir().join(format!("clarity-images-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(images::IMAGES_DIR)).unwrap();
    std::fs::write(dir.join("shop.clarity"), HOME).unwrap();
    std::fs::write(dir.join("images/logo.png"), [1, 2, 3]).unwrap();
    std::fs::write(dir.join("images/.hidden"), [0]).unwrap();
    let project = clarity_spec::load_dir(&dir).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(errors(&project), Vec::<String>::new());
    assert_eq!(project.images, Some(vec![ProjectImage { name: "logo.png".into(), bytes: vec![1, 2, 3] }]));
}
