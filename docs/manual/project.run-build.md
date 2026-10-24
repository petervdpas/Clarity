# Run and Build

How Run, Build and Stop turn the project into a program, where the generated code goes, what each stack needs installed, and the Output panel.

## Run, Build and Stop

The three buttons are in the Application group of the **Preview** tab.

- **Run** generates the application, builds a debug program with the stack's own tool, and starts it. A program that Run started before is closed first.
- **Build** generates the application and builds a release program. It does not start it; the Output panel names the program it made.
- **Stop** closes the program that Run started.

Run and Build are off while the project has errors (see the Problems list) and while a build is busy. A data dictionary that no migration step covers yet also stops the build, with a message; see [Migrations](data.migrations.md).

The tool each stack builds with:

| Stack | Command | Program |
|---|---|---|
| Rust | `cargo build` | `target/debug/<name>` or `target/release/<name>` |
| C# with Avalonia | `dotnet build` | `bin/Debug/...` or `bin/Release/...` |
| Go | `go build` | `bin/<name>` |
| C, C++ | `cmake --workflow --preset debug` or `release` | `build/debug/<name>` or `build/release/<name>` |

## The output folder

Clarity writes the generated project into the output folder, `out` in the project's folder unless the `output` setting names another. It is a complete project for the stack, such as a `Cargo.toml` or a `.csproj`, which the stack's tools build without Clarity.

Clarity writes it on Run and Build only. Each time, the folder ends up holding exactly what the project generates: changed files are written, unchanged ones are left alone so the build tool sees no change, and files that are no longer generated are removed. The build tool's own folders (such as `target`, `bin`, `obj` or `build`) and `data` are kept.

Because it removes files, Clarity marks the folder as its own with a file `.clarity-out`. It refuses to write into a folder that holds files but has no marker, and asks for an empty or new folder. Do not edit files in the output folder: the next Run replaces them.

## Build tools

Clarity uses the tools of each stack and does not bring them along. What it looks for:

- Rust: `cargo`; on Linux, also the WebKitGTK development files (found with `pkg-config`).
- C#: a .NET SDK of the version the generated project targets.
- Go: `go`; on Linux, also the WebKitGTK development files.
- C and C++: a compiler, CMake, and make or Ninja; on Linux, also the SQLite and WebKitGTK development files, and nlohmann-json for C++. C also needs a C++ compiler for the webview.

The **Build tools** row, in the New project dialog and in the project settings under Stack, says what was found or what to install, with where to get it. **Environment...** beside it opens Clarity settings at Environment, where the tools are scanned and chosen; the row follows when you come back.

### Where Clarity finds them

Clarity looks on the PATH and in the folders the official installers use. **Clarity settings**, **Environment** shows every stack (ready, or what is missing) and every tool with each copy a scan found and its version. Pick the copy builds use, or type a program's path when it is somewhere else (on Windows, a compiler of Visual Studio's, say); "The first one found" leaves it to Clarity. **Scan again** looks once more after you install something.

The choices hold for every project on this computer and for the `clarity` command too: they are kept in `environment.json` in Clarity's own folder (`~/.clarity`, or the folder `CLARITY_HOME` names). A chosen tool's folder comes first for every build, so the tools it starts are its own, and chosen C and C++ compilers reach CMake as `CC` and `CXX`.

Without the tools you can still design, preview and generate. Run and Build then still write the output folder, and stop with a message that says where the project is and what to install, so you can build it on another machine.

## The generated README

Every generated project has a `README.md` with what the stack needs, the commands that build and run it without Clarity, and its data sources: for a server database, the environment variable to set and an example connection. See the stack pages: [Rust](stacks.rust.md), [C#](stacks.csharp.md), [Go](stacks.go.md), [C](stacks.c.md), [C++](stacks.cpp.md).

## The Output panel

Run and Build open the Output panel at the bottom of the main area. Its first line is the command and the folder it runs in; then come the tool's own lines, the newest at the bottom (the panel keeps the last 400). The header says Building..., Running, the program that was built, or Failed with the reason.

Drag the line above the panel to make it taller or lower. The cross in its header hides it; the next Run or Build opens it again.
