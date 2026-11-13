# Windows

How windows, dialogs, views and the other pages are declared and designed: their files, their data and events, their size, the snap grid, and how one window opens another.

## Where windows and views live

Every window and view is a file of its own in the module's `design` folder, named after it: `shop/design/CustomerForm.cxaml`, and so is the app (`design/Shop.cxaml`, see [The app and its frame](design.frame.md)). The file's root element says what it is (`<Window>`, `<Dialog>`, `<View>`, `<Splash>`, `<Toolbox>`, `<SignIn>` or `<App>`), and its `class` names the module and the page, as `class="shop.CustomerForm"`. Its code is the block `code CustomerForm` of that module.

The module's `.clarity` files keep the rest: entities, records, enums, themes, and the code of each window and of the app. Handlers, functions and actions are Clarity, in the module; the `.cxaml` file names them (`onclick="saveClicked"`) and holds no code.

The designer reads and writes these files; the Text tab of the inspector shows the file as it is, and you can edit it there. A part writes only what differs from its kind's defaults. A value with `<` or `&` writes them as `&lt;` and `&amp;`, as XML wants: `type="list&lt;Order&gt;"`.

The checker refuses a `class` whose module the project does not have, and a `page` or `app` block still written in a module's `.clarity` file, the old form. **Move pages into design** in Problems, or `clarity convert-pages` on the command line, moves such a block into its own file.

## Window, content, view

A window shows its parts in a view: its content part lists the views it can show, and the view holds the parts, data and handlers. **New window** builds a window this way, with an empty view `<Name>View` in a content named `main`:

```xml
<!-- shop/design/CustomerForm.cxaml -->
<Window class="shop.CustomerForm" title="@customerFormTitle" onreturn="windowReturned">
  <Content name="main" at="1,1" span="12" rows="10">
    <View name="CustomerFormView" />
  </Content>
</Window>
```

```xml
<!-- shop/design/CustomerFormView.cxaml -->
<View class="shop.CustomerFormView">
  <Data name="customer" type="Customer" />
  <Data name="orders" type="list&lt;Order&gt;" />
  <Label name="nameLabel" at="1,1" span="3" caption="@nameLabel" for="name" />
  <Textbox name="name" at="4,1" span="6" bind="customer.name" />
  <Table name="orderTable" at="1,3" span="12" rows="6" bind="orders" columns="number, total" />
  <Button name="saveButton" at="11,10" span="2" caption="@saveButtonCaption" default="true" onclick="saveClicked" />
</View>
```

```clarity
code CustomerForm
    handler windowReturned
        reload orders

code CustomerFormView
    handler saveClicked
        save customer
```

A window may also hold its parts itself, without a content and a view; a dialog made blank does.

## New windows and dialogs

Click **New window** on the Project or Design tab. It asks for a Name, what it starts from, and In which module it goes:

- **Blank window**: a window showing an empty view in its content.
- **Blank dialog**: a dialog, a window of its own that opens over another.
- **Blank view**: a view, shown in a content.
- **Splash screen**: what shows while the app starts, 480 by 280, with a heading and a `status` label.
- **Toolbox**: the tools a canvas shows, designed as a view.
- **Sign-in window**: offered when people sign in and the project has none yet.
- **Settings window**: offered when the project has settings; a dialog with a labeled input for every setting and a Close button.
- **Form for** an entity: a label and an input per field, with Save and Close buttons.
- **Browse for** an entity: a filter, a table, and New and Open buttons, with the code that filters and refreshes.

The result is an ordinary window in its own file. Views also have their own section in the project tree, where **+ New view** starts a blank one.

## Renaming a window or view

Right-click a window or view in the project tree and pick **Rename**. Its file takes the new name (`design/Customer.cxaml`, `class="shop.Customer"`), and so does every place that names it: its `code` block, buttons and menu items that open it, content view lines, the app's main window and splash, and `open` and `ask` in code. Undo takes it all back. An agent renames one with the `rename` tool (`kind` window or view).

## Kinds

The root element is the page's kind.

### `Window`

A window of its own. Only a window has frame regions of its own, written as elements in its file: `<Menu>`, `<Toolbar>`, `<Ribbon>`, `<Sidebar>` and `<Footer>` (see [The app and its frame](design.frame.md)). The app's main window must be a window.

### `Dialog`

