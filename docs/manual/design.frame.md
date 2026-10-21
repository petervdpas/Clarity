# The app and its frame

The `app` block: the main window, the frame every window shares (menu, toolbar, ribbon, sidebars, footer), keys, and the built-in click targets.

## The app block

A project has one `app` block, in any module. It names the window the application starts with and declares the frame that every window shows:

```clarity
app Shop
    title @shopTitle
    main CustomerBrowse
    on start: appStarted
    menu name mainMenu
        item @fileItem name fileItem
            item @newCustomerItem name newCustomerItem key "Ctrl+N" on click -> CustomerForm
            separator
            item @quitItem name quitItem key "Ctrl+Q" on click -> quit
        item @viewItem name viewItem
            item @languageItem name languageItem on click -> language
            item @themeItem name themeItem on click -> theme
    toolbar name mainToolbar
        button @customersButtonCaption name customersButton icon "list" on click -> CustomerBrowse
        separator
        button @ordersButtonCaption name ordersButton on click: ordersClicked
    footer
        status @shopStatus
```

- `main Window` is the window shown at start; it must be a `window`.
- `on start` runs once, before the main window opens. The app's handlers are in `code Shop`.
- `data name: Entity` gives the parts in the sidebars and footer data of their own.

To design it, click **App** on the Project tab, or App at the top of the project tree. A new project already has one; **New app** makes one when there is none. The canvas then shows only the frame, and The window in the properties has the title, the Main window and the start event.

## Regions

In the Parts tab, the Frame group holds the regions. Drag one onto an edge of the window: a menu, toolbar or ribbon docks at the top, a sidebar on the left or right, a footer at the bottom. A new menu starts with File and Quit.

- `menu name mainMenu`: `item @key` lines, which can hold items below them (a submenu), and `separator`.
- Every menu item and button has a camelCase name, as every part does: `item @newCustomerItem name newCustomerItem`. A new one is named after its caption and kind, as `ordersButton`; a separator has none. Double-click an item or button to go to its click handler, which Clarity writes when there is none yet.
- `toolbar name mainToolbar`: `button` and `separator`.
- The menu and the toolbar are parts with a camelCase name, as `mainMenu` and `mainToolbar`, which a new one gets. Code hides one with `mainToolbar.visible = false` and turns it off with `mainMenu.enabled = false`.
- `ribbon`: `tab @key` blocks with `group @key` blocks of buttons and separators; a button can be `large`.
- `sidebar left` or `sidebar right`: parts on a grid of its own, docked to that edge; see [Sidebars](#sidebars).
- `footer`: an optional `status "text"` and parts on a one-row grid of 12 columns.

Pick a region or a command on the canvas to edit it. The properties panel then offers Add item, Add submenu item, Add separator, Add button, Add tab and Add group, Move up and Move down, and Remove. The menu, the toolbar and the ribbon have their Name; a sidebar its own options (see [Sidebars](#sidebars)); a footer its Status text.

A `window` can declare regions of its own in the same way. A window's region replaces the app's region of that kind, and its sidebar replaces the app's sidebar on the same side; the other regions still come from the app. A `dialog` and a `view` have no frame.

## The ribbon

A ribbon is a named part of the frame, docked at the top, and its tabs and groups have names too:

```clarity
app Shop
    main CustomerCard
    ribbon name mainRibbon collapsible
        tab @homeTab name homeTab
            group @clipboardGroup name clipboardGroup
                button @pasteButtonCaption name pasteButton icon "paste" large on click: pasteClicked
        tab @viewTab name viewTab
            group @zoomGroup name zoomGroup
                button @zoomInButtonCaption name zoomInButton on click: zoomInClicked
```

- `collapsible`: the person double-clicks a tab to fold the ribbon to its tabs, and opening a tab unfolds it again. `collapsible collapsed` starts it folded.
- Code reads and sets the open tab as `mainRibbon.tab`, counted from 1, and the folding as `mainRibbon.collapsed`. A tab or group hides with `homeTab.visible = false`; when the open tab is hidden, the first visible one opens.
- A new ribbon, tab or group is named for you: `mainRibbon`, and a tab or group after its title, as `homeTab` and `clipboardGroup`. Pick it on the canvas to change its name and, for the ribbon, its folding under Properties.

## Sidebars

A sidebar is a part docked to the left or right edge of a window, beside the window's own grid; in the app it shows beside every window that has none of its own on that side. It holds parts on a grid of its own: a list, a form, a canvas, whatever the window needs at hand.

```clarity
page Albums: window
    data albums: list<Album>
    sidebar left name nav width 280 min 200 max 480 resizable collapsible maximizable grid 3 columns
        list name albumList at 1,1 span 3 rows 10 bind albums show title
```

- `name`: every sidebar has one, like every part; a new one is named after its side, as `leftSidebar`.
- `width 280`: its width in pixels, 240 without it; a sidebar is 100 to 1600 pixels wide.
- `resizable`: the person drags its inner edge to size it, from `min` to `max` (100 and 1600 without them).
- `collapsible`: a button folds it to a narrow strip at its edge, and the strip's button opens it again. `collapsible collapsed` starts it folded.
- `maximizable`: a button lets it fill the window, and gives the window back.
- `grid 3 columns`: the columns of its own grid, 3 without it.

Code reads and sets `nav.collapsed` and `nav.maximized`, on a sidebar that allows them, so a handler can open it or let it fill the window. In the designer, pick a sidebar on the canvas to set all of this under Properties. A sidebar written the old way, `sidebar left span 3` (in columns), is an error; **Update the sidebars** in Problems writes it in pixels and names it.

## Commands

Menu items and toolbar and ribbon buttons are commands. Each has a Name, a Caption and an Icon; a toolbar or ribbon button needs a caption or an icon. A click either runs a handler or opens something, not both:

- `on click: handler` runs a handler. **Wire** names it after the command (`orders_click`), writes it and opens the Code panel.
- `on click -> Target` ("Or on click, open") opens a window, optionally `in new window`, or one of the built-in targets.

An item with a submenu opens the submenu and has no click of its own.

## Built-in click targets

- `quit` ends the application.
- `close` closes the current window and goes back to the one below.
- `language` lets the person pick the language the application shows, from the project's translations. See [Languages](languages.md).
- `theme` lets the person pick one of the themes the project offers. See [Themes](themes.md).

The application remembers the chosen language and theme for its next start. A button on a window can use `quit` and `close` too; `language` and `theme` are for menus and toolbars.

## Keys

A menu item can have a key, written as modifiers and one key joined by `+`: `key "Ctrl+N"`, `"Ctrl+Shift+F2"`. The menu shows it next to the caption. A button on a window takes a key in the same way (Shortcut in the properties), and works anywhere in the window. The modifiers are `Ctrl`, `Alt`, `Shift` and `Meta`. See [the language reference](../LANGUAGE.md) for the keys you can name.

On C# with Avalonia, a menu item's key also works when pressed. On the stacks with the web renderer, the menu shows the key but pressing it does nothing yet.
