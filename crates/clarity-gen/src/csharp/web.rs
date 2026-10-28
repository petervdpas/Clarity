//! The web target for C# with Avalonia (DESIGN 7.6): the windows live on a
//! server, in a session per browser tab, where their handlers run against
//! the server's database; the browser shows a mirror of each. The output has
//! three projects: the application as a library (the windows, views and
//! modules the desktop program has, without its `Program`), the server that
//! keeps the sessions and serves the browser program, and the browser
//! program (Avalonia.Browser), which the server's build publishes into it.

use clarity_project::Settings;
use clarity_spec::Project;
use clarity_spec::layout::MESSAGE_MS;

use super::names::string;
use super::{HEADER, SERVERS, Server, check_engines, main_window, opening, project_name};
use crate::OutFile;

/// The runtime's Avalonia project, which names the Avalonia it is made with.
const AVALONIA_PROJECT: &str = include_str!("../../../../runtimes/csharp/Clarity.Avalonia/Clarity.Avalonia.csproj");

/// The Avalonia the runtime is made with: the browser program takes the same.
fn avalonia_version() -> &'static str {
    let at = AVALONIA_PROJECT.find("Include=\"Avalonia\" Version=\"").expect("Clarity.Avalonia references Avalonia") + "Include=\"Avalonia\" Version=\"".len();
    &AVALONIA_PROJECT[at..at + AVALONIA_PROJECT[at..].find('"').expect("the version is closed")]
}

fn text(path: &str, text: String) -> OutFile {
    OutFile { path: path.to_string(), text, bytes: None }
}

/// The three projects, from the files the desktop program has.
pub(super) fn generate(project: &Project, settings: &Settings, desktop: Vec<OutFile>) -> Result<Vec<OutFile>, Vec<String>> {
    let name = project_name(settings);
    let project_file = format!("{name}.csproj");
    let (databases, _) = crate::databases(project, settings)?;
    let databases = crate::with_secret_locations(databases, &crate::settings_program(settings));
    let servers: Vec<&Server> = SERVERS.iter().filter(|s| databases.iter().any(|(_, sql, _)| *sql == s.sql)).collect();
    let main = main_window(project).ok_or_else(|| vec!["the project has no window to start with".to_string()])?;
    let engines = !crate::engines(project).is_empty();
    let mut files = Vec::new();
    for file in desktop {
        match file.path.as_str() {
            // The server and the browser program start it instead.
            "Program.cs" | "README.md" => {}
            "App.axaml.cs" => files.push(text("App/App.axaml.cs", app_cs(project, &name))),
            path if path == project_file => files.push(text(&format!("App/{path}"), file.text.replace("<OutputType>WinExe</OutputType>", "<OutputType>Library</OutputType>"))),
            path => files.push(OutFile { path: format!("App/{path}"), ..file }),
        }
    }
    files.push(text("App/Databases.cs", databases_cs(&name, &opening(&databases, &servers, settings, "Open"))));
    files.push(text(&format!("Server/{name}.Server.csproj"), server_csproj(&name)));
    files.push(text("Server/Program.cs", server_cs(&name, &main, &clarity_project::serve_address(settings), engines)));
    files.push(text(&format!("Browser/{name}.Browser.csproj"), browser_csproj(&name)));
    files.push(text("Browser/Program.cs", browser_cs(&name)));
    files.push(text("Browser/wwwroot/index.html", index_html(&clarity_project::title(settings))));
    files.push(text("Browser/wwwroot/main.js", MAIN_JS.to_string()));
    files.push(text("README.md", readme(&name, &clarity_project::serve_address(settings))));
    Ok(files)
}

fn databases_cs(name: &str, opening: &str) -> String {
    format!("{HEADER}\nusing Clarity.Runtime;\n\nnamespace {name};\n\n/// <summary>The application's databases, which the server opens.</summary>\npublic static class Databases\n{{\n{opening}}}\n")
}

/// The application in the browser: in the person's language and theme, a
/// mirror shell over the sessions of the server the page came from.
fn app_cs(project: &Project, name: &str) -> String {
    let currency = project.locale.currency.as_deref().map_or_else(|| "null".to_string(), string);
    format!(
        "{HEADER}
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using Clarity.Avalonia;

namespace {name};

public partial class App : Application
{{
    /// <summary>The sessions the windows are mirrors of: those of the server the page came from (DESIGN 7.6).</summary>
    public static ISessionLink? Sessions {{ get; set; }}

    public override void Initialize() => AvaloniaXamlLoader.Load(this);

    public override async void OnFrameworkInitializationCompleted()
    {{
        base.OnFrameworkInitializationCompleted();
        if (ApplicationLifetime is ISingleViewApplicationLifetime single && Sessions is {{ }} link)
        {{
            Locale.Start(Texts.BaseLanguage, Texts.Languages, {currency}, Texts.In, Labels.Named);
            Themes.Start(AppThemes.Offered);
            var holder = new ContentControl();
            single.MainView = holder;
            holder.Content = await MirrorShell.Start(link, Views.Create, TimeSpan.FromMilliseconds({MESSAGE_MS}));
        }}
    }}
}}
"
    )
}

