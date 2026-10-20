# Parts

The parts you place on a window, grouped as the Parts palette shows them, with what each binds to and its events, and the properties panel.

## The Parts palette

While you design a window, the sidebar's **Parts** tab lists the parts in groups. Drag a tile onto the window to place it; it lands on the grid cell under the pointer, inside the innermost container there. A new part gets a name such as `button1` or `textbox2`.

Below the groups, the palette lists the window's data. A field of a single record is a tile that places a label and a bound input (and the help text when the field has one and there is room). A list is a tile, Table of name, that places a table bound to it.

For every part, its own properties, events and binding rules are in [the language reference](../LANGUAGE.md).

## Text

- `label`: a caption for an input; `for name` ties it to the input, and clicking it focuses that input. No events.
- `help`: a help text for an input; without a caption it shows the field's `help`, and it shows the error a handler sets.
- `heading`: a heading.
- `text`: shows the value of a field of any type, `bind customer.name`, or a plain value of the window, `bind shown`.

## Basic input

- `textbox` binds to text, email or url; `memo` to text, over several lines; `password` to text, hidden while typed. Events: `change`, `enter`, `leave`.
- `number` binds to integer, decimal, money, id or duration. Events: `change`, `enter`, `leave`.
- `checkbox` and `switch` bind to bool; `dropdown` to an enum or a `ref`; `radio` to an enum, one button per value; `date` to date or datetime. Event: `change`.
- `button` binds to nothing. Event: `click`. It can have a `key`: `"Ctrl+S"`, or a character such as `"7"` or `"+"`, which the keyboard and the number pad both type (not while typing in a text input), and be the `default` (Enter) or `cancel` (Escape) button. With `on click: digit(7)` it gives its handler values; see [Handlers](code.handlers.md).

`change` fires only when the person changes the value to one that differs. Typed text counts when the input is left or Enter is pressed.

## Containers

- `form`: binds to one record (`bind customer`) and holds the inputs of that record on its own grid. Event: `submit`, when Enter is pressed in one of its inputs.
- `panel`: parts in a frame with the caption.
- `tabs`: `tab "Title"` blocks, each with parts. Event: `change`. Add tab is in the properties.
- `splitter`: exactly two `pane` blocks, side by side or stacked.

## Data

- `table`: binds to a list (`bind orders`) and shows `columns`. Columns named in `edit` are inputs in the cells. Events: `select` (another row is chosen), `open` (double-click or Enter), `change` (a cell was edited).
- `list`: binds to a list and shows the fields in `show`. Events: `select`, `open`.

## Graphics

- `image`: binds to an `image` field, or shows a fixed picture of the project's `images` folder with `source "logo.png"`. Event: `click`. See [Pictures and files](data.pictures-files.md).
- `display`: shows a number in seven-segment digits, as on a calculator, with its dark segments faint: `display name screen at 1,1 span 4 rows 2 bind shown digits 12`. It binds to an integer, decimal or money (a field or a plain value of the window). A value is rounded half away from zero to fit its digits (12 without `digits n`), and shows `E` when its whole part does not fit. Its style gives its colors: `background` behind the digits, `text` for the lit segments.
- `canvas`: free graphics, drawn by its draw handler (`on draw: drawDial`) in its own units (`units 200 x 100`, 100 x 100 without it): lines, rectangles, circles, rings and text in the theme's colors. The handler runs when the window opens and after every other handler, so the drawing follows the data; see [Handlers](code.handlers.md). With `on click: handler` it can be clicked, and the handler reads where. Without a draw handler it shows as a placeholder.

When the window is a `window` or the app, a Frame group comes first, with the menu, toolbar, ribbon, sidebar and footer; see [The app and its frame](design.frame.md).

## The properties panel

The panel on the right has two sections, which fold: **The window** (title, grid columns, data and the window's events; see [Windows](design.windows.md)) and **What is picked**. Click a part on the canvas, or a line in the sidebar's Window tab, to pick it. Click the window's background or title bar to pick nothing.

What is picked shows the fields that part has:

- Name, Caption, and Column, Row, Width and Height.
- Bound to: only the data and fields that part accepts. Columns or Shown fields, and Edited in the table, for tables and lists.
- On click, open (and In a new window) for buttons and images; For input for labels and help texts; Placeholder, Tooltip, Style, Tab order, Shortcut; Read-only and the default and cancel boxes.
- Events: a row per event. **Wire** names the handler after the part and the event, writes it and opens the Code panel; **Code** shows it; the cross unwires it. See [Handlers](code.handlers.md).
- Remove.

Every change is written to the file at once. **Delete** in the ribbon, or the Delete key, removes the picked part.
