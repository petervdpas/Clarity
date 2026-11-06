//! An application's splash screen in every stack (docs/APP-ICON-SPLASH-PLAN.md,
//! step 3), as the todo example has one: the splash page is a window the program carries, its name and
//! time are in the model Clarity.UI reads, and `splash.status = ...` in code
//! becomes the stack's own call into its runtime.

use std::path::{Path, PathBuf};

fn folder(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("splash").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
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

#[test]
fn every_stack_carries_the_splash_and_its_status() {
    let root = folder("stacks");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    for (stack, call) in [
        ("csharp desktop", "Clarity.Runtime.Splash.SetStatus("),
        ("go desktop", "clarity.SetSplashStatus("),
        ("rust desktop", "clarity_runtime::splash::set_status("),
        ("c desktop", "clarity_splash_set_status("),
        ("cpp desktop", "clarity::splash::set_status("),
    ] {
        let project_file = root.join("todo.clarity");
        let dir = root.join("todo");
        let _ = std::fs::remove_dir_all(&dir);
        copy(&source.join("todo"), &dir);
        let text = std::fs::read_to_string(source.join("todo.clarity")).unwrap();
        let stack_line = text.lines().find(|l| l.trim_start().starts_with("stack ")).unwrap().to_string();
        std::fs::write(&project_file, text.replace(&stack_line, &format!("    stack {stack}"))).unwrap();
        // The todo example has its splash: Welcome, 2.5 seconds, a status from `on start`.
        let project = clarity_project::load_modules(&dir).unwrap();
        let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
        assert!(errors.is_empty(), "{stack}: {errors:#?}");
        let settings = clarity_project::load(&project_file).settings.unwrap();
        let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{stack}: {e:?}"));
        // As an Avalonia splash Window says it in its own XAML: its size, no
        // system decorations, centred on the screen, fixed, on top, not in the taskbar.
        let welcome = files.iter().find(|f| f.path.to_lowercase().ends_with("windows/welcome.cxaml")).unwrap_or_else(|| panic!("{stack}: the splash page is a window the program carries"));
        assert!(welcome.text.contains(r#"<Window name="Welcome" columns="56" cell="10" style="splashBack" width="560" height="290" decorations="none" startup="centerScreen" resizable="false" topmost="true" taskbar="false" frame="none">"#), "{stack}: {}", welcome.text);
        // And as Avalonia's App starts it: the app names its splash.
        let app = files.iter().find(|f| f.path.to_lowercase().ends_with("apps/todo.cxaml")).unwrap_or_else(|| panic!("{stack}: no app markup"));
        assert!(app.text.contains(r#"<App name="Todo" title="@todoTitle" splash="Welcome" splashFor="2.5">"#), "{stack}: {}", app.text);
        let main = files.iter().find(|f| f.path.to_lowercase().ends_with("windows/main.cxaml")).unwrap();
        assert!(!main.text.contains("decorations="), "a window that declares nothing keeps the system's window");
        assert!(files.iter().any(|f| f.text.contains(call)), "{stack}: {call}");
    }
}

/// The app's version (docs/VERSION-PLAN.md): `app.version` in code is the
/// project's version in every stack, and a text that shows `{app.version}`
/// carries it in what the program reads its texts from.
#[test]
fn every_stack_carries_the_app_s_version() {
    let root = folder("versioned");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    for (stack, code) in [("csharp desktop", r#""9.8.7""#), ("go desktop", r#""9.8.7""#), ("rust desktop", r#"String::from("9.8.7")"#), ("c desktop", r#""9.8.7""#), ("cpp desktop", r#"std::string("9.8.7")"#)] {
        let project_file = root.join("todo.clarity");
        let dir = root.join("todo");
        let _ = std::fs::remove_dir_all(&dir);
        copy(&source.join("todo"), &dir);
        let text = std::fs::read_to_string(source.join("todo.clarity")).unwrap();
        let stack_line = text.lines().find(|l| l.trim_start().starts_with("stack ")).unwrap().to_string();
        let text: String = text.lines().filter(|l| !l.trim_start().starts_with("version ")).map(|l| format!("{l}\n")).collect();
        std::fs::write(&project_file, text.replace(&stack_line, &format!("    stack {stack}\n    version \"9.8.7\""))).unwrap();
        let module = std::fs::read_to_string(dir.join("todo.clarity")).unwrap();
        std::fs::write(dir.join("todo.clarity"), module.replace("        splash.status = @welcomeLoading\n", "        splash.status = @welcomeLoading\n        let shown = app.version\n")).unwrap();
        // The example's splash shows "Version {app.version}" (welcomeVersion).
        let mut project = clarity_project::load_modules(&dir).unwrap();
        let settings = clarity_project::load(&project_file).settings.unwrap();
        project.diagnostics.extend(clarity_project::target_problems(&settings, &project, "todo.clarity"));
        let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
        assert!(errors.is_empty(), "{stack}: {errors:#?}");
        let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{stack}: {e:?}"));
        assert!(files.iter().any(|f| f.text.contains(code)), "{stack}: app.version as {code}");
        assert!(files.iter().any(|f| f.text.replace('\\', "").contains("Version 9.8.7")), "{stack}: the text carries the version");
        assert!(!files.iter().any(|f| f.text.contains("{app.version}")), "{stack}: no slot left");
        // The package says it, as the stack's build keeps a version.
        let text_of = |path: &str| files.iter().find(|f| f.path == path || (path == ".csproj" && f.path.ends_with(".csproj") && !f.path.contains('/'))).map(|f| f.text.clone()).unwrap_or_default();
        match stack {
            "rust desktop" => assert!(text_of("Cargo.toml").contains("version = \"9.8.7\""), "{stack}"),
            "csharp desktop" => assert!(text_of(".csproj").contains("<Version>9.8.7</Version>"), "{stack}"),
            "c desktop" => assert!(text_of("CMakeLists.txt").contains(" VERSION 9.8.7 LANGUAGES C)"), "{stack}: {}", text_of("CMakeLists.txt")),
            "cpp desktop" => assert!(text_of("CMakeLists.txt").contains(" VERSION 9.8.7 LANGUAGES CXX)"), "{stack}"),
            _ => {}
        }
    }
}
