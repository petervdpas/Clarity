# Parts

The Parts palette and the properties panel: placing and naming parts, binding them to data, what every part has, and where each group of parts is described.

## The Parts palette

While you design a window, the sidebar's **Parts** tab lists the parts in groups. Drag a tile onto the window to place it; it lands on the grid cell under the pointer, inside the innermost container there. A new part gets a name after what it shows, such as `ordersTable` or `email`, or `button1` while it says nothing of its purpose yet. A part with such a numbered name takes a name after its purpose once you give it one: a label for `vatNumber` becomes `vatNumberLabel`, a button with the caption "Add line" `addLine`. A name code already goes by stays.

The palette shows every part type Clarity has, grouped, the same in every project, in a toolbox page and on the sign-in window too: what a part means is settled where it is dropped, and the checker says when a part does not belong where it is (a tool outside a toolbox, a sign-in part outside the sign-in window and its views). Drag a tile onto the window; set what it binds to under Bind in its properties. **Custom part** is one of them: dropped, it asks which of the project's custom parts it is, or makes a new one; its properties change it later.

Each group has a page of its own, with a section per part: what it is, its properties with their defaults, an example, and what the checker refuses.

- [Text](parts.text.md): labels, help texts, headings, icons, badges, links, text and Markdown.
- [Basic input](parts.input.md): text boxes, memos, numbers, check boxes, dropdowns, dates and the other inputs, and buttons.
- [Containers](parts.containers.md): forms, panels, tabs, content, accordions, wizards, popovers, slide-outs and splitters.
- [Data](parts.data.md): tables, lists, trees, charts, graphs, trends and status items.
- [Graphics](parts.graphics.md): images, displays, canvases and custom parts.
- [Media](parts.media.md): sound and video.
- [Time](parts.time.md): timers.
- [Toolbox](parts.toolbox.md): the parts of a toolbox page.
- [Sign-in](parts.signin.md): the parts of the sign-in window's views.

When the window is a `window` or the app, a Frame group comes first, with the menu, toolbar, ribbon, sidebar and footer; see [The app and its frame](design.frame.md).

## Parts in a window's file

In a window's or view's file a part is an element: its kind as the element's name, its properties as attributes, and the parts it holds inside it.

```xml
<Textbox name="email" at="4,2" span="6" bind="customer.email" placeholder="@emailPlaceholder" onchange="emailChanged" />
```

The checker's messages name the properties in Clarity's own words (`bind customer.email`, `on change: emailChanged`). In a `.cxaml` file they are written this way:

