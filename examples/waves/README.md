# Waves

Water drawn by a custom part written in Go. Waves run out from a few
sources and meet, and the part colours every pixel by how high the water
stands there. When the water rises higher than it has since it was last
calmed, the app keeps that swell in a SQLite database.

## What you see and can do

The Water window holds one view, WaterView:

- The water, on the left. It moves about 60 times a second while the window shows.
- A speed slider (1 to 10) and a sources slider (1 to 5).
- The highest swell, shown as a three-digit display.
- **Calm the water**, which stills the water and starts it again from new places.

A slide-out, Swells seen, holds the swells kept so far, highest first, with
the time each was seen.

## Stack

Go, desktop. An example builds only for the stack its project file names.

## What it shows

- A custom part: `part WaveField` with `go "parts/WaveField"` declares its properties (`speed`, `sources`), what it tells the window (`swelled`), what it hears (`calm`) and its size. The window places it as a `Custom` part.
- The part's Go package draws into a buffer of pixels of its own and puts it on Clarity.UI's canvas. `clarity.go` is written by Clarity from the declaration; `wavefield.go` is the part's own code.
- A part that tells its window something: each swell runs `waterSwelled`, which saves it.

## Files

- `../waves.clarity`: the project file (title, stack, output folder, the SQLite data source).
- `waves.clarity`: the Swell entity, the WaveField part's declaration and the code behind the view.
- `design/`: `Water.cxaml` (the window with its slide-out), `WaterView.cxaml` (the water, sliders, display and button), `KeptSwells.cxaml` (the table of swells).
- `parts/WaveField/`: the part's Go module.
- `translations/`: English and Dutch.
- `migrations/main/`: one step that adds the swells.
