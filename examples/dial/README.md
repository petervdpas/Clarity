# Dial

A speedometer drawn on a canvas. The needle runs from 0 to 200 km/h and
changes colour as the speed gets high. Where it points and which colour it
has come from an engine written in Rust and loaded as WebAssembly.

## What you see and can do

The Main window holds one view, MainView:

- A canvas with the dial: a ring, marks at 0, the middle and 200, the needle, and the speed as text (`80 km/h` at the start).
- **Slower** and **Faster** (also the `-` and `+` keys) change the speed by 10, staying between 0 and 200.
- A slider sets the speed directly, in steps of 10.

The needle is the theme's accent colour below 120 km/h, its warning colour
from 120 and its error colour from 160. The canvas draws again after every
handler, so the dial always shows the current speed.

## Stack

Go, desktop. An example builds only for the stack its project file names.

## What it shows

- An engine: `engine Gauge from "gauge.wasm"` declares two functions, `angle` and `zone`, that the Clarity code calls like its own. Their source is the Rust crate in `engines/gauge`, built for `wasm32-unknown-unknown` into `engines/gauge.wasm`.
- A canvas drawn from code (`ondraw`), with `clear`, `ring`, `line`, `circle` and `text` in theme colours.
- A view's own data: the speed is `Data` in `MainView.cxaml`; the example keeps nothing in a database.

## Files

- `../dial.clarity`: the project file (stack, output folder, the SQLite data source).
- `dial.clarity`: the Gauge engine declaration and MainView's handlers (drawing the dial, changing the speed).
- `design/`: `Dial.cxaml` (the app), `Main.cxaml` (the window), `MainView.cxaml` (the canvas, buttons and slider).
- `engines/gauge/`: the engine's Rust source; `engines/gauge.wasm` is the built engine the app loads.
- `translations/`: English and Dutch.
