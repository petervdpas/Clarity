# Custom parts

How to put a part on a window that an outside library draws, such as a game scene drawn with Bevy, written in your project's own language.

## What a custom part is

A custom part is a part of your own, drawn by code in your project's language with any library that language has: a game scene, a map, a code editor. It is declared in a module, so the checker, code and the designer treat it as a built-in part, and placed in a window by its name. Clarity knows it by its declaration: what it shows, its properties, what it tells the window and what code tells it.

| Project's language | Library line | The part's folder holds |
|---|---|---|
| C, C++ | `native "parts/Arena"` | a CMake project with its `CMakeLists.txt`, using SFML, SDL or anything CMake finds; Clarity writes `clarity/Arena.h` |
| Rust | `crate "parts/Arena"` | a crate with its `Cargo.toml`, using any crate; Clarity writes `src/clarity.rs` with the trait `ArenaPart` and `ArenaHost` |
| Go | `go "parts/arena"` | a module with its `go.mod`, using any module; Clarity writes `clarity.go` with the interface `Part`, which the package's `New` makes, and `Host` |
| C# | `nuget "Package" 1.2.0` or `dll "libs/Plot.dll"` | an adapter `Arena.cs`; Clarity writes `Arena.g.cs` |

A part is written in the project's language only: a library line of another language is an error at the part, so a project that changes stack sees each part to write again.

## Declaring one

```clarity
module game

part Arena
    crate "parts/Arena"
    binds integer
    speed: integer, min 1, max 10, default 3
    title: text
    tells scored(points: integer)
    hears restart(level: integer)
    size 8 x 6
    grows
    preview "arena.png"
    description @arenaDescription
```

### `part`

```text
part Name
    native "folder" | crate "folder" | go "folder" | nuget "Package" version | dll "file"
    binds type
    name: type[, default value][, min value][, max value]
    tells name[(values)]
    hears name[(values)]
    size columns x rows
    grows
    preview "file"
    description text
```

| Line | What it says | Default |
|---|---|---|
| `native`, `crate`, `go`, `nuget`, `dll` | Where its code is, in the project's language (table above). Required. | none: an error |
| `binds type` | The type of value it shows and edits, which the window's `bind` gives it: text, a number, a date, a duration, an enum or bool. | binds nothing |
| `name: type, ...` | A property, set in the window and in code: text, a number, a date, a duration, an enum or bool, with a `default`, and `min` and `max` for a number. A text property's text is shown, so it is a key like a caption. | none |
| `tells name(values)` | One of its events, with typed values. | none |
| `hears name(values)` | Something code tells it to do. | none |
| `size n x n` | The span and rows the Parts palette drops it with. | the palette's own |
| `grows` | Placed in a window, it takes the height the window has left, as a table or a canvas does, so a game board fills a window the person makes taller. Without it, it keeps the height of its rows. | off |
| `preview "file"` | Its picture on the design canvas, from the resources. | a box with its name |
| `description text` | What a screen reader says of it besides its name, a key. | none |

A part's name starts with a capital, which no built-in part's does. Its properties cannot be named after a word every part has: `at`, `span`, `rows`, `bottom`, `bind`, `name`, `on`, `tooltip`, `taborder`, `visible`, `enabled` or `value`. `visible` and `enabled` are every part's: code hides and disables a custom part as any other.

The checker says: "part Arena needs its library, in the project's language: native "folder" (C, C++), crate "folder" (Rust), go "folder" (Go), nuget "Package" version or dll "file" (C#)"; "part Arena has two properties, tells or hears named 'speed'"; "'name' is a word every part has; name the property otherwise"; "a property is text, a number, a date, a duration, an enum or bool, not Order"; "a part binds text, a number, a date, a duration, an enum or bool, not list<Order>"; "a value of a tell or hear is text, a number, a date, a duration, an enum or bool, not Order"; "the default of speed is text, but speed is integer"; "'min' and 'max' bound a number; title is text"; "a part spans at least 1 column and 1 row". Where it is placed: "part Arena has no property 'sped'"; "speed is at least 1"; "speed is at most 10"; "part Arena tells no 'scorde'"; "part Arena binds nothing; leave out 'bind'"; "part Arena binds integer; 'player.name' is text".

