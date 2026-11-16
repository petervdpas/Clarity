//! DevTools' release planning: Clarity's releases, read from the
//! repository as it is (its tags, the files changed since each sorted by
//! kind, the versions in source), the chain of steps a release would start,
//! the checks a release needs first and how each last ended. The IDE shows
//! it for review; the agent reads it through MCP (`release_state`) and
//! writes the Release Helper's plan itself. Git is only read here; the
//! person commits, pushes and tags.

use std::path::{Path, PathBuf};
use std::process::Command;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::jobs::Runnable;

/// Where a release's version is written in the repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionAt {
    /// The `[package]` version of a Cargo.toml; the crates naming it follow a bump.
    Cargo(&'static str),
    /// The `<Version>` of a props file.
    Props(&'static str),
    /// `const Version = "v0.64.0"` of a Go file.
    GoConst(&'static str),
    /// The `"version"` of a package.json.
    PackageJson(&'static str),
}

/// How the Release Helper sees a release is out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Out {
    /// crates.io has the crate's version.
    Crates,
    /// nuget.org has the version of every package (runtimes/csharp's packable projects).
    Nuget,
    /// The GitHub release of the tag has every asset the last one had, at the new version.
    GithubRelease,
    /// proxy.golang.org knows the version of this module.
    GoProxy(&'static str),
}

/// A release of Clarity: a tag that starts a workflow publishing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unit {
    pub key: &'static str,
    pub name: &'static str,
    /// The tag before its version: `runtimes/rust/v`.
    pub tag: &'static str,
    pub workflow: Option<&'static str>,
    /// Where its version is; none when the tag alone names it (the IDE).
    pub version: Option<VersionAt>,
    /// The crate it publishes, for a crate.
    pub crate_name: Option<&'static str>,
    /// What it is made from: a file changed under one since its tag asks for a release.
    pub sources: &'static [&'static str],
    pub out: Out,
    /// The next release may go once its workflow passed (nuget.org lists slowly).
    pub next_after_workflow: bool,
}

/// A crate released to crates.io from `folder`, by the tag `<folder>/v`.
const fn krate(key: &'static str, name: &'static str, tag: &'static str, workflow: &'static str, manifest: &'static str, sources: &'static [&'static str]) -> Unit {
    Unit { key, name, tag, workflow: Some(workflow), version: Some(VersionAt::Cargo(manifest)), crate_name: Some(name), sources, out: Out::Crates, next_after_workflow: false }
}

/// The Go runtime's module.
pub const GO_MODULE: &str = "github.com/petervdpas/Clarity/runtimes/go";

/// Clarity's releases, in the order they go: a crate after the crates it
/// names (tested against their Cargo.toml), the library before what
/// fetches it, the Go sums after the Go tag, the IDE last.
pub const UNITS: [Unit; 15] = [
    krate("clarityFormats", "clarity-formats", "runtimes/rust-formats/v", "release-rust-formats.yml", "runtimes/rust-formats/Cargo.toml", &["runtimes/rust-formats"]),
    krate("clarityImage", "clarity-image", "runtimes/rust-image/v", "release-rust-image.yml", "runtimes/rust-image/Cargo.toml", &["runtimes/rust-image"]),
    krate("clarityAuth", "clarity-auth", "runtimes/rust-auth/v", "release-rust-auth.yml", "runtimes/rust-auth/Cargo.toml", &["runtimes/rust-auth"]),
    krate("clarityAppMcp", "clarity-app-mcp", "runtimes/rust-app-mcp/v", "release-rust-app-mcp.yml", "runtimes/rust-app-mcp/Cargo.toml", &["runtimes/rust-app-mcp"]),
    krate("clarityEngine", "clarity-engine", "runtimes/rust-engine/v", "release-rust-engine.yml", "runtimes/rust-engine/Cargo.toml", &["runtimes/rust-engine"]),
    krate("clarityRuntime", "clarity-runtime", "runtimes/rust/v", "release-rust-runtime.yml", "runtimes/rust/Cargo.toml", &["runtimes/rust"]),
    krate("clarityAudio", "clarity-audio", "runtimes/rust-audio/v", "release-rust-audio.yml", "runtimes/rust-audio/Cargo.toml", &["runtimes/rust-audio"]),
    krate("clarityVideo", "clarity-video", "runtimes/rust-video/v", "release-rust-video.yml", "runtimes/rust-video/Cargo.toml", &["runtimes/rust-video"]),
    krate("clarityGraph", "clarity-graph", "runtimes/rust-graph/v", "release-rust-graph.yml", "runtimes/rust-graph/Cargo.toml", &["runtimes/rust-graph"]),
    krate("clarityUi", "clarity-ui", "runtimes/rust-ui/v", "release-rust-ui.yml", "runtimes/rust-ui/Cargo.toml", &["runtimes/rust-ui"]),
    Unit {
        key: "clarityUiLibrary",
        name: "Clarity.UI library",
        tag: "runtimes/rust-ui-ffi/v",
        workflow: Some("release-clarity-ui.yml"),
        version: Some(VersionAt::Cargo("runtimes/rust-ui-ffi/Cargo.toml")),
        crate_name: None,
        // Its archives hold every native library and the browser files (release-clarity-ui.yml).
        sources: &["runtimes/rust", "runtimes/rust-ui", "runtimes/rust-ui-ffi", "runtimes/rust-ui-browser", "runtimes/rust-audio", "runtimes/rust-audio-ffi", "runtimes/rust-video", "runtimes/rust-video-ffi", "runtimes/rust-image", "runtimes/rust-image-ffi", "runtimes/rust-formats", "runtimes/rust-formats-ffi", "runtimes/rust-auth", "runtimes/rust-auth-ffi", "runtimes/rust-app-mcp", "runtimes/rust-app-mcp-ffi"],
        out: Out::GithubRelease,
        next_after_workflow: false,
    },
    Unit { key: "csharpRuntime", name: "C# runtime", tag: "runtimes/csharp/v", workflow: Some("release-csharp-runtime.yml"), version: Some(VersionAt::Props("runtimes/csharp/Directory.Build.props")), crate_name: None, sources: &["runtimes/csharp"], out: Out::Nuget, next_after_workflow: true },
    Unit { key: "goRuntime", name: "Go runtime", tag: "runtimes/go/v", workflow: None, version: Some(VersionAt::GoConst("runtimes/go/clarity/version.go")), crate_name: None, sources: &["runtimes/go"], out: Out::GoProxy(GO_MODULE), next_after_workflow: false },
    Unit { key: "vscode", name: "VS Code extension", tag: "editors/vscode/v", workflow: Some("release-vscode.yml"), version: Some(VersionAt::PackageJson("editors/vscode/package.json")), crate_name: None, sources: &["editors/vscode"], out: Out::GithubRelease, next_after_workflow: false },
    Unit { key: "ide", name: "Clarity IDE", tag: "v", workflow: Some("release.yml"), version: None, crate_name: None, sources: &["."], out: Out::GithubRelease, next_after_workflow: false },
];

/// The releases, in their order.
pub fn units() -> impl Iterator<Item = &'static Unit> {
    UNITS.iter()
}

/// The release kept under `key`.
pub fn unit(key: &str) -> Option<&'static Unit> {
    units().find(|u| u.key == key)
}

