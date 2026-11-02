//! The C# program on Clarity.UI when its databases do not open (DESIGN 5.27):
//! Program.cs starts Clarity.UI's unconnected flow through the desktop host,
//! with the window that keeps the connection, the app's `on unconnected`,
//! the settings and an empty database in memory.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, SourceKind, Ui, starter};

const SHOP: &str = "module shop

settings
    shopDb: secret

entity Customer
    name: text(80)

page Main: window
    password name connection at 1,1 span 6 bind settings.shopDb
";

const HANDLED: &str = "
app Shop
    main Main
    on unconnected: notConnected

code Shop
    handler notConnected(reason: text)
        message reason
        open Main
";

const MIGRATION: &str = "migration \"First\"\n    add entity Customer\n        name: text(80)\n";

fn files(source: &str, secret: bool) -> Vec<OutFile> {
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Clarity;
    if secret {
        settings.datasources[0].kind = SourceKind::Postgres;
        settings.datasources[0].url = "setting:shopDb".into();
    }
    let project = clarity_spec::load_in([("shop.clarity", source)], [("0001.clarity", MIGRATION)], &clarity_project::source_refs(&settings));
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

#[test]
fn the_window_that_keeps_the_connection_shows_when_the_databases_do_not_open() {
    let files = files(SHOP, true);
    let program = file(&files, "Program.cs");
    assert!(program.contains("using Clarity.Ui;\n"), "{program}");
    assert!(program.contains("        catch (Exception e)\n        {\n            // The databases did not open (DESIGN 5.27): what keeps the connection shows first.\n            DesktopWindow.Run(new Unconnected\n            {\n                Connect = OpenDatabases,\n                Empty = Sqlite.Memory,\n                Main = db => new MainViewModel(db),\n                Window = db => new MainViewModel(db),\n                Settings = Settings.Current,\n            }, Unconnected.Reason(e));\n            return 0;\n        }\n"), "{program}");
    assert!(!program.contains("Handler ="), "{program}");
    // The empty database in memory is SQLite's, even when the data sources are not.
    let csproj = file(&files, "Shop.csproj");
    assert!(csproj.contains("<PackageReference Include=\"Clarity.Runtime.Sqlite\""), "{csproj}");
}

#[test]
fn the_apps_handler_runs_on_its_window_that_is_not_shown() {
    let source = format!("{SHOP}{HANDLED}");
    let files = files(&source, true);
    let program = file(&files, "Program.cs");
    assert!(program.contains("                Handler = (db, unconnecting) => new AppUnconnectedViewModel(db).Unconnected(unconnecting),\n"), "{program}");
    assert!(program.contains("                Window = db => new MainViewModel(db),\n"), "{program}");
}

#[test]
fn without_a_window_or_handler_the_program_says_why_and_stops() {
    let plain = SHOP.replace("    password name connection at 1,1 span 6 bind settings.shopDb\n", "    textbox name nameBox at 1,1 span 6\n");
    let files = files(&plain, false);
    let program = file(&files, "Program.cs");
    assert!(program.contains("Console.Error.WriteLine(Locale.Show(Unconnecting.Why(e)));\n            return 1;"), "{program}");
    assert!(!program.contains("new Unconnected"), "{program}");
}
