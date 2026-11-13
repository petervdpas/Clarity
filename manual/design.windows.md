# Windows

How a window is declared and designed: its file, its data and events, the snap grid, and how one window opens another.

## Where windows and views live

Every window and view is a file of its own in the module's `design` folder, named after it: `shop/design/CustomerForm.cxaml`, and so is the app (`design/Shop.cxaml`, see [The app and its frame](design.frame.md)). The file's root element says what it is (`<Window>`, `<Dialog>`, `<View>`, `<Splash>`, `<Toolbox>` or `<App>`), and its `class` names the module and the page, as `class="shop.CustomerForm"`. The module's `.clarity` file keeps the rest: entities, records, enums, themes, and the code of each window and of the app in a `code CustomerForm` block. The designer reads and writes these files; you can also open one and edit it as text. A page or app written as a block in a module is an error.

## A window's file

The root's attributes are the window's settings; inside are its data and then its parts:

```xml
<Window class="shop.CustomerForm" title="@customerFormTitle" onopen="windowOpened" onreturn="windowReturned">
  <Data name="customer" type="Customer" />
  <Data name="orders" type="list&lt;Order&gt;" />
  <Label name="nameLabel" at="1,1" span="3" caption="@nameLabel" for="name" />
  <Textbox name="name" at="4,1" span="6" bind="customer.name" />
  <Table name="orderTable" at="1,3" span="12" rows="6" bind="orders" columns="number, total" />
  <Button name="saveButton" at="11,10" span="2" caption="@saveButtonCaption" default="true" onclick="saveClicked" />
</Window>
```

The handlers it names (`windowOpened`, `saveClicked`) are in the module's file:

```clarity
code CustomerForm
    handler saveClicked
        save customer
        close
```

The root element is `Window` (a window of its own, which can have a menu, toolbar and other frame regions), `Dialog` (a modal window) or `View`. Only a window has frame regions; see [The app and its frame](design.frame.md).

## Views and content

A **view** is a piece of a window, designed on its own as a window is: its parts, its data and its code, in its own file, `design/CustomerEditor.cxaml` with `<View class="shop.CustomerEditor">`, and its handlers in `code CustomerEditor`. A window shows views through a **content** part, which lists the views it can show:

```xml
<Window class="loans.FriendBrowse" title="@friendBrowseTitle" onreturn="windowReturned">
  <Content name="main" at="1,1" span="12" rows="9">
    <View name="FriendBrowseView" />
  </Content>
</Window>
```

The view's parts, data and handlers join the window's, so the window's code sets what it shows. A slide-out shows a view the same way, with a content inside it. A view's parts end by the rows the window holds from the top of the content area that shows it; an edit cuts them to fit. Views have their own section in the project tree; **+ New view** there starts a blank one.

## New windows and dialogs

To add one, click **New window** on the Project or Design tab. Give it a name and choose what it starts from: a blank window, Form for an entity (a label and an input per field, with Save and Close buttons) or Browse for an entity (a filter, a table, and New and Open buttons, with the code that filters and refreshes). The result is an ordinary window in its own file. New window always makes a window; to make a dialog, change the root element `<Window` to `<Dialog` (and the closing tag) in its file. A dialog that answers a question says what it gives, `<Dialog class="loans.PickFriend" gives="Friend">`, and its handler closes it with one: `close with grid.selected`; `ask PickFriend then lend(album)` opens it from another window, and the answer runs `lend` there (see [Code behind](code.handlers.md)).

## Data and events

`<Data name="customer" type="Customer" />` is one record the window works on; `type="list&lt;Customer&gt;"` is a list (the `<` written as `&lt;`, as XML wants). Parts bind to the data and handlers read and change it.

A window can also hold plain values, which are no records and are never stored: a number it shows, a choice, a flag. They live as long as the window:

```xml
<View class="calculator.Keypad" columns="4">
  <Data name="shown" type="decimal" />
  <Data name="pending" type="Operation" />
  <Data name="fresh" type="bool" default="true" />
  <Display name="screen" at="1,1" span="4" rows="2" bind="shown" digits="12" style="display" />
</View>
```

