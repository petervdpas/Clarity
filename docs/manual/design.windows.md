# Windows

How a window is declared and designed: its kind, its data and events, the snap grid, and how one window opens another.

## A window is a page block

A window is a `page` block in a module. Its settings come first, then its parts:

```clarity
page CustomerForm: window
    title @customerFormTitle
    data customer: Customer
    data orders: list<Order>
    grid 12 columns
    on open: windowOpened
    on return: windowReturned

    label @nameLabel name nameLabel at 1,1 span 3 for name
    textbox name name at 4,1 span 6 bind customer.name
    table name orderTable at 1,3 span 12 rows 6 bind orders columns number, total
    button @saveButtonCaption name saveButton at 11,10 span 2 default on click: saveClicked
```

The kind after the colon is `window` (a window of its own, which can have a menu, toolbar and other frame regions), `dialog` (a modal window) or `view`. Only a `window` has frame regions; see [The app and its frame](design.frame.md).

To add one, click **New window** on the Project or Design tab. Give it a name and choose what it starts from: a blank window, Form for an entity (a label and an input per field, with Save and Close buttons) or Browse for an entity (a filter, a table, and New and Open buttons, with the code that filters and refreshes). The result is an ordinary window. New window always makes a `window`; to make a dialog, change `: window` to `: dialog` in the text (see [Design as text](design.text.md)). A dialog that answers a question says what it gives, `: dialog gives Friend`, and its handler closes it with one: `close with grid.selected`; `ask PickFriend then lend(album)` opens it from another window, and the answer runs `lend` there (see [Code behind](code.handlers.md)).

## Data and events

`data name: Entity` is one record the window works on; `data name: list<Entity>` is a list. Parts bind to the data and handlers read and change it.

A window can also hold plain values, which are no records and are never stored: a number it shows, a choice, a flag. They live as long as the window:

```clarity
page Main: window
    data shown: decimal
    data pending: Operation
    data fresh: bool, default true
    text name display at 1,1 span 4 bind shown
```

A plain value is `text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, `duration` or an enum, and starts at its `default`, or empty (0, empty text, false, or the enum's first value). A part binds it by its name, `bind shown`, and handlers read and change it like a local. A plain `decimal` shows the digits it has.

In the designer, add data under The window in the properties panel: a name, and what it holds: an entity (with list ticked or not) under Records, or a type or enum under Values. Each data item is listed in the Parts panel, to drag onto the window: a record's fields, a list as a table, a plain value as one part bound to it.

Data that is not filled when the window opens starts as a new, empty record, for a list as all records of the entity, and for a plain value at its default.

A window has three events of its own, listed under Events in The window:

- `on open`: the window has opened and its data is loaded.
- `on return`: a window this one opened has closed, and this one is on top again. This is where a list reloads after a form saved a record.
- `on close`: the window is about to close.

**Wire** fills in a handler name (`windowOpened`, `windowReturned`, `window_closed`), writes an empty handler and opens it in the Code panel. See [Handlers](code.handlers.md).

## Size

`size 320 x 460` is the size the window's desktop window opens at, in pixels: 200 to 3840 wide and 150 to 2160 high. Without it a window opens at 1100 x 760. The desktop window takes the size of the window it shows first; a window opened in place keeps the size it has. Set it as Size under The window in the properties, or drag the window's right edge, bottom edge or bottom-right corner in the designer; the size shows while you drag. The designer shows the window at its size.

## The grid

Every part sits on a snap grid of 12 columns, or the number `grid N columns` gives (1 to 24, set as Grid columns in the properties). A part's place is `at col,row`, counted from 1, and its size is `span n` columns and `rows n` rows. Containers have a grid of their own, as wide as their span.

In the designer, drag a part to move it and drag its corner handle to resize it. The **Grid** button on the Design tab shows or hides the grid cells. The checker reports parts that start outside the grid, run past its last column, or overlap another part.

Rows have a fixed height in the designer. In the running application the columns share the window's width, and rows that hold something that grows (a table, a list, an image, a canvas, or a container with one of these inside) share the extra height. Everything else keeps its height.

## Opening windows

A button, image, menu item or toolbar button can open a window without code:

```clarity
button @allCustomersCaption name allCustomers at 1,12 span 3 on click -> CustomerBrowse
button @newCustomerCaption name newCustomer at 4,12 span 3 on click -> CustomerForm in new window
```

In the properties, pick the window under "On click, open". From a handler, `open CustomerForm with customer = grid.selected` opens a window and fills its data, and `close` closes the current one.

## One desktop window

An application has one desktop window, on every stack. Opening a window shows it in place, on top of the one that opened it, and closing it goes back to the one below. A new desktop window appears only where the design says so: a `dialog` opens one of its own, and so does `-> Target in new window` (the In a new window box). Closing the last window there closes that desktop window, and the window that opened it gets its `on return`.

On the Preview tab, a button that opens a window shows that window on top, and **Back** or the trail of names goes back.
