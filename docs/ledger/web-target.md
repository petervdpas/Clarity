# Web target

Status: built. `stack <backend> web`: the program keeps the application on a server; the browser runs Clarity.UI (WebAssembly, Skia on WebGL2, in a Web Worker) and sends each command to the server.

## Code
- Spec: `crates/clarity-project/src/file.rs:401-473` (`stack ... web`), :369 (`port N`; default 8080 at `127.0.0.1:<port>`, `crates/clarity-project/src/lib.rs:285`)
- Generators: browser files `crates/clarity-gen/src/web_browser.rs` (`browser/index.html`, `browser/clarity/files.json`, resources, the library's four files added by `with_library`); `crates/clarity-gen/src/lib.rs:947` (`programs`), :983 (`windowed_target`: `out/web` when served alone), :1628 (`check_parts_in_browser`), :971 (picture functions refused on Go, C, C++ web-only); C# second program `crates/clarity-gen/src/csharp/web.rs`
- Runtimes: R `runtimes/rust/src/serve.rs:275` (`serve`; `--serve` in a desktop + web program, `crates/clarity-gen/src/rust/mod.rs:816`); Go `runtimes/go/shell/serve.go` (`go/mod.rs:938`); C `runtimes/c/service/serve.c`, `service.c`, `wire.c`; C++ `runtimes/cpp/shell/serve.cpp`, `service.cpp`; C# `runtimes/csharp/Clarity.Ui.Server/WebServer.cs` (plain socket, not HttpListener), the App library on `Clarity.Runtime`
- Clarity.UI: `runtimes/rust-ui/src/browser/` (host.rs, input.rs, page.rs, web.rs); build for the browser `runtimes/rust-ui-browser` (Emscripten, `build.sh`); sound and video through the page (`page.rs:16-22`)
- IDE: Run starts the server at the project's port, Browser opens it (`app/src-tauri/src/build.rs:514`)
- Docs: manual `project.run-build.md`, stack pages

What each stack writes:

| Stack | Web program |
|---|---|
| R | the desktop crate plus browser files embedded; desktop + web is one program, `--serve` serves |
| Go | module with browser files; one program, `--serve` serves |
| C | CMake project with browser files carried in the program |
| C++ | the same shape |
| C# | `out/web/`: App library (view models, markup, model) on Clarity.Runtime, Server on Clarity.Ui.Server, `browser/` |

- Address: `--address host:port`, else `CLARITY_ADDRESS`, else `127.0.0.1:<port>`.
- What belongs to the browser: title is the tab's, links open tabs, saves are downloads, settings in browser storage without secrets; the folder part has no picker (`runtimes/rust-ui/src/parts/inputs/folder.rs:8`).
- Sign-in on the server: see [signin](signin.md).

## Tests
- `crates/clarity-gen/tests/build.rs:594`, `c_build.rs:279`, `cpp_build.rs:352` (`the_web_target_serves_in_*`, ignored); `csharp_web.rs`; `browser_fresh.rs`; `out_layout.rs` (`a_program_served_alone_is_the_web_program`); `everything_build.rs` (web for R, Go, C, C++); `everything_live.rs` (headless Chromium: go, rust, c, cpp)
- `runtimes/go/shell/serve_test.go`; `runtimes/csharp/Clarity.Ui.Server.Tests`
- Examples: `invaders` (go web), `walker` (csharp web); no example uses rust web, c web or cpp web

## Open
- The web targets in Edge; the walker in a browser on Windows; C# web on Windows and macOS: UNTRIED (WINDOWS-TESTS.md:88, :96, :104-110).
- The C# program driven live in tests: NOT BUILT (`everything_live.rs` has go, rust, c, cpp; DESIGN.md:272).
- Everything in the browser (database in the browser, offline tool): NOT BUILT, later and only on request (DESIGN 7.6 open choices).

## Conflicts
- DESIGN.md :57-58, :64, :1746, :1750, :2025-2026 describe the web renderer, axum and ASP.NET as what web programs serve; every web program serves Clarity.UI's browser files.
- DESIGN 7.8 (G6, `@clarity/ui` built into apps, `windowMarkup.ts`) is written as current; `runtimes/web` serves only the IDE.
- `crates/clarity-project/src/lib.rs:876-878` speaks of the shared web screen.
- Port 8080 is written twice: `crates/clarity-project/src/lib.rs:285`, `app/src/lib/ProjectSettings.svelte:369` (equal).
- WINDOWS-TESTS.md:88 treats C# in the browser as missing; it is built.

## Decisions
- 2026-10-27: the web target serves the application at the project's port; each tab runs its own application against the server's database (W1).
- 2026-10-28: C# keeps live view models on the server (W2); its `.NET` browser program was replaced (B2, 2026-11-03) by Clarity.UI's browser files answered through `ProgramWire`.
- 2026-11-01: no web tools (npm, Vite) in an application; windows are markup read at run time. User: "npm tends to get blocked on windows very often!!!"
- 2026-11-03: Clarity.UI runs in the browser, compiled once for every language; C# with desktop + web writes two programs, the others one.

## Was
- DESIGN 7.3, DESIGN 7.6 (replaced), DESIGN 7.8 (replaced); roadmap W1, W2 (W2a-W2f), G6 (G6a-G6d), M10
