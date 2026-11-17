# Run and Build

How Generate writes the project as source code, and Run, Build and Stop turn that into a program; where the generated code goes, what each stack needs installed, and the Output panel.

## Run, Build, Stop and Browser

The buttons are in the Application group of the **Preview** tab.

- **Generate** writes the application's source in its stack's language and UI framework into the output folder. The Output panel names the folder; build and run it there with the stack's own tools, or here with **Run** and **Build**.
- **Build** builds a release program from the generated source. It does not start it; the Output panel names the program it made.
- **Run** builds a debug program from the generated source with the stack's own tool, and starts it.
- **Stop** closes the program that Run started.
- **Browser** opens a running web application (the `web` target) in your browser, at its address. The Output panel's header shows the same address as a link.

Run and Build build only what Generate wrote. Before the first Generate, or when the project changed since, they say so and build nothing: press **Generate** first.

Run and Build are off while the project has errors (see the Problems list) and while a build is busy. Run is also off while the program it started runs: Stop it first. A web application runs at the project's port (see [The project file](project.file.md)); when another program already listens there, Run says so and starts nothing. A data dictionary that no migration step covers yet also stops the build, with a message; see [Migrations](data.migrations.md).

The tool each stack builds with:

| Stack | Command | Program |
|---|---|---|
| Rust | `cargo build` | `target/debug/<name>` or `target/release/<name>` |
| C# | `dotnet build` | `bin/Debug/...` or `bin/Release/...` |
| Go | `go build` | `bin/<name>` |
| C, C++ | `cmake --workflow --preset debug` or `release` | `build/debug/<name>` or `build/release/<name>` |

## The output folder

Clarity writes the generated project into the output folder, `out` in the project's folder unless the `output` setting names another. With an Apps folder set in **Clarity settings**, **Folders**, a project left at `out` is written to a folder named after its project file in the Apps folder instead: `shop.clarity` goes to `<Apps>/shop`. The project settings show the folder Generate writes to. It is a complete project for the stack, such as a `Cargo.toml` or a `.csproj`, which the stack's tools build without Clarity.

Clarity writes it on Generate only. Each time, every file is written again and the Output panel lists each one, and files that are no longer generated are removed, so the folder holds exactly what the project generates. The build tool's own folders (such as `target`, `bin`, `obj` or `build`) and `data` are kept.

Because it removes files, Clarity marks the folder as its own with a file `.clarity-out`. It refuses to write into a folder that holds files but has no marker, and asks for an empty or new folder. Do not edit files in the output folder: the next Generate replaces them. The folder is a project of its own, which can be committed and pushed like any other.

## Build tools

Clarity uses the tools of each stack and does not bring them along. What it looks for:

- Rust: `cargo`.
- C#: a .NET SDK of the version the generated project targets.
- Go: `go` and a C compiler (cgo).
- C and C++: a compiler, CMake, and make or Ninja; on Linux, also the SQLite development files, and nlohmann-json for C++.

Rust, Go, C and C++ desktop programs draw with the Clarity.UI library, and a Go, C or C++ program of any target whose code reads or writes a [data format](code.formats.md) links Clarity.Formats, which lies in the same folder. It is no tool to install: Clarity's installer carries it for your system. A Clarity without it (built from source, say) fetches it on the first build from its release on GitHub, checked against the release's checksums, into your cache folder, once per version, and the Build tools row says "fetched on first build" until then. To use a library of your own, set the environment variable `CLARITY_UI` to its folder (`include/clarity_ui.h` and `lib/`); that always wins.

The **Build tools** row, in the New project dialog and in the project settings under Stack, says what was found or what to install, with where to get it. **How to install** beside it opens [Setting up the build tools](setup.tools.md) at the part about the first missing tool, for the system you are on. **Environment...** beside it opens Clarity settings at Environment, where the tools are scanned and chosen; the row follows when you come back.

### Where Clarity finds them

