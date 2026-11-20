//! The runtimes the installed Clarity carries, so a project builds with no
//! internet (docs/ledger/offline.md, "The installer"). The release fetches
//! the published runtimes of this Clarity's versions with the projects
//! `manifests` writes (runtimes/carried/, .github/scripts/stage-runtimes.sh)
//! and packs each stack's into an archive in the installer's `runtimes/`
//! folder, beside `clarity-ui/`:
//!   rust.tar.zst   the crates, as `cargo vendor --versioned-dirs` writes them
//!   go.tar.zst     the modules, as a Go module proxy in files
//! C# carries none: its packages (with every system's native libraries)
//! would make the installer hundreds of MB bigger, so its first build needs
//! the internet, and NuGet's own cache serves the builds after that.
//! A build takes the internet first: only when the stack's registry
//! (`registry`) does not answer is its archive unpacked into the person's
//! cache folder (`unpacked`, once per version) and the build pointed at it
//! (`offline`).

use std::path::{Path, PathBuf};

use clarity_project::Backend;

use crate::Toolchain;

/// The folder the installed Clarity carries the runtimes in, below its
/// resource folder (clarity_ui::installed_resources).
pub const FOLDER: &str = "runtimes";

static CARRIED: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Names the folder of the runtimes the installed Clarity carries, which
/// the IDE does when it starts.
pub fn set_carried(folder: PathBuf) {
    let _ = CARRIED.set(folder);
}

/// The folder of the carried runtimes, when the installed Clarity has it.
pub fn carried() -> Option<&'static Path> {
    CARRIED.get().map(PathBuf::as_path).filter(|f| f.is_dir())
}

/// A stack's carried runtimes: the archive's name in the carried folder
/// (without `.tar.zst`) and the version of the runtime it holds. None for C
/// and C++, which copy their runtime into the project, and for C#.
fn stack(backend: Backend) -> Option<(&'static str, &'static str)> {
    match backend {
        Backend::Rust => Some(("rust", crate::rust::runtime_version())),
        Backend::Go => Some(("go", crate::go::runtime_version())),
        _ => None,
    }
}

/// The archive of `backend`'s runtimes in the carried folder.
pub fn archive_name(backend: Backend) -> Option<String> {
    stack(backend).map(|(name, _)| format!("{name}.tar.zst"))
}

/// The address asked whether `backend`'s registry answers before a build;
/// none for the stacks that carry no runtimes.
pub fn registry(backend: Backend) -> Option<&'static str> {
    match backend {
        Backend::Rust => Some("https://index.crates.io/config.json"),
        Backend::Go => Some("https://proxy.golang.org/"),
        _ => None,
    }
}

/// The folder `backend`'s carried runtimes are unpacked into:
/// `<cache>/clarity/runtimes/<stack>/<version>`.
pub fn unpacked_dir(cache: &Path, backend: Backend) -> Option<PathBuf> {
    stack(backend).map(|(name, version)| cache.join("clarity").join(FOLDER).join(name).join(version))
}