fn position(key: &str) -> Option<usize> {
    UNITS.iter().position(|u| u.key == key)
}

/// A step a release starts: the checks on main, a tag with its workflow and
/// where it shows, or a step done by hand (the Go sums).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub title: String,
    pub tag: Option<String>,
    pub workflow: Option<String>,
    /// Where the release shows once its workflow passed: `crates.io: clarity-runtime`.
    pub shows: Option<String>,
    /// The steps before it that it needs: the checks, the crates it names, the Go tag.
    pub waits_for: Vec<String>,
    /// The next step may go once this one's workflow passed (nuget.org lists slowly).
    pub next_after_workflow: bool,
}

/// Where a release shows once its workflow passed.
fn shows(unit: &Unit) -> String {
    match unit.out {
        Out::Crates => format!("crates.io: {}", unit.crate_name.unwrap_or(unit.name)),
        Out::Nuget => "nuget.org: the C# runtime's packages".to_string(),
        Out::GithubRelease => format!("a GitHub release with its files: {}", unit.name),
        Out::GoProxy(module) => format!("the Go proxy: {module}"),
    }
}

/// The steps releasing `picked` (each with its new version) start, in the
/// table's order: the checks on main, then each release's tag waiting for
/// the crates it names (`needs`) or else the step before it, the Go sums
/// after the Go tag.
pub fn chain_of(picked: &[(&Unit, String)], needs: impl Fn(&Unit) -> Vec<String>) -> Vec<Event> {
    let checks = "Checks on main".to_string();
    let mut events = vec![Event { title: checks.clone(), tag: None, workflow: Some("checks.yml".into()), shows: None, waits_for: vec![], next_after_workflow: false }];
    let mut ordered: Vec<&(&Unit, String)> = picked.iter().collect();
    ordered.sort_by_key(|(u, _)| position(u.key));
    let mut crates: Vec<(String, String)> = Vec::new();
    for (unit, version) in ordered {
        let title = format!("{} {version}", unit.name);
        let named: Vec<String> = needs(unit).iter().filter_map(|c| crates.iter().find(|(name, _)| name == c).map(|(_, t)| t.clone())).collect();
        let waits_for = if named.is_empty() { vec![events.last().map(|e| e.title.clone()).unwrap_or(checks.clone())] } else { named };
        events.push(Event { title: title.clone(), tag: Some(format!("{}{version}", unit.tag)), workflow: unit.workflow.map(str::to_string), shows: Some(shows(unit)), waits_for, next_after_workflow: unit.next_after_workflow });
        if let Some(name) = unit.crate_name {
            crates.push((name.to_string(), title.clone()));
        }
        if matches!(unit.out, Out::GoProxy(_)) {
            events.push(Event { title: "Go sums".into(), tag: None, workflow: None, shows: Some("the examples' go.sum, committed by hand".into()), waits_for: vec![title], next_after_workflow: false });
        }
    }
    events
}

/// The steps releasing every release whose code changed since its tag, at the version it would release.
pub fn chain(repo: &Path, standing: &[Standing]) -> Vec<Event> {
    let picked: Vec<(&Unit, String)> = standing.iter().filter(|s| !s.changes.code.is_empty()).filter_map(|s| Some((unit(&s.key)?, s.next.clone()))).collect();
    chain_of(&picked, |u| needs(repo, u))
}

/// A version's numbers: `0.61.0` and `v0.61.0` give [0, 61, 0].
pub fn parse(version: &str) -> Option<Vec<u64>> {
    let digits = version.strip_prefix('v').unwrap_or(version);
    let numbers: Option<Vec<u64>> = digits.split('.').map(|n| n.parse().ok()).collect();
    numbers.filter(|n| n.len() == 3)
}

/// The next minor version: `1.1.0` gives `1.2.0` (the runtimes stay 0.x past .99).
pub fn next_minor(version: &str) -> Option<String> {
    let n = parse(version)?;
    Some(format!("{}.{}.0", n[0], n[1] + 1))
}

/// The newest of `tags` that start with `prefix` and end in a version.
pub fn newest(tags: &[String], prefix: &str) -> Option<String> {
    tags.iter().filter_map(|t| Some((parse(t.strip_prefix(prefix)?)?, t))).max_by(|a, b| a.0.cmp(&b.0)).map(|(_, t)| t.clone())
}

fn read(repo: &Path, file: &str) -> Result<String, String> {
    std::fs::read_to_string(repo.join(file)).map_err(|e| format!("cannot read {file}: {e}"))
}

/// The text after `start` up to the next `end`, and where it starts.
fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<(usize, &'a str)> {
    let at = text.find(start)? + start.len();
    let len = text[at..].find(end)?;
    Some((at, &text[at..at + len]))
}

fn version_in(text: &str, at: VersionAt) -> Option<(usize, String)> {
    let (pos, v) = match at {
        VersionAt::Cargo(_) => between(text, "\nversion = \"", "\"")?,
        VersionAt::Props(_) => between(text, "<Version>", "</Version>")?,
        VersionAt::GoConst(_) => between(text, "const Version = \"", "\"")?,
        VersionAt::PackageJson(_) => between(text, "\"version\": \"", "\"")?,
    };
    Some((pos, v.to_string()))
}

