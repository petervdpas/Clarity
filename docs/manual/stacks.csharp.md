# C# with Avalonia

How Clarity turns a project into a C# desktop program whose windows are native Avalonia views, what you need to install, and how to build it.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```clarity
stack csharp avalonia desktop
```

C# always uses Avalonia. C# with the web renderer is not generated yet. See [The project file](project.file.md).

## What is generated

Clarity writes a .NET project into the output folder (`out` by default). `<Name>` is the project name in Pascal case.

- `<Name>.csproj`: the project, targeting .NET 10 (`net10.0`).
- `Modules/<Module>.cs`: one file per Clarity module, with its enums, records, functions, actions and data events.
- `Windows/<Window>View.axaml`, its code behind `<Window>View.axaml.cs`, and `Windows/<Window>ViewModel.cs`: each window as an Avalonia view with a view model that holds its data and handlers.
- `Program.cs`, `App.axaml`, `App.axaml.cs`, `Views.cs` and `Shown.cs`: the start of the program, the list of views, and how fields are shown and read.
- `Schema.cs`: the migration steps of every database Clarity owns.
- `Texts.resx` with the texts as written, `Texts.<language>.resx` per language, and `Texts.cs`, a typed class with a property per text. See [Languages](languages.md).
- `Themes/<Theme>.axaml` per offered theme, with its Light and Dark colors, and `AppThemes.cs`, which lists them. See [Themes](themes.md).
- `images/`: the project's pictures, embedded in the program.
- `README.md`: how to build the project without Clarity.

Do not edit these files. Change the project in Clarity; it writes the folder again.

## The runtime packages

The generated project carries no runtime sources. It references NuGet packages from nuget.org, at one shared version:

- `Clarity.Avalonia`: the windows, parts and shell. It brings `Clarity.Runtime`, which stores records in SQLite.
- `Clarity.Runtime.PostgreSql`, `Clarity.Runtime.MySql` and `Clarity.Runtime.SqlServer`: one per server database, added only when a data source uses it.
- `Clarity.Runtime.Engines`: runs [engines](code.engines.md) with Wasmtime, added only when the project has one.

To build on the runtime's own projects in Clarity's repository instead (while you work on the runtime itself), name their folder:

```shell
dotnet build -p:ClarityRuntime=<Clarity>/runtimes/csharp
```

Or set the environment variable `CLARITY_RUNTIME` to that folder. Without either, the build uses the packages.

## What to install

- The .NET 10 SDK, from https://dotnet.microsoft.com/download

Clarity checks for it itself: run `clarity tools`, or look at Build tools on the Project tab.

## Build and run

**Run** on the Preview tab builds a debug program and starts it; **Build** makes a release program. Without Clarity, in the output folder:

```shell
dotnet build -c Release
dotnet run
```

The release program is `bin/Release/net10.0/<Name>` (with `.exe` on Windows). A debug build goes to `bin/Debug/net10.0/<Name>`.

## Windows on screen

The application has one desktop window. `open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. A dialog gets a desktop window of its own. A click target marked `in new window` opens a second desktop window with its own stack of windows. See [Windows](design.windows.md).

The designer shows an Avalonia window's layout faithfully, but not pixel for pixel. The real window appears with Run.

## The same on every stack

The parts of a window behave alike on Avalonia and on the web renderer that the other stacks use. Each part is Clarity's own control (in `Clarity.Avalonia`) rather than a stock one, and the shared cases in `conformance/` say what each part must do. Both renderers run them, and a difference fails the build of the renderer that has it.
