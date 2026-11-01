//! The screen package travels whole inside every generated web project
//! (DESIGN 7.8): the generator carries each of its files, as it carries the
//! C runtime. A file added to the package and not to the list fails here.
//! `CLARITY_WRITE_SCREEN_FILES=1 cargo test -p clarity-gen --test screen_package`
//! writes the list again.

use std::path::{Path, PathBuf};

fn package_files(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            package_files(&path, root, out);
        } else if !name.ends_with(".test.ts") {
            out.push(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
        }
    }
}

#[test]
fn the_generator_carries_every_file_of_the_screen_package() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/web");
    let mut files = Vec::new();
    package_files(&root, &root, &mut files);
    if std::env::var("CLARITY_WRITE_SCREEN_FILES").is_ok() {
        let mut list = String::from("// The files of the screen package, runtimes/web, which every generated web\n// project carries (DESIGN 7.8). Written by tests/screen_package.rs.\npub(crate) const SCREEN_FILES: &[(&str, &str)] = &[\n");
        for file in &files {
            list.push_str(&format!("    ({file:?}, include_str!(\"../../../runtimes/web/{file}\")),\n"));
        }
        list.push_str("];\n");
        std::fs::write(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/screen_files.rs"), list).unwrap();
    }
    let carried: Vec<&str> = clarity_gen::screen_package::files().iter().map(|(path, _)| *path).collect();
    assert_eq!(carried, files, "write the list again with CLARITY_WRITE_SCREEN_FILES=1");
}
