//! DevTools' Disk tab: the folders Clarity's builds and checks fill in its
//! repository (every one a folder git ignores, made again by a build), how
//! big each is, what removing it costs, and removing it.

use std::path::{Path, PathBuf};

use sysinfo::{DiskRefreshKind, Disks};

/// Below this the Disk tab warns: builds and checks need room.
pub const ROOM: u64 = 40 * 1024 * 1024 * 1024;

/// A folder, or folders of one kind, that builds fill.
#[derive(Debug, Clone)]
pub struct Place {
    pub key: &'static str,
    pub name: &'static str,
    /// What removing it costs, as people read it.
    pub cost: &'static str,
    pub paths: Vec<PathBuf>,
}

/// `<folder>/*/<name>`: that folder in every folder of `folder`.
fn each(repo: &Path, folder: &str, name: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(repo.join(folder))
        .map(|d| d.flatten().map(|e| e.path().join(name)).filter(|p| p.is_dir()).collect())
        .unwrap_or_default();
    found.sort();
    found
}

/// The folders of `examples/` with no project file beside them: an
/// example's folder sits beside `<name>.clarity`, so one without it is what
/// an example removed from Clarity left behind (its ignored out/ and data/).
fn leftover_examples(repo: &Path) -> Vec<PathBuf> {
    let examples = repo.join("examples");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&examples)
        .map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_dir() && p.file_name().is_some_and(|n| !examples.join(format!("{}.clarity", n.to_string_lossy())).exists())).collect())
        .unwrap_or_default();
    found.sort();
    found
}

/// Every place builds fill in the repository `repo`, the biggest kinds first.
pub fn places(repo: &Path) -> Vec<Place> {
    let target = |rel: &str| vec![repo.join("target").join(rel)];
    let leftovers = leftover_examples(repo);
    // A leftover's out/ is counted once, with the leftover it belongs to.
    // The examples' and the test projects' generated apps alike.
    let mut outs: Vec<PathBuf> = each(repo, "examples", "out").into_iter().filter(|out| !leftovers.iter().any(|l| out.starts_with(l))).collect();
    outs.extend(each(repo, "test-projects", "out"));
    let mut csharp = each(repo, "runtimes/csharp", "bin");
    csharp.extend(each(repo, "runtimes/csharp", "obj"));
    vec![
        Place { key: "debug", name: "Debug builds: the IDE, the CLI, tests and checks (target/debug)", cost: "The next start of the IDE, the next check and the MCP server build everything again, which takes long.", paths: target("debug") },
        Place { key: "generated", name: "Tests' builds of generated apps (target/generated)", cost: "The build tests compile their apps from scratch the next time, which takes long.", paths: target("generated") },
        Place { key: "freshTarget", name: "The fresh checkout's builds (target/fresh-target)", cost: "The next release checks build from scratch.", paths: target("fresh-target") },
        Place { key: "freshCheckout", name: "The fresh checkout (target/fresh-checkout)", cost: "None: the next release checks make it again.", paths: target("fresh-checkout") },
        Place { key: "release", name: "Release builds of your own libraries (target/release)", cost: "Your own builds of Clarity's libraries go: press their Build again before you use them.", paths: target("release") },
        Place { key: "examples", name: "The examples' builds (target/g8c-shared)", cost: "Examples build again from scratch.", paths: target("g8c-shared") },
        Place { key: "browser", name: "Builds of Clarity.UI's browser files (target/wasm32-unknown-emscripten, target/browser)", cost: "Your own browser files build again from scratch when you press their Build.", paths: [target("wasm32-unknown-emscripten"), target("browser")].concat() },
        Place { key: "testScratch", name: "Tests' scratch folders (target/tmp)", cost: "None: tests make them again.", paths: target("tmp") },
        Place { key: "exampleOuts", name: "The examples' and test projects' generated apps (examples/*/out, test-projects/*/out)", cost: "Generate an example again before you run it.", paths: outs },
        Place { key: "csharp", name: "The C# runtime's builds (runtimes/csharp/*/bin and obj)", cost: "The C# runtime builds again.", paths: csharp },
        Place {
            key: "leftoverExamples",
            name: "Left by examples no longer in Clarity (examples/* without a project file)",
            cost: "Everything in them goes for good, a database in data/ too; no project of Clarity's uses them.",
            paths: leftovers,
        },
    ]
}

fn size_of(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return 0 };
    if !meta.is_dir() {
        return meta.len();
    }
    std::fs::read_dir(path).map(|d| d.flatten().map(|e| size_of(&e.path())).sum()).unwrap_or(0)
}

