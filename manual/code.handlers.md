# Handlers

How the code behind a window works: handlers wired to events, the window's and the app's events, what a handler sees, part properties, undo, asking the person, data events that run on every save and delete, and the Code panel.

## The code behind a window

A window's handlers live in a `code` block with the window's name, in its module's `.clarity` file; the window's own `.cxaml` file wires its events to them. A handler has a name, and parameters when an event gives it values. It runs when an event wired to it happens.

```clarity
code CustomerForm
    handler kindChanged
        // Only business customers have a VAT number.
        vatNumber.enabled = customer.kind == Business

    handler saveClicked
        name.error = ""
        if customer.name == ""
            name.error = @nameIsRequired
            return
        save customer
        message @savedName(name = customer.name)
        close
```

A part wires an event in its window's file: `onclick="saveClicked"`, `onchange="kindChanged"`. A window wires its own on its root with `onopen`, `onclose` and `onreturn` (when a window it opened has closed: the place to read a list again). The app wires `onstart` and `onunconnected` on the root of its file, and its handlers are in a `code` block named after the app. A view's handlers are in a `code` block named after the view; a view's code joins the code of the window it shows in, so a part of the view may run a handler of that window, and the window's handlers may run the view's.

Handlers run in the backend. The window sends its data with the event, the handler runs, and the window then shows what changed: data, part properties, messages, and windows to open or close.

**Fail safe.** A handler that stops on an error (a refusal, a record that is gone, a list position out of range, anything the runtime did not foresee) shows the error as an error message, and the application keeps working: the next handler runs as usual, on every stack. A part that cannot be drawn shows its name and the reason in its own place, and the rest of the window works.

### `code`

    code Window
        handler name[(parameter: type, ...)]
            statements
    code Entity
        before save | after save | before delete | after delete
            statements

