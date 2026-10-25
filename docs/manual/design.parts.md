# Parts

The parts you place on a window, grouped as the Parts palette shows them, with what each binds to and its events, and the properties panel.

## The Parts palette

While you design a window, the sidebar's **Parts** tab lists the parts in groups. Drag a tile onto the window to place it; it lands on the grid cell under the pointer, inside the innermost container there. A new part gets a name after what it shows, such as `ordersTable` or `email`, or `button1` while it says nothing of its purpose yet. A part with such a numbered name takes a name after its purpose once you give it one: a label for `vatNumber` becomes `vatNumberLabel`, a button with the caption "Add line" `addLine`. A name code already goes by stays.

Below the groups, the palette lists the window's data. A field of a single record is a tile that places a label and a bound input (and the help text when the field has one and there is room). A list is a tile, Table of name, that places a table bound to it.

For every part, its own properties, events and binding rules are in [the language reference](../LANGUAGE.md).

## Text

- `label`: a caption for an input; `for name` ties it to the input, and clicking it focuses that input. No events.
- `help`: a help text for an input; without a caption it shows the field's `help`, and it shows the error a handler sets.
- `heading`: a heading.
- `markdown`: shows Markdown with its headings, lists, bold, italics, code and links, wrapped, scrolling when it is longer than the part. Its text is a Markdown file of the project's `resources` folder: one for every language with **Source** (`source "license.md"`), or a file per language with **Key** (`markdown @gameRules`, each language file pointing the key at its file, as `gameRules resource "rules-nl.md"`, under i18n). It can also show a text it binds, `bind notes`. A link to an `http`, `https` or `mailto` address opens in the browser. No events.
- `text`: shows the value of a field of any type, `bind customer.name`, or a plain value of the window, `bind shown`. On one row it is one line, cut short when it is too long. Over several rows (`rows 6`) it is text to read: it wraps from the top, keeps its paragraphs (`\n\n` in a translation), and scrolls when it is longer than its rows.

## Basic input

- `textbox` binds to text, email or url; `memo` to text, over several lines; `password` to text, hidden while typed; the eye beside it shows what is typed or hides it again, and the button next to it copies it. Events: `change`, `enter`, `leave`.
- `number` binds to integer, decimal, money, id or duration. Events: `change`, `enter`, `leave`.
- `checkbox` and `switch` bind to bool; `dropdown` to an enum or a `ref`; `radio` to an enum, one button per value; `date` to date or datetime. Event: `change`.
- `button` binds to nothing. Event: `click`. It can have a `key`: `"Ctrl+S"`, or a character such as `"7"` or `"+"`, which the keyboard and the number pad both type (not while typing in a text input), and be the `default` (Enter) or `cancel` (Escape) button. With `on click: digit(7)` it gives its handler values; see [Handlers](code.handlers.md).

`change` fires only when the person changes the value to one that differs. Typed text counts when the input is left or Enter is pressed.

## Containers

- `form`: binds to one record (`bind customer`) and holds the inputs of that record on its own grid. Event: `submit`, when Enter is pressed in one of its inputs.
- `panel`: parts in a frame with the caption. Tick **The person can fold it by its header** under Folding in its properties (`foldable`), and **Starts folded** to start it folded (`foldable folded`); a form folds the same way. Code reads and sets it as `details.folded`.
- `tabs`: `tab @key` blocks, each with parts. Event: `change`. Add tab is in the properties.
- `splitter`: exactly two `pane` blocks, side by side or stacked.

## Data

- `table`: binds to a list (`bind orders`) and shows `columns`. Columns named in `edit` are inputs in the cells. A column can have a width, set by dragging its header edge on the canvas or in the properties; `resizable` lets the person resize the columns (remembered for them), `sortable` sorts by a click on a header, and `order by` gives the order the rows show in first. Events: `select` (another row is chosen), `open` (double-click or Enter), `change` (a cell was edited).
- `list`: binds to a list and shows the fields in `show`. Events: `select`, `open`.
- `chart`: bars, a line or a pie of a list's rows grouped by a field; or, bound to a list of numbers (`data readings: list<decimal>`), the numbers as they are, as bars or a line. **Points** shows a list's last numbers and **Top** fixes the top of the axis.
- `trend`: one number's last values as a line, taken every so many seconds while the window is open: `trend name cpuTrend at 1,1 span 6 rows 4 bind cpu points 60 max 100`. **Points** is how many it keeps (60 without it), **Every (s)** the seconds between them (1 without it), **Top** the top of the axis. The designer shows a still sample line.

## Time

- `timer`: runs its `on tick` handler every so many seconds while its window is open: `timer name clock every 1 on tick: sampled`. It has no place in the window, so the designer keeps it in the tray under the window, with the sounds; pick it there to set **Every (s)** and its handler. `clock.enabled = false` in code stops it, and setting it back to true starts it again. A tick that comes while a handler still runs is left out. A list of numbers grows with `+`, as in `readings = readings + [cpu]`, for a chart to draw; see [The language reference](../LANGUAGE.md#live-graphs).

## Graphics

- `image`: binds to an `image` field, shows a fixed picture of the project's `resources` folder with `source "logo.png"`, or a picture per language through its key (`image @bannerPicture`). Event: `click`. See [Pictures and files](data.pictures-files.md).
- `display`: shows a number in seven-segment digits, as on a calculator, with its dark segments faint: `display name screen at 1,1 span 4 rows 2 bind shown digits 12`. It binds to an integer, decimal or money (a field or a plain value of the window). A value is rounded half away from zero to fit its digits (12 without `digits n`), and shows `E` when its whole part does not fit. Its style gives its colors: `background` behind the digits, `text` for the lit segments.
- `canvas`: free graphics, drawn by its draw handler (`on draw: drawDial`) in its own units (`units 200 x 100`, 100 x 100 without it): lines, rectangles, circles, rings and text in the theme's colors. The handler runs when the window opens and after every other handler, so the drawing follows the data; see [Handlers](code.handlers.md). With `on click: handler` it can be clicked, and the handler reads where. Or an engine draws it frame by frame: `runs Screen`, an engine declared `beside the screen` (see [Engines](code.engines.md)). Without either it shows as a placeholder.

When the window is a `window` or the app, a Frame group comes first, with the menu, toolbar, ribbon, sidebar and footer; see [The app and its frame](design.frame.md).

## The properties panel

The inspector on the right has the tabs Properties, Code and Text ([Design as text](design.text.md)). Properties shows what is selected. Click a part on the canvas, or a line in the sidebar's Window tab, to select it and see its properties and events. Click the window's background or title bar to select the window itself: its title, grid columns, size, data and events (see [Windows](design.windows.md)).

What is picked shows the fields that part has:

- Name, Caption, and Column, Row, Width and Height.
- Bound to: only the data and fields that part accepts. Columns (each with its width) or Shown fields, Edited in the table, and Table (resizable, sortable, the first order), for tables and lists.
- On click, open (and In a new window) for buttons and images; For input for labels and help texts; Placeholder, Tooltip, Style, Tab order, Shortcut; Read-only and the default and cancel boxes.
- Events: a row per event. **Wire** names the handler after the part and the event, writes it and opens the Code panel; **Code** shows it; the cross unwires it. See [Handlers](code.handlers.md).
- Remove.

Every change is written to the file at once. **Delete** in the ribbon, or the Delete key, removes the picked part.
