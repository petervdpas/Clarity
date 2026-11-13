//! Clarity's environment: where the build tools of the five stacks are on
//! this computer. A scan finds every copy of each tool; the person may
//! choose one, which every build Clarity runs then uses first, for every
//! project. Kept in `environment.json` in Clarity's own folder, beside the
//! vault, so the app and the `clarity` command read the same choices.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use clarity_project::Backend;
use serde::{Deserialize, Serialize};

/// A build tool: the program Clarity runs, what it is, and the stacks that build with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tool {
    pub program: &'static str,
    pub name: &'static str,
    pub stacks: &'static [Backend],
}

const C_AND_CPP: &[Backend] = &[Backend::C, Backend::Cpp];

/// The tools of every stack, in the order the Environment settings show them.
pub const TOOLS: [Tool; 9] = [
    Tool { program: "cargo", name: "Rust (cargo)", stacks: &[Backend::Rust] },
    Tool { program: "go", name: "Go", stacks: &[Backend::Go] },
    Tool { program: "dotnet", name: ".NET SDK", stacks: &[Backend::Csharp] },
    Tool { program: "cmake", name: "CMake", stacks: C_AND_CPP },
    Tool { program: "cc", name: "C compiler", stacks: &[Backend::C] },
    Tool { program: "c++", name: "C++ compiler", stacks: C_AND_CPP },
    Tool { program: "make", name: "make", stacks: C_AND_CPP },
    Tool { program: "ninja", name: "Ninja", stacks: C_AND_CPP },
    Tool { program: "pkg-config", name: "pkg-config (Linux)", stacks: &[Backend::Rust, Backend::Go, Backend::C, Backend::Cpp] },
];

/// Other names a tool goes by, which a scan finds too: a C compiler is
/// also gcc or clang; Windows' Visual Studio compiler is cl.
fn names(program: &str) -> &'static [&'static str] {
    match program {
        "cc" => &["cc", "gcc", "clang", "cl"],
        "c++" => &["c++", "g++", "clang++", "cl"],
        "cargo" => &["cargo"],
        "go" => &["go"],
        "dotnet" => &["dotnet"],
        "cmake" => &["cmake"],
        "make" => &["make", "mingw32-make"],
        "ninja" => &["ninja"],
        "pkg-config" => &["pkg-config"],
        _ => &[],
    }
}

/// The tools the person chose, by program, and the folders chosen for
/// Clarity's libraries and packages (Clarity settings > Libraries), by key.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    #[serde(default)]
    tools: BTreeMap<String, PathBuf>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    libraries: BTreeMap<String, PathBuf>,
}

impl Environment {
    /// The choices in `file`; none when it is not there or cannot be read.
    pub fn load(file: &Path) -> Environment {
        std::fs::read_to_string(file).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
    }

    /// The choices of this computer, from Clarity's own folder.
    pub fn current() -> Environment {
        Environment::load(&file())
    }

    pub fn save(&self, file: &Path) -> std::io::Result<()> {
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(file, serde_json::to_string_pretty(self).unwrap_or_default() + "\n")
    }

    /// Chooses `path` for `program`, or forgets the choice.
    pub fn choose(&mut self, program: &str, path: Option<PathBuf>) {
        match path {
            Some(path) => self.tools.insert(program.to_string(), path),
            None => self.tools.remove(program),
        };
    }

    pub fn chosen(&self, program: &str) -> Option<&Path> {
        self.tools.get(program).map(PathBuf::as_path)
    }

    /// Chooses `folder` for `which`, or forgets the choice: builds then take Clarity's own.
    pub fn choose_library(&mut self, which: Choosable, folder: Option<PathBuf>) {
        match folder {
            Some(folder) => self.libraries.insert(which.key().to_string(), folder),
            None => self.libraries.remove(which.key()),
        };
    }

    /// The folder chosen for `which`, if any.
    pub fn library(&self, which: Choosable) -> Option<&Path> {
        self.libraries.get(which.key()).map(PathBuf::as_path)
    }

    /// The factory reset: every library choice forgotten, so builds take
    /// the libraries that belong to the running Clarity. The tools stay.
    pub fn reset_libraries(&mut self) {
        self.libraries.clear();
    }
}

/// What a build uses of Clarity whose folder the person may choose, in
/// place of the one that belongs to the running Clarity (Clarity settings >
/// Libraries). The Go module and the C and C++ runtime have no choice: the
/// generator cannot point a program at a folder of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choosable {
    /// Clarity.UI's library, which Go, C, C++ and Rust desktop programs link.
    ClarityUi,
    /// Clarity.UI's browser files: web programs, and the IDE's design and preview.
    ClarityUiBrowser,
    /// The Rust crates (clarity-runtime and those it takes): `<Clarity>/runtimes/rust`.
    RustCrates,
    /// The NuGet packages, as the runtime's own projects: `<Clarity>/runtimes/csharp`.
    NugetPackages,
}

impl Choosable {
    pub const ALL: [Choosable; 4] = [Choosable::ClarityUi, Choosable::ClarityUiBrowser, Choosable::RustCrates, Choosable::NugetPackages];

