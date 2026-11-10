//! A turn of an outside agent (docs/ACP-PLAN.md): the project's text files
//! before and after it, compared as one change for Undo however they were
//! written, and what the turn asked Clarity to show.

use std::path::{Path, PathBuf};

use clarity_tools::turn::{self, Shown};

/// A project in a scratch folder: its file `todo.clarity`, its folder `todo/`
/// with a module, and an app folder of its own (`app-out`, never moved by
/// Clarity's output folder setting).
fn project(name: &str) -> (PathBuf, PathBuf) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("turn-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("todo");
    std::fs::create_dir_all(dir.join("design")).unwrap();
    let file = root.join("todo.clarity");
    std::fs::write(&file, "project Todo\n    stack rust desktop\n    output \"app-out\"\n").unwrap();
    std::fs::write(dir.join("todo.clarity"), "module todo\n").unwrap();
    std::fs::write(dir.join("design/Main.cxaml"), "<Window class=\"todo.Main\"/>\n").unwrap();
    (file, dir)
}

fn put(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn a_turn_that_changes_nothing_has_no_changes() {
    let (file, dir) = project("nothing");
    let before = turn::take(&file, &dir);
    let after = turn::take(&file, &dir);
    let changes = turn::changes(&before, &after);
    assert!(changes.files.is_empty());
    assert!(changes.others.is_empty());
}

#[test]
fn added_changed_and_removed_files_each_come_once() {
    let (file, dir) = project("each");
    let before = turn::take(&file, &dir);
    std::fs::write(dir.join("todo.clarity"), "module todo\n\nentity Task\n").unwrap();
    std::fs::write(dir.join("todo.clarity"), "module todo\n\nentity Task\n    title text\n").unwrap();
    put(&dir.join("README.md"), b"# Todo\n");
    std::fs::remove_file(dir.join("design/Main.cxaml")).unwrap();
    let changes = turn::changes(&before, &turn::take(&file, &dir));

    assert_eq!(changes.files.len(), 3, "{:?}", changes.files);
    let of = |path: PathBuf| changes.files.iter().find(|c| c.file == path).unwrap_or_else(|| panic!("{} changed", path.display()));
    let changed = of(dir.join("todo.clarity"));
    assert_eq!(changed.before.as_deref(), Some("module todo\n"));
    assert_eq!(changed.after.as_deref(), Some("module todo\n\nentity Task\n    title text\n"));
    let added = of(dir.join("README.md"));
    assert_eq!((added.before.as_deref(), added.after.as_deref()), (None, Some("# Todo\n")));
    let removed = of(dir.join("design/Main.cxaml"));
    assert_eq!((removed.before.as_deref(), removed.after.as_deref()), (Some("<Window class=\"todo.Main\"/>\n"), None));
}

#[test]
fn the_project_file_beside_the_folder_is_part_of_the_turn() {
    let (file, dir) = project("file");
    let before = turn::take(&file, &dir);
    std::fs::write(&file, "project Todo\n    stack go desktop\n    output \"app-out\"\n").unwrap();
    let changes = turn::changes(&before, &turn::take(&file, &dir));
    assert_eq!(changes.files.len(), 1);
    assert_eq!(changes.files[0].file, file);
}

#[test]
fn the_app_data_build_and_hidden_folders_are_left_out() {
    let (file, dir) = project("skipped");
    let before = turn::take(&file, &dir);
    // The app folder, the databases, a custom part's cargo build, the project's own kept folder, a repository.
    put(&dir.join("app-out/desktop/src/main.rs"), b"fn main() {}\n");
    put(&dir.join("data/todo.db.txt"), b"rows\n");
    put(&dir.join("parts/Arena/target/debug/build.log"), b"compiled\n");
    put(&dir.join(".git/HEAD"), b"ref: refs/heads/main\n");
    let changes = turn::changes(&before, &turn::take(&file, &dir));
    assert!(changes.files.is_empty(), "{:?}", changes.files);
    assert!(changes.others.is_empty(), "{:?}", changes.others);
}

#[test]
fn pictures_and_sounds_are_not_read_as_text_but_their_changes_are_named() {
    let (file, dir) = project("binary");
    put(&dir.join("resources/logo.png"), &[0x89, b'P', b'N', b'G', 0, 1, 2]);
    let before = turn::take(&file, &dir);
    put(&dir.join("resources/logo.png"), &[0x89, b'P', b'N', b'G', 0, 1, 2, 3, 4]);
    put(&dir.join("resources/roll.wav"), b"RIFF....WAVE");
    // Not text by its bytes, whatever its name says.
    put(&dir.join("resources/notes.bin"), &[0, 159, 146, 150]);
    let changes = turn::changes(&before, &turn::take(&file, &dir));
    assert!(changes.files.is_empty(), "{:?}", changes.files);
    let mut others = changes.others.clone();
    others.sort();
    assert_eq!(others, vec![dir.join("resources/logo.png"), dir.join("resources/notes.bin"), dir.join("resources/roll.wav")]);
}

#[test]
fn what_a_turn_shows_is_written_and_read_back_in_order() {
    let (_, dir) = project("shown");
    let file = dir.join("shown.jsonl");
    assert_eq!(turn::shown(&file), Vec::<Shown>::new(), "nothing shown without the file");
    turn::show(&file, &Shown::Window("Main".into())).unwrap();
    turn::show(&file, &Shown::Diagram("billing.mermaid".into())).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "{\"show_window\":\"Main\"}\n{\"show_diagram\":\"billing.mermaid\"}\n");
    let made = dir.join("shop.clarity");
    turn::show(&file, &Shown::Project(made.clone())).unwrap();
    assert_eq!(turn::shown(&file), vec![Shown::Window("Main".into()), Shown::Diagram("billing.mermaid".into()), Shown::Project(made)]);
}

#[test]
fn a_line_that_cannot_be_read_is_skipped() {
    let (_, dir) = project("broken");
    let file = dir.join("shown.jsonl");
    std::fs::write(&file, "{\"show_window\":\"Main\"}\nhalf a li\n{\"other\":1}\n").unwrap();
    assert_eq!(turn::shown(&file), vec![Shown::Window("Main".into())]);
}
