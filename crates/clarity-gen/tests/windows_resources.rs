//! The program file's resources on Windows: its icon
//! and its version
//!, checked against Windows' own formats
//! with real tools where the machine has them: MinGW links a program with
//! Clarity's resource object, and llvm-readobj reads the resources back out
//! of the program. Skipped where the tools are missing.

use std::path::{Path, PathBuf};
use std::process::Command;

use clarity_gen::windows_resources::{ico, icon_resources, images, resource_object, resources_object, version_info, Resource, MACHINES, SIZES};

fn tool(names: &[&str]) -> Option<String> {
    names.iter().find(|n| Command::new(n).arg("--version").output().is_ok_and(|o| o.status.success())).map(|n| n.to_string())
}

fn folder(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("windows-resources-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn clarity_icon() -> Vec<(u32, Vec<u8>)> {
    let svg = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src-tauri/icons/icon.svg")).unwrap();
    images("icon.svg", &svg).unwrap()
}

#[test]
fn a_program_linked_with_the_resource_object_carries_the_icon() {
    let (Some(cc), Some(readobj)) = (tool(&["x86_64-w64-mingw32-gcc"]), tool(&["llvm-readobj-19", "llvm-readobj"])) else {
        eprintln!("MinGW or llvm-readobj is missing: skipped");
        return;
    };
    let dir = folder("link");
    let drawn = clarity_icon();
    let (_, machine, relocation) = MACHINES[0];
    std::fs::write(dir.join("icon-amd64.obj"), resource_object(&drawn, machine, relocation)).unwrap();
    std::fs::write(dir.join("main.c"), "int main(void) { return 0; }\n").unwrap();
    let linked = Command::new(&cc).current_dir(&dir).args(["main.c", "icon-amd64.obj", "-o", "app.exe"]).output().unwrap();
    assert!(linked.status.success(), "{}", String::from_utf8_lossy(&linked.stderr));
    let read = Command::new(&readobj).arg("--coff-resources").arg(dir.join("app.exe")).output().unwrap();
    let text = String::from_utf8_lossy(&read.stdout);
    assert!(read.status.success(), "{}", String::from_utf8_lossy(&read.stderr));
    assert!(text.contains("ICON"), "{text}");
    assert!(text.contains("GROUP_ICON"), "{text}");
    // Each size an icon of its own, and one group naming them.
    assert_eq!(text.matches("Name: (ID ").count(), SIZES.len() + 1, "{text}");
    // The linker placed each icon's address: every one points at its PNG.
    assert_eq!(text.matches("89504E47 0D0A1A0A").count(), SIZES.len(), "{text}");
}

#[test]
fn every_processor_s_object_is_read_as_a_resource_object() {
    let Some(readobj) = tool(&["llvm-readobj-19", "llvm-readobj"]) else {
        eprintln!("llvm-readobj is missing: skipped");
        return;
    };
    let dir = folder("objects");
    let drawn = clarity_icon();
    for (arch, machine, relocation) in MACHINES {
        let path = dir.join(format!("icon-{arch}.obj"));
        std::fs::write(&path, resource_object(&drawn, machine, relocation)).unwrap();
        let read = Command::new(&readobj).arg("--coff-resources").arg("--relocations").arg(&path).output().unwrap();
        let text = String::from_utf8_lossy(&read.stdout);
        assert!(read.status.success(), "{arch}: {}", String::from_utf8_lossy(&read.stderr));
        assert!(text.contains("GROUP_ICON"), "{arch}: {text}");
        assert!(text.contains("ADDR32NB") || text.contains("DIR32NB"), "{arch}: {text}");
    }
    // The icon file reads as one too.
    std::fs::write(dir.join("app.ico"), ico(&drawn)).unwrap();
    let kind = Command::new("file").arg(dir.join("app.ico")).output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    assert!(kind.is_empty() || kind.contains("icon resource"), "{kind}");
}

/// Go links each `.syso` for its own system and processor: a Windows
/// program built by go itself carries the icon, on x64 and on ARM64.
#[test]
fn go_links_the_icon_into_a_windows_program() {
    let (Some(_), Some(readobj)) = (tool(&["go"]).or_else(|| Command::new("go").arg("version").output().ok().filter(|o| o.status.success()).map(|_| "go".to_string())), tool(&["llvm-readobj-19", "llvm-readobj"])) else {
        eprintln!("go or llvm-readobj is missing: skipped");
        return;
    };
    let dir = folder("go");
    let drawn = clarity_icon();
    for (arch, machine, relocation) in MACHINES {
        std::fs::write(dir.join(clarity_gen::windows_resources::syso_path(arch)), resource_object(&drawn, machine, relocation)).unwrap();
    }
    std::fs::write(dir.join("go.mod"), "module icon\n\ngo 1.22\n").unwrap();
    std::fs::write(dir.join("main.go"), "package main\n\nfunc main() {}\n").unwrap();
    for arch in ["amd64", "arm64"] {
        let built = Command::new("go").current_dir(&dir).env("GOOS", "windows").env("GOARCH", arch).env("GOFLAGS", "-mod=mod").env("GOTOOLCHAIN", "local").args(["build", "-o", &format!("app-{arch}.exe"), "."]).output().unwrap();
        assert!(built.status.success(), "{arch}: {}", String::from_utf8_lossy(&built.stderr));
        let read = Command::new(&readobj).arg("--coff-resources").arg(dir.join(format!("app-{arch}.exe"))).output().unwrap();
        let text = String::from_utf8_lossy(&read.stdout);
        assert!(text.contains("GROUP_ICON"), "{arch}: {text}");
        assert_eq!(text.matches("89504E47 0D0A1A0A").count(), SIZES.len(), "{arch}: {text}");
    }
}

/// Every stack carries the icon its own way: C# names the icon file in its
/// project, Go has a `.syso` per processor beside main.go, Rust links an
/// object from build.rs, C and C++ from CMakeLists.txt.
#[test]
fn every_stack_carries_the_project_s_icon() {
    let root = folder("stacks");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    for stack in ["csharp desktop", "go desktop", "rust desktop", "c desktop", "cpp desktop"] {
        let project_file = root.join("todo.clarity");
        let dir = root.join("todo");
        let _ = std::fs::remove_dir_all(&dir);
        copy(&source.join("todo"), &dir);
        let text = std::fs::read_to_string(source.join("todo.clarity")).unwrap();
        let stack_line = text.lines().find(|l| l.trim_start().starts_with("stack ")).unwrap().to_string();
        // The example names its own icon; this test gives it another, a picture of a kind it does not have.
        let text: String = text.lines().filter(|l| !l.trim_start().starts_with("icon ")).map(|l| format!("{l}\n")).collect();
        std::fs::write(&project_file, text.replace(&stack_line, &format!("    stack {stack}\n    icon \"app.svg\""))).unwrap();
        std::fs::create_dir_all(dir.join("resources")).unwrap();
        std::fs::copy(source.join("../app/src-tauri/icons/icon.svg"), dir.join("resources/app.svg")).unwrap();
        let project = clarity_project::load_modules(&dir).unwrap();
        let settings = clarity_project::load(&project_file).settings.unwrap();
        assert_eq!(project.icon.as_deref(), Some("app.svg"), "{stack}");
        let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{stack}: {e:?}"));
        let has = |path: &str| files.iter().any(|f| f.path == path);
        let text_of = |path: &str| files.iter().find(|f| f.path == path).map(|f| f.text.clone()).unwrap_or_default();
        match stack {
            "csharp desktop" => {
                assert!(has("windows/app.ico"), "{stack}");
                let csproj = files.iter().find(|f| f.path.ends_with(".csproj")).unwrap();
                assert!(csproj.text.contains("<ApplicationIcon>windows\\app.ico</ApplicationIcon>"), "{}", csproj.text);
            }
            "go desktop" => assert!(["amd64", "arm64", "386"].iter().all(|a| has(&format!("resources_windows_{a}.syso"))), "{stack}"),
            "rust desktop" => {
                assert!(has("windows/resources-amd64.obj") && has("build.rs"), "{stack}");
                assert!(text_of("build.rs").contains("cargo:rustc-link-arg-bins"), "{stack}");
            }
            _ => {
                assert!(has("windows/resources-arm64.obj"), "{stack}");
                assert!(text_of("CMakeLists.txt").contains("windows/resources-${CLARITY_RESOURCES_CPU}.obj"), "{stack}");
            }
        }
    }
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let name = entry.file_name();
        if name == "out" || name == "data" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() { copy(&path, &to.join(&name)) } else { std::fs::copy(&path, to.join(&name)).map(|_| ()).unwrap() }
    }
}

