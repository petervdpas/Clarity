//! The web target of C# on Clarity.UI (DESIGN 7.6, 7.9): the windows' view
//! models live on a server, in a session per browser tab, where their
//! handlers run against the server's databases; the browser program is
//! Clarity.UI compiled to WebAssembly, which draws each window from its
//! markup over a mirror of the server's state and sends what the person does
//! as commands. The output has three projects: the application as a library
//! (the view models, windows' markup, modules, texts and themes the desktop
//! program has, without its `Program`), the server that keeps the sessions
//! and serves the browser program and the windows' markup, and the browser
//! program, which the server's build publishes into it.

use clarity_project::Settings;
use clarity_spec::Project;

use super::names::string;
use super::{DRIVERS, Driver, HEADER, check_engines, main_window, opening, project_name};
use crate::OutFile;

/// The package the application's library is made on: Clarity.UI without a
/// desktop of its own, which both the server and the browser program take.
pub(super) const APP_PACKAGE: &str = "Clarity.Ui";

/// The package of the server: the sessions over HTTP, and the browser program's files.
const SERVER_PACKAGE: &str = "Clarity.Ui.Server";

/// The package of the browser program: Clarity.UI on the page's canvas, and the mirror of the server's windows.
const BROWSER_PACKAGE: &str = "Clarity.Ui.Browser";

fn text(path: &str, text: String) -> OutFile {
    OutFile { path: path.to_string(), text, bytes: None }
}

/// The three projects, from the files the desktop program has (made on `APP_PACKAGE`).
pub(super) fn generate(project: &Project, settings: &Settings, desktop: Vec<OutFile>) -> Result<Vec<OutFile>, Vec<String>> {
    let name = project_name(settings);
    let project_file = format!("{name}.csproj");
    let (databases, _) = crate::databases(project, settings)?;
    let databases = crate::with_secret_locations(databases, &crate::settings_program(settings));
    let drivers: Vec<&Driver> = DRIVERS.iter().filter(|s| databases.iter().any(|(_, sql, _)| *sql == s.sql)).collect();
    let main = main_window(project).ok_or_else(|| vec!["the project has no window to start with".to_string()])?;
    let engines = !crate::engines(project).is_empty();
    let kept = project.entities().find(|e| e.settings).map(|e| e.name.name.clone());
    let mut files = Vec::new();
    for file in desktop {
        match file.path.as_str() {
            // The server and the browser program start it instead.
            "Program.cs" | "README.md" => {}
            path if path == project_file => files.push(text(&format!("App/{path}"), library_csproj(&file.text))),
            path => files.push(OutFile { path: format!("App/{path}"), ..file }),
        }
    }
    // The databases open on the server, which alone carries their drivers.
    files.push(text("Server/Databases.cs", databases_cs(&name, &opening(project, &databases, &drivers, settings, "Open"))));
    files.push(text(&format!("Server/{name}.Server.csproj"), server_csproj(&name, &drivers)));
    files.push(text("Server/Program.cs", server_cs(&name, &main, &clarity_project::serve_address(settings), engines, kept.as_deref())));
    files.push(text(&format!("Browser/{name}.Browser.csproj"), browser_csproj(&name)));
    files.push(text("Browser/Directory.Build.props", browser_props(&name)));
    files.push(text("Browser/Program.cs", browser_cs(project, &name, kept.as_deref())));
    files.push(text("Browser/wwwroot/index.html", index_html(&clarity_project::title(settings))));
    files.push(text("Browser/wwwroot/main.js", MAIN_JS.to_string()));
    files.push(text("README.md", readme(&name, &clarity_project::serve_address(settings))));
    Ok(files)
}

