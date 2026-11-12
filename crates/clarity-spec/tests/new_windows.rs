//! A new window, view, splash, toolbox or app is made as what its file holds
//! (`templates::new_window`), and written as its `.cxaml`: what the old way,
//! a page block of Clarity moved out, gave.

use std::path::Path;

use clarity_spec::model::PageKind;
use clarity_spec::templates::{PageTemplate, new_page, new_window};

fn shop() -> clarity_spec::Project {
    example("shop")
}

/// An example as the IDE loads it: its modules with the data sources its project file names.
fn example(name: &str) -> clarity_spec::Project {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(name);
    clarity_project::load_modules(&dir).unwrap()
}

/// The example's files: its modules and its design files, by path.
fn files_of(name: &str) -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(name);
    let mut out = Vec::new();
    for folder in [dir.clone(), dir.join("design")] {
        for entry in std::fs::read_dir(folder).unwrap().flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "clarity" || e == "cxaml") {
                out.push((p.display().to_string(), std::fs::read_to_string(&p).unwrap()));
            }
        }
    }
    out.sort();
    out
}

/// Every template, added beside a module of an example, writes files that
/// read back clean and give the project no new problem. (Until windows
/// moved into `.cxaml` files each gave what its page block
/// of Clarity, moved into design/, gave.)
#[test]
fn every_template_writes_files_that_load_clean() {
    let shop_entity = shop().entities().find(|e| !e.settings && !e.record).unwrap().name.name.clone();
    for template in [
        PageTemplate::Blank,
        PageTemplate::View,
        PageTemplate::Splash,
        PageTemplate::Toolbox,
        PageTemplate::Form { entity: shop_entity.clone() },
        PageTemplate::Browse { entity: shop_entity.clone() },
        PageTemplate::Settings,
    ] {
        // Settings of a project that has them.
        let example = if template == PageTemplate::Settings { "calculator" } else { "shop" };
        let files = files_of(example);
        let load = |files: &[(String, String)]| clarity_spec::load_sources(files.iter().map(|(p, t)| (p.as_str(), t.as_str())));
        let before = load(&files);
        let (module_path, module_text) = files.iter().find(|(p, _)| p.ends_with(".clarity")).unwrap().clone();
        let added = clarity_spec::cxaml::added(&before, Path::new(&module_path), &module_text, &clarity_spec::edit::Edit::AddWindow { name: "Fresh".into(), template: template.clone() }).unwrap().unwrap();
        for (path, text) in &added.files {
            assert!(clarity_spec::cxaml::read_designed(text, &path.display().to_string()).is_ok(), "{template:?}: {}\n{text}", path.display());
        }
        let mut after: Vec<(String, String)> = files.iter().map(|(p, t)| (p.clone(), if *p == module_path { added.module.clone() } else { t.clone() })).collect();
        after.extend(added.files.iter().map(|(p, t)| (p.display().to_string(), t.clone())));
        let mut after = load(&after);
        // The keys the texts were made, as the language files give them.
        after.translations.entry("en".into()).or_default().extend(added.keys.iter().cloned());
        let mut before = before;
        before.translations.entry("en".into()).or_default();
        assert!(clarity_spec::new_error(&before, &after).is_none(), "{template:?}: {:?}", clarity_spec::new_error(&before, &after));
    }
}

#[test]
fn a_new_window_takes_a_free_name() {
    let project = shop();
    let taken = project.pages().next().unwrap().name.name.clone();
    assert!(new_window(&project, &taken, &PageTemplate::Blank).unwrap_err().contains("already declared"));
    assert!(new_page(&project, "two words", PageKind::View).is_err());
    assert!(clarity_spec::templates::new_app(&project, "Other").unwrap_err().contains("already has an app"));
}

/// Adding a window beside a module: its files in design/, its texts keys,
/// its code in the module's file.
#[test]
fn an_added_window_is_its_files_beside_the_module() {
    use clarity_spec::edit::Edit;
    let project = shop();
    let module = Path::new("/nowhere/shop/sales.clarity");
    let text = "module sales\n\nuse customers\n";
    let entity = project.entities().find(|e| !e.settings && !e.record).unwrap().name.name.clone();
    let added = clarity_spec::cxaml::added(&project, module, text, &Edit::AddWindow { name: "Fresh".into(), template: PageTemplate::Browse { entity } }).unwrap().unwrap();
    let paths: Vec<String> = added.files.iter().map(|(p, _)| p.display().to_string()).collect();
    assert_eq!(paths, ["/nowhere/shop/design/Fresh.cxaml", "/nowhere/shop/design/FreshView.cxaml"]);
    assert!(added.files[0].1.contains("<Window class=\"sales.Fresh\" title=\"@freshTitle\""), "{}", added.files[0].1);
    assert!(added.keys.iter().any(|(k, v)| k == "freshTitle" && v == "Fresh"), "{:?}", added.keys);
    assert!(added.module.starts_with(text.trim_end()) && added.module.contains("code Fresh\n"), "{}", added.module);
    // Another edit adds no files.
    assert!(clarity_spec::cxaml::added(&project, module, text, &Edit::AddEntity { name: "Thing".into() }).is_none());
}

/// Removing a window: its file goes, and its code block in the module with it.
#[test]
fn a_removed_window_takes_its_code_along() {
    let markup = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Window class=\"m.Card\" title=\"Card\">\n</Window>\n";
    let module = "module m\n\ncode Card\n    handler opened\n        close\n\ncode Other\n    handler x\n        close\n";
    let root = clarity_spec::cxaml::read_page(markup, "").0.unwrap().span.start;
    let without = clarity_spec::cxaml::removed(markup, root, module).unwrap().unwrap();
    assert!(!without.contains("code Card") && without.contains("code Other"), "{without}");
    // Anything else of the file is no removal of the window.
    assert!(clarity_spec::cxaml::removed(markup, root + 1, module).is_none());
}
