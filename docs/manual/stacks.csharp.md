# C# with Clarity.UI

How Clarity turns a project into a C# desktop program whose windows Clarity.UI draws, what you need to install, and how to build it.

Clarity.UI is Clarity's own UI framework. It draws the windows itself with Skia, on the desktop and in a browser page, and every stack gets its own version of it (DESIGN 7.9). The C# version is the first. It does not draw a window's parts yet: a C# program opens its main window with the window's title and size, and the parts follow as Clarity.UI gets them.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```clarity
stack csharp desktop
```

Clarity.UI is not named in the stack line: after the language come the targets. The web target of C# is not generated yet. See [The project file](project.file.md).

## What is generated

Clarity writes a .NET project into the output folder (`out` by default). `<Name>` is the project name in Pascal case.

- `<Name>.csproj`: the project, targeting .NET 10 (`net10.0`).
- `Modules/<Module>.cs`: one file per Clarity module, with its enums, records, functions, actions and data events.
- `Windows/<Window>ViewModel.cs`: each window's view model, which holds its data, the state of its parts and its handlers.
- `Windows/<Window>.cxaml` and `Apps/<App>.cxaml`: each window and each app frame as markup, which Clarity.UI draws (see [Windows](design.windows.md)).
- `Program.cs`: the start of the program. It starts the language and theme kept from the last run, opens the databases and shows the main window.
- `Labels.cs`: the labels of enum values, for texts with slots.
- `Schema.cs`: the migration steps of every database Clarity owns.
- `Texts.resx` with the texts as written, `Texts.<language>.resx` per language, and `Texts.cs`, a typed class with a property per text. See [Languages](languages.md).
- `Themes/<theme>.light.json` and `Themes/<theme>.dark.json` per offered theme, as Clarity resolves it, and `AppThemes.cs`, which lists them. See [Themes](themes.md).
- `resources/`: the project's resources (pictures and Markdown files), embedded in the program.
- `README.md`: how to build the project without Clarity.

Do not edit these files. Change the project in Clarity; it writes the folder again.

## The runtime packages

The generated project carries no runtime sources. It references NuGet packages from nuget.org, at one shared version:

- `Clarity.Ui.Desktop`: Clarity.UI in the system's own window, on Linux, Windows and macOS. It brings `Clarity.Ui`, which draws, and `Clarity.Runtime`: the window model (view models, part state, undo), records, queries and migrations, without a database driver.
- `Clarity.Runtime.Sqlite`, `Clarity.Runtime.PostgreSql`, `Clarity.Runtime.MySql` and `Clarity.Runtime.SqlServer`: one per kind of database, added only when a data source uses it.
- `Clarity.Runtime.Engines`: runs [engines](code.engines.md) with Wasmtime, added only when the project has one.

To build on the runtime's own projects in Clarity's repository instead (while you work on the runtime itself), name their folder:

```shell
dotnet build -p:ClarityRuntime=<Clarity>/runtimes/csharp
```

Or set the environment variable `CLARITY_RUNTIME` to that folder. Without either, the build uses the packages.

## What to install

- The .NET 10 SDK, from https://dotnet.microsoft.com/download

Clarity checks for it itself: run `clarity tools`, or look at Build tools on the Project tab.

Step by step, with the commands for Windows, Linux and macOS: [Setting up the build tools](setup.tools.md).

## Build and run

**Generate** on the Preview tab writes the source into the output folder; then **Run** builds a debug program from it and starts it, and **Build** makes a release program. Without Clarity, in the output folder:

```shell
dotnet build -c Release
dotnet run
```

The release program is `bin/Release/net10.0/<Name>` (with `.exe` on Windows). A debug build goes to `bin/Debug/net10.0/<Name>`.

## Not yet on C#

These come with Clarity.UI's parts and its custom controls (DESIGN 7.9):

- the parts of a window, its frame and its dialogs;
- the window that keeps a database connection when the databases do not open; until then the program says why and stops;
- custom parts from a .NET library;
- the web target.

## The same on every stack

The parts of a window behave alike on every stack. The shared cases in `conformance/` say what each part must do, and every version of Clarity.UI runs them.
