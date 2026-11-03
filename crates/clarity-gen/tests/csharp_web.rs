//! The web target of C# on Clarity.UI (DESIGN 7.6, 7.9): the windows' view
//! models run on a server, in a session per browser tab; the browser program
//! is Clarity.UI in WebAssembly, drawing each window from its markup over a
//! mirror of the server's state. Three projects: the application as a
//! library, the server, and the browser program the server's build
//! publishes into it.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Target, Ui, starter};

const WALKER: &str = "module walker

settings
    speed: integer, default 2

// Kept in the server's database.
entity Score
    points: integer

app Walker
    title \"Walker\"
    main Field

page Field: window
    title \"Field\"
    data x: integer, default 10
    canvas name field at 1,1 span 12 rows 8 units 200 x 100 on draw: drawField
    button \"Walk\" name walkButton at 1,9 span 3 on click: walkClicked
    timer name clock every 0.1 on tick: stepped

code Field
    handler walkClicked
        clock.enabled = true

    handler stepped
        x = x + 1

    handler drawField
        field.clear(\"panel\")
";

fn settings(targets: Vec<Target>) -> Settings {
    let mut settings = starter("Walker", "walker");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Clarity;
    settings.stack.targets = targets;
    settings
}

fn files(targets: Vec<Target>) -> Vec<OutFile> {
    let project = clarity_spec::keyed::load_keyed([("walker.clarity", WALKER)], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    generate(&project, &settings(targets)).unwrap()
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

fn lacks(text: &str, unexpected: &str) {
    assert!(!text.contains(unexpected), "did not expect\n{unexpected}\nin\n{text}");
}

#[test]
fn the_web_target_writes_the_application_the_server_and_the_browser_program() {
    let files = files(vec![Target::Web]);
    for path in [
        "App/Walker.csproj",
        "App/Windows/FieldViewModel.cs",
        "App/Windows/Field.cxaml",
        "App/Modules/Walker.cs",
        "App/Texts.cs",
        "App/Fields.cs",
        "Server/Walker.Server.csproj",
        "Server/Program.cs",
        "Server/Databases.cs",
        "Browser/Walker.Browser.csproj",
        "Browser/Directory.Build.props",
        "Browser/Program.cs",
        "Browser/wwwroot/index.html",
        "Browser/wwwroot/main.js",
        "README.md",
    ] {
        file(&files, path);
    }
    // The application starts nowhere itself: the server and the browser program start it.
    assert!(!files.iter().any(|f| f.path == "App/Program.cs" || f.path == "Program.cs"));
}

#[test]
fn the_application_is_a_library_on_clarity_ui_without_the_desktop_or_a_driver() {
    let files = files(vec![Target::Web]);
    let app = file(&files, "App/Walker.csproj");
    has(app, "<OutputType>Library</OutputType>");
    // On the browser package, which carries Clarity.UI's drawing in C# until the browser build (R2).
    has(app, "<PackageReference Include=\"Clarity.Ui.Browser\" Version=");
    has(app, "<ProjectReference Include=\"$(ClarityRuntime)/Clarity.Ui.Browser/Clarity.Ui.Browser.csproj\" />");
    has(app, "<EmbeddedResource Include=\"Windows/*.cxaml\"");
    // The browser program carries the library: no native window, no database driver.
    lacks(app, "Clarity.Ui.Desktop");
    lacks(app, "Clarity.Runtime.Sqlite");
    lacks(app, "WinExe");
}

#[test]
fn the_server_keeps_the_sessions_opens_the_databases_and_serves_the_browser_program() {
    let files = files(vec![Target::Web]);
    let server = file(&files, "Server/Walker.Server.csproj");
    has(server, "<ProjectReference Include=\"../App/Walker.csproj\" />");
    has(server, "<PackageReference Include=\"Clarity.Ui.Server\" Version=");
    has(server, "<ProjectReference Include=\"$(ClarityRuntime)/Clarity.Ui.Server/Clarity.Ui.Server.csproj\" />");
    has(server, "<PackageReference Include=\"Clarity.Runtime.Sqlite\" Version=");
    // Its build publishes the browser program beside it, unless told not to.
    has(server, "dotnet publish &quot;../Browser/Walker.Browser.csproj&quot;");
    has(server, "Condition=\"'$(SkipBrowser)' != 'true'\"");
    // The nested publish leaves no compiler server or build nodes behind, which would hold a build's lock.
    has(server, "-p:UseSharedCompilation=false -nodeReuse:false --nologo\" EnvironmentVariables=\"MSBUILDDISABLENODEREUSE=1\" />");
    let program = file(&files, "Server/Program.cs");
    has(program, "using Clarity.Ui.Server;");
    has(program, "using var db = Databases.Open();");
    has(program, "new Sessions(() => new FieldViewModel(db))");
    // The windows' markup is the server's: it serves it from the application it runs.
    has(program, "typeof(FieldViewModel).Assembly");
    has(program, "WebServer.Address(args, \"127.0.0.1:8080\")");
    // The person's settings come with each request and go back with each reply (DESIGN 7.6).
    has(program, "Settings = (() => Settings.Current.ToJson(), sent => Settings.Current.Apply(sent))");
    has(program, "args.Contains(\"--check\")");
    let databases = file(&files, "Server/Databases.cs");
    has(databases, "        Sqlite.Use();\n");
    has(databases, "public static Db Open()");
}

#[test]
fn the_browser_program_is_clarity_ui_in_webassembly_over_the_server_s_sessions() {
    let files = files(vec![Target::Web]);
    let browser = file(&files, "Browser/Walker.Browser.csproj");
    has(browser, "<Project Sdk=\"Microsoft.NET.Sdk.WebAssembly\">");
    has(browser, "<TargetFramework>net10.0-browser</TargetFramework>");
    has(browser, "<ProjectReference Include=\"../App/Walker.csproj\" />");
    has(browser, "<PackageReference Include=\"Clarity.Ui.Browser\" Version=");
    has(browser, "<ProjectReference Include=\"$(ClarityRuntime)/Clarity.Ui.Browser/Clarity.Ui.Browser.csproj\" />");
    lacks(browser, "Avalonia");
    lacks(browser, "Sqlite");
    let program = file(&files, "Browser/Program.cs");
    has(program, "using Clarity.Ui.Browser;");
    // What the browser keeps (the language, the theme, the settings) is read before the windows show.
    has(program, "await BrowserCanvas.Ready();");
    has(program, "Remembered.Use(\"Walker\");");
    has(program, "Locale.Start(Texts.BaseLanguage, Texts.Languages, null, Texts.In, Labels.Named);");
    has(program, "AppThemes.Start();");
    has(program, "Settings.Start();");
    has(program, "await BrowserCanvas.Serve(typeof(Texts).Assembly, settings: (() => Settings.Current.ToJson(), sent => Settings.Current.Apply(sent)));");
    let page = file(&files, "Browser/wwwroot/index.html");
    has(page, "<title>Walker</title>");
    has(page, "<canvas id=\"clarity\"");
    has(file(&files, "Browser/wwwroot/main.js"), "import { dotnet } from './_framework/dotnet.js';");
    let readme = file(&files, "README.md");
    has(readme, "dotnet workload install wasm-tools");
    has(readme, "dotnet build Server");
    lacks(readme, "Avalonia");
}

/// With `stack csharp desktop web` the target list gives two programs, each
/// in its folder of `out` and each building on its own: the desktop program
/// and the web program (application, server and browser program).
#[test]
fn desktop_and_web_together_make_the_desktop_and_the_web_program() {
    let project = clarity_spec::keyed::load_keyed([("walker.clarity", WALKER)], &[], "en");
    let programs = clarity_gen::programs(&project, &settings(vec![Target::Desktop, Target::Web])).unwrap_or_else(|e| panic!("{e:#?}"));
    assert_eq!(programs.iter().map(|p| p.target).collect::<Vec<_>>(), [Target::Desktop, Target::Web]);
    let desktop = &programs[0].files;
    file(desktop, "Walker.csproj");
    file(desktop, "Program.cs");
    assert!(!desktop.iter().any(|f| f.path.starts_with("Server/") || f.path.starts_with("Browser/")));
    has(file(desktop, "Walker.csproj"), "<PackageReference Include=\"Clarity.Ui\" ");
    let web = &programs[1].files;
    file(web, "App/Walker.csproj");
    file(web, "Server/Walker.Server.csproj");
    file(web, "Browser/Walker.Browser.csproj");
    assert!(!web.iter().any(|f| f.path == "Program.cs"));
    lacks(file(web, "App/Walker.csproj"), "Clarity.Ui.Desktop");
    // Each README tells how to build its own program.
    has(file(web, "README.md"), "dotnet build Server");
    lacks(file(desktop, "README.md"), "dotnet build Server");
}

#[test]
fn generated_web_files_use_no_dashes_of_prose() {
    for f in files(vec![Target::Web]) {
        assert!(!f.text.contains('\u{2014}') && !f.text.contains('\u{2013}'), "{}", f.path);
    }
}

/// The web program has one README: how to build and start the server, and
/// where its data is. A second, the desktop program's, written over it said
/// to build and run a project the folder does not have.
#[test]
fn the_web_program_has_one_readme_with_its_server_and_its_data() {
    let files = files(vec![Target::Web]);
    let readmes: Vec<&OutFile> = files.iter().filter(|f| f.path == "README.md").collect();
    assert_eq!(readmes.len(), 1, "{:?}", readmes.iter().map(|f| &f.text).collect::<Vec<_>>());
    let readme = &readmes[0].text;
    has(readme, "dotnet build Server");
    has(readme, "--address host:port");
    has(readme, "## Data");
    has(readme, "CLARITY_DATA");
    lacks(readme, "dotnet run\n");
}

/// A language the person picks in the browser shows: the page carries the
/// data of every culture (the default ICU shard of an English page has no
/// Dutch) and loads every language's texts, not only the starting one's.
/// Seen in the walker example, where Nederlands was chosen and kept but the
/// page stayed English, before and after a reload.
#[test]
fn the_browser_program_carries_every_language_it_can_be_shown_in() {
    let files = files(vec![Target::Web]);
    has(file(&files, "Browser/Walker.Browser.csproj"), "<BlazorWebAssemblyLoadAllGlobalizationData>true</BlazorWebAssemblyLoadAllGlobalizationData>");
    has(file(&files, "Browser/wwwroot/main.js"), "await dotnet.withConfig({ loadAllSatelliteResources: true }).run();");
}

/// The server knows the application's languages and themes: the settings a
/// browser sends are checked against them, and a language it did not know
/// (Dutch) was refused and sent back as English, so the page never switched.
#[test]
fn the_server_knows_the_languages_and_themes_the_settings_may_name() {
    let files = files(vec![Target::Web]);
    let program = file(&files, "Server/Program.cs");
    has(program, "Locale.Start(Texts.BaseLanguage, Texts.Languages, null, Texts.In, Labels.Named);");
    has(program, "AppThemes.Start();");
    let start = program.find("Locale.Start(").unwrap();
    assert!(start < program.find("new Sessions(").unwrap(), "{program}");
}
