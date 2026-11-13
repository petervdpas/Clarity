# The project file

The project file and every line it takes (stack, output, version, icon, language, currency, themes, data sources, file stores, sign-in, agents, codecs), and the Project settings view that edits them.

## The project and its folder

A project is one file, such as `shop.clarity`: the file you open. Next to it is a folder with the same name, which holds the modules, the windows, the translations, the resources, the migrations and the data:

```text
shop.clarity           the project file: open this one
shop/
    customers.clarity  modules
    sales.clarity
    design/            a .cxaml file per window and view, and the app's
    translations/      a file per language
    resources/         pictures, Markdown and other files the program carries
    migrations/        the steps that bring each database to the entities
        main/          one folder per SQL data source
            0001.clarity
    data/
```

The project file holds one `project` block. You rarely type it: the Project settings view below writes it for you.

## The Project settings view

On the **Project** tab, click **Settings** (or the app folder or the stack in the sidebar). The view edits every line:

- Project: **Name**, **Title**, **Icon** (a picture among the resources, or Clarity's own), **Version**, the project's **Folder**, and the **App folder** (the `output` line), chosen with **Browse...** in Clarity's file browser (see [Opening a project](start.overview.md#opening-a-project)) and set back to `out` with **Default**; **Output keeps**, the folders of the output that are the project's own, and **Codecs**, a row per codec library with its name and path, with **Add codec**.
- Stack: **Backend**, **UI** and **Targets** (Desktop, Web, CLI), and the Build tools row with **Environment...**, which opens Clarity settings where the tools are chosen.
- Language, formats and theme: **Language**, **Currency**, the switch **People set the shortcut keys themselves**, **Engines**, **Row height**, **Port** (for the `web` target), and the themes that are offered, with the one the application starts with.
- Data sources: a row per source with its kind, location, auth and the **External**, **Read-only** and **Beside the project** boxes; **Import...** for an external SQLite database or a file source, **Own...** for an external SQLite database, **API...** for a web API, the format's options under a file source, and **Add data source**. See [Data sources](data.sources.md).
- File stores: a row per store, with **Browse...**, which chooses its folder; **Add file store**. See [Pictures and files](data.pictures-files.md).
- Below them, the Migrations of each database and the Translations table.

Who signs in has a view of its own, **Sign-in** in the project tree (see [Signing in](project.signin.md)), and so has what the app offers AI agents, **Agents** (see [Agents](project.agents.md)).

A change is written to the file at once. Clarity rewrites only the lines whose value changed, so your comments and blank lines stay. A change the file cannot take is refused with its reason, and the view shows the file as it is. Undo on the Design tab takes a settings change back too.

## The project block

```clarity
project Shop
    title "Shop"
    icon "shop.png"
    version "1.2.0"
    stack rust desktop
    output "out"
    language "en"
    currency EUR
    theme Shop Night
    datasource main sqlite "data/shop.db"
    datasource erp sqlite "legacy/erp.db" external readonly
    datasource payments rest "https://api.example.com/v2"
        auth bearer "env:PAYMENTS_TOKEN"
    filestore photos "data/photos"
```

- The file starts with `project Name`; every setting is a line indented under it. A line that is not indented is refused: "a project file holds one project; indent its settings".
- Only `stack` is required: without it, "the project needs a 'stack' line, like: stack rust desktop".
- Each setting appears once, except `datasource`, `filestore` and `codec`. A second one is refused: "'title' is set twice".
- An unknown setting is an error that lists the settings, and a wrong value an error that names the line and the words that are accepted. A typo never silently picks a default.
- The lines indented under a `datasource` (`auth`, `paging`, `openapi`, `pick` and a file format's options) and under `signin` belong to that line.

### `project`

The project's name, a Clarity name like `MyShop`. It is also the program's name.

```text
project Shop
```

A name that is no Clarity name is refused: "'my shop' is not a valid project name; use a name like MyShop, and a title for the text people see".

### `title`

The name people see, the program's name: its settings folder and its package use it, and a window shows it when neither the window nor the app has a title of its own. Without it, it is derived from the project's name.

```clarity
title "Shop"
```

It is plain text, without slots. To show the version in a title, write `{app.version}` in the window's or the app's title (`"Todo v{app.version}"`); a title in the project file with braces is refused with that advice.

### `icon`

The application's icon: a picture among the project's resources (PNG, JPEG, WebP or SVG, square, best 256 by 256 or larger). Its windows and taskbar button show it on every desktop stack, and its web page's tab. On Windows Clarity draws every size Windows asks for and writes them into the generated program's `windows/` folder: `app.ico`, which C# names as its `ApplicationIcon`, and a resource object per processor, which Rust, C and C++ link, and Go keeps as `resources_windows_<processor>.syso` beside `main.go` (with the version, when the project has one). No build needs a tool of its own for it. Without the line, the program has Clarity's icon.

```clarity
icon "shop.png"
```

The checker refuses an icon that is not in the resources folder ("icon \"shop.png\": there is no shop.png in the resources folder") or is no picture. A resources folder that holds `icon.png` while the project names no icon gets a warning to name it.

### `version`

The application's version, as `major.minor.patch` with an optional pre-release (`2.0.0-beta.1`). Code reads it as `app.version`, and a text by key shows it with the slot `{app.version}`, which Clarity fills itself (`welcomeVersion "Version {app.version}"`), so a splash, a title or an About dialog shows it where the designer puts it. Each stack's package carries it (C#'s `<Version>`, Rust's `Cargo.toml`, CMake's `project(... VERSION)`), and on Windows the program file's Properties show it.

```clarity
version "1.2.0"
```

Without it nothing shows a version, and a use of `app.version` or `{app.version}` is an error: "the project has no version for app.version to show: write version \"1.0.0\" in the project file".

### `stack`

What the project is transpiled into, read by position: a language, then one or more targets.

```clarity
stack go desktop cli
```

- Languages: `rust`, `go`, `csharp`, `c`, `cpp`. No language names a UI: the windows are drawn by Clarity.UI, Clarity's own UI framework (see [C# with Clarity.UI](stacks.csharp.md) and [Rust with Clarity.UI](stacks.rust.md)), so the targets follow the language. A UI written after the language is refused with the line to write instead: "Rust draws its windows with Clarity.UI and names no UI; write stack rust desktop".
- Targets: `desktop` for the program with windows, `web` for a web server, `cli` for a command-line program beside it, whose commands the modules declare. A project whose only target is `cli` names no UI either: `stack go cli`. At least one target is needed, and each once.
- With the target `web` the program is a web server: open its address in a browser to use it. Clarity.UI draws the same windows on the page it serves, in every language. It listens on this computer at the project's `port`, and elsewhere when you start it with `--address host:port` or set `CLARITY_ADDRESS`. Run in Clarity starts it there; the Browser button opens it. In a browser the window's title is the tab's, a saved file is a download, and the person's settings stay in their browser; each tab runs the application on its own, against the server's database.
- With both targets, `desktop web`, the program opens its window, and `--serve` makes it a web server instead.

Examples: `stack rust desktop`, `stack rust web`, `stack go desktop cli`, `stack go cli`, `stack csharp desktop`, `stack csharp web`, `stack c desktop`, `stack cpp desktop`.

### `output`

Where Generate writes the application's source, which Run and Build build. The default is `out`, inside the project's folder; with an Apps folder set in **Clarity settings**, **Folders**, the default is a folder named after the project file there instead. It can also be a full path. See [Run and Build](project.run-build.md).

```clarity
output "out"
```

The output folder is a folder of its own: the project's folder itself (`"."` or empty) is refused, since Generate removes what it no longer writes.

### `output keeps`

Folders in the output that are the project's own. Clarity leaves them where they are and does not count them as somebody else's files. The folders the stack's build tools make (such as `target`, `bin` or `build`) are kept without this line.

```clarity
output keeps "vendor", "notes"
```

Each is the name of one folder in the output, as a text in quotes. A path is refused: "'lib/vendor' is a path; name one folder in the output, like \"vendor\"".

### `language`

The language the project's texts are written in, as a language tag: `"en"`, `"nl-NL"`, `"sr-Latn-RS"`. It is the default language, whose translation file gives every key its text. Dates and numbers follow the locale of the person using the application; this language where their system names none. Without the line the default language is `en`. See [Languages](languages.md).

```clarity
language "en"
```

A tag that is no language tag is refused: "'english' is not a language tag; write one like en, nl or nl-NL".

### `currency`

The currency of `money` fields, as its ISO 4217 code. Without it, amounts show without a currency sign.

```clarity
currency EUR
```

A code of other than three capital letters is refused: "'eur' is not a currency code; write its ISO code, like EUR or USD".

### `theme`

The themes the application offers, declared in modules; the first is the one it starts with. People switch with the click target `theme`. Without the line, the application looks as Clarity does, and parts cannot have a `style`. See [Themes](themes.md).

```text
theme Shop Night
```

### `keys editable`

The people using the application set its shortcut keys themselves: a menu item or button with the click target `keys` (`opens="keys"`) opens Clarity's keys window, and each person's keys are kept in their settings file. Without the line the keys are the design's, and a click to `keys` is an error: "keysItem clicks to keys, the keys window, which opens when the project file says keys editable". See [Shortcut keys](app.keys.md).

```clarity
keys editable
```

### `engines`

How the engines written in Clarity run: `engines compiled` (the default, also without the line) turns their code into WebAssembly; `engines interpreted` carries their code in the program and runs it with Clarity's interpreter. Both give the same answers on every stack. See [Engines](code.engines.md).

```clarity
engines interpreted
```

### `row height`

How high a row of the windows' grid is, in pixels: a whole number from 24 to 64, 36 without the line. Inputs and buttons are a row high, and a table's rows are never higher. The design canvas, the preview and every stack lay out by it. Clarity's own fields are 30 high, so `row height 30` makes an application's fields match them.

```clarity
row height 30
```

### `port`

Where a web application (the `web` target) listens on this computer: a number from 1 to 65535, 8080 without the line. Started with `--address host:port` or `CLARITY_ADDRESS` it listens there instead.

```clarity
port 9090
```

### `agents while closed`

What AI agents may do through the app's MCP service while the app is closed and nobody is there to allow it: `read`, `change` or both. It is where the settings `agentReadsWhileClosed` and `agentChangesWhileClosed` start, until the person using the app sets them (see [Settings](data.settings.md)). Without the line, agents may do nothing while the app is closed. See [Agents](project.agents.md).

```clarity
agents while closed read
```

### `agents at most`

How many AI agents the running app serves at once through its MCP service: a whole number from 1 to 100, 3 without the line. One more is told why it is refused, and the app's window says it was turned away.

```clarity
agents at most 5
```

### `datasource`

A data source: its name, its kind and where it is, then `external`, `readonly` or `beside` as they apply. Entities live in the first data source unless their module names another with `source`. The kinds, the indented lines a web API or a file takes, and the rules are in [Data sources](data.sources.md).

```clarity
datasource main sqlite "data/shop.db"
datasource sales postgres "env:SHOP_DATABASE"
datasource erp sqlite "legacy/erp.db" external readonly
datasource payments rest "https://api.example.com/v2"
    auth bearer "setting:paymentsToken"
```

### `filestore`

A file store: a name and the folder that keeps the pictures and files of the `image` and `file` fields with `store name`, and the files code lists, reads and writes. The folder is relative to the project's folder, `"env:NAME"` (from that environment variable) or `"setting:name"` (a folder setting each person chooses). See [Pictures and files](data.pictures-files.md).

```clarity
filestore photos "data/photos"
```

Two stores with one name, a name that is no Clarity name, and a store without a folder are refused.

### `codec`

A codec library the project adds to its videos: its name, and its path beside the project without the system's extension. `codec vp9 from "codecs/libvp9"` reads `codecs/libvp9.so` on Linux, `.dylib` on macOS and `.dll` on Windows. Each codec is named once. See [Video](design.video.md).

```clarity
codec vp9 from "codecs/libvp9"
```

### `signin`

Who uses the program: the entity of people and the targets people sign in on, with the strategies and their options indented under it. See [Signing in](project.signin.md).

```clarity
signin Member on desktop web
    password email passwordHash
```

See also [Data sources](data.sources.md), [Settings](data.settings.md), [Languages](languages.md) and [Run and Build](project.run-build.md).