/// The application's project as a library that neither opens a window nor
/// a database: the drivers are the server's, so the browser program carries
/// no native database.
fn library_csproj(desktop: &str) -> String {
    desktop
        .replace("<OutputType>WinExe</OutputType>", "<OutputType>Library</OutputType>")
        .lines()
        .filter(|line| !DRIVERS.iter().any(|d| line.contains(&format!("\"{}\"", d.package)) || line.contains(&format!("/{}.csproj\"", d.package))))
        .map(|line| format!("{line}\n"))
        .collect()
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

/// The server: it publishes the browser program into its folder as it builds
/// (`-p:SkipBrowser=true` leaves that out), and serves it from there.
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
  <!-- The sessions and the files, and the drivers of the databases, which the server opens. -->
{references}  <!-- The browser program, published beside the server, which serves it: again
       only when a file of the application or the browser program changed. -->
  <ItemGroup>
    <BrowserInputs Include=\"../App/**;../Browser/**\" Exclude=\"../App/bin/**;../App/obj/**;../Browser/bin/**;../Browser/obj/**\" />
  </ItemGroup>
  <!-- Built on Clarity's runtime projects instead of its packages, their sources count too. -->
  <ItemGroup Condition=\"'$(ClarityRuntime)' != ''\">
    <BrowserInputs Include=\"$(ClarityRuntime)/**/*.cs;$(ClarityRuntime)/**/*.js;$(ClarityRuntime)/**/*.csproj\" Exclude=\"$(ClarityRuntime)/**/bin/**;$(ClarityRuntime)/**/obj/**\" />
  </ItemGroup>
  <Target Name=\"PublishBrowser\" BeforeTargets=\"Build\" Condition=\"'$(SkipBrowser)' != 'true'\" Inputs=\"@(BrowserInputs)\" Outputs=\"browser/$(Configuration)/published\">
    <!-- Its own build leaves no compiler server or build nodes running after it. -->
    <Exec Command=\"dotnet publish &quot;../Browser/{name}.Browser.csproj&quot; -c $(Configuration) -o &quot;$(MSBuildProjectDirectory)/browser/$(Configuration)&quot; &quot;-p:ClarityRuntime=$(ClarityRuntime)&quot; -p:UseSharedCompilation=false -nodeReuse:false --nologo\" EnvironmentVariables=\"MSBUILDDISABLENODEREUSE=1\" />
    <Touch Files=\"browser/$(Configuration)/published\" AlwaysCreate=\"true\" />
  </Target>
  <Target Name=\"CopyBrowser\" AfterTargets=\"Build\" Condition=\"'$(SkipBrowser)' != 'true'\">
    <ItemGroup>
      <BrowserFiles Include=\"browser/$(Configuration)/wwwroot/**\" />
    </ItemGroup>
    <Copy SourceFiles=\"@(BrowserFiles)\" DestinationFiles=\"@(BrowserFiles->'$(OutDir)wwwroot/%(RecursiveDir)%(Filename)%(Extension)')\" SkipUnchangedFiles=\"true\" />
  </Target>
</Project>
",
        framework = super::TARGET_FRAMEWORK,
        references = runtime_references(&packages),
    )
}

/// The person's settings as the sessions and the mirror take them: read, and applied.
fn settings_hooks(kept: &str) -> String {
    format!("(() => {kept}.Current.ToJson(), sent => {kept}.Current.Apply(sent))")
}

fn server_cs(name: &str, main: &str, address: &str, engines: bool, kept: Option<&str>) -> String {
    let check_engines = check_engines(engines);
    // Each request runs with the settings of the person who sent it (DESIGN 7.6).
    let settings = kept.map(|s| format!(" {{ Settings = {} }}", settings_hooks(s))).unwrap_or_default();
    format!(
        "{HEADER}
using Clarity.Runtime;
using Clarity.Ui.Server;

namespace {name}.Server;

/// <summary>
/// The application as a web server (DESIGN 7.6): each browser tab has a
/// session of its windows here, where their handlers run against the
/// databases; the browser program draws them from the windows' markup,
/// which the server serves from the application.
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
        var files = Path.Combine(AppContext.BaseDirectory, \"wwwroot\");
        var sessions = new Sessions(() => new {main}ViewModel(db)){settings};
        new WebServer(sessions, files, typeof({main}ViewModel).Assembly).Serve({program}, WebServer.Address(args, {address}));
        return 0;
    }}
}}
",
        ready = string(&format!("{name}: the databases are ready.")),
        program = string(name),
        address = string(address),
    )
}