/// How many bytes `place`'s files take.
pub fn size(place: &Place) -> u64 {
    place.paths.iter().map(|p| size_of(p)).sum()
}

/// `path` inside `repo` as the tab and the marks name it: `target/debug`.
fn relative(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo).unwrap_or(path).components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

fn place(repo: &Path, key: &str) -> Result<Place, String> {
    places(repo).into_iter().find(|p| p.key == key).ok_or_else(|| format!("there is no place {key}"))
}

/// Removes the place `key` of `repo`; how many bytes that freed.
pub fn remove(repo: &Path, key: &str) -> Result<u64, String> {
    let place = place(repo, key)?;
    let freed = size(&place);
    for path in place.paths.iter().filter(|p| p.exists()) {
        std::fs::remove_dir_all(path).map_err(|e| format!("cannot remove {}: {e}", relative(repo, path)))?;
    }
    Ok(freed)
}

/// The folders marked to go at the next start, one per line, relative to
/// the repository with `/`. dev.sh and dev.bat remove them before anything
/// builds or runs (app/scripts/clean-at-start.mjs), when no running IDE
/// holds their files, as Windows does with its own.
pub fn at_start_file(repo: &Path) -> PathBuf {
    repo.join("target").join("remove-at-start")
}

fn marked_folders(repo: &Path) -> Vec<String> {
    std::fs::read_to_string(at_start_file(repo)).map(|t| t.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect()).unwrap_or_default()
}

/// The places marked for the next start: those whose every folder is marked.
pub fn marked(repo: &Path) -> Vec<&'static str> {
    let folders = marked_folders(repo);
    places(repo).into_iter().filter(|p| !p.paths.is_empty() && p.paths.iter().all(|f| folders.contains(&relative(repo, f)))).map(|p| p.key).collect()
}

/// Marks the place `key` to go at the next start, or takes the mark away.
pub fn mark(repo: &Path, key: &str, marked: bool) -> Result<(), String> {
    let place = place(repo, key)?;
    let mut folders = marked_folders(repo);
    for folder in place.paths.iter().map(|p| relative(repo, p)) {
        folders.retain(|f| *f != folder);
        if marked {
            folders.push(folder);
        }
    }
    folders.sort();
    let file = at_start_file(repo);
    if folders.is_empty() {
        let _ = std::fs::remove_file(&file);
        return Ok(());
    }
    std::fs::create_dir_all(file.parent().expect("target/")).map_err(|e| format!("cannot write {}: {e}", file.display()))?;
    std::fs::write(&file, folders.iter().map(|f| format!("{f}\n")).collect::<String>()).map_err(|e| format!("cannot write {}: {e}", file.display()))
}

/// A place as the Disk tab shows it, to the person and to the agent alike.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceView {
    pub key: &'static str,
    pub name: &'static str,
    pub cost: &'static str,
    pub bytes: u64,
    /// Marked to go at the next start of the IDE.
    pub marked: bool,
}

/// The Disk tab: every place with its size and mark, and the free space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskView {
    pub places: Vec<PlaceView>,
    /// The free space of the disk the repository is on; none when unknown.
    pub free: Option<u64>,
    /// Free space is below what builds and checks need ([`ROOM`]).
    pub low: bool,
}

/// The Disk tab of `repo`, as the IDE shows it and the agent reads it.
pub fn view(repo: &Path) -> DiskView {
    let marked = marked(repo);
    let places = places(repo)
        .into_iter()
        .map(|p| PlaceView { key: p.key, name: p.name, cost: p.cost, bytes: size(&p), marked: marked.contains(&p.key) })
        .collect();
    let free = free_space(repo);
    DiskView { places, free, low: free.is_some_and(|f| f < ROOM) }
}

/// The free space of the disk holding `path`: the one whose mount point is the longest start of it.
pub fn free_space(path: &Path) -> Option<u64> {
    // Plain: on Windows canonicalize puts `\\?\` before the path, which no mount point (`E:\`) starts.
    let path = clarity_project::plain(path.canonicalize().ok()?);
    let disks = Disks::new_with_refreshed_list_specifics(DiskRefreshKind::nothing().with_storage());
    disks.list().iter().filter(|d| path.starts_with(d.mount_point())).max_by_key(|d| d.mount_point().as_os_str().len()).map(|d| d.available_space())
}

/// A size as people read it: `39 GB`, `9.4 GB`, `448 MB`.
pub fn bytes_text(bytes: u64) -> String {
    let units = ["bytes", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    match unit {
        0 => format!("{bytes} bytes"),
        _ if value >= 10.0 => format!("{} {}", value.round(), units[unit]),
        _ => format!("{value:.1} {}", units[unit]),
    }
}
