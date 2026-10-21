//! The resources of a project: the files of its `resources` folder, which
//! every generated application carries. Pictures (X2), shown by
//! `image source "logo.png"`, and Markdown files, shown by
//! `markdown source "rules.md"`, one file per language: `rules.md` in the
//! default language, `rules.nl.md` in Dutch.

use clarity_spec::resources::{self, ResourceFile};
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
    let project = with_files(&HOME.replace("logo.png", "logo.svg"), &["logo.svg"]);
    assert_eq!(errors(&project), ["'logo.svg' is not a picture every stack can show; use a .png, .jpg, .gif, .bmp or .webp file"]);
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

/// The project's languages, as its translation files give them: English by default, and Dutch.
fn in_languages(source: &str, files: Vec<ResourceFile>) -> Project {
    let mut project = load_sources([("shop.clarity", source)]);
    project.translations.insert("en".into(), Default::default());
    project.translations.insert("nl".into(), Default::default());
    resources::attach(&mut project, files);
    project
}

#[test]
fn a_markdown_document_has_a_file_per_language() {
    let project = in_languages(RULES, vec![text("rules.md", "# Rules\n"), text("rules.nl.md", "# Regels\n"), text("logo.png", "")]);
    assert_eq!(errors(&project), Vec::<String>::new());
    // By name, then by language; the Markdown files are not among the files carried as they are.
    let rules = &project.resources.as_ref().unwrap().markdown["rules.md"];
    assert_eq!((rules["en"].as_str(), rules["nl"].as_str()), ("# Rules\n", "# Regels\n"));
    assert_eq!(project.resources.as_ref().unwrap().files.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["logo.png"]);
}

#[test]
fn a_markdown_document_is_checked_like_a_picture() {
    let missing = in_languages(RULES, vec![text("help.md", "x"), text("help.nl.md", "x")]);
    assert_eq!(errors(&missing), ["markdown 'rules.md' is not in the resources folder; it has help.md"]);
    let none = in_languages(RULES, vec![]);
    assert_eq!(errors(&none), ["markdown 'rules.md' is not in the resources folder, which has no Markdown files yet"]);
    let not_markdown = in_languages(&RULES.replace("rules.md", "rules.txt"), vec![]);
    assert_eq!(errors(&not_markdown), ["'rules.txt' is not a Markdown file; use a .md file"]);
    // Every language has its own file, as every language file has every key.
    let untranslated = in_languages(RULES, vec![text("rules.md", "# Rules")]);
    assert_eq!(errors(&untranslated), ["markdown 'rules.md' has no nl file; add resources/rules.nl.md, as every language has each Markdown file"]);
}

#[test]
fn a_markdown_part_shows_a_file_or_a_text_it_binds_never_a_key() {
    let key = in_languages(&RULES.replace("markdown name", "markdown @rules name"), vec![text("rules.md", "x"), text("rules.nl.md", "x")]);
    assert!(errors(&key).iter().any(|e| e == "a markdown shows a Markdown file of the resources folder (source \"rules.md\") or a text it binds, not a key"), "{:#?}", errors(&key));
    let both = RULES.replace("page Home: window\n", "page Home: window\n    data rules: text\n").replace("source \"rules.md\"", "source \"rules.md\" bind rules");
    let both = in_languages(&both, vec![text("rules.md", "x"), text("rules.nl.md", "x")]);
    assert!(errors(&both).iter().any(|e| e == "a markdown shows a Markdown file (source) or a text it binds, not both"), "{:#?}", errors(&both));
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
    assert_eq!(resources.files, vec![ResourceFile { name: "logo.png".into(), bytes: vec![1, 2, 3] }]);
    assert_eq!(resources.markdown["notes.md"].values().collect::<Vec<_>>(), ["Notes"]);
}
