//! Each stack builds its generated project with its own tools: Run and Build
//! in Clarity run the command `toolchain` gives and start the program it
//! names.

use std::path::PathBuf;

use clarity_project::{Backend, Ui};

/// A project named shop on `backend`: a new project's settings, not the
/// shop example, which is generated for its own stack only.
fn shop(backend: Backend, ui: Ui) -> clarity_project::Settings {
    let mut settings = clarity_project::starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    settings
}

fn exe(path: &str) -> PathBuf {
    if cfg!(windows) { PathBuf::from(format!("{path}.exe")) } else { PathBuf::from(path) }
}

#[test]
fn rust_builds_with_cargo() {
    let settings = shop(Backend::Rust, Ui::Clarity);
    let debug = clarity_gen::toolchain(&settings, false).unwrap();
    assert_eq!(debug.tool, "cargo");
    assert_eq!(debug.args, ["build", "--color", "never"]);
    assert_eq!(debug.program, exe("target/debug/shop"));
    let release = clarity_gen::toolchain(&settings, true).unwrap();
    assert_eq!(release.args, ["build", "--color", "never", "--release"]);
    assert_eq!(release.program, exe("target/release/shop"));
}

#[test]
fn csharp_builds_with_dotnet() {
    let settings = shop(Backend::Csharp, Ui::Clarity);
    let debug = clarity_gen::toolchain(&settings, false).unwrap();
    assert_eq!(debug.tool, "dotnet");
    assert_eq!(debug.args, ["build", "--nologo", "-tl:off", "-c", "Debug"]);
    // The program dotnet writes next to Shop.dll.
    assert_eq!(debug.program, exe("bin/Debug/net10.0/Shop"));
    let release = clarity_gen::toolchain(&settings, true).unwrap();
    assert_eq!(release.args, ["build", "--nologo", "-tl:off", "-c", "Release"]);
    assert_eq!(release.program, exe("bin/Release/net10.0/Shop"));
    assert!(debug.missing.contains(".NET SDK"), "{}", debug.missing);
}

#[test]
fn go_builds_with_go_build() {
    let settings = shop(Backend::Go, Ui::Clarity);
    let debug = clarity_gen::toolchain(&settings, false).unwrap();
    assert_eq!(debug.tool, "go");
    // `go build -o` writes the name it is given, so the program is built
    // under the name Run starts: `.exe` on Windows.
    let built = exe("bin/shop").to_string_lossy().into_owned();
    // -v names each package as it is built, so a long first build shows progress.
    assert_eq!(debug.args, ["build", "-v", "-o", built.as_str(), "."]);
    assert_eq!(debug.program, exe("bin/shop"));
    let release = clarity_gen::toolchain(&settings, true).unwrap();
    // Without debug information.
    assert_eq!(release.args, ["build", "-v", "-trimpath", "-ldflags=-s -w", "-o", built.as_str(), "."]);
    assert!(debug.missing.contains("Go"), "{}", debug.missing);
}

/// A Go desktop program links Clarity.UI's library through cgo, from the
/// folder CLARITY_UI names; it finds the shared library beside it, else in
/// that folder (DESIGN 7.9).
#[test]
fn go_takes_clarity_ui_from_the_folder_clarity_ui_names() {
    use clarity_gen::go_cgo_env;
    assert!(go_cgo_env(None, "linux").is_empty());
    let folder = std::path::Path::new("/opt/clarity-ui");
    let linux = go_cgo_env(Some(folder), "linux");
    let value = |env: &[(&str, String)], name: &str| env.iter().find(|(n, _)| *n == name).map(|(_, v)| v.clone()).unwrap_or_default();
    assert_eq!(value(&linux, "CGO_ENABLED"), "1");
    assert_eq!(value(&linux, "CGO_CFLAGS"), format!("-I{}", folder.join("include").display()));
    let lib = folder.join("lib").display().to_string();
    assert_eq!(value(&linux, "CGO_LDFLAGS"), format!("-L{lib} -Wl,-rpath,$ORIGIN -Wl,-rpath,{lib}"));
    assert_eq!(value(&go_cgo_env(Some(folder), "macos"), "CGO_LDFLAGS"), format!("-L{lib} -Wl,-rpath,@executable_path -Wl,-rpath,{lib}"));
    assert_eq!(value(&go_cgo_env(Some(folder), "windows"), "CGO_LDFLAGS"), format!("-L{lib}"));
}

