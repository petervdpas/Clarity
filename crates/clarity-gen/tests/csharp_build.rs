//! Generates the shop example as C# and builds it with `dotnet build`, then
//! runs it: without windows yet (G2a), the program opens its database,
//! applies the migration steps and stops. Slow (NuGet and the compiler), so
//! it only runs when asked: `cargo test -p clarity-gen --test csharp_build -- --ignored`.

use std::path::Path;
use std::process::Command;

use clarity_project::{Backend, Ui};

#[test]
#[ignore]
fn the_shop_example_builds_and_runs_in_csharp() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join("examples/shop.clarity");
    let project = clarity_spec::load_dir(&clarity_project::module_dir(&file)).unwrap();
    let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let files = clarity_gen::generate(&project, &settings).unwrap();

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

    let ran = Command::new("dotnet").args(["run", "--no-build"]).current_dir(&out).output().unwrap();
    let said = String::from_utf8_lossy(&ran.stdout);
    assert!(ran.status.success(), "{said}{}", String::from_utf8_lossy(&ran.stderr));
    assert!(said.contains("Shop: the databases are ready."), "{said}");
    // The database the project file names, with every step applied.
    assert!(out.join("data/shop.db").exists());
    std::fs::remove_dir_all(&out).unwrap();
}
