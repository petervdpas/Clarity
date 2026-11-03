//! The web target of C# on Clarity.UI (DESIGN 7.6, 7.9): the windows' view
//! models live on a server, where their handlers run against the server's
//! databases; in the browser runs Clarity.UI's library, compiled once for
//! every language (the browser files), which draws each window and asks the
//! program on the server, as the native library asks it on the desktop. The
//! output has the application as a library (the view models, windows'
//! markup, model, modules, texts and themes the desktop program has, without
//! its `Program`, and the browser files it carries for the server to serve),
//! the server, and `browser/` with the page, beside which Clarity puts the
//! library's browser files (`web_browser::with_library`).

use clarity_project::Settings;
use clarity_spec::Project;

use super::names::string;
use super::{DRIVERS, Driver, HEADER, check_engines, main_window, opening, project_name};
use crate::OutFile;

/// The package the application's library is made on: the runtime, whose
/// ProgramWire answers Clarity.UI. It draws nothing on the server, so it
/// takes neither Clarity.Ui nor its native library.
pub(super) const APP_PACKAGE: &str = "Clarity.Runtime";

/// The package of the server: the browser files, the program's files and Clarity.UI's commands over HTTP.
const SERVER_PACKAGE: &str = "Clarity.Ui.Server";

fn text(path: &str, text: String) -> OutFile {
    OutFile { path: path.to_string(), text, bytes: None }
}

/// The application and the server, from the files the desktop program has (made on `APP_PACKAGE`), and the page.
pub(super) fn generate(project: &Project, settings: &Settings, desktop: Vec<OutFile>) -> Result<Vec<OutFile>, Vec<String>> {
    let name = project_name(settings);
    let project_file = format!("{name}.csproj");
    let (databases, _) = crate::databases(project, settings)?;
    let databases = crate::with_secret_locations(databases, &crate::settings_program(settings));
    let drivers: Vec<&Driver> = DRIVERS.iter().filter(|s| databases.iter().any(|(_, sql, _)| *sql == s.sql)).collect();
    let main = main_window(project).ok_or_else(|| vec!["the project has no window to start with".to_string()])?;
    let engines = !crate::engines(project).is_empty();
    let mut files = Vec::new();
    for file in desktop {
        match file.path.as_str() {
            // The server starts it instead.
            "Program.cs" | "README.md" => {}
            path if path == project_file => files.push(text(&format!("App/{path}"), library_csproj(&file.text))),
            path => files.push(OutFile { path: format!("App/{path}"), ..file }),
        }
    }
    // The databases open on the server, which alone carries their drivers.
    files.push(text("Server/Databases.cs", databases_cs(&name, &opening(project, &databases, &drivers, settings, "Open"))));
    files.push(text(&format!("Server/{name}.Server.csproj"), server_csproj(&name, &drivers)));
    files.push(text("Server/Program.cs", server_cs(&name, &main, &clarity_project::serve_address(settings), engines, &project.locale.currency.as_deref().map_or_else(|| "null".to_string(), string))));
    // The page; Clarity puts the library's browser files beside it, and the application carries them all.
    files.push(text(crate::web_browser::PAGE, crate::web_browser::page(&clarity_project::title(settings))));
    files.push(text("README.md", readme(&name, &clarity_project::serve_address(settings))));
    Ok(files)
}

/// The application's project as a library that neither opens a window nor
/// a database (the drivers are the server's), carrying the browser files of
/// `browser/` (the page and Clarity.UI's library) under `browser/` for the server.
fn library_csproj(desktop: &str) -> String {
    let library: String = desktop
        .replace("<OutputType>WinExe</OutputType>", "<OutputType>Library</OutputType>")
        .lines()
        .filter(|line| !DRIVERS.iter().any(|d| line.contains(&format!("\"{}\"", d.package)) || line.contains(&format!("/{}.csproj\"", d.package))))
        .map(|line| format!("{line}\n"))
        .collect();
    library.replacen(
        "</Project>",
        "  <!-- The browser files the server serves: the page and Clarity.UI's library, which Clarity puts in ../browser. -->\n  <ItemGroup>\n    <EmbeddedResource Include=\"../browser/**\" LogicalName=\"browser/%(RecursiveDir)%(Filename)%(Extension)\" />\n  </ItemGroup>\n</Project>",
        1,
    )
}

fn databases_cs(name: &str, opening: &str) -> String {
    format!("{HEADER}\nusing Clarity.Runtime;\n\nnamespace {name};\n\n/// <summary>The application's databases, which the server opens.</summary>\npublic static class Databases\n{{\n{opening}}}\n")
}