#[test]
fn c_and_cpp_build_with_a_cmake_workflow() {
    for backend in [Backend::C, Backend::Cpp] {
        let settings = shop(backend, Ui::Clarity);
        let debug = clarity_gen::toolchain(&settings, false).unwrap();
        assert_eq!(debug.tool, "cmake");
        assert_eq!(debug.args, ["--workflow", "--preset", "debug"]);
        assert_eq!(debug.program, exe("build/debug/shop"));
        let release = clarity_gen::toolchain(&settings, true).unwrap();
        assert_eq!(release.args, ["--workflow", "--preset", "release"]);
        assert_eq!(release.program, exe("build/release/shop"));
    }
}

#[test]
fn stacks_without_a_generator_have_no_toolchain() {
    assert!(clarity_gen::toolchain(&shop(Backend::Csharp, Ui::Webview), false).is_none());
}

/// The command-line program (DESIGN 5.20) builds in `out/cli` with the same
/// tools, into a program named as it is typed.
#[test]
fn the_command_line_program_builds_with_the_same_tools() {
    let rust = clarity_gen::cli_toolchain(&shop(Backend::Rust, Ui::Clarity), false).unwrap();
    assert_eq!((rust.tool, rust.program.clone()), ("cargo", exe("target/debug/shop")));
    let go = clarity_gen::cli_toolchain(&shop(Backend::Go, Ui::Clarity), true).unwrap();
    assert_eq!(go.tool, "go");
    assert_eq!(go.program, exe("bin/shop"));
    assert!(!go.args.contains(&"production".to_string()), "{:?}", go.args);
    let csharp = clarity_gen::cli_toolchain(&shop(Backend::Csharp, Ui::Clarity), false).unwrap();
    assert_eq!((csharp.tool, csharp.program.clone()), ("dotnet", exe("bin/Debug/net10.0/shop")));
    for backend in [Backend::C, Backend::Cpp] {
        let release = clarity_gen::cli_toolchain(&shop(backend, Ui::Clarity), true).unwrap();
        assert_eq!((release.tool, release.program.clone()), ("cmake", exe("build/release/shop")));
    }
}

/// Clarity.UI's library for Go, C and C++ (DESIGN 7.9) comes from the
/// release of the version Clarity was built with: the ffi crate's.
#[test]
fn clarity_ui_is_the_ffi_crates_version() {
    use clarity_gen::clarity_ui::{version, version_in};
    assert_eq!(version_in("[package]\nname = \"x\"\nversion = \"1.2.3\"\n\n[dependencies]\nversion = \"9\"\n"), Some("1.2.3"));
    assert_eq!(version_in("[dependencies]\nversion = \"9\"\n"), None);
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../runtimes/rust-ui-ffi/Cargo.toml")).unwrap();
    assert_eq!(Some(version()), version_in(&manifest));
}

