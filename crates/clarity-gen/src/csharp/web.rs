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
            "App.axaml.cs" => files.push(text("App/App.axaml.cs", app_cs(project, &name, project.entities().find(|e| e.settings).map(|e| e.name.name.as_str())))),
            path if path == project_file => files.push(text(&format!("App/{path}"), file.text.replace("<OutputType>WinExe</OutputType>", "<OutputType>Library</OutputType>"))),
            path => files.push(OutFile { path: format!("App/{path}"), ..file }),
        }
    }
    files.push(text("App/Databases.cs", databases_cs(&name, &opening(&databases, &servers, settings, "Open"))));
    files.push(text(&format!("Server/{name}.Server.csproj"), server_csproj(&name)));
    // The person's settings, which their browser keeps and each request brings (DESIGN 7.6).
    let kept = project.entities().find(|e| e.settings).map(|e| e.name.name.clone());
    files.push(text("Server/Program.cs", server_cs(&name, &main, &clarity_project::serve_address(settings), engines, kept.as_deref())));
    files.push(text(&format!("Browser/{name}.Browser.csproj"), browser_csproj(&name)));
    // Engines beside the screen run in the browser's own WebAssembly, through engine.js.
    let screen = !crate::screen_engines(project).is_empty();
    files.push(text("Browser/Program.cs", browser_cs(&name, screen)));
    if screen {
        files.push(text("Browser/Engines.cs", engines_cs(&name)));
        files.push(text("Browser/wwwroot/engine.js", ENGINE_JS.to_string()));
    }
    files.push(text("Browser/Page.cs", page_cs(&name)));
    files.push(text("Browser/wwwroot/clarity.js", CLARITY_JS.to_string()));
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
fn app_cs(project: &Project, name: &str, kept: Option<&str>) -> String {
    let currency = project.locale.currency.as_deref().map_or_else(|| "null".to_string(), string);
    // The person's settings, from the browser's storage, which each request takes to the server.
    let (start_settings, settings) = match kept {
        Some(s) => (format!("            {s}.Start();\n"), format!(", settings: (() => {s}.Current.ToJson(), sent => {s}.Current.Apply(sent))")),
        None => (String::new(), String::new()),
    };
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
{start_settings}            var holder = new ContentControl();
            single.MainView = holder;
            holder.Content = await MirrorShell.Start(link, Views.Create, TimeSpan.FromMilliseconds({MESSAGE_MS}){settings});
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
  <!-- The browser program, published beside the server, which serves it: again
       only when a file of the application or the browser program changed. -->
  <ItemGroup>
    <BrowserInputs Include=\"../App/**;../Browser/**\" Exclude=\"../App/bin/**;../App/obj/**;../Browser/bin/**;../Browser/obj/**\" />
  </ItemGroup>
  <!-- Built on Clarity's runtime projects instead of its packages, their sources count too. -->
  <ItemGroup Condition=\"'$(ClarityRuntime)' != ''\">
    <BrowserInputs Include=\"$(ClarityRuntime)/**/*.cs;$(ClarityRuntime)/**/*.csproj\" Exclude=\"$(ClarityRuntime)/**/bin/**;$(ClarityRuntime)/**/obj/**\" />
  </ItemGroup>
  <Target Name=\"PublishBrowser\" BeforeTargets=\"Build\" Condition=\"'$(SkipBrowser)' != 'true'\" Inputs=\"@(BrowserInputs)\" Outputs=\"browser/$(Configuration)/published\">
    <Exec Command=\"dotnet publish &quot;../Browser/{name}.Browser.csproj&quot; -c $(Configuration) -o &quot;$(MSBuildProjectDirectory)/browser/$(Configuration)&quot; &quot;-p:ClarityRuntime=$(ClarityRuntime)&quot; --nologo\" />
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
        framework = super::TARGET_FRAMEWORK
    )
}

fn server_cs(name: &str, main: &str, address: &str, engines: bool, kept: Option<&str>) -> String {
    let check_engines = check_engines(engines);
    // Each request runs with the settings of the person who sent it (DESIGN 7.6).
    let settings = kept.map(|s| format!(" {{ Settings = (() => {s}.Current.ToJson(), sent => {s}.Current.Apply(sent)) }}")).unwrap_or_default();
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
        new WebServer(new Sessions(() => new {main}ViewModel(db)){settings}, files).Serve({program}, WebServer.Address(args, {address}));
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
    <PackageReference Include=\"Avalonia.Browser\" Version=\"{avalonia}\" />
    <!-- A browser has none of the system's fonts: Inter, which the themes fall back on. -->
    <PackageReference Include=\"Avalonia.Fonts.Inter\" Version=\"{avalonia}\" />
    <ProjectReference Include=\"../App/{name}.csproj\" />
  </ItemGroup>
  <!-- Kept whole when trimmed: the windows, whose state the snapshot reads and
       sets by name, and Clarity's runtime, which does it. -->
  <ItemGroup>
    <TrimmerRootAssembly Include=\"{name}\" />
    <TrimmerRootAssembly Include=\"Clarity.Avalonia\" />
    <TrimmerRootAssembly Include=\"Clarity.Runtime\" />
  </ItemGroup>
</Project>
",
        framework = super::TARGET_FRAMEWORK,
        avalonia = avalonia_version()
    )
}

