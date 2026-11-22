//! The version bumps a release needs (clarity-cdev's bump_versions): every
//! release in the chain (`release::chain`) gets the version it would release
//! (`release::Standing::next`) in source, and every Cargo.toml naming that
//! crate with a version names the new one. The Go runtime is left as it is:
//! its version moves in the Go sums step, after its tag.

use std::path::Path;

use super::release::{self, VersionAt};

/// What one bump changed: in which file, from which version to which.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Bump {
    pub file: String,
    pub what: String,
    pub from: String,
    pub to: String,
}

/// Bumps every release in the chain to the version it would release, and the
/// requirements naming its crate; what changed, one line per change. Again,
/// it changes nothing: each is then at its version.
pub fn bump(repo: &Path) -> Result<Vec<Bump>, String> {
    bump_rows(repo, &release::standing(repo)?)
}

/// `bump` of the releases as `standing` says they stand.
pub fn bump_rows(repo: &Path, standing: &[release::Standing]) -> Result<Vec<Bump>, String> {
    let mut done = Vec::new();
    for row in standing.iter().filter(|s| !s.changes.code.is_empty()) {
        let Some(unit) = release::unit(&row.key) else { continue };
        let Some(at) = unit.version else { continue };
        if matches!(at, VersionAt::GoConst(_)) {
            continue;
        }
        let Some(from) = row.source.clone() else { continue };
        if from == row.next {
            continue;
        }
        let file = release::file_of(at);
        let text = release::read(repo, file)?;
        let (pos, written) = release::version_in(&text, at).ok_or_else(|| format!("{file} names no version"))?;
        let prefix = if written.starts_with('v') { "v" } else { "" };
        let changed = format!("{}{prefix}{}{}", &text[..pos], row.next, &text[pos + written.len()..]);
        std::fs::write(repo.join(file), changed).map_err(|e| format!("cannot write {file}: {e}"))?;
        done.push(Bump { file: file.to_string(), what: row.name.clone(), from: from.clone(), to: row.next.clone() });
        if let Some(name) = unit.crate_name {
            done.extend(requirements(repo, name, &from, &row.next)?);
        }
    }
    Ok(done)
}

/// Every Cargo.toml of the repository (committed or new, never ignored)
/// that names `name` at version `from` names it at `to`: `name = "0.1.0"`,
/// `name = { path = ..., version = "0.1.0" }`, and `"=0.1.0"` as the carried crates pin it.
fn requirements(repo: &Path, name: &str, from: &str, to: &str) -> Result<Vec<Bump>, String> {
    let listed = release::git(repo, &["ls-files", "--cached", "--others", "--exclude-standard", "--", "*Cargo.toml"])?;
    let mut done = Vec::new();
    for file in listed.lines().filter(|f| f.ends_with("Cargo.toml")) {
        let Ok(text) = std::fs::read_to_string(repo.join(file)) else { continue };
        let mut changed = false;
        let lines: Vec<String> = text
            .split_inclusive('\n')
            .map(|line| {
                let named = line.trim_start().strip_prefix(name).is_some_and(|rest| rest.trim_start().starts_with('='));
                if !named {
                    return line.to_string();
                }
                let mut out = line.to_string();
                for (old, new) in [(format!("\"={from}\""), format!("\"={to}\"")), (format!("\"{from}\""), format!("\"{to}\""))] {
                    if out.contains(&old) {
                        out = out.replacen(&old, &new, 1);
                        changed = true;
                        break;
                    }
                }
                out
            })
            .collect();
        if changed {
            std::fs::write(repo.join(file), lines.concat()).map_err(|e| format!("cannot write {file}: {e}"))?;
            done.push(Bump { file: file.to_string(), what: format!("{name} required"), from: from.to_string(), to: to.to_string() });
        }
    }
    Ok(done)
}
