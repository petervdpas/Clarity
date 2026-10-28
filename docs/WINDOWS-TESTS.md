# Windows tests

What to try on Windows after the work of 2026-10-25, which was built and tested on Linux only. Most important first. Tick a line when it holds; note what did not.

## 1. Compile first

These were never built on Windows.

- [x] The C and C++ shells: while the databases are not open, the title bar's close of the main window goes to the screen. On Windows this replaces the window procedure (`SetWindowLongPtrW`, `WM_CLOSE`), in the `_WIN32` branch of `runtimes/c/shell/shell.c` and `runtimes/cpp/shell/shell.cpp`. Build each runtime's tests, then `cargo test -p clarity-gen --test secrets an_unconnected_app_builds_in_c_and_cpp -- --ignored`.
  - 2026-10-25: the generated C and C++ apps build (with MSYS2 UCRT64 first on PATH and `CMAKE_PREFIX_PATH=C:/msys64/ucrt64`; Strawberry's MinGW has no SQLite). The C and C++ runtimes' own tests pass, without warnings, after fixes for Windows: the tests clear folders and set file times without `rm`, `timegm` or `utimes`; the settings test keeps its settings in a temp folder, not the user's `%APPDATA%`; the C++ file-store tests close a file before removing it; and the C runtime uses MinGW's C99 printf (`__USE_MINGW_ANSI_STDIO`), so `%zu` prints and checks.
- [x] Every stack's application with `on unconnected`: `cargo test -p clarity-gen --test secrets -- --ignored`. Rust, Go and C# build on a Postgres connection kept as a secret setting.
- [x] Not asked, seen 2026-10-25: `dotnet test Clarity.Runtime.Tests` fails 7 of 91 on Windows, every one in EngineTests with "it ran longer than 1 second" (the WASM engine's limit), also run alone. The rest pass.
  - They pass on Linux. A lead: in `Engine.CallIn` (runtimes/csharp/Clarity.Runtime.Engines/Engine.cs) the one-second timer starts before `Instance()`, so making the instance (and its `on open`) counts toward the call's second. If that is slow on Windows, the call stops at its first check. To see: time `Instance()` and `Invoke` apart in one failing test; if the instance is the slow part, start the clock after it (and give `on open` its own second).
  - 2026-10-28: not the timer. By then 13 of 16 in EngineTests and ScreenEngineTests failed, each within 50 ms. The trap came from `Linker.Instantiate`: a new Wasmtime store's epoch deadline is the epoch it starts at, and only `Invoke` set it, so the module's start code stopped at once. Fixed: `Instance()` sets the deadline when it makes the store. All 90 pass on Windows (3 skipped). Only the C# runtime uses epochs. Needs a NuGet release (Clarity.Runtime.Engines) before generated applications have it.

## 2. When the databases do not open (DESIGN 5.27)

Per stack, with a Postgres database whose connection is a secret setting (a Postgres in Docker will do).

2026-10-25, run from `dev.bat` (Run in Preview) on Go, Rust, C# Avalonia, C and C++, with a Postgres 17 in Docker. C and C++ keep SQLite only, so for them the database file's folder was made unopenable and the app had `on unconnected` (without it the C app stops at the start with "storage: cannot make the folder ..."). C and C++ built in Clarity only with MSYS2 UCRT64 first on PATH and `CMAKE_PREFIX_PATH=C:/msys64/ucrt64` set before `dev.bat`.

- [x] No secret kept yet: the window that keeps the connection opens, saying "The database connection is not set yet". Go, Rust, Avalonia.
- [x] The server is down: it says "The database did not open: ..." with the database's own message. Go (the full "actively refused" text), Rust ("cannot connect to PostgreSQL: error connecting to server"), Avalonia ("Failed to connect to 127.0.0.1:5433"); C ("cannot make the folder ...") through `on unconnected`.
- [x] The title bar's close (X or Alt+F4) after changing the connection: it connects and the main window opens. Go (X and Alt+F4), Rust (X), Avalonia (X and Alt+F4), C and C++ (X, through the window procedure).
- [x] The title bar's close with nothing changed: the application ends. Go, Rust, Avalonia, C.
- [x] Once connected, the title bar's close ends the application as usual. Go, Rust, Avalonia, C.
- [x] Avalonia, the same; its close goes through its own shell, as before.
- [ ] The secret is in the Credential Manager (`Program:connDb`), not in `%APPDATA%\Program\settings.json`.
  - 2026-10-25: never in settings.json on any stack. Go and C# keep it as `wintest:shopDb` / `Wintest:shopDb`, but Rust keeps it as `shopDb.wintest`: `keyring::Entry::new(program, name)` (runtimes/rust/src/secrets.rs) uses keyring's default target; `Entry::new_with_target(&format!("{program}:{name}"), program, name)` would match. And the blob differs: C# (`StringToCoTaskMemUni`, Secrets.cs) writes UTF-16, C and C++ write and read UTF-8 bytes, so after the C# app kept the connection the C app read it as `p` and the C++ app with a zero byte between letters. Each stack reads back its own; switching a project's stack loses or garbles the kept secret. (keyring-rs also writes UTF-16 on Windows, as far as known; Go's blob was not checked.)
  - Fixed on Linux the same day: every stack keeps `Program:name` with the value's UTF-8 bytes, as Go (go-keyring), C and C++ did; Rust (crate 0.28.0) and C# (NuGet 0.39.0) changed, checked by conformance/secret-store.json. To try again: keep the connection in one stack's app and open it in another's (C# then C, Rust then Go). A secret the old C# or Rust kept is not found or garbled once: the app asks for it again.
  - Tried again 2026-10-25 on Windows (C# on the local runtime with `CLARITY_RUNTIME`, Rust on crates.io 0.28.0). The runtimes' own secret tests pass on the Credential Manager (`CLARITY_SECRET_STORE=1`) on every stack, after the settings tests also set `APPDATA` (they set only `XDG_CONFIG_HOME`, so on Windows Rust's, C#'s and Go's failed and C's and C++'s wrote into the user's `%APPDATA%`). Reading across stacks works: C and Go read "Password=pä€ from C#" as C# kept it, and Go read what Rust kept, `ä€` and all; Rust and then Go wrote the one entry `wintest:shopDb`.
  - Still wrong: the program's name differs per stack. For the project `Wintest`, C# keeps `Wintest:shopDb` and Go, Rust, C and C++ `wintest:shopDb`; for `ShopDesk`, Rust, C and C++ say `shop_desk`, Go `shopdesk` and C# `ShopDesk` (and C++ `ShopDesk` in places). The Credential Manager reads a target whatever its case but writes one apart for each case: after C# kept the connection, C and Go wrote their changes to `wintest:shopDb` beside it, without an error, and every app went on reading C#'s `Wintest:shopDb`. With a name of two words the stacks do not even read each other's secret, nor share the settings folder. The program name wants to be one form on every stack (in conformance/secret-store.json too).
  - Fixed the same day: every stack, desktop and command-line program alike, keeps its settings folder and its secrets under the project's name in snake case (`clarity_gen::settings_program`: `Wintest` is `wintest`, `ShopDesk` `shop_desk`), listed in conformance/secret-store.json (`programs`) and checked by `clarity-gen/tests/secrets.rs` on every stack. Go (`shopdesk`) and C# (`ShopDesk`) changed, and the command-line secrets of Rust and C (`shop-desk`). An application built before keeps its settings and secrets under its old name: they are asked for again once. To try: keep a secret in a C# app, then change it in a C or Go app; the C# app shows the change.
  - Also seen: keyring-rs (Rust) writes with enterprise persistence, the other stacks with local machine. A write by either replaces the other's, but an enterprise credential roams with the account: a Rust app's secret, forgotten with `cmdkey /delete`, came back later with its old value. Fixed the same day: the Rust runtime writes the Credential Manager through Win32 with local-machine persistence, as the other stacks, and keeps keyring for macOS and Linux; conformance/secret-store.json says `persist`, and `runtimes/rust/tests/secrets.rs` reads a kept credential back to check it. Needs a release of the crate (0.28.1) before generated applications have it.

## 3. Showing and copying a secret

- [x] A password input's eye shows what is typed and hides it again, on the web stacks (WebView2) and on Avalonia. Go, Rust, Avalonia, C++ (2026-10-25).
- [x] Its copy button puts the text on the clipboard. In WebView2 the clipboard API may be refused; the code then copies a selection instead. Go, Rust, Avalonia: the clipboard held the input's text.
- [ ] The same buttons on the vault's fields in Clarity.
  - 2026-10-25: eye and copy work on the master password fields of a vault not made yet; the fields inside an open vault not tried (no vault was made).

## 4. Clarity's own keys (the IDE)

- [x] WebView2's own keys do not get in the way: F5 and Ctrl+R reload a page, Ctrl+P prints, Ctrl+F finds. F5 must run Reload (Project) or Run (Preview), not reload Clarity.
  - 2026-10-25: at first Ctrl+P opened WebView2's print preview, Ctrl+F its find bar, and Ctrl+R reloaded Clarity's page: `onkeydown` in App.svelte only stops a key that is a Clarity command. Fixed the same day: Clarity's main window turns off WebView2's browser keys (`AreBrowserAcceleratorKeysEnabled`, app/src-tauri/src/browser_keys.rs). Tried again: Ctrl+P, Ctrl+F, Ctrl+R, F5 and Alt+Left do nothing in the Vault view; Ctrl+A in a field, Ctrl+1 and F7 still work. The manual's window keeps its browser keys (find in the page, print a page).
- [x] F7 (Check), Ctrl+Shift+B (Build), Shift+F5 (Stop), Alt+Left (Back), Alt+Home (Main window).
- [x] Ctrl+1 to Ctrl+4 (the views), Ctrl+O, Ctrl+Shift+N, Ctrl+, (Clarity settings).
- [ ] The Vault: Ctrl+N (Add, or New connection), F2, Delete, Ctrl+Shift+C, Ctrl+L. Not tried: no vault was made.
- [ ] AltGr on a European layout is Ctrl+Alt: typing `@`, `€` or `{` in a text field must not run a command. Not tried: this machine has a US layout only. AltGr arrives as `Ctrl+Alt+<char>` and no default key uses Ctrl+Alt.
- [x] Clarity settings, Keys: change a key, swap one taken in the same view, Reset, Reset all; the keys hold after a restart.
- [x] F1: the manual opens quickly after the first time, on the page of the view, and its close only hides it. Project gave "The project file", Design "Windows".

## 5. Keys in a generated application

A project with `keys editable`, a menu item `on click -> keys`, a toolbar or ribbon button with a key, and a window button with a key.

2026-10-25: Go and Rust in full, Avalonia in full, C and C++ the window's and toolbar's keys.

- [x] A menu item's key works while the menu is shut; a ribbon button's key whichever tab is open.
- [x] A window's own button with the frame's key goes first (Ctrl+H on both).
- [x] The keys window: Change takes the new key; a key taken in the same place offers a swap; a refused key says why; Reset and Reset all. Also the other place's note: "Ctrl+Q is also the key of Hello, elsewhere; here Save goes first" (Avalonia). On Avalonia the keys window grows and shrinks with its message line, so its buttons move under the mouse.
  - Fixed the same day, on Avalonia and the web: the message line is always there, the window has a fixed width, and "Press the new key" shows in that line (the waiting command in bold), not in its key column. To try again: Change, a refused key, a swap; nothing moves.
  - Tried again 2026-10-25 on Avalonia and on the web (C): "Press the new key", a refused key and a swap offer each show in the message line, the waiting command in bold, and no row or button moves.
- [x] `%APPDATA%\Program\settings.json` keeps only what differs, as `keys`. Only tried in a project with a `settings` block (the secret).
- [x] On Avalonia the same, and the file shows `Ctrl+K` as written.
- [x] A toolbar or ribbon button's tooltip shows its key (it used to say "Opens Window"): Print showed "Ctrl+P" on Go, Rust and Avalonia.

## 5b. An engine beside the screen: pointer, keys, tells and hears (N5a, 2026-10-26)

Built and tried on Linux only. `cargo test -p clarity-gen --test go_build -- --ignored --exact an_engine_beside_the_screen_builds_in_go` and `--test csharp_build ... an_engine_beside_the_screen_builds_in_csharp`, with `CLARITY_KEEP=1` to keep the app in `target/generated/clarity-gen-go-play` and `clarity-gen-csharp-play`.

2026-10-28 on Windows: both build tests pass, and both apps were run (Avalonia's on the local runtime, so with the engine fix of section 1). Keys were held down with `keybd_event`, so that the window could be looked at while they were down.

- [x] Go (WebView2, a Web Worker from a `blob:` URL): the dot moves across the canvas and the time under it counts up.
- [x] Avalonia (Wasmtime): the same. If the canvas says "The engine stopped: ... it ran longer than ... second" at once, the app has a Clarity.Runtime.Engines from before the fix in section 1 (2026-10-28).
- [x] Closing the window ends the engine (no leftover process).
- [x] Pointer (both): press on the canvas and drag; the dot follows. Let go beside the window: the dot stays at the canvas's edge.
- [ ] Keys (both): click the canvas, hold Left and A; the text says "holding ArrowLeft a". Let go of Left: "holding a". Click elsewhere (or Alt+Tab) while holding A: "holding" goes away. A shortcut with Ctrl still reaches the window.
  - 2026-10-28 on Windows: all of it on Go and Avalonia, but for the Ctrl shortcut: the demo has none to see.
- [x] Tells (both): the first number under the canvas counts the laps, one every 1.6 seconds. Click the canvas and press Escape: the second number shows the time.
- [x] Hears (both): Start sets the text to "... ms, level 2", the time back near 0 and the laps to 0.
- [ ] Log (both): run the app from a terminal; each Start writes `engine Screen: level 2` there.
  - 2026-10-28 on Windows: Go writes it in PowerShell. Avalonia's app is a Windows GUI program, so PowerShell does not wait for it nor show what it writes; with its standard error redirected to a file, the line is there.
- [x] The web target on Windows (Winsock; the C and C++ clarity_service link ws2_32 and shell32; Rust's files by include_bytes!): `the_web_target_serves_in_c`, `_in_cpp` (cpp_build) and `_in_rust` (build) pass; the invaders example (Go): `the_invaders_example_serves_in_go` with `CLARITY_KEEP=1`, run it, open http://127.0.0.1:8080 in a browser and play.
  - 2026-10-28 on Windows: the four tests pass (MSYS2 UCRT64 first on PATH for C and C++). The invaders play in a browser: Start, a shot, the score.
- [ ] Avalonia in the browser, the browser's own (C#): in the web todo the font is Inter, the tab takes the window's title, a file field downloads; a setting changed survives a reload; a sound part plays; a game with an engine beside the screen plays.
  - 2026-10-28 on Windows, in the desktop app's browser pane (Chromium), on NuGet 0.51.0: the todo (switched to `stack csharp avalonia web`) has Inter, the tab says Todo and then Users when that window opens, and the Dark theme survives a reload. The pills, switched to C#, play (the canvas is empty until Start, where Go shows the paddle and ball). The records, switched to C#, open. Not tried: a file field's download (it needs a file stored first, through the system's file picker) and a sound part (the dice, the one example with one, has a part written in Rust).
- [ ] Avalonia in the browser (C#): `the_walker_example_serves_in_csharp` passes (wasm-tools installed); `dotnet build Server` in its `out/web`, run the server, open http://127.0.0.1:8080, press Walk: the figure walks across and back. The IDE's Install for the WebAssembly tools asks with UAC. As a person who is not an administrator, and from a folder with a space in its path (W2e): the server starts without "Access is denied", and `--address 0.0.0.0:8080` lets another machine open it (Windows Firewall asks once).
  - 2026-10-28 on Windows: `the_walker_example_serves_in_csharp` passes, with the wasm-tools workload installed from the command line. Generated into `target/generated/web apps/walker` (a space in the path), the server builds and runs and the figure walks. Generated into a deeper folder, the browser program does not publish: "Could not copy ... webcil\...wasm because it was not found", its paths being some 270 characters, past Windows' 260. Not tried: the IDE's Install (UAC), a person who is not an administrator, `--address 0.0.0.0:8080` (the firewall's question).
- [x] SVG pictures (C#): `cargo test -p clarity-gen --test csharp_build -- --ignored --exact pictures_build_in_csharp` builds; an app with `image source "logo.svg"` shows it.
  - 2026-10-28 on Windows: `pictures_build_in_csharp` passes, and an image with `source "logo.svg"` shows in a desktop Avalonia app (at the picture's own size, past its cell).
- [x] The pills example (Go): `the_pills_example_builds_and_runs_in_go` with `CLARITY_KEEP=1`; run it: the pills and the paddle are pictures, Start plays, the mouse moves the paddle.
  - 2026-10-28 on Windows: the test passes, and the game plays: pictures, Start, the paddle follows the mouse, the score counts.
- [x] Written in Clarity (Go): `cargo test -p clarity-gen --test go_build -- --ignored --exact a_clarity_engine_beside_the_screen_builds_in_go` with `CLARITY_KEEP=1`, then run `target/generated/clarity-gen-go-sky/skies.exe`: the dot sits left; Start moves it a little right; ten seconds later the number under it shows the time.
  - 2026-10-28 on Windows: the test passes, and with Start pressed within ten seconds it is as above. Pressed later, Start is lost: past ten seconds the engine tells `finished` at every frame, so a handler is nearly always running, and `run` in app/src/runtime/Runtime.svelte drops a click that comes while one runs (`if (!current || busy) return false`), where a tell waits its turn. Not for Windows only.
  - Fixed the same day: `run` waits its turn instead, ahead of what parts tell (`told` waits while a person's event does); crates/clarity-gen/ui/runtime.js rebuilt (`npm run build:runtime`). Tried again: Start pressed past ten seconds moves the dot and starts the time again. Also fixed: three app tests (dialogs.test.ts, fileNames.test.ts) read `new URL(...).pathname`, which is `/E:/...` on Windows; they use `fileURLToPath` now, and `npm test` passes on Windows.
- [ ] Clarity.Avalonia.Desktop (C#, W2e): `the_todo_example_builds_and_runs_in_csharp` and `the_noughts_example_builds_and_runs_in_csharp` pass; a desktop app opens its window, plays a sound part, and `quit` and `keys` work (they go through `UseClarityDesktop()` now). On macOS too.
  - 2026-10-28 on Windows: both tests pass (the todo once failed with a build error in a full run and passed alone; the error text was lost). A todo with `keys editable`, a keys menu item and a sound part, on NuGet 0.51.0: the window opens, the keys window opens and changes Ding to Ctrl+K (kept in `%APPDATA%\todo\settings.json` as `keys`), and Ctrl+Q quits. The sound's button raised no error, but nobody listened.
- [ ] Avalonia in the browser, Release (W2e): in a generated `out/web`, `dotnet build Server -c Release` (trims the browser program), run `Server/bin/Release/net10.0/<Name>.Server.dll`, open it in Edge: the windows show with the theme, a second window opens, and the browser's network tab shows `content-encoding: br` on the `.wasm` files.
  - 2026-10-28 on Windows, the walker: the Release server sends `Avalonia.Base.*.wasm` with `Content-Encoding: br` (464 KB for 1.67 MB), and the trimmed program walks in the browser pane (Chromium, not Edge itself). The second window and the theme were seen in the Debug todo above, not in Release.
- [x] Engines written in Clarity on the backend: `an_engine_written_in_clarity_runs_in_{go,c,cpp,csharp}` and `an_interpreted_engine_runs_in_{go,c,cpp,csharp}` (ignored build tests) pass on Windows.
  - 2026-10-28 on Windows: all eight pass. The C++ and C# tests at first failed only when removing their build folder ("being used by another process", WebView2's unpacked files or dotnet's build servers); they now retry the removal for up to ten seconds (`remove` in cpp_build.rs and csharp_build.rs).

## 6. Not done, so not to be tested

- On macOS, Ctrl in a generated application's keys is the Control key, not Cmd.
- A view that brings its own ribbon tab and menu items (the next part of DESIGN 5.28).
- An older bug: on Avalonia, a spin box or number input bound to a window's own number (not a record's field) does not compile.

## Releases

- [x] Upload NuGet 0.38.0 (on nuget.org).
- [x] NuGet 0.39.0 and 0.40.0 are on nuget.org.
- [x] NuGet 0.41.0 and 0.42.0 are on nuget.org.
- [x] The Rust crate 0.28.1 is on crates.io.
- C and C++ need no release: their files are copied into each app.
- [x] Check that the Rust crate 0.27.0 is on crates.io; its release workflow was still running when last looked at. (On crates.io, 2026-10-25.)
- [x] NuGet 0.43.0, crate 0.29.0 and Go v0.32.0 (sums recorded) are released: engines written in Clarity are WebAssembly now, so the runtimes' `random` and C's keep helpers went.
- [x] Tag Go runtime v0.33.0 and crate 0.30.0: the web target's server (shell.Serve, clarity_runtime::serve); sums recorded.
- [ ] Tag Go runtime v0.34.0 (package desktop apart from shell, Address takes the project's address) and crate 0.31.0 (serve::address takes it); record the Go sums after the tag.
- [x] Upload NuGet 0.44.0 (SVG pictures in Clarity.Avalonia, the canvas's image shape).
- [x] Upload NuGet 0.49.0: seven packages now, `Clarity.Avalonia.Desktop` new (the desktop's own, apart from Clarity.Avalonia).
- [x] Upload NuGet 0.50.0: the web server sends compressed files.
- [x] Upload NuGet 0.51.0: the web server over a plain socket (Windows), and the icon with the sparks on every package.
- [ ] A NuGet release with the engine fix of 2026-10-28 (section 1): 0.51.0 does not have it.
- [x] Release the VS Code extension (0.1.16 on the Marketplace; Open VSX has no token): its grammar knows record, tell, beside, engines, const and picture.
