# Orbs

A thirty-second game: orbs appear on a field, grow, then shrink away. Click
one before it is gone to catch it. Each round is saved with the player's
name, and the best scores sit in a slide-out.

## What you see and can do

The Play window holds one view, PlayView:

- The orb field on the left: orbs in five colours drift and bounce off the edges. A bar along the top shows how much of the round is left.
- Who plays: a text box with the player's name.
- Speed: a slider from 1 to 10. Higher is faster orbs that come more often.
- Orbs caught: a digit display that counts up as you catch them.
- Play again: a new round from zero.

When the round is over the score is saved and a message says how many orbs
you caught. The best scores slide-out, pinnable and resizable, lists every
round played, the most orbs first.

## Stack

C++, desktop. An example builds only for the stack its project file names.

## What it shows

- A custom part written in C++: `part OrbField` with `native "parts/OrbField"` declares a property (`speed`), two messages it tells (`caught`, `finished`) and one it hears (`restart`). The view wires the tells to its handlers and calls `orbField.restart()`.
- The part draws on the canvas Clarity.UI gives, takes the pointer and wakes itself about 60 times a second.
- Data: a Score entity saved after each round and shown in a table ordered by points.

## Files

- `../orbs.clarity`: the project file (stack, output folder, the SQLite data source).
- `orbs.clarity`: the Score entity, the OrbField part declaration and PlayView's handlers.
- `design/`: `Play.cxaml` (the window with the best scores slide-out), `PlayView.cxaml` (the field and the controls beside it), `BestScores.cxaml` (the scores table).
- `parts/OrbField/`: the part's library. `clarity/OrbField.h` is written by Clarity from the declaration; `OrbField.cpp` and `CMakeLists.txt` are the part's own code.
- `translations/`: English and Dutch.
- `migrations/main/`: one step that adds Score.