fn file_of(at: VersionAt) -> &'static str {
    match at {
        VersionAt::Cargo(f) | VersionAt::Props(f) | VersionAt::GoConst(f) | VersionAt::PackageJson(f) => f,
    }
}

/// The version in source of `unit`, without a leading `v`.
pub fn source_version(repo: &Path, unit: &Unit) -> Result<String, String> {
    let at = unit.version.ok_or_else(|| format!("{}'s version is its tag alone", unit.name))?;
    let text = read(repo, file_of(at))?;
    let (_, v) = version_in(&text, at).ok_or_else(|| format!("{} names no version", file_of(at)))?;
    let v = v.strip_prefix('v').unwrap_or(&v).to_string();
    parse(&v).map(|_| v.clone()).ok_or_else(|| format!("{}: {v} is no version", file_of(at)))
}

/// The workspace's members, from the repository's Cargo.toml.
pub fn members(repo: &Path) -> Vec<String> {
    let text = read(repo, "Cargo.toml").unwrap_or_default();
    let Some((_, list)) = between(&text, "members = [", "]") else { return Vec::new() };
    list.split(',').map(|m| m.trim().trim_matches('"').to_string()).filter(|m| !m.is_empty()).collect()
}

/// The clarity-* crates `unit`'s Cargo.toml names with a version, which crates.io must have first.
pub fn needs(repo: &Path, unit: &Unit) -> Vec<String> {
    let Some(VersionAt::Cargo(manifest)) = unit.version else { return Vec::new() };
    let Some(own) = unit.crate_name else { return Vec::new() };
    let text = read(repo, manifest).unwrap_or_default();
    let mut needed: Vec<String> = text
        .lines()
        .filter_map(|line| {
            let (name, rest) = line.split_once(" = {")?;
            let name = name.trim();
            (name.starts_with("clarity-") && name != own && rest.contains("version = \"")).then(|| name.to_string())
        })
        .collect();
    needed.sort();
    needed.dedup();
    needed
}


fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").args(args).current_dir(repo).output().map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(format!("git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// The repository's tags.
pub fn tags(repo: &Path) -> Result<Vec<String>, String> {
    Ok(git(repo, &["tag", "-l"])?.lines().map(str::to_string).collect())
}

/// The files under `sources` that differ from `tag` (committed since, or
/// not committed yet), and the new files git does not ignore.
pub fn changed_since(repo: &Path, tag: &str, sources: &[&str]) -> Result<Vec<String>, String> {
    let mut args = vec!["diff", "--name-only", tag, "--"];
    args.extend(sources);
    let mut files: Vec<String> = git(repo, &args)?.lines().map(str::to_string).collect();
    let mut others = vec!["ls-files", "--others", "--exclude-standard", "--"];
    others.extend(sources);
    files.extend(git(repo, &others)?.lines().map(str::to_string));
    files.sort();
    files.dedup();
    Ok(files)
}

/// A release's changed files by kind: a manifest, a test or a text alone is
/// no reason to release it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Changes {
    pub code: Vec<String>,
    /// Cargo.toml, package.json, .csproj, .props, go.mod and their locks.
    pub manifest: Vec<String>,
    /// Under a tests/ folder or a *.Tests project, *_test.go, *.test.ts.
    pub tests: Vec<String>,
    /// Markdown and licences.
    pub docs: Vec<String>,
}

const MANIFESTS: [&str; 6] = ["Cargo.toml", "Cargo.lock", "package.json", "package-lock.json", "go.mod", "go.sum"];

/// `files` sorted by kind, each kind in the order given.
pub fn changes(files: &[String]) -> Changes {
    let mut c = Changes::default();
    for file in files {
        let mut parts: Vec<&str> = file.split('/').collect();
        let name = parts.pop().unwrap_or_default();
        let kind = if MANIFESTS.contains(&name) || name.ends_with(".csproj") || name.ends_with(".props") {
            &mut c.manifest
        } else if parts.iter().any(|d| *d == "tests" || d.ends_with(".Tests")) || name.ends_with("_test.go") || name.contains(".test.") {
            &mut c.tests
        } else if name.ends_with(".md") || name.starts_with("LICENSE") {
            &mut c.docs
        } else {
            &mut c.code
        };
        kind.push(file.clone());
    }
    c
}

/// How a release stands now.
#[derive(Debug, Clone, Serialize)]
pub struct Standing {
    pub key: String,
    pub name: String,
    /// Its newest tag, and the version that released.
    pub tag: Option<String>,
    pub released: Option<String>,
    /// The version in source; none when the tag alone names it.
    pub source: Option<String>,
    /// The files changed since its tag.
    pub changed: Vec<String>,
    /// Those files by kind.
    pub changes: Changes,
    /// The version it would release: the source's when it is past the
    /// released one, else the next minor one.
    pub next: String,
    /// The version in source is past the released one.
    pub bumped: bool,
    /// It has a version in source to bump.
    pub bumpable: bool,
}

/// How every release stands.
pub fn standing(repo: &Path) -> Result<Vec<Standing>, String> {
    let tags = tags(repo)?;
    units()
        .map(|unit| {
            let tag = newest(&tags, unit.tag);
            let released = tag.as_deref().and_then(|t| t.strip_prefix(unit.tag)).map(|v| v.strip_prefix('v').unwrap_or(v).to_string());
            let source = unit.version.map(|_| source_version(repo, unit)).transpose()?;
            let changed = match &tag {
                Some(t) => changed_since(repo, t, unit.sources)?,
                None => vec![format!("{} was never released", unit.name)],
            };
            let bumped = match (&source, &released) {
                (Some(s), Some(r)) => parse(s) > parse(r),
                (Some(_), None) => true,
                _ => false,
            };
            let next = match (&source, &released) {
                (Some(s), _) if bumped => s.clone(),
                (_, Some(r)) => next_minor(r).unwrap_or_default(),
                (Some(s), None) => s.clone(),
                (None, None) => "0.1.0".to_string(),
            };
            let changes = if tag.is_some() { changes(&changed) } else { Changes::default() };
            Ok(Standing { key: unit.key.into(), name: unit.name.into(), tag, released, source, changed, changes, next, bumped, bumpable: unit.version.is_some() })
        })
        .collect()
}