| Clarity's words | In a `.cxaml` file |
|---|---|
| the kind, `textbox`, `treetable` | the element, `<Textbox>`, `<TreeTable>` |
| the caption, `button @saveCaption` | `caption="@saveCaption"` |
| `at 4,2`, `span 6`, `rows 3` | `at="4,2"`, `span="6"`, `rows="3"` |
| a word alone, `default`, `bottom`, `sortable` | `default="true"`, `bottom="true"`, `sortable="true"` |
| a word and a value, `bind customer.email`, `columns number, total` | `bind="customer.email"`, `columns="number, total"` |
| `on change: emailChanged`, `on click: digit(7)` | `onchange="emailChanged"`, `onclick="digit(7)"` |
| `on click -> CustomerBrowse`, `in new window`, `with book` | `opens="CustomerBrowse"`, `newWindow="true"`, `openWith="book"` |
| `key "Ctrl+S"` (a shortcut) | `key="Ctrl+S"` |
| `taborder 2` | `taborder="2"` |
| `no undo` | `noUndo="true"` |
| `children children` | `childList="children"` |
| `key id parent parentId` (a tree's rows) | `rowKey="id" parentKey="parentId"` |
| `width 420` of a slide-out | `slideWidth="420"` |
| `horizontal` of an accordion | `orientation="horizontal"` |
| `reorder by position` | `reorder="position"` |
| `view RecipeEditor` in a content | `<View name="RecipeEditor" />` inside the `<Content>` |
| a value with several fields, such as a canvas's `units 160 x 100` or its tells | JSON, as the designer writes it: `units="{&quot;height&quot;:100,&quot;width&quot;:160}"` |

A part writes only what differs from its kind's defaults, in a fixed order. See [Design as text](design.text.md) to edit the file in the designer.

## What every part has

- `name` (camelCase): the part's name, unique in its window (sidebars, footer, menu, toolbar and ribbon included). Handlers refer to the part by it. The checker wants every part named (`button at 1,3 has no name; every part has one (Name them, in Problems, names them all)`), in camelCase, and each name once (`duplicate part name 'save'`).
- `caption` (a key): the text it shows, for the kinds that have one.
- `at="col,row"`, `span`, `rows` (whole numbers from 1): place and size on the grid; see [Windows](design.windows.md#the-grid). The checker refuses a part that starts at 0 (`button at 0,1: position and size start at 1`), runs past the last column (`button at 11,1 span 4 runs past column 12`) or overlaps another (`button at 3,2 overlaps the textbox at 1,2`).
- `bottom` (true or false, false by default): sticks to the bottom of its grid when no row grows; see [Windows](design.windows.md#the-grid). A part without a place (a popover, slide-out, sound or timer) has none.
- `tooltip` (a key, none by default): shown when the pointer rests on the part.
- `style` (a style of the project's theme, none by default): its colors, and the size, font and alignment of its text. See [Themes](themes.md#styles-on-parts).
- `on<event>="handler"`: runs `handler` in the window's code behind when `event` happens. Each event is wired at most once per part. With values, `onclick="digit(7)"`, it runs the handler with these values, one per parameter of the handler: a number, text in quotes, `true`, `false` or a value of the parameter's enum. See [Handlers](code.handlers.md).

Code reads and sets `visible` and `enabled` of every part, and more of some; see [Handlers](code.handlers.md#part-properties).

## Binding

`bind` connects a part to the window's data:

- An input, a `text` or a `heading` binds to one field of a single record, `bind="customer.name"`, or to a plain value by its name, `bind="shown"`. The type must be one the part accepts; a check box needs a `bool`. A plain `decimal` shows the digits it has.
- A `table` or `list` binds to a list, `bind="orders"`. Its `columns` or `show` name fields of the listed entity.
- A `form` binds to a single record, `bind="customer"`.

What the person types goes into the data, and handlers read and change the data by name. A part that cannot be bound, such as a label, may not have `bind`. An input that is not bound yet is allowed while designing. In the properties, **Bind** lists only the data and fields that part accepts.

The checker refuses data the window does not have (`textbox at 4,2: unknown data 'client'`), a field its entity does not have (`textbox at 4,2: 'Customer' has no field 'mail'`), a field of a type the part does not show (`checkbox at 1,3: 'name' is text; a checkbox shows bool`), a list where one record is wanted and the other way round, a hashed field (`textbox at 4,2: password is hashed, so no part shows it`), and `columns` or `show` on a part that binds nothing (`table at 1,1 lists fields but is not bound to data`).

## Clicking to open a window

A button, an image, a link, a status item, a menu item, or a toolbar or ribbon button can open a window without code:

```xml
<Button name="allCustomers" at="1,12" span="3" caption="@allCustomersCaption" opens="CustomerBrowse" />
```

- `opens="Target"` opens the window `Target`, in place, on top of the window the click is in. In the properties, pick it under **On click, open**.
- `openWith="book, chapter"` (**With data**) fills the opened window's data of those names from the data of the same names of the window the click is in, as `open Target with book = book` does in a handler. From a menu or toolbar of the app that is the window showing, and data that window does not have starts new. On a button the window must hold the data, of the same entity.
- `newWindow="true"` (**In a new window**) opens it in a desktop window of its own instead: the one way, besides a dialog, that an application gets a second desktop window. Closing the last window there closes that desktop window, and the window that opened it gets its `return` event.
- `opens` can also name a popover or slide-out of the window, which then opens next to the part or at its side.

A button can use the built-in targets `close` (closes the current window), `quit` (ends the application), `undo` and `redo` (take back or do again the last edit of the part that had the focus), `back` and `forward`, and `keys` (the keys window). `back` and `forward` go through the places the person was in the window, as a browser does: each place is the chosen row of its lists, tables and trees and the open section, tab or step of its accordions, tabs and steps, noted once the person's click or key is done. Going back chooses those rows and opens those sections again, and their `select` and `change` handlers run as when the person did it; Alt+Left and Alt+Right do the same, unless the part with the focus takes those keys (a tree that reorders). `language` and `theme` are for menus and toolbars; see [The app and its frame](design.frame.md#built-in-click-targets). On a button of the app's splash, `opens="signin password"` signs in with that strategy, and `opens="signout"` signs out (see [Signing in](project.signin.md)).

A click either opens something this way or runs a handler (`onclick`), not both. The checker refuses:

- `opens` with data on a popover or slide-out (`'filters' is part of this window; only a window opens with data`), data the target does not have (`CustomerForm has no data 'book' to open it with`), data the window does not have (`this window has no data 'book' to open CustomerForm with`), and data of another entity (`'book' here is Book, and ChapterForm's 'book' is Chapter`).
- A target that is something else than a window (`'Customer' is an entity, a click opens a window`), the splash (`Welcome is the splash screen, which the application shows itself as it starts; nothing opens it`) or the sign-in window.
- A target it finds nothing of is a warning: `'Customers' is not a window; handlers come with code behind`.
- `language` or `theme` on a button (`'language' picks in a menu or a toolbar; a button can close its window or quit`), and `newWindow` on a built-in target (`only a window opens in a new window`).

## Context menus

Any part can have a context menu: a `<Menu>` inside it, with items written as the frame's menu items are (`<Item>`, `<Item kind="separator" />`, submenus, `onclick="handler"` or `opens="Target"`). A right click opens it at the pointer; on a table or list it chooses the row under the pointer first, so the handler finds it as `grid.selected`. On a canvas the handler finds where it was opened as `canvas.x` and `canvas.y`, in the canvas's units, to act on the box there, and the line it was opened on as `canvas.bent` (its name, empty on a box or the paper) with its points as `canvas.via`, so a **Remove line** item knows which; the line is taken in hand as a press on it takes it. Named items hide and disable as any part does. **Add a context menu** in the part's properties starts one; pick it to use **Add item** and **Add separator** (see [The app and its frame](design.frame.md#commands)).

```xml
<Table name="grid" at="1,1" span="12" rows="6" bind="albums" columns="title">
  <Menu>
    <Item name="openItem" caption="@openItem" onclick="openClicked" />
    <Item kind="separator" />
    <Item name="deleteItem" caption="@deleteItem" onclick="deleteClicked" />
  </Menu>
</Table>
```

The checker warns about a context menu without items, and refuses an item with both a submenu and a click (`an item with a submenu opens the submenu; remove its 'on click'`).

## The properties panel

The inspector on the right has the tabs Properties, Code and Text ([Design as text](design.text.md)). Properties shows what is selected. Click a part on the canvas, or a line in the sidebar's Window tab, to select it and see its properties and events. Click the window's background or title bar to select the window itself: its title, grid columns, size, data and events (see [Windows](design.windows.md)).

What is picked shows the fields that part has:

- Name, Caption, and Column, Row, Width and Height.
- Bound to: only the data and fields that part accepts. Columns (each with its width) or Shown fields, in their order with Move up and Move down, Edited in the table, Link handlers, and Table (resizable, sortable, the first order), for tables and lists; a tree table has Resizable columns of its own. Striped rows for tables, lists, trees and tree tables. A tree and a tree table have Children (a list field of the rows' own kind), or Key and Parent (the field that names a row and the field that holds the key of its parent, text or integer), which arrange a list of records built in code into one tree.
- On click, open for buttons and images: a window (with In a new window and With data), or a slide-out or popover of the window; Action for a toolbox's tool (`does pencil`, or coded by its click handler); For input for labels and help texts; Placeholder, Tooltip, Style, Tab order, Shortcut; Read-only and the default and cancel boxes.
- Events: a row per event. **Wire** names the handler after the part and the event, writes it and opens the Code panel; **Code** shows it; the cross unwires it. See [Handlers](code.handlers.md).
- Takes rows from, for lists, tables, trees and memos: each part rows are dragged from with its handler (`on drop from fragmentList: linkFragment`); pick a part to add one, the cross takes it away.
- Remove.

Every change is written to the file at once. **Delete** in the ribbon, or the Delete key, removes the picked part. The designer refuses a property the part does not have (`'placeholder' does not apply to a checkbox`) and an event it does not have (`a label has no click event`).
