//! The every-part project (tests/fixtures/everything.clarity), made ready
//! for one stack the way the IDE makes a project ready: copied into
//! target/generated, its custom part written in the stack's own language
//! (DESIGN 5.19), the part's starter written, and loaded again. A program
//! served to a browser alone has no custom part, so there the part is left out.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

const PART_LINE: &str = "    native \"parts/Gauge\"\n";

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

/// The library line of the part on a backend, by the stack's first word.
fn library(backend: &str) -> &'static str {
    match backend {
        "rust" => "    crate \"parts/Gauge\"\n",
        "go" => "    go \"parts/gauge\"\n",
        "csharp" => "    nuget \"Newtonsoft.Json\" 13.0.1\n",
        _ => PART_LINE,
    }
}

/// The project ready for `stack` (as a project file writes it, such as
/// `csharp avalonia web`), in `target/generated/<name>`: its project file,
/// the loaded project and its settings.
pub fn prepare(name: &str, stack: &str) -> (PathBuf, clarity_spec::Project, clarity_project::Settings) {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated").join(name);
    let _ = std::fs::remove_dir_all(&root);
    copy(&fixtures.join("everything"), &root.join("everything"));
    let file = root.join("everything.clarity");
    let project_text = std::fs::read_to_string(fixtures.join("everything.clarity")).unwrap().replace("stack csharp avalonia desktop cli", &format!("stack {stack}"));
    std::fs::write(&file, &project_text).unwrap();
    let settings = clarity_project::parse(&project_text).unwrap_or_else(|e| panic!("{stack}: {e:?}"));
    let module = root.join("everything/everything.clarity");
    let mut text = std::fs::read_to_string(&module).unwrap();
    assert!(text.contains(PART_LINE), "the every-part project declares its part with {PART_LINE}");
    if settings.stack.has(clarity_project::Target::Desktop) {
        text = text.replace(PART_LINE, library(stack.split_whitespace().next().unwrap_or_default()));
    } else {
        // In a browser alone: no custom part, nor the code that speaks to it.
        let start = text.find("part Gauge\n").unwrap();
        let end = start + text[start..].find("\n\n").unwrap() + 2;
        text.replace_range(start..end, "");
        text = text.lines().filter(|l| !l.trim_start().starts_with("Gauge name gauge")).collect::<Vec<_>>().join("\n") + "\n";
        let handlers = text.find("    handler gaugeChanged").unwrap();
        text.truncate(handlers);
    }
    std::fs::write(&module, text).unwrap();
    let dir = clarity_project::module_dir(&file);
    let project = clarity_project::load_modules(&dir).unwrap();
    clarity_gen::parts::write_part_folders(&dir, &project).unwrap();
    let project = clarity_project::load_modules(&dir).unwrap();
    (file, project, settings)
}