/// A generated-file guard: its test writes the file again with the
/// variable set; without, it fails when the file is behind (CLAUDE.md).
#[derive(Debug, Clone, Copy)]
pub struct Guard {
    pub variable: &'static str,
    pub package: &'static str,
    /// The test target; none for the package's lib tests.
    pub test: Option<&'static str>,
    pub writes: &'static str,
}

/// Every guard, in CLAUDE.md's order; a variable without a name writes as it tests (ts-rs).
pub const GUARDS: [Guard; 9] = [
    Guard { variable: "CLARITY_WRITE_GRAMMAR", package: "clarity-spec", test: Some("highlight"), writes: "the VS Code grammar" },
    Guard { variable: "CLARITY_WRITE_DEFAULTS", package: "clarity-gen", test: Some("web_windows"), writes: "the Clarity.UI catalog" },
    Guard { variable: "", package: "clarity-spec", test: None, writes: "the TypeScript types" },
    Guard { variable: "", package: "clarity-preview", test: None, writes: "the IDE's TypeScript types" },
    Guard { variable: "CLARITY_WRITE_DIAGRAMS", package: "clarity-spec", test: Some("diagram_views"), writes: "the examples' diagrams" },
    Guard { variable: "CLARITY_WRITE_CONFORMANCE", package: "clarity-gen", test: Some("servers"), writes: "conformance/migrations.json" },
    Guard { variable: "CLARITY_WRITE_DEFAULTS", package: "clarity-spec", test: Some("clarity_ui_theme"), writes: "the Clarity.UI themes" },
    Guard { variable: "CLARITY_WRITE_PICTURES", package: "clarity-image", test: Some("pictures"), writes: "the picture cases" },
    Guard { variable: "CLARITY_WRITE_MANUAL", package: "clarity-spec", test: Some("manual_catalog"), writes: "the parts pages' catalog tables" },
];

/// The examples every release checks (crates/clarity-gen/tests/examples_build.rs builds each on its own stack).
pub const EXAMPLES: [&str; 21] = ["calculator", "dashboard", "dial", "invaders", "kitchen", "ledger", "maintenance", "modelling", "mp3player", "network", "noughts", "novel", "orbs", "pills", "records", "shop", "stock", "todo", "videoplayer", "walker", "waves"];

/// Where the checks run: a fresh checkout of the repository, as the checks
/// on GitHub have it, in the tree's ignored target/.
pub fn fresh_checkout_dir(repo: &Path) -> PathBuf {
    repo.join("target").join("fresh-checkout")
}

/// Where the fresh checkout builds: a folder of its own, never the tree's
/// target/. Cargo names a workspace crate's build alike in both folders, so
/// a shared one let the tree's builds take the checkout's as up to date.
pub fn fresh_target_dir(repo: &Path) -> PathBuf {
    repo.join("target").join("fresh-target")
}

/// How the tree stood when the fresh checkout was made, beside it.
fn snapshot_file(dir: &Path) -> PathBuf {
    dir.with_extension("status")
}

/// A local repository as a git URL, on every system.
fn file_url(repo: &Path) -> Result<String, String> {
    let path = repo.canonicalize().map_err(|e| format!("cannot find {}: {e}", repo.display()))?;
    let text = path.to_string_lossy().replace('\\', "/");
    let text = text.strip_prefix("//?/").unwrap_or(&text).to_string();
    Ok(if text.starts_with('/') { format!("file://{text}") } else { format!("file:///{text}") })
}

/// Makes `dir` a fresh checkout of `repo` as the checks on GitHub have it:
/// the last commit alone, without tags or history, with the tree's changes
/// not committed yet over it (changed, new and deleted files) and nothing
/// git ignores. Notes how the tree stands, for [`clean_tree`] after the checks.
/// Reads `repo` only; needs no network.
pub fn fresh_checkout(repo: &Path, dir: &Path) -> Result<(), String> {
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| format!("cannot remove the old {}: {e}", dir.display()))?;
    }
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))?;
    }
    let url = file_url(repo)?;
    let target = dir.to_string_lossy().to_string();
    git(repo, &["clone", "--quiet", "--depth", "1", "--no-tags", &url, &target])?;
    let mut files: Vec<String> = git(repo, &["diff", "--name-only", "HEAD"])?.lines().map(str::to_string).collect();
    files.extend(git(repo, &["ls-files", "--others", "--exclude-standard"])?.lines().map(str::to_string));
    for file in files.iter().filter(|f| !f.is_empty()) {
        let (from, to) = (repo.join(file), dir.join(file));
        if from.is_file() {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))?;
            }
            std::fs::copy(&from, &to).map_err(|e| format!("cannot copy {file}: {e}"))?;
        } else if to.is_file() {
            std::fs::remove_file(&to).map_err(|e| format!("cannot remove {file}: {e}"))?;
        }
    }
    std::fs::write(snapshot_file(dir), snapshot(repo)?).map_err(|e| format!("cannot write how the tree stands: {e}"))
}

/// The tree's changes as `git status` lists them, each with a hash of its
/// file, so a file changed again shows too.
fn snapshot(repo: &Path) -> Result<String, String> {
    let status = git(repo, &["status", "--porcelain", "--untracked-files=all"])?;
    Ok(status
        .lines()
        .filter(|l| l.len() > 3)
        .map(|line| {
            // A renamed file is read where it is now (`old -> new`).
            let hash = std::fs::read(repo.join(line[3..].rsplit(" -> ").next().unwrap_or(&line[3..]))).map(|bytes| bytes.iter().fold(0xcbf29ce484222325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100000001b3))).unwrap_or(0);
            format!("{line}\t{hash:x}\n")
        })
        .collect())
}

/// The files in `now` that were not in `before` as they are: new ones and
/// ones changed again (two [`snapshot`]s).
pub fn left_behind(before: &str, now: &str) -> Vec<String> {
    let was: std::collections::BTreeSet<&str> = before.lines().collect();
    now.lines().filter(|l| !was.contains(l)).filter_map(|l| l.split('\t').next()).filter(|l| l.len() > 3).map(|l| l[3..].to_string()).collect()
}