- A `code` block is named after a window, view, dialog or the app of its module, or after an entity of the same file (then it holds that entity's [data events](#data-events)).
- "there is already a code block for CustomerForm"; "there is no window, app or entity 'CustomerFrom' in this file"; "'before save' belongs in the code of an entity; CustomerForm is a window".

### `handler`

```text
handler name
handler name(parameter: type, ...)
```

- A handler's name and its parameters are camelCase: name it after what it is for, `saveClicked`, `newGameClicked`.
- Handlers of one window are named once each, and none has the name of a handler of the app: "handler 'saveClicked' is written twice"; "handler 'refresh' of Main has the name of a handler of the app Shop; give one of them another name".
- A handler that is wired but not written is a warning, "handler 'saveClicked' is wired but not written yet", and so is one written but neither wired nor run by another handler: "handler 'saveClicked' is not wired to any event".
- A handler that asks the person and closes its window is an error: "handler 'deleteClicked' asks and closes its window: the answer would have nowhere to go".

## Values from an event

One handler can serve many parts when each gives it a value. The digit keys of a calculator share one handler:

```xml
<!-- calculator/design/Keypad.cxaml -->
<View class="calculator.Keypad" columns="4">
  <Data name="shown" type="decimal" />
  <Data name="pending" type="Operation" />
  <Button name="digit7" at="1,4" span="1" caption="7" key="7" onclick="digit(7)" />
  <Button name="digit8" at="2,4" span="1" caption="8" key="8" onclick="digit(8)" />
  <Button name="divideKey" at="4,3" span="1" caption="÷" key="/" onclick="operation(Divide)" />
</View>
```

```clarity
code Keypad
    handler digit(d: integer)
        shown = shown * 10 + d

    handler operation(op: Operation)
        pending = op
```

- A value a part gives is `text`, `integer`, `decimal`, `money`, `bool` or an enum, written as in code: `7`, `0.5`, `"text"`, `true`, or a value of the enum. As many values as the handler has parameters, each of its type: "'on click: digit(7, 8)' gives 2 values; handler digit takes 1 (d: integer)"; "'on click: digit("7")': d is an integer, not text"; "Operation has no value Divde".
- The window's own events (`onopen`, `onclose`, `onreturn`) and the app's `onstart` give no values, so their handlers take none: "handler loaded takes values (n: integer); 'on open' has none to give".
- The app's `onunconnected` handler runs when the databases did not open, and may take one `text`: why.
- A tell of a custom part, of an engine beside the screen or of a view gives the tell's values: "tell scored of part Arena gives (points: integer); handler arenaScored takes ()". A tree's row of an entity, on select, gives its record: `handler projectPicked(p: Project)`.
- A handler that the answer to a question runs may take dates, durations and records too; see [Asking the person](#asking-the-person).
- Another handler runs it with values: `digit(3)`.

## What a handler sees

- The window's data, by its names: `customer`, `orders`, and its plain values: `shown`.
- Its parameters, by name.
- The window's parts, by their names: `save`, `vatNumber`.
- Its own locals, made with `let`.
- Enum values, bare (`Business`) where the expected type says which enum is meant, and qualified (`CustomerKind.Business`) where it does not.
- Constants, `settings` and, with sign-in, `me`.
- Functions, actions and engines of its own module and of the modules it uses; see [Functions and actions](code.functions.md).

A name that is both data and a part in the same window is an error: "'customer' is both data and a part of CustomerForm". See [Expressions](code.expressions.md#names-and-paths) for every kind of name.

## Part properties

Code reads and sets these properties of the window's parts by name, `vatNumber.enabled = false`, in handlers.

| Property | On | Type | Can be set |
|---|---|---|---|
| `visible`, `enabled` | every part, the menu, the toolbar, and a ribbon's tabs and groups | bool | yes |
| `caption` | labels, help texts, headings, buttons, commands | text | yes |
| `error` | inputs; on a status item, the warning it shows | text | yes; `""` clears it |
| `readonly` | inputs | bool | yes |
| `text` | text boxes, memos and password inputs that are not bound | text: what the person typed | yes |
| `checked` | checkboxes and switches that are not bound | bool: whether it is on | yes |
| `selected` | tables, lists, trees, tree tables and graphs | the record of the selected row; with none chosen an empty record, as `new` makes (its `id` is 0), so `if grid.selected.id == 0` asks first | yes: the first row that is that record (a stored record by its id, a record by its values), or `none` for no row |
| `folded` | foldable panels and forms | bool: whether it is folded, by the person or by code | yes |
| `collapsed` | collapsible sidebars and ribbons | bool: whether a sidebar is folded to its button, a ribbon to its tabs | yes |
| `maximized` | maximizable sidebars | bool: whether it fills the window | yes |
| `pinned` | pinnable slide-outs | bool: pinned open beside the window's parts | yes |
| `tab` | ribbons, tabs, a window's content | integer: the open tab, or the view the content shows, counted from 1 | yes |
| `step` | wizards (`steps`) | integer: the step shown, counted from 1 | yes |
| `open` | accordions | integer: the open section, counted from 1; 0 when none is open | yes |
| `min`, `max` | spin boxes | integer: its range, as its file says until code sets it | yes |
| `foldButtons` | trees and tree tables | bool: whether Expand all and Collapse all show above its rows | yes |
| `caret` | text boxes and memos | integer: where the caret is, as how many characters stand before it (0 at the start) | yes: the caret moves there, selecting nothing |
| `selectedText` | text boxes and memos | text: what the person selected; empty when nothing is | no |
| `link` | text boxes, memos and markdown parts | text: the address of the link last clicked in it | no |

A custom part's own properties are read and set the same way (`arena.speed = 7`); see [Custom parts](design.custom-parts.md). A canvas has properties of its own (where it was clicked, its grid, its tool); a sound part and a video part report what they play; see [Parts](design.parts.md), [Sound](design.sound.md) and [Video](design.video.md).

A bound input changes through its data: `customer.name = "Ada"` changes the text box bound to `customer.name`, and setting its `text` is refused: "textbox 'name' is bound to customer.name; use customer.name". When you set an input's `error`, its help text shows the message and the input is marked.

The checker says "button 'save' has no property 'colour'", "button 'save' is a part; set one of its properties", and "manuscriptBox.selectedText is read, not set: it is what the person chose".

### Text at the caret

A text box or memo takes text from code where the person is: `manuscriptBox.insert(text)` puts it where the selection is (at the caret when nothing is selected), replacing the selection, as one step Ctrl+Z takes back. It goes to the field the memo binds, without its `on change`. So a tool inserts at the caret: `manuscriptBox.insert("> " + manuscriptBox.selectedText)` turns the selection into a quote.

A memo can have a toolbox of such tools, a page of its own as a canvas's is: `toolbox="caret"` and `toolboxPage="WritingTools"` on the memo, and a `<Toolbox class="novel.WritingTools">` file holding tools with `onclick` (`<Tool name="quoteTool" at="1,1" span="1" icon="quote" onclick="quoteClicked" />`), buttons and labels. With `caret`, Ctrl+Space calls it up just under the caret, and a right-click moves the caret where it was pressed and calls it up there, with the memo's Cut, Copy and Paste as its last row. A tool's handler runs on the text where the person is, then the toolbox goes; a press beside it or Escape closes it too. `toolbox="popup"` floats it over the window instead, and `toolbox="slideout"` against the memo's edge, while the memo may be written in (in a markdown memo, while it shows Write).

A passage put in from elsewhere can keep where it came from as a link into the application: `manuscriptBox.insert("[" + f.text + "](clarity:fragment/" + f.id + ")")`. The preview, or a markdown part, shows it as a link; a click on it opens no browser but runs the part's `onlink` handler, which reads the address as `manuscriptBox.link` (`clarity:fragment/412`) and goes there.

## Messages, windows and data

- `message @savedName(name = customer.name)` shows a message by its key: a toast at the top right that goes by itself after 5 seconds, or by its small x. `message warning ...` or `message error ...` shows in Clarity's dialog and is closed with OK.
- `toast saved(name = customer.name)` shows a toast the project declares, with its own lifespan, level, icon and corner: see [Toasts](code.toasts.md).
- `open CustomerForm with customer = grid.selected` opens a window over this one and fills its data by name. `close` closes this window, and the window below runs its `onreturn` handler.
- `reload customers` reads the window's data again; `reload customer` of a local or a parameter reads that record again as it is stored now.
- `filterChanged()` runs another handler of the same window or app.
- `save` and `delete` store and remove records, and run the entity's [data events](#data-events). A save of a record someone else saved since it was read is refused with Clarity's `savedSince` text; `save customer anyway` writes regardless.
- `copy order.number` puts a text on the clipboard once the handler has run.

```clarity
code CustomerBrowse
    handler windowReturned
        filterChanged()

    handler filterChanged
        customers = find c in Customer where c.name contains nameFilter.text order by c.name

    handler openClicked
        open CustomerForm with customer = grid.selected
```

Each statement, with where it may stand and what the checker says, is on [Statements](code.statements.md).

## Asking the person

A handler can ask the person something: a yes or no, a text, a number, a day, a value of an enum or a record. The question shows in Clarity's own dialog once the handler has finished, never one of the system or the browser, and the answer runs another handler:

```clarity
code AlbumCard
    handler deleteClicked
        ask @deleteAlbum(title = album.title) yes @delete no @keep then deleteConfirmed

    handler deleteConfirmed
        delete album

    handler lendClicked
        ask @lendTo for Friend from friends then lend(album)

    handler lend(what: Album, friend: Friend)
        what.copies = what.copies - 1
        save what
```

- `yes @word no @word` asks yes or no, and the handler runs on yes, taking no answer. `for` a type asks for a value of it, and the answer comes last, after the values written in the call: `then lend(album)` runs `lend(album, friend)`.
- A dialog of your own answers too: `ask PickFriend then lend(album)` opens the dialog, and what its handler closes it with (`close with grid.selected`, in a dialog whose file says `gives="Friend"`) is the answer.
- Cancelling (no, Escape, a click beside the dialog, a dialog closed without a value) runs nothing.
- Only handlers ask with `then`; a command asks on the terminal (see [Commands](code.commands.md#ask-in-a-command)).

Every form of `ask` and what the checker says of it is under [`ask`](code.statements.md#ask).

## Undo

Every part the person edits takes back its own edits with Ctrl+Z, and does them again with Ctrl+Y (or Ctrl+Shift+Z), on every stack, with nothing to write.

- **What one edit is.** A click on a canvas, its drag and drop (from the first drag to the drop), a bent line, a new line drawn between boxes, a box resized by its grip, a table cell, a change of an input (bound or not), a picture loaded into an image or removed, a row moved in a part that reorders. A list or tree that does not reorder only selects, so it has no undo, nor a custom part, which brings its own, nor a canvas's `onpointer` and `onkey`. Its undo puts back the window's data as it was before the edit, and what the handler of that edit (`ondrop`, `onbend`, `onconnect`, `onresize`, `onchange`, `onreorder`) saved or deleted, cascades included.
- **The whole window.** An undo also puts back what the edit's handler set on parts: a caption, `visible`, `enabled`, `readonly`, an error, and each value of a custom part (`board.count = count` in a slider's `onchange`), value by value.
- **Where.** Ctrl+Z acts on the part with the focus. In a `form` or a `panel`, it walks back the fields one by one, the last first. In a text input, its own text undo comes first while it has something to take back.
- **A menu item or a button** reaches it with the built-in targets `undo` and `redo` (`opens="undo"` on a menu item with `key="Ctrl+Z"`). They act on the part that had the focus last; see [The app and its frame](design.frame.md).
- **Not taken back:** moving around (a tab, folding, zoom and pan, the selected row), what a click or a timer did, and effects outside the databases and the window (files written, messages, the clipboard, calls to other services).
- **`noUndo="true"`** on a part switches its undo off.
- **Safe.** When what an undo would take back was changed since (by another edit, window or person), it is refused with a message (Clarity's word `undoChangedSince`) and changes nothing. It compares the record's version where its entity has one (see [Entities](data.entities.md)), the whole row where it has none.

## Data events

An entity's `code` block holds its data events: code that runs whenever one of its records is saved or deleted, from a handler, an action, a table edit, a command or another data event. The block has the entity's name and is in the same file as the entity.

```clarity
code Order
    before save
        if order.status == Approved and not withinCredit(order)
            refuse @orderOverCreditLimit(number = order.number)

    after save
        logChange("saved " + order.number)

    before delete
        if order.status != Pending
            refuse @onlyPendingDeletable
```

### `before save`

Runs before a record is stored; what it changes in the record is stored too. A new record has id 0 here, so `order.id == 0` tells you it is new.

### `after save`

Runs after a record is stored; a new record has its id by then.

### `before delete`

Runs before a record is removed.

### `after delete`

Runs after a record is removed.

- **The record** is named after the entity in camelCase: `order` in the events of `Order`, `orderLine` in those of `OrderLine`.
- **An event is an action** on its record. It can read and change records, `save` and `delete` others, call functions and actions, and reach files; it declares no effects. It does not talk to the screen: `message`, `open`, `close` and part properties are not allowed ("'message' belongs in a handler, not in ..."). It does not call out with `http`: "a data event cannot call out: it runs inside the save's transaction, which must not wait on the network".
- **`refuse "text"`** stops the save or delete. The text is the error the window shows, so write it for the person using the application. It can hold slots: `refuse "{loan.album.title} is with another friend already"`.
- **All or nothing.** The save or delete runs in one transaction with its events, and with the saves and deletes they make in the same database. When an event fails or refuses, none of it is kept, and a new record is still new.
- Each event is written once per entity, and an entity's code block holds no handlers: "'before save' is written twice"; "the code of entity Order holds data events (before save, after save, before delete, after delete), not handlers"; "unknown event 'before update'; the events are before save, after save, before delete and after delete".
- An event that saves its own record in a save event, or deletes it in a delete event, would run itself again without end: "saving order in its own 'before save' would run the event again, without end".
- Renaming an entity renames its code block and the record's name in its events.

In the IDE, open a module from Modules in the project tree. Under **Data events** each entity of the module is listed. **Add events** writes the entity's code block with a first `before save` and opens it as text; an entity that has events shows which ones, and opens them when you pick it. The Code panel of the window designer does not show data events.

## Drawing on a canvas

A canvas is drawn by its draw handler, wired with `ondraw="drawDial"`. The handler draws shapes in the canvas's own units, x to the right and y down, 100 x 100 unless the canvas says other units:

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

The shapes include `clear(color)`, `line(x1, y1, x2, y2, color, width)`, `rect(x, y, width, height, color)`, `circle(x, y, r, color)`, `ring(x, y, r, color, width)`, `text(x, y, text, color, size, align)` and `image(x, y, width, height, picture)`; a width can be left out (1), and so can a text's color, size and alignment (`"text"`, 10, `"center"`). A text is centered on x; with `"left"` it starts at x and with `"right"` it ends there. `length(text)` says how many characters a text has, so a box can be as wide as its longest line. An image draws a picture of the resources folder by its name, an SVG among them, stretched to fill its box; a picture constant (`const pill: picture = "pill.svg"`) names one the checker has found. A color is a color of the theme by name or written out, `"#101810"`. `sin` and `cos` take degrees.

The draw handler runs when the window opens and after every other handler of the window, each time on an empty canvas, so it only says what the canvas shows for the data as it is. The canvas scales its units to fit the part, keeping their proportion, on every stack alike. Only a canvas's own draw handler draws on it, and the handlers it calls that nothing else runs: "'dial' is drawn only by its draw handler, drawDial, and the handlers only it runs".

`onclick="boardClicked"` on a canvas runs the handler when the canvas is clicked. `board.x` and `board.y` say where, in the canvas's own units, rounded down; they are 0 before the first click. A click beside the drawing runs nothing. After the handler the canvas is drawn anew.

Every shape, the canvas's modes and its other events are on [Parts](design.parts.md).

## The Code panel

On the **Design** tab, the **Code** button in the Window group slides the Code panel out over the window you design. It shows the window's handlers as text, with a button per handler to jump to it. The text is colored as Clarity reads it, in every code editor: words of the language, types, names, texts with their slots, keys and comments, each in a color of its own. Problems are marked by line while you type. **Save** (Ctrl+S) writes the code and stays off while there are errors; **Revert** puts back the saved text. A save is one designer edit, so Undo takes it back.

In the part properties, each event has a field for its handler; typing a name there wires that handler and writes it empty if it is new. **Wire** names a handler after the part and the event (`saveClick` for the `click` of a button named `save`), writes it empty into the code behind and opens it in the inspector's Code tab. **Code** next to a wired event jumps to its handler. With no part picked, the properties show the window's own events in the same way.

See also [Statements](code.statements.md), [Expressions](code.expressions.md) and [Functions and actions](code.functions.md).
