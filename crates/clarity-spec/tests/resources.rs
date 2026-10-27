//! The resources of a project: the files of its `resources` folder, which
//! every generated application carries. Pictures (X2), shown by
//! `image source "logo.png"`, and Markdown files, shown by
//! `markdown source "rules.md"`. A key can point each language at a
//! resource of its own (`gameRules resource "rules-nl.md"`), which
//! `markdown @gameRules` and `image @bannerPicture` show.

use clarity_spec::resources::{self, ResourceFile};
use clarity_spec::translations::Value;
use clarity_spec::{Project, Severity, load_sources};

const HOME: &str = "module shop

page Home: window
    image name image1 at 1,1 span 3 rows 3 source \"logo.png\"
";

fn file(name: &str) -> ResourceFile {
    ResourceFile { name: name.to_string(), bytes: vec![0x89, b'P', b'N', b'G'] }
}

fn text(name: &str, text: &str) -> ResourceFile {
    ResourceFile { name: name.to_string(), bytes: text.as_bytes().to_vec() }
}

fn with_files(source: &str, names: &[&str]) -> Project {
    let mut project = load_sources([("shop.clarity", source)]);
    resources::attach(&mut project, names.iter().map(|n| file(n)).collect());
    project
}

fn errors(project: &Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

#[test]
fn a_picture_of_the_resources_folder_checks_clean() {
    let project = with_files(HOME, &["banner.jpg", "logo.png"]);
    assert_eq!(errors(&project), Vec::<String>::new());
    let names: Vec<&str> = project.resources.as_ref().unwrap().files.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["banner.jpg", "logo.png"]);
}

#[test]
fn a_picture_that_is_not_in_the_folder_is_an_error() {
    // Only the pictures are offered, not the Markdown files beside them.
    let project = with_files(HOME, &["banner.jpg", "rules.md"]);
    assert_eq!(errors(&project), ["image 'logo.png' is not in the resources folder; it has banner.jpg"]);
    let empty = with_files(HOME, &[]);
    assert_eq!(errors(&empty), ["image 'logo.png' is not in the resources folder, which has no pictures yet"]);
}

#[test]
fn a_picture_must_be_one_every_stack_can_show() {
    let project = with_files(&HOME.replace("logo.png", "logo.tiff"), &["logo.tiff"]);
    assert_eq!(errors(&project), ["'logo.tiff' is not a picture every stack can show; use a .png, .jpg, .gif, .bmp, .webp or .svg file"]);
    // An SVG is a picture too.
    let drawn = with_files(&HOME.replace("logo.png", "logo.svg"), &["logo.svg"]);
    assert_eq!(errors(&drawn), Vec::<String>::new());
}

/// Pictures inside containers count too.
#[test]
fn pictures_in_containers_are_checked() {
    let source = "module shop\n\npage Home: window\n    tabs name tabs1 at 1,1 span 6 rows 6\n        tab \"Cover\"\n            image name image1 at 1,1 span 3 rows 3 source \"cover.jpg\"\n";
    assert_eq!(errors(&with_files(source, &["logo.png"])), ["image 'cover.jpg' is not in the resources folder; it has logo.png"]);
}

/// A project loaded without its folder (from memory) is not checked.
#[test]
fn without_the_folder_nothing_is_checked() {
    let project = load_sources([("shop.clarity", HOME)]);
    assert!(project.resources.is_none());
    assert_eq!(errors(&project), Vec::<String>::new());
}

const RULES: &str = "module shop

page Home: window
    markdown name rulesText at 1,1 span 6 rows 8 source \"rules.md\"
";

/// The project's languages: English by default, and Dutch, with the values its language files give.
fn in_languages(source: &str, files: Vec<ResourceFile>, values: &[(&str, &str, Value)]) -> Project {
    let mut project = load_sources([("shop.clarity", source)]);
    project.translations.insert("en".into(), Default::default());
    project.translations.insert("nl".into(), Default::default());
    for (language, key, value) in values {
        match value {
            Value::Text(text) => {
                project.translations.get_mut(*language).unwrap().insert(key.to_string(), text.clone());
            }
            Value::Resource(file) => {
                project.resource_keys.entry(language.to_string()).or_default().insert(key.to_string(), file.clone());
            }
        }
    }
    resources::attach(&mut project, files);
    project
}