A modal window. It never shows the app's frame and has none of its own. A dialog that answers a question says what it gives, `gives="Friend"`, and its handler closes it with one: `close with grid.selected`. `ask PickFriend then lend(album)` opens it from another window, and the answer runs `lend` there (see [Handlers](code.handlers.md)).

```xml
<Dialog class="loans.PickFriend" title="@pickFriendTitle" gives="Friend" width="420" height="360">
  <Data name="friends" type="list&lt;Friend&gt;" />
  <List name="grid" at="1,1" span="12" rows="6" bind="friends" show="name" />
  <Button name="pickButton" at="11,7" span="2" caption="@pickCaption" default="true" onclick="pickClicked" />
</Dialog>
```

The checker refuses `gives` on any page but a dialog, and menus, toolbars, ribbons, sidebars and footers in a dialog.

### `View`

A piece of a window, designed on its own as a window is (see [Views](#views)). It has no frame.

### `Splash`

The splash screen the app shows while it starts (see [Splash screen](#splash-screen)).

### `Toolbox`

The tools a canvas or memo shows, designed as a view: its parts are tool buttons, tool groups, size, opacity and colors, and buttons, labels, headings and lines. A canvas or memo names it with `toolboxPage`, as `toolboxPage="CoverTools"`. The designer draws it as small as its columns of tools and its rows, as a canvas shows it as a panel. See [Handlers](code.handlers.md#drawing-on-a-canvas).

### `SignIn`

The sign-in window the program shows before the main window when people sign in. Nothing opens it, it has no frame and no `allow`, and without a size it is 420 by 380. See [Signing in](project.signin.md).

## A page's file

```xml
<Root class="module.Name" title="@key" width="n" height="n" columns="n" cell="n" style="name"
      allow="condition" gives="Type" onopen="handler" onclose="handler" onreturn="handler">
  <Data name="name" type="type" default="value" />
  <Tells message="name(params)" />
  regions and parts
</Root>
```

The root's attributes are the page's settings; its data are `Data` elements, the first things inside it.

| Attribute | Type | Default | Meaning |
|---|---|---|---|
| `class` | `module.Name` | required | The module and the page's name. |
| `title` | key | the app's title | The window's title, in its title bar. A key may hold `{app.version}`, as `mainTitle "Todo v{app.version}"`. |
| `width`, `height` | whole numbers | 1100 x 760 | The size its desktop window opens at, in pixels (see [Size](#size)). Both or neither. |
| `columns` | 1 to 96 | 12 | The number of grid columns (see [The grid](#the-grid)). |
| `cell` | 8 to 64 | the project's rows | The page's own exact grid, rows of this many pixels. |
| `style` | a style name | none | A style of the project's theme for the window itself. |
| `allow` | a condition | everyone | Who may open it, a condition on `me`: `allow="me.role == Admin"`. |
| `gives` | a type | none | What a dialog closes with. |
| `onopen`, `onclose`, `onreturn` | handler names | none | The window's events (see [Events](#events)). |

`style="name"` gives the window a style of the project's theme: its background fills the whole window and its text color is its parts' text, as a splash screen's deep color. Every theme the application offers must have it. In the designer, Style under The window lists the theme's styles. See [Themes](themes.md).

`allow` shows as Allow under The window when people sign in; see [Signing in](project.signin.md).

The checker refuses a size with only a width or a height, a size outside 200 to 3840 wide and 150 to 2160 high ("a window is 200 to 3840 pixels wide and 150 to 2160 high"), a grid of 0 or more than 96 columns, and a `cell` outside 8 to 64.

## Data

`<Data name="customer" type="Customer" />` is one record the window works on; `type="list&lt;Customer&gt;"` is a list of them. Parts bind to the data and handlers read and change it.

```xml
<Data name="name" type="type" default="value" />
```

- An entity's name: one record, new when the window opens.
- `list<Entity>`: a list of its records, all of them when the window opens.
- A [record](code.engines.md)'s name, or a list of records: held by the window, new or empty when it opens; code fills them and nothing stores them.
- A plain value: `text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, `duration` or an enum, or a list of text, integers, decimals or dates, which starts empty. It is no record and is never stored; it lives as long as the window.

A plain value starts at its `default`, or empty: 0, empty text, false, no date, or the enum's first value. A part binds it by its name, `bind="shown"`, and handlers read and change it like a local. A plain `decimal` shows the digits it has.

```xml
<View class="calculator.Keypad" columns="4">
  <Data name="shown" type="decimal" />
  <Data name="pending" type="Operation" />
  <Data name="fresh" type="bool" default="true" />
  <Display name="screen" at="1,1" span="4" rows="2" bind="shown" digits="12" style="display" />
</View>
```

When a window is opened with `open CustomerForm with customer = ...`, the data is filled by name. Data that is not filled starts as a new, empty record, for a list as all records of the entity, and for a plain value at its default. When the project has settings, every window also holds them as `settings`.

In the designer, add data under Data in The window: a name, and what it holds, an entity under Records or a type or enum under Values. The Parts panel holds Clarity's parts only: drop one, then set what it binds to (a field of the data, a list, a plain value or a setting) under Bind in its properties.

The checker refuses:

- `<Data>` without a name and a type, and a name that is not camelCase.
- A `default` on a list ("a list of values starts empty"; "a list starts with every record").
- A list of values of another type than text, integer, decimal or date.
- `file` and `bytes`: a file belongs to a record, and bytes are no window data.
- Data named `settings`, or of the settings' type, when the project has settings.
- `reload` of a plain value: only records and lists are read again.

## Events

A window has three events of its own, listed under Events in The window and written on the root as `onopen`, `onreturn` and `onclose`:

- `onopen`: the window has opened and its data is loaded.
- `onreturn`: a window this one opened has closed, and this one is on top again. This is where a list reloads after a form saved a record.
- `onclose`: the window is about to close.

**Wire** fills in a handler name (`windowOpened`, `windowReturned`, `windowClosed`), writes an empty handler and opens it in the Code panel. See [Handlers](code.handlers.md).

The checker refuses another event on the root ("a window has the events open, close and return") and a value that is not a handler's name.

## Views

A window is a composition of views. A view is designed as a window is (its parts on its grid, its data, its code), in its own file with a `<View>` root, without a frame of its own. It shows where a window's content part names it, with a `<View name="..." />` line. A content shows one of the views it lists at a time; code shows another with `main.swap(OtherView)`, and `main.tab` is the one shown, counted from 1 in its list. A slide-out, tab, step or section shows a view through a content inside it. A content holds no parts of its own besides its view lines.

```xml
<!-- design/WorkspaceView.cxaml -->
<View class="modelling.WorkspaceView">
  <Data name="picked" type="text" />
  <Tree name="classTree" at="1,1" span="3" rows="20" bind="nodes" rowKey="key" parentKey="parent" show="label" onselect="classPicked" />
  <Slideout name="classPanel" at="4,1" span="6" rows="12" caption="@classCaption" slideWidth="520" pinnable="true">
    <Content name="classContent" at="1,1" span="6" rows="12">
      <View name="ClassEditor" />
    </Content>
  </Slideout>
</View>
```

```xml
<!-- design/ClassEditor.cxaml -->
<View class="modelling.ClassEditor">
  <Data name="className" type="text" />
  <Label name="nameLabel" at="1,1" span="2" caption="@nameLabel" for="nameBox" />
  <Textbox name="nameBox" at="3,1" span="4" bind="className" />
  <Button name="saveButton" at="5,2" span="2" caption="@saveCaption" onclick="saveClicked" />
</View>
```

```clarity
code WorkspaceView
    handler classPicked
        className = classTree.selected.label
        classPanel.visible = true

code ClassEditor
    handler saveClicked
        // works on its own parts and data
```

- Where it shows, the view's parts, data and handlers join the window's: the window's code reaches them by name, so the window sets what the view shows (`className = ...`). The view's own code reaches only its own parts and data, so one view can show in several windows.
- A window shows a view once; the view's part and data names differ from the window's.
- The designer shows the content as written; the Preview shows it with the view in it.
- A view's parts end by the rows the window holds from the top of the content area that shows it; an edit cuts them to fit (see [Size](#size)).

An accordion, tabs or a wizard can be a composition of views too, each section, tab or step holding a content with one view:

```xml
<Accordion name="recipeParts" at="5,1" span="8" rows="14" orientation="horizontal">
  <Section title="@overviewSection">
    <Content name="recipeOverviewContent" at="1,1" span="8" rows="14">
      <View name="RecipeOverview" onsaved="recipeSaved" />
    </Content>
  </Section>
  <Section title="@timerSection">
    <Button name="startTimerButton" at="1,2" span="3" caption="@startTimerCaption" onclick="startTimer" />
  </Section>
</Accordion>
```

The checker refuses a content without a view line, a view line holding parts, a view that does not exist or is another kind of page, a view that shows itself, and a view shown twice in one window. A tab, step or section that names a view directly is refused too: put a content in it. In code, `main.swap(X)` takes only a view the content lists.

### Tells

A view tells its window what happened. Its file says what it tells, `<Tells message="saved" />` (with values, `<Tells message="saved(name: text)" />`), its code says `tell saved()`, and the window hears it on the view line that shows it, `<View name="RecipeEditor" onsaved="recipeSaved" />`, with a handler that takes those values. A tell the window does not hear does nothing.

```xml
<!-- design/CookbookView.cxaml -->
<View class="kitchen.CookbookView">
  <Data name="recipes" type="list&lt;Recipe&gt;" />
  <Content name="recipeContent" at="5,1" span="8" rows="14">
    <View name="RecipeEditor" onsaved="recipeSaved" />
  </Content>
</View>
```

```xml
<!-- design/RecipeEditor.cxaml -->
<View class="kitchen.RecipeEditor">
  <Data name="dish" type="Recipe" />
  <Tells message="saved" />
  <Button name="saveButton" at="5,8" span="2" caption="@saveCaption" onclick="saveClicked" />
</View>
```

```clarity
code CookbookView
    handler recipeSaved
        recipes = find r in Recipe order by r.name

code RecipeEditor
    handler saveClicked
        save dish
        tell saved()
```

In the designer, with nothing picked on the view, its **Tells** list declares each one, `saved(name: text)`, with **Add** and a cross to take one away. In the window, pick the content's view line: **Tells of** the view lists them, and **Wire** gives each a handler of the window that takes the tell's values.

The checker refuses `<Tells>` on any page but a view ("a view tells the window it shows in"), and a view line that hears a tell the view does not declare.

## Splash screen

An application may show a splash screen while it starts; it is optional, and an app without one opens its main window at once. The splash is a page of the kind `Splash`, designed as any page is (a picture, the name, a progress bar), and the app names it on its root with how long it shows at least: `splash` and `splashFor`, in seconds (see [The app and its frame](design.frame.md)).

```xml
<!-- shop/design/Shop.cxaml -->
<App class="shop.Shop" main="Main" splash="Welcome" splashFor="2" onstart="loadCatalogue">
</App>
```

```xml
<!-- shop/design/Welcome.cxaml -->
<Splash class="shop.Welcome" width="560" height="290" style="splashBack">
  <Image name="logo" at="1,1" span="4" rows="5" source="logo.png" />
  <Heading name="titleHeading" at="5,2" span="8" rows="2" caption="@shopTitle" style="splashTitle" />
  <Label name="tagline" at="5,4" span="8" caption="@shopTagline" style="splashTagline" />
  <Busy name="status" at="5,5" span="8" caption="@welcomeStarting" style="splashStatus" />
</Splash>
```

```clarity
code Shop
    handler loadCatalogue
        splash.status = @loadingCatalogue
```

- It shows first, while the app's `onstart` runs, and goes once the main window shows, but not before the seconds `splashFor` gives (0.1 to 30; without it, as soon as the main window shows).
- `splash.status = text` in code says what loads: the caption of the splash page's part named `status`, a label, or a busy part that spins while it shows. Code anywhere may set it; it shows while the splash does.
- On the desktop it is a borderless window in the middle of the screen, above the others and not in the taskbar, without the app's frame. In the browser it fills the page until the main window takes its place. A splash page without `width` and `height` is 480 by 280.
- When the databases did not open, the app starts without its splash and shows what keeps the connection.
- The app's splash may double as the sign-in window: with a sign-in part on it, or the strategies' inputs and buttons that click to `signin <strategy>`, it stays open once `onstart` is done until someone signs in, then the main window opens (see [Signing in](project.signin.md)).

The application shows and closes it. The checker refuses a click that opens it, events on it ("it has no 'on open' of its own"), a splash as the main window, an app's `splash` that names no splash page, and setting anything of `splash` in code but its status. A splash page no app names gets a warning.

## Size

`width="320" height="460"` on the root is the size the window's desktop window opens at, in pixels: 200 to 3840 wide and 150 to 2160 high. Without it a window opens at 1100 x 760. The desktop window takes the size of the window it shows first; a window opened in place keeps the size the desktop window has. Set it as Size under The window in the properties, or drag the window's right edge, bottom edge or bottom-right corner in the designer; the size shows while you drag. The designer shows the window at its size.

The window's parts end by the last row its height holds. That is its height less 12 pixels of padding above and below and the frame's bars (a menu bar is 30 pixels, a toolbar 38, a ribbon 30 and 92 more while it is open, a footer 28, or a row and 8 when it holds parts), in rows of the project's row height with 8 pixels between them. A window of 756 pixels with a toolbar holds 15 rows of 36. A dialog counts only its own bars, as it never shows the app's frame.

No edit leaves parts reaching further: the designer, and every agent working through Clarity's tools, cut a part to end by the last row, a slide-out to the rows the window holds, and a view's parts to the rows the window holds from its content area's top (a content area grows to fill the window). The window keeps its size. A part placed wholly below the last row stays where it is, and the checker warns that the window scrolls, with the height that would hold it; a long form may scroll on purpose.

## The grid

Every part sits on the snap grid of the window, or of the container it is in. The window's grid has 12 columns, or the number the root's `columns` gives (1 to 96, Grid columns in the properties). Containers have a grid of their own, as wide as their span.

```xml
<Button name="save" at="3,11" span="2" rows="1" caption="@saveCaption" bottom="true" />
```

| Attribute | Type | Default | Meaning |
|---|---|---|---|
| `at` | `col,row` | required | The top-left cell, counted from 1. |
| `span` | whole number | the part's kind | The columns it covers. |
| `rows` | whole number | the part's kind | The rows it covers. |
| `bottom` | bool | false | Sticks it to the bottom when nothing in the grid grows. |

In the designer, drag a part to move it and drag its corner handle to resize it. The **Grid** button on the Design tab shows or hides the grid cells.

The designer shows the window as it runs, and its grid follows: the columns share the window's width, and rows that hold something that grows share the extra height, so their cells on the canvas are taller. What grows is a table, list, tree, tree table, chart, graph, image, canvas, markdown or memo, a custom part declared with `grows`, or a container with one of these inside. Everything else keeps its height, also in a row that grows: a button beside a list stays a button's height, at the top of its rows. To put a form beside a list, with a divider to set the list's width, place both in a splitter (see [Parts](design.parts.md)).

When nothing in a grid grows, its parts stay at the top and the spare height sits below them. Tick **Stick to bottom** under Layout in the properties (`bottom="true"`) to move a part down to the bottom instead, together with every part on its row and below, keeping their heights. Buttons under a form, in a pane that a list beside it makes taller, sit at the bottom of the pane this way. Where a row grows, `bottom` changes nothing.

`columns="56" cell="10"` gives a page its own exact grid, for a designed page such as a splash: rows of `cell` pixels (8 to 64, Grid rows (px) in the properties), cells without gaps or padding, and rows that keep their height. On a 560 pixel window that is square cells of 10 pixels.

The checker reports parts that start outside the grid, run past its last column, or overlap another part in the same container.

## Opening windows

A button, image, menu item or toolbar button can open a window without code:

```xml
<Button name="allCustomers" at="1,12" span="3" caption="@allCustomersCaption" opens="CustomerBrowse" />
<Button name="newCustomer" at="4,12" span="3" caption="@newCustomerCaption" opens="CustomerForm" newWindow="true" />
```

In the properties, pick the window under "On click, open". From a handler, `open CustomerForm with customer = grid.selected` opens a window and fills its data, and `close` closes the current one. A window, dialog or view with `allow` opens only for whoever the condition lets in.

## One desktop window

An application has one desktop window, on every stack. Opening a window shows it in place, on top of the one that opened it, and closing it goes back to the one below. A new desktop window appears only where the design says so: a dialog may open one of its own, and so does `newWindow="true"` beside `opens` (the In a new window box). Closing the last window there closes that desktop window, and the window that opened it gets its `onreturn`. The checker refuses `newWindow` on anything but a window.

On the Preview tab, a button that opens a window shows that window on top, and **Back** or the trail of names goes back.

## Related pages

- [The app and its frame](design.frame.md)
- [Parts](design.parts.md)
- [Handlers](code.handlers.md)
- [Themes](themes.md)
- [Signing in](project.signin.md)
