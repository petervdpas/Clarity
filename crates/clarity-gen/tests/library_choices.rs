//! Clarity settings > Libraries: the folder each library or package set a
//! build uses may be chosen by the person (Clarity.UI's library, its browser
//! files, the Rust crates, the NuGet packages), kept in environment.json
//! beside the tools, and a factory reset forgets every choice so builds take
//! what belongs to the running Clarity. The IDE (and the `clarity` command)
//! hand the choices to the generator once (`use_libraries`); every build and
//! the preview find them in that one place.

use std::path::{Path, PathBuf};

use clarity_gen::clarity_ui::{self, Library};
use clarity_gen::tools::environment::{self, Choosable, Environment};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("library-choices-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A folder holding Clarity.UI's library as a build takes it.
fn library_folder(name: &str) -> PathBuf {
    let dir = scratch(name);
    std::fs::create_dir_all(dir.join("include")).unwrap();
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(dir.join("include").join("clarity_ui.h"), "").unwrap();
    std::fs::write(dir.join("lib").join(clarity_ui::library_file(std::env::consts::OS)), "").unwrap();
    dir
}

/// A folder holding Clarity.UI's browser files.
fn browser_folder(name: &str) -> PathBuf {
    let dir = scratch(name);
    for file in clarity_ui::BROWSER_FILES {
        std::fs::write(dir.join(file), "").unwrap();
    }
    dir
}

#[test]
fn every_choice_has_a_written_out_camel_case_key() {
    assert_eq!(Choosable::ALL.map(Choosable::key), ["clarityUi", "clarityUiBrowser", "rustCrates", "nugetPackages"]);
    for which in Choosable::ALL {
        assert_eq!(Choosable::from_key(which.key()), Some(which));
    }
    assert_eq!(Choosable::from_key("goModule"), None);
}

#[test]
fn choices_are_kept_beside_the_tools_and_a_reset_forgets_only_them() {
    let dir = scratch("kept");
    let file = dir.join("environment.json");
    // A file from before the Libraries section loads as it was.
    std::fs::write(&file, "{\n  \"tools\": {\n    \"go\": \"/opt/go/bin/go\"\n  }\n}\n").unwrap();
    let mut env = Environment::load(&file);
    assert_eq!(env.chosen("go"), Some(Path::new("/opt/go/bin/go")));
    assert_eq!(env.library(Choosable::ClarityUi), None);
    env.choose_library(Choosable::ClarityUi, Some(PathBuf::from("/work/clarity-ui")));
    env.choose_library(Choosable::RustCrates, Some(PathBuf::from("/work/Clarity/runtimes/rust")));
    env.save(&file).unwrap();
    let again = Environment::load(&file);
    assert_eq!(again, env);
    assert_eq!(again.library(Choosable::ClarityUi), Some(Path::new("/work/clarity-ui")));
    // Without choices the file says nothing of libraries, as before.
    let mut reset = again.clone();
    reset.reset_libraries();
    assert_eq!(reset.library(Choosable::ClarityUi), None);
    assert_eq!(reset.library(Choosable::RustCrates), None);
    assert_eq!(reset.chosen("go"), Some(Path::new("/opt/go/bin/go")));
    reset.save(&file).unwrap();
    assert!(!std::fs::read_to_string(&file).unwrap().contains("libraries"));
    // One choice forgotten alone.
    let mut one = again;
    one.choose_library(Choosable::ClarityUi, None);
    assert_eq!(one.library(Choosable::ClarityUi), None);
    assert!(one.library(Choosable::RustCrates).is_some());
}

#[test]
fn a_chosen_folder_must_hold_what_it_is_chosen_for() {
    let os = std::env::consts::OS;
    let library = library_folder("ui");
    assert_eq!(Choosable::ClarityUi.folder_in(&library, os).unwrap(), library);
    let browser = browser_folder("browser");
    assert_eq!(Choosable::ClarityUiBrowser.folder_in(&browser, os).unwrap(), browser);
    // The wrong folder says what it lacks, and names the folder.
    let empty = scratch("empty");
    for which in Choosable::ALL {
        let why = which.folder_in(&empty, os).unwrap_err();
        assert!(why.contains(&empty.display().to_string()), "{why}");
    }
    assert!(Choosable::ClarityUi.folder_in(&browser, os).unwrap_err().contains("clarity_ui.h"));
    assert!(Choosable::ClarityUiBrowser.folder_in(&library, os).unwrap_err().contains("clarity-ui.js"));
}

/// Clarity's repository, its runtimes folder or a runtime beside the one
/// wanted all point at the runtime itself, which is what is kept.
#[test]
fn the_runtimes_are_found_from_any_folder_of_the_repository() {
    let os = std::env::consts::OS;
    let repo = repo();
    let runtimes = repo.join("runtimes");
    for picked in [repo.clone(), runtimes.clone(), runtimes.join("csharp"), runtimes.join("rust")] {
        assert_eq!(Choosable::RustCrates.folder_in(&picked, os).unwrap(), runtimes.join("rust"), "{}", picked.display());
        assert_eq!(Choosable::NugetPackages.folder_in(&picked, os).unwrap(), runtimes.join("csharp"), "{}", picked.display());
    }
}

#[test]
fn a_chosen_folder_comes_before_what_clarity_finds() {
    let chosen = PathBuf::from("/work/clarity-ui");
    let found = Library::Fetched(PathBuf::from("/cache/clarity-ui"));
    assert_eq!(clarity_ui::chosen_or(Some(chosen.clone()), found.clone()), Library::Chosen(chosen.clone()));
    assert_eq!(clarity_ui::chosen_or(None, found.clone()), found);
    // It is used as chosen, without a download.
    let get = |_: &str| -> Result<Vec<u8>, String> { panic!("nothing is fetched") };
    assert_eq!(clarity_ui::provide(Library::Chosen(chosen.clone()), "https://example.invalid", &get, &|_| {}).unwrap(), chosen);
}

/// The choices the IDE hands over reach every build: Clarity.UI's library
/// and browser files through `library` and `browser_library`, the NuGet
/// packages through a build's variables, the Rust crates through the
/// project's patch. The only test of this binary that hands any over.
#[test]
fn handed_choices_reach_the_builds() {
    let library = library_folder("in-use-ui");
    let browser = browser_folder("in-use-browser");
    let csharp = repo().join("runtimes").join("csharp");
    let rust = repo().join("runtimes").join("rust");
    let mut env = Environment::default();
    env.choose_library(Choosable::ClarityUi, Some(library.clone()));
    env.choose_library(Choosable::ClarityUiBrowser, Some(browser.clone()));
    env.choose_library(Choosable::NugetPackages, Some(csharp.clone()));
    env.choose_library(Choosable::RustCrates, Some(rust.clone()));
    environment::use_libraries(&env);
    assert_eq!(environment::library_in_use(Choosable::ClarityUi), Some(library.clone()));
    assert_eq!(clarity_ui::library(), Library::Chosen(library.clone()));
    assert_eq!(clarity_ui::browser_library(), Library::Chosen(browser.clone()));
    assert_eq!(clarity_gen::web_browser::local_folder(None).unwrap(), browser);
    let vars = clarity_gen::tools::tool_env();
    assert!(vars.contains(&("CLARITY_RUNTIME".to_string(), clarity_project::slashed(&csharp))), "{vars:?}");
    // A Rust project patches the runtime onto the chosen crates.
    let mut settings: clarity_project::Settings = clarity_project::starter("Shop", "shop");
    settings.stack.backend = clarity_project::Backend::Rust;
    settings.stack.ui = clarity_project::Ui::Clarity;
    let project = clarity_spec::load_converted([("shop.clarity", "module shop\n\nentity Customer\n    name: text(80)\n\npage Card: window\n    title \"Card\"\n    data customer: Customer\n")]);
    let files = clarity_gen::generate(&project, &settings).unwrap();
    let patch = files.iter().find(|f| f.path == ".cargo/config.toml").expect("the patch is written");
    assert_eq!(patch.text, clarity_gen::rust_local_patch(&rust).unwrap());
    // A factory reset: Clarity's own again, and nothing of the choices stays in a build.
    environment::use_libraries(&Environment::default());
    assert_eq!(environment::library_in_use(Choosable::ClarityUi), None);
    assert!(!matches!(clarity_ui::library(), Library::Chosen(_)));
    assert!(!clarity_gen::tools::tool_env().iter().any(|(k, v)| k == "CLARITY_RUNTIME" && v == &clarity_project::slashed(&csharp)));
    // CLARITY_RUNTIME of the environment (as dev.sh sets it) still counts after a reset: it is not set in this binary.
    if std::env::var_os("CLARITY_RUNTIME").is_none() {
        let files = clarity_gen::generate(&project, &settings).unwrap();
        assert!(!files.iter().any(|f| f.path == ".cargo/config.toml"));
    }
}

/// A patch Clarity wrote goes when it is no longer generated (after a reset),
/// so the next build takes the published crates; one written by hand stays.
#[test]
fn a_patch_clarity_wrote_goes_when_it_is_no_longer_generated() {
    let out = scratch("patch");
    let patch = clarity_gen::rust_local_patch(&repo().join("runtimes").join("rust")).unwrap();
    std::fs::create_dir_all(out.join(".cargo")).unwrap();
    std::fs::write(out.join(".cargo").join("config.toml"), &patch).unwrap();
    let keep = clarity_gen::build_output(clarity_project::Backend::Rust);
    clarity_gen::write_out(&out, &[], keep).unwrap();
    assert!(!out.join(".cargo").join("config.toml").exists());
    // By hand: kept.
    std::fs::create_dir_all(out.join(".cargo")).unwrap();
    std::fs::write(out.join(".cargo").join("config.toml"), "[patch.crates-io]\nclarity-runtime = { path = \"/mine\" }\n").unwrap();
    clarity_gen::write_out(&out, &[], keep).unwrap();
    assert!(out.join(".cargo").join("config.toml").exists());
}