/// UTF-16 as it lies in a version resource, as llvm-readobj prints its bytes in hex.
fn wide_hex(text: &str) -> String {
    text.encode_utf16().flat_map(|c| c.to_le_bytes()).map(|b| format!("{b:02X}")).collect()
}

/// The version beside the icon: a program linked with the resource object
/// carries a VERSIONINFO whose texts are the title and the version.
#[test]
fn a_program_linked_with_the_resource_object_carries_the_version() {
    let (Some(cc), Some(readobj)) = (tool(&["x86_64-w64-mingw32-gcc"]), tool(&["llvm-readobj-19", "llvm-readobj"])) else {
        eprintln!("MinGW or llvm-readobj is missing: skipped");
        return;
    };
    let dir = folder("version");
    let mut resources = icon_resources(&clarity_icon());
    resources.push(Resource { kind: 16, id: 1, data: version_info("1.2.3", "Todo", "todo") });
    let (_, machine, relocation) = MACHINES[0];
    std::fs::write(dir.join("resources-amd64.obj"), resources_object(&resources, machine, relocation)).unwrap();
    std::fs::write(dir.join("main.c"), "int main(void) { return 0; }\n").unwrap();
    let linked = Command::new(&cc).current_dir(&dir).args(["main.c", "resources-amd64.obj", "-o", "app.exe"]).output().unwrap();
    assert!(linked.status.success(), "{}", String::from_utf8_lossy(&linked.stderr));
    let read = Command::new(&readobj).arg("--coff-resources").arg(dir.join("app.exe")).output().unwrap();
    let text = String::from_utf8_lossy(&read.stdout).replace([' ', '\n'], "");
    assert!(read.status.success(), "{}", String::from_utf8_lossy(&read.stderr));
    assert!(text.contains("Type:VERSIONINFO(ID16)"), "{text}");
    assert!(text.contains("GROUP_ICON"), "the icon stays beside it");
    // The linker placed the version's address: it points at the whole resource.
    let size = version_info("1.2.3", "Todo", "todo").len();
    let after = &text[text.find("Type:VERSIONINFO(ID16)").unwrap()..];
    assert!(after.contains(&format!("DataSize:{size}")), "{after}");
    assert!(after.contains(&wide_hex("VS_")), "it starts with its key: {after}");
}