A plain value is `text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, `duration` or an enum, and starts at its `default`, or empty (0, empty text, false, or the enum's first value). A part binds it by its name, `bind="shown"`, and handlers read and change it like a local. A plain `decimal` shows the digits it has.

In the designer, add data under The window in the properties panel: a name, and what it holds: an entity (with list ticked or not) under Records, or a type or enum under Values. Each data item is listed in the Parts panel, to drag onto the window: a record's fields, a list as a table, a plain value as one part bound to it.

Data that is not filled when the window opens starts as a new, empty record, for a list as all records of the entity, and for a plain value at its default.

A window has three events of its own, listed under Events in The window and written on the root as `onopen`, `onreturn` and `onclose`:

- `on open`: the window has opened and its data is loaded.
- `on return`: a window this one opened has closed, and this one is on top again. This is where a list reloads after a form saved a record.
- `on close`: the window is about to close.

**Wire** fills in a handler name (`windowOpened`, `windowReturned`, `windowClosed`), writes an empty handler and opens it in the Code panel. See [Handlers](code.handlers.md).

## Size

`width="320" height="460"` on the root is the size the window's desktop window opens at, in pixels: 200 to 3840 wide and 150 to 2160 high. Without it a window opens at 1100 x 760. The desktop window takes the size of the window it shows first; a window opened in place keeps the size it has. Set it as Size under The window in the properties, or drag the window's right edge, bottom edge or bottom-right corner in the designer; the size shows while you drag. The designer shows the window at its size.

The window's parts end by the last row its height holds. That is its height less 12 pixels of padding above and below and the frame's bars (a menu bar is 30 pixels, a toolbar 38, a ribbon 30 and 92 more while it is open, a footer 28, or a row and 8 when it holds parts), in rows of the project's row height with 8 pixels between them. A window of 756 pixels with a toolbar holds 15 rows of 36. A dialog counts only its own bars, as it never shows the app's frame. No edit leaves parts reaching further: the designer, and every agent working through Clarity's tools, cut a part to end by the last row, a slide-out to the rows the window holds, and a view's parts to the rows the window holds from its content area's top (a content area grows to fill the window). The window keeps its size. A part placed wholly below the last row stays where it is, and the checker warns that the window scrolls, with the height that would hold it; a long form may scroll on purpose.

## The grid

Every part sits on a snap grid of 12 columns, or the number the root's `columns` gives (1 to 24, set as Grid columns in the properties). A part's place is `at="col,row"`, counted from 1, and its size is `span` columns and `rows` rows. Containers have a grid of their own, as wide as their span.

In the designer, drag a part to move it and drag its corner handle to resize it. The **Grid** button on the Design tab shows or hides the grid cells. The checker reports parts that start outside the grid, run past its last column, or overlap another part.

Rows have a fixed height in the designer. In the running application the columns share the window's width, and rows that hold something that grows (a table, a list, an image, a canvas, or a container with one of these inside) share the extra height. Everything else keeps its height, also in a row that grows: a button beside a list stays a button's height, at the top of its rows. To put a form beside a list, with a divider to set the list's width, place both in a splitter (see [Parts](design.parts.md)).

When nothing in a grid grows, its parts stay at the top and the spare height sits below them. Tick **Stick to bottom** under Layout in the properties (`bottom="true"`) to move a part down to the bottom instead, together with every part on its row and below: `<Button name="save" at="3,11" span="2" caption="@saveCaption" bottom="true" />`. Where a row grows, it changes nothing.

## Opening windows

A button, image, menu item or toolbar button can open a window without code:

```xml
<Button name="allCustomers" at="1,12" span="3" caption="@allCustomersCaption" opens="CustomerBrowse" />
<Button name="newCustomer" at="4,12" span="3" caption="@newCustomerCaption" opens="CustomerForm" newWindow="true" />
```

In the properties, pick the window under "On click, open". From a handler, `open CustomerForm with customer = grid.selected` opens a window and fills its data, and `close` closes the current one.

## One desktop window

An application has one desktop window, on every stack. Opening a window shows it in place, on top of the one that opened it, and closing it goes back to the one below. A new desktop window appears only where the design says so: a `dialog` opens one of its own, and so does `newWindow="true"` beside `opens` (the In a new window box). Closing the last window there closes that desktop window, and the window that opened it gets its `on return`.

On the Preview tab, a button that opens a window shows that window on top, and **Back** or the trail of names goes back.