Clarity looks on the PATH and in the folders the official installers use. **Clarity settings**, **Environment** shows every stack (ready, or what is missing), what engines need (Rust's WebAssembly target, which **Install** adds after asking), and every tool with each copy a scan found and its version. Pick the copy builds use, or choose the program with **Browse...** in Clarity's file browser when it is somewhere else (on Windows, a compiler of Visual Studio's, say); the path is chosen, not typed in the field; "The first one found" leaves it to Clarity. Each missing tool there has **How to install**, which opens [Setting up the build tools](setup.tools.md) at its part. **Scan again** looks once more after you install something.

The choices hold for every project on this computer and for the `clarity` command too: they are kept in `environment.json` in Clarity's own folder (`~/.clarity`, or the folder `CLARITY_HOME` names). A chosen tool's folder comes first for every build, so the tools it starts are its own, and chosen C and C++ compilers reach CMake as `CC` and `CXX`.

### Which of Clarity's libraries builds use

Builds use the release of Clarity's libraries and packages that belongs to this Clarity: **Clarity.UI library** (which Go, C, C++ and Rust desktop programs link), **Clarity.UI browser files** (web programs, and Clarity's own design and preview), the **Rust crates**, the **NuGet packages** and the **Go module**. The installer carries the Clarity.UI library and browser files; a Clarity without them fetches them once per version on the first build that needs them. It also carries the Rust crates and the Go modules, packed, for building with no internet: a build asks crates.io or the Go proxy first, and only when it does not answer does Clarity unpack the stack's runtimes into your cache folder (once per version) and build from them, saying so in the build's output. C# fetches its NuGet packages from nuget.org on its first build, so that one needs the internet; later builds take them from NuGet's cache. C and C++ copy their runtime into the project; the libraries they take from the system (SQLite, and nlohmann_json for C++) work offline once installed. The environment variables `CLARITY_UI`, `CLARITY_UI_BROWSER` and `CLARITY_RUNTIME` name a folder of your own in their place.

A Clarity run from its own repository has one more section in **Clarity settings**: **DevTools**, for working on Clarity itself. Its first two tabs, **Disk** and **Release planning**, are always there. A piece of Clarity that apps use (Clarity.UI library, Clarity.UI browser files, Rust crates, NuGet packages, Go module, Clarity.Audio, Clarity.Video, Clarity.Image, Clarity.Formats, Clarity.Auth, Clarity.AppMcp) shows its tab by itself when your own build of it is older than its source, marked **!**, until you build it again; add the tab of any other piece with **+ Add a piece**, and a piece swapped to your own build keeps its tab. Every piece's tab closes with its **x**: its piece is the release again; a tab marked **!** comes back at Clarity's next start while your build is still behind. When the tabs do not fit, the arrows beside them go to the tab before or after, round from the last to the first. Each tab shows the **Piece**, what is **In use** (the release or your own build), its **State**, **Swap**, which chooses between the release and your own build from the repository, and **Build**, which makes your own build (such as `cargo build --release -p clarity-ui-ffi` or `runtimes/rust-ui-browser/build.sh`). While it builds, the tab shows a turning mark and the build's lines; one build runs at a time and the others wait their turn. When your own build is not there or is older than its source, the State says so. A swap counts from the next build; the design and preview draw with swapped browser files at once. Swapping a native library puts the release's libraries and your own build together in `target/clarity-swap`, put together again after each Build, so swapping one piece keeps the release of the others. The swaps are kept in `environment.json` beside the tools, and the `clarity` command run from the repository uses them too; an installed Clarity always uses the release. An agent working on Clarity presses a Build through Clarity's MCP server (`build_piece`); Clarity allows it as you chose under **Agent** (ask, allow or never). The agent also reads how the releases stand (`release_state`) and how big the build folders are (`disk_state`), and may ask to remove one (`remove_folder`), which Clarity always asks you first, unless agents may never. It writes a release plan only with `write_release_plan`, which refuses while a release check has not passed on the tree as it is. These tools are only in the MCP server of a Clarity run from its repository, never in an installed one.

The **Disk** tab lists the folders Clarity's builds and checks fill in the repository, biggest first: the debug builds (`target/debug`), the fresh checkout and its builds, your own library builds (`target/release`), the examples' builds and generated apps, the browser files' builds, the tests' scratch folders and the C# runtime's builds. Each shows its size and what removing it costs, with **Remove**, which asks first and waits until no build runs. Above them is the free space of the disk, marked when it is below the 40 GB builds and checks need. Every folder there is one git ignores and a build makes again.

The last tab, **Release planning**, is for reviewing a release; it writes no plan. **Release checks** comes first: one line says how they stand ("Not run yet", "Running 12 of 169: ...", "3 of 169 failed", "All 169 checks passed"), with **Run release checks**, which runs them all, one at a time, and the failed ones listed, each opening its last lines. The checks are every generated-file guard written and checked in your tree; then **A fresh checkout, as on GitHub** (`target/fresh-checkout`: the last commit without tags or history, your uncommitted changes over it, nothing git ignores, made from your repository without the network, building in `target/fresh-target`), where the test targets of each crate changed since the IDE's last release, every example's spec check and each changed example's build run, so a test that only passes on your machine fails here and not on GitHub; the IDE's page checks when it changed; and last **The checks left the tree clean**, which fails when a check wrote files into your tree that the release commit would take. **All checks** lists each with how it last ended, kept across starts. A check counts only for the tree it ran on: each result keeps the last commit and the uncommitted changes it saw, and once either changes, the result shows "on another tree" and counts as not run. Under the outcome one line says "A plan can be written" once every check passed on the tree as it is, else "Held back: N checks not passed on this tree". The two guards are also commands, run inside the repository: `clarity release fresh-checkout` and `clarity release clean-tree`. **What a release would start** is the chain of steps for every release whose code changed since its tag, in the order the Release Helper goes: the checks on main, then each tag with the workflow it runs, where it shows (crates.io, nuget.org, a GitHub release, the Go proxy) and what it waits for, the Go sums after the Go tag. **Changes since each release** lists the releases whose code changed, each opening its files, and folds away those with only a manifest, tests or docs changed. An agent reads all of this through Clarity's MCP server (`release_state`) and writes the Release Helper's plan with `write_release_plan`, which refuses while the plan is held back and naming the checks that hold it; after a failed release it changes only that plan's commit message, and only once the checks passed again; Clarity only reads git here: you commit, and the Release Helper tags.

Without the tools you can still design, preview and generate. Run and Build then still write the output folder, and stop with a message that says where the project is and what to install, so you can build it on another machine.

## The generated README

Every generated project has a `README.md` with what the stack needs, the commands that build and run it without Clarity, and its data sources: for a server database, the environment variable to set and an example connection. See the stack pages: [Rust](stacks.rust.md), [C#](stacks.csharp.md), [Go](stacks.go.md), [C](stacks.c.md), [C++](stacks.cpp.md).

## The Output panel

Run and Build open the Output panel at the bottom of the main area. Its first line is the command and the folder it runs in; then come the tool's own lines, the newest at the bottom (the panel keeps the last 400). The header says Building..., Running, the program that was built, or Failed with the reason.

Drag the line above the panel to make it taller or lower. The cross in its header hides it; the next Generate, Run or Build opens it again.
