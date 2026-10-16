//! Transpiles a checked Clarity project into a real application for the
//! stack its `clarity.toml` names (docs/DESIGN.md section 1.2). Generation is
//! deterministic: the same project always gives the same files, byte for
//! byte, so `out/` can be compared, cached and regenerated at any time.

mod print;
mod rust;

use clarity_project::{Backend, Settings};
use clarity_spec::Project;

/// One generated file, by its path inside `out/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutFile {
    pub path: String,
    pub text: String,
}

/// The files of the application, sorted by path; or why it cannot be
/// generated.
pub fn generate(project: &Project, settings: &Settings) -> Result<Vec<OutFile>, Vec<String>> {
    if project.has_errors() {
        return Err(vec!["the project has errors; fix the errors before generating it".to_string()]);
    }
    let mut files = match settings.stack.backend {
        Backend::Rust => rust::generate(project, settings)?,
        other => {
            let name = match other {
                Backend::Go => "Go",
                Backend::Csharp => "C#",
                Backend::C => "C",
                Backend::Cpp => "C++",
                Backend::Rust => "Rust",
            };
            return Err(vec![format!("the {name} backend is not generated yet; Rust is")]);
        }
    };
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Writes `files` into `dir`, which then holds exactly these files: files
/// that changed are rewritten, unchanged ones are left alone (so build
/// tools see no change), and files no longer generated are removed.
/// `keep` names top-level entries that are not generated, such as the
/// build tool's `target` folder.
pub fn write_out(dir: &std::path::Path, files: &[OutFile], keep: &[&str]) -> std::io::Result<usize> {
    let mut written = 0;
    for file in files {
        let path = dir.join(&file.path);
        if std::fs::read_to_string(&path).ok().as_deref() == Some(file.text.as_str()) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &file.text)?;
        written += 1;
    }
    remove_stale(dir, dir, files, keep)?;
    Ok(written)
}

fn remove_stale(root: &std::path::Path, dir: &std::path::Path, files: &[OutFile], keep: &[&str]) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        if dir == root && keep.contains(&relative.as_str()) {
            continue;
        }
        if path.is_dir() {
            remove_stale(root, &path, files, keep)?;
            if std::fs::read_dir(&path)?.next().is_none() {
                std::fs::remove_dir(&path)?;
            }
        } else if !files.iter().any(|f| f.path == relative) {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}
