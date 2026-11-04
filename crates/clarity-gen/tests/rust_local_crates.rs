//! A Rust project builds on the crates in Clarity's repository, while working
//! on them, through the environment variable CLARITY_RUNTIME (as a C# project
//! does): set while Clarity generates the project, it names the Rust
//! runtime's folder (`<Clarity>/runtimes/rust`), or any runtime folder of the
//! repository, the runtimes folder or the repository, and the generator writes
//! `.cargo/config.toml` patching the crates the project uses onto that folder
//! and its neighbours. Unset, nothing is written and the project builds on
//! crates.io.

use std::path::{Path, PathBuf};

use clarity_gen::{generate, rust_local_patch};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::load_sources;

fn runtime() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/rust").canonicalize().unwrap()
}

/// A path as TOML holds it: `/` between the folders, never a backslash.
fn slashed(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

#[test]
fn a_clarity_ui_project_patches_the_runtime_and_clarity_ui() {
    let text = rust_local_patch(&runtime(), true).unwrap();
    assert!(text.contains("[patch.crates-io]\n"), "{text}");
    assert!(text.contains(&format!("clarity-runtime = {{ path = \"{}\" }}\n", slashed(&runtime()))), "{text}");
    let ui = runtime().parent().unwrap().join("rust-ui");
    assert!(text.contains(&format!("clarity-ui = {{ path = \"{}\" }}\n", slashed(&ui))), "{text}");
    // The file says where it comes from and how to go back to crates.io.
    assert!(text.starts_with("# Written by Clarity"), "{text}");
    assert!(text.contains("CLARITY_RUNTIME"), "{text}");
}

#[test]
fn a_webview_project_patches_only_the_runtime() {
    // A patch of a crate the project does not use would be a warning in every build.
    let text = rust_local_patch(&runtime(), false).unwrap();
    assert!(text.contains("clarity-runtime = "), "{text}");
    assert!(!text.contains("clarity-ui"), "{text}");
}

/// One CLARITY_RUNTIME serves every language: naming the C# runtime's
/// folder (as the IDE is started for C#), the runtimes folder or the
/// repository, a Rust project finds the Rust runtime beside it.
#[test]
fn the_rust_runtime_is_found_from_any_runtime_folder_of_the_repository() {
    let runtimes = runtime().parent().unwrap().to_path_buf();
    let want = rust_local_patch(&runtime(), true).unwrap();
    for named in [runtimes.join("csharp"), runtimes.clone(), runtimes.parent().unwrap().to_path_buf()] {
        assert_eq!(want, rust_local_patch(&named, true).unwrap(), "{}", named.display());
    }
}

#[test]
fn a_folder_without_the_runtime_says_what_to_name() {
    let err = rust_local_patch(Path::new("/no/such/folder"), false).unwrap_err();
    assert!(err.contains("CLARITY_RUNTIME"), "{err}");
    assert!(err.contains("runtimes/rust"), "{err}");
    assert!(err.contains("/no/such/folder"), "{err}");
}

/// The only test of this file that sets the variable or generates, so no
/// other test sees it.
#[test]
fn generating_with_clarity_runtime_set_writes_the_patch() {
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Rust;
    settings.stack.ui = Ui::Clarity;
    let project = load_sources([("shop.clarity", "module shop\n\nentity Customer\n    name: text(80)\n\npage Card: window\n    title \"Card\"\n    data customer: Customer\n")]);
    // SAFETY: no other thread of this test binary reads or writes the environment.
    unsafe { std::env::remove_var("CLARITY_RUNTIME") };
    let plain = generate(&project, &settings).unwrap();
    assert!(!plain.iter().any(|f| f.path == ".cargo/config.toml"), "no patch without the variable");
    unsafe { std::env::set_var("CLARITY_RUNTIME", runtime()) };
    let patched = generate(&project, &settings);
    unsafe { std::env::remove_var("CLARITY_RUNTIME") };
    let patched = patched.unwrap();
    let config = patched.iter().find(|f| f.path == ".cargo/config.toml").expect("the patch is written");
    assert_eq!(config.text, rust_local_patch(&runtime(), true).unwrap());
    // The README says how.
    let readme = &patched.iter().find(|f| f.path == "README.md").unwrap().text;
    assert!(readme.contains("set the environment variable `CLARITY_RUNTIME` to `<Clarity>/runtimes/rust`"), "{readme}");
}
