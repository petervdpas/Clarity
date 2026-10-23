# Clarity's manual

How to make applications with Clarity, one topic per page. Start with [What Clarity is](start.overview.md) and [Your first project](start.first-project.md); the exact syntax of every construct is in [the language reference](../LANGUAGE.md).

## Start

- [What Clarity is](start.overview.md): What Clarity is, the few ideas the rest of the manual builds on, and where to find things in its window.
- [Your first project](start.first-project.md): A short walk from an empty folder to a running application: a task entity, a window with a form and a table, Preview and Run.

## Projects

- [The project file](project.file.md): The settings in the project file (stack, output, language, currency, themes, data sources and file stores) and the Project settings view that edits them.
- [Run and Build](project.run-build.md): How Run, Build and Stop turn the project into a program, where the generated code goes, what each stack needs installed, and the Output panel.

## Designing windows

- [Windows](design.windows.md): How a window is declared and designed: its kind, its data and events, the snap grid, and how one window opens another.
- [Parts](design.parts.md): The parts you place on a window, grouped as the Parts palette shows them, with what each binds to and its events, and the properties panel.
- [The app and its frame](design.frame.md): The `app` block: the main window, the frame every window shares (menu, toolbar, ribbon, sidebars, footer), keys, and the built-in click targets.
- [Custom parts](design.custom-parts.md): How to put a part on a window that an outside library draws, such as a game scene drawn with SFML or Bevy, written in your project's own language.
- [Sound](design.sound.md): How a window plays sound files, tones and melodies with the sound part, on every stack.
- [Design as text](design.text.md): The text panel of the designer: a window as Clarity text, with the canvas following what you type, and the code behind next to it.

## Writing Clarity code

- [The language](code.language.md): A guide to the parts of Clarity you write code with: lines and names, values and types, text with slots, operators, statements and expressions.
- [Handlers](code.handlers.md): How the code behind a window works: handlers wired to events, part properties, messages, opening and closing windows, and the Code panel.
- [Functions and actions](code.functions.md): How to write logic that is not tied to one window: functions and actions in a module, with parameters, results, contracts and effects.
- [Queries](code.queries.md): How to read records with `find`, and how code lists, reads and writes files in a file store.
- [Data events](code.data-events.md): How to run code whenever a record is saved or deleted, from any window or action, and how a refusal stops it.
- [Libraries](code.libraries.md): How code calls a package of your project's own language, such as a NuGet package or a Rust crate, through a library you declare.
- [Engines](code.engines.md): How to make, edit and build a processing engine compiled to WebAssembly in Clarity, call it from code, or bring one written in another language.

## Data

- [Entities and enums](data.entities.md): How you describe your data with entities and enums, the field types and modifiers, and how the entity editor writes them for you.
- [Migrations](data.migrations.md): How a database follows your entities through migration steps, written as Clarity files that Clarity proposes and the program runs when it starts.
- [Data sources](data.sources.md): Where your records live: SQLite and the server databases, keeping passwords out of the project, several sources, and external databases that Clarity reads without owning.
- [Pictures and files](data.pictures-files.md): How an application shows fixed pictures from the resources folder, keeps pictures in records (in the database or in a file store), and reads and writes files from code.

## Languages and looks

- [Languages and formats](languages.md): How to translate an application, let people choose its language, and control how numbers, dates and money show.
- [Themes](themes.md): How to change the look of an application with themes: colors, fonts, sizes, part looks and named styles, and how people switch between themes.

## Stacks

- [Rust with Tauri](stacks.rust.md): How Clarity turns a project into a Rust desktop program with Tauri and the web renderer, what you need to install, and how to build it.
- [C# with Avalonia](stacks.csharp.md): How Clarity turns a project into a C# desktop program whose windows are native Avalonia views, what you need to install, and how to build it.
- [Go with Wails](stacks.go.md): How Clarity turns a project into a Go desktop program with Wails v3 and the web renderer, what you need to install, and how to build it.
- [C with a webview](stacks.c.md): How Clarity turns a project into a C17 desktop program that shows the web renderer in a webview, what you need to install, and how to build it with CMake.
- [C++ with a webview](stacks.cpp.md): How Clarity turns a project into a C++20 desktop program that shows the web renderer in a webview, what you need to install, and how to build it with CMake.

## AI and the vault

- [The agent](ai.agent.md): How the Agent panel works: what the agent can do in your project, taking a turn back, stopping and starting over, what the token line means, and how to ask well.
- [Connections for the agent](ai.connections.md): How the agent reaches its model through a connection in Clarity's vault: the fields of an Anthropic connection, keeping the key in the vault, and choosing a connection in the panel.
- [The vault](vault.md): Clarity's own encrypted store of secrets and its connections: what it keeps, how it locks and unlocks, the Vault tab, and where it lives on disk.

## Keys

- [Keyboard shortcuts](app.keys.md): The keys that Clarity itself responds to, by where you are in the window.
