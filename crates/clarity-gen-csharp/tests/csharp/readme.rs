//! The README.md this stack writes for a project (P3): what it needs, how to
//! build and run it without Clarity (clarity_gen_core::readme and its own sections).

use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::load_converted;
use clarity_gen_csharp::program as generate;

const SHOP: &str = "module shop

entity Customer
    name: text(80)

page Card: window
    title \"Customer card\"
    data customer: Customer
";

fn readme(settings: &Settings) -> String {
    let project = load_converted([("shop.clarity", SHOP)]);
    let files = generate(&project, settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.into_iter().find(|f| f.path == "README.md").expect("a README.md").text
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn a_csharp_project_says_what_it_needs_and_how_to_build_it() {
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Clarity;
    let text = readme(&settings);
    has(&text, "C# with Clarity.UI");
    has(&text, "- the .NET 10 SDK, from https://dotnet.microsoft.com/download\n");
    has(&text, "    dotnet build -c Release\n");
    has(&text, "    dotnet run\n");
    has(&text, "bin/Release/net10.0/Shop");
    // The runtime comes from nuget.org, or from its own projects while working on it.
    has(&text, &format!("Clarity's runtime comes as NuGet packages (Clarity.Ui {},", clarity_gen_csharp::runtime_version()));
    has(&text, "    dotnet build -p:ClarityRuntime=<Clarity>/runtimes/csharp\n");
    // A run by hand or in the IDE chooses its data without editing the program.
    has(&text, "Set the environment variable `CLARITY_DATA` to a folder to open it there instead, under its own file name.");
}
