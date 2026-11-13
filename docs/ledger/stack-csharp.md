# C# stack

Status: built. `stack csharp ...`: a net10.0 project on the Clarity NuGets; desktop on Clarity.Ui, web as a second program.

Progress: 90%

## Code
- Generators: `crates/clarity-gen/src/csharp/` (logic.rs 1551 lines, windows.rs 898 (view models), mod.rs 811, model.rs 530, cli.rs 383, names.rs 262, parts.rs 221, plus web.rs, texts.rs (`.resx` and a typed `Texts`), themes.rs (theme JSON))
- Runtimes: C# `runtimes/csharp` (Version 0.96.0, `Directory.Build.props:9`, tagged; `Clarity.slnx`). 14 NuGets: Clarity.Runtime, .Sqlite, .PostgreSql, .MySql, .SqlServer, .Engines (Wasmtime 48), Clarity.Ui, Clarity.Ui.Server, Clarity.Audio, Clarity.Video, Clarity.Image, Clarity.Formats, Clarity.Auth, Clarity.AppMcp (`runtimes/csharp/Clarity.AppMcp`, `<program> mcp`; see [app-mcp](app-mcp.md))
  - `Clarity.Runtime/`: Db, Query, Sql, Dialect, Record, EditBus, Undo, Backup, Signin, Secrets, Files, Pictures, RecordFiles, Http, Zip, Cli, Formats, Agents, WindowViewModel, PartState, `Program/` (ProgramWire, Handlers, Members, OpenWindow, WindowMarkup, AppFiles)
  - `Clarity.Ui/`: DesktopWindow.cs and `Custom/` (CustomPart, CustomParts, Drawing); the drawing is the Rust Clarity.UI library in the package's native folder
  - `Clarity.Ui.Server/WebServer.cs`: its own socket server for the browser files and the wire
- Desktop program: `<Name>.csproj` WinExe on Clarity.Ui; one C# file per module, a view model per window, `Windows/*.cxaml`, `Apps/*.cxaml`, theme JSON, `.resx` (`csharp/mod.rs:1-9`). `ClarityRuntime` (or CLARITY_RUNTIME) switches PackageReference to the repository's projects (`csharp/mod.rs:317-359`)
- Web: a second program, `out/web/` with `App/` (library on Clarity.Runtime) and `Server/` (on Clarity.Ui.Server) (`csharp/web.rs`, `crates/clarity-gen/src/lib.rs:974-980`); see [web-target](web-target.md)
- Custom parts: `nuget` and `dll` libraries become Clarity.UI custom controls (`csharp/parts.rs:26, 56, 138`)
- Docs: manual/stacks.csharp.md

## Tests
- `runtimes/csharp/Clarity.Runtime.Tests` (UndoTests.cs, VersionsTests.cs, ServerTests.cs, ...), `Clarity.Ui.Native.Tests` (ProgramWireTests.cs, CustomPartTests.cs, ...), `Clarity.Ui.Server.Tests` (WebServerTests.cs, ...)
- Conformance skipped by C#: accordions, charts, dropdown-options, folding, ribbons, sidebars, steps, table-pages, trees, ranges, table-order, custom-parts; `migrations` runs only on server databases, which CI does not have; `calls` parsed in `Program/Members.cs:116` with no test (CONFLICTING 3)
- Generator: `crates/clarity-gen/tests/csharp.rs`, csharp_windows.rs, csharp_web.rs, csharp_canvas_modes.rs, unconnected_csharp.rs, versions_csharp.rs, signin_csharp.rs; build: csharp_build.rs (ignored)
- Examples on this stack: mp3player, noughts, todo (`csharp desktop`), walker (`csharp web`), ledger (`csharp cli`)

## Open
- The C# program driven live in tests: NOT BUILT (`crates/clarity-gen/tests/everything_live.rs` has go, rust, c, cpp)
- `list<Entity>` fields: NOT BUILT (`csharp/logic.rs:1017`)
- Dead code: `Melodies.cs`, `KeyRules.cs`, `TablePages.cs`, `Charts.cs`, `Ranges.cs`, `Palette.cs`, `PersonKeys.Set` have no callers outside tests; each copies a rule Clarity.UI or clarity-runtime decides (CONFLICTING 4)

## Conflicts
- Ordered comparisons: plain `(l < r)` (`csharp/logic.rs:1277`); `none < d` is false on dates (first on the others); text `<` does not compile
- Edit bus never drops edits (`EditBus.cs:29-35`); the others keep 500
- The person's keys have two writers: Clarity.UI (`runtimes/rust-ui/src/window/mod.rs:104-109`) and `PersonKeys` on every settings save (`csharp/model.rs:287`, `PersonKeys.cs:25`)
- HTTP timeout says "no answer within 30 seconds" (`Http.cs:260`); the others pass the library's text
- One-letter keys counted in UTF-16 units (`KeyRules.cs:79`); Rust counts chars (`runtimes/rust/src/keys.rs:107`)
- `csharp/logic.rs:558` says custom parts are refused "until CP1 builds them"; they are generated
- Conformance "about" texts still name Avalonia (custom-parts.json, forms.json, tables.json, table-order.json, display.json, themes.json)
- `.github/workflows/release-csharp-runtime.yml` says eleven packages; it publishes 14 (`release-csharp-runtime.yml:198`).

## Decisions
- 2026-10-19: runtimes are NuGets, not copied sources. User: "we'll need to make that one into a nuget too... and there where others which I said should be nugets"
- 2026-11-02: Avalonia removed. User: "I NO LONGER WANT TO SEE AVALONIA AT ALL"
- 2026-11-03: the browser program and sessions (W2) replaced by Clarity.UI in the browser answered by `ProgramWire` (DESIGN 7.6 note)
- Locked dotnet runs need `MSBUILDDISABLENODEREUSE=1`: MSBuild nodes inherit the build lock (memory: msbuild-holds-lock.md)
- Building and publishing the NuGets: see [release-ci](release-ci.md); the database drivers: [databases](databases.md); themes: [themes](themes.md); the web program's history (DESIGN 7.6, W2): [web-target](web-target.md)

## Was
- G2a, G2b, G2c (formats and translations; themes part is [themes](themes.md)), DESIGN 7.1 (C# row)
- memory: msbuild-holds-lock.md