#[test]
fn a_markdown_file_is_a_resource_like_any_other() {
    let project = in_languages(RULES, vec![text("rules.md", "# Rules\n"), text("logo.png", "")], &[]);
    assert_eq!(errors(&project), Vec::<String>::new());
    let resources = project.resources.as_ref().unwrap();
    assert_eq!(resources.markdown["rules.md"], "# Rules\n");
    // Every file is carried, the Markdown files too; names are only names.
    assert_eq!(resources.files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["logo.png", "rules.md"]);
}

#[test]
fn a_markdown_source_is_checked_like_a_picture() {
    let missing = in_languages(RULES, vec![text("help.md", "x")], &[]);
    assert_eq!(errors(&missing), ["markdown 'rules.md' is not in the resources folder; it has help.md"]);
    let none = in_languages(RULES, vec![], &[]);
    assert_eq!(errors(&none), ["markdown 'rules.md' is not in the resources folder, which has no Markdown files yet"]);
    let not_markdown = in_languages(&RULES.replace("rules.md", "rules.txt"), vec![], &[]);
    assert_eq!(errors(&not_markdown), ["'rules.txt' is not a Markdown file; use a .md file"]);
}

const BY_KEY: &str = "module shop

page Home: window
    markdown @gameRules name rulesText at 1,1 span 6 rows 8
    image @bannerPicture name banner at 1,9 span 6 rows 2
    label @saveCaption name saveLabel at 1,11 span 2
";

/// A key can point at a resource per language: which file each language
/// shows is up to its language file.
#[test]
fn a_key_points_each_language_at_a_resource() {
    let files = || vec![text("rules-en.md", "# Rules"), text("rules-nl.md", "# Regels"), file("banner-en.png"), file("banner-nl.png")];
    let values = [
        ("en", "gameRules", Value::Resource("rules-en.md".into())),
        ("nl", "gameRules", Value::Resource("rules-nl.md".into())),
        ("en", "bannerPicture", Value::Resource("banner-en.png".into())),
        ("nl", "bannerPicture", Value::Resource("banner-nl.png".into())),
        ("en", "saveCaption", Value::Text("Save".into())),
        ("nl", "saveCaption", Value::Text("Opslaan".into())),
    ];
    assert_eq!(errors(&in_languages(BY_KEY, files(), &values)), Vec::<String>::new());
    // An image shown by its key has what it shows.
    let warnings: Vec<String> = in_languages(BY_KEY, files(), &values).diagnostics.iter().filter(|d| d.severity == Severity::Warning).map(|d| d.message.clone()).collect();
    assert_eq!(warnings, Vec::<String>::new());

    // A markdown's key points at a Markdown file in every language, and an image's at a picture: never a text.
    let mut text_rules = values.clone();
    text_rules[1] = ("nl", "gameRules", Value::Text("# Regels".into()));
    assert_eq!(
        errors(&in_languages(BY_KEY, files(), &text_rules)),
        ["@gameRules is a text in nl; a markdown shows a Markdown file: point the key at one, as gameRules resource \"rules.md\""]
    );
    let mut wrong = values.clone();
    wrong[1] = ("nl", "gameRules", Value::Resource("banner-nl.png".into()));
    wrong[3] = ("nl", "bannerPicture", Value::Text("Banner".into()));
    assert_eq!(
        errors(&in_languages(BY_KEY, files(), &wrong)),
        [
            "@gameRules points at banner-nl.png in nl; a markdown shows a Markdown file",
            "@bannerPicture is a text in nl; an image shows a picture: point the key at one, as bannerPicture resource \"logo.png\"",
        ]
    );
    let mut gone = values.clone();
    gone[1] = ("nl", "gameRules", Value::Resource("regels.md".into()));
    assert_eq!(errors(&in_languages(BY_KEY, files(), &gone)), ["@gameRules points at regels.md in nl, which is not in the resources folder"]);
    // A part that shows text cannot show a file.
    let mut label = values.clone();
    label[5] = ("nl", "saveCaption", Value::Resource("rules-nl.md".into()));
    assert_eq!(errors(&in_languages(BY_KEY, files(), &label)), ["@saveCaption points at rules-nl.md in nl; a label shows text"]);
}