/// Whether the checks left the tree as the fresh checkout `dir` found it:
/// a test writing into the tree would put its files in the release commit.
pub fn clean_tree(repo: &Path, dir: &Path) -> Result<(), String> {
    let before = std::fs::read_to_string(snapshot_file(dir)).map_err(|_| "run the fresh checkout first: it notes how the tree stands before the checks".to_string())?;
    let left = left_behind(&before, &snapshot(repo)?);
    if left.is_empty() {
        Ok(())
    } else {
        Err(format!("the checks left {} files in the tree: {}", left.len(), left.join(", ")))
    }
}

/// The files changed since the IDE's last release (its newest `v` tag), and
/// that tag: what the checks count from.
pub fn since_ide_release(repo: &Path) -> Result<(Vec<String>, Option<String>), String> {
    let since = newest(&tags(repo)?, "v");
    let files = match &since {
        Some(tag) => changed_since(repo, tag, &["."])?,
        None => Vec::new(),
    };
    Ok((files, since))
}

/// How a check ended the last time it ran, kept for the agent that writes
/// the plan (MCP `release_state`): the IDE runs the checks, the agent reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRecord {
    pub key: String,
    pub title: String,
    pub passed: bool,
    pub why: Option<String>,
    /// When it ended, in milliseconds since 1970.
    pub ended: u64,
    /// The commit it ran on (the tree may have had changes over it).
    pub head: String,
    /// The fingerprint of what it read of the tree it ran on
    /// ([`TreeState::fingerprint`]); it counts only while that is still the same.
    #[serde(default)]
    pub tree: String,
    /// The last lines of a failed one.
    pub lines: Vec<String>,
}

/// Where the checks' results are kept, in the tree's ignored target/.
pub fn checks_file(repo: &Path) -> PathBuf {
    repo.join("target").join("release-checks.json")
}

/// The last result of every check run, by its key; none when nothing ran.
pub fn check_records(repo: &Path) -> BTreeMap<String, CheckRecord> {
    std::fs::read_to_string(checks_file(repo)).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

/// Keeps how a check ended, over its earlier run.
pub fn record_check(repo: &Path, record: &CheckRecord) -> Result<(), String> {
    let mut all = check_records(repo);
    all.insert(record.key.clone(), record.clone());
    let file = checks_file(repo);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?;
    // Written beside it and moved over it: a write cut short (a full disk) leaves the kept records as they were.
    let new = file.with_extension("json.new");
    std::fs::write(&new, text).map_err(|e| {
        let _ = std::fs::remove_file(&new);
        format!("cannot write {}: {e}", new.display())
    })?;
    std::fs::rename(&new, &file).map_err(|e| format!("cannot write {}: {e}", file.display()))
}

/// The commit the tree is on.
pub fn head(repo: &Path) -> Result<String, String> {
    Ok(git(repo, &["rev-parse", "HEAD"])?.trim().to_string())
}

/// The fingerprint of a tree: its commit and its uncommitted changes, each
/// changed file with a hash of its content (`snapshot`).
pub fn fingerprint(head: &str, changes: &str) -> String {
    let hash = changes.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
    format!("{head}:{hash:016x}")
}

/// A package of the workspace's resolve: its name, version and where it
/// comes from (none for a crate of the workspace's own folders), and the
/// packages it needs, each with whether only for its tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub source: Option<String>,
    /// Its folder in the tree, for a workspace member (`runtimes/rust-ui`).
    pub folder: Option<String>,
    /// The packages it needs (their places in [`TreeState::packages`]), true for one its tests alone need.
    pub needs: Vec<(usize, bool)>,
}

/// A tree as the checks read it: each file with its content's hash (git's
/// for a committed file, the snapshot's for a changed one) and the
/// packages Cargo resolves the workspace to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeState {
    pub files: BTreeMap<String, String>,
    pub packages: Vec<Package>,
}

/// The tree as it is now.
pub fn tree_state(repo: &Path) -> Result<TreeState, String> {
    state_of(repo, &head(repo)?, &snapshot(repo)?)
}

/// The tree the fresh checkout `dir` was made from: its commit and the changes noted beside it.
pub fn checkout_state(dir: &Path) -> Result<TreeState, String> {
    let changes = std::fs::read_to_string(snapshot_file(dir)).map_err(|e| format!("cannot read how the tree stood: {e}"))?;
    state_of(dir, &head(dir)?, &changes)
}

fn state_of(repo: &Path, head: &str, changes: &str) -> Result<TreeState, String> {
    let mut files: BTreeMap<String, String> = git(repo, &["ls-tree", "-r", head])?
        .lines()
        .filter_map(|line| {
            let (meta, path) = line.split_once('\t')?;
            Some((path.to_string(), meta.rsplit(' ').next()?.to_string()))
        })
        .collect();
    for line in changes.lines() {
        let Some((status, hash)) = line.rsplit_once('\t') else { continue };
        let Some(path) = status.get(3..) else { continue };
        let path = path.rsplit(" -> ").next().unwrap_or(path);
        if let Some((old, _)) = status[3..].split_once(" -> ") {
            files.remove(old);
        }
        if hash == "0" {
            files.remove(path);
        } else {
            files.insert(path.to_string(), hash.to_string());
        }
    }
    let packages = lock_packages(repo);
    Ok(TreeState { files, packages })
}

