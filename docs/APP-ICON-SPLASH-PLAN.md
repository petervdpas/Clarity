# An application's icon and splash screen

A plan, agreed 2026-11-06; built in the order of its steps. All three steps are built (2026-11-06).

## Goal

An application made with Clarity looks like a finished program from its first second: its own icon on its window, in the taskbar, on its program file and on its web page, and, when it wants one, a splash screen while it starts. Today a desktop window takes its icon from a resource that happens to be named `icon.png`, which nothing in the project says, and the program file keeps no icon at all. There is no splash screen.

## Decisions

- **The icon is declared**, in the project file: `icon "app.png"`, a picture among the project's resources (PNG, JPEG, WebP or SVG, square, 256 by 256 or larger for the sharpest result). The Project tab picks it. Nothing is found by a file's name any more: a project with `icon.png` and no `icon` line shows Clarity's icon, and the checker warns with the line to add.
- **One icon, everywhere it shows**: the window and the taskbar on every desktop stack, the favicon of a web page, and the program file's own icon on Windows. Clarity draws every size a system asks for from the one picture.
- **No new tool in any build.** Clarity writes the Windows icon resource itself and hands it to each stack's own build: C#'s `ApplicationIcon`, CMake's resource file for C and C++, a `.syso` beside a Go program, a link argument for Rust.
- **The splash screen is a designed page**: `page Splash: splash`, made in the designer with any parts (a picture, the name, a progress bar, a status text), shown borderless in the middle of the screen. The app names it: `splash Splash for 2 seconds`.
- **It shows while the application starts**, as Avalonia's splash window does: it appears first, stays while the app's `on start` runs, and closes when the main window shows, but not before the time the line gives. Code tells the person what loads: `splash.status = @loadingData`, a text part named `status` on the splash page.
- **Every stack alike**: Rust, Go, C#, C and C++ on the desktop through Clarity.UI. A web application has no window to centre: its splash page fills the browser page until the main window is ready, then the main window takes its place.
- **Every system alike**: Linux, Windows and macOS show the same borderless, centred window.

## Steps

### 1. The declared icon

`icon "app.png"` in the project file (read, written, checked: the resource exists and is a picture), handed to the project's model as the language and theme are, so Clarity.UI shows it on every stack without a change to its C interface; the web page's favicon; the Project tab's picker; the designer and the preview show it too.

### 2. The program file's icon on Windows

Clarity writes a Windows icon (`.ico`, every size) and the resource around it, and each stack's build carries it into the program file. Built: `windows/app.ico` and `windows/resources-<amd64|arm64|386>.obj` (a COFF `.rsrc` object, marked as Windows' own converter marks it); C# names the icon in its project, Rust links the object from `build.rs`, C and C++ from `CMakeLists.txt`, Go takes it as `resources_windows_<processor>.syso` (named so since they carry the version too, docs/VERSION-PLAN.md). Checked by linking real Windows programs: MinGW (x64), Go (x64 and ARM64), dotnet, each read back with llvm-readobj.

### 3. The splash page

The `splash` page kind and the app's `splash Splash for N seconds` line in the language, checker and designer; Clarity.UI shows it first and closes it when the main window shows; `splash.status` from code; every stack, and the web. Built: on the desktop the splash is a borderless, centred window above the others, the app's `on start` runs on a thread of its own and the main window opens in its own desktop window once it answered. In the browser the splash fills the page and `start_app` is sent without waiting, as a long call's request is. As in Avalonia, the markup carries it: the splash's window file declares its window (`width`, `height`, `decorations="none"`, `startup="centerScreen"`, `resizable="false"`, `topmost="true"`, `taskbar="false"`, `frame="none"`), which Clarity.UI applies to any window that declares it, and the app's markup names the splash (`splash="Welcome" splashFor="2"`). `splash.status` is a text each runtime keeps (Rust, Go, C#, C, C++), which Clarity.UI asks for with the wire command `splash_status` every 150 ms, and the desktop programs and web servers answer while `on start` runs. The designer adds a splash page (Start from: Splash screen) and picks the app's splash and its time in the app's properties.
