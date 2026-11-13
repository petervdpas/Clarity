# The app and its frame

The app's file: its settings, the main window and splash, the frame every window shares (menu, toolbar, ribbon, sidebars, footer), its commands and keys, and the built-in click targets.

## The app's file

A project has one app, in a file of its own beside the windows: `design/Shop.cxaml`, whose root `<App class="shop.Shop">` names its module and its name. It names the window the application starts with and declares the frame that every window shows:

```xml
<App class="shop.Shop" title="@shopTitle" main="CustomerBrowse" onstart="appStarted">
  <Data name="session" type="Session" />
  <Menu name="mainMenu">
    <Item caption="@fileItem" name="fileItem">
      <Item caption="@newCustomerItem" name="newCustomerItem" key="Ctrl+N" opens="CustomerForm" />
      <Item kind="separator" />
      <Item caption="@quitItem" name="quitItem" key="Ctrl+Q" opens="quit" />
    </Item>
    <Item caption="@viewItem" name="viewItem">
      <Item caption="@languageItem" name="languageItem" opens="language" />
      <Item caption="@themeItem" name="themeItem" opens="theme" />
    </Item>
  </Menu>
  <Toolbar name="mainToolbar">
    <Item kind="button" caption="@customersButtonCaption" name="customersButton" icon="list" opens="CustomerBrowse" />
    <Item kind="separator" />
    <Item kind="button" caption="@ordersButtonCaption" name="ordersButton" onclick="ordersClicked" />
  </Toolbar>
  <Sidebar side="left" name="leftSidebar" width="192">
    <List name="recentList" at="1,1" span="3" rows="10" bind="recent" show="name" />
  </Sidebar>
  <Footer status="@shopStatus" columns="12">
    <Text name="userText" at="10,1" span="3" bind="session.user" />
  </Footer>
</App>
```

The app's handlers are Clarity, in the block `code Shop` of its module:

```clarity
code Shop
    handler appStarted
        // runs once, before the main window opens

    handler ordersClicked
        open OrderBrowse
```

To design it, click **App** on the Project tab, or App at the top of the project tree. A new project already has one, `design/<App>.cxaml`; **New app** makes one when there is none, asking its name and the module it goes in. The canvas then shows only the frame, and The window in the properties has the Title, the Main window, the Version (a line of the project file), the Splash screen with how long it Shows at least, the app's data and its events.

The checker refuses a second app, an app written as an `app` block in a module's `.clarity` file (**Move pages into design** in Problems, or `clarity convert-pages`, moves it into its own file), and any element inside `<App>` but `<Data>`, `<Menu>`, `<Toolbar>`, `<Ribbon>`, `<Sidebar>` and `<Footer>`.

## App settings

```xml
<App class="module.Name" title="@key" main="Window" splash="Page" splashFor="seconds"
     onstart="handler" onunconnected="handler">
  <Data name="name" type="type" default="value" />
  regions
</App>
```

The app's settings are attributes of its root; its data are `Data` elements, the first things inside it.

