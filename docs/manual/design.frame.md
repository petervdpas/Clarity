# The app and its frame

The app's file: the main window, the frame every window shares (menu, toolbar, ribbon, sidebars, footer), keys, and the built-in click targets.

## The app's file

A project has one app, in a file of its own beside the windows: `design/Shop.cxaml`, whose root `<App class="shop.Shop">` names its module and its name. It names the window the application starts with and declares the frame that every window shows:

```xml
<App class="shop.Shop" title="@shopTitle" main="CustomerBrowse" onstart="appStarted">
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
  <Footer status="@shopStatus" columns="12" />
</App>
```

The app's handlers are Clarity, in the module's file:

```clarity
code Shop
    handler appStarted
        // runs once, before the main window opens

    handler ordersClicked
        open OrderBrowse
```

- `main="CustomerBrowse"` is the window shown at start; it must be a window.
- `onstart` runs once, before the main window opens. The app's handlers are in `code Shop`.
- `onunconnected` runs instead of the main window when the databases did not open, such as a server that is down. `handler notConnected(reason: text)` gets why; what it opens shows first, and closing it after changing a setting connects again. Without it, a database that connects with a [secret setting](../LANGUAGE.md#secret-settings) shows the window where that setting is entered.
- `<Data name="session" type="Session" />` gives the parts in the sidebars and footer data of their own.

To design it, click **App** on the Project tab, or App at the top of the project tree. A new project already has one, `design/<App>.cxaml`; **New app** makes one when there is none. The canvas then shows only the frame, and The window in the properties has the title, the Main window and the start event.

## Regions

In the Parts tab, the Frame group holds the regions. Drag one onto an edge of the window: a menu, toolbar or ribbon docks at the top, a sidebar on the left or right, a footer at the bottom. A new menu starts with File and Quit.

- `<Menu name="mainMenu">`: `<Item>` elements, which can hold items inside them (a submenu), and `<Item kind="separator" />`.
- Every menu item and button has a camelCase name, as every part does: `<Item caption="@newCustomerItem" name="newCustomerItem">`. A new one is named after its caption and kind, as `ordersButton`; a separator has none. Double-click an item or button to go to its click handler, which Clarity writes when there is none yet.
- `<Toolbar name="mainToolbar">`: `<Item kind="button">` and separators.
- The menu and the toolbar are parts with a camelCase name, as `mainMenu` and `mainToolbar`, which a new one gets. Code hides one with `mainToolbar.visible = false` and turns it off with `mainMenu.enabled = false`.
- `<Ribbon>`: `<Tab title="@key">` elements with `<Group title="@key">` elements of buttons and separators; a button can be `large="true"`.
- `<Sidebar side="left">` or `side="right"`: parts on a grid of its own, docked to that edge; see [Sidebars](#sidebars).
- `<Footer>`: an optional `status="@key"` and parts on a one-row grid of 12 columns.

Pick a region or a command on the canvas to edit it. The properties panel then offers Add item, Add submenu item, Add separator, Add button, Add tab and Add group, Move up and Move down, and Remove. The menu, the toolbar and the ribbon have their Name; a sidebar its own options (see [Sidebars](#sidebars)); a footer its Status text.

A window can declare regions of its own, with the same elements in its own `.cxaml` file. A window's region replaces the app's region of that kind, and its sidebar replaces the app's sidebar on the same side; the other regions still come from the app. A dialog and a view have no frame.

## The ribbon

A ribbon is a named part of the frame, docked at the top, and its tabs and groups have names too:

```xml
<App class="shop.Shop" main="CustomerCard">
  <Ribbon name="mainRibbon" collapsible="true">
    <Tab title="@homeTab" name="homeTab">
      <Group title="@clipboardGroup" name="clipboardGroup">
        <Item kind="button" caption="@pasteButtonCaption" name="pasteButton" icon="paste" large="true" onclick="pasteClicked" />
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

- `collapsible`: the person double-clicks a tab to fold the ribbon to its tabs, and opening a tab unfolds it again. `collapsed="true"` as well starts it folded.
- Code reads and sets the open tab as `mainRibbon.tab`, counted from 1, and the folding as `mainRibbon.collapsed`. A tab or group hides with `homeTab.visible = false`; when the open tab is hidden, the first visible one opens.
- A new ribbon, tab or group is named for you: `mainRibbon`, and a tab or group after its title, as `homeTab` and `clipboardGroup`. Pick it on the canvas to change its name and, for the ribbon, its folding under Properties.

## Sidebars

A sidebar is a part docked to the left or right edge of a window, beside the window's own grid; in the app it shows beside every window that has none of its own on that side. It holds parts on a grid of its own: a list, a form, a canvas, whatever the window needs at hand.

In the window's file, `design/Albums.cxaml`:

```xml
<Window class="records.Albums" title="@albumsTitle">
  <Data name="albums" type="list&lt;Album&gt;" />
  <Sidebar side="left" name="nav" width="280" min="200" max="480" resizable="true" collapsible="true" maximizable="true" columns="3">
    <List name="albumList" at="1,1" span="3" rows="10" bind="albums" show="title" />
  </Sidebar>
</Window>
```

- `side`: `left` or `right`.
- `name`: every sidebar has one, like every part; a new one is named after its side, as `leftSidebar`.
- `width="280"`: its width in pixels, 240 without it; a sidebar is 100 to 1600 pixels wide.
- `resizable`: the person drags its inner edge to size it, from `min` to `max` (100 and 1600 without them).
- `collapsible`: a button folds it to a narrow strip at its edge, and the strip's button opens it again. `collapsed="true"` as well starts it folded.
- `maximizable`: a button lets it fill the window, and gives the window back.
- `columns="3"`: the columns of its own grid, 3 without it.

Code reads and sets `nav.collapsed` and `nav.maximized`, on a sidebar that allows them, so a handler can open it or let it fill the window. In the designer, pick a sidebar on the canvas to set all of this under Properties. 
## Commands

Menu items and toolbar and ribbon buttons are commands. Each has a Name, a Caption and an Icon; a toolbar or ribbon button needs a caption or an icon. A click either runs a handler or opens something, not both:

- `onclick="ordersClick"` runs a handler. **Wire** names it after the command (`ordersClick`), writes it and opens the Code panel.
- `opens="CustomerBrowse"` ("Or on click, open") opens a window, with `newWindow="true"` in a desktop window of its own, or one of the built-in targets: `opens="close"`.

An item with a submenu opens the submenu and has no click of its own.

## Built-in click targets

- `quit` ends the application.
- `close` closes the current window and goes back to the one below.
- `language` lets the person pick the language the application shows, from the project's translations. See [Languages](languages.md).
- `theme` lets the person pick one of the themes the project offers. See [Themes](themes.md).
- `keys` opens Clarity's keys window, where the person sets the application's shortcut keys for themselves. It needs `keys editable` in the project file (the switch in Project settings).

The application remembers the chosen language and theme for its next start. A button on a window can use `quit` and `close` too; `language` and `theme` are for menus and toolbars.

## Keys

A menu item, a toolbar button and a ribbon button can have a key, written as modifiers and one key joined by `+`: `key="Ctrl+N"`, `key="Ctrl+Shift+F2"`. The menu shows it next to the caption, and a button's tooltip shows it. A ribbon button's key works whichever tab is open, and a menu item's while the menu is shut. A button on a window takes a key in the same way (Shortcut in the properties), and works anywhere in the window. The modifiers are `Ctrl`, `Alt`, `Shift` and `Meta`. See [the language reference](../LANGUAGE.md) for the keys you can name.

What is innermost goes first: a key of the window's own buttons wins over the same key in the frame, so a window can take a frame's key for itself. Within one place, one key runs one command: two different commands with the same key in the frame, or among a window's buttons, is an error. A menu item and a button that open the same window or run the same handler are one command, and may share a key. What code hid or disabled does not answer its key.

Keys work alike on every stack.

### The keys window

With `keys editable` in the project file, each person can change the keys for themselves. A menu item or button with `opens="keys"` opens the keys window. It lists every named command that does something, Everywhere (the app's frame) first and then each window's own commands and buttons, also those without a key in the design. Pick Change and press the new key: a key another command has in the same place offers to swap the two, and a key another place has is allowed, the window saying which command goes first where. Letters without Ctrl or Alt, Tab, Enter and Escape cannot be used. Reset puts a key back to the design's, Reset all every one. The keys are kept in the person's settings file, beside their other settings, as only what differs from the design, so a key you change in a new version still reaches whoever left it as it was. The window speaks in Clarity's own words, which the project translates (see [Languages](languages.md)).
