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

/// Every place builds fill in the repository `repo`, the biggest kinds first.
pub fn places(repo: &Path) -> Vec<Place> {
    let target = |rel: &str| vec![repo.join("target").join(rel)];
    let mut csharp = each(repo, "runtimes/csharp", "bin");
    csharp.extend(each(repo, "runtimes/csharp", "obj"));
    vec![
        Place { key: "debug", name: "Debug builds: the IDE, the CLI, tests and checks (target/debug)", cost: "The next start of the IDE, the next check and the MCP server build everything again, which takes long.", paths: target("debug") },
        Place { key: "freshTarget", name: "The fresh checkout's builds (target/fresh-target)", cost: "The next release checks build from scratch.", paths: target("fresh-target") },
        Place { key: "freshCheckout", name: "The fresh checkout (target/fresh-checkout)", cost: "None: the next release checks make it again.", paths: target("fresh-checkout") },
        Place { key: "release", name: "Release builds of your own libraries (target/release)", cost: "Your own builds of Clarity's libraries go: press their Build again before you use them.", paths: target("release") },
        Place { key: "examples", name: "The examples' builds (target/g8c-shared)", cost: "Examples build again from scratch.", paths: target("g8c-shared") },
        Place { key: "browser", name: "Builds of Clarity.UI's browser files (target/wasm32-unknown-emscripten, target/browser)", cost: "Your own browser files build again from scratch when you press their Build.", paths: [target("wasm32-unknown-emscripten"), target("browser")].concat() },
        Place { key: "testScratch", name: "Tests' scratch folders (target/tmp)", cost: "None: tests make them again.", paths: target("tmp") },
        Place { key: "exampleOuts", name: "The examples' generated apps (examples/*/out)", cost: "Generate an example again before you run it.", paths: each(repo, "examples", "out") },
        Place { key: "csharp", name: "The C# runtime's builds (runtimes/csharp/*/bin and obj)", cost: "The C# runtime builds again.", paths: csharp },
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

/// Removes the place `key` of `repo`; how many bytes that freed.
pub fn remove(repo: &Path, key: &str) -> Result<u64, String> {
    let place = places(repo).into_iter().find(|p| p.key == key).ok_or_else(|| format!("there is no place {key}"))?;
    let freed = size(&place);
    for path in place.paths.iter().filter(|p| p.exists()) {
        std::fs::remove_dir_all(path).map_err(|e| format!("cannot remove {}: {e}", path.display()))?;
    }
    Ok(freed)
}

/// The free space of the disk holding `path`: the one whose mount point is the longest start of it.
pub fn free_space(path: &Path) -> Option<u64> {
    let path = path.canonicalize().ok()?;
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
