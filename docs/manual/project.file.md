# The project file

The settings in the project file (stack, output, language, currency, themes, data sources and file stores) and the Project settings view that edits them.

## One block of settings

The project file, such as `shop.clarity`, is the file you open. It holds one `project` block:

```clarity
project Shop
    title "Shop"
    stack rust web desktop
    output "out"
    language "en"
    currency EUR
    theme Shop Night
    datasource main sqlite "data/shop.db"
    datasource erp sqlite "legacy/erp.db" external readonly
    filestore photos "data/photos"
```

Only `stack` is required. Each setting appears once, except `datasource` and `filestore`. An unknown setting or value is an error that names the line and lists the words that are accepted.

## The settings

- `project Name`: the project's name, a Clarity name like `MyShop`. It is also the program's name.
- `title "text"`: the name people see, in the window title. Without it, it is derived from the name.
- `stack backend ui target...`: what the project is transpiled into, read by position. Backends: `rust`, `go`, `csharp`, `c`, `cpp`. UIs: `web` (the web renderer) or `avalonia`, which is for `csharp` only. Targets: `desktop`, `web`, or both. Clarity generates Rust, Go, C and C++ with `web`, and C# with `avalonia`; C# with `web` is not generated yet. The web target (serving the application over HTTP) is not built yet either: the generated program is a desktop program.
- `output "folder"`: where Run and Build write the generated application. The default is `out`, inside the project's folder; with an Apps folder set in **Clarity settings**, **Folders**, the default is a folder named after the project file there instead. It can also be a full path. See [Run and Build](project.run-build.md).
- `output keeps "vendor", "notes"`: folders in the output that are the project's own. Clarity leaves them where they are and does not count them as somebody else's files. Each is the name of one folder in the output, not a path. The folders the stack's build tools make (such as `target`, `bin` or `build`) are kept without this line.
- `language "tag"`: the language the project's texts are written in, such as `"en"` or `"nl-NL"`. See [Languages](languages.md).
- `currency CODE`: the currency of `money` fields, as its ISO code: `EUR`, `USD`. Without it, amounts show without a currency sign.
- `theme Name...`: the themes the application offers; the first is the one it starts with. Without any, the application looks as Clarity does. See [Themes](themes.md).
- `filestore name "folder"`: a folder that keeps pictures and files. See [Pictures and files](data.pictures-files.md).

## Data sources

A `datasource` line names a data source, its kind and where it is:

```clarity
datasource main sqlite "data/shop.db"
datasource sales postgres "env:SHOP_DATABASE"
datasource erp sqlite "legacy/erp.db" external
```

- Kinds: `sqlite`, `postgres`, `mysql`, `mssql` and `rest`. A SQLite file is relative to the project's folder. The server databases take a connection string or URL.
- `"env:NAME"` reads the location from that environment variable when the application starts, so a password never sits in the project.
- `external` marks a database Clarity does not own: it reads and writes the records, but never changes the tables. `readonly` means records are only read; saving or deleting them is an error.
- A `rest` source is a web API, with an indented `auth` line (`bearer`, `api-key` or `basic`) that names where the secret is, such as `"env:NAME"` or `"keychain:NAME"`. Its entities live in the service, in a module with `source name`. An indented `openapi "file.yaml"` names its OpenAPI description in the `apis` folder, and `pick "operationId", ...` (and `pick schema "Name", ...`) what the application uses of it; see [Calling web APIs](code.web-apis.md#an-openapi-description).

Entities live in the first data source unless their module names another with `source`. See [Data sources](data.sources.md).

## The Project settings view

On the **Project** tab, click **Settings** (or the app folder or the stack in the sidebar). The view edits every setting:

- Project: Name, Title, the project's Folder, and the App folder (the `output` setting), chosen with Browse and set back to `out` with Default, and the folder builds write to.
- Stack: Backend, UI and Targets, and the Build tools row with Environment..., which opens Clarity settings where the tools are chosen.
- Language, formats and theme: Language, Currency, and the themes that are offered, with the one the application starts with.
- Data sources: a row per source with its kind, location, auth and the External and Read-only boxes; Import and Own for an external SQLite database; API for a web API, which picks what the application uses of its description; Add data source.
- File stores: a row per store, with Browse; Add file store.
- Below them, the Migrations of each database and the Translations table.

A change is written to the file at once. Clarity rewrites only the lines whose value changed, so your comments and blank lines stay. A change the file cannot take is refused with its reason, and the view shows the file as it is. Undo on the Design tab takes a settings change back too.
