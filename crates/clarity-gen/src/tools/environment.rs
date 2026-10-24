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

/// The tools the person chose, by program.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    #[serde(default)]
    tools: BTreeMap<String, PathBuf>,
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
        .filter_map(|(variable, program)| env.chosen(program).map(|p| (variable.to_string(), p.display().to_string())))
        .collect()
}
