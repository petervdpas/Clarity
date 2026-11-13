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

### `library`

    library Name
        nuget "Package" version | dll "file" | crate "name" "version" | go "module" version | cmake "Package"
        function name(parameter: type, ...) [-> type]

- One package line, in the project's language, and a `function` line per function code calls. Values in and out are text, a number, a date, a duration or bool.
- The checker says: "library Words needs its package, in the project's language: nuget "Package" version or dll "file" (C#), crate "name" "version" (Rust), go "module" version (Go), cmake "Package" (C, C++)"; "library Words has two functions named 'spell'"; "a library's values are text, a number, a date, a duration or bool, not Order". At a call: "library Words has no function spel"; "Words.spell takes 1 argument, not 2"; "argument 'n' of Words.spell is an integer, not text".

## Writing the adapter

The file Clarity writes from the declaration names each function as your language spells it, with its types; the adapter writes exactly those. Read it first.

- **C#**: `Words.g.cs` holds `namespace ClarityLibraries;` and `public static partial class Words`, with a `public static partial` method per function in PascalCase: `spell` is `Spell`, `isLoud` is `IsLoud`. An `integer` is a `long`, `text` a `string`, `bool` a `bool`. `Words.cs` is the other half: the same namespace and class, each method again as `public static partial`, now with its body.

```csharp
namespace ClarityLibraries;

public static partial class Words
{
    public static partial string Spell(long n) => Humanizer.NumberToWordsExtension.ToWords((int)n);
}
```

- **Go**: each function in PascalCase in the library's package (`Spell`).
- **Rust**: the methods of the `WordsLibrary` trait in snake_case (`spell`, `is_loud`).
- **C, C++**: the functions as the header declares them.

## Using one

- Code calls a function as `Words.spell(42)`, checked like an engine's. Values in and out are text, a number, a date, a duration or bool.
- A library in another language than the project's is an error.
- **+ New library** under Libraries in the project tree asks for its name, its module and its package, in the kinds of the project's stack, and declares it.
- Pick it under Libraries in the project tree: its view sets its package and its functions (name, values, result) as a form, one line of the declaration each, writes the adapter's starter, and edits its folder in tabs, as a custom part's view does. A Rust or Go library builds on its own with **Build**; the others build with the program.
