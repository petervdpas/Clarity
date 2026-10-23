# Libraries

How code calls a package of your project's own language, such as a NuGet package or a Rust crate, through a library you declare.

## What a library is

A library gives your code functions that a package of the project's language does: spelling numbers out, a checksum, a format Clarity does not read. You declare it with the functions code calls and its package; an adapter in your project's language writes those functions with the package.

```clarity
library Words
    nuget "Humanizer.Core" 2.14.1
    function spell(n: integer) -> text

action describe(order: Order)
    effects db.write
    order.words = Words.spell(42)
```

| Project's language | Package line | The adapter in `libraries/Words/` |
|---|---|---|
| C# | `nuget "Package" 1.2.0`, `dll "file"` | `Words.cs`, beside Clarity's `Words.g.cs` |
| Rust | `crate "name" "1"` | a crate whose `Words` implements `WordsLibrary` (`src/clarity.rs`) |
| Go | `go "module/path" v1.0.0` | a module whose package has the functions (`clarity.go` types them) |
| C, C++ | `cmake "Package"` | a CMake target `Words` with the functions of `clarity/Words.h` or `.hpp` |

## Using one

- Code calls a function as `Words.spell(42)`, checked like an engine's. Values in and out are text, a number, a date, a duration or bool.
- A library in another language than the project's is an error.
- Pick it under Libraries in the project tree: its view lists what code calls, writes the adapter's starter, and edits its folder in tabs, as a custom part's view does. A Rust or Go library builds on its own with **Build**; the others build with the program.
