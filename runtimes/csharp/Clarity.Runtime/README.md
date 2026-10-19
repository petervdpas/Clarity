# Clarity.Runtime

[![NuGet](https://img.shields.io/nuget/v/Clarity.Runtime.svg)](https://www.nuget.org/packages/Clarity.Runtime/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/petervdpas/Clarity/blob/main/runtimes/csharp/assets/LICENSE.txt)

**Clarity.Runtime** is the runtime of the C# applications that [Clarity](https://github.com/petervdpas/Clarity) generates. Clarity is a designer-first IDE with its own small language: you design the windows, data and logic, and it writes readable source code for Rust, Go, C#, C or C++. In a C# application, the generated code stands on this library.

It holds no user interface. The Avalonia half is [Clarity.Avalonia](https://www.nuget.org/packages/Clarity.Avalonia/).

## What it does

- **Records and storage.** `Db` reads, saves and deletes the records of a project's entities (`IRecord<T>`) on SQLite, PostgreSQL, MySQL and SQL Server, and opens a database with every migration step applied.
- **Queries.** `Query` and `Cond` are the `find ... where ...` of the Clarity language, written as SQL for each database (`Dialect`).
- **Values in the person's format.** `Formats` shows and reads numbers, money, dates and durations in the person's locale, and fills texts with slots (`Text`, `Slot`).
- **Rules.** `Contracts` checks what an action requires and ensures; a refusal (`RefusedException`) carries its text and slots to the screen.
- **Pictures and files.** `Picture` and `Pictures` keep images in the database or in a file store; `Locations` names where an application keeps its files.

## How it is used

You do not call it by hand: a Clarity project with `stack csharp avalonia desktop` generates a .NET project that references it, with records, migration steps and logic written for your project. For example, a handler written in Clarity as

```
handler task_changed
    save grid.selected
```

becomes a C# method that calls `Db.Save` on the chosen task, with the refusals and checks of its entity.

## The same on every stack

Every Clarity stack has a runtime like this one, in its own language. They are checked against the same cases (`conformance/*.json` in the Clarity repository): the SQL of queries, migrations, formats, pictures and files give the same results in Rust, Go, C, C++ and C#.

## Packages

All four database drivers come with this package. A later version splits PostgreSQL, MySQL and SQL Server into packages of their own, so an application takes only the drivers it uses.

## License

MIT, see [LICENSE.txt](https://github.com/petervdpas/Clarity/blob/main/runtimes/csharp/assets/LICENSE.txt).