/// The runtime's projects or packages, as the application's project takes them.
const CLARITY_RUNTIME: &str = "    <!-- Where Clarity's runtime projects are, to build on them instead of its
         packages: dotnet build -p:ClarityRuntime=<folder>, or the environment
         variable CLARITY_RUNTIME. Empty: the packages. -->
    <ClarityRuntime Condition=\"'$(ClarityRuntime)' == ''\">$(CLARITY_RUNTIME)</ClarityRuntime>\n";

/// The server: it publishes the browser program into its folder as it builds
/// (`-p:SkipBrowser=true` leaves that out), and serves it from there.
fn server_csproj(name: &str) -> String {
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
  <!-- The browser program, published beside the server, which serves it. -->
  <Target Name=\"PublishBrowser\" BeforeTargets=\"Build\" Condition=\"'$(SkipBrowser)' != 'true'\">
    <Exec Command=\"dotnet publish ../Browser/{name}.Browser.csproj -c $(Configuration) -o $(MSBuildProjectDirectory)/browser -p:ClarityRuntime=$(ClarityRuntime) --nologo\" />
  </Target>
  <Target Name=\"CopyBrowser\" AfterTargets=\"Build\" Condition=\"'$(SkipBrowser)' != 'true'\">
    <ItemGroup>
      <BrowserFiles Include=\"browser/wwwroot/**\" />
    </ItemGroup>
    <Copy SourceFiles=\"@(BrowserFiles)\" DestinationFiles=\"@(BrowserFiles->'$(OutDir)wwwroot/%(RecursiveDir)%(Filename)%(Extension)')\" SkipUnchangedFiles=\"true\" />
  </Target>
</Project>
",
        framework = super::TARGET_FRAMEWORK
    )
}

fn server_cs(name: &str, main: &str, address: &str, engines: bool) -> String {
    let check_engines = check_engines(engines);
    format!(
        "{HEADER}
using Clarity.Avalonia;
using Clarity.Runtime;

namespace {name}.Server;

/// <summary>
/// The application as a web server (DESIGN 7.6): each browser tab has a
/// session of its windows here, where their handlers run against the
/// databases, and the browser shows them.
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
        new WebServer(new Sessions(() => new {main}ViewModel(db)), files).Serve({program}, WebServer.Address(args, {address}));
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
    <!-- Not trimmed: the windows' state crosses by reflection, and themes load by name. -->
    <PublishTrimmed>false</PublishTrimmed>
    <RootNamespace>{name}.Browser</RootNamespace>
    <AssemblyName>{name}.Browser</AssemblyName>
{CLARITY_RUNTIME}  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include=\"Avalonia.Browser\" Version=\"{avalonia}\" />
    <ProjectReference Include=\"../App/{name}.csproj\" />
  </ItemGroup>
</Project>
",
        framework = super::TARGET_FRAMEWORK,
        avalonia = avalonia_version()
    )
}

fn browser_cs(name: &str) -> String {
    format!(
        "{HEADER}
using System.Runtime.InteropServices.JavaScript;
using System.Runtime.Versioning;
using Avalonia;
using Avalonia.Browser;
using Clarity.Avalonia;

[assembly: SupportedOSPlatform(\"browser\")]

namespace {name}.Browser;

/// <summary>The application in the browser: mirrors of the windows the server keeps (DESIGN 7.6).</summary>
internal sealed partial class Program
{{
    private static System.Threading.Tasks.Task Main(string[] args)
    {{
        // The sessions are the server's the page came from.
        var origin = JSHost.GlobalThis.GetPropertyAsJSObject(\"location\")!.GetPropertyAsString(\"origin\")!;
        App.Sessions = new HttpSessionLink(new HttpClient {{ BaseAddress = new Uri(origin) }});
        return AppBuilder.Configure<App>().StartBrowserAppAsync(\"out\");
    }}
}}
"
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
  <script type=\"module\" src=\"./main.js\"></script>
</head>
<body style=\"margin: 0; height: 100vh; overflow: hidden\">
  <div id=\"out\" style=\"height: 100%\"></div>
</body>
</html>
"
    )
}

/// Starts .NET in the page, which starts the application.
const MAIN_JS: &str = "// Generated by Clarity. Do not edit: change the project in Clarity instead.
import { dotnet } from './_framework/dotnet.js';

const runtime = await dotnet.withApplicationArguments().create();
await runtime.runMain();
";

fn readme(name: &str, address: &str) -> String {
    format!(
        "# {name}: the web application

Generated by Clarity. Do not edit these files; change the project in Clarity and generate it again.

The windows live on the server, in a session for each browser tab, where their handlers run against its databases; the browser shows them, made with Avalonia for the browser.

- `App`: the application's windows, views and modules.
- `Server`: the server, which keeps the sessions and serves the browser program.
- `Browser`: the browser program.

## Build and run

Building the browser program needs .NET's WebAssembly tools, once on each machine:

    dotnet workload install wasm-tools

Then build the server, which publishes the browser program into it, and start it:

    dotnet build Server
    dotnet Server/bin/Debug/net10.0/{name}.Server.dll

Open http://{address} in a browser. `--address host:port`, or the environment variable `CLARITY_ADDRESS`, makes it listen elsewhere.
"
    )
}
