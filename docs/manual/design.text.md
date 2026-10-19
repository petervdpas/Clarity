# Design as text

The text panel of the designer: a window as Clarity text, with the canvas following what you type, and the code behind next to it.

## The text panel

Every window is plain Clarity text in its module, and you can edit that text without leaving the designer. On the **Design** tab, with a window or the app open, click **Text** in the ribbon. A panel slides out over the canvas with two tabs:

- **Design**: the window's own declaration, its `page` block (or the `app` block when you design the app).
- **Code**: the window's code behind, its `code` block.

**Code** in the ribbon opens the same panel at the Code tab. Click the button again, the cross in the panel's header, or press Escape in the text, to close it. Drag the panel's left edge to make it wider.

## The designer follows the text

In the Design tab, type as you like: move a part by changing its `at`, add a line for a new part, change `: window` to `: dialog`. Shortly after you stop typing, Clarity checks the text. When it is one valid declaration, the canvas shows the window as the text would make it, before you save. A note under the text says so: "The designer shows this text. Save it to keep it, or Revert."

```
page CustomerForm: dialog
    title "Customer"
    data customer: Customer

    label "Name" at 1,1 span 3 for name
    textbox name name at 4,1 span 6 bind customer.name
    button "Save" name save at 11,3 span 2 default on click: save_clicked
```

While the canvas shows unsaved text, it does not take changes of its own: dragging or editing a property there says to save or revert the text first.

## Save and Revert

- **Save** (or Ctrl+S in the text) writes the text into the file. It is one designer edit, so **Undo** takes it back. Save is off while the text has errors.
- **Revert** puts back the saved text, and the canvas shows the saved window again.

The bar above the text says "unsaved" while the text differs from the file. Switching between Design and Code keeps unsaved text in both. Closing the panel drops text you did not save.

When the file changes while the panel is open (from the canvas, the agent or another editor), the text loads again, unless you have unsaved text there.

## Problems

The line numbers on the left are marked for lines with an error or a warning. Below the text, the panel lists the problems with their line and column; click one to put the cursor on that line. These are the same checks as the Problems list, run on the text you are typing.

## The Code panel

The Code tab edits the window's handlers in the same way, checked while you type, with Save and Revert. Above the text is a button per handler; click one to jump into it. **Wire** and **Code** next to an event in the properties panel open this tab at that handler. What a handler can do is in [Handlers](code.handlers.md).

## Typing in the panel

- Tab puts in four spaces; Clarity uses spaces for indentation, never tabs.
- Enter keeps the indentation of the line, and adds one level after a line that opens a block, such as `handler`, `if`, `form` or `menu`.
- Ctrl+S saves; Escape closes the panel.

Modules, functions and themes have editors of their own that work the same way; see [Functions and actions](code.functions.md) and [Themes](themes.md). For the full syntax, see [the language reference](../LANGUAGE.md).
