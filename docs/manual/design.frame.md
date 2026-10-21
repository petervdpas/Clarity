# The app and its frame

The `app` block: the main window, the frame every window shares (menu, toolbar, ribbon, sidebars, footer), keys, and the built-in click targets.

## The app block

A project has one `app` block, in any module. It names the window the application starts with and declares the frame that every window shows:

```clarity
app Shop
    title "Shop"
    main CustomerBrowse
    on start: appStarted
    menu
        item "File" name fileItem
            item "New customer" name newCustomerItem key "Ctrl+N" on click -> CustomerForm
            separator
            item "Quit" name quitItem key "Ctrl+Q" on click -> quit
        item "View" name viewItem
            item "Language" name languageItem on click -> language
            item "Theme" name themeItem on click -> theme
    toolbar
        button "Customers" name customersButton icon "list" on click -> CustomerBrowse
        separator
        button "Orders" name ordersButton on click: ordersClicked
    footer
        status "Ready"
```

- `main Window` is the window shown at start; it must be a `window`.
- `on start` runs once, before the main window opens. The app's handlers are in `code Shop`.
- `data name: Entity` gives the parts in the sidebars and footer data of their own.

To design it, click **App** on the Project tab, or App at the top of the project tree. A new project already has one; **New app** makes one when there is none. The canvas then shows only the frame, and The window in the properties has the title, the Main window and the start event.

## Regions

In the Parts tab, the Frame group holds the regions. Drag one onto an edge of the window: a menu, toolbar or ribbon docks at the top, a sidebar on the left or right, a footer at the bottom. A new menu starts with File and Quit.

- `menu`: `item "Caption"` lines, which can hold items below them (a submenu), and `separator`.
- Every menu item and button has a camelCase name, as every part does: `item "New customer" name newCustomerItem`. A new one is named after its caption and kind, as `ordersButton`; a separator has none. Double-click an item or button to go to its click handler, which Clarity writes when there is none yet.
- `toolbar`: `button` and `separator`.
- `ribbon`: `tab "Title"` blocks with `group "Title"` blocks of buttons and separators; a button can be `large`.
- `sidebar left` or `sidebar right`, `span n` columns wide (3 by default): parts on a grid of its own.
- `footer`: an optional `status "text"` and parts on a one-row grid of 12 columns.

Pick a region or a command on the canvas to edit it. The properties panel then offers Add item, Add submenu item, Add separator, Add button, Add tab and Add group, Move up and Move down, and Remove. A sidebar has its Side and Width in columns; a footer its Status text.

A `window` can declare regions of its own in the same way. A window's region replaces the app's region of that kind, and its sidebar replaces the app's sidebar on the same side; the other regions still come from the app. A `dialog` and a `view` have no frame.

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
