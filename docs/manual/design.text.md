# Design as text

The inspector's Code and Text tabs: a window as Clarity text, with the canvas following what you type, and the code of what is picked.

## The inspector

On the right of the designer is the inspector, with three tabs about what is selected on the canvas: a part, or the window itself when you click its background or title bar.

- **Properties**: the selected part's properties and events, or the window's title, size, data and events.
- **Code**: its code behind, the handlers of its events; see [Code per part](#code-per-part).
- **Text**: the window's own declaration as Clarity text, its `page` block (or the `app` block when you design the app).

The Code and Text tabs reach down to the bottom. The inspector keeps one width whatever tab it shows; drag its left edge to change it. Escape in the text goes back to Properties, and **Hide** folds the inspector away.

## Code per part

The Code tab shows the code of what is picked on the canvas: the handlers of that part's events, each with the handlers it calls, such as a canvas's `drawBoard` with its `draw_mark`. With nothing picked it shows the handlers of the window's own events (`on open`, `on close`). The line above the code says whose it is, as `board: on draw, on click`; tick **Whole window** to see all of the window's code. Saving puts what you typed back in place of those handlers, and the rest of the code stays as it was.

Double-click a part on the canvas to go to the code of its main event: a button's click, a canvas's drawing, an input's change. When that event has no handler yet, Clarity writes one named after the part and the event (`saveClick`), wires it, and opens it, ready for you to fill in. A part needs a name for that; give it one in its properties. The other events are under Events in the part's properties, each with **Wire** or **Code**.

## The designer follows the text

In the Text tab, type as you like: move a part by changing its `at`, add a line for a new part, change `: window` to `: dialog`. Shortly after you stop typing, Clarity checks the text. When it is one valid declaration, the canvas shows the window as the text would make it, before you save. A note under the text says so: "The designer shows this text. Save it to keep it, or Revert."

```clarity
page CustomerForm: dialog
    title @customerFormTitle
    data customer: Customer

    label @nameLabel name nameLabel at 1,1 span 3 for name
    textbox name name at 4,1 span 6 bind customer.name
    button @saveButtonCaption name saveButton at 11,3 span 2 default on click: saveClicked
```

While the canvas shows unsaved text, it does not take changes of its own: dragging or editing a property there says to save or revert the text first.

## Save and Revert

- **Save** (or Ctrl+S in the text) writes the text into the file. It is one designer edit, so **Undo** takes it back. Save is off while the text has errors.
- **Revert** puts back the saved text, and the canvas shows the saved window again.

The bar above the text says "unsaved" while the text differs from the file. Switching between the tabs keeps unsaved text in Code and Text.

When the file changes while a tab shows its text (from the canvas, the agent or another editor), the text loads again, unless you have unsaved text there.

## Problems

The line numbers on the left are marked for lines with an error or a warning. Below the text, the tab lists the problems with their line and column; click one to put the cursor on that line. These are the same checks as the Problems list, run on the text you are typing.

## The Code panel

The Code tab edits the window's handlers in the same way, checked while you type, with Save and Revert. Above the text is a button per handler; click one to jump into it. **Wire** and **Code** next to an event in the properties panel open this tab at that handler. What a handler can do is in [Handlers](code.handlers.md).

## Typing in the Code and Text tabs

- Tab puts in four spaces; Clarity uses spaces for indentation, never tabs.
- Enter keeps the indentation of the line, and adds one level after a line that opens a block, such as `handler`, `if`, `form` or `menu`.
- Ctrl+S saves; Escape goes back to Properties.

Modules, functions and themes have editors of their own that work the same way; see [Functions and actions](code.functions.md) and [Themes](themes.md). For the full syntax, see [the language reference](../LANGUAGE.md).