fn browser_csproj(name: &str) -> String {
    format!(
        "<!-- Generated by Clarity. Do not edit: change the project in Clarity instead. -->
<Project Sdk=\"Microsoft.NET.Sdk.WebAssembly\">
  <PropertyGroup>
    <TargetFramework>{framework}-browser</TargetFramework>
    <OutputType>Exe</OutputType>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
    <!-- Trimmed in Release, which makes the first load a few times smaller; a
         Debug build is not, so it publishes faster. The windows' state crosses
         as JSON by reflection, so that stays on, and so do the assemblies
         whose types it reaches (below). -->
    <PublishTrimmed Condition=\"'$(Configuration)' == 'Release'\">true</PublishTrimmed>
    <PublishTrimmed Condition=\"'$(Configuration)' != 'Release'\">false</PublishTrimmed>
    <JsonSerializerIsReflectionEnabledByDefault>true</JsonSerializerIsReflectionEnabledByDefault>
    <RootNamespace>{name}.Browser</RootNamespace>
    <AssemblyName>{name}.Browser</AssemblyName>
{CLARITY_RUNTIME}  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include=\"../App/{name}.csproj\" />
  </ItemGroup>
{references}  <!-- Kept whole when trimmed: the windows, whose state the snapshot reads and
       sets by name, and Clarity's runtime and UI, which do it and find the
       parts by their kind. What the trimmer warns of is code reached by name,
       and what that code reaches is in these assemblies, kept whole. -->
  <PropertyGroup>
    <SuppressTrimAnalysisWarnings>true</SuppressTrimAnalysisWarnings>
  </PropertyGroup>
  <ItemGroup>
    <TrimmerRootAssembly Include=\"{name}\" />
    <TrimmerRootAssembly Include=\"Clarity.Ui\" />
    <TrimmerRootAssembly Include=\"Clarity.Runtime\" />
  </ItemGroup>
</Project>
",
        framework = super::TARGET_FRAMEWORK,
        references = runtime_references(&[BROWSER_PACKAGE]),
    )
}

/// Where the browser program's build files go. On Windows a folder of their
/// own under `%LOCALAPPDATA%`, one per place the project is in: the
/// WebAssembly steps nest deep (`bin/Release/net10.0-browser/wwwroot/_framework/`),
/// past Windows' 260 characters when the project's own folder is deep too.
fn browser_props(name: &str) -> String {
    format!(
        "<!-- Generated by Clarity. Do not edit: change the project in Clarity instead. -->
<Project>
  <!-- A Directory.Build.props further up still counts. -->
  <Import Project=\"$([MSBuild]::GetPathOfFileAbove('Directory.Build.props', '$(MSBuildThisFileDirectory)../'))\" Condition=\"'$([MSBuild]::GetPathOfFileAbove(&quot;Directory.Build.props&quot;, &quot;$(MSBuildThisFileDirectory)../&quot;))' != ''\" />
  <!-- On Windows the build files go to a short folder of their own: the
       WebAssembly steps nest deep, past Windows' 260 characters when this
       folder is deep too. The number tells apart copies of the project. -->
  <PropertyGroup Condition=\"'$(OS)' == 'Windows_NT' And '$(LOCALAPPDATA)' != ''\">
    <ClarityBuild>$(LOCALAPPDATA)\\Clarity\\build\\{name}-$([MSBuild]::StableStringHash($(MSBuildThisFileDirectory)))\\</ClarityBuild>
    <BaseIntermediateOutputPath>$(ClarityBuild)obj\\</BaseIntermediateOutputPath>
    <BaseOutputPath>$(ClarityBuild)bin\\</BaseOutputPath>
  </PropertyGroup>
</Project>
"
    )
}

