# An application's version

A plan, agreed 2026-11-06; built in the order of its steps. All four steps are built (2026-11-06).

## Goal

An application made with Clarity says which version it is: on its splash screen and in its About dialog, in its program file's properties on Windows, and in the package each stack builds. Today a project has no version at all, so a splash can only leave it out.

## Decisions

- **The version is declared** in the project file: `version "1.2.0"`, as `major.minor.patch` with an optional pre-release (`2.0.0-beta.1`). The Project tab edits it. It is optional: without the line nothing shows a version, and each stack's package keeps the version its build needs, as today (`0.1.0` where one is required).
- **Every stack carries it the way its own build does**: C#'s `<Version>` in the project, Rust's `version` in `Cargo.toml`, CMake's `project(... VERSION)` for C and C++ (its numbers only); Go has no version in its module file, so its program file carries it through the Windows resource below. Generated code writes `app.version` as the literal version, and the texts carry `{app.version}` filled. No new tool in any build.
- **Code reads it** as `app.version`, a text.
- **Texts show it without code**: any text by key may hold the slot `{app.version}` (`welcomeVersion "Version {app.version}"`), which Clarity fills itself; a use gives it no value. A splash page, a window's title or an About dialog shows the version so, when the designer puts it there; nothing shows it by itself.
- **Used without one is an error**: code or a text that reads `app.version` in a project without a version says to write `version "1.0.0"` in the project file.
- **Windows shows it** in the program file's Properties: Clarity writes a version resource (file and product version, the title) into the resource object it already writes for the icon.
- **Every stack and system alike**: the same version in every program; macOS and Linux have it in the package and the program, Windows also in the file's properties.

## Steps

### 1. The setting, code and texts

`version "1.2.0"` in the project file (read, written, checked), handed to the model; `app.version` in code on every stack; `{app.version}` filled in texts by key; the Project tab's field.

### 2. Each stack's package

C#'s `<Version>`, Rust's `Cargo.toml`, CMake's `project(... VERSION)`.

### 3. The program file's version on Windows

A `VERSIONINFO` resource beside the icon, in `windows/resources-<processor>.obj` (and `resources_windows_<processor>.syso` for Go), which also exists for a project with a version and no icon; checked by linking a program with MinGW and reading it back with llvm-readobj. C# writes its own from `<Version>`.

### 4. The todo example

Its splash shows "Version 1.0.0".
