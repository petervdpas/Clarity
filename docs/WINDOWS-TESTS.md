# Windows tests

What to try on Windows after the work of 2026-10-25, which was built and tested on Linux only. Most important first. Tick a line when it holds; note what did not.

## 1. Compile first

These were never built on Windows.

- [x] The C and C++ shells: while the databases are not open, the title bar's close of the main window goes to the screen. On Windows this replaces the window procedure (`SetWindowLongPtrW`, `WM_CLOSE`), in the `_WIN32` branch of `runtimes/c/shell/shell.c` and `runtimes/cpp/shell/shell.cpp`. Build each runtime's tests, then `cargo test -p clarity-gen --test secrets an_unconnected_app_builds_in_c_and_cpp -- --ignored`.
  - 2026-10-25: the generated C and C++ apps build (with MSYS2 UCRT64 first on PATH and `CMAKE_PREFIX_PATH=C:/msys64/ucrt64`; Strawberry's MinGW has no SQLite). The C and C++ runtimes' own tests pass, without warnings, after fixes for Windows: the tests clear folders and set file times without `rm`, `timegm` or `utimes`; the settings test keeps its settings in a temp folder, not the user's `%APPDATA%`; the C++ file-store tests close a file before removing it; and the C runtime uses MinGW's C99 printf (`__USE_MINGW_ANSI_STDIO`), so `%zu` prints and checks.
- [x] Every stack's application with `on unconnected`: `cargo test -p clarity-gen --test secrets -- --ignored`. Rust, Go and C# build on a Postgres connection kept as a secret setting.

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
- [x] `%APPDATA%\Program\settings.json` keeps only what differs, as `keys`. Only tried in a project with a `settings` block (the secret).
- [x] On Avalonia the same, and the file shows `Ctrl+K` as written.
- [x] A toolbar or ribbon button's tooltip shows its key (it used to say "Opens Window"): Print showed "Ctrl+P" on Go, Rust and Avalonia.

## 6. Not done, so not to be tested

- On macOS, Ctrl in a generated application's keys is the Control key, not Cmd.
- A view that brings its own ribbon tab and menu items (the next part of DESIGN 5.28).
- An older bug: on Avalonia, a spin box or number input bound to a window's own number (not a record's field) does not compile.

## Releases

- [ ] Upload NuGet 0.38.0: the six packages in each project's `bin/Release`.
- [x] Check that the Rust crate 0.27.0 is on crates.io; its release workflow was still running when last looked at. (On crates.io, 2026-10-25.)
- Go v0.30.0 is released and its sums are recorded.