## Placing it and its code

Place it with the palette's **Custom part** tile, which asks which part it is (or makes a new one). In the window's file it is a `Custom` element that names the part, with its property values and its tells wired to handlers:

```xml
<!-- game/design/GameView.cxaml -->
<View class="game.GameView">
  <Data name="player" type="Player" />
  <Custom name="arena" at="1,1" span="8" rows="6" bind="player.best" part="Arena" values="{&quot;speed&quot;:{&quot;kind&quot;:&quot;number&quot;,&quot;value&quot;:&quot;5&quot;},&quot;title&quot;:{&quot;kind&quot;:&quot;str&quot;,&quot;value&quot;:&quot;@levelOne&quot;}}" tells="[{&quot;handler&quot;:&quot;arenaScored&quot;,&quot;tell&quot;:&quot;scored&quot;}]" onchange="bestChanged" />
</View>
```

```clarity
code GameView
    handler arenaScored(points: integer)
        message @scored(points = points)
        arena.speed = arena.speed + 1
        arena.restart(2)

    handler bestChanged
        save player
```

- A property is set where the part is placed (`speed` 5 above) and read and set in code: `arena.speed`.
- `onchange` runs when the person changed the value it binds; a tell (`scored`) runs the handler wired to it, which takes the tell's values: "tell scored of part Arena gives (points: integer); handler arenaScored takes ()".
- `arena.restart(2)` gives the part a hear with its values: "part Arena hears no 'reset'"; "Arena.restart takes 1 value, not 0"; "value 'level' of Arena.restart is integer, not text".

## Writing its code

**+ New custom part** under Custom parts in the project tree asks for its name, its module and what draws it, in the kinds of the project's stack (a folder for C, C++, Rust and Go; a NuGet package or an assembly for C#), and declares it.

Pick the part under Custom parts in the project tree. Its view shows what the window sees as a form, and its folder. The form sets the library, the value it binds, its size on the palette, its picture, **Grows with the window**, its properties (name, type, min, max, default) and its tells and hears with their values; each change writes one line of the declaration, which the first tab shows as text. **Grows with the window** in the Properties panel of a placed part sets it too, for every place the part is used.

- **Write interface and starter** writes the file Clarity owns (the interface the declaration gives) and, in an empty folder, a starter that builds and draws nothing yet. Clarity writes the interface file again when the declaration changes; Build and Run write it too, and the starter for a new part (`clarity parts shop.clarity` does it from the command line).
- The **file tree** opens files in tabs; **+ File**, **+ Folder**, **Rename** and **Delete** change the folder. Clarity's file opens read only: change the declaration and it follows.
- **Build** builds the part on its own (CMake, Cargo or go build), its compiler's output in the Output panel. A C# adapter builds with the program.

Every Build and Run of the program builds the part in with it: a C or C++ part with CMake, a crate as a dependency of the program's, a Go module required by the program's. With Bevy, run its App without its own window loop and call `app.update()` in its frame.

On every stack the part is a control of Clarity.UI and draws on the window's own surface, in its place, through the interface Clarity writes for it (in Rust, `ArenaPart` in the crate's `src/clarity.rs`). It takes the pointer, the wheel, keys and the focus as any part does, and menus and dialogs lie over it. The `orbs` example (C++) shows one. See [Rust](stacks.rust.md#custom-parts) for the Rust trait.

**A .NET part** (`nuget` or `dll`, the C# stack) is a partial class in `parts/Arena/`. Its adapter `Arena.cs` holds the library's control and writes `ShowValue`, a `Show<Property>` per property (`ShowSpeed`) and an `On<Hear>` per hear (`OnRestart`), which the half Clarity generates (`Arena.g.cs`) calls. It calls a tell by its name (`Scored(3)`), and `Changed(value)` when the person changed the value. Declaring the part writes a starter adapter; one that no longer fits the declaration does not compile.

See also [Libraries](code.libraries.md) for calling a package from code, and [Handlers](code.handlers.md).
