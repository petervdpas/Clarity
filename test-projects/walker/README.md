# Walker

A stick figure walks across the window and back, with a picture from the
resources folder for each step of its walk. It runs as a web application:
the window lives on the server, where a timer moves the figure, and
Clarity.UI draws it in the browser.

## What you see and can do

The Field window holds one view, FieldView:

- A canvas with a floor line and the figure, with its name above its head.
- **Walk** starts the figure, **Stop** stops it; only the one that makes sense can be clicked.
- A speed slider from 1 to 6.
- A name box for the figure.

The figure takes a step ten times a second. It turns around at each wall
and then faces the other way. The toolbar has Language and Theme.

## Stack

C#, web. An example builds only for the stack its project file names. Run
it and open its address in a browser.

## What it shows

- A web target in C#: the handlers run on the server, and the browser only draws.
- A canvas drawn from code (`ondraw`) in its own units (200 by 100), with `clear`, `rect`, `image` and `text` in theme colours.
- Pictures as constants (`const right1: picture = "walk-right-1.svg"`), picked per step by a function.
- A Timer part that is switched on and off from code.
- Settings per person (language, theme, speed and the figure's name), which on the web are kept by the browser.
- Two themes, Light and Dark.

## Files

- `../walker.clarity`: the project file (title, stack, output folder, the two themes, the SQLite data source).
- `walker.clarity`: the pictures and sizes, the `frame` function, the settings and the code behind the window and its view.
- `themes.clarity`: the Light and Dark themes.
- `design/`: `Walker.cxaml` (the app with its toolbar), `Field.cxaml` (the window), `FieldView.cxaml` (the canvas, buttons, slider, name box and timer).
- `resources/`: eight SVG pictures, four steps facing right and four facing left.
- `translations/`: English and Dutch.