/// The release's names: one archive per system, under the tag with its slashes.
#[test]
fn clarity_ui_release_names() {
    use clarity_gen::clarity_ui::{RELEASES, archive_name, archive_url, rid, sums_url};
    assert_eq!(rid("linux", "x86_64").as_deref(), Some("linux-x64"));
    assert_eq!(rid("linux", "aarch64").as_deref(), Some("linux-arm64"));
    assert_eq!(rid("windows", "x86_64").as_deref(), Some("win-x64"));
    assert_eq!(rid("windows", "aarch64").as_deref(), Some("win-arm64"));
    assert_eq!(rid("macos", "x86_64").as_deref(), Some("osx-x64"));
    assert_eq!(rid("macos", "aarch64").as_deref(), Some("osx-arm64"));
    assert_eq!(rid("freebsd", "x86_64"), None);
    assert_eq!(rid("linux", "riscv64"), None);
    assert_eq!(archive_name("0.1.0", "linux-x64"), "clarity-ui-0.1.0-linux-x64.tar.gz");
    assert_eq!(
        archive_url(RELEASES, "0.1.0", "osx-arm64"),
        "https://github.com/petervdpas/Clarity/releases/download/runtimes/rust-ui-ffi/v0.1.0/clarity-ui-0.1.0-osx-arm64.tar.gz"
    );
    assert_eq!(sums_url(RELEASES, "0.1.0"), "https://github.com/petervdpas/Clarity/releases/download/runtimes/rust-ui-ffi/v0.1.0/SHA256SUMS");
    // The workflow publishes the same names: its tag, its archives, every system rid() names.
    let workflow = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.github/workflows/release-clarity-ui.yml")).unwrap();
    assert!(workflow.contains("\"runtimes/rust-ui-ffi/v*\""));
    assert!(workflow.contains("name=\"clarity-ui-$version-${{ matrix.rid }}\"") && workflow.contains("tar czf \"$name.tar.gz\" \"$name\""));
    assert!(workflow.contains("sha256sum * > SHA256SUMS"));
    for os in ["linux", "windows", "macos"] {
        for arch in ["x86_64", "aarch64"] {
            let rid = rid(os, arch).unwrap();
            assert!(workflow.contains(&format!("- rid: {rid}\n")), "{rid}");
        }
    }
}

/// Fetched once per version into the person's cache folder, per system.
#[test]
fn clarity_ui_is_kept_in_the_cache_folder() {
    use clarity_gen::clarity_ui::{cache_dir_in, library_dir};
    let env = |vars: &'static [(&'static str, &'static str)]| move |name: &str| vars.iter().find(|(n, _)| *n == name).map(|(_, v)| v.to_string());
    assert_eq!(cache_dir_in("linux", env(&[("HOME", "/home/a")])), Some(PathBuf::from("/home/a/.cache")));
    assert_eq!(cache_dir_in("linux", env(&[("HOME", "/home/a"), ("XDG_CACHE_HOME", "/c")])), Some(PathBuf::from("/c")));
    assert_eq!(cache_dir_in("linux", env(&[("HOME", "/home/a"), ("XDG_CACHE_HOME", "")])), Some(PathBuf::from("/home/a/.cache")));
    assert_eq!(cache_dir_in("macos", env(&[("HOME", "/Users/a")])), Some(PathBuf::from("/Users/a/Library/Caches")));
    assert_eq!(cache_dir_in("windows", env(&[("LOCALAPPDATA", "C:\\Users\\a\\AppData\\Local")])), Some(PathBuf::from("C:\\Users\\a\\AppData\\Local")));
    assert_eq!(cache_dir_in("windows", env(&[("HOME", "/x")])), None);
    assert_eq!(cache_dir_in("linux", env(&[])), None);
    assert_eq!(library_dir(std::path::Path::new("/c"), "0.1.0", "linux-x64"), PathBuf::from("/c/clarity/clarity-ui/0.1.0/linux-x64"));
}

/// SHA256SUMS as sha256sum writes it, and an archive checked against it.
#[test]
fn clarity_ui_archive_is_checked_against_sha256sums() {
    use clarity_gen::clarity_ui::{check_archive, sha256_hex, sum_for};
    assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    let sums = format!("{}  a.tar.gz\n{} *b.tar.gz\n", "1".repeat(64), sha256_hex(b"abc"));
    assert_eq!(sum_for(&sums, "a.tar.gz"), Some("1".repeat(64).as_str()));
    assert_eq!(sum_for(&sums, "b.tar.gz"), Some(sha256_hex(b"abc").as_str()));
    assert_eq!(sum_for(&sums, "c.tar.gz"), None);
    assert!(check_archive(b"abc", &sums, "b.tar.gz").is_ok());
    assert!(check_archive(b"abd", &sums, "b.tar.gz").unwrap_err().contains("not the release's"));
    assert!(check_archive(b"abc", &sums, "c.tar.gz").unwrap_err().contains("names no c.tar.gz"));
}