/// A project folder without a resources folder has no resources: a
/// picture a window shows is missing, not unchecked.
#[test]
fn a_project_folder_without_resources_still_checks_its_pictures() {
    let dir = std::env::temp_dir().join(format!("clarity-no-resources-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("shop.clarity"), HOME).unwrap();
    let project = clarity_spec::load_dir(&dir).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(errors(&project), ["image 'logo.png' is not in the resources folder, which has no pictures yet"]);
    assert_eq!(project.resources.map(|r| r.files.len()), Some(0));
}

#[test]
fn a_project_folder_brings_its_resources() {
    let dir = std::env::temp_dir().join(format!("clarity-resources-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(resources::RESOURCES_DIR)).unwrap();
    std::fs::write(dir.join("shop.clarity"), HOME).unwrap();
    std::fs::write(dir.join("resources/logo.png"), [1, 2, 3]).unwrap();
    std::fs::write(dir.join("resources/notes.md"), "Notes").unwrap();
    std::fs::write(dir.join("resources/.hidden"), [0]).unwrap();
    let project = clarity_spec::load_dir(&dir).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(errors(&project), Vec::<String>::new());
    let resources = project.resources.unwrap();
    assert_eq!(resources.files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["logo.png", "notes.md"]);
    assert_eq!(resources.files[0], ResourceFile { name: "logo.png".into(), bytes: vec![1, 2, 3] });
    assert_eq!(resources.markdown["notes.md"], "Notes");
}

/// Every resource has a type, by its extension: Markdown, a picture (SVG
/// among them), a PDF, a drawing (DXF), or another file the application
/// carries as it is.
#[test]
fn a_resource_has_a_type_by_its_extension() {
    use clarity_spec::resources::{ResourceKind, kind_of, kinds};
    assert_eq!(kind_of("rules-nl.md"), ResourceKind::Markdown);
    assert_eq!(kind_of("Logo.PNG"), ResourceKind::Picture);
    assert_eq!(kind_of("manual.pdf"), ResourceKind::Pdf);
    assert_eq!(kind_of("floor-plan.dxf"), ResourceKind::Drawing);
    assert_eq!(kind_of("icon.svg"), ResourceKind::Picture);
    assert_eq!(kind_of("data.csv"), ResourceKind::File);
    assert_eq!(kind_of("roll.WAV"), ResourceKind::Sound);
    // The types a new resource can have, each with its name and extensions; the first extension is the one a file gets.
    let listed: Vec<(ResourceKind, String, Vec<String>)> = kinds().into_iter().map(|k| (k.kind, k.name, k.extensions)).collect();
    assert_eq!(listed[0], (ResourceKind::Markdown, "Markdown".to_string(), vec!["md".to_string()]));
    assert_eq!(listed[1].0, ResourceKind::Picture);
    assert_eq!(listed[1].2, ["png", "jpg", "jpeg", "gif", "bmp", "webp", "svg"]);
    assert_eq!(listed[2], (ResourceKind::Pdf, "PDF".to_string(), vec!["pdf".to_string()]));
    assert_eq!(listed[3], (ResourceKind::Drawing, "Drawing".to_string(), vec!["dxf".to_string()]));
    assert_eq!(listed[4], (ResourceKind::Sound, "Sound".to_string(), vec!["wav".to_string(), "mp3".to_string(), "flac".to_string()]));
    assert_eq!(listed[5], (ResourceKind::File, "Other file".to_string(), vec![]));
}

const FIELD: &str = "module play

page Play: window
    canvas name field at 1,1 span 6 rows 6 units 160 x 100 on draw: drawField

code Play
    handler drawField
        field.image(10, 20, 17, 5, \"pill.svg\")
";

/// A canvas draws a picture of the resources folder by its name (conformance/drawing.json).
#[test]
fn a_picture_a_canvas_draws_is_in_the_folder() {
    assert_eq!(errors(&with_files(FIELD, &["pill.svg"])), Vec::<String>::new());
    assert_eq!(errors(&with_files(FIELD, &["paddle.svg", "rules.md"])), ["field.image draws 'pill.svg', which is not in the resources folder; it has paddle.svg"]);
    assert_eq!(errors(&with_files(&FIELD.replace("pill.svg", "pill.tiff"), &["pill.tiff"])), ["field.image draws 'pill.tiff', which is not a picture every stack can show; use a .png, .jpg, .gif, .bmp, .webp or .svg file"]);
}