| Attribute | Type | Default | Meaning |
|---|---|---|---|
| `class` | `module.Name` | required | The module and the app's name. |
| `title` | key | none | The application's title: what a window without a title of its own shows in its title bar. A key may hold `{app.version}`. |
| `main` | a window | none | The window shown at start. |
| `splash` | a splash page | none | The [splash screen](design.windows.md#splash-screen) shown while the application starts. |
| `splashFor` | seconds, 0.1 to 30 | until the main window shows | How long the splash shows at least. |
| `onstart` | handler | none | Runs once when the application starts, before its main window opens. |
| `onunconnected` | handler | none | Runs instead of the main window when the databases did not open. |

`<Data name="session" type="Session" />` gives the parts in the sidebars and the footer data of their own, as a window's data (see [Windows](design.windows.md#data)).

`onunconnected` runs when the databases did not open, such as a server that is down or a [secret setting](data.sources.md#passwords-stay-out-of-the-project) not kept yet. `handler notConnected(reason: text)` gets why, in the person's language (Clarity's texts `unconnectedNotKept` and `unconnectedFailed`). What it opens shows first. Closing the last window, or the desktop window from its title bar, then connects again if a setting changed, and the main window opens once it does; closing it unchanged ends the application. Without `onunconnected`, a database that connects with a secret setting shows the window where that setting is entered.

The checker refuses:

- Another attribute on `<App>` ("`<App>` has no 'x'"), and another event than `start` and `unconnected`.
- A `main` that is not a window ("the main window must be a page of kind window"); an app without `main` gets a warning.
- A `splash` that names no splash page, and a `splashFor` outside 0.1 to 30.

## Regions

In the Parts tab, the Frame group holds the regions. Drag one onto an edge of the window: a menu, toolbar or ribbon docks at the top, a sidebar on the left or right, a footer at the bottom. A new menu starts with File and Quit.

| Region | Docks | Holds |
|---|---|---|
| `<Menu>` | at the top | `<Item>` elements, which can hold items inside them (a submenu), and `<Item kind="separator" />` |
| `<Toolbar>` | at the top, below the menu | `<Item kind="button">` and separators |
| `<Ribbon>` | at the top | `<Tab>` elements with `<Group>` elements of buttons and separators |
| `<Sidebar>` | at its `side`, left or right | parts, on a grid of its own |
| `<Footer>` | at the bottom | an optional `status="@key"` and parts on a one-row grid |

Pick a region or a command on the canvas to edit it. The properties panel then offers Add item, Add submenu item, Add separator, Add button, Add tab and Add group, Move up and Move down, and Remove. The menu, the toolbar and the ribbon have their Name; a sidebar its own options; a footer its Status text.

A window can declare regions of its own, with the same elements in its own `.cxaml` file. A window's region replaces the app's region of that kind, and its sidebar replaces the app's sidebar on the same side; the other regions still come from the app. A dialog, a view and a sign-in window have no frame, and the checker refuses regions in them.

```xml
<Window class="todo.Users" title="@usersTitle" onopen="usersOpened">
  <Menu name="usersMenu">
    <Item caption="@fileItem" name="usersFileItem">
      <Item caption="@mainItem" name="mainItem" opens="close" />
      <Item kind="separator" />
      <Item caption="@languageItem" name="usersLanguageItem" opens="language" />
    </Item>
  </Menu>
  <Sidebar side="left" name="nav" width="280" min="200" max="480" resizable="true" columns="4">
    <List name="userList" at="1,1" span="4" rows="10" bind="users" show="name" />
  </Sidebar>
  <Content name="main" at="1,1" span="12" rows="11">
    <View name="UsersView" />
  </Content>
</Window>
```

### `Menu`

```xml
<Menu name="mainMenu"> items </Menu>
```

A named part: `name` is camelCase, and a new menu gets `mainMenu`. Code hides it with `mainMenu.visible = false` and turns it off with `mainMenu.enabled = false`. Its items are [commands](#commands). A menu with no items gets a warning; a menu without a name is refused.

### `Toolbar`

```xml
<Toolbar name="mainToolbar"> buttons and separators </Toolbar>
```

A named part, as the menu is: a new one gets `mainToolbar`, and code sets its `visible` and `enabled`. Its items are `<Item kind="button">` and `<Item kind="separator" />`. A toolbar with no buttons gets a warning; one without a name is refused.

### `Ribbon`

```xml
<Ribbon name="mainRibbon" collapsible="true" collapsed="true">
  <Tab title="@key" name="name">
    <Group title="@key" name="name"> buttons and separators </Group>
  </Tab>
</Ribbon>
```

A ribbon is a named part of the frame, docked at the top, and its tabs and groups have names too:

```xml
<App class="shop.Shop" main="CustomerCard">
  <Ribbon name="mainRibbon" collapsible="true">
    <Tab title="@homeTab" name="homeTab">
      <Group title="@clipboardGroup" name="clipboardGroup">
        <Item kind="button" caption="@pasteButtonCaption" name="pasteButton" icon="clipboard-paste" large="true" onclick="pasteClicked" />
      </Group>
    </Tab>
    <Tab title="@viewTab" name="viewTab">
      <Group title="@zoomGroup" name="zoomGroup">
        <Item kind="button" caption="@zoomInButtonCaption" name="zoomInButton" onclick="zoomInClicked" />
      </Group>
    </Tab>
  </Ribbon>
</App>
```

| Attribute | Type | Default | Meaning |
|---|---|---|---|
| `name` | camelCase | required | The ribbon's name; a new one gets `mainRibbon`. |
| `collapsible` | bool | false | The person double-clicks a tab to fold the ribbon to its tabs, and opening a tab unfolds it again. |
| `collapsed` | bool | false | With `collapsible`, it starts folded. |

A tab and a group have a `title` (a key) and a `name`; a new one is named after its title, as `homeTab` and `clipboardGroup`. Pick one on the canvas to change its name and, for the ribbon, its folding under Properties.

Code reads and sets the open tab as `mainRibbon.tab`, counted from 1, and, on a collapsible ribbon, the folding as `mainRibbon.collapsed`. A tab or group hides with `homeTab.visible = false`; when the open tab is hidden, the first visible one opens.

A ribbon with no tabs gets a warning. The checker refuses a ribbon, tab or group without a camelCase name, and `collapsed` without `collapsible`.

### `Sidebar`

A sidebar is a part docked to the left or right edge of a window, beside the window's own grid; in the app it shows beside every window that has none of its own on that side. It holds parts on a grid of its own: a list, a form, a canvas, whatever the window needs at hand.

```xml
<Window class="records.Albums" title="@albumsTitle">
  <Data name="albums" type="list&lt;Album&gt;" />
  <Sidebar side="left" name="nav" width="280" min="200" max="480" resizable="true" collapsible="true" maximizable="true" columns="3">
    <List name="albumList" at="1,1" span="3" rows="10" bind="albums" show="title" />
  </Sidebar>
</Window>
```

| Attribute | Type | Default | Meaning |
|---|---|---|---|
| `side` | `left` or `right` | `left` | The edge it docks to. |
| `name` | camelCase | required | Its name; a new one is named after its side, as `leftSidebar`. |
| `width` | pixels, 100 to 1600 | 240 | Its width. |
| `resizable` | bool | false | The person drags its inner edge to size it, from `min` to `max`. |
| `min`, `max` | pixels, 100 to 1600 | 100 and 1600 | How narrow and wide a resizable sidebar may be dragged. |
| `collapsible` | bool | false | A button folds it to a narrow strip at its edge, and the strip's button opens it again. |
| `collapsed` | bool | false | With `collapsible`, it starts folded. |
| `maximizable` | bool | false | A button lets it fill the window, and gives the window back. |
| `columns` | 1 to 96 | 3 | The columns of its own grid. |

Code reads and sets `nav.collapsed` and `nav.maximized`, on a sidebar that allows them, so a handler can open it or let it fill the window. In the designer, pick a sidebar on the canvas to set all of this under Properties.

The checker refuses a sidebar without a camelCase name, a width, `min` or `max` outside 100 to 1600, `min` or `max` without `resizable`, a `min` above its width or a `max` below it, `collapsed` without `collapsible`, and a grid of 0 or more than 96 columns.

### `Footer`

```xml
<Footer status="@key" columns="12"> parts </Footer>
```

| Attribute | Type | Default | Meaning |
|---|---|---|---|
| `status` | key | none | The status text, at the left. |
| `columns` | whole number | 12 | The columns of its one-row grid. |

A footer holds parts on one row. A status item is the footer's own part: a caption with an icon, a small gauge, a check mark and a warning, which code sets (see [Parts](design.parts.md)). The checker refuses a part in the footer that is not at row 1 and one row high ("a footer has one row").

## Commands

Menu items and toolbar and ribbon buttons are commands, `<Item>` elements.

```xml
<Item kind="button" caption="@key" name="name" icon="name" key="Ctrl+N" large="true"
      opens="Target" newWindow="true" openWith="data" onclick="handler" allow="condition"> items </Item>
```

| Attribute | On | Type | Meaning |
|---|---|---|---|
| `kind` | toolbar and ribbon | `button` or `separator` | A button, or a separator (also in a menu). A menu's items have no kind. |
| `caption` | all | key | The text. A menu item needs one; a button needs a caption or an icon. |
| `name` | all | camelCase | The command's name, for handlers and code. |
| `icon` | all | an icon name | An icon beside the caption. |
| `key` | all | a key | The key that runs it (see [Keys](#keys)). |
| `large` | ribbon buttons | bool | A large button. |
| `opens` | all | a window or a built-in target | What a click opens. |
| `newWindow` | with `opens` | bool | The window opens in a desktop window of its own. |
| `openWith` | with `opens` | data names | The opened window's data of those names is filled from the data of the same names of the window showing. |
| `onclick` | all | handler | The handler a click runs. |
| `allow` | all | a condition | Who may pick it, a condition on `me`. |

Each command has a Name, a Caption and an Icon in the properties. A click either runs a handler or opens something, not both:

- `onclick="ordersClick"` runs a handler. **Wire** names it after the command (`ordersClick`), writes it and opens the Code panel. Double-click an item or button on the canvas to go to its click handler, which Clarity writes when there is none yet.
- `opens="CustomerBrowse"` ("Or on click, open") opens a window, with `newWindow="true"` in a desktop window of its own, or one of the [built-in targets](#built-in-click-targets): `opens="close"`.

An item with a submenu opens the submenu and has no click of its own.

Every menu item and button has a name, in camelCase like a part's (`<Item caption="@newCustomerItem" name="newCustomerItem">`); a separator has none. Its name is unique among the window's parts, including the menu and the toolbar themselves, and differs from the window's data. The designer names a new one after its caption and kind, as `newCustomerItem` or `ordersButton`, which keeps it apart from data and parts of the same word.

Icons are [Lucide](https://lucide.dev)'s, by name: `icon="save"`, `icon="trash"`, `icon="triangle-alert"`. A button, a menu item and a toolbar or ribbon button show one beside their caption, and the icon part shows one on its own. The checker knows every name and suggests the nearest for a misspelt one; the designer offers them all. An application carries only the icons it uses, with Lucide's license in its `THIRD_PARTY_NOTICES.md`.

A part's context menu (a `<Menu>` inside the part) is edited the same way: pick it and use **Add item** and **Add separator**; an agent's `add_command` reaches it by the part's name.

The checker refuses:

- A menu item or button without a name, or with one that is not camelCase.
- A menu item without a caption, a button without a caption or an icon, an empty caption, and `large` on anything but a ribbon button.
- A separator with a caption, icon, key or click.
- Both `onclick` and `opens` ("'click' is wired twice"), and an item with a submenu that has a click of its own.
- An unknown icon, and a key that is not a key.
- `newWindow` with a built-in target ("only a window opens in a new window"), and `opens` naming the splash or the sign-in window, which the application shows itself.
- `openWith` naming data the opened window does not have.
- An attribute `<Item>` does not have ("`<Item>` has no 'x'").

## Built-in click targets

- `quit` ends the application.
- `close` closes the current window and goes back to the one below.
- `undo` and `redo` take back or do again the last edit of the part that had the focus.
- `back` and `forward` go through the places the person was in the window, as a browser does: the chosen rows of its lists, tables and trees and the open sections, tabs and steps.
- `language` lets the person pick the language the application shows, from the project's translations. See [Languages](languages.md).
- `theme` lets the person pick one of the themes the project offers. See [Themes](themes.md).
- `keys` opens Clarity's keys window, where the person sets the application's shortcut keys for themselves. It needs `keys editable` in the project file (the switch in Project settings).
- `signout` signs out who signed in, when people sign in. See [Signing in](project.signin.md).

The application remembers the chosen language and theme for its next start. A button on a window can use every target but `language` and `theme`, which are for menus and toolbars.

## Keys

A menu item, a toolbar button and a ribbon button can have a key, written as modifiers and one key joined by `+`: `key="Ctrl+N"`, `key="Ctrl+Shift+F2"`. The menu shows it next to the caption, and a button's tooltip shows it. A ribbon button's key works whichever tab is open, and a menu item's while the menu is shut. A button on a window takes a key in the same way (Shortcut in the properties), and works anywhere in the window.

The modifiers are `Ctrl`, `Alt`, `Shift` and `Meta`, each once. The key is a letter or digit, `F1` to `F12`, one of `Enter`, `Escape`, `Tab`, `Space`, `Backspace`, `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, `Up`, `Down`, `Left` and `Right`, or one of the characters `+ - * / . , =` (`Ctrl++` is Ctrl and `+`).

What is innermost goes first: a key of the window's own buttons wins over the same key in the frame, so a window can take a frame's key for itself. Within one place, one key runs one command: two different commands with the same key in the frame, or among a window's buttons, is an error ("key 'Ctrl+N' runs both newItem and newButton in the app's frame"). A menu item and a button that open the same window or run the same handler are one command, and may share a key. What code hid or disabled does not answer its key.

Keys work alike on every stack.

### The keys window

With `keys editable` in the project file, each person can change the keys for themselves. A menu item or button with `opens="keys"` opens the keys window. It lists every named command that does something, Everywhere (the app's frame) first and then each window's own commands and buttons, also those without a key in the design. Pick Change and press the new key: a key another command has in the same place offers to swap the two, and a key another place has is allowed, the window saying which command goes first where. Letters without Ctrl or Alt, Tab, Enter and Escape cannot be used. Reset puts a key back to the design's, Reset all every one. The keys are kept in the person's settings file, beside their other settings, as only what differs from the design, so a key you change in a new version still reaches whoever left it as it was. The window speaks in Clarity's own words, which the project translates (see [Languages](languages.md)).

## Related pages

- [Windows](design.windows.md)
- [Parts](design.parts.md)
- [Themes](themes.md)
- [The project file](project.file.md)
- [Handlers](code.handlers.md)
