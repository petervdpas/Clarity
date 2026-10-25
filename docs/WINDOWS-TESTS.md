# Windows tests

What to try on Windows after the work of 2026-10-25, which was built and tested on Linux only. Most important first. Tick a line when it holds; note what did not.

## 1. Compile first

These were never built on Windows.

- [ ] The C and C++ shells: while the databases are not open, the title bar's close of the main window goes to the screen. On Windows this replaces the window procedure (`SetWindowLongPtrW`, `WM_CLOSE`), in the `_WIN32` branch of `runtimes/c/shell/shell.c` and `runtimes/cpp/shell/shell.cpp`. Build each runtime's tests, then `cargo test -p clarity-gen --test secrets an_unconnected_app_builds_in_c_and_cpp -- --ignored`.
- [ ] Every stack's application with `on unconnected`: `cargo test -p clarity-gen --test secrets -- --ignored`. Rust, Go and C# build on a Postgres connection kept as a secret setting.

## 2. When the databases do not open (DESIGN 5.27)

Per stack, with a Postgres database whose connection is a secret setting (a Postgres in Docker will do).

- [ ] No secret kept yet: the window that keeps the connection opens, saying "The database connection is not set yet".
- [ ] The server is down: it says "The database did not open: ..." with the database's own message.
- [ ] The title bar's close (X or Alt+F4) after changing the connection: it connects and the main window opens. Wails, Tauri, and C and C++ through the window procedure.
- [ ] The title bar's close with nothing changed: the application ends.
- [ ] Once connected, the title bar's close ends the application as usual.
- [ ] Avalonia, the same; its close goes through its own shell, as before.
- [ ] The secret is in the Credential Manager (`Program:connDb`), not in `%APPDATA%\Program\settings.json`.

## 3. Showing and copying a secret

- [ ] A password input's eye shows what is typed and hides it again, on the web stacks (WebView2) and on Avalonia.
- [ ] Its copy button puts the text on the clipboard. In WebView2 the clipboard API may be refused; the code then copies a selection instead.
- [ ] The same buttons on the vault's fields in Clarity.

## 4. Clarity's own keys (the IDE)

- [ ] WebView2's own keys do not get in the way: F5 and Ctrl+R reload a page, Ctrl+P prints, Ctrl+F finds. F5 must run Reload (Project) or Run (Preview), not reload Clarity.
- [ ] F7 (Check), Ctrl+Shift+B (Build), Shift+F5 (Stop), Alt+Left (Back), Alt+Home (Main window).
- [ ] Ctrl+1 to Ctrl+4 (the views), Ctrl+O, Ctrl+Shift+N, Ctrl+, (Clarity settings).
- [ ] The Vault: Ctrl+N (Add, or New connection), F2, Delete, Ctrl+Shift+C, Ctrl+L.
- [ ] AltGr on a European layout is Ctrl+Alt: typing `@`, `€` or `{` in a text field must not run a command.
- [ ] Clarity settings, Keys: change a key, swap one taken in the same view, Reset, Reset all; the keys hold after a restart.
- [ ] F1: the manual opens quickly after the first time, on the page of the view, and its close only hides it.

## 5. Keys in a generated application

A project with `keys editable`, a menu item `on click -> keys`, a toolbar or ribbon button with a key, and a window button with a key.

- [ ] A menu item's key works while the menu is shut; a ribbon button's key whichever tab is open.
- [ ] A window's own button with the frame's key goes first.
- [ ] The keys window: Change takes the new key; a key taken in the same place offers a swap; a refused key says why; Reset and Reset all.
- [ ] `%APPDATA%\Program\settings.json` keeps only what differs, as `keys`, also in a project without a `settings` block.
- [ ] On Avalonia the same, and the file shows `Ctrl+K` as written (the C# writer used to write `Ctrl+K`).
- [ ] A toolbar or ribbon button's tooltip shows its key (it used to say "Opens Window").

## 6. Not done, so not to be tested

- On macOS, Ctrl in a generated application's keys is the Control key, not Cmd.
- A view that brings its own ribbon tab and menu items (the next part of DESIGN 5.28).
- An older bug: on Avalonia, a spin box or number input bound to a window's own number (not a record's field) does not compile.

## Releases

- [ ] Upload NuGet 0.38.0: the six packages in each project's `bin/Release`.
- [ ] Check that the Rust crate 0.27.0 is on crates.io; its release workflow was still running when last looked at.
- Go v0.30.0 is released and its sums are recorded.
