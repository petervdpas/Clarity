# What Clarity is

What Clarity is, the few ideas the rest of the manual builds on, and where to find things in its window.

## One language, real programs

Clarity is a development environment for desktop applications. You write the whole application in one language, Clarity: the settings, the data, the windows and the code behind them. Clarity then transpiles it into a real project for a stack you choose and builds it with that stack's own tools.

Five stacks are generated today: Rust with Tauri, C# with Avalonia, Go with Wails, C and C++ with a webview. You design a window once; the stack decides how it is drawn. See [Rust](stacks.rust.md), [C#](stacks.csharp.md), [Go](stacks.go.md), [C](stacks.c.md) and [C++](stacks.cpp.md).

Nobody edits the generated code. When you want a change, you change the Clarity text and build again.

## A project is a file and a folder

A project is one project file, such as `shop.clarity`, with a folder of the same name next to it:

```
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

- **Project**: New, Open, Reload and Settings for the project, New window and App, and Check, which reads every file again and checks the whole project. The sidebar shows the project's name, file, app folder, stack and data, and the project tree (App, Windows, Data, Themes, Modules).
- **Design**: the designer. Pick a window or the app in the tree and it opens on the canvas, with Elements, Parts, Window and Settings tabs in the sidebar and the properties on the right. Entities, enums, themes and modules open in their own editors.
- **Preview**: shows a window as it will look, with sample data, and follows buttons that open other windows. Run, Build and Stop are here; see [Run and Build](project.run-build.md).
- **Vault**: secrets and connections that Clarity uses while you work, such as the key of an AI connection. See [The vault](vault.md).

The **Agent** button at the right of the tabs opens the Agent panel on the right side of every tab. It talks to an AI model that reads and changes the open project; see [The agent](ai.agent.md).

The **Manual** button beside it, or **F1**, opens this manual in a window of its own beside Clarity's, on the page about the tab you are on. The topics are on its left, with a box to find one; links lead to other pages and to the language reference, and **Back** (or Alt+Left) goes back. It opens again on the page you last read, and closes with Clarity.

Below the sidebar is the Problems list: errors and warnings of the whole project, each with its file, line and column.

The **Output** panel docks at the bottom of the main area when you Run or Build. It shows the build tool's output line by line. Close it with the cross in its header.

## Next

Make a project of your own in [Your first project](start.first-project.md).
