# Custom parts

How to put a part on a window that an outside library draws, such as a game scene drawn with Bevy, written in your project's own language.

## What a custom part is

A custom part is a part of your own, drawn by code in your project's language with any library that language has. Clarity knows it by its declaration: what it shows, its properties, what it tells the window and what code tells it. The checker, the code and the designer treat it as a built-in part.

| Project's language | Library line | The part's folder holds |
|---|---|---|
| C, C++ | `native "parts/Arena"` | a CMake project; Clarity writes `clarity/Arena.h` |
| Rust | `crate "parts/Arena"` | a crate; Clarity writes `src/clarity.rs` with the trait `ArenaPart` |
| Go | `go "parts/arena"` | a module; Clarity writes `clarity.go` with the interface `Part` |
| C# | `nuget "Package" 1.2.0` or `dll "libs/Plot.dll"` | an adapter `Arena.cs`; Clarity writes `Arena.g.cs` |

A part is written in the project's language only: a C++ part in a Rust project is an error.

## Declaring one

```clarity
part Arena
    crate "parts/Arena"
    speed: integer, min 1, max 10, default 3
    tells scored(points: integer)
    hears restart
    size 8 x 10
    grows
```

- **Properties** (`speed`) are set on the window line and from code: `arena.speed = 7`.
- **Tells** are what the part tells the window; a handler runs: `Arena name arena ... on scored: arenaScored`.
- **Hears** are what code tells the part: `arena.restart()`.
- **`binds`** gives it a value the window binds, as an input has; `size` is its size when placed.
- **`grows`** makes it take the height its window has left, as a table or a canvas does, so a game board fills a window the person makes taller. Without it, it keeps the height of its rows. In the IDE, **Grows with the window** in the part's view and in the Properties panel of a placed part sets it for every place the part is used.

Place it by its name, from the Custom parts group of the palette or as a line: `Arena name arena at 1,1 span 8 rows 10 speed 3`.

## Writing its code

Pick the part under Custom parts in the project tree. Its view shows what the window sees, and its folder:

- **Write interface and starter** writes the file Clarity owns (the interface the declaration gives) and, in an empty folder, a starter that builds and draws nothing yet.
- The **file tree** opens files in tabs; **+ File**, **+ Folder**, **Rename** and **Delete** change the folder. Clarity's file opens read only: change the declaration and it follows.
- **Build** builds the part on its own (CMake, Cargo or go build), its compiler's output in the Output panel. A C# adapter builds with the program.

Every Build and Run of the program builds the part in with it. On every stack the part draws on the canvas Clarity.UI gives it, in its place on the window; the `orbs` example (C++) shows one.

On Clarity.UI (Rust and C#) a part is no native window: it is a control of Clarity.UI and draws on the window's own surface, in its place, through the trait Clarity writes for it (in Rust, `GaugePart` in the crate's `src/clarity.rs`). It takes the pointer, the wheel, keys and the focus as any part does, and menus and dialogs lie over it. See [Rust](stacks.rust.md#custom-parts) for the Rust trait.
