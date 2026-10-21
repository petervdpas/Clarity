# Handlers

How the code behind a window works: handlers wired to events, part properties, messages, opening and closing windows, and the Code panel.

## The code behind a window

A window's handlers live in a `code` block with the window's name, in the same file as the window. A handler has a name, and parameters when a part gives it values. It runs when an event wired to it happens.

```clarity
code CustomerForm
    handler kindChanged
        vatNumber.enabled = customer.kind == Business

    handler saveClicked
        name.error = ""
        if customer.name == ""
            name.error = "Name is required"
            return
        save customer
        message "Saved {customer.name}"
        close
```

A part wires an event with `on click: saveClicked`; a window wires its own with `on open:`, `on close:` and `on return:`. The app's handlers, such as `on start`, are in a `code` block named after the app. A handler that is wired but not written is an error; one that is written but wired nowhere is a warning.

## Values from a click

One handler can serve many parts when each gives it a value. The ten digit buttons of a calculator share one handler:

```clarity
    button "7" at 1,4 span 1 on click: digit(7)
    button "8" at 2,4 span 1 on click: digit(8)
    button "+" at 4,6 span 1 on click: operation(Add)

code Main
    handler digit(d: integer)
        shown = shown * 10 + d

    handler operation(op: Operation)
        pending = op
```

A parameter is `text`, `integer`, `decimal`, `money`, `bool` or an enum, and a value is written as in code: `7`, `0.5`, `"text"`, `true`, or a value of the enum. The checker says when a part gives too few or too many values, or one of the wrong type. A handler that takes values runs only from parts that give them, so the window's own events (`on open`) wire handlers without parameters. Another handler runs it with values too: `digit(3)`.

Handlers run in the backend. The window sends its data with the event, the handler runs, and the window then shows what changed: data, part properties, messages, and windows to open or close.

## What a handler can use

- The window's data, by its `data` names: `customer`, `orders`, and its plain values: `shown`.
- Its parameters, by name.
- The window's parts, by their names: `save`, `vatNumber`.
- Its own locals, made with `let`.
- Functions and actions of its module and of the modules it uses; see [Functions and actions](code.functions.md).

A name that is both data and a part in the same window is an error.

## Part properties

| Property | On | Can be set |
|---|---|---|
| `visible`, `enabled` | every part | yes |
| `caption` | labels, help texts, headings, buttons, commands | yes |
| `error` | inputs; `""` clears it | yes |
| `readonly` | inputs | yes |
| `text` | text boxes, memos and password inputs that are not bound | yes |
| `checked` | checkboxes and switches that are not bound | yes |
| `selected` | tables and lists: the record of the chosen row | no |

A bound input changes through its data: `customer.name = "Ada"` changes the text box bound to `customer.name`. When you set an input's `error`, its help text shows the message and the input is marked.

## Drawing on a canvas

A canvas is drawn by its draw handler: `canvas name dial at 1,1 span 12 rows 7 on draw: drawDial`. The handler draws shapes in the canvas's own units, x to the right and y down, 100 x 100 unless the canvas says `units 200 x 100`:

```clarity
code Main
    handler drawDial
        let angle = 210 - speed * 240 / 200
        dial.clear("panel")
        dial.ring(50, 50, 42, "border", 3)
        dial.line(50, 50, 50 + 34 * cos(angle), 50 - 34 * sin(angle), "accent", 3)
        dial.circle(50, 50, 4, "accent")
        dial.text(50, 72, "{speed} km/h")
```

The shapes are `clear(color)`, `line(x1, y1, x2, y2, color, width)`, `rect(x, y, width, height, color)`, `circle(x, y, r, color)`, `ring(x, y, r, color, width)` and `text(x, y, text, color, size)`; a width can be left out (1), and so can a text's color and size (`"text"`, 10). A color is a color of the theme by name or written out, `"#101810"`. `sin` and `cos` take degrees.

The draw handler runs when the window opens and after every other handler of the window, each time on an empty canvas, so it only says what the canvas shows for the data as it is. The canvas scales its units to fit the part, keeping their proportion, on every stack alike.

Only a canvas's own draw handler draws on it, and the handlers it calls that nothing else runs. That lets a drawing of many alike things say each once: the noughts example draws its board with `draw_mark(square, left, top)` nine times, and `draw_mark` draws a cross as two lines or a nought as a ring.

### Clicking a canvas

`on click: boardClicked` on a canvas runs the handler when the canvas is clicked. `board.x` and `board.y` say where, in the canvas's own units, rounded down; they are 0 before the first click. A click beside the drawing, where the part shows none of it, runs nothing. After the handler the canvas is drawn anew, so it shows what the click changed:

```clarity
    handler boardClicked
        let square = 1
        if board.x >= 200
            square = 3
        else if board.x >= 100
            square = 2
        if board.y >= 200
            square = square + 6
        else if board.y >= 100
            square = square + 3
        status = Board.play(square)
```

## Messages, windows and data

- `message "Saved {customer.name}"` shows a message. Write `message warning ...` or `message error ...` for another level; the default is info. Values go in slots, see [The language](code.language.md).
- `open CustomerForm` opens a window on top of this one. `open CustomerForm with customer = grid.selected` fills its data by name.
- `close` closes this window. The window below then runs its `on return` handler, the place to read a list again.
- `return` leaves the handler at once.
- `reload customers` reads the window's data `customers` again: a list loads all its records, a record loads again by its id.
- `filterChanged()` runs another handler of the same window or app.
- `save` and `delete` store and remove records; see [Data events](code.data-events.md).

```clarity
code CustomerBrowse
    handler windowReturned
        filterChanged()

    handler openClicked
        open CustomerForm with customer = grid.selected
```

## The Code panel

On the **Design** tab, the **Code** button in the Window group slides the Code panel out over the window you design. It shows the window's handlers as text, with a button per handler to jump to it. Problems are marked by line while you type. **Save** (Ctrl+S) writes the code and stays off while there are errors; **Revert** puts back the saved text. A save is one designer edit, so Undo takes it back.

In the part properties, each event has a field for its handler; typing a name there wires that handler and writes it empty if it is new. **Wire** names a handler after the part and the event (`saveClick` for the `click` of a button named `save`), writes it empty into the code behind and opens it in the inspector's Code tab. **Code** next to a wired event jumps to its handler. With no part picked, the properties show the window's own events in the same way.

The full rules are in [the language reference](../LANGUAGE.md#8-code-behind).
