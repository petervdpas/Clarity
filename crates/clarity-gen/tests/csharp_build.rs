//! Generates the shop example as C# with Avalonia and builds it with
//! `dotnet build` (the AXAML compiler checks every binding), then runs it with
//! `--check`: the program opens its database, applies the migration steps and
//! stops before showing a window. Slow (NuGet and the compiler), so it only
//! runs when asked: `cargo test -p clarity-gen --test csharp_build -- --ignored`.

use std::path::Path;
use std::process::Command;

use clarity_project::{Backend, Ui};

#[test]
#[ignore]
fn the_shop_example_builds_and_runs_in_csharp() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join("examples/shop.clarity");
    // As Run loads it: with the project file's language, currency and themes.
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    assert!(!project.themes.is_empty() && !project.translations.is_empty(), "the example has themes and translations");
    let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let files = clarity_gen::generate(&project, &settings).unwrap();
    assert!(files.iter().any(|f| f.path == "Texts.nl.resx"));

    let out = std::env::temp_dir().join(format!("clarity-gen-csharp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();

    let built = Command::new("dotnet")
        .args(["build", "--nologo", "-v", "quiet", "-warnaserror"])
        .current_dir(&out)
        .output()
        .expect("the .NET SDK is installed");
    assert!(
        built.status.success(),
        "the generated project does not build; it is in {}\n{}",
        out.display(),
        String::from_utf8_lossy(&built.stdout)
    );

    let ran = Command::new("dotnet").args(["run", "--no-build", "--", "--check"]).current_dir(&out).output().unwrap();
    let said = String::from_utf8_lossy(&ran.stdout);
    assert!(ran.status.success(), "{said}{}", String::from_utf8_lossy(&ran.stderr));
    assert!(said.contains("Shop: the databases are ready."), "{said}");
    // The database the project file names, with every step applied.
    assert!(out.join("data/shop.db").exists());
    // The Dutch texts as a satellite assembly, as .NET looks for them.
    assert!(out.join("bin/Debug/net10.0/nl/Shop.resources.dll").exists());
    std::fs::remove_dir_all(&out).unwrap();
}