/// The workspace's packages as its Cargo.lock resolves them, read from
/// the lock itself (no cargo, no network: the same on every machine). A
/// member's needs its `[dev-dependencies]` alone name are its tests' only.
fn lock_packages(repo: &Path) -> Vec<Package> {
    let lock = std::fs::read_to_string(repo.join("Cargo.lock")).unwrap_or_default();
    let field = |block: &str, key: &str| block.lines().find_map(|l| l.strip_prefix(key)?.trim_start().strip_prefix('=')).map(|v| v.trim().trim_matches('"').to_string());
    let blocks: Vec<&str> = lock.split("[[package]]").skip(1).collect();
    let mut packages: Vec<Package> = blocks
        .iter()
        .map(|b| Package { name: field(b, "name").unwrap_or_default(), version: field(b, "version").unwrap_or_default(), source: field(b, "source"), folder: None, needs: Vec::new() })
        .collect();
    // The members' folders, by the names their manifests give.
    let mut tests_only: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    for member in members(repo) {
        let Ok(manifest) = std::fs::read_to_string(repo.join(&member).join("Cargo.toml")) else { continue };
        let Some((_, name)) = between(&manifest, "name = \"", "\"") else { continue };
        if let Some(p) = packages.iter_mut().find(|p| p.name == name && p.source.is_none()) {
            p.folder = Some(member.clone());
        }
        let (mut dev, mut other) = (std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
        let mut section = String::new();
        for line in manifest.lines().map(str::trim) {
            if line.starts_with('[') {
                section = line.to_string();
                continue;
            }
            if !section.ends_with("dependencies]") || line.starts_with('#') || !line.contains('=') {
                continue;
            }
            let key = line.split(['=', '.']).next().unwrap_or_default().trim();
            let name = between(line, "package = \"", "\"").map(|(_, n)| n.to_string()).unwrap_or_else(|| key.to_string());
            if section.ends_with("dev-dependencies]") { dev.insert(name) } else { other.insert(name) };
        }
        tests_only.insert(name.to_string(), dev.difference(&other).cloned().collect());
    }
    let find = |entry: &str, packages: &[Package]| {
        let mut words = entry.split_whitespace();
        let name = words.next().unwrap_or_default();
        let version = words.next();
        packages.iter().position(|p| p.name == name && version.is_none_or(|v| p.version == v))
    };
    for (at, block) in blocks.iter().enumerate() {
        let Some(list) = block.split("dependencies = [").nth(1).and_then(|rest| rest.split(']').next()) else { continue };
        let own = packages[at].folder.is_some().then(|| tests_only.get(&packages[at].name)).flatten().cloned().unwrap_or_default();
        let needs: Vec<(usize, bool)> = list
            .split(',')
            .map(|e| e.trim().trim_matches('"'))
            .filter(|e| !e.is_empty())
            .filter_map(|e| find(e, &packages).map(|to| (to, own.contains(e.split_whitespace().next().unwrap_or_default()))))
            .collect();
        packages[at].needs = needs;
    }
    packages
}

impl TreeState {
    /// The packages `package`'s build and tests use: its own needs, tests'
    /// too, and theirs without their tests'. None known: every package.
    fn closure(&self, package: &str) -> Option<std::collections::BTreeSet<usize>> {
        let start = self.packages.iter().position(|p| p.name == package && p.folder.is_some())?;
        let mut seen = std::collections::BTreeSet::from([start]);
        let mut next: Vec<usize> = self.packages[start].needs.iter().map(|(to, _)| *to).collect();
        while let Some(at) = next.pop() {
            if seen.insert(at) {
                next.extend(self.packages[at].needs.iter().filter(|(_, tests)| !tests).map(|(to, _)| *to));
            }
        }
        Some(seen)
    }

    /// The fingerprint of what `check` reads (its [`Inputs`]) and of its command.
    pub fn fingerprint(&self, check: &Check) -> String {
        let mut text = format!("{}\n", check.run.shown());
        for (name, value) in &check.run.env {
            text.push_str(&format!("{name}={value}\n"));
        }
        let rust = |path: &str| path.ends_with(".rs") || path.ends_with("Cargo.toml") || path == "Cargo.lock";
        match &check.inputs {
            Inputs::Whole => self.files.iter().for_each(|(p, h)| text.push_str(&format!("{p}\t{h}\n"))),
            Inputs::NoRust => self.files.iter().filter(|(p, _)| !rust(p)).for_each(|(p, h)| text.push_str(&format!("{p}\t{h}\n"))),
            Inputs::Crate(package) => match self.closure(package) {
                Some(closure) => {
                    let outside: Vec<String> = self.packages.iter().enumerate().filter(|(i, _)| !closure.contains(i)).filter_map(|(_, p)| p.folder.as_ref().map(|f| format!("{f}/"))).collect();
                    let read = |path: &str| path != "Cargo.lock" && !(rust(path) && outside.iter().any(|f| path.starts_with(f.as_str()) && !self.inner_member(path, f)));
                    self.files.iter().filter(|(p, _)| read(p)).for_each(|(p, h)| text.push_str(&format!("{p}\t{h}\n")));
                    let mut used: Vec<String> = closure.iter().map(|&i| &self.packages[i]).map(|p| format!("{} {} {}", p.name, p.version, p.source.as_deref().unwrap_or("-"))).collect();
                    used.sort();
                    used.iter().for_each(|u| text.push_str(&format!("{u}\n")));
                }
                None => self.files.iter().for_each(|(p, h)| text.push_str(&format!("{p}\t{h}\n"))),
            },
        }
        let hash = text.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
        format!("{hash:016x}")
    }

    /// Whether `path`, under the member folder `folder`, belongs to a member
    /// folder inside it (which counts as its own).
    fn inner_member(&self, path: &str, folder: &str) -> bool {
        self.packages.iter().filter_map(|p| p.folder.as_ref()).any(|f| f.len() + 1 > folder.len() && f.starts_with(folder) && path.starts_with(&format!("{f}/")))
    }
}

/// The fingerprint of the tree as it is now.
pub fn tree(repo: &Path) -> Result<String, String> {
    Ok(fingerprint(&head(repo)?, &snapshot(repo)?))
}

/// The checks of `checks` (key, title, the fingerprint of what it reads now)
/// that hold a plan back, each with why: failed, ran on another tree, or not run.
pub fn held_back_of(checks: impl Iterator<Item = (String, String, String)>, records: &BTreeMap<String, CheckRecord>) -> Vec<String> {
    checks
        .filter_map(|(key, title, tree)| match records.get(&key) {
            Some(r) if r.tree != tree => Some(format!("{title}: ran on another tree")),
            Some(r) if r.passed => None,
            Some(_) => Some(format!("{title}: failed")),
            None => Some(format!("{title}: not run")),
        })
        .collect()
}

/// The release checks that hold a plan back now: none when every one passed on the tree as it is.
pub fn held_back(repo: &Path) -> Result<Vec<String>, String> {
    let (files, _) = since_ide_release(repo)?;
    let state = tree_state(repo)?;
    let checks = checks(repo, &files).into_iter().map(|c| {
        let print = state.fingerprint(&c);
        (c.key, c.title, print)
    });
    Ok(held_back_of(checks, &check_records(repo)))
}

/// Whether `stamp` is a plan's name: `YYYY-MM-DD-HHMM`.
fn plan_stamp(stamp: &str) -> bool {
    let b = stamp.as_bytes();
    b.len() == 15 && b.iter().enumerate().all(|(i, c)| if [4, 7, 10].contains(&i) { *c == b'-' } else { c.is_ascii_digit() })
}

/// Writes the Release Helper's plan `plan` (its format, ~/Projects/ReleaseHelper/README.md)
/// as `releases/<stamp>.json` of `repo`: its steps begin with the checks on main
/// (a watch step of checks.yml), and a plan already there is never overwritten.
/// The caller makes sure every release check passed on the tree (`held_back`).
pub fn write_plan(repo: &Path, stamp: &str, plan: &Value) -> Result<PathBuf, String> {
    if !plan_stamp(stamp) {
        return Err(format!("'{stamp}' is no plan's name: YYYY-MM-DD-HHMM, as 2026-11-17-0930"));
    }
    let steps = plan.get("steps").and_then(Value::as_array).filter(|s| !s.is_empty()).ok_or("a plan has steps")?;
    let first = &steps[0];
    if first.get("workflow").and_then(Value::as_str) != Some("checks.yml") || first.get("tag").is_some() {
        return Err("a plan begins with the checks on main: a step with \"workflow\": \"checks.yml\" and no tag".to_string());
    }
    let file = repo.join("releases").join(format!("{stamp}.json"));
    if file.exists() {
        return Err(format!("{} is there already: a plan is never overwritten", file.display()));
    }
    std::fs::create_dir_all(repo.join("releases")).map_err(|e| format!("cannot make releases/: {e}"))?;
    let text = serde_json::to_string_pretty(plan).map_err(|e| e.to_string())? + "\n";
    std::fs::write(&file, text).map_err(|e| format!("cannot write {}: {e}", file.display()))?;
    Ok(file)
}

/// Gives the plan `releases/<stamp>.json` the commit message `message`, its
/// steps as they are: the fix after a failed release.
pub fn set_commit_message(repo: &Path, stamp: &str, message: &str) -> Result<PathBuf, String> {
    let file = repo.join("releases").join(format!("{stamp}.json"));
    let text = std::fs::read_to_string(&file).map_err(|_| format!("there is no plan {stamp} in releases/"))?;
    let mut plan: Value = serde_json::from_str(&text).map_err(|e| format!("{} does not read: {e}", file.display()))?;
    plan["commit_message"] = Value::String(message.to_string());
    std::fs::write(&file, serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())? + "\n").map_err(|e| format!("cannot write {}: {e}", file.display()))?;
    Ok(file)
}

