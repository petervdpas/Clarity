# Clarity's manual

How to make applications with Clarity, one topic per page. Start with [What Clarity is](start.overview.md) and [Your first project](start.first-project.md); every declaration, part, statement and expression has its page below.

## Start

- [What Clarity is](start.overview.md): What Clarity is, the few ideas the rest of the manual builds on, and where to find things in its window.
- [How Clarity works](start.how-it-works.md): What Clarity is made of, and how a design becomes a program: from the designer and the files, through the checker and the preview, to generated source code built on its own stack.
- [Your first project](start.first-project.md): A short walk from an empty folder to a running application: a task entity, a window whose view holds a form and a table, Preview and Run.
- [A complete example](start.complete-example.md): A whole small project, every file of it: a customer list with a filter that opens a form, the form that saves a customer, and a data event that clears the VAT number of a private customer. Every file here is checked as one project, so it is always valid Clarity.

## Projects

- [The project file](project.file.md): The project file and every line it takes (stack, output, version, icon, language, currency, themes, data sources, file stores, sign-in, agents, codecs), and the Project settings view that edits them.
- [Run and Build](project.run-build.md): How Generate writes the project as source code, and Run, Build and Stop turn that into a program; where the generated code goes, what each stack needs installed, and the Output panel.
- [Signing in](project.signin.md): Who signs in, and how: the project file's `signin` block, its strategies and options, the sign-in window, and who signed in as code sees it.
- [Apps for AI agents](project.agents.md): How an app's commands become tools for AI agents, and what the person using the app allows.
- [Setting up the build tools](setup.tools.md): What to install for each stack on Windows, Linux and macOS, where to get it, and how Clarity finds it afterwards.

## Designing windows

- [Windows](design.windows.md): How windows, dialogs, views and the other pages are declared and designed: their files, their data and events, their size, the snap grid, and how one window opens another.
- [The app and its frame](design.frame.md): The app's file: its settings, the main window and splash, the frame every window shares (menu, toolbar, ribbon, sidebars, footer), its commands and keys, and the built-in click targets.
- [Parts](design.parts.md): The Parts palette and the properties panel: placing and naming parts, binding them to data, what every part has, and where each group of parts is described.
- [Custom parts](design.custom-parts.md): How to put a part on a window that an outside library draws, such as a game scene drawn with Bevy, written in your project's own language.
- [Sound](design.sound.md): How a window plays sound files, tones and melodies with the sound part, on every stack.
- [Video](design.video.md): How a window plays a video with the video part, on every stack.
- [Design as text](design.text.md): The inspector's Code and Text tabs: a window as its `.cxaml` text, with the canvas following what you type, and the code of what is picked.

## Parts

- [Text parts](parts.text.md): The parts of the Text group of the Parts palette: labels, help texts, headings, icons, badges, links, text and Markdown, which show words and take no input.
- [Basic input parts](parts.input.md): The parts of the Basic input group of the Parts palette: text boxes, memos, numbers, check boxes, dropdowns, dates, files, colors, sliders and the other inputs, the button, and the sign-in part of the splash.
- [Containers](parts.containers.md): The parts of the Containers group of the Parts palette: forms, panels, tabs, content, accordions, scroll boxes, button groups, separators, wizards, popovers, slide-outs and splitters, which hold other parts on a grid of their own.
- [Data parts](parts.data.md): The parts of the Data group: tables, lists, trees, charts, trends and graphs of the window's data, and the progress bar, busy indicator and status item that show how things stand.
- [Graphics parts](parts.graphics.md): The parts of the Graphics group: an image, a seven-segment display, a canvas drawn by code or changed by the person, and a custom part an outside library draws.
- [Media parts](parts.media.md): The parts of the Media group: the sound, which plays sound files, tones, melodies and songs, and the video, which plays a video in its place on the window.
- [Time parts](parts.time.md): The part of the Time group: the timer, which runs a handler every so many seconds while its window is open, and how it makes live graphs with a trend or a chart.
- [Toolbox parts](parts.toolbox.md): The parts of the Toolbox group: tools, tool groups, brush size, opacity, colours and layers, and how a toolbox page made of them serves a canvas or a memo.
- [Sign-in parts](parts.signin.md): The parts of the Sign-in group, which stand on the sign-in views of the sign-in window: the fields, the sign-in buttons, keep me signed in, the language dropdown, the other way and the refusal.

## Writing Clarity code