/// The runtime's projects or packages, as each project takes them.
const CLARITY_RUNTIME: &str = "    <!-- Where Clarity's runtime projects are, to build on them instead of its
         packages: dotnet build -p:ClarityRuntime=<folder>, or the environment
         variable CLARITY_RUNTIME. Empty: the packages. -->
    <ClarityRuntime Condition=\"'$(ClarityRuntime)' == ''\">$(CLARITY_RUNTIME)</ClarityRuntime>\n";

/// A package of the runtime, or its project in the runtime's folder.
fn runtime_references(packages: &[&str]) -> String {
    let version = super::runtime_version();
    let references: String = packages.iter().map(|p| format!("    <PackageReference Include=\"{p}\" Version=\"{version}\" />\n")).collect();
    let projects: String = packages.iter().map(|p| format!("    <ProjectReference Include=\"$(ClarityRuntime)/{p}/{p}.csproj\" />\n")).collect();
    format!("  <ItemGroup Condition=\"'$(ClarityRuntime)' == ''\">\n{references}  </ItemGroup>\n  <ItemGroup Condition=\"'$(ClarityRuntime)' != ''\">\n{projects}  </ItemGroup>\n")
}

/// The server, on the application and the drivers of its databases.
fn server_csproj(name: &str, drivers: &[&Driver]) -> String {
    let packages: Vec<&str> = std::iter::once(SERVER_PACKAGE).chain(drivers.iter().map(|d| d.package)).collect();
    format!(
        "<!-- Generated by Clarity. Do not edit: change the project in Clarity instead. -->
<Project Sdk=\"Microsoft.NET.Sdk\">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>{framework}</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <RootNamespace>{name}.Server</RootNamespace>
    <AssemblyName>{name}.Server</AssemblyName>
{CLARITY_RUNTIME}  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include=\"../App/{name}.csproj\" />
  </ItemGroup>
  <!-- The server, and the drivers of the databases, which it opens. -->
{references}</Project>
",
        framework = super::TARGET_FRAMEWORK,
        references = runtime_references(&packages),
    )
}

fn server_cs(name: &str, main: &str, address: &str, engines: bool, currency: &str) -> String {
    let check_engines = check_engines(engines);
    format!(
        "{HEADER}
using Clarity.Runtime;
using Clarity.Ui.Server;

namespace {name}.Server;

/// <summary>
/// The application as a web server (DESIGN 7.6, 7.9): Clarity.UI runs in the
/// browser and asks the windows here, where their handlers run against the
/// databases. The server serves the browser files and the program's files,
/// which the application carries.
/// </summary>
public static class Program
{{
    public static int Main(string[] args)
    {{
{check_engines}        // `--check` opens and migrates the databases, for builds and scripts.
        if (args.Contains(\"--check\"))
        {{
            using var check = Databases.Open();
            Console.WriteLine({ready});
            return 0;
        }}
        using var db = Databases.Open();
        Locale.Start(Texts.BaseLanguage, Texts.Languages, {currency}, Texts.In, Labels.Named);
        AppThemes.Start();
        new WebServer(new {main}ViewModel(db)).Serve({program}, WebServer.Address(args, {address}));
        return 0;
    }}
}}
",
        ready = string(&format!("{name}: the databases are ready.")),
        program = string(name),
        address = string(address),
    )
}

fn readme(name: &str, address: &str) -> String {
    format!(
        "# {name}: the web application

Generated by Clarity. Do not edit these files; change the project in Clarity and generate it again.

The windows live on the server, where their handlers run against its databases. In the browser runs Clarity.UI, compiled to WebAssembly once for every language: it draws each window on the page's canvas from the markup the server serves, and asks the server for everything a window does.

- `App`: the application's windows, their markup, model, modules, texts and themes, and the browser files it carries for the server.
- `Server`: the server, which serves the page, the browser files and the program's files, and answers what the browser asks.
- `web`: the page and Clarity.UI's browser files (`clarity-ui.js`, `clarity-ui.wasm`, `clarity-ui-worker.js`, `clarity-ui-page.js`), which Clarity puts here when it writes the project. The application embeds them.

## Build and run

    dotnet build Server
    dotnet Server/bin/Debug/net10.0/{name}.Server.dll

Open http://{address} in a browser. `--address host:port`, or the environment variable `CLARITY_ADDRESS`, makes it listen elsewhere. The server sends the browser files compressed to a browser that takes them.

Clarity's runtime comes as NuGet packages ({SERVER_PACKAGE} and the driver of each kind of database, version {version}), which the first build fetches from nuget.org. To build on the runtime's own projects instead, while working on them, name their folder in Clarity's repository:

    dotnet build Server -p:ClarityRuntime=<Clarity>/runtimes/csharp

or set the environment variable `CLARITY_RUNTIME` to it.
",
        version = super::runtime_version()
    )
}