/// An archive as the release workflow packs it: the folder
/// clarity-ui-<version>-<rid> with include/, lib/ and LICENSE.
fn clarity_ui_archive(version: &str, rid: &str, library: &str) -> Vec<u8> {
    let top = clarity_gen::clarity_ui::folder_name(version, rid);
    let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default()));
    for (path, body) in [("include/clarity_ui.h", "/* header */\n"), (&*format!("lib/{library}"), "library"), ("LICENSE", "MIT")] {
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, format!("{top}/{path}"), body.as_bytes()).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap()
}

/// A first build fetches the library: the checksums, then the archive,
/// checked and unpacked into the cache; the next build finds it there. The
/// folder CLARITY_UI names always wins.
#[test]
fn clarity_ui_is_fetched_once_checked_and_unpacked() {
    use clarity_gen::clarity_ui::{Library, archive_url, complete, library_file, library_in, provide, sha256_hex, sums_url};
    let os = std::env::consts::OS;
    let cache = std::env::temp_dir().join(format!("clarity-ui-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let archive = clarity_ui_archive("9.9.9", "linux-x64", library_file(os));
    let sums = format!("{}  clarity-ui-9.9.9-linux-x64.tar.gz\n", sha256_hex(&archive));
    let base = "https://example.test/releases";
    let asked = std::cell::RefCell::new(Vec::new());
    let get = |url: &str| -> Result<Vec<u8>, String> {
        asked.borrow_mut().push(url.to_string());
        if url == sums_url(base, "9.9.9") {
            Ok(sums.clone().into_bytes())
        } else if url == archive_url(base, "9.9.9", "linux-x64") {
            Ok(archive.clone())
        } else {
            Err(format!("no {url}"))
        }
    };
    let said = std::cell::RefCell::new(Vec::new());
    let say = |line: String| said.borrow_mut().push(line);

    let first = library_in(None, None, Some(&cache), Some("linux-x64"), "9.9.9", os);
    let folder = cache.join("clarity/clarity-ui/9.9.9/linux-x64");
    assert_eq!(first, Library::ToFetch { folder: folder.clone(), version: "9.9.9".into(), rid: "linux-x64".into() });
    assert_eq!(provide(first, base, &get, &say).unwrap(), folder);
    assert!(complete(&folder, os));
    assert_eq!(std::fs::read_to_string(folder.join("LICENSE")).unwrap(), "MIT");
    assert_eq!(asked.borrow().len(), 2);
    assert!(said.borrow()[0].contains("fetching https://example.test/releases/runtimes/rust-ui-ffi/v9.9.9/clarity-ui-9.9.9-linux-x64.tar.gz"), "{:?}", said.borrow());
    // Nothing is left beside it.
    let beside: Vec<_> = std::fs::read_dir(folder.parent().unwrap()).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(beside, ["linux-x64"]);

    // Once per version: the next build finds it.
    let again = library_in(None, None, Some(&cache), Some("linux-x64"), "9.9.9", os);
    assert_eq!(again, Library::Fetched(folder.clone()));
    assert_eq!(provide(again, base, &get, &say).unwrap(), folder);
    assert_eq!(asked.borrow().len(), 2);

    // The folder CLARITY_UI names wins, even with one fetched; an empty one names nothing.
    let named = std::path::Path::new("/opt/clarity-ui");
    assert_eq!(library_in(Some(named.as_os_str()), None, Some(&cache), Some("linux-x64"), "9.9.9", os), Library::Named(named.to_path_buf()));
    assert_eq!(library_in(Some(std::ffi::OsStr::new("")), None, Some(&cache), Some("linux-x64"), "9.9.9", os), Library::Fetched(folder.clone()));
    // The folder the installed Clarity carries comes before the cache, when it is complete; CLARITY_UI before both.
    let bundled = cache.join("installed");
    assert_eq!(library_in(None, Some(&bundled), Some(&cache), Some("linux-x64"), "9.9.9", os), Library::Fetched(folder.clone()));
    std::fs::create_dir_all(bundled.join("include")).unwrap();
    std::fs::create_dir_all(bundled.join("lib")).unwrap();
    std::fs::write(bundled.join("include/clarity_ui.h"), "").unwrap();
    std::fs::write(bundled.join("lib").join(library_file(os)), "").unwrap();
    assert_eq!(library_in(None, Some(&bundled), Some(&cache), Some("linux-x64"), "8.8.8", os), Library::Bundled(bundled.clone()));
    assert_eq!(library_in(None, Some(&bundled), None, None, "8.8.8", os), Library::Bundled(bundled.clone()));
    assert_eq!(library_in(Some(named.as_os_str()), Some(&bundled), Some(&cache), Some("linux-x64"), "8.8.8", os), Library::Named(named.to_path_buf()));
    assert_eq!(provide(Library::Bundled(bundled.clone()), base, &get, &say).unwrap(), bundled);
    std::fs::remove_dir_all(&bundled).unwrap();
    // No archive for this system, or no cache folder: nothing to fetch.
    assert_eq!(library_in(None, None, Some(&cache), None, "9.9.9", os), Library::Unavailable);
    assert_eq!(library_in(None, None, None, Some("linux-x64"), "9.9.9", os), Library::Unavailable);
    assert!(provide(Library::Unavailable, base, &get, &say).unwrap_err().contains("CLARITY_UI"));

    // An archive that is not the release's is refused and not unpacked.
    let other = library_in(None, None, Some(&cache), Some("linux-x64"), "8.8.8", os);
    let wrong = |url: &str| -> Result<Vec<u8>, String> {
        if url.ends_with("SHA256SUMS") { Ok(format!("{}  clarity-ui-8.8.8-linux-x64.tar.gz\n", "0".repeat(64)).into_bytes()) } else { Ok(clarity_ui_archive("8.8.8", "linux-x64", library_file(os))) }
    };
    assert!(provide(other, base, &wrong, &say).unwrap_err().contains("not the release's"));
    assert!(!cache.join("clarity/clarity-ui/8.8.8").exists());
    // A download that fails says so.
    let offline = |url: &str| -> Result<Vec<u8>, String> { Err(format!("cannot download {url}")) };
    let other = library_in(None, None, Some(&cache), Some("linux-x64"), "8.8.8", os);
    assert!(provide(other, base, &offline, &say).unwrap_err().contains("cannot download"));
    std::fs::remove_dir_all(&cache).unwrap();
}

/// An archive without the folder the release names is refused, and leaves nothing.
#[test]
fn clarity_ui_archive_must_hold_its_folder() {
    use clarity_gen::clarity_ui::unpack;
    let cache = std::env::temp_dir().join(format!("clarity-ui-unpack-{}", std::process::id()));
    let into = cache.join("7.7.7/linux-x64");
    let archive = clarity_ui_archive("7.7.7", "win-x64", "clarity_ui.dll");
    assert!(unpack(&archive, "7.7.7", "linux-x64", &into).unwrap_err().contains("clarity-ui-7.7.7-linux-x64/include/clarity_ui.h"));
    assert_eq!(std::fs::read_dir(cache.join("7.7.7")).unwrap().count(), 0);
    assert!(unpack(b"not an archive", "7.7.7", "linux-x64", &into).is_err());
    std::fs::remove_dir_all(&cache).unwrap();
}

/// Clarity's build hands the library's folder to the stack's tools: cgo's
/// flags for Go, CLARITY_UI for CMake; only the windowed Go, C and C++
/// programs link it.
#[test]
fn clarity_ui_goes_to_the_build() {
    let folder = std::path::Path::new("/cache/clarity-ui");
    let go = clarity_gen::with_clarity_ui(clarity_gen::toolchain(&shop(Backend::Go, Ui::Clarity), false).unwrap(), Backend::Go, folder);
    assert_eq!(go.env, clarity_gen::go_cgo_env(Some(folder), std::env::consts::OS));
    for backend in [Backend::C, Backend::Cpp] {
        let cmake = clarity_gen::with_clarity_ui(clarity_gen::toolchain(&shop(backend, Ui::Clarity), false).unwrap(), backend, folder);
        assert_eq!(cmake.env, [("CLARITY_UI", folder.display().to_string())]);
    }
    for backend in [Backend::Go, Backend::C, Backend::Cpp] {
        assert!(clarity_gen::uses_clarity_ui(&shop(backend, Ui::Clarity).stack));
    }
    assert!(!clarity_gen::uses_clarity_ui(&shop(Backend::Rust, Ui::Clarity).stack));
    assert!(!clarity_gen::uses_clarity_ui(&shop(Backend::Csharp, Ui::Clarity).stack));
}

/// The C and C++ runtimes fetch the library the same way, with the version
/// the generated CMakeLists.txt names.
#[test]
fn cmake_fetches_clarity_ui_of_the_generated_version() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../runtimes");
    let c = std::fs::read_to_string(format!("{root}/c/clarity_ui.cmake")).unwrap();
    assert_eq!(c, std::fs::read_to_string(format!("{root}/cpp/clarity_ui.cmake")).unwrap());
    assert!(c.contains("EXPECTED_HASH SHA256=${hash}") && c.contains("file(ARCHIVE_EXTRACT"));
    assert!(c.contains(clarity_gen::clarity_ui::RELEASES));
    let test = std::fs::read_to_string(format!("{root}/c/tests/clarity_ui_fetch.cmake")).unwrap();
    assert_eq!(test, std::fs::read_to_string(format!("{root}/cpp/tests/clarity_ui_fetch.cmake")).unwrap());
    assert!(clarity_gen::clarity_ui::cmake_version_lines().contains(&format!("set(CLARITY_UI_VERSION {})", clarity_gen::clarity_ui::version())));
}

/// Clarity run from its repository serves the library it built as the
/// installer's folder would: header and library copied into target/clarity-ui.
#[test]
fn the_repository_s_library_serves_as_the_carried_one() {
    use clarity_gen::clarity_ui::{complete, library_file, repository_library};
    let os = std::env::consts::OS;
    let repo = std::env::temp_dir().join(format!("clarity-ui-repo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    assert_eq!(repository_library(&repo, os), None);
    std::fs::create_dir_all(repo.join("runtimes/rust-ui-ffi/include")).unwrap();
    std::fs::create_dir_all(repo.join("target/release")).unwrap();
    std::fs::write(repo.join("runtimes/rust-ui-ffi/include/clarity_ui.h"), "/* header */").unwrap();
    assert_eq!(repository_library(&repo, os), None, "the library is not built yet");
    std::fs::write(repo.join("target/release").join(library_file(os)), "one").unwrap();
    std::fs::write(repo.join("target/release/clarity_ui.dll.lib"), "import").unwrap();
    let folder = repository_library(&repo, os).unwrap();
    assert_eq!(folder, repo.join("target/clarity-ui"));
    assert!(complete(&folder, os));
    // A new build of the library is copied again.
    std::fs::write(repo.join("target/release").join(library_file(os)), "second").unwrap();
    repository_library(&repo, os).unwrap();
    assert_eq!(std::fs::read_to_string(folder.join("lib").join(library_file(os))).unwrap(), "second");
    std::fs::remove_dir_all(&repo).unwrap();
}
