# Windows tests

What to try on Windows after the work of 2026-10-25, which was built and tested on Linux only. Most important first. Tick a line when it holds; note what did not.

C# now runs on Clarity.UI, Clarity's own UI framework, in place of the UI framework it had before. The C# results below were seen on that earlier UI; the open C# steps that only applied to it are gone.

## 2. When the databases do not open (DESIGN 5.27)

Per stack, with a Postgres database whose connection is a secret setting (a Postgres in Docker will do).

2026-10-25, run from `dev.bat` (Run in Preview) on Go, Rust, C#, C and C++, with a Postgres 17 in Docker. C and C++ keep SQLite only, so for them the database file's folder was made unopenable and the app had `on unconnected` (without it the C app stops at the start with "storage: cannot make the folder ..."). C and C++ built in Clarity only with MSYS2 UCRT64 first on PATH and `CMAKE_PREFIX_PATH=C:/msys64/ucrt64` set before `dev.bat`.

- [ ] The secret is in the Credential Manager (`Program:connDb`), not in `%APPDATA%\Program\settings.json`.
  - 2026-10-25: never in settings.json on any stack. Go and C# keep it as `wintest:shopDb` / `Wintest:shopDb`, but Rust keeps it as `shopDb.wintest`: `keyring::Entry::new(program, name)` (runtimes/rust/src/secrets.rs) uses keyring's default target; `Entry::new_with_target(&format!("{program}:{name}"), program, name)` would match. And the blob differs: C# (`StringToCoTaskMemUni`, Secrets.cs) writes UTF-16, C and C++ write and read UTF-8 bytes, so after the C# app kept the connection the C app read it as `p` and the C++ app with a zero byte between letters. Each stack reads back its own; switching a project's stack loses or garbles the kept secret. (keyring-rs also writes UTF-16 on Windows, as far as known; Go's blob was not checked.)
  - Fixed on Linux the same day: every stack keeps `Program:name` with the value's UTF-8 bytes, as Go (go-keyring), C and C++ did; Rust (crate 0.28.0) and C# (NuGet 0.39.0) changed, checked by conformance/secret-store.json. To try again: keep the connection in one stack's app and open it in another's (C# then C, Rust then Go). A secret the old C# or Rust kept is not found or garbled once: the app asks for it again.
  - Tried again 2026-10-25 on Windows (C# on the local runtime with `CLARITY_RUNTIME`, Rust on crates.io 0.28.0). The runtimes' own secret tests pass on the Credential Manager (`CLARITY_SECRET_STORE=1`) on every stack, after the settings tests also set `APPDATA` (they set only `XDG_CONFIG_HOME`, so on Windows Rust's, C#'s and Go's failed and C's and C++'s wrote into the user's `%APPDATA%`). Reading across stacks works: C and Go read "Password=pä€ from C#" as C# kept it, and Go read what Rust kept, `ä€` and all; Rust and then Go wrote the one entry `wintest:shopDb`.
  - Still wrong: the program's name differs per stack. For the project `Wintest`, C# keeps `Wintest:shopDb` and Go, Rust, C and C++ `wintest:shopDb`; for `ShopDesk`, Rust, C and C++ say `shop_desk`, Go `shopdesk` and C# `ShopDesk` (and C++ `ShopDesk` in places). The Credential Manager reads a target whatever its case but writes one apart for each case: after C# kept the connection, C and Go wrote their changes to `wintest:shopDb` beside it, without an error, and every app went on reading C#'s `Wintest:shopDb`. With a name of two words the stacks do not even read each other's secret, nor share the settings folder. The program name wants to be one form on every stack (in conformance/secret-store.json too).
  - Fixed the same day: every stack, desktop and command-line program alike, keeps its settings folder and its secrets under the project's name in snake case (`clarity_gen::settings_program`: `Wintest` is `wintest`, `ShopDesk` `shop_desk`), listed in conformance/secret-store.json (`programs`) and checked by `clarity-gen/tests/secrets.rs` on every stack. Go (`shopdesk`) and C# (`ShopDesk`) changed, and the command-line secrets of Rust and C (`shop-desk`). An application built before keeps its settings and secrets under its old name: they are asked for again once. To try: keep a secret in a C# app, then change it in a C or Go app; the C# app shows the change.
  - Also seen: keyring-rs (Rust) writes with enterprise persistence, the other stacks with local machine. A write by either replaces the other's, but an enterprise credential roams with the account: a Rust app's secret, forgotten with `cmdkey /delete`, came back later with its old value. Fixed the same day: the Rust runtime writes the Credential Manager through Win32 with local-machine persistence, as the other stacks, and keeps keyring for macOS and Linux; conformance/secret-store.json says `persist`, and `runtimes/rust/tests/secrets.rs` reads a kept credential back to check it. Needs a release of the crate (0.28.1) before generated applications have it.

## 3. Showing and copying a secret

- [ ] The same buttons on the vault's fields in Clarity.
  - 2026-10-25: eye and copy work on the master password fields of a vault not made yet; the fields inside an open vault not tried (no vault was made).

## 4. Clarity's own keys (the IDE)

- [ ] The Vault: Ctrl+N (Add, or New connection), F2, Delete, Ctrl+Shift+C, Ctrl+L. Not tried: no vault was made.
- [ ] AltGr on a European layout is Ctrl+Alt: typing `@`, `€` or `{` in a text field must not run a command. Not tried: this machine has a US layout only. AltGr arrives as `Ctrl+Alt+<char>` and no default key uses Ctrl+Alt.

## 5b. An engine beside the screen: pointer, keys, tells and hears (N5a, 2026-10-26)

Built and tried on Linux only. `cargo test -p clarity-gen --test go_build -- --ignored --exact an_engine_beside_the_screen_builds_in_go` and `--test csharp_build ... an_engine_beside_the_screen_builds_in_csharp`, with `CLARITY_KEEP=1` to keep the app in `target/generated/clarity-gen-go-play` and `clarity-gen-csharp-play`.

2026-10-28 on Windows: both build tests pass, and both apps were run (C#'s on the local runtime, so with the engine fix of 2026-10-28). Keys were held down with `keybd_event`, so that the window could be looked at while they were down.

- [ ] Keys (both): click the canvas, hold Left and A; the text says "holding ArrowLeft a". Let go of Left: "holding a". Click elsewhere (or Alt+Tab) while holding A: "holding" goes away. A shortcut with Ctrl still reaches the window.
  - 2026-10-28 on Windows: all of it on Go and C#, but for the Ctrl shortcut: the demo has none to see.
- [ ] Log (both): run the app from a terminal; each Start writes `engine Screen: level 2` there.
  - 2026-10-28 on Windows: Go writes it in PowerShell. The C# app is a Windows GUI program, so PowerShell does not wait for it nor show what it writes; with its standard error redirected to a file, the line is there.
- [ ] C# in the browser (`crates/clarity-gen/src/csharp/web.rs`): the walker example's web target, opened in a browser on Windows, draws its window and answers clicks and keys.
- [ ] C# on Clarity.UI (G7): `the_todo_example_builds_and_runs_in_csharp` passes on Windows and macOS, and the generated program opens its main window with the window's title and size.

## 5c. One Clarity.UI for every language, WYSIWYG and screen readers (2026-11-04)

Built and tried on Linux only. The released library is 0.20.0 (GitHub release `runtimes/rust-ui-ffi/v0.20.0`), C# 0.92.0.

- [ ] The library's own tests: `cargo test -p clarity-ui` passes (the `windows` test binary holds the screen reader tests, `access.rs`).
- [ ] A generated program per language opens its main window, drawn by Clarity.UI: Rust, Go, C, C++ and C# (the todo example, or any other). Go, C and C++ fetch `clarity-ui-0.20.0-win-x64.tar.gz` themselves when `CLARITY_UI` is not set; check the build finds `clarity_ui.dll` and the program starts with it beside the exe.
- [ ] The same program's web target, opened in Edge: the window draws in the page (the browser files from `clarity-ui-0.20.0-browser.tar.gz`), clicks and typing work.
- [ ] Narrator (Ctrl+Win+Enter) in a desktop program: Tab through the parts. A button says its caption, an input its label and value, a check box whether it is ticked, a password field never its text. A status message or alert is read when it appears.
- [ ] Narrator in Edge on the web target: the same, read from the page's hidden mirror.
- [ ] The IDE (WebView2): the preview and the design canvas draw with Clarity.UI from the localhost preview server (127.0.0.1); moving a part in the designer shows in the preview.
- [ ] Row height: `row height 30` in a project's file (or the Project tab) makes rows lower in the designer and in the running program alike.

## 6. Not done, so not to be tested

- On macOS, Ctrl in a generated application's keys is the Control key, not Cmd.
- A view that brings its own ribbon tab and menu items (the next part of DESIGN 5.28).

## After releases

- [ ] Paging on Windows: run `cargo test -p clarity-gen --test http_build -- --ignored` (each stack reads every page of a local server).
- [ ] Folder settings on Windows and macOS (X2): the runtimes' tests (`cargo test --test settings --test pictures` in runtimes/rust, `go test ./clarity`, `dotnet test Clarity.Runtime.Tests`, `ctest` for C and C++) pass, the Windows and macOS cases of conformance/settings.json `folders` included.
- [ ] The folder part's button opens the system's folder picker on Windows and macOS, on every desktop stack: since every language draws with the one Clarity.UI library, that is its file picker (COM on Windows, osascript on macOS), only compiled on Linux so far. A folder chosen there is where the next saved picture of a store with `"setting:name"` goes.
- [ ] Progress and Stop on Windows and macOS: the runtimes' progress tests (`cargo test --features engines --test progress` in runtimes/rust, `go test ./engine`, `dotnet test Clarity.Runtime.Tests --filter ProgressTests`, `ctest` for C and C++) pass.
- [ ] A long engine call on Windows and macOS, on every desktop stack (all on the one Clarity.UI library): the every-part project's Plan button, or an engine whose call takes seconds, shows the bar above the messages and keeps painting; Stop ends it with "it was stopped". The Go, C and C++ layers' watch threads (`GetCurrentThreadId` in Go) have only been built on Linux.
- [ ] Go, C and C++ on Clarity.UI (round L) on Windows and macOS: with `CLARITY_UI` set to a folder holding `include/clarity_ui.h` and the system's library in `lib/`, `go test ./...` in runtimes/go and `ctest` for runtimes/c and runtimes/cpp pass, and a generated `stack go desktop`, `stack c desktop` and `stack cpp desktop` program opens its window (the library found beside the program: rpath `@loader_path` / `@executable_path` on macOS, the DLL beside the program on Windows; macOS runs the window on the main thread).
