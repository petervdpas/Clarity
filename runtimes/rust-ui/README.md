# clarity-ui

Clarity.UI for Rust: Clarity's own UI framework. It draws a program's windows itself with
[Skia](https://skia.org) (through `skia-safe`), with no webview and no native controls, and runs
a program Clarity generated for `stack rust desktop`.

It reads the same window and app markup (`.cxaml`) and the same resolved themes as every other
version of Clarity.UI (C#, Go, C, C++), and passes the same conformance cases.

## What is in it

- `screen`: a window's screen, which draws its elements and sends the pointer, the wheel, keys,
  typed text and the focus to the element they are for; overlays, wake-ups and tooltips.
- `element`: elements in a tree; `grid`: the window's grid (`conformance/grid-layout.json`).
- `theme` and `fonts`: Clarity's light and dark themes and a project's theme over them; Inter
  is carried (SIL Open Font License, `fonts/Inter-LICENSE.txt`).
- `markup`: the `.cxaml` reader; `parts`: one module per kind of part behind the `PartHost`
  contract; a kind not drawn yet says so in its place.
- `window`: a window's view, its host over the runtime's `WindowState`, and the stack of windows,
  run through `clarity_runtime::program::Program`.
- `facts`: what the program's model says beyond the window's values (dialogs, window sizes, the
  facts of each field, enums); `choices`: the language and theme the person chose, kept for the
  next start.
- `later`: work off the screen's thread (a file dialog, a long handler), which comes back to the
  element that asked; `files`: the file dialogs a part asks for (`FilePicker`).
- `picture`: a picture of the resources as the parts draw it: a bitmap Skia decodes, or an SVG
  read and drawn by resvg into a bitmap at the size it shows.
- `parts::custom`: custom parts a crate of the program brings (`CustomPart`, `register`);
  `parts::sound` and `parts::timer`; `melodies` (`conformance/melody.json`).
- `desktop` (feature `desktop`, on by default): the system's window through winit, OpenGL through
  glutin, the clipboard through arboard, sound through rodio, the input method (IME), and the
  system's own file dialogs (the desktop portal over D-Bus, zenity or kdialog on Linux; osascript
  on macOS; the common item dialog on Windows), written in the crate.

## Use

A generated program carries its files and starts the desktop host:

```rust
fn main() -> std::process::ExitCode {
    clarity_ui::desktop::run(App::new(), FILES)
}
```

`FILES` is a `clarity_ui::AppFiles` with the windows' and apps' markup, the themes, the model, the
title and the size.

A custom part is a type of the program's own that implements `clarity_ui::parts::custom::CustomPart`
(`set`, `hear`, `draw`, and the input it wants), registered by its declared name before the
program starts:

```rust
clarity_ui::parts::custom::register("Gauge", || Box::new(Gauge::default()));
```

It changes its bound value with `cx.change(value)` and tells its window with `cx.tell("reached", values)`.

## Building

Skia comes as prebuilt binaries for the features `gl` and `textlayout` on Linux, Windows and
macOS; it is never built from source.

## License

MIT, see `LICENSE`.
