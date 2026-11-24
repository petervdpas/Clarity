//! Generate and Build of a project, as the IDE and the clarity command do them.

use std::path::{Path, PathBuf};

use clarity_build::{GENERATE_IN_THE_IDE, GENERATE_IN_THE_TERMINAL, Generated, can_serve_at, cannot_build, generate, generate_out};

fn generate_source(file: &Path) -> Result<clarity_build::Source, String> {
    generate(file, None)
}

/// A copy of the project `name` in the repository's `folder` (a project of
/// the tests), on the stack its project file names.
fn project_copy(folder: &str, name: &str, test: &str) -> PathBuf {
    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let path = entry.unwrap().path();
            let file = path.file_name().unwrap();
            if path.is_dir() {
                // What a run left behind is not the project's.
                if file != "out" && file != "data" {
                    copy(&path, &to.join(file));
                }
            } else {
                std::fs::copy(&path, to.join(file)).unwrap();
            }
        }
    }
    let projects = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(folder);
    let dir = std::env::temp_dir().join(format!("clarity-build-{name}-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let project = format!("{name}.clarity");
    std::fs::copy(projects.join(&project), dir.join(&project)).unwrap();
    copy(&projects.join(name), &dir.join(name));
    dir
}

#[test]
fn generates_the_project_into_out() {
    let dir = project_copy("test-projects", "shop", "out");
    let file = dir.join("shop.clarity");
    // Build builds only what Generate wrote, and only while it is the project as it is; the IDE and the terminal each say what to do.
    assert!(generate_out(&file, true, false, GENERATE_IN_THE_IDE).err().unwrap_or_default().contains("is not generated yet: press Generate first"));
    assert!(generate_out(&file, true, false, GENERATE_IN_THE_TERMINAL).err().unwrap_or_default().contains("is not generated yet: run clarity gen first"));
    // Generate writes every file again each time, and says each one it wrote.
    let first = generate_source(&file).unwrap().written;
    assert!(first.written.contains(&"desktop/Cargo.toml".to_string()) && first.written.contains(&"desktop/src/main.rs".to_string()), "{first:?}");
    let again = generate_source(&file).unwrap().written;
    assert!(again.written == first.written && again.removed.is_empty(), "{again:?}");
    let Generated { out, toolchain, .. } = generate_out(&file, true, false, GENERATE_IN_THE_IDE).unwrap();
    // The desktop program, in its own folder of the output (manual/code.commands.md).
    assert_eq!(out, dir.join("shop/out/desktop"));
    assert!(out.join("Cargo.toml").exists());
    assert!(out.join("src/main.rs").exists());
    assert_eq!(toolchain.tool, "cargo");
    assert_eq!(out.join(&toolchain.program), out.join(if cfg!(windows) { "target/release/shop.exe" } else { "target/release/shop" }));
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, text.replacen("currency EUR", "currency USD", 1)).unwrap();
    assert!(generate_out(&file, true, false, GENERATE_IN_THE_IDE).err().unwrap_or_default().contains("changed since it was generated: press Generate first"));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Generating is one way for the IDE, the clarity command and the agents:
/// into the project's output folder or one given, saying what it wrote; a
/// project with errors is not generated, and the checker's errors say why.
#[test]
fn generating_says_what_it_wrote_and_why_it_could_not() {
    let dir = project_copy("test-projects", "shop", "lines");
    let file = dir.join("shop.clarity");
    let elsewhere = dir.join("elsewhere");
    let source = generate(&file, Some(&elsewhere)).unwrap();
    assert_eq!(source.out, elsewhere);
    assert!(elsewhere.join("desktop/Cargo.toml").is_file());
    let said = source.lines().join("\n");
    assert!(said.contains(&format!("file(s) in {}", elsewhere.join("desktop").display())), "{said}");
    assert!(said.contains("changed"), "{said}");
    std::fs::write(dir.join("shop/broken.clarity"), "module broken\n\nentity Task\n    title: nonsense\n").unwrap();
    let why = generate(&file, None).err().unwrap_or_default();
    assert!(why.contains("broken.clarity"), "{why}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Without the stack's tools the project is still generated, and Build
/// says what to install, so it can be built here later or elsewhere.
#[test]
fn missing_tools_say_what_to_install_after_generating() {
    let check = clarity_gen::tools::StackTools {
        backend: clarity_project::Backend::Csharp,
        ui: clarity_project::Ui::Clarity,
        generated: true,
        found: vec![],
        missing: vec!["the .NET 10 SDK, from https://dotnet.microsoft.com/download".into()],
        guides: vec!["net-on-linux"],
    };
    assert_eq!(
        cannot_build(&check, Path::new("/p/shop/out")),
        Some(
            "the project is generated in /p/shop/out; to build C# with Clarity.UI here, install the .NET 10 SDK, from https://dotnet.microsoft.com/download (the manual's page Setting up the build tools says how)"
                .to_string()
        )
    );
    let ready = clarity_gen::tools::StackTools { missing: vec![], guides: vec![], found: vec![".NET SDK 10.0.402".into()], ..check };
    assert_eq!(cannot_build(&ready, Path::new("/p/shop/out")), None);
}

#[test]
fn a_web_application_is_served_where_nothing_else_listens() {
    let held = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = held.local_addr().unwrap().to_string();
    let port = held.local_addr().unwrap().port();
    assert_eq!(can_serve_at(&address), Err(format!("another program listens on port {port}; stop it, or set another port on the Project tab")));
    drop(held);
    assert_eq!(can_serve_at(&address), Ok(()));
}

#[test]
fn a_csharp_project_is_generated_for_dotnet() {
    let dir = project_copy("test-projects", "todo", "dotnet");
    generate_source(&dir.join("todo.clarity")).unwrap();
    let Generated { out, toolchain, .. } = generate_out(&dir.join("todo.clarity"), false, false, GENERATE_IN_THE_IDE).unwrap();
    assert!(out.join("Todo.csproj").exists());
    assert!(!out.join("Cargo.toml").exists());
    assert_eq!(toolchain.tool, "dotnet");
    assert!(toolchain.program.starts_with("bin/Debug"), "{}", toolchain.program.display());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A first Go, C or C++ desktop build fetches Clarity.UI's library from
/// its release on GitHub. Ignored: it needs the network and the release
/// of this library version (the tag runtimes/rust-ui-ffi/v<version>).
#[test]
#[ignore]
fn clarity_ui_library_is_fetched_from_its_release() {
    use clarity_gen::clarity_ui;
    let cache = std::env::temp_dir().join(format!("clarity-ui-fetch-{}", std::process::id()));
    let os = std::env::consts::OS;
    let rid = clarity_ui::rid(os, std::env::consts::ARCH);
    let library = clarity_ui::library_in(None, None, Some(&cache), rid.as_deref(), clarity_ui::release_version(), os);
    let folder = clarity_ui::provide(library, clarity_ui::RELEASES, &clarity_build::download, &|line| println!("{line}"));
    let unpacked = folder.as_ref().is_ok_and(|folder| folder.starts_with(&cache) && clarity_ui::complete(folder, os));
    let _ = std::fs::remove_dir_all(&cache);
    assert!(unpacked, "{folder:?}");
}