/// `backend`'s carried runtimes, unpacked from the archive in `carried`
/// into the cache folder the first time; the folder after that.
pub fn unpacked(carried: &Path, backend: Backend, cache: &Path) -> Result<PathBuf, String> {
    let (Some(name), Some(into)) = (archive_name(backend), unpacked_dir(cache, backend)) else { return Err(format!("{backend:?} carries no runtimes")) };
    if into.is_dir() {
        return Ok(into);
    }
    let archive = carried.join(&name);
    let file = std::fs::File::open(&archive).map_err(|err| format!("cannot open {}: {err}", archive.display()))?;
    let parent = into.parent().ok_or_else(|| format!("{} has no parent folder", into.display()))?;
    std::fs::create_dir_all(parent).map_err(|err| format!("cannot make {}: {err}", parent.display()))?;
    let partial = parent.join(format!(".partial-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&partial);
    let done = (|| {
        let decoder = ruzstd::decoding::StreamingDecoder::new(std::io::BufReader::new(file)).map_err(|err| format!("cannot read {}: {err}", archive.display()))?;
        // tar leaves out entries that would land outside the folder (`..`, absolute paths).
        tar::Archive::new(decoder).unpack(&partial).map_err(|err| format!("cannot unpack {}: {err}", archive.display()))?;
        std::fs::rename(&partial, &into).map_err(|err| format!("cannot move the runtimes to {}: {err}", into.display()))
    })();
    let _ = std::fs::remove_dir_all(&partial);
    done.map(|()| into)
}

/// `toolchain` building a project in `out` with no internet, on `backend`'s
/// carried runtimes unpacked in `folder`: Go's proxy is the folder, cargo
/// takes the crates in place of crates.io.
pub fn offline(mut toolchain: Toolchain, backend: Backend, folder: &Path, out: &Path) -> Toolchain {
    match backend {
        Backend::Go => {
            toolchain.env.retain(|(name, _)| *name != "GOPROXY");
            toolchain.env.push(("GOPROXY", file_url(folder)));
        }
        Backend::Rust => toolchain.args.extend(cargo_offline(folder, out)),
        _ => {}
    }
    toolchain
}

/// A folder as a file URL, with `/` between folders and spaces escaped.
pub fn file_url(folder: &Path) -> String {
    let path = folder.display().to_string().replace('\\', "/").replace(' ', "%20");
    if path.starts_with('/') { format!("file://{path}") } else { format!("file:///{path}") }
}

/// What cargo takes with no internet: the carried crates (`crates`) in place of
/// crates.io when they hold every crate of the project's Cargo.lock (or
/// there is no lock yet), else its own cache (`--offline`).
fn cargo_offline(crates: &Path, out: &Path) -> Vec<String> {
    let held = match std::fs::read_to_string(out.join("Cargo.lock")) {
        Ok(lock) => locked_crates(&lock).iter().all(|(name, version)| crates.join(format!("{name}-{version}")).is_dir()),
        Err(_) => true,
    };
    if !held {
        return vec!["--offline".to_string()];
    }
    // A literal TOML string: the path as it is, with `/` between folders.
    let directory = crates.display().to_string().replace('\\', "/");
    vec!["--config".to_string(), "source.crates-io.replace-with='clarity-carried'".to_string(), "--config".to_string(), format!("source.clarity-carried.directory='{directory}'"), "--offline".to_string()]
}

/// The crates.io crates a Cargo.lock names, as (name, version).
fn locked_crates(lock: &str) -> Vec<(String, String)> {
    let field = |block: &str, key: &str| block.lines().find_map(|l| l.strip_prefix(key)?.trim_start().strip_prefix('=')).map(|v| v.trim().trim_matches('"').to_string());
    lock.split("[[package]]")
        .skip(1)
        .filter(|b| field(b, "source").is_some_and(|s| s.starts_with("registry+") || s.starts_with("sparse+")))
        .filter_map(|b| Some((field(b, "name")?, field(b, "version")?)))
        .collect()
}

/// The projects the release fetches the carried runtimes with, by file name
/// (runtimes/carried/): every runtime a generated project can ask for, at
/// the version this Clarity generates.
pub fn manifests() -> Vec<(&'static str, String)> {
    let header = "Generated by Clarity (crates/clarity-gen/src/carried.rs); written again by the carried test of clarity-gen with CLARITY_WRITE_CARRIED=1";
    let features = crate::rust::runtime_features().iter().map(|f| format!("\"{f}\"")).collect::<Vec<_>>().join(", ");
    let cargo = format!(
        "# {header}\n# The crates the installed Clarity carries: `cargo vendor --versioned-dirs`.\n[package]\nname = \"carried\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n[dependencies]\nclarity-runtime = {{ version = \"={}\", default-features = false, features = [{features}] }}\nclarity-pdf = \"={}\"\n{}\n[workspace]\n",
        crate::rust::runtime_version(),
        crate::rust::pdf_version(),
        crate::rust::COMPANIONS
    );
    let go_line = crate::go::runtime_go_line();
    let go = format!("// {header}\n// The modules the installed Clarity carries: `go mod download all`.\nmodule carried\n\n{go_line}\n\nrequire {} {}\n", crate::go::RUNTIME_MODULE, crate::go::runtime_version());
    vec![("Cargo.toml", cargo), ("go.mod", go)]
}
