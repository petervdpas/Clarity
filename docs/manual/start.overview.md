# What Clarity is

What Clarity is, the few ideas the rest of the manual builds on, and where to find things in its window.

## One language, real programs

Clarity is a development environment for desktop applications; the program is called Clarity IDE. You write the whole application in one language, Clarity: the settings, the data, the windows and the code behind them. Clarity then transpiles it into a real project for a stack you choose and builds it with that stack's own tools.

Five stacks are generated today: Rust with Tauri, C# with Avalonia, Go with Wails, C and C++ with a webview. You design a window once; the stack decides how it is drawn. See [Rust](stacks.rust.md), [C#](stacks.csharp.md), [Go](stacks.go.md), [C](stacks.c.md) and [C++](stacks.cpp.md).

Nobody edits the generated code. When you want a change, you change the Clarity text and build again.

## A project is a file and a folder

A project is one project file, such as `shop.clarity`, with a folder of the same name next to it:

```text
shop.clarity           the project file: open this one
shop/
    customers.clarity  modules
    sales.clarity
    migrations/        steps that bring each database up to date
    out/               the generated application
```

The project file holds the settings: the stack, the data sources, the language. See [The project file](project.file.md). Every other `.clarity` file in the folder is a module. A module holds entities and enums (the data), windows, the app and its frame, code, functions and actions.

## What you see is the text

The designer does not keep a hidden model. Every change you make on the canvas or in a panel is written straight into the `.clarity` files, as ordinary, readable lines. You can open the same window as text, type in it, and the designer follows; see [Design as text](design.text.md). Undo and Redo take changes back, whoever made them.

This also means design and code are the same kind of thing. A window is a `page` block, and its handlers are a `code` block next to it, both in Clarity.

## Where things are

The ribbon at the top has four tabs:

- **Project**: New, Open, Reload and Settings for the project, New window and App, and Check, which reads every file again and checks the whole project. The sidebar shows the project's name, file, app folder, stack and data, and the project tree (App, Windows, Data, Web APIs, Diagrams, Themes, Languages, Resources, Modules).
- **Design**: the designer. Pick a window or the app in the tree and it opens on the canvas, with Elements, Parts, Window and Settings tabs in the sidebar and the inspector on the right: the properties, code and text of what is picked. Entities, enums, themes and modules open in their own editors.
- **Preview**: shows a window as it will look, with sample data, and follows buttons that open other windows. Run, Build and Stop are here; see [Run and Build](project.run-build.md).
- **Vault**: secrets and connections that Clarity uses while you work, such as the key of an AI connection. See [The vault](vault.md).

The **Agent** button at the right of the tabs opens the Agent panel on the right side of every tab. It talks to an AI model that reads and changes the open project; see [The agent](ai.agent.md).

Each tab keeps its own view: switching to another tab shows what you last had open there, or nothing until you choose something in the project tree, and coming back shows the view you left. Preview starts with the main window.

The status bar along the foot of the window shows what is open and the project file; then the build (Ready, Building..., Running, or how the last build ended), which shows the build output on a click; the errors and warnings, which check the whole project on a click; the stack and the folder builds write to, which open the project settings; and three meters, cpu, mem and io, for how busy the processors, the memory and the disk are, which open Clarity settings at System.

**Clarity settings**, at the right of the tabs, holds the settings of Clarity itself, kept for you on this computer and the same in every project; a project's own settings are under **Settings** on the Project tab. It opens in place of the work below the ribbon, with its sections on the left; **Close**, Escape or a tab goes back to the work as you left it. **Folders** holds the **Projects** folder, where **New** makes a project after asking only its name and where **Open** starts, and the **Apps** folder, where Run and Build write each application in a folder named after its project file (`shop.clarity` goes to `shop`); a project whose own app folder is not the default `out` keeps it. Either can be cleared, and the `clarity` command follows the same Apps folder. **System** shows how busy the computer has been over the last minute: each processor, memory and swap, the network and the disk, with Clarity's own share, which counts the IDE and the builds and applications it started; and the space on each disk: how much is available, of how much, with a bar that turns red past 90%. Clarity reads the processors one by one, its own processes and the network only while this page is open. **Agent** says whether the agent may run or build the application; see [The agent](ai.agent.md). **Display** holds the **Theme**, which colors Clarity: **System** (light or dark, as the system is), **Light**, **Dark**, or **Cream and green** (cream pages ruled in green, with a dark green binding). The code editors' colors follow the theme. A window you design keeps its own application's theme.

The **Manual** button beside it, or **F1**, opens this manual in a window of its own beside Clarity's, on the page about the tab you are on. The topics are on its left, with a box to find one; links lead to other pages and to the language reference, and **Back** (or Alt+Left) goes back. It opens again on the page you last read, and closes with Clarity.

Below the sidebar is the Problems list: errors and warnings of the whole project, each with its file, line and column.

The **Output** panel docks at the bottom of the main area when you Run or Build. It shows the build tool's output line by line. Close it with the cross in its header.

## Next

Make a project of your own in [Your first project](start.first-project.md).
