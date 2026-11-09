//! An application's version (docs/VERSION-PLAN.md): the project file says
//! it (`version "1.2.0"`), the checker says when it is not a version, and
//! the model carries it to every stack.

use clarity_project::{load, load_modules};

fn project(name: &str, line: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("clarity-version-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("shop");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(root.join("shop.clarity"), format!("project Shop\n    stack rust desktop\n{line}")).unwrap();
    std::fs::write(dir.join("shop.clarity"), "module shop\n").unwrap();
    dir
}

#[test]
fn the_project_file_says_its_version() {
    let dir = project("named", "    version \"1.2.0\"\n");
    let file = load(&dir.parent().unwrap().join("shop.clarity"));
    assert!(file.problems.is_empty(), "{:?}", file.problems);
    assert_eq!(file.settings.as_ref().unwrap().version.as_deref(), Some("1.2.0"));
    assert_eq!(load_modules(&dir).unwrap().version.as_deref(), Some("1.2.0"), "the model carries it");
    let pre = project("pre", "    version \"2.0.0-beta.1\"\n");
    assert_eq!(load_modules(&pre).unwrap().version.as_deref(), Some("2.0.0-beta.1"));
}

#[test]
fn a_version_is_optional() {
    let dir = project("none", "");
    assert!(load(&dir.parent().unwrap().join("shop.clarity")).problems.is_empty());
    assert_eq!(load_modules(&dir).unwrap().version, None);
}

#[test]
fn a_version_is_major_minor_patch() {
    for wrong in ["1.2", "v1.2.0", "1.2.x", "", "1.2.0-"] {
        let dir = project("wrong", &format!("    version \"{wrong}\"\n"));
        let file = load(&dir.parent().unwrap().join("shop.clarity"));
        assert!(file.problems.iter().any(|p| p.contains("major.minor.patch")), "{wrong}: {:?}", file.problems);
    }
}

fn with_code(name: &str, line: &str, module: &str, english: &str) -> std::path::PathBuf {
    let dir = project(name, line);
    clarity_spec::cxaml::write_with_pages_moved_out(&dir.join("shop.clarity"), module).unwrap();
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    std::fs::write(dir.join("translations/en.clarity"), english).unwrap();
    dir
}

const MODULE: &str = "module shop
app Shop
    main Main
    on start: started
page Main: window
    title @mainTitle
    label @versionLine name versionLabel at 1,1 span 4
code Shop
    handler started
        let shown = app.version
";

const ENGLISH: &str = "translation en
    mainTitle \"Shop\"
    versionLine \"Version {app.version}\"
";

/// Code reads `app.version`, and a text shows `{app.version}` with no value
/// given: Clarity fills it. Without a version in the project file, either is an error.
#[test]
fn code_and_texts_show_the_version() {
    let dir = with_code("shown", "    version \"1.2.0\"\n", MODULE, ENGLISH);
    let errors: Vec<String> = load_modules(&dir).unwrap().diagnostics.into_iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let without = with_code("unversioned", "", MODULE, ENGLISH);
    let errors: Vec<String> = load_modules(&without).unwrap().diagnostics.into_iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message).collect();
    assert!(errors.iter().any(|e| e.contains("the project has no version")), "{errors:#?}");
    let texts_only = with_code("texts", "", &MODULE.replace("        let shown = app.version\n", "        message @mainTitle\n"), ENGLISH);
    let errors: Vec<String> = load_modules(&texts_only).unwrap().diagnostics.into_iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message).collect();
    assert!(errors.iter().any(|e| e.contains("the project has no version")), "a text that shows it needs one too: {errors:#?}");
}

/// The project's title is the program's name (its settings folder, its
/// package): plain text, so a slot in it is refused with where the version goes.
#[test]
fn the_project_s_title_takes_no_slots() {
    let dir = project("title", "    title \"Todo v{version}\"\n");
    let file = load(&dir.parent().unwrap().join("shop.clarity"));
    assert!(file.problems.iter().any(|p| p.contains("the project's title is plain text") && p.contains("{app.version}")), "{:?}", file.problems);
}