/// A version resource's blocks say their own lengths, each on a 4-byte
/// boundary, and its numbers say the version for Windows to compare.
#[test]
fn the_version_resource_is_laid_out_as_windows_reads_it() {
    let info = version_info("2.10.4-beta.1", "Shop", "shop");
    let length = u16::from_le_bytes([info[0], info[1]]) as usize;
    assert_eq!(length, info.len(), "the outer block is the whole resource");
    assert_eq!(u16::from_le_bytes([info[2], info[3]]), 52, "its value is the fixed file info");
    let fixed = info.windows(4).position(|w| w == [0xBD, 0x04, 0xEF, 0xFE]).expect("the fixed file info's signature");
    assert_eq!(fixed % 4, 0);
    let dword = |at: usize| u32::from_le_bytes(info[fixed + at..fixed + at + 4].try_into().unwrap());
    assert_eq!(dword(8), (2 << 16) | 10, "2.10");
    assert_eq!(dword(12), 4 << 16, ".4");
    assert_eq!(dword(28) & 0x2, 0x2, "a pre-release is marked so");
    let hex: String = info.iter().map(|b| format!("{b:02X}")).collect();
    for written in ["VS_VERSION_INFO", "StringFileInfo", "040904B0", "FileVersion", "2.10.4-beta.1", "ProductName", "Shop", "shop.exe", "VarFileInfo", "Translation"] {
        assert!(hex.contains(&wide_hex(written)), "{written}");
    }
}
