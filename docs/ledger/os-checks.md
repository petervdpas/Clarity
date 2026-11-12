# Windows and macOS checks

Status: partial. What is built but has never run on Windows or macOS (UNTRIED), and what those systems still lack (NOT BUILT). Development and CI run on Linux.

## Code
- Clarity.UI per-system code: `runtimes/rust-ui/src/desktop/files/mod.rs` (folder and file pickers: COM on Windows, osascript on macOS), `runtimes/rust-ui/src/shortcuts.rs:134-137` (key matching, no Cmd mapping), AccessKit adapters ([clarity-ui](clarity-ui.md))
- Generation: `crates/clarity-gen/src/windows_resources.rs` (icon and VERSIONINFO), `crates/clarity-gen/src/tools.rs:402` (library check on Linux only)
- Secrets: `runtimes/go/secrets/secrets.go:11-40`; see [settings-secrets](settings-secrets.md)
- CI: `.github/workflows/checks.yml` (ubuntu-24.04 only); `release.yml:145` (windows-2025 builds and install-tests the IDE installers); `release-clarity-ui.yml` (libraries for win and osx, x64 and arm64)
- Docs: none of its own

## Tests
- No test runs on Windows or macOS in `checks.yml`. Release workflows build there; nothing runs a generated program there.

## Open
Untried (from WINDOWS-TESTS.md at HEAD, unticked lines):
- One name per stack for secrets in the Credential Manager (`Program:name`, snake-case program name), and keyring-rs writing with enterprise persistence while the others use local machine: UNTRIED. WINDOWS-TESTS.md:29-35; needs a Rust runtime release.
- The vault's eye and copy on fields of an open vault: UNTRIED. :41-42 ([vault](vault.md)).
- The Vault's keys (Ctrl+N, F2, Delete, Ctrl+Shift+C, Ctrl+L): UNTRIED. :50.
- AltGr on a European layout (Ctrl+Alt+char) must not run a command: UNTRIED. :51 (US layout only).
- An engine beside the screen: a Ctrl shortcut reaching the window while keys are held: UNTRIED. :80-81 (the demo has none).
- An engine's log from a C# GUI program on Windows reaches no console unless stderr is redirected: open note. :84-85.
- C# on Clarity.UI: the todo example builds, runs and opens its main window on Windows and macOS: UNTRIED. :96.
- `cargo test -p clarity-ui` (screen reader tests in `access.rs`) on Windows: UNTRIED. :104.
- A generated program per language opens its Clarity.UI window on Windows (Go, C and C++ fetch the win-x64 library when `CLARITY_UI` is unset): UNTRIED. :105.
- The web target in Edge (browser files, clicks, typing); C# in the browser (the walker example) on Windows: UNTRIED. :106, :88 (C# in the browser is built: `crates/clarity-gen/src/csharp/web.rs`).
- Narrator in a desktop program and in Edge's hidden mirror; NVDA, VoiceOver, TalkBack: UNTRIED. :107-108; DESIGN.md:2032-2034.
- The IDE in WebView2: preview and design canvas drawn by Clarity.UI from the localhost preview server: UNTRIED. :109.
- `row height 30` alike in designer and program on Windows: UNTRIED. :110.
- Paging of web APIs on Windows (`cargo test -p clarity-gen --test http_build -- --ignored`): UNTRIED. :137.
- Folder settings: the runtimes' settings and pictures tests on Windows and macOS: UNTRIED. :138.
- The folder part's button opens the system picker on Windows and macOS, every desktop stack: UNTRIED. :139.
- Progress and Stop: the runtimes' progress tests on Windows and macOS: UNTRIED. :140.
- A long engine call shows the bar and Stop on Windows and macOS, every desktop stack: UNTRIED. :142.
- Go, C and C++ on Clarity.UI on Windows and macOS (`CLARITY_UI`, rpath, the main thread on macOS): UNTRIED. :144.
- The Windows (Media Foundation) and macOS (AVFoundation) video decoders: UNTRIED, compiled only. VIDEO-PLAN.md:3 ([video-camera](video-camera.md)).

Not built for these systems:
- Ctrl means Cmd on macOS, in matching and in what is shown: NOT BUILT. WINDOWS-TESTS.md:114; `shortcuts.rs:134-137` ([frame-keys](frame-keys.md)).
- Secrets alike on macOS: PARTIAL. Go's go-keyring stores `go-keyring-base64:` values no other runtime reads.
- macOS app bundles (Info.plist, .icns) and a macOS IDE installer: NOT BUILT ([release-ci](release-ci.md)).
- The tools check for C and C++ libraries on Windows and macOS; Visual Studio's environment (vcvars); `CMAKE_PREFIX_PATH` from the chosen compiler; Fedora, Arch and Homebrew hints: NOT BUILT ([build-tools](build-tools.md)).
- A Windows and a macOS job in `checks.yml`: NOT BUILT (promised DESIGN.md:1704, 2484; [release-ci](release-ci.md)).

## Conflicts
- DESIGN 5.28 says Ctrl is Cmd on macOS; WINDOWS-TESTS.md section 6 says not done; the code agrees with WINDOWS-TESTS.
- manual/design.video.md:42 describes the Windows and macOS decoders with no caveat; they have not run.
- WINDOWS-TESTS.md:5 says C#'s parts and dialogs come later; DESIGN G7 and the walker example say they are built.
- WINDOWS-TESTS.md:11 points to `runtimes/c/shell/shell.c`; it is `runtimes/c/desktop/desktop.c`.
- WINDOWS-TESTS.md:123 says C and C++ need no release; README.md:74 says they fetch the Clarity.UI release, and releases go through the Release Helper (CLAUDE.md).

## Decisions
- Before 2026-10-25: features work alike on every system; branch per system only where the source differs, and put each system's shape in the tests. User: "fix it generic (os independent)"
- 2026-11-04: the desktop is GNOME on Wayland; xdotool cannot drive it, so live checks run under Xvfb (memory: desktop-wayland.md).

## Was
- WINDOWS-TESTS.md (sections 1-6 and Releases; unticked lines :29, :41, :50, :51, :80, :84, :88, :96, :104-110, :137-140, :142, :144; section 6 :114-115)
- DESIGN 7.9 (screen readers untried, DESIGN.md:2032-2034)
- memory: every-os.md