- [Modules](code.modules.md): How a project's code is split into modules: the `module`, `use` and `source` lines, what a module declares, how lines, names and values are written, constants and records.
- [Handlers](code.handlers.md): How the code behind a window works: handlers wired to events, the window's and the app's events, what a handler sees, part properties, undo, asking the person, data events that run on every save and delete, and the Code panel.
- [Statements](code.statements.md): Every statement of Clarity's code, one section each: what it does, how it is written, where it may stand and what the checker refuses.
- [Expressions](code.expressions.md): The values code computes with: literals, names, operators and their order, lists, text with slots, the types the checker knows, and the built-in functions.
- [Functions and actions](code.functions.md): How to write logic that is not tied to one window: functions and actions in a module, with parameters, results, contracts, effects and who may run them.
- [Commands](code.commands.md): How a project becomes a command-line program too: the `cli` block, commands with their arguments and options, help, what a command may do, and commands offered to AI agents.
- [Queries](code.queries.md): How to read records with `find`, and how code lists, reads and writes files in a file store.
- [Toasts](code.toasts.md): How to show a short message in a corner of the window that goes by itself, each kind with its own text, lifespan, level, icon and corner.
- [Libraries](code.libraries.md): How code calls a package of your project's own language, such as a NuGet package or a Rust crate, through a library you declare.
- [Calling web APIs](code.web-apis.md): How code calls a web API with `http get` and the others, what a reply holds, and where keys go.
- [Data formats](code.formats.md): Reading records from text in a format, and writing values in one: JSON, CSV, YAML and XML, inline or through a mapping.
- [Engines](code.engines.md): How to write a processing engine in Clarity, or make, edit and build one compiled to WebAssembly, call it from code, or bring one written in another language.

## Data

- [Entities and enums](data.entities.md): How you describe your data with enums, entities and records: the field types and modifiers, versions, lists in records, and how the entity editor writes them for you.
- [Settings](data.settings.md): The app's `settings` block: what each person using the app sets for themselves (Clarity's own language, theme and agent settings, and the app's own), secret, folder and reference settings, and the Settings editor.
- [Data sources](data.sources.md): Where your records live: the project file's data sources (SQLite, the server databases, web APIs and files), keeping passwords out of the project, several sources and a module's `source`, external databases that Clarity reads without owning, how each database stores a field, and the migration steps of a source.
- [Migrations](data.migrations.md): How a database follows your entities through migration steps, written as Clarity files that Clarity proposes and the program runs when it starts.
- [Diagrams](data.diagrams.md): Designing data and code in diagrams: the ERD and class designer, Apply and From project, and the Mermaid Clarity reads.
- [Pictures and files](data.pictures-files.md): The resources an application carries (fixed pictures, Markdown and other files), pictures and files in records (in the database or in a file store), file stores, and reading and writing files from code.

## Languages and looks

- [Languages and formats](languages.md): How to translate an application: texts by key and their slots, the language files and their rules, Clarity's own words, keys that point at resources per language, letting people choose the language, and how numbers, dates and money show.
- [Themes](themes.md): How to change the look of an application with themes: colors, fonts, sizes, part looks and named styles, and how people switch between themes.

## Stacks

- [Rust with Clarity.UI](stacks.rust.md): How Clarity turns a project into a Rust program drawn by Clarity.UI, what you need to install, and how to build it.
- [C# with Clarity.UI](stacks.csharp.md): How Clarity turns a project into a C# program whose windows Clarity.UI draws, on the desktop or in a browser, what you need to install, and how to build it.
- [Go with Clarity.UI](stacks.go.md): How Clarity turns a project into a Go desktop program drawn by Clarity.UI, or a Go web server, what you need to install, and how to build it.
- [C with Clarity.UI](stacks.c.md): How Clarity turns a project into a C17 desktop program whose windows Clarity.UI draws, or a C web server whose windows Clarity.UI draws in the browser, what you need to install, and how to build it with CMake.
- [C++ with Clarity.UI](stacks.cpp.md): How Clarity turns a project into a C++20 desktop program whose windows Clarity.UI draws, or a C++ web server whose windows Clarity.UI draws in the browser, what you need to install, and how to build it with CMake.

## AI and the vault

- [The agent](ai.agent.md): How the Agent panel works: what the agent can do, taking a turn back, stopping, the token line, and asking well.
- [Connections for the agent](ai.connections.md): How the agent reaches its model through a connection in Clarity's vault: the fields of an Anthropic connection, your company's models on Microsoft Foundry, keeping the key in the vault, and choosing a connection in the panel.
- [Clarity in your own agent](ai.mcp.md): How an agent you already use (Claude Code, Codex, Copilot) designs the open project through `clarity mcp`, and how to add it.
- [Your own agent in Clarity](ai.your-agent.md): Codex or GitHub Copilot in the Agent panel with your own subscription and no API key: setup, signing in, what it may do.
- [The vault](vault.md): Clarity's own encrypted store of secrets and its connections: what it keeps, how it locks and unlocks, the Vault tab, and where it lives on disk.

## Keys

- [Keyboard shortcuts](app.keys.md): The keys that Clarity itself responds to, by where you are in the window.
