//! The web target of C# on Clarity.UI (DESIGN 7.6, 7.9): the windows' view
//! models run on a server; in the browser runs Clarity.UI's library,
//! compiled once for every language, which asks the server for what each
//! window does. Two projects, the application as a library and the server,
//! and `browser/` with the page, beside which Clarity puts the browser files.

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
fn the_web_target_writes_the_application_the_server_and_the_page() {
    let files = files(vec![Target::Web]);
    for path in [
        "App/Walker.csproj",
        "App/Windows/FieldViewModel.cs",
        "App/Windows/Field.cxaml",
        "App/Modules/Walker.cs",
        "App/Texts.cs",
        "App/Fields.cs",
        "App/model.json",
        "Server/Walker.Server.csproj",
        "Server/Program.cs",
        "Server/Databases.cs",
        "browser/index.html",
        "README.md",
    ] {
        file(&files, path);
    }
    // The application starts nowhere itself: the server starts it.
    assert!(!files.iter().any(|f| f.path == "App/Program.cs" || f.path == "Program.cs"));
    // No browser program of its own: Clarity.UI's browser files are the same for every language.
    assert!(!files.iter().any(|f| f.path.starts_with("Browser/")), "{:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>());
    // So the library's files join it, as they join every web program.
    assert!(clarity_gen::web_browser::serves(&files));
}

#[test]
fn the_application_is_a_library_on_the_runtime_carrying_the_browser_files() {
    let files = files(vec![Target::Web]);
    let app = file(&files, "App/Walker.csproj");
    has(app, "<OutputType>Library</OutputType>");
    // On the runtime alone, which answers Clarity.UI: no Clarity.Ui, no native library.
    has(app, "<PackageReference Include=\"Clarity.Runtime\" Version=");
    has(app, "<ProjectReference Include=\"$(ClarityRuntime)/Clarity.Runtime/Clarity.Runtime.csproj\" />");
    lacks(app, "\"Clarity.Ui\"");
    lacks(app, "Clarity.Ui.Browser");
    has(app, "<EmbeddedResource Include=\"Windows/*.cxaml\"");
    // The page and the library's browser files, for the server to serve.
    has(app, "<EmbeddedResource Include=\"../browser/**\" LogicalName=\"browser/%(RecursiveDir)%(Filename)%(Extension)\" />");
    // The server opens the databases: no driver here.
    lacks(app, "Clarity.Runtime.Sqlite");
    lacks(app, "WinExe");
    // The page is the one every language serves.
    let page = file(&files, "browser/index.html");
    has(page, "<title>Walker</title>");
    has(page, "<canvas id=\"clarity-ui\"></canvas>");
    has(page, "<script src=\"clarity-ui-page.js\"></script>");
}

#[test]
fn the_server_opens_the_databases_and_serves_the_application() {
    let files = files(vec![Target::Web]);
    let server = file(&files, "Server/Walker.Server.csproj");
    has(server, "<ProjectReference Include=\"../App/Walker.csproj\" />");
    has(server, "<PackageReference Include=\"Clarity.Ui.Server\" Version=");
    has(server, "<ProjectReference Include=\"$(ClarityRuntime)/Clarity.Ui.Server/Clarity.Ui.Server.csproj\" />");
    has(server, "<PackageReference Include=\"Clarity.Runtime.Sqlite\" Version=");
    // Nothing is published into it: no browser program, no WebAssembly tools.
    lacks(server, "dotnet publish");
    lacks(server, "Browser");
    let program = file(&files, "Server/Program.cs");
    has(program, "using Clarity.Ui.Server;");
    has(program, "using var db = Databases.Open();");
    has(program, "new WebServer(new FieldViewModel(db)).Serve(\"Walker\", WebServer.Address(args, \"127.0.0.1:8080\"));");
    has(program, "args.Contains(\"--check\")");
    lacks(program, "Sessions");
    let databases = file(&files, "Server/Databases.cs");
    has(databases, "        Sqlite.Use();\n");
    has(databases, "public static Db Open()");
    let readme = file(&files, "README.md");
    has(readme, "dotnet build Server");
    lacks(readme, "wasm-tools");
    lacks(readme, "Avalonia");
}

/// With `stack csharp desktop web` the target list gives two programs, each
/// in its folder of `out` and each building on its own: the desktop program
/// and the web program (application, server and page).
#[test]
fn desktop_and_web_together_make_the_desktop_and_the_web_program() {
    let project = clarity_spec::keyed::load_keyed([("walker.clarity", WALKER)], &[], "en");
    let programs = clarity_gen::programs(&project, &settings(vec![Target::Desktop, Target::Web])).unwrap_or_else(|e| panic!("{e:#?}"));
    assert_eq!(programs.iter().map(|p| p.target).collect::<Vec<_>>(), [Target::Desktop, Target::Web]);
    let desktop = &programs[0].files;
    file(desktop, "Walker.csproj");
    file(desktop, "Program.cs");
    assert!(!desktop.iter().any(|f| f.path.starts_with("Server/") || f.path.starts_with("browser/")));
    has(file(desktop, "Walker.csproj"), "<PackageReference Include=\"Clarity.Ui\" ");
    let web = &programs[1].files;
    file(web, "App/Walker.csproj");
    file(web, "Server/Walker.Server.csproj");
    file(web, "browser/index.html");
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

/// The server knows the application's languages and themes, which the
/// library in the browser may choose (set_language).
#[test]
fn the_server_knows_the_languages_and_themes_the_settings_may_name() {
    let files = files(vec![Target::Web]);
    let program = file(&files, "Server/Program.cs");
    has(program, "Locale.Start(Texts.BaseLanguage, Texts.Languages, null, Texts.In, Labels.Named);");
    has(program, "AppThemes.Start();");
    let start = program.find("Locale.Start(").unwrap();
    assert!(start < program.find("new WebServer(").unwrap(), "{program}");
}
