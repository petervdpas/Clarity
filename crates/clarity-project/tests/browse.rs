//! What Clarity's own file browser lists of a folder (docs/PROJECT-BROWSER-PLAN.md):
//! folders first, then files, by name; hidden ones left out unless asked;
//! only the kinds of file a use wants; and Clarity's projects marked exactly:
//! a project file (a `.clarity` file whose first line is `project`) with its
//! name and stack, the folder of its modules beside it, and a folder that
//! holds one.

use std::path::{Path, PathBuf};

use clarity_project::browse::{Show, list, stack_words};

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-browse-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn names(listing: &clarity_project::browse::Listing) -> Vec<String> {
    listing.entries.iter().map(|e| e.name.clone()).collect()
}

const SHOP: &str = "// The shop.\nproject Shop\n    title \"My shop\"\n    stack csharp desktop cli\n";

#[test]
fn folders_come_first_then_files_by_name_without_hidden_ones() {
    let dir = folder("order");
    for f in ["b.txt", "A.md", ".secret"] {
        std::fs::write(dir.join(f), "x").unwrap();
    }
    for d in ["zeta", "Alpha", ".git"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    let listing = list(&dir, &Show::Everything, false);
    assert_eq!(names(&listing), ["Alpha", "zeta", "A.md", "b.txt"]);
    assert!(listing.entries[0].folder && !listing.entries[2].folder);
    assert_eq!(listing.up.as_deref(), dir.parent());
    let with_hidden = list(&dir, &Show::Everything, true);
    assert_eq!(names(&with_hidden), [".git", "Alpha", "zeta", ".secret", "A.md", "b.txt"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_use_sees_only_what_it_can_pick() {
    let dir = folder("kinds");
    std::fs::create_dir_all(dir.join("apis")).unwrap();
    for f in ["forms.yaml", "forms.JSON", "notes.md"] {
        std::fs::write(dir.join(f), "x").unwrap();
    }
    assert_eq!(names(&list(&dir, &Show::Folders, false)), ["apis"]);
    let files = Show::Files(vec!["yaml".into(), "yml".into(), "json".into()]);
    assert_eq!(names(&list(&dir, &files, false)), ["apis", "forms.JSON", "forms.yaml"], "folders to go into, and the kinds asked, in any case");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clarity_projects_are_marked_exactly() {
    let dir = folder("projects");
    std::fs::write(dir.join("shop.clarity"), SHOP).unwrap();
    std::fs::create_dir_all(dir.join("shop")).unwrap();
    std::fs::write(dir.join("shop/customers.clarity"), "module customers\n").unwrap();
    std::fs::write(dir.join("broken.clarity"), "project Broken\n    stack nonsense\n").unwrap();
    std::fs::create_dir_all(dir.join("Holder")).unwrap();
    std::fs::write(dir.join("Holder/todo.clarity"), "project Todo\n    stack rust desktop\n").unwrap();
    std::fs::create_dir_all(dir.join("plain")).unwrap();

    let listing = list(&dir, &Show::Everything, false);
    let entry = |name: &str| listing.entries.iter().find(|e| e.name == name).unwrap_or_else(|| panic!("no {name}"));

    let shop = entry("shop.clarity").project.as_ref().expect("a project file");
    assert_eq!((shop.title.as_str(), shop.stack.as_str(), shop.problem.as_deref()), ("My shop", "C#, Clarity.UI, Desktop and CLI", None));
    assert_eq!(entry("shop").modules_of.as_deref(), Some("My shop"), "the folder beside it holds its modules");

    let broken = entry("broken.clarity").project.as_ref().expect("still a project file");
    assert!(broken.problem.is_some() && broken.stack.is_empty(), "{broken:?}");

    let holder = entry("Holder");
    assert_eq!(holder.projects.iter().map(|p| p.title.as_str()).collect::<Vec<_>>(), ["Todo"], "a folder that holds a project");
    assert!(entry("plain").projects.is_empty() && entry("plain").modules_of.is_none());

    // A module is no project, inside its folder too.
    let modules = list(&dir.join("shop"), &Show::Everything, false);
    assert!(modules.entries[0].project.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_folder_that_cannot_be_read_says_so() {
    let gone = std::env::temp_dir().join(format!("clarity-browse-gone-{}", std::process::id()));
    let listing = list(&gone, &Show::Everything, false);
    assert!(listing.entries.is_empty());
    assert!(listing.problem.as_deref().is_some_and(|p| p.contains("cannot read")), "{:?}", listing.problem);
    let file = folder("file").join("a.txt");
    std::fs::write(&file, "x").unwrap();
    assert!(list(&file, &Show::Everything, false).problem.as_deref().is_some_and(|p| p.contains("not a folder")));
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

#[test]
fn a_stack_reads_as_the_status_bar_says_it() {
    use clarity_project::{Backend, Stack, Target, Ui};
    let stack = |backend, targets: &[Target]| Stack { backend, ui: Ui::Clarity, targets: targets.to_vec() };
    assert_eq!(stack_words(&stack(Backend::Go, &[Target::Cli])), "Go, CLI", "no UI without a window");
    assert_eq!(stack_words(&stack(Backend::Rust, &[Target::Desktop, Target::Web])), "Rust, Clarity.UI, Desktop and Web");
}

#[test]
fn every_system_has_a_place_to_start() {
    let roots = clarity_project::browse::roots();
    assert!(!roots.is_empty());
    if cfg!(windows) {
        assert!(roots.iter().all(|r| r.to_string_lossy().ends_with(":\\")), "drives: {roots:?}");
    } else {
        assert_eq!(roots, [Path::new("/")]);
    }
}

/// A project's card shows the opening of its README: the first paragraph
/// after the heading, cut to a couple of lines at a word.
#[test]
fn a_project_tells_the_opening_of_its_readme() {
    let dir = folder("readme");
    std::fs::write(dir.join("shop.clarity"), SHOP).unwrap();
    std::fs::create_dir_all(dir.join("shop")).unwrap();
    let readme = "# Shop\n\nA shop with customers and orders,\nwritten as an example.\n\n## Files\n\nMore.\n";
    std::fs::write(dir.join("shop/README.md"), readme).unwrap();
    let shop = clarity_project::browse::project_info(&dir.join("shop.clarity")).unwrap();
    assert_eq!(shop.readme.as_deref(), Some("A shop with customers and orders, written as an example."));
    let long = format!("# Long\n\n{}\n", "word ".repeat(80));
    std::fs::write(dir.join("shop/README.md"), long).unwrap();
    let cut = clarity_project::browse::project_info(&dir.join("shop.clarity")).unwrap().readme.unwrap();
    assert!(cut.len() <= 203 && cut.ends_with("...") && !cut.contains("wor..."), "{cut}");
    std::fs::remove_file(dir.join("shop/README.md")).unwrap();
    assert_eq!(clarity_project::browse::project_info(&dir.join("shop.clarity")).unwrap().readme, None, "no README, no lines");
    let _ = std::fs::remove_dir_all(&dir);
}