/// A check a release needs first, run as a build.
#[derive(Debug, Clone)]
pub struct Check {
    pub key: String,
    pub title: String,
    pub run: Runnable,
    /// It runs in the fresh checkout, so it counts on the tree the checkout was made from.
    pub fresh: bool,
    /// What it reads, which its fingerprint is made of.
    pub inputs: Inputs,
}

/// What a check reads of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inputs {
    /// All of it (the fresh checkout, the clean tree).
    Whole,
    /// What a crate's build and tests read: every file but the Rust code
    /// (`.rs`, `Cargo.toml`) of the workspace crates it does not build, and
    /// for Cargo.lock, the versions its own crates resolve to.
    Crate(String),
    /// What the IDE's pages read: every file but Rust code and Cargo's files.
    NoRust,
}

fn cargo_test(repo: &Path, package: &str, test: Option<&str>, features: &[String]) -> Runnable {
    let mut args = vec!["test", "-p", package];
    match test {
        Some(t) => args.extend(["--test", t]),
        None => args.push("--lib"),
    }
    let features = features.join(",");
    if !features.is_empty() {
        args.extend(["--features", features.as_str()]);
    }
    Runnable::new("cargo", &args, repo).with_env("CARGO_BUILD_JOBS", "2")
}

/// The features the test target `test` declares in `manifest` (`required-features`
/// of its `[[test]]`): cargo leaves the target out without them.
fn required_features(manifest: &str, test: &str) -> Vec<String> {
    let value = |line: &str, key: &str| line.strip_prefix(key).map(str::trim_start).and_then(|rest| rest.strip_prefix('=')).map(str::trim).map(str::to_string);
    let mut features = Vec::new();
    for block in manifest.split("[[test]]").skip(1) {
        let lines: Vec<&str> = block.lines().map(str::trim).take_while(|l| !l.starts_with('[')).collect();
        if !lines.iter().any(|l| value(l, "name").as_deref() == Some(&format!("\"{test}\""))) {
            continue;
        }
        if let Some(list) = lines.iter().find_map(|l| value(l, "required-features")) {
            features.extend(list.trim_matches(['[', ']']).split(',').map(|f| f.trim().trim_matches('"').to_string()).filter(|f| !f.is_empty()));
        }
    }
    features
}

/// What the IDE run by dev.sh sets that the checks on GitHub do not have:
/// the repository's runtimes and Clarity.UI's library or browser files of
/// its own. Empty is unset to each of them.
const IDE_ONLY: [&str; 4] = ["CLARITY_RUNTIME", "CLARITY_UI", "CLARITY_UI_BROWSER", "CLARITY_PROJECT"];

/// Clarity.UI's library as the IDE builds apps with it now (chosen in
/// DevTools, named, carried or fetched before); none while it is still to fetch.
fn library_in_use() -> Option<PathBuf> {
    use crate::clarity_ui::Library;
    match crate::clarity_ui::library() {
        Library::Chosen(f) | Library::Named(f) | Library::Bundled(f) | Library::Fetched(f) => Some(f),
        Library::ToFetch { .. } | Library::Unavailable { .. } => None,
    }
}