/// The browser program: what the browser keeps first (the language, the
/// theme, the person's settings), then the windows of the server the page
/// came from, drawn on the page's canvas.
fn browser_cs(project: &Project, name: &str, kept: Option<&str>) -> String {
    let currency = project.locale.currency.as_deref().map_or_else(|| "null".to_string(), string);
    let (start_settings, settings) = match kept {
        Some(s) => (format!("        {s}.Start();\n"), format!(", settings: {}", settings_hooks(s))),
        None => (String::new(), String::new()),
    };
    format!(
        "{HEADER}
using System.Runtime.Versioning;
using Clarity.Runtime;
using Clarity.Ui.Browser;

[assembly: SupportedOSPlatform(\"browser\")]

namespace {name}.Browser;

/// <summary>The application in the browser: the windows the server keeps, drawn by Clarity.UI on the page (DESIGN 7.6, 7.9).</summary>
internal static class Program
{{
    private static async Task Main()
    {{
        // The page's storage, which holds the language, theme and settings the person chose.
        await BrowserCanvas.Ready();
        Remembered.Use({application});
        Locale.Start(Texts.BaseLanguage, Texts.Languages, {currency}, Texts.In, Labels.Named);
        AppThemes.Start();
{start_settings}        // The windows are the server's the page came from: their state, their markup and their handlers.
        await BrowserCanvas.Serve(typeof(Texts).Assembly{settings});
    }}
}}
",
        application = string(name),
    )
}

fn index_html(title: &str) -> String {
    let title = title.replace('&', "&amp;").replace('<', "&lt;");
    format!(
        "<!DOCTYPE html>
<!-- Generated by Clarity. Do not edit: change the project in Clarity instead. -->
<html>
<head>
  <meta charset=\"utf-8\">
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
  <title>{title}</title>
  <style>html, body {{ margin: 0; height: 100%; overflow: hidden; }}</style>
</head>
<body>
  <canvas id=\"clarity\" style=\"position: fixed; inset: 0; width: 100vw; height: 100vh\"></canvas>
  <script type=\"module\" src=\"./main.js\"></script>
</body>
</html>
"
    )
}

/// Starts .NET in the page, which starts the application.
const MAIN_JS: &str = "// Generated by Clarity. Do not edit: change the project in Clarity instead.
import { dotnet } from './_framework/dotnet.js';

await dotnet.run();
";

fn readme(name: &str, address: &str) -> String {
    format!(
        "# {name}: the web application

Generated by Clarity. Do not edit these files; change the project in Clarity and generate it again.

The windows live on the server, in a session for each browser tab, where their handlers run against its databases. The browser program is Clarity.UI compiled to WebAssembly: it draws each window on the page's canvas from the markup the server sends, and sends what the person does to the server.

- `App`: the application's windows, their markup, modules, texts and themes.
- `Server`: the server, which keeps the sessions and serves the browser program and the windows' markup.
- `Browser`: the browser program.

## Build and run

Building the browser program needs .NET's WebAssembly tools, once on each machine:

    dotnet workload install wasm-tools

Then build the server, which publishes the browser program into it, and start it:

    dotnet build Server
    dotnet Server/bin/Debug/net10.0/{name}.Server.dll

Open http://{address} in a browser. `--address host:port`, or the environment variable `CLARITY_ADDRESS`, makes it listen elsewhere.

To put it online, build it in Release: the browser program is trimmed then, and the server sends its files compressed:

    dotnet build Server -c Release
    dotnet Server/bin/Release/net10.0/{name}.Server.dll

On Windows the browser program's build files go to `%LOCALAPPDATA%\\Clarity\\build\\{name}-<number>`, not to `Browser\\bin` and `Browser\\obj`: its WebAssembly steps nest deep, past Windows' 260 characters when this folder is deep too (see `Browser/Directory.Build.props`).

Clarity's runtime comes as NuGet packages ({BROWSER_PACKAGE}, {SERVER_PACKAGE} and the driver of each kind of database, version {version}), which the first build fetches from nuget.org. To build on the runtime's own projects instead, while working on them, name their folder in Clarity's repository:

    dotnet build Server -p:ClarityRuntime=<Clarity>/runtimes/csharp

or set the environment variable `CLARITY_RUNTIME` to it.
",
        version = super::runtime_version()
    )
}
