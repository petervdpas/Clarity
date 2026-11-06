//! The application's icon (docs/APP-ICON-SPLASH-PLAN.md): the project file
//! names a picture among the resources, the checker says when it is not
//! there or no picture, and the model carries it to every stack.

use clarity_project::load_modules;

fn project(name: &str, line: &str, resources: &[&str]) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("clarity-icon-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("shop");
    std::fs::create_dir_all(dir.join("resources")).unwrap();
    std::fs::write(root.join("shop.clarity"), format!("project Shop\n    stack rust desktop\n{line}")).unwrap();
    std::fs::write(dir.join("shop.clarity"), "module shop\n").unwrap();
    for file in resources {
        std::fs::write(dir.join("resources").join(file), b"\x89PNG").unwrap();
    }
    dir
}

fn messages(dir: &std::path::Path) -> Vec<String> {
    load_modules(dir).unwrap().diagnostics.into_iter().map(|d| d.message).collect()
}

#[test]
fn the_icon_is_a_picture_among_the_resources() {
    let named = project("named", "    icon \"app.png\"\n", &["app.png"]);
    assert!(!messages(&named).iter().any(|m| m.contains("icon")), "{:?}", messages(&named));
    assert_eq!(load_modules(&named).unwrap().icon.as_deref(), Some("app.png"), "the model carries it");
    let missing = project("missing", "    icon \"app.png\"\n", &[]);
    assert!(messages(&missing).iter().any(|m| m.contains("there is no app.png in the resources folder")), "{:?}", messages(&missing));
    let text = project("text", "    icon \"notes.md\"\n", &["notes.md"]);
    assert!(messages(&text).iter().any(|m| m.contains("is no picture")), "{:?}", messages(&text));
    let unnamed = project("unnamed", "", &["icon.png"]);
    assert!(messages(&unnamed).iter().any(|m| m.contains("write icon \"icon.png\"")), "{:?}", messages(&unnamed));
    for dir in [named, missing, text, unnamed] {
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }
}