fn node_tool(name: &str) -> String {
    if cfg!(windows) { format!("{name}.cmd") } else { name.to_string() }
}

/// A command of Clarity's own CLI, run in the tree.
fn clarity_cli(repo: &Path, args: &[&str]) -> Runnable {
    let mut all = vec!["run", "-q", "-p", "clarity-cli", "--"];
    all.extend(args);
    Runnable::new("cargo", &all, repo).with_env("CARGO_BUILD_JOBS", "2")
}

/// The checks before a plan, with `changed` the files changed since the
/// last IDE release: every guard written and tested in the tree; then a
/// fresh checkout as the checks on GitHub have it, where the test targets
/// of every crate changed, every example's spec check and each changed
/// example's build run (building in a folder of its own); the IDE page's checks
/// when app/ or a model type changed (in the tree, which has its
/// node_modules); and last, that the checks left the tree clean.
pub fn checks(repo: &Path, changed: &[String]) -> Vec<Check> {
    let canonical = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    let repo = canonical.as_path();
    let fresh = fresh_checkout_dir(repo);
    let own = fresh_target_dir(repo).to_string_lossy().to_string();
    // Without debug info: every test target is a program of its own, and with it they took 35 GB; a failure still names its file and line.
    let in_fresh = |run: Runnable| {
        let run = Runnable { dir: fresh.clone(), ..run }.with_env("CARGO_TARGET_DIR", &own).with_env("CARGO_PROFILE_DEV_DEBUG", "0").with_env("CARGO_PROFILE_TEST_DEBUG", "0");
        IDE_ONLY.iter().fold(run, |run, name| run.with_env(name, ""))
    };
    let mut checks = Vec::new();
    for g in &GUARDS {
        let target = g.test.map(|t| format!("--test {t}")).unwrap_or_else(|| "--lib".into());
        if !g.variable.is_empty() {
            checks.push(Check { key: format!("guard:write:{}:{target}", g.package), title: format!("Write {}", g.writes), run: cargo_test(repo, g.package, g.test, &[]).with_env(g.variable, "1"), fresh: false, inputs: Inputs::Crate(g.package.to_string()) });
        }
        checks.push(Check { key: format!("guard:test:{}:{target}", g.package), title: format!("Check {}", g.writes), run: cargo_test(repo, g.package, g.test, &[]), fresh: false, inputs: Inputs::Crate(g.package.to_string()) });
    }
    checks.push(Check { key: "freshCheckout".into(), title: "A fresh checkout, as on GitHub".into(), run: clarity_cli(repo, &["release", "fresh-checkout"]), fresh: false, inputs: Inputs::Whole });
    let under = |folder: &str| changed.iter().any(|f| f == folder || f.starts_with(&format!("{folder}/")));
    for member in members(repo) {
        if !under(&member) {
            continue;
        }
        let Ok(manifest) = std::fs::read_to_string(repo.join(&member).join("Cargo.toml")) else { continue };
        let Some((_, package)) = between(&manifest, "name = \"", "\"") else { continue };
        if package == "clarity-ui-ffi" {
            // The C header's test links the library, which cargo test does not build (as checks.yml does first).
            let run = in_fresh(Runnable::new("cargo", &["build", "-p", package], repo).with_env("CARGO_BUILD_JOBS", "2"));
            checks.push(Check { key: format!("library:{package}"), title: format!("{package}: the library its tests link"), run, fresh: true, inputs: Inputs::Crate(package.to_string()) });
        }
        if repo.join(&member).join("src/lib.rs").is_file() {
            checks.push(Check { key: format!("test:{package}:--lib"), title: format!("{package}: lib tests"), run: in_fresh(cargo_test(repo, package, None, &[])), fresh: true, inputs: Inputs::Crate(package.to_string()) });
        }
        let mut tests: Vec<String> = std::fs::read_dir(repo.join(&member).join("tests"))
            .map(|d| d.flatten().filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".rs").map(str::to_string)).collect())
            .unwrap_or_default();
        tests.sort();
        for test in tests {
            checks.push(Check { key: format!("test:{package}:{test}"), title: format!("{package}: {test}"), run: in_fresh(cargo_test(repo, package, Some(&test), &required_features(&manifest, &test))), fresh: true, inputs: Inputs::Crate(package.to_string()) });
        }
    }
    if under("app") || under("runtimes/web/src/generated") {
        let app = repo.join("app");
        checks.push(Check { key: "app:svelte-check".into(), title: "The IDE's pages: svelte-check".into(), run: Runnable::new(&node_tool("npx"), &["svelte-check", "--threshold", "warning"], &app), fresh: false, inputs: Inputs::NoRust });
        checks.push(Check { key: "app:test".into(), title: "The IDE's pages: tests".into(), run: Runnable::new(&node_tool("npm"), &["test"], &app), fresh: false, inputs: Inputs::NoRust });
    }
    for example in EXAMPLES {
        let folder = format!("examples/{example}");
        checks.push(Check {
            key: format!("spec:{example}"),
            title: format!("{example}: spec check"),
            run: in_fresh(clarity_cli(repo, &["spec", "check", &folder])),
            fresh: true,
            inputs: Inputs::Crate("clarity-cli".into()),
        });
    }
    // A Go, C or C++ desktop example builds with Clarity.UI's library as the IDE gives it to apps.
    let library = library_in_use().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    for example in EXAMPLES.iter().filter(|e| under(&format!("examples/{e}"))) {
        let exact = format!("the_{example}_example_builds_without_warnings");
        checks.push(Check {
            key: format!("build:{example}"),
            title: format!("{example}: builds on its stack"),
            run: in_fresh(Runnable::new("cargo", &["test", "-p", "clarity-gen", "--test", "examples_build", "--", "--ignored", "--exact", &exact], repo).with_env("CARGO_BUILD_JOBS", "2")).with_env("CLARITY_UI", &library),
            fresh: true,
            inputs: Inputs::Crate("clarity-gen".into()),
        });
    }
    checks.push(Check { key: "cleanTree".into(), title: "The checks left the tree clean".into(), run: clarity_cli(repo, &["release", "clean-tree"]), fresh: false, inputs: Inputs::Whole });
    checks
}
