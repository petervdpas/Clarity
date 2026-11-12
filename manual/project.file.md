# The project file

The settings in the project file (stack, output, language, currency, themes, data sources and file stores) and the Project settings view that edits them.

## One block of settings

The project file, such as `shop.clarity`, is the file you open. It holds one `project` block:

```clarity
project Shop
    title "Shop"
    stack rust desktop
    output "out"
    language "en"
    currency EUR
    theme Shop Night
    datasource main sqlite "data/shop.db"
    datasource erp sqlite "legacy/erp.db" external readonly
    filestore photos "data/photos"
```

Only `stack` is required. Each setting appears once, except `datasource`, `filestore` and `codec`. An unknown setting or value is an error that names the line and lists the words that are accepted.

## The settings

- `project Name`: the project's name, a Clarity name like `MyShop`. It is also the program's name.
- `title "text"`: the name people see, in the window title. Without it, it is derived from the name.
- `stack language ui target...`: what the project is transpiled into, read by position. Languages: `rust`, `go`, `csharp`, `c`, `cpp`. No language names a UI: the windows are drawn by Clarity.UI, Clarity's own UI framework (see [C# with Clarity.UI](stacks.csharp.md) and [Rust with Clarity.UI](stacks.rust.md)), so the targets follow the language. The web target draws the same windows in the browser: the program serves a page on which Clarity.UI draws them, in every language. Targets: `desktop` for the program with windows, `web` for a web server, `cli` for a command-line program; one or more of them. A project whose only target is `cli` names no UI. For example: `stack rust desktop`, `stack rust web`, `stack go web`, `stack go desktop cli`, `stack go cli`, `stack csharp desktop`, `stack csharp web`. With the target `web` the program is a web server: open its address in a browser to use it. It listens on this computer at the project's port, 8080 unless the project file says `port 9090` (or the Project tab's Port field), and elsewhere when you start it with `--address host:port` or set `CLARITY_ADDRESS`. Run in Clarity starts it there; the Browser button opens it. With both targets, `desktop web`, the program opens its window, and `--serve` makes it a web server instead. In a browser the window's title is the tab's, a saved file is a download, and the person's settings stay in their browser; each tab runs the application on its own, against the server's database.
- `output "folder"`: where Generate writes the application's source, which Run and Build build. The default is `out`, inside the project's folder; with an Apps folder set in **Clarity settings**, **Folders**, the default is a folder named after the project file there instead. It can also be a full path. See [Run and Build](project.run-build.md).
- `output keeps "vendor", "notes"`: folders in the output that are the project's own. Clarity leaves them where they are and does not count them as somebody else's files. Each is the name of one folder in the output, not a path. The folders the stack's build tools make (such as `target`, `bin` or `build`) are kept without this line.
- `language "tag"`: the language the project's texts are written in, such as `"en"` or `"nl-NL"`. See [Languages](languages.md).
- `currency CODE`: the currency of `money` fields, as its ISO code: `EUR`, `USD`. Without it, amounts show without a currency sign.
- `theme Name...`: the themes the application offers; the first is the one it starts with. Without any, the application looks as Clarity does. See [Themes](themes.md).
- `keys editable`: the people using the application set its shortcut keys themselves, in Clarity's keys window (`opens="keys"`). In Project settings it is the switch "People set the shortcut keys themselves". See [The app and its frame](design.frame.md).
- `engines interpreted`: the engines written in Clarity run with Clarity's interpreter instead of compiled; without the line (or with `engines compiled`) they are compiled to WebAssembly. Both give the same answers on every stack. In Project settings it is the **Engines** choice.
- `row height 30`: how high a row of the windows' grid is, in pixels: a whole number from 24 to 64. Without the line a row is 36 high. Inputs and buttons are a row high, and a table's rows are never higher. The design canvas, the preview and the application all lay out by it. Clarity's own fields are 30 high, so `row height 30` makes an application's fields match them. In Project settings it is the **Row height** field.
- `filestore name "folder"`: a folder that keeps pictures and files. See [Pictures and files](data.pictures-files.md).
- `codec name from "path"`: a codec library the project adds to its videos, its path beside the project without the system's extension (`codec vp9 from "codecs/libvp9"` reads `codecs/libvp9.so` on Linux, `.dylib` on macOS, `.dll` on Windows). See [Video](design.video.md).

## Data sources

A `datasource` line names a data source, its kind and where it is:

```clarity
datasource main sqlite "data/shop.db"
datasource sales postgres "env:SHOP_DATABASE"
datasource erp sqlite "legacy/erp.db" external
```

- Kinds: `sqlite`, `postgres`, `mysql`, `mssql`, `rest`, and a format for a file in it, `csv`, `json`, `xml` or `yaml` (always `external`, with the format's options on indented lines such as `separator ";"`; see [A file as a data source](data.sources.md#a-file-as-a-data-source)). A SQLite file is relative to the project's folder. The server databases take a connection string or URL.
- `"env:NAME"` reads the location from that environment variable when the application starts, so a password never sits in the project.
- `external` marks a database Clarity does not own: it reads and writes the records, but never changes the tables. `readonly` means records are only read; saving or deleting them is an error. A built program keeps the SQLite database the project keeps beside it in the person's folder for application data (`~/.local/share/<program>` on Linux, `~/Library/Application Support/<program>` on macOS, `%APPDATA%\<program>` on Windows), and Clarity's Run uses the project's own file; `beside` keeps it in the project's folder for the built program too, for an app that lives in its own folder.
- A `rest` source is a web API, with an indented `auth` line (`bearer`, `api-key` or `basic`) that names where the secret is, such as `"env:NAME"` or `"setting:name"` (a [secret setting](../docs/LANGUAGE.md#secret-settings), kept in the system's protected store). Its entities live in the service, in a module with `source name`. An indented `paging` line says how it hands out a long list (`paging page "page" size "per_page" 100`; see [Every page of a list](code.web-apis.md#every-page-of-a-list)). An indented `openapi "file.yaml"` names its OpenAPI description in the `apis` folder, and `pick "operationId", ...` (and `pick schema "Name", ...`) what the application uses of it; see [Calling web APIs](code.web-apis.md#an-openapi-description).

Entities live in the first data source unless their module names another with `source`. See [Data sources](data.sources.md).

## The Project settings view

On the **Project** tab, click **Settings** (or the app folder or the stack in the sidebar). The view edits every setting:

- Project: Name, Title, the project's Folder, and the App folder (the `output` setting), chosen with Browse in Clarity's file browser (see [Opening a project](start.overview.md#opening-a-project)) and set back to `out` with Default, and the folder Generate writes to.
- Stack: Backend, UI and Targets (Desktop, Web, CLI), and the Build tools row with Environment..., which opens Clarity settings where the tools are chosen.
- Language, formats and theme: Language, Currency, Keys, Engines, Row height, Port (for the `web` target), and the themes that are offered, with the one the application starts with.
- Data sources: a row per source with its kind, location, auth and the External and Read-only boxes; Import for an external SQLite database or a file source (for an XML file, its repeated elements or its XSD schema), and Own for an external SQLite database; the format's options under a file source; API for a web API, which picks what the application uses of its description; Add data source.
- File stores: a row per store, with Browse, which chooses its folder; Add file store.
- Below them, the Migrations of each database and the Translations table.

A change is written to the file at once. Clarity rewrites only the lines whose value changed, so your comments and blank lines stay. A change the file cannot take is refused with its reason, and the view shows the file as it is. Undo on the Design tab takes a settings change back too.
