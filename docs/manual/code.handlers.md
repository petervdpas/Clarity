# Handlers

How the code behind a window works: handlers wired to events, part properties, messages, opening and closing windows, and the Code panel.

## The code behind a window

A window's handlers live in a `code` block with the window's name, in the same file as the window. A handler has a name and no parameters. It runs when an event wired to it happens.

```
code CustomerForm
    handler kind_changed
        vat_number.enabled = customer.kind == Business

    handler save_clicked
        name.error = ""
        if customer.name == ""
            name.error = "Name is required"
            return
        save customer
        message "Saved {customer.name}"
        close
```

A part wires an event with `on click: save_clicked`; a window wires its own with `on open:`, `on close:` and `on return:`. The app's handlers, such as `on start`, are in a `code` block named after the app. A handler that is wired but not written is an error; one that is written but wired nowhere is a warning.

Handlers run in the backend. The window sends its data with the event, the handler runs, and the window then shows what changed: data, part properties, messages, and windows to open or close.

## What a handler can use

- The window's data, by its `data` names: `customer`, `orders`.
- The window's parts, by their names: `save`, `vat_number`.
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

## Messages, windows and data

- `message "Saved {customer.name}"` shows a message. Write `message warning ...` or `message error ...` for another level; the default is info. Values go in slots, see [The language](code.language.md).
- `open CustomerForm` opens a window on top of this one. `open CustomerForm with customer = grid.selected` fills its data by name.
- `close` closes this window. The window below then runs its `on return` handler, the place to read a list again.
- `return` leaves the handler at once.
- `reload customers` reads the window's data `customers` again: a list loads all its records, a record loads again by its id.
- `filter_changed()` runs another handler of the same window or app.
- `save` and `delete` store and remove records; see [Data events](code.data-events.md).

```
code CustomerBrowse
    handler window_returned
        filter_changed()

    handler open_clicked
        open CustomerForm with customer = grid.selected
```

## The Code panel

On the **Design** tab, the **Code** button in the Window group slides the Code panel out over the window you design. It shows the window's handlers as text, with a button per handler to jump to it. Problems are marked by line while you type. **Save** (Ctrl+S) writes the code and stays off while there are errors; **Revert** puts back the saved text. A save is one designer edit, so Undo takes it back.

In the part properties, each event has a field for its handler; typing a name there wires that handler and writes it empty if it is new. **Wire** names a handler after the part and the event (`save_click` for the `click` of a button named `save`), writes it empty into the code behind and opens the panel there. **Code** next to a wired event jumps to its handler. With no part picked, the properties show the window's own events in the same way.

The full rules are in [the language reference](../LANGUAGE.md#8-code-behind).
