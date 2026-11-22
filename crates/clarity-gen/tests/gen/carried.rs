//! The runtimes the installed Clarity carries for a build with no internet
//! (docs/ledger/offline.md, "The installer"): what the release fills them
//! from, and how a build takes them, the internet first.

use std::path::{Path, PathBuf};

use clarity_gen::carried;
use clarity_project::Backend;

fn toolchain(tool: &'static str) -> clarity_gen::Toolchain {
    clarity_gen::Toolchain { tool, args: vec!["build".to_string()], env: Vec::new(), program: PathBuf::from("program"), missing: "" }
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("carried-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// runtimes/carried/ holds the projects the release fetches the runtimes
/// with; written by CLARITY_WRITE_CARRIED=1, failing when behind.
#[test]
fn the_release_s_manifests_are_current() {
    let folder = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../runtimes/carried"));
    for (name, text) in carried::manifests() {
        let path = folder.join(name);
        if std::env::var_os("CLARITY_WRITE_CARRIED").is_some() {
            std::fs::create_dir_all(folder).unwrap();
            crate::write_changed(&path, &text);
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap_or_default(), text, "{} is behind: CLARITY_WRITE_CARRIED=1 cargo test -p clarity-gen --test gen carried::", path.display());
    }
}

/// Every runtime a generated project can ask for, at the version this
/// Clarity generates.
#[test]
fn the_manifests_name_every_runtime_a_project_asks_for() {
    let manifests = carried::manifests();
    let text = |name: &str| manifests.iter().find(|(n, _)| *n == name).map(|(_, t)| t.clone()).unwrap();
    let cargo = text("Cargo.toml");
    assert!(cargo.contains(&format!("clarity-runtime = {{ version = \"={}\"", clarity_gen::rust_runtime_version())), "{cargo}");
    for feature in ["sqlite", "postgres", "mysql", "mssql", "engines", "native-ui", "secrets"] {
        assert!(cargo.contains(&format!("\"{feature}\"")), "{feature}: {cargo}");
    }
    assert!(cargo.contains("serde_json"));
    let go = text("go.mod");
    assert!(go.contains(&format!("require github.com/petervdpas/Clarity/runtimes/go {}", clarity_gen::go_runtime_version())), "{go}");
}

/// Each stack's registry, asked whether the internet answers before a
/// build; C and C++ fetch no runtime.
#[test]
fn each_registry_is_named() {
    assert_eq!(carried::registry(Backend::Rust), Some("https://index.crates.io/config.json"));
    // C# carries none: its first build needs the internet.
    assert_eq!(carried::registry(Backend::Csharp), None);
    assert_eq!(carried::archive_name(Backend::Csharp), None);
    assert_eq!(carried::registry(Backend::Go), Some("https://proxy.golang.org/"));
    assert_eq!(carried::registry(Backend::C), None);
    assert_eq!(carried::archive_name(Backend::Go).as_deref(), Some("go.tar.zst"));
    assert_eq!(carried::archive_name(Backend::Cpp), None);
}

/// With no internet Go's proxy is the unpacked modules.
#[test]
fn offline_go_takes_the_carried_modules() {
    assert_eq!(carried::file_url(Path::new("/home/p/.cache/clarity/runtimes/go/v0.64.0")), "file:///home/p/.cache/clarity/runtimes/go/v0.64.0");
    assert_eq!(carried::file_url(Path::new(r"C:\Users\P Q\AppData\Local\clarity\runtimes\go")), "file:///C:/Users/P%20Q/AppData/Local/clarity/runtimes/go");
    let go = carried::offline(toolchain("go"), Backend::Go, Path::new("/cache/go"), Path::new("."));
    assert_eq!(go.env, [("GOPROXY", "file:///cache/go".to_string())]);
}

#[test]
fn offline_c_and_csharp_build_as_always() {
    for (backend, tool) in [(Backend::Csharp, "dotnet"), (Backend::C, "cmake")] {
        let built = carried::offline(toolchain(tool), backend, Path::new("/cache"), Path::new("."));
        assert_eq!(built.args, ["build"], "{backend:?}");
        assert!(built.env.is_empty(), "{backend:?}");
    }
}

/// Offline, cargo takes the carried crates in place of crates.io when the
/// project's Cargo.lock names only crates they hold (or there is no lock
/// yet); else it builds offline from its own cache.
#[test]
fn offline_rust_takes_the_carried_crates_when_they_hold_the_lock() {
    let crates = scratch("crates");
    for dir in ["serde-1.0.228", "clarity-runtime-0.50.0"] {
        std::fs::create_dir_all(crates.join(dir)).unwrap();
    }
    let replace = |args: &[String]| args.iter().any(|a| a.contains("replace-with"));
    let project = scratch("project");
    let built = carried::offline(toolchain("cargo"), Backend::Rust, &crates, &project);
    assert!(replace(&built.args), "no lock yet: {:?}", built.args);
    assert!(built.args.iter().any(|a| a.contains("directory") && a.contains(&crates.display().to_string().replace('\\', "/"))), "{:?}", built.args);
    let lock = "version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.228\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n[[package]]\nname = \"shop\"\nversion = \"0.1.0\"\n";
    std::fs::write(project.join("Cargo.lock"), lock).unwrap();
    assert!(replace(&carried::offline(toolchain("cargo"), Backend::Rust, &crates, &project).args));
    std::fs::write(project.join("Cargo.lock"), format!("{lock}\n[[package]]\nname = \"rand\"\nversion = \"0.9.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n")).unwrap();
    let own = carried::offline(toolchain("cargo"), Backend::Rust, &crates, &project);
    assert_eq!(own.args, ["build", "--offline"]);
}

/// A stack's archive is unpacked into the cache folder once, and only
/// when asked for.
#[test]
fn the_archive_is_unpacked_into_the_cache_once() {
    let carried_folder = scratch("installed");
    let cache = scratch("cache");
    // A zstd archive of one file, as the release packs it (tar -C go -c . | zstd).
    let stage = scratch("stage");
    std::fs::create_dir_all(stage.join("github.com/x/@v")).unwrap();
    std::fs::write(stage.join("github.com/x/@v/list"), "v1.0.0\n").unwrap();
    let tar = carried_folder.join("go.tar");
    let packed = std::process::Command::new("tar").arg("-cf").arg(&tar).arg("-C").arg(&stage).arg(".").status().unwrap().success()
        && std::process::Command::new("zstd").arg("-q").arg("--rm").arg(&tar).arg("-o").arg(carried_folder.join("go.tar.zst")).status().is_ok_and(|s| s.success());
    if !packed {
        eprintln!("no tar or zstd here: not tried");
        return;
    }
    let into = carried::unpacked(&carried_folder, Backend::Go, &cache).unwrap();
    assert_eq!(into, carried::unpacked_dir(&cache, Backend::Go).unwrap());
    assert_eq!(std::fs::read_to_string(into.join("github.com/x/@v/list")).unwrap(), "v1.0.0\n");
    std::fs::remove_file(carried_folder.join("go.tar.zst")).unwrap();
    assert_eq!(carried::unpacked(&carried_folder, Backend::Go, &cache).unwrap(), into, "kept: the archive is not read again");
    assert!(carried::unpacked(&carried_folder, Backend::Rust, &cache).is_err());
}