    /// The key the choice is kept under, and the IDE names it by.
    pub fn key(self) -> &'static str {
        match self {
            Choosable::ClarityUi => "clarityUi",
            Choosable::ClarityUiBrowser => "clarityUiBrowser",
            Choosable::RustCrates => "rustCrates",
            Choosable::NugetPackages => "nugetPackages",
        }
    }

    pub fn from_key(key: &str) -> Option<Choosable> {
        Choosable::ALL.into_iter().find(|c| c.key() == key)
    }

    /// The folder to keep for `picked` on system `os`: `picked` itself when
    /// it holds what this is, else the one Clarity's repository holds it in
    /// when `picked` is (a folder of) the repository. Why not, naming
    /// `picked` and what it lacks.
    pub fn folder_in(self, picked: &Path, os: &str) -> Result<PathBuf, String> {
        let shown = picked.display();
        let found = match self {
            Choosable::ClarityUi => {
                let candidates = [picked.to_path_buf(), picked.join("target").join("clarity-ui")];
                candidates.into_iter().find(|c| crate::clarity_ui::complete(c, os)).ok_or_else(|| {
                    format!("{shown} does not hold Clarity.UI's library: it needs include/clarity_ui.h and lib/{}", crate::clarity_ui::library_file(os))
                })
            }
            Choosable::ClarityUiBrowser => {
                let candidates = [picked.to_path_buf(), picked.join("target").join("clarity-ui-browser")];
                candidates.into_iter().find(|c| crate::clarity_ui::browser_complete(c)).ok_or_else(|| {
                    format!("{shown} does not hold Clarity.UI's browser files: it needs {}", crate::clarity_ui::BROWSER_FILES.join(", "))
                })
            }
            Choosable::RustCrates => runtime_in(picked, "rust", &|f| std::fs::read_to_string(f.join("Cargo.toml")).is_ok_and(|m| m.contains("\nname = \"clarity-runtime\"")))
                .ok_or_else(|| format!("{shown} does not hold Clarity's Rust crates: choose <Clarity>/runtimes/rust, which holds clarity-runtime")),
            Choosable::NugetPackages => runtime_in(picked, "csharp", &|f| f.join("Clarity.Ui").join("Clarity.Ui.csproj").is_file())
                .ok_or_else(|| format!("{shown} does not hold the projects of Clarity's NuGet packages: choose <Clarity>/runtimes/csharp, which holds Clarity.Ui")),
        }?;
        Ok(found.canonicalize().map(|f| PathBuf::from(clarity_project::slashed(&f))).unwrap_or(found))
    }
}

/// The runtime folder `name` (`rust`, `csharp`) for `named`: the folder
/// itself, a runtime beside it, the runtimes folder or the repository; the
/// first that `holds` the runtime.
pub fn runtime_in(named: &Path, name: &str, holds: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    let candidates = [named.to_path_buf(), named.join("..").join(name), named.join(name), named.join("runtimes").join(name)];
    candidates.into_iter().find(|c| holds(c))
}

/// The library choices this process builds with, which the IDE (and the
/// `clarity` command) hand over from environment.json with `use_libraries`.
/// Kept apart from the file, so a test sees only what it hands over.
static IN_USE: std::sync::RwLock<BTreeMap<String, PathBuf>> = std::sync::RwLock::new(BTreeMap::new());

/// Builds from now on use the library choices of `env`; none after a reset.
pub fn use_libraries(env: &Environment) {
    if let Ok(mut in_use) = IN_USE.write() {
        *in_use = env.libraries.clone();
    }
}

/// The folder builds use for `which` as chosen, if one is.
pub fn library_in_use(which: Choosable) -> Option<PathBuf> {
    IN_USE.read().ok().and_then(|in_use| in_use.get(which.key()).cloned())
}

/// Where this computer's environment is kept: `environment.json` in Clarity's own folder.
pub fn file() -> PathBuf {
    clarity_project::clarity_home().join("environment.json")
}

/// The file of `program`: the chosen one while it is there, else the first in `path`.
pub fn find(program: &str, env: &Environment, path: &OsStr) -> Option<PathBuf> {
    if let Some(chosen) = env.chosen(program).filter(|p| super::is_program(p)) {
        return Some(chosen.to_path_buf());
    }
    super::find_on_path(program, path)
}

/// Every copy of `program` in the folders of `path`, under any of its names, in order.
pub fn candidates(program: &str, path: &OsStr) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in std::env::split_paths(path) {
        for name in names(program).iter().copied().chain(std::iter::once(program)) {
            if let Some(file) = super::find_on_path(name, dir.as_os_str())
                && !found.contains(&file)
            {
                found.push(file);
            }
        }
    }
    found
}

/// `path` with the folders of the chosen tools first, so a build runs them
/// and the tools they start themselves (rustc of the chosen cargo, the
/// compilers of the chosen Go).
pub fn search_path(env: &Environment, path: &OsStr) -> OsString {
    let mut folders: Vec<PathBuf> = Vec::new();
    for chosen in env.tools.values() {
        if let Some(folder) = chosen.parent().map(Path::to_path_buf)
            && !folders.contains(&folder)
        {
            folders.push(folder);
        }
    }
    for folder in std::env::split_paths(path) {
        if !folders.contains(&folder) {
            folders.push(folder);
        }
    }
    std::env::join_paths(folders).unwrap_or_default()
}

/// The chosen C and C++ compilers as CMake reads them: `CC` and `CXX`.
pub fn compilers(env: &Environment) -> Vec<(String, String)> {
    [("CC", "cc"), ("CXX", "c++")]
        .into_iter()
        .filter_map(|(variable, program)| env.chosen(program).map(|p| (variable.to_string(), clarity_project::slashed(p))))
        .collect()
}
