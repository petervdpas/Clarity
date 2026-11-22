# Windows and macOS checks

Status: partial. What is built but has never run on Windows or macOS (UNTRIED), and what those systems still lack (NOT BUILT). Development and CI run on Linux.

Progress: 70%

## Code
- Clarity.UI per-system code: `runtimes/rust-ui/src/desktop/files/mod.rs` (folder and file pickers: COM on Windows, osascript on macOS), `runtimes/rust-ui/src/shortcuts.rs:134-137` (key matching, no Cmd mapping), AccessKit adapters ([clarity-ui](clarity-ui.md))
- Generation: `crates/clarity-gen/src/windows_resources.rs` (icon and VERSIONINFO), `crates/clarity-gen/src/tools.rs:408` (library check on Linux only)
- Secrets: `runtimes/go/secrets/secrets.go:11-40`; see [settings-secrets](settings-secrets.md)
- CI: `.github/workflows/checks.yml` (ubuntu-24.04 only); `release.yml:145` (windows-2025 builds and install-tests the IDE installers); `release-clarity-ui.yml` (libraries for win and osx, x64 and arm64)
- Docs: none of its own

## Tests
- No test runs on Windows or macOS in `checks.yml`. Release workflows build there; nothing runs a generated program there.

## Open
Untried:
- One name per stack for secrets in the Credential Manager (`Program:name`, the program name in snake case, `crates/clarity-gen/src/lib.rs:783` `settings_program`), and Rust writing with local-machine persistence as the others do (`runtimes/rust/src/secrets.rs:100-107`): built on Linux, UNTRIED on Windows. To try: keep a secret in a C# app, change it in a C or Go app, and see the C# app show the change. A secret an older C# or Rust app kept is asked for again once.
- The vault's eye and copy on fields of an open vault: UNTRIED ([vault](vault.md)).
- The Vault's keys (Ctrl+N, F2, Delete, Ctrl+Shift+C, Ctrl+L): UNTRIED.
- AltGr on a European layout (Ctrl+Alt+char) must not run a command: UNTRIED (US layout only).
- An engine beside the screen: a Ctrl shortcut reaching the window while keys are held: UNTRIED (the demo has none).
- An engine's log from a C# GUI program on Windows reaches no console unless stderr is redirected: open note.
- C# on Clarity.UI: the todo example builds, runs and opens its main window on Windows and macOS: UNTRIED.
- `cargo test -p clarity-ui` (screen reader tests in `access.rs`) on Windows: UNTRIED.
- A generated program per language opens its Clarity.UI window on Windows (Go, C and C++ fetch the win-x64 library when `CLARITY_UI` is unset): UNTRIED.
- The web target in Edge (browser files, clicks, typing); C# in the browser (the walker example) on Windows: UNTRIED (C# in the browser is built: `crates/clarity-gen/src/csharp/web.rs`).
- Narrator in a desktop program and in Edge's hidden mirror; NVDA, VoiceOver, TalkBack: UNTRIED. DESIGN.md:2032-2034.
- The IDE in WebView2: preview and design canvas drawn by Clarity.UI from the localhost preview server: UNTRIED.
- `row height 30` alike in designer and program on Windows: UNTRIED.
- Paging of web APIs on Windows (`cargo test -p clarity-gen --test gen http_build:: -- --ignored`): UNTRIED.
- Folder settings: the runtimes' settings and pictures tests on Windows and macOS: UNTRIED.
- The folder part's button opens the system picker on Windows and macOS, every desktop stack: UNTRIED.
- Progress and Stop: the runtimes' progress tests on Windows and macOS: UNTRIED.
- A long engine call shows the bar and Stop on Windows and macOS, every desktop stack: UNTRIED.
- Go, C and C++ on Clarity.UI on Windows and macOS (`CLARITY_UI`, rpath, the main thread on macOS): UNTRIED.
- The Windows (Media Foundation) and macOS (AVFoundation) video decoders: UNTRIED, compiled only. ([video-camera](video-camera.md)).

Not built for these systems:
- Ctrl means Cmd on macOS, in matching and in what is shown: NOT BUILT. `shortcuts.rs:134-137` ([frame-keys](frame-keys.md)).
- Secrets alike on macOS: PARTIAL. Go's go-keyring stores `go-keyring-base64:` values no other runtime reads.
- macOS app bundles (Info.plist, .icns) and a macOS IDE installer: NOT BUILT ([release-ci](release-ci.md)).
- The tools check for C and C++ libraries on Windows and macOS; Visual Studio's environment (vcvars); `CMAKE_PREFIX_PATH` from the chosen compiler; Fedora, Arch and Homebrew hints: NOT BUILT ([build-tools](build-tools.md)).
- A Windows and a macOS job in `checks.yml`: NOT BUILT (promised DESIGN.md:1704, 2484; [release-ci](release-ci.md)).

## Conflicts
- manual/design.video.md:42 describes the Windows and macOS decoders with no caveat; they have not run.

## Decisions
- Before 2026-10-25: features work alike on every system; branch per system only where the source differs, and put each system's shape in the tests. User: "fix it generic (os independent)"
- 2026-11-04: the desktop is GNOME on Wayland; xdotool cannot drive it, so live checks run under Xvfb (memory: desktop-wayland.md).

## Was
- WINDOWS-TESTS.md (removed 2026-11-16): sections 2-6 and After releases; unticked lines :13, :23, :28, :29, :37, :39, :41, :42, :48-54, :63-68; section 6 :58 here, :59 (a view bringing its own ribbon tab) in [frame-keys](frame-keys.md). Its version notes (crate 0.28.1, Clarity.UI 0.20.0) were stale and are dropped.
- DESIGN 7.9 (screen readers untried, DESIGN.md:2032-2034)
- memory: every-os.md