fn browser_cs(name: &str, screen: bool) -> String {
    let engines = if screen {
        "        // Engines beside the screen, in the browser's own WebAssembly.\n        await JSHost.ImportAsync(\"engine\", \"/engine.js\");\n        ScreenEngines.InBrowser = PageFrames.Load;\n"
    } else {
        ""
    };
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
    private static async System.Threading.Tasks.Task Main(string[] args)
    {{
        // What belongs to the browser: the tab's title, downloads, the person's storage.
        await JSHost.ImportAsync(\"clarity\", \"/clarity.js\");
        var page = new Page();
        BrowserPage.Current = page;
        // The person's settings stay in the browser's storage, and sounds play through its audio.
        Clarity.Runtime.KeptSettings.Store = (page.Read, page.Write);
        Sound.Output = new PageSoundOutput(page);
{engines}
        // The sessions are the server's the page came from.
        var origin = JSHost.GlobalThis.GetPropertyAsJSObject(\"location\")!.GetPropertyAsString(\"origin\")!;
        App.Sessions = new HttpSessionLink(new HttpClient {{ BaseAddress = new Uri(origin) }});
        // Inter, carried in the program, is the font the themes fall back on.
        await AppBuilder.Configure<App>().WithInterFont().With(new Avalonia.Media.FontManagerOptions {{ DefaultFamilyName = \"fonts:Inter#Inter\" }}).StartBrowserAppAsync(\"out\");
    }}
}}
"
    )
}

/// The browser's side of what belongs to it, through clarity.js.
fn page_cs(name: &str) -> String {
    format!(
        "{HEADER}
using System.Runtime.InteropServices.JavaScript;
using Clarity.Avalonia;

namespace {name}.Browser;

/// <summary>The page the application runs in: its tab, its downloads and the person's storage (DESIGN 7.6).</summary>
internal sealed partial class Page : IBrowserPage
{{
    [JSImport(\"setTitle\", \"clarity\")]
    private static partial void TabTitle(string title);

    [JSImport(\"download\", \"clarity\")]
    private static partial void Saved(string name, string data);

    [JSImport(\"show\", \"clarity\")]
    private static partial void Shown(string name, string data);

    [JSImport(\"read\", \"clarity\")]
    private static partial string? Kept(string key);

    [JSImport(\"write\", \"clarity\")]
    private static partial void Keep(string key, string value);

    [JSImport(\"play\", \"clarity\")]
    private static partial int Sounded(string data, double volume, bool repeat);

    [JSImport(\"stop\", \"clarity\")]
    private static partial void Stopped(int sound);

    public void SetTitle(string title) => TabTitle(title);

    public void Download(string name, byte[] data) => Saved(name, Convert.ToBase64String(data));

    public void Show(string name, byte[] data) => Shown(name, Convert.ToBase64String(data));

    public string? Read(string key) => Kept(key);

    public void Write(string key, string value) => Keep(key, value);

    public int Play(byte[] file, double volume, bool repeat) => Sounded(Convert.ToBase64String(file), volume, repeat);

    public void Stop(int sound) => Stopped(sound);
}}
"
    )
}

/// The engines beside the screen in a browser (DESIGN 7.6), built from the
/// web renderer's engine host (app/src/lib/browserEngines.ts).
const ENGINE_JS: &str = include_str!("../../ui-engine/engine.js");

/// A canvas's frames in the browser: its engine in the browser's own
/// WebAssembly, through engine.js, each call answered at once.
fn engines_cs(name: &str) -> String {
    format!(
        "{HEADER}
using System.Runtime.InteropServices.JavaScript;
using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace {name}.Browser;

/// <summary>An engine beside the screen in the browser (DESIGN 7.6): the browser's own WebAssembly, on the page's thread.</summary>
internal sealed partial class PageFrames : IFrames
{{
    [JSImport(\"load\", \"engine\")]
    private static partial int Loaded(string name, string wasm, string declared);

    [JSImport(\"hears\", \"engine\")]
    private static partial string HearsOf(int engine);

    [JSImport(\"frame\", \"engine\")]
    private static partial string Framed(int engine, double ms, string input);

    [JSImport(\"close\", \"engine\")]
    private static partial void Closed(int engine);

    readonly int engine;

    PageFrames(int engine)
    {{
        this.engine = engine;
        var hears = JsonNode.Parse(HearsOf(engine))!;
        Hears = new(hears[\"pointer\"]!.GetValue<bool>(), hears[\"keys\"]!.GetValue<bool>());
    }}

    /// <summary>Loads engine <paramref name=\"name\"/> from its file, as its declaration says.</summary>
    public static IFrames Load(string name, byte[] bytes, string[] uses, bool opens, long seconds, long megabytes)
    {{
        var declared = new JsonObject {{ [\"uses\"] = new JsonArray([.. uses.Select(u => JsonValue.Create(u))]), [\"opens\"] = opens, [\"seconds\"] = seconds, [\"megabytes\"] = megabytes }};
        try
        {{
            return new PageFrames(Loaded(name, Convert.ToBase64String(bytes), declared.ToJsonString()));
        }}
        catch (JSException e)
        {{
            throw new EngineException(Why(e));
        }}
    }}

    public Hears Hears {{ get; }}

    public Drawn Frame(double ms, IReadOnlyList<ScreenInput> input)
    {{
        var sent = new JsonArray();
        foreach (var given in input)
        {{
            sent.Add(given switch
            {{
                ScreenInput.Pointer p => new JsonObject {{ [\"pointer\"] = p.Kind, [\"x\"] = p.X, [\"y\"] = p.Y }},
                ScreenInput.Key k => new JsonObject {{ [\"key\"] = k.Kind, [\"name\"] = k.Name }},
                ScreenInput.Hear h => new JsonObject {{ [\"hear\"] = h.Name, [\"values\"] = JsonSerializer.SerializeToNode(h.Values) }},
                _ => null,
            }});
        }}
        string drawn;
        try
        {{
            drawn = Framed(engine, ms, sent.ToJsonString());
        }}
        catch (JSException e)
        {{
            throw new EngineException(Why(e));
        }}
        var reply = JsonNode.Parse(drawn)!;
        var tells = reply[\"tells\"]!.AsArray().Select(t => new ScreenTell(t![\"tell\"]!.GetValue<string>(), JsonSerializer.Deserialize<JsonElement[]>(t[\"values\"]!.ToJsonString())!)).ToList();
        return new Drawn(reply[\"shapes\"]!.DeepClone().AsArray(), tells);
    }}

    public void Dispose() => Closed(engine);

    /// <summary>What failed, as the engine host said it, without JavaScript's name for an error.</summary>
    static string Why(JSException e) => e.Message.StartsWith(\"Error: \") ? e.Message[7..] : e.Message;
}}
"
    )
}

/// What the page does for the application in the browser, which .NET calls.
const CLARITY_JS: &str = "// Generated by Clarity. Do not edit: change the project in Clarity instead.
// What belongs to the browser (DESIGN 7.6), as the application asks it.

const kinds = { pdf: 'application/pdf', png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif', svg: 'image/svg+xml', txt: 'text/plain', csv: 'text/csv', md: 'text/markdown' };

function blob(name, data) {
  const bytes = Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
  const extension = name.split('.').pop().toLowerCase();
  return URL.createObjectURL(new Blob([bytes], { type: kinds[extension] ?? 'application/octet-stream' }));
}

export function setTitle(title) {
  document.title = title;
}

export function download(name, data) {
  const link = document.createElement('a');
  link.href = blob(name, data);
  link.download = name;
  link.click();
}

export function show(name, data) {
  window.open(blob(name, data), '_blank', 'noopener');
}

// The person's storage: what a browser that keeps nothing cannot keep is not kept.
export function read(key) {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function write(key, value) {
  try {
    localStorage.setItem(key, value);
  } catch {
  }
}

// Sounds through the browser's own audio, each by its number until it ends or stops.
const sounds = new Map();
let nextSound = 0;

export function play(data, volume, repeat) {
  const audio = new Audio(blob('sound', data));
  audio.volume = Math.max(0, Math.min(1, volume));
  audio.loop = repeat;
  const number = ++nextSound;
  sounds.set(number, audio);
  audio.addEventListener('ended', () => sounds.delete(number));
  audio.play().catch(() => sounds.delete(number));
  return number;
}

export function stop(number) {
  const audio = sounds.get(number);
  if (audio) {
    audio.pause();
    sounds.delete(number);
  }
}
";

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

To put it online, build it in Release: the browser program is trimmed then, and the server sends its files compressed, so the first load is some 8 MB instead of nearly 50:

    dotnet build Server -c Release
    dotnet Server/bin/Release/net10.0/{name}.Server.dll

Clarity's runtime comes as NuGet packages (Clarity.Avalonia {version}, and the driver of each server database), which the first build fetches from nuget.org. To build on the runtime's own projects instead, while working on them, name their folder in Clarity's repository:

    dotnet build Server -p:ClarityRuntime=<Clarity>/runtimes/csharp

or set the environment variable `CLARITY_RUNTIME` to it.
",
        version = super::runtime_version()
    )
}
