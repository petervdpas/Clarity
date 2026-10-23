//! A custom part is written in the project's own language (DESIGN 5.19):
//! a library line that does not fit the stack is a problem at the part.

use clarity_project::{parse, target_problems};

const PARTS: &str = "module game\n\npart Native\n    native \"parts/Native\"\n\npart Rusty\n    crate \"parts/Rusty\"\n\npart Gopher\n    go \"parts/gopher\"\n\npart Dotnet\n    nuget \"Newtonsoft.Json\" 13.0.1\n";

fn problems(stack: &str) -> Vec<String> {
    let settings = parse(&format!("project Game\n    stack {stack}\n")).unwrap();
    let project = clarity_spec::load_sources([("game.clarity", PARTS)]);
    target_problems(&settings, &project, "game.clarity").into_iter().map(|d| d.message).collect()
}

fn misfits(stack: &str) -> Vec<String> {
    let mut found: Vec<String> = problems(stack).iter().filter_map(|m| m.strip_prefix("part ").and_then(|m| m.split(' ').next()).map(String::from)).collect();
    found.sort();
    found
}

#[test]
fn each_stack_takes_the_parts_of_its_language() {
    assert_eq!(misfits("cpp web desktop"), ["Dotnet", "Gopher", "Rusty"]);
    assert_eq!(misfits("c web desktop"), ["Dotnet", "Gopher", "Rusty"]);
    assert_eq!(misfits("rust web desktop"), ["Dotnet", "Gopher", "Native"]);
    assert_eq!(misfits("go web desktop"), ["Dotnet", "Native", "Rusty"]);
    assert_eq!(misfits("csharp avalonia desktop"), ["Gopher", "Native", "Rusty"]);
    assert!(problems("rust web desktop").iter().any(|m| m == "part Native is C or C++ (native), but the project is written in Rust: write it as a crate, crate \"folder\""), "{:?}", problems("rust web desktop"));
}

#[test]
fn each_stack_takes_the_libraries_of_its_language() {
    let text = "module game\n\nlibrary A\n    cmake \"SFML\"\n\nlibrary B\n    crate \"humanize\" \"0.1\"\n\nlibrary C\n    go \"github.com/x/y\" v1.0.0\n\nlibrary D\n    nuget \"P\" 1.0.0\n";
    let misfits = |stack: &str| {
        let settings = parse(&format!("project Game\n    stack {stack}\n")).unwrap();
        let project = clarity_spec::load_sources([("game.clarity", text)]);
        let mut found: Vec<String> = target_problems(&settings, &project, "game.clarity").into_iter().filter_map(|d| d.message.strip_prefix("library ").and_then(|m| m.split(' ').next()).map(String::from)).collect();
        found.sort();
        found
    };
    assert_eq!(misfits("cpp web desktop"), ["B", "C", "D"]);
    assert_eq!(misfits("rust web desktop"), ["A", "C", "D"]);
    assert_eq!(misfits("go web desktop"), ["A", "B", "D"]);
    assert_eq!(misfits("csharp avalonia desktop"), ["A", "B", "C"]);
}
