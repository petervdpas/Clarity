# Clarity: Design Document

Status: draft 0.2
Date: 2026-10-16

## 1. Purpose

AI tools now write code faster than humans can review it. The bottleneck has moved from producing code to understanding it, and the gap fills up with code that works but that nobody on the team chose on purpose. Over time the architecture drifts, intent gets lost, and humans lose control of their own codebase.

Clarity keeps that control. It is a development environment that enforces a small set of engineering principles, for AI agents and humans alike:

- Modular structure with explicit, allowed dependencies
- Test-driven development
- No duplicated code or knowledge (DRY)
- The backend is the single source of truth; the frontend is driven by it

It does this in two ways. For new projects, the whole application is written in Clarity, its own language: the windows are designed visually, the behavior behind them is written in Clarity, and Clarity transpiles both into real frontend and backend code for a chosen stack. Nobody, human or AI, writes or edits that generated code. For all code, including hand-written code and existing projects, a directive layer and a rule engine check every change against the project's rules.

Humans stop reviewing every line. They own the spec, the rules and the contracts, and Clarity enforces them.

The approach draws on two tools from the early 1990s:

- **Clarion's data dictionary and application generator:** define every table and field once, generate screens and code from it, and write hand code only in designated embed points.
- **Borland Turbo C++ for Windows (Visual Edition):** Resource Workshop designed dialogs visually and saved them as plain-text `.rc` resource scripts, AppExpert generated the application skeleton, and ClassExpert connected controls to code by generating handler stubs and response table entries. The developer wrote only the handler bodies.

Clarity keeps that split (a visual tool writes a text description, generated glue connects it, humans write the behavior) and avoids the weaknesses of both: binary project files, logic scattered through embed points, generated code mixed into hand-written files through marker comments, poor testability and vendor lock-in.

### 1.1 Direction: one language, designer first (2026-10-16)

These decisions replace earlier parts of this document where they conflict:

- **One language.** Clarity is a programming language, not only a spec language. It describes the user interface (windows, their parts and properties) and the behavior behind it (event handlers, actions, validation). Clarity code is transpiled into real frontend and backend code for the chosen stack (backends: Rust, Go, C#, C, C++; the UI is the web renderer or a native toolkit, section 1.2). The target languages are output; in a Clarity project nobody writes them by hand.
- **A project is the unit.** A Clarity project holds the windows, the data and the behavior. Transpiling and compiling it produces a working application with its user interface, for the chosen stack, without any hand work in between.
- **Designer first.** The UI is built in the designer, like Resource Workshop and Delphi: every window is placed together from the same Parts catalog (frame regions, basic input, containers, data, graphics), parts are moved and resized freely on a snap grid, and each part has properties and events. The designer writes Clarity; the text stays readable and diffable.
- **Code behind, in Clarity.** Each part has events (click, change, open, ...). Wiring an event creates a handler in the window's code behind, written in Clarity, which the developer fills in. Clarity generates the typed bindings, so handlers read and set the window's parts and data.
- **Patterns are starters.** "Form for an entity" and "browse for an entity" are templates that create an ordinary window, like AppExpert. They are not separate kinds of screen.

Superseded by this: the form and browse screen kinds (section 5.5 and later mentions of screens), behavior written in the target language (sections 6.1, 6.4 and 6.5), the spec-only scope of the language (section 2), and the restriction of the designer to business patterns (section 8). Those sections are rewritten as the Clarity language is designed. Adopted projects (section 3) still apply rules and directives to existing hand-written code.

### 1.2 Projects, stacks and output (2026-10-16)

A Clarity project is one file, `<name>.clarity`, with a folder of the same name next to it that holds the modules (the project file is described at the end of this section). The project file names the stack the application is transpiled into:

```clarity
project Shop
    stack csharp avalonia desktop   // backend (rust, go, csharp, c, cpp), UI (web or a toolkit), targets (desktop, web)
```

**Backends.** Five backend languages: Rust, Go, C#, C and C++. Each needs one transpile library, made of three parts:

1. **Emitter:** Clarity in, that language out: data, behavior, bindings and the event dispatch to handlers.
2. **Runtime library:** data access, validation from the dictionary, the binding protocol and event dispatch, written by hand once per language (section 7.5).
3. **Shells:** the desktop host and the web server for that language.

| Backend | Desktop shell (web UI) | Web server |
|---|---|---|
| Rust | Tauri | an HTTP server in Rust |
| Go | Wails | net/http |
| C# | WebView2 or Photino; Avalonia when it renders natively | ASP.NET |
| C, C++ | the webview C library | a small embedded HTTP server |

**UI.** The UI is either the web renderer or a native toolkit:

- **Web renderer.** One frontend runtime, shared by every backend and both targets: HTML and JavaScript in a webview on the desktop and in a browser on the web. It is the renderer the designer's preview already uses, so the preview and the real application cannot drift apart. It is built and tested once, so a project does not choose a frontend framework: five backends need one frontend, not five times three. Svelte, Vue or React return only if hand-extending the frontend in that framework becomes a real need.
- **Native toolkits.** A project can render its windows with a native toolkit of its backend instead. Each toolkit is one more transpile library: an emitter for its markup and a small runtime. Avalonia for C# comes first; Qt for C++ and Slint for Rust can follow where they are wanted. Avalonia can also target the browser through WebAssembly.

The window model maps onto a toolkit without loss, because it does not depend on one. For Avalonia:

| Clarity | Avalonia |
|---|---|
| a window on a 12-column grid | `Window` with a `Grid` of 12 `ColumnDefinitions` |
| `textbox at 4,2 span 6 bind customer.email` | `<TextBox Grid.Column="3" Grid.Row="1" Grid.ColumnSpan="6" Text="{Binding Email}"/>` |
| panel, tabs, splitter | `HeaderedContentControl` in a `Border`, `TabControl`, `GridSplitter` |
| menu, toolbar, footer | `Menu`, a toolbar panel, a status bar `DockPanel` |
| `on click: saveClick` | `Command="{Binding SaveClick}"`, a generated command that calls the handler |
| data and validation | a generated ViewModel with typed properties and validation from the dictionary |

The designer is the same for every UI: a window is designed once, and the stack decides how it is rendered. For a native toolkit the preview shows the layout faithfully but not pixel for pixel; the real window appears with Run.

**Output.** Clarity keeps an `out/` folder in sync with the project, the way the preview follows every change: a complete project for the chosen stack that its own toolchain compiles (`Cargo.toml`, `go.mod`, a `.csproj`, `CMakeLists.txt`). Nobody edits it; it is regenerated deterministically from the Clarity files. **Run** on the Preview tab compiles it and starts the real application; **Build** produces a release. Clarity calls the native toolchains and never replaces them.

**AI.** Clarity knows the whole project as a structured model: data, windows, handlers, diagnostics, the edit history, the generated code, compile errors and, later, runtime measurements. Through its MCP server (section 17.3) an AI can read all of it. An AI changes a project only with the edits the designer uses, so the checker judges its changes like anyone's and Undo takes them back. Optimization works in two loops:

- **Inside a project:** the AI proposes improvements (layout, data model, slow handlers, duplicated logic); the developer accepts or rejects them, and the rules decide what is valid.
- **For Clarity itself:** patterns that recur across projects (the same field added everywhere, generated code that is always slow) become proposals for better templates, emitters and rules, and improve Clarity as a whole once a human approves them.

Generation, checking and enforcement stay deterministic (section 2): the AI proposes, Clarity decides what is valid, humans decide what is accepted.

Superseded by this: the frontend framework choice (sections 7.2 and 11.4 where they assume Svelte, Vue or React output) and the stack milestones M2, M5, M7 and M10 in the roadmap.

**The project file (2026-10-16).** A project is one file, `<name>.clarity`, which is what you open, with a folder of the same name next to it that holds the modules, the output and the data. The settings are written in Clarity, so the whole project is one language:

```text
examples/
  shop.clarity          the project: open this
  shop/
    customers.clarity   modules
    sales.clarity
    out/                generated application (setting `output`)
    data/
```

```clarity
project Shop
    stack rust web desktop
    output "out"
    datasource main sqlite "data/shop.db"
    datasource payments rest "https://api.example.com/v2"
        auth bearer "keychain:payments"
```

The settings are `title` (optional; the name people see, derived from the project name when left out), `stack` (backend, UI, then one or more targets, read by position), `output` (default `out`, relative to the project's folder) and one `datasource` line per data source, with an indented `auth` line for web APIs. Reading is strict: an unknown setting or value is an error that names the line and the accepted words. The Project tab edits the settings and writes only the lines whose value changed, so comments and blank lines written by hand stay. This replaces the earlier `clarity.toml`.

### 1.3 Data sources and migrations (2026-10-16)

A project declares where its data lives. The project file lists one or more data sources, in order; the first is the default:

```clarity
project Shop
    stack rust web desktop
    // sqlite, postgres, mysql, mssql or rest; secrets only as env: or keychain: references
    datasource main postgres "env:SHOP_DB_URL"
    datasource payments rest "https://api.example.com/v2"
        auth bearer "keychain:payments"   // bearer, api_key or basic
```

The text after the kind is a file path for SQLite (`data/shop.db`), a connection for the SQL servers and the base URL for a web API. Passwords, tokens and connection strings with secrets never appear in the project file; it only says where to find them (`env:` or `keychain:`), and the checker refuses a connection that contains a password. A new project gets one SQLite source named `main`.

**Which entity lives where.** A module says it with a `source erp` line; its entities live in that source. Without one they use the first source, so a project with one source never mentions it. Each owned SQL source has its own migration steps (`migrations/<source>/`) and its own database at run time; a reference into another database is a plain id without a foreign key (D1c, 2026-10-17). Windows do not know which source is behind an entity: they bind to the dictionary, and the dictionary supplies labels and validation for every source.

**The Project tab.** The sidebar gets a Data sources section: add, edit and remove a source, and test its connection.

**SQL sources** (SQLite, PostgreSQL, MySQL, Microsoft SQL Server) come in two kinds:

- **Owned by Clarity.** The dictionary drives the schema: tables, keys, constraints and indexes, and migrations when the dictionary changes (below).
- **External.** For a database that already exists, marked `external` on its data source line. Clarity imports its schema into the dictionary, the way Clarion's dictionary import does, as entities mapped onto the tables with `table`, `key` and `column`, and never changes their tables: no migration steps, no migrating at run time. Each load compares the entities with the live tables and reports what no longer fits. `readonly` refuses `save` and `delete` (D1e, 2026-10-17). "Take ownership" turns an external source into an owned one: its current schema becomes the baseline migration, and from then on Clarity migrates it.

Each backend's transpile library maps every database onto that language's usual library: `database/sql` for Go, EF Core or Dapper for C#. The Rust runtime has a driver per database (rusqlite, postgres, mysql, tiberius), each a cargo feature that a generated application turns on only for the databases its sources are, and writes the SQL of each in one place (D1d, 2026-10-17).

**REST and other web APIs.** Clarity does not own their schema. An entity maps onto endpoints (list, get, create, update, delete) with a field mapping, paging and the source's auth (bearer token, API key, basic). Importing an OpenAPI document creates the entities and the mappings in one step. What the API does not offer, the designer does not offer either: an entity whose API is read-only can be browsed but has no Save.

**The UI never talks to a data source.** Not on the desktop and not on the web: every access goes through the generated backend, so credentials stay out of the frontend and the runtime library applies the dictionary's validation before anything is written.

#### Migrations

Clarity has migrations built in, for every source it owns. Its advantage over the usual migration tools: every dictionary change is a designer edit with a known intent. A renamed field is a rename, not a dropped column and an added one, so no data is lost by guessing.

- **Steps, and no snapshot.** The project keeps `migrations/` with numbered steps (`0001.clarity`), committed with the project. Replaying the steps gives the schema as last migrated, so there is no separate `schema.lock` to keep in step with them (D1b, 2026-10-17).
- **Pending changes.** When the dictionary differs from the replayed schema, the checker warns on the entity, the generator refuses to write the application, and the Project tab lists the changes: entity added, renamed or removed, field added, renamed, retyped or removed, unique on or off. Renames are proposed when a name goes and one of the same type comes, and confirmed or declined there, since the text can be edited outside the designer too. **Create step** writes them as the next file; `clarity migrate` does the same.
- **Written in Clarity.** Steps do not depend on a database. Each database's transpile library turns them into its own SQL dialect, so one project can migrate SQLite on the desktop and PostgreSQL on a server.
- **Risky changes need a decision** before the step is created:
  - A field that becomes required asks for a default, or a Clarity expression that fills the existing rows.
  - A narrowing type change (`text(80)` to `text(20)`, `decimal` to `integer`) shows how many rows in the dev database would not fit.
  - Removing a field or an entity is marked destructive and needs confirmation.
- **Data migrations.** A step can carry Clarity code that transforms rows, for example splitting `name` into `first_name` and `last_name`.
- **Down steps.** A step gets a down step when it can be reversed, and is marked one-way when it cannot (a removed field).
- **Applying.** Run on the Preview tab applies pending steps to the dev database first. The generated application checks its schema version at startup and applies what is missing; a command-line switch applies them separately, for server deployments where an administrator does it. Every step runs in a transaction where the database allows it.

**Moving data between databases.** When a project switches a source from one database to another (SQLite to PostgreSQL, say), the dictionary describes both sides, so Clarity can generate a copy tool that moves the data. It comes after the rest.

Superseded by this: section 7.4 (persistence), which assumed SQLite first and PostgreSQL only with the web target.

### 1.4 The Rust output (2026-10-16)

`clarity gen` (and later Run and Build) writes the application into `out/`. Generation is deterministic: the same project gives the same files byte for byte, unchanged files are not rewritten, and files that are no longer generated are removed. The build output of the stack (`target/` and `Cargo.lock` for Rust, `bin/` and `obj/` for C#) and the application's `data/` belong to the build and stay.

- **Layout:** a Cargo project with one Rust module per Clarity module (`src/sales.rs`), `src/schema.rs` with the migration steps, `src/main.rs`, and the `clarity-runtime` crate copied in, so `out/` builds on its own.
- **Records:** every entity is a struct with an `id` and a `Record` implementation; enums are Rust enums stored by name. Text is `String` (empty means no value), integer `i64`, decimal and money `Decimal`, date and datetime `Option<...>`, and a `ref` is the id of the record it points at. Reading `order.customer.credit_limit` loads the customer through the database. List fields are not generated yet; the generator says so.
- **Storage:** SQLite, in the file the data source names; a relative location is in the project's folder, for Clarity and the application alike (so the development database is `shop/data/shop.db`, not inside `out/`). The project's migration steps become SQL (a project loaded without its `migrations` folder gets the whole dictionary as one step). Changing or removing a column builds the table again and copies the rows, converting values where the stored type changes. Steps run with references unchecked and are kept only when every reference still points at a record. Decimals and dates are stored as text, so nothing loses precision. A record that is still referred to cannot be deleted.
- **Logic:** functions and actions become Rust functions that take the database and return a `Result`. `require` and `ensure` become runtime checks whose failure names the condition as written in Clarity. The emitter takes every type from the checker, so the checker is the only place types are decided.
- **Window data:** list data loads all records when the window opens, single data starts as a new record, and `open Window with ...` passes records in. Handlers narrow a list with a query (`customers = find c in Customer where ...`).
- **Queries:** `find` becomes a `Query` of the runtime: a tree of conditions on columns with the values as parameters, which the runtime writes as SQL. Generated code never contains SQL text, so another database is another way of writing the same tree in the runtime. Decimals and money are stored as text and compared and ordered as numbers. Unique text fields get a unique index that leaves empty text out, since empty text means no value.
- **The binding.** The renderer and the backend are bound three ways, all declared in Clarity: data (`bind customer.name`: the window's data goes to the parts, and what the user types goes back with the next event), events (`on click: saveClicked`: the screen reports the event, the backend runs the handler) and properties (`save.enabled = ...`: the reply carries part properties to the screen). Each window becomes a Rust struct of its data; each handler a function on it with a `Ui` for messages, windows to open and `close`. The frontend is the designer's renderer, built once (`npm run build:runtime`) and copied into `out/ui`; the shell is Tauri. For a native toolkit (G2) the same Clarity becomes that toolkit's own bindings.
- **Run and Build** on the Preview tab generate `out/`, build it with the stack's own tools (cargo for Rust, dotnet for C#; `clarity_gen::toolchain` says how) with their output in an Output panel, and start the program (Run) or build a release (Build); Stop ends it. The build output and the application's `data` are left alone by generation.
- **The output folder** is set in the project file (`output "out"`, the default): a folder in the project's folder or a full path elsewhere, never the project folder itself. Because generation removes what it no longer produces, Clarity writes only into an empty folder or one that carries its `.clarity-out` marker.
- **Fluid display.** Windows always resize with the screen, on the web and the desktop alike. The columns share the width; rows that hold a part that grows (a table, a list, an image or a canvas, or a panel, tabs or a splitter that holds one) share the extra height, and the other rows keep their height, so inputs never stretch. A window smaller than its design scrolls. The designer keeps every row at its fixed height, so placing parts stays exact. A native toolkit gets the same rule from its own layout: in Avalonia, rows of growing parts become `*` rows. Sidebars in a running application have a splitter on their inner edge; the designed width is where they start.

### 1.5 Languages, formats and themes (2026-10-17)

An application made with Clarity is shipped to people who read another language, write dates another way and expect their own house style. All three are part of the project, written once and carried to every stack. Nothing here may depend on a browser: the C# stack draws with Avalonia, so a theme is not CSS and a translation is not a JavaScript library's file. How each stack gets it is under "Every stack, in its own idiom" below.

**Formats (I1a).** How values look depends on the locale, not on the developer.

- The project names its base language and, for money, its currency: `language "en"` and `currency EUR` in the project file.
- Dates, times, numbers and money are shown and entered in the locale of the person using the application: `17-10-2026` or `10/17/2026`, `1.250,50` or `1,250.50`, `€ 1.250,50`. The locale comes from the system, and the application can offer a choice.
- A field's `format` (accepted today, not used yet) picks a format by name rather than by pattern: `format short`, `format long`, `format percent`. Named formats stay the same across stacks and locales; a pattern does not.
- Money shows its currency's decimals, so `0.0000` from a server database reads `€ 0,00`.
- Values are stored the same way in every locale; only showing and entering them changes.

**Text with slots (I1b).** A translator must be able to move the parts of a sentence, so text in code names its values instead of adding them together:

```clarity
message "Saved {customer.name}"
refuse "Order {order.number} is over its customer's credit limit"
```

- A slot is a name or a member path in braces, checked like any other expression; `{{` writes a brace. Slots are paths only, so a translator sees `{customer.name}` and knows what goes there.
- A slot shows its value in the locale's format (I1a).
- `+` on text stays valid, but the checker suggests slots for text that is shown: a message, a refusal, a caption.

**Translations (I1c).** Every text a person sees is in the project already: labels, help texts, titles, captions, enum labels, menu items, messages and refusals. Clarity collects them; nobody marks them.
- The text as written is the base language. Translations are one file per language for the whole project (2026-10-17), in the module folder, `translations/nl.clarity`. The same words ("Name", "Save") recur across modules, a translator works through one table, and each stack takes one file per language: `.resx` for .NET, a key map for the web renderer.

  ```clarity
  translation nl
      "Customers" "Klanten"
      "Saved {customer.name}" "{customer.name} opgeslagen"
  ```

- A text is found by what it says. A text whose base changes loses its translation, which is what should happen: the old translation no longer says the same thing, and the checker warns that its line is no longer shown anywhere.
- One text that needs two translations (the word "Order" as a noun and as a verb) needs a way to say which is meant where it is used; that is open, and both read the same until then.
- A translation must keep the slots of its base text; the checker reports a missing or unknown slot. Missing translations are listed per language, as information rather than warnings, so a project can grow a language step by step.
- The generated application carries the texts as explicit literal key maps (section 6.7), and changes language while it runs, without restarting.
- The designer gets a Translations table on the Project tab (base text, where it is used, one column per language) and a language choice for the canvas and Preview, so a window can be checked in German before it ships.

**Texts by key (2026-10-21), replacing texts found by what they say.** Design and code name every text people read by a key, `@saveCaption`, and a file per language gives each key its text; the default language (the project's `language`) is such a file too. Finding a text by what it said made a changed text lose its translations silently, gave one word two meanings no way apart (the open point above), and put the default language in the code, where it could not be swapped. With keys:
- Every language file has the same keys (a key one lacks or adds is an error); the default gives every key a text, another may leave one empty, which shows the default's.
- A key's slots are named, and code gives each its value by name: `@orderApproved(number = order.number)` with `orderApproved "Order {number} is approved"`. A translator sees what each slot holds and moves it; the same key works wherever code gives its values.
- A text with letters written in quotes where people read it is an error. A text without letters (a calculator's `"7"`) stays as written.
- The designer proposes a key from the part and what the text is to it (`saveCaption`, `customerNameLabel`), writes it into every language file, and lets it be renamed; Properties edit the default's text. Any edit that writes a text makes it a key (`edit::apply_in`, `keyed`).
- **Make texts keys** (`clarity spec keys`) migrates a project written before: each text becomes a key, old translation files give the same keys their texts.
- Runtimes: `Text` carries a key with its slot values (`{"key": ..., "values": ...}` on the wire); the screen looks it up, a backend fills it itself from the model's languages (falling back to the default). .NET keeps each key as a resource name, the default language as the neutral `.resx`.

**Themes (T1).** A theme is a set of named values, not a style sheet:

```clarity
theme Shop
    mode system
    color accent "#2f6fdf"
    color danger "#c62828"
    font "Inter"
    size 14
    radius 6
    style primary background accent, text white
    style quiet border none
```

- `mode` is `light`, `dark` or `system`; each color can have a value per mode (`color accent "#2f6fdf" dark "#5b8cff"`).
- Clarity's own look is itself a theme, the built-in `theme Clarity` (2026-10-17), which sets every value: 21 color roles (surfaces, frame, text, borders, accent and its tints, selection, focus, inputs, error, warning, success) with a dark value each, fonts for text, headings and code, text sizes, radii per kind, padding, border width, a shadow, and the look of each kind of part (button, input, table header, tab, menu) in its states (hover, pressed, focus). Every theme starts from it and sets only what it changes; a new theme is a full copy of it. The application's own CSS reads only these values, and is kept apart from the designer's CSS.
- Parts pick a named style, `button "Delete" style danger`, never raw colors, so a design stays consistent and the theme can change without touching windows.
- The project file names the themes its application offers (`theme Shop Night`, 2026-10-17), the first one the one it starts with; people switch with the click target `theme`, by each theme's title, and the choice is kept for the next start. Every offered theme must have every style a part uses. A project without any gets Clarity's own, the look the renderer has today, which already follows the system's light or dark mode.
- The web renderer turns a theme into CSS variables, and Avalonia into resources and styles. The designer draws with the same renderer as the application, so the canvas shows the theme as it will ship.
- The designer gets a theme editor with a live preview of a sample window.

**Every stack, in its own idiom.** All of this is the transpiler's job: the project says what, and each stack gets it the way a developer of that stack would write it. There are two renderers, and each feature is done only when both have it:

- **The web renderer**, for Rust with Tauri, Go with Wails, C and C++ with webview, C# with a webview, and the web target.
- **Avalonia**, for C#, the second stack after Rust: on the desktop, and in the browser through WebAssembly.

| | Web renderer | C# with Avalonia |
|---|---|---|
| Formats | `Intl.DateTimeFormat` and `Intl.NumberFormat` (with the currency) in the renderer, which also reads what is typed back into typed values | `CultureInfo`: formatted ViewModel properties and binding converters that format and parse with the culture |
| Text with slots | the backend sends a message as its key and typed slot values; the renderer fills the slots in the locale's format | the ViewModel fills the slots with `string.Format` and the culture |
| Translations | generated literal key maps per language in the UI bundle, switched while the application runs | a `.resx` file per language with a generated typed accessor class, as Visual Studio makes them, switched through `CultureInfo.CurrentUICulture` |
| Themes | Clarity's own theme as the base file (`ui/themes/clarity.css` on `.clarity-app`) and a CSS file per offered theme (`ui/themes/shop.css`): the theme's class (`theme-shop`) on the application holds its variables, `data-scheme="dark"` its dark values, part looks are variables the root CSS reads (`--button-hover-bg`), and a style is a class on the part (`style-primary`); what a theme leaves out falls back to the base through the cascade | the generator merges Clarity's own theme with the project's (Avalonia has no cascade) into a `ResourceDictionary` with `ThemeDictionaries` Light and Dark; part looks override the Fluent theme's per-state resource keys (`ButtonBackgroundPointerOver`, `TextControlBorderBrushFocused`); a style is a style class (`Classes="danger"`); spacing the web root derives from `padding` (three times it beside button text) must be defined once in the generator, so both stacks use the same multiples |

- **Backends send keys, not sentences.** A message or refusal made in a handler leaves the backend as its key and its slot values, typed. The renderer translates and formats it, so formatting lives once per renderer instead of once per backend language. In Avalonia the ViewModel does the same in the same process.
- **The designer** draws with the web renderer for every stack. For Avalonia it shows the theme and texts faithfully but not pixel for pixel, as it already does for layout; Run shows the real window.
- **One answer on every stack.** The platforms take their locale data from different places (`Intl` from the browser's ICU, .NET from ICU or Windows), so the same named format can differ in small ways. A conformance test runs one sample project through every generator and compares the shown texts, formatted values and theme values with one expected set. Where platforms disagree on a named format, Clarity pins it with its own pattern for that locale, so a project looks the same whatever stack it ships on.
- **Every row counts per stack.** I1a to I1c and T1 are done when every stack that exists at that time has them; a stack added later (G2 first) brings them along as part of its own row.

**Order.** Formats first, since they fix visible gaps (dates as raw text, money with four decimals). Then text with slots, since it changes the language and is cheapest to settle while little code is written the old way. Then translations, which build on both. Themes stand apart and can come at any point; they mostly touch the renderer.

### 1.6 One behavior on every renderer: the Clarity UI library (2026-10-19)

Clarity has two renderers: the web renderer, which the Rust, Go, C and C++ stacks share (they cannot differ from each other), and Avalonia for C#. A part must behave the same on both, not only work on both: what it can pick, when `change` and `select` run, which empty choices it offers, how text that is no value is marked. Stock controls each bring habits of their own (a grid that moves to the next row on Enter, a dropdown that clears its value while it is drawn), so generated windows do not use them directly.

- **The library.** Each renderer has Clarity's own parts, with one contract: the web's in `app/src/lib/ui/` (Svelte), Avalonia's in `Clarity.Avalonia`, under the same names: the table (`Table.svelte`, `ClarityTable`) and the inputs `TextInput` (text, password, memo), `NumberInput`, `CheckInput` (a switch is `SwitchInput` on Avalonia), `ChoiceInput`, `RadioInput`, `DateInput` and `DateTimeInput`. Every input tells when the person changed its value to one that differs (`onchange` on the web, `IInput.ValueChanged` on Avalonia), and never for what the code sets; a table cell holds the same input as a form. A generator only places parts and describes them (a table's columns); how they behave is the library's.
- **The cases.** Every part has cases in `conformance/` (`tables.json` for the table, `forms.json` for the inputs of a form), written once: what the person or the code does, and what must come out (events, values, marks, what a field shows). Both renderers run them against their real controls: the web part in a simulated page (`npm test`, vitest with jsdom), the Avalonia control headless (`dotnet test runtimes/csharp/Clarity.Avalonia.Tests`). A difference fails the build of whichever renderer has it.
- **The rule.** A part is done when its cases pass on both renderers. A new behavior gets its case first.
- **Packages.** `Clarity.Runtime` (SQLite), `Clarity.Avalonia`, and one package per server database (`Clarity.Runtime.PostgreSql`, `Clarity.Runtime.MySql`, `Clarity.Runtime.SqlServer`, each with only its own client; a program turns a driver on with `PostgreSql.Use()` and the like before it opens its databases) are NuGet packages on nuget.org, separate projects in one solution, `runtimes/csharp/Clarity.slnx`, with their tests; the version they share is in `runtimes/csharp/Directory.Build.props`, and `dotnet build runtimes/csharp/Clarity.slnx -c Release` builds and packs them all, into each project's `bin/Release` (`dotnet pack` alone packs the last build without compiling). A generated project references `Clarity.Avalonia` (which brings `Clarity.Runtime`) and the driver package of each server database it uses, at that version, and carries no runtime sources. While working on the runtime, `dotnet build -p:ClarityRuntime=<folder>` (or the environment variable `CLARITY_RUNTIME`) builds it on the runtime's own projects in that folder instead; the build tests do so, and one builds on the published packages.
- **Go module.** The Go runtime is the module `github.com/petervdpas/Clarity/runtimes/go` (2026-10-19): packages `clarity` and `shell`, and one package per server database (`postgresql`, `mysql`, `sqlserver`), which turns its driver on when imported (`clarity.UseServer`), as `database/sql` drivers do. Its version is `clarity.Version` in `runtimes/go/clarity/version.go`, published by tagging `runtimes/go/vX.Y.Z`. A generated module requires that version, imports only the driver packages it needs, and lists the requirements as `go mod tidy` would. Its go.sum holds the published module's sums, recorded in `crates/clarity-gen/src/go/runtime.sum` after tagging. While working on the runtime, a go.work next to the module (`go work init .` and `go work edit -replace <module>=<Clarity>/runtimes/go`) builds it on the repository; Clarity keeps that file. The build tests do so, one builds on the published module, and one checks the recorded sums.
- **Rust crate.** The Rust runtime is the crate `clarity-runtime` on crates.io (2026-10-19), with a feature per database (`sqlite` by default, `postgres`, `mysql`, `mssql`). Its version is its own, in `runtimes/rust/Cargo.toml`, released with a `runtimes/rust/vX.Y.Z` tag and `cargo publish -p clarity-runtime`. A generated project depends on that version with only the features of its databases, and carries no runtime sources. While working on the runtime, `.cargo/config.toml` in the project (`[patch.crates-io] clarity-runtime = { path = "<Clarity>/runtimes/rust" }`) builds it on the repository; Clarity keeps that folder. The build tests do so, and one builds on the published crate.

Done: the table, with every kind of cell (text, number, checkbox, switch, choice, date, moment); the inputs of a form (2026-10-19), with the table's cells on the same inputs. Fixed on the way on Avalonia: a text box's `change` ran whenever it was left, even unchanged; a checkbox, dropdown, radio group or date ran `change` when a handler set its value; Enter in a memo ran it too. Not in the cases yet: `enter` and `leave`, and pictures. Rows read again (2026-10-19): a table or list showing a list that a handler reads again or replaces has no row selected afterwards, on every stack: the web backends call the runtime's `deselect` for each part showing that data (`clarity_gen::rows_shown`), Avalonia's table clears it with its new rows; two cases in `tables.json`.

## 2. Goals and non-goals

### Goals

- One language, Clarity, for the whole application: data dictionary, validation, actions with contracts, windows and the behavior behind them, modules.
- Transpilation of Clarity into real code for a chosen backend/frontend pair; the generated code is frozen.
- A visual designer for every window that writes Clarity, never the generated code.
- A directive layer and rule engine that work on top of any mainstream language, through language packs. The core never contains language-specific logic.
- Enforcement at three gates: during AI edits, before commits, and in CI.
- Every rule tunable, with metrics that show how the rules behave and data-based tuning proposals.
- Fully deterministic. No AI model is needed to generate, build, check or enforce anything.

### Non-goals

- A text editor. Clarity integrates with existing editors through LSP. The designer edits screens in the spec, not source code.
- Replacing native toolchains (cargo, CMake, dotnet, go, npm). Clarity calls them.
- Any git operation that changes state. Clarity reads git (`diff`, `log`, `ls-files`) and never runs `init`, `add`, `commit` or similar. Scaffolding tools are called with their opt-out flags (for example `cargo new --vcs none`).

## 3. Two ways to use Clarity

| | Spec-driven project | Adopted project |
|---|---|---|
| Created with | `clarity init --stack ...` | `clarity adopt` |
| Shape defined in | `.clarity` spec files | the existing code |
| Generated code | backend, shell, frontend | none (boundary codegen only, if configured) |
| Hand-written code | action implementations, frontend extensions | everything |
| Rules and directives | all | all, starting in ratchet mode |

Both modes share the rule engine, the gates, the directives and the tuning loop. Spec-driven projects get the strongest guarantees, because most structure cannot be written by hand at all.

## 4. Architecture overview

```text
 surfaces      CLI | LSP server | MCP server | Desktop app (designer + control room)
                                     |
 spec          parser -> neutral model -> generators (backend, shell, frontend)
                                     |
 core          config | index | rule engine | metrics
                                     |
 packs         rust | c | cpp | csharp | go | python | ruby | typescript
                                     |
 native tools  compilers, test runners, coverage, mutation, LSPs
```

- **Spec** parses `.clarity` files into a language-neutral model and runs the generators for the project's stack (sections 5 to 7).
- **Config** reads `clarity.toml`: stack, module map, rules, profiles, boundaries.
- **Index** is a language-neutral model of the source: files, symbols, directives, dependency edges, tests. Built by the packs, stored in `.clarity/index.db` (SQLite), updated incrementally by file hash.
- **Rule engine** evaluates rules against the spec, the index and a diff. Every violation has a stable id, a location and a fix hint.
- **Metrics** records violations, waivers and false-positive marks, and feeds the tuning loop (section 14).
- **Packs** teach Clarity a language (section 15).
- **Surfaces** expose the same engine to humans, editors, AI agents and CI (section 17).

## 5. The spec language

The spec language describes what an application consists of, not how it behaves. It is small on purpose: every construct has one meaning, and there is one canonical formatting (`clarity fmt`).

This section records how the language came about. The reference for what it is today, construct by construct, is [LANGUAGE.md](LANGUAGE.md); where the two differ, LANGUAGE.md is current.

### 5.1 Files and modules

Spec files have the extension `.clarity` and live in the project's folder, next to the project file (section 1.2). Each file starts with the module it belongs to.

```clarity
module sales

use customers
```

`use` follows the module map: a spec module can only reference modules it may use.

### 5.2 Entities and enums

Entities are the data dictionary. Everything about a field is defined here once: type, validation, label, help text, display format. Screens, API types, database schema and frontend validation are all generated from it.

```clarity
enum OrderStatus
    Pending label @orderStatusPending
    Approved label @orderStatusApproved
    Shipped label @orderStatusShipped
    Cancelled label @orderStatusCancelled

entity Customer
    name: text(80), required, label @customerNameLabel
    email: email, unique, label @customerEmailLabel
    credit_limit: money, min 0, default 0, label @customerCreditLimitLabel
    notes: text, help @customerNotesHelp, label @customerNotesLabel

entity Order
    number: text(12), unique, pattern "^ORD-[0-9]{8}$", label @orderNumberLabel
    customer: ref Customer, required, label @orderCustomerLabel
    status: OrderStatus, default Pending, label @orderStatusLabel
    lines: list<OrderLine>, owned, label @orderLinesLabel
    total: money, computed, label @orderTotalLabel
```

Types: `text`, `text(n)`, `integer`, `decimal(p, s)`, `money`, `bool`, `date`, `datetime`, `email`, `url`, `id`, enums, `ref Entity`, `list<T>`.

Field modifiers: `required`, `unique`, `min`, `max`, `pattern`, `default`, `computed`, `owned` (for lists: lifetime bound to the parent), `label`, `help`, `format`.

`computed` fields are declared in the spec and implemented in hand-written code (section 6.4), like actions.

### 5.3 Actions

Actions declare an operation: its signature, contracts and effects. The body is never in the spec.

```clarity
action approve_order(order: Order) -> Order
    require order.status == Pending
    require order.total <= order.customer.credit_limit
    ensure  result.status == Approved
    effects db.write

action order_total(order: Order) -> money
    ensure result >= 0
    effects none
```

Effects use the same vocabulary as directives (section 9.3).

### 5.4 Expression language

Contracts in the spec must work for every backend, so they use a small neutral expression language. Generators translate it to each target language.

- Literals, field access (`order.customer.credit_limit`), enum values
- Arithmetic `+ - * /`, comparison `== != < <= > >=`, boolean `and or not`
- `result`, `old(expr)`, `count(list)`, `sum(list.field)`, `all(list, cond)`, `any(list, cond)`, `x in [a, b]`

No user-defined functions and no calls into hand-written code. Anything more complex belongs in an action and its tests.

### 5.5 Windows

Every window of an application is a `page` block: a `window`, a `dialog` or a `view` (a window's content shown inside the app frame). There are no separate screen kinds. A new window starts blank or from a template:

| Template | Produces |
|---|---|
| Form for an entity | a `form` bound to the record, holding per field a label `for` its input, the input (a memo for text without a maximum length) and a help text when the field has `help`; Save (`default`) and Close (`cancel`) buttons |
| Browse for an entity | a filter, a table of the entity with its first columns, and New and Open buttons |

The result is an ordinary window; the designer changes it like any other. Labels, input types and validation still come from the data dictionary through the bindings.

### 5.6 Pages and widgets

Windows are built widget-first, the way RAD tools such as Turbo C++ for Windows worked: drag widgets from a palette onto a window, dialog or view, then set their properties and bind them to data.

```clarity
page CustomerEdit: window
    title @customerEditTitle
    data customer: Customer
    data orders: list<Order>
    grid 12 columns

    label @nameLabel at 1,1 span 3
    textbox at 4,1 span 6 bind customer.name
    dropdown at 4,2 span 4 bind customer.kind
    checkbox @activeCaption at 4,3 span 3 bind customer.active
    panel @ordersCaption at 1,5 span 12 rows 6
        table at 1,1 span 12 rows 5 bind orders columns number, ordered_on, total
    button @closeCaption at 11,12 span 2 on click -> CustomerBrowse
```

- **Page kinds:** `window` (a top-level window), `dialog` (modal) and `view` (a part embedded in another page).
- **One desktop window, on every stack:** an application has one desktop window (one browser page on the web). `open` shows a window in place, on top of the one that opened it; `close` goes back to the one below, which gets `on return`; closing the last one ends the application. A new desktop window opens only where the design says so explicitly: a click target marked `in new window` (its window starts a stack of its own there; closing its last window closes it and tells the window that opened it), or a `dialog`. The cases every stack passes are in `conformance/windows.json`. Every renderer and generator keeps to this: the web renderer keeps a stack of windows, Avalonia a `Shell` with a view per window.
- **Data:** `data` declares what the page works on, one record (`data customer: Customer`) or a collection (`data orders: list<Order>`). Widgets bind to it.
- **Snap grid:** a page has a grid of columns (12 unless `grid N columns` says otherwise) and rows of fixed height. Every widget sits at a cell (`at column,row`) and covers `span` columns and `rows` rows. Placement feels free in the designer, the result is always aligned, and on narrow windows and the web the grid can reflow (widgets stack in reading order).
- **Containers:** `panel`, `tabs` (with `tab "Title"` pages) and `splitter` (with two `pane` blocks) have an inner grid as wide as their span. Their children use coordinates relative to the container.
- **Binding:** `bind customer.name` connects a widget to a field. Label, input type, length, required and validation come from the dictionary, so a page cannot redefine a field. Unbound input widgets are allowed while designing and reported as warnings.
- **Events:** `on click -> Target` opens a page or screen. Calling actions follows when actions are part of the spec language.

The palette comes from a widget catalog in the Clarity core, not from the designer. Each catalog entry declares the widget's keyword, palette category, default size, the field types it can bind to, whether it is a container, and its extra properties. The palette, the checker and the properties panel all read the catalog, so adding a widget is one catalog entry.

The catalog so far (U1 in the roadmap adds to it in steps):

| Category | Widgets |
|---|---|
| Text | `label` (with `for`), `help` (with `for`), `heading`, `text` (a field's value, read-only) |
| Basic input | `textbox`, `memo`, `password`, `number`, `checkbox`, `switch`, `dropdown`, `radio` (an enum as radio buttons), `date`, `button` |
| Containers | `form` (bound to one record), `panel`, `tabs`, `splitter` |
| Data | `table` (bound to a list, with `columns`), `list` (bound to a list, with `show`) |
| Graphics | `image` (`source "file"`), `canvas` (`surface` and `renderer`, see 5.7) |

The checker reports widgets outside the grid, overlapping widgets, bindings to unknown data or fields, bindings whose field type does not fit the widget (a checkbox needs a `bool`), and click targets that do not exist.

#### Names and events

Every part has a name, unique in its window (sidebars, footer, menus and toolbars included), and the events that fit its kind:

| Parts | Events |
|---|---|
| button, image, menu item, toolbar or ribbon button | `click` |
| text box, memo, password, number | `change`, `enter`, `leave` |
| check box, switch, dropdown, radio buttons, date, tabs | `change` |
| form | `submit` (Enter in one of its inputs) |
| table, list | `select`, `open` |
| canvas | `pointer`, `key` |

```clarity
button "Save" name save at 9,10 span 2 on click: saveClicked
textbox name email at 4,2 span 6 bind customer.email on change: emailChanged
```

`on click: saveClicked` wires the event to a handler, written in Clarity in the window's code behind (section 1.1). Parts placed in the designer get a free name at once (`button1`, `textbox2`), and templates name their parts after the field or caption, so events can be wired straight away. `on click -> Window` remains as the one built-in action, opening a window, so the preview can navigate before code behind exists; a click is wired to a handler or opens a window, not both.

#### Forms, labels and help texts

A label, its input and the input's help text are three separate parts, each placed on the grid like any other. `for` ties a label or help text to an input by its part name:

```clarity
form name details at 1,1 span 12 rows 4 bind customer
    label @nameLabel name nameLabel at 1,1 span 3 for name
    textbox name name at 4,1 span 6 bind customer.name placeholder @fullName
    help name nameHelp at 10,1 span 3 for name
    label @kindLabel name kindLabel at 1,2 span 3 for kind
    radio name kind at 4,2 span 6 bind customer.kind
button @saveCaption name save at 9,6 span 2 key "Ctrl+S" default on click: saveClicked
button @closeCaption name close at 11,6 span 2 cancel on click: closeClicked
```

- **Label.** Clicking it focuses its input, and screen readers announce them together. A label only shows text: it cannot be bound. A field's value, read-only, is a `text` part.
- **Required.** A label for an input bound to a `required` field shows a marker. The rule lives on the entity, so the page has no `required` of its own.
- **Help text.** Shows its caption, or else the `help` of the field its input is bound to. When a handler sets the input's `error`, the help text shows that message instead, and the input is marked.
- **Form.** Binds to one record (`bind customer`) and holds the inputs of that record; an input inside it bound to other data, or a form inside a form, is an error. Enter in one of its inputs fires the form's `submit` event; when that is not wired, Enter clicks the window's default button. Without a caption a form has no frame and its grid lines up with the grid around it.
- **Properties for inputs and buttons.**

| Property | On | Meaning |
|---|---|---|
| `placeholder "text"` | text box, memo, password, number | shown while the input is empty |
| `tooltip "text"` | every part | shown when the pointer rests on it |
| `readonly` | inputs | shows the value but cannot change it; handlers can change it (`notes.readonly = true`) |
| `taborder N` | inputs, buttons | position in the keyboard order; parts without one follow in reading order |
| `key "Ctrl+S"` | buttons (and menu items) | the key that clicks it: `Ctrl`, `Alt`, `Shift`, `Meta`, then a letter, digit, `F1` to `F12` or a named key such as `Enter` or `Delete` |
| `default` | buttons | Enter clicks it; one per window |
| `cancel` | buttons | Escape clicks it; one per window |

Dropping a field from the Data panel places the same parts the form template makes: a label for the input, the input named after the field, and a help text when the field has one and there is room.

#### Frame regions: menu, toolbar, ribbon, sidebar, footer

Application chrome is not placed on the grid. It docks to the edges of a window, around the grid, and holds structured items instead of freely placed widgets:

```clarity
app Shop
    main CustomerCard
    menu name mainMenu
        item @fileItem name fileItem
            item @newCustomerItem name newCustomerItem key "Ctrl+N" on click -> CustomerCard
            separator
            item @quitItem name quitItem key "Ctrl+Q" on click -> quit
        item @helpItem name helpItem
            item @aboutItem name aboutItem on click -> About
    ribbon name mainRibbon collapsible
        tab @homeTab name homeTab
            group @customersGroup name customersGroup
                button @newButtonCaption name newButton icon "plus" large on click -> CustomerCard
                button @browseButtonCaption name browseButton icon "list" on click -> CustomerBrowse
    sidebar left name leftSidebar width 192
        list name recentList at 1,1 span 3 rows 10 bind recent show name
    footer
        status @shopStatus
        text at 10,1 span 3 bind session.user
```

| Region | Docks | Holds |
|---|---|---|
| `menu` | top, above everything | nested `item`s with `on click`, `separator`, keyboard shortcuts (`key "Ctrl+N"`) |
| `toolbar` | top, below the menu | a row of buttons and separators |
| `ribbon` | top, instead of or below the menu | `tab`s with `group`s of large and small buttons |
| `sidebar` | `left` or `right` | its own snap grid, `span` columns wide, with any widgets (navigation lists, filters, tool panels) |
| `footer` | bottom | a `status` text and a one-row, 12-column snap grid with any widgets |

- **Where regions live.** An `app` declares the main window and the frame regions shared by every window of the application. A `page` of kind `window` can declare its own regions, which take precedence over the app's for that window. Pages of kind `view` are shown inside the app frame and never have regions of their own.
- **Overrides per region.** A window's own region replaces the app's region of the same kind, and a sidebar replaces the app's sidebar on the same side; the other regions still come from the app. One project has one `app`, and its `main` must be a screen or a page of kind window.
- **Built-in commands.** `quit` and `close` are click targets that are not pages or screens. Other unknown targets are warnings until actions are part of the spec language. An item with a submenu opens the submenu and has no `on click` of its own.
- **Catalog.** Regions and their items (`item`, `separator`, ribbon `group`, `status`) are catalog entries in a `frame` category, with the dock edges they accept. The palette offers them like any other widget.
- **Designer.** The design canvas shows drop zones on the window edges while a region is dragged, only on the edges that region accepts. A dropped region arrives with starter content (a File menu with Quit, a Home ribbon tab, a Ready status). Clicking a menu item, button, ribbon tab or group picks it; its caption, icon, shortcut, size and click target are edited under "What is picked", where items are also added, nested and moved. Sidebars and the footer use the snap grid like the page body. The app is designed as a window frame around an empty body; a page shows the app's regions dimmed and can drop its own regions to override them.
- **Commands.** Clicks in menus, toolbars and ribbons open pages and screens now, and call actions once actions are part of the spec language. The same target can be bound to a menu item, a toolbar button and a ribbon button, so a command is declared once.

### 5.7 Canvas

A canvas is a standard element, like a field or a dropdown, whose content is free graphics: an image or vector editor, a diagram, a chart, a floor plan, a signature pad or a game. It follows the same split as the rest of the spec: the spec declares the surface, hand-written code draws on it.

What the spec declares:

- **Size:** fixed (`size 800 x 600`) or `fill` (the available space).
- **Surface:** `svg` (vector, retained), `bitmap` (2D pixels) or `gpu` (WebGL/WebGPU).
- **Input:** which events reach the code (`pointer`, `keyboard`, `wheel`, `touch`).
- **Data:** which fields or actions the canvas reads and writes, so the canvas is typed and checked like any other screen element.
- **Tick:** an optional frame rate for animation and games (`tick 60`).
- **Renderer:** the name of the hand-written renderer.

As an element inside a form:

```clarity
entity Drawing
    name: text(80), required
    image: bitmap, max_size "4MB"
    sketch: vector

screen DrawingForm: form Drawing
    name
    image: widget canvas edit
    sketch: widget canvas edit
```

`bitmap` and `vector` are field types. The standard canvas widget views them, and with `edit` it adds built-in editing tools (select, draw, erase, crop, shapes, undo). No hand-written code is needed for that case.

As a full screen:

```clarity
screen Asteroids: canvas
    size fill
    surface gpu
    input keyboard, pointer
    tick 60
    renderer Asteroids
```

**Where the renderer runs** is chosen per canvas:

- **Backend renderer:** the backend produces the content (an SVG scene, a draw list or a bitmap) and the frontend displays it and forwards input. This keeps "backend drives frontend" fully intact and suits editors, diagrams, charts and turn-based games.
- **Frontend renderer:** for real-time graphics, where a round trip per frame is too slow. The renderer lives in `frontend/src/custom/` with generated typed props and input events. Like custom widgets, it cannot call the backend directly; it only uses the generated typed actions the spec allows.

The renderer is hand-written behavior and falls under the normal rules: modularity, TDD (renderers are tested on their scene or draw-list output, not on pixels), and effects.

Built (2026-10-19): the backend renderer, as a draw handler written in Clarity (`on draw`, shapes in the canvas's own units, `conformance/drawing.json`), on every stack. The frontend renderer becomes an engine beside the screen (section 5.11): compiled once to WebAssembly and run in the renderer, with the same shapes.

### 5.8 Canonical form

`clarity fmt` defines the only valid formatting. The parser keeps comments and blank lines (a lossless syntax tree), so the designer, the formatter and humans can all edit the same file without destroying each other's work.

### 5.9 Code behind (2026-10-16)

A window's handlers live in a `code` block with the window's name, in the same file as the window. The designer rewrites only the `page` block, so layout edits never touch the code.

```clarity
page CustomerForm: window
    title @customerFormTitle
    data customer: Customer
    ...
    dropdown name kind at 4,2 span 4 bind customer.kind on change: kindChanged
    textbox name vatNumber at 4,3 span 4 bind customer.vat_number
    button @saveButtonCaption name saveButton at 9,8 span 2 on click: saveClicked

code CustomerForm
    handler kindChanged
        vatNumber.visible = customer.kind == Business

    handler saveClicked
        if customer.name == ""
            message error @nameIsRequired
            return
        save customer
        message @savedName(name = customer.name)
        close
```

**Where handlers run.** In the backend. When an event fires, the UI sends it with the window's state; the handler runs and the backend returns what changed: data, part properties, messages, windows to open or close. With a native toolkit (Avalonia) the same handler becomes a ViewModel command. The UI never holds behavior of its own.

**Names in a handler.** A handler sees the window's data (`customer`), its parts by name (`vatNumber`, `save`) and its own local values (`let`). Enum values are written bare (`Business`) when the expected type says which enum they belong to, or qualified (`CustomerKind.Business`). A name that is both data and a part is an error.

**Part properties.** Every part has `visible` and `enabled` (bool). Labels, help texts, headings and buttons have `caption` (text). Inputs have `error` (text: a message shown at the input and in its help text, empty clears it) and `readonly` (bool). Tables and lists have `selected`, the record of the selected row (read-only). Bound inputs are changed through their data: `customer.name = "..."` updates the text box bound to it.

**Statements**, one per line, blocks by indentation:

| Statement | Meaning |
|---|---|
| `let name = expr` | a local value; its type is the expression's |
| `target = expr` | set a local, a data field (`customer.credit_limit`) or a part property (`save.enabled`) |
| `if expr` / `else if expr` / `else` | the condition must be bool |
| `return` | leave the handler |
| `message [info\|warning\|error] expr` | show a message; the default is info |
| `open Window [with name = expr, ...]` | open a window, filling its data by name |
| `close` | close this window |
| `save expr` / `delete expr` | store or remove a record in its data source (section 1.3) |
| `reload data` | read the window's data again from its source: a list loads all its records, a record loads again by its id (a new record stays as it is) |

**Window and app events.** Besides its parts, a window has events of its own, written as setting lines in the `page` block, and the app has one in the `app` block:

| Event | When |
|---|---|
| `on open: handler` | the window has opened and its data is loaded |
| `on close: handler` | the window is about to close |
| `on return: handler` | a window this one opened has closed, and this one is on top again |
| `on start: handler` (app) | the application starts, before its main window opens |

```clarity
page CustomerBrowse: window
    data customers: list<Customer>
    on return: windowReturned
    table name grid at 1,1 span 12 rows 8 bind customers columns name, credit_limit

code CustomerBrowse
    handler windowReturned
        reload customers
```

This is how a list refreshes after a form that edits one of its records: in code, not in a setting. The browse template writes exactly this, so a new browse window refreshes from the start; delete the line or the handler to change it. In the designer, the window section of the properties lists the events with Wire, Code and a remove button, the same as for parts; the app's events are in the same place when the app is selected. A window or app handler counts as wired.

**Expressions:** literals (`12`, `9.95`, `"text"`, `true`, `false`), paths (`customer.name`, `orders.selected.total`), enum values, `not`, unary `-`, `* /`, `+ -` (`+` also joins text), comparisons `== != < <= > >=`, `and`, `or`, `x in [a, b]`, parentheses and `count(list)`.

**Types** come from the dictionary: `text` (also email and url), `integer` (also id), `decimal` (also money), `bool`, `date`, `datetime`, an enum, a record of an entity and a list of records. Integers widen to decimals; nothing else converts implicitly. The checker reports unknown names, wrong types, assignments to read-only properties, and handlers that are wired but missing (an error) or written but not wired (a warning).

Later, with D1 and beyond: queries (`find`, `where`).

**Design as text.** The designer and the text are two views of the same declaration. The Text button shows a window's `page` block (or the `app` block) in the slide-out panel; while it is typed the backend parses and checks it, and the designer draws the window from the text without saving it. Save writes it as one edit; until then the canvas does not take edits, because its positions belong to the saved file. Every canvas edit shows up in the text at once.

### 5.10 Functions and actions (2026-10-16)

Modules hold logic that is not tied to one window: functions and actions, written in Clarity and callable from any handler and from each other through `use`.

```clarity
function within_credit(order: Order) -> bool
    return order.total <= order.customer.credit_limit

function order_total(order: Order) -> money
    ensure result >= 0
    let total: money = 0
    for line in order.lines
        total = total + line.amount
    return total

action approve(order: Order)
    require order.status == Pending
    require within_credit(order)
    effects db.write
    order.status = Approved
    save order
```

- **A function** has no effects. It may not `save` or `delete`, may not call actions, and may not show messages or open windows. It is safe to call anywhere, including in conditions.
- **An action** may change data, and says so with `effects`: `db.write` for `save` and `delete`, `db.read` for reading. An action that calls another action declares that action's effects too. Messages and windows stay in handlers: the backend logic does not talk to the screen.
- **Contracts.** `require` lines check the arguments before the body runs; `ensure` lines check `result` after it. Both must be bool. They become runtime checks and generated tests.
- **Signatures.** Parameters and results are typed: `text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, an entity (a record), an enum, or `list<Entity>`. A function with a result returns a value on every path; the checker says so when one is missing.
- **Statements** are those of handlers (section 5.9), plus `return value`, `for item in list`, a call on its own line (`approve(order)`), and typed locals (`let total: money = 0`).
- **In the designer** a module shows its functions and actions; each opens in the same editor as the code behind, checked while it is typed. New function and New action add a starter.

Superseded by this: section 5.3, where an action only declares its signature and its body is written in the target language.

### 5.11 Engines (2026-10-20)

An engine is a processing module that every stack runs the same way: the logic of a game, document processing (parsing, layout, conversion, rendering a page to shapes), a pricing or scheduling engine, a solver. It is compiled once to WebAssembly, and each generated application hosts that one file, so the engine behaves identically on every stack by construction, not by translation.

**The engine computes; the application does all input and output.** An engine has no database, no files, no network, no windows and no clock of the system. The application reads the records and files, hands them to the engine as values, and stores, writes or shows what the engine gives back. This keeps the sandbox absolute, makes an engine testable on its own with plain values, and keeps every effect where Clarity already checks it: in the handlers and actions of the application (section 5.10, effects in section 9.3).

```clarity
engine Layout from "layout.wasm"
    function paginate(text: text, page: PageSize) -> list<PageBox>
    function measure(text: text, style: TextStyle) -> Extent
    uses clock

record PageSize
    width: decimal
    height: decimal
```

```clarity
action export_report(report: Report)
    effects db.read, files.write
    let pages = Layout.paginate(report.body, PageSize(210, 297))
    for page in pages
        write render_page(page) to exports "report-{page.number}.svg"
```

**What an engine is.**

- **A declaration and a file.** `engine Name from "file.wasm"` declares its functions with Clarity types; the file lives in the project's `engines` folder, beside `resources`, and goes into every generated application like the resources do.
- **Written in Clarity, or brought along.** An engine written in Clarity (`engine Board` with what it keeps and its functions below it, no `from`) is generated by each stack as code of its own, like a function (N2, below). An engine brought along is any plain WebAssembly module that speaks the engine interface and imports nothing but the services it uses: from Rust, C or C++ (clang without a C library), Zig (`wasm32-freestanding`) or AssemblyScript.
- **The engine interface.** The module exports its `memory`, `clarity_alloc(len) -> place`, `clarity_free(place, len)`, and each function under its own name as `(place, len) -> i64`. The host writes the arguments as a JSON array into room from `clarity_alloc`; the function answers `{"value": ...}` or `{"error": "why"}` and gives its place and length packed in the i64 (place in the high 32 bits). Values are Clarity's JSON as everywhere else. The interface is JSON over plain memory rather than the WebAssembly component model and WIT, because the hosts the stacks need (wazero, wasm3, Wasmtime for .NET) do not run components; JSON is also what every runtime already reads and writes for the screen.
- **Values cross, never references.** Parameters and results are plain values (`text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, `duration`), enums, `bytes` (the content of a file), lists, and records. Entities cross as copies: an engine can read an `Order`, but a changed `Order` it returns is only saved when the application saves it. Records that belong to no table (`record PageSize`) are declared for engines and code alike. A decimal and money cross as exact text, as they travel everywhere else in Clarity.
- **Its imports are services, never data.** What an engine may call is declared with `uses`, and the host gives it exactly that and nothing else: `clock` (the time, which the application can fix for tests), `random` (seeded per call, so a run can be repeated), `log` (lines into the application's log), and `progress` (how far a long call is, which the screen can show). There is no import for the database, files or network, and none will be added: an engine that needs data gets it as a parameter.
- **State, when it needs some.** By default an engine is stateless: each call starts fresh. `state per window` keeps one instance for each window that uses it, made at the window's first call and dropped when it closes (a game's world, a document being edited); `state per app` keeps one for the application. State lives only inside the engine's memory; what must be kept is returned and saved by the application.
- **Hooks.** Besides functions it is called with, an engine can export hooks the application calls at set moments: `on tick(ms)` for an engine beside a canvas, or `on open` when an instance is made. There is no `on close`: an engine can neither save nor show anything, so a window that must keep what an engine holds asks it in its own `on close` handler. The application never hands the engine a callback into its own code, so an engine cannot reach the application's data by calling back.

**Where it runs.** On the backend by default, where handlers and actions call it like a function. Each stack hosts it with a WebAssembly runtime: Rust with wasmi (pure Rust, small), Go with wazero (pure Go), C and C++ with wasm3 (portable C, copied in like the webview), C# with Wasmtime for .NET. An engine declared `beside the screen` runs in the renderer instead, for work where a round trip per call is too slow, such as a game loop at 60 frames a second: the web renderer (the Rust, Go, C and C++ stacks) runs it in the browser engine's own WebAssembly, Avalonia in Wasmtime in the same process. An engine beside the screen draws through the canvas's shapes (section 5.7, `conformance/drawing.json`) and takes its input events; what it reports back to the window (a score, the end of a game) travels as a value to a handler.

**Limits.** Every call runs with a time and a memory limit, set per engine (`limit 2 seconds, 256 MB`) with defaults, so an engine that loops or grows without end fails that call and never hangs or exhausts the application. A failure (a trap, a limit, an engine's own `refuse`) arrives as an error the application shows like a refusal.

**Checked twice.** The checker types every call to an engine against its declaration. The designer reads the `.wasm` file's imports and exports when the project loads. When the application starts, the host checks it again: that it exports the interface and every declared function, and imports nothing beyond its `uses`. A file that does not match is refused at start, never at the first call.

**Alike on every stack.** WebAssembly computes the same on every machine (integers exactly; floating point too, apart from the bits of a NaN), so one shared test engine and `conformance/engines.json` (calls, results, limits, refused imports) run against every host: the four backends, the web renderer and Avalonia.

Built (N1, 2026-10-20): stateless engines brought along, on the backend of all five stacks. `engine Name from "file.wasm"` with `function` lines, `Engine.function(args)` calls typed by the checker, the `engines` folder of the module folder read by `clarity_spec::engines`, and a host per runtime (Rust: the crate's `engines` feature; Go: package `engine`; C#: the package `Clarity.Runtime.Engines`; C and C++: `engine/` and `third_party/wasm3`, copied only into a project with an engine). Each call runs on a fresh instance, and on Rust on its own thread with a large stack, since wasmi's debug builds need one. wasm3 v0.5.0 carries one patch (`call_indirect` reads its table index as an unsigned LEB, as current compilers write it). The test engine (`conformance/engines/test`, Rust) and `conformance/engines.json` run against every host, and a build test per stack generates `conformance/engines/shop.clarity` and calls every function. Not yet: state, hooks, `bytes`, `record` declarations, engines written in Clarity and engines beside the screen (N2, N4, N5).

Built (N3, 2026-10-20): `uses clock, random, log` and `limit 2 seconds, 64 MB` (defaults 5 seconds and 256 MB, `clarity_spec::engines`). The services are functions of module `clarity`: `clock() -> i64` (milliseconds since 1970 UTC), `random() -> i64` (splitmix64, seeded afresh for every call, so the same seed gives the same numbers on every host) and `log(place, len)` (`engine <Name>: <text>` to the standard error). A host refuses any other import, a service the declaration does not use, or one of another type; tests fix the clock and the seed and keep the log lines (`conformance/engines.json`, `services` and `served`). The time limit is the host's own, so every host stops a call alike and says `it ran longer than 2 seconds`: wasmi runs a call in slices of fuel with a look at the clock between them (short slices in debug builds, whose interpreter takes stack as it runs), wazero stops at a context deadline, Wasmtime for .NET at an epoch a timer moves, and wasm3 asks a yield hook at every call and every turn of a loop (a second patch: `m3_YieldHook`, and loops that yield). The memory limit caps growth on every host (store limits in wasmi and Wasmtime, memory pages in wazero, the runtime's maximum pages in wasm3); an engine that then fails says so in the host's words, since Wasmtime for .NET cannot tell the host that it refused a growth. Not built: `progress`, which needs the screen to update while a handler runs, on every stack.

Built (2026-10-21): i18n in code and engines. A text with slots is an ordinary text value in code, always translated. Text that goes to the screen (messages, refusals, captions, errors, and text values a handler sets, which a reply carries as `shown`) travels as written with its slot values and the screen translates and fills it; text elsewhere, such as a function's result, the backend translates (`i18n` in each runtime, loaded from the model's translations, the language set by the screen with `set_language`) and fills plainly (`conformance/translations.json`, `code`). An engine lists its texts with `says`, which join the project's texts; a text an engine function returns is translated where code calls it. The service `language(place, room) -> len` gives an engine the language the screen picked (`conformance/engines.json`); `clarity-engine` 0.2 has `language()`.

Built (N4, 2026-10-20): `state per app`, `state per window` and `on open` on all five stacks. Hosts load an engine from a declaration (functions, `uses`, limits, state, `on open`) and keep instances by window number, one at 0 for state per app, behind a lock, so the calls of an engine with state take turns. `on open` calls the export `clarity_open` with the engine interface and no arguments on every new instance; when it refuses or answers badly, the call it came before fails with `its on open refused: <why>` or `its on open gave no answer`. A failed call or one past its limit drops the instance; a refusal keeps it (`conformance/engines.json`, `kept` and `windows`). Every open window now has a number: `instance` in the window state that travels to the screen and back on the web stacks (given out where the state is made, kept by every reply), and `Instance` on the ViewModel in C#. Each core runtime keeps a registry of what listens for closed windows; the shell reports a window after its own `on close` has run (a failing `on close` keeps the window open, so its engine instance stays too), and an engine with state per window listens from the moment it loads. Rust's generated `main.rs` calls its engines' `close_window` itself. The checker allows a call to an engine with state per window only in a window's handlers. Not covered: a desktop window that opened `in new window` and is closed by its window manager reports none of its windows, so their instances stay until the application ends. Also built: `bytes`, the contents of a file as a value (`read bytes of f`, `write b to store "path"`, `b.size`), which crosses to an engine as base64 text: natively in Go and C#, through the runtime's base64 in Rust, C and C++ (`conformance/files.json`, `bytes`; the records example backs up its photos with it). Bytes are no window data.

Engines in the examples (2026-10-20): `runtimes/rust-engine` (the `clarity-engine` crate) is what an engine written in Rust stands on (the interface as `interface!()`, functions with typed tuple arguments as `function!`, `on_open!`, the services, `Bytes`), so an engine holds only its own logic; N2 can generate against it. Each example has an engine with its source in `<example>/engines/<name>/`, built by `examples/build-engines.sh`: Pricing (shop, order lines priced in a data event), Images (records, picture sizes from `bytes`), Gauge (dial, called from a draw handler) and Board (the new noughts example, `state per window`, `on open`, `random`). The dial and noughts examples now have build tests on every stack too; they found that a project without entities wrote a Go `schema.go` gofmt would change.

Not an engine: code that needs the database or files while it works. That is an action, which Clarity already compiles to every stack and checks for its effects; an action can call an engine for the processing in between.

### 5.12 Engines beside the screen, engines in Clarity, and progress (2026-10-20, a design)

Three pieces that belong together, since each needs the screen to change while code runs: progress of a long call on the backend (P), engines written in Clarity (N2), and engines that run beside the screen, in the renderer, at the pace of the screen (N5). Engines written in Clarity are built (N2a); the rest is not yet. The decisions still open are marked **Open**.

#### Progress of a long call

An engine that works long (a document of a thousand pages, a schedule for a year) says how far it is: `uses progress`, and the engine calls `clarity.progress(done, total)`. While the call runs, the window shows a progress bar with a **Stop** button, in the place its messages show.

- **The way to the screen.** Today a handler runs and then replies; nothing reaches the screen in between. Progress needs a message while the handler runs: each shell already has a way to send one, used for nothing else yet: Tauri's `emit`, Wails' events, the webview's `eval` from the shell's own thread (C and C++), and Avalonia's `Dispatcher.UIThread.Post`. The runtime gives the host a progress sink per call; the shell sends `progress {window, done, total}` to the renderer, at most ten times a second.
- **Stop.** The renderer sends `stop {window}`; the host stops the engine's call at its next check, the same place the time limit stops it (a fuel slice in wasmi, the context in wazero, the epoch in Wasmtime, the yield hook in wasm3). The call fails with `engine X failed in f: it was stopped`, which the handler shows as any error; an engine with state drops its instance, as after any failure.
- **The window stays busy but painted.** While a handler runs, the window takes no other click (as today), shows the bar and keeps painting. For that the handler leaves the screen's thread: Wails already calls a bound method on a goroutine; Tauri's commands become `async`; the C and C++ webview shells answer a bound call later from a thread of their own (`webview_return`); Avalonia runs the handler as a task and posts its reply back.
- **Also for code.** A handler's own long loop can show progress too: `progress n of total` as a statement in handlers and actions, so a long import shows the same bar. **Open:** whether this statement comes with the engine service or later.

#### Engines written in Clarity (N2)

An engine without `from` is written in Clarity, in the module that declares it:

```
engine Board
    uses random
    state per window
    keep squares: list<integer> = [0, 0, 0, 0, 0, 0, 0, 0, 0]

    function play(square: integer) -> text
        if squares[square] != 0
            refuse @thatSquareIsTaken
        squares[square] = 1
        return @yourMove
```

- **Transpiled per stack (decided 2026-10-21).** Each generator writes the engine as code of its own, beside the module's functions: a struct of what it keeps, the kept values of each instance (a window's number, or 0 for the app) behind a lock, and per function an entry that takes the instance's values and an inner function that works on them. No WebAssembly and no Rust toolchain are needed for it; the WebAssembly route stays for engines brought along.
- **Keeps.** `keep name: type = value` is what an instance keeps between calls. State works as for any engine: none gives every call the values as written, `state per app` one set, `state per window` one set per open window, dropped when it closes (each runtime's window-closed listeners).
- **A call changes nothing when it fails.** The entry works on a copy and keeps it only when the call succeeds. In C, kept values live on the heap and a call's copy in the scratch arena (`clarity_integers_keep` and its kin).
- **What its code may do.** What a function may do, plus its keeps by name, its own functions by name, and `random(n)` with `uses random`. No database, files, windows or messages. Its texts are keys, shown in the person's language.
- **Lists of values.** `list<text|integer|decimal|bool>` came with it, for code and keeps alike: literals `[1, 2, 3]`, items counted from 1 (`squares[3]`, read and set), `for` and `count`. A list is a value: reading one gives a copy. Every runtime has `at`/`set` and splitmix64 `random`, against `conformance/lists.json`.

Built (N2a, 2026-10-21) on all five stacks, with the noughts example's Board rewritten in Clarity; a build test per stack plays it (`an_engine_written_in_clarity_runs*`). Not yet: `record` declarations and `while`.

#### Engines beside the screen (N5)

An engine declared `beside the screen` runs where the canvas is drawn, so a game or an animation runs at the pace of the screen without a trip to the backend per frame:

```
engine Asteroids from "asteroids.wasm"
    beside the screen
    uses random
    tells finished(score: integer)
    hears start(level: integer)

page Play: window
    canvas name sky at 1,1 span 12 rows 10 units 160 x 100 runs Asteroids on finished: gameOver
```

- **One instance per canvas.** The engine starts when its window opens (`on open`) and goes when it closes; it keeps its state while it runs, like state per window.
- **The loop.** At every frame the renderer calls the engine's `clarity_tick(ms)` with the time since the last frame, then `clarity_draw()`, which answers the canvas's shapes in its units, the same shapes as a draw handler's (`conformance/drawing.json`), so both renderers already know how to show them. Pointer and keys on the canvas go to `clarity_pointer(kind, x, y)` in canvas units and `clarity_key(kind, key)`.
- **Back to the window: tells.** What the engine has to say to the application (the game is over, with its score) it tells: its answer to a tick or an input carries `{"tell": "finished", "values": [1200]}`, and the window's handler wired to that tell runs on the backend as any handler does, with the values as its parameters (`handler gameOver(score: integer)`).
- **From the window: hears.** A handler sends the engine an order with `sky.start(2)`; the reply carries it to the renderer, which calls the engine's export of that name.
- **Where it runs.** On the web stacks in a Web Worker of its own, with the browser engine's WebAssembly: the worker draws nothing itself, it answers shapes, and the page shows them. A worker can be stopped from outside, so the limits hold here too: a frame that takes longer than its limit ends the engine, and the canvas shows the failure. On Avalonia in Wasmtime in the same process, on a timer at the screen's rate, stopped by its epoch. (Decided 2026-10-20: a worker, not the page's own thread, where an engine that loops would freeze the window.)
- **Written in Clarity too.** An engine beside the screen can be a Clarity engine (N2): `on tick(ms)`, `on draw` (with the same drawing statements as a draw handler: `sky.circle(...)`), `on pointer(kind, x, y)`, `on key(kind, key)`, its `tell` statements, and functions for what it hears. That is where N2 earns its place: work that must run beside the screen, written in the same language as the rest.
- **Services beside the screen** are the renderer's: the clock and random from the page or the process, the log into the application's log through the shell. Progress has no meaning there.

#### In order

1. **P**: progress and Stop for engine calls on the backend, on all five stacks and both renderers, with `progress n of total` if that is chosen.
2. **N2a**: `record` declarations and engines written in Clarity, on the backend, built by Clarity into the engines folder.
3. **N5a**: engines beside the screen, brought along: the frame loop, input, tells and hears, in a Web Worker and in Avalonia.
4. **N5b**: Clarity engines beside the screen: `on tick`, `on draw`, `on pointer`, `on key`, `tell`.

### 5.13 Diagrams (2026-10-22, a design)

Clarity draws and designs the model as diagrams in Mermaid: ERDs and class diagrams first, then state and sequence diagrams, then activity and architecture. They are a way of designing the application, as the window designer is, for the person and for the agent alike. Agreed with the user on 2026-10-22; nothing is built yet. The decisions still open are marked **Open**.

- **The data dictionary stays the one source of truth.** A diagram is a view of the project or a proposal to change it, never a second model that drifts from it.
- **In the IDE only.** Diagrams are design: generated applications get no diagram part and carry no diagram files, so this needs no work per stack. Showing a diagram inside an application (a mermaid block in a Markdown resource, drawn when the application is generated, since Avalonia runs no JavaScript) would be a separate row.

#### Views and written diagrams

- **Written from the project** (decided 2026-10-22, replacing views that were not files). An ERD or class diagram is written from the project when it is made (New diagram: starts from the project, or with a small example) and again with **From project** in its header: of everything (an ERD: every entity, its fields, refs and lists; a class diagram: entities, enums, engines with their functions, and a `<<module>>` class per module with its functions and actions, each in its module's namespace), or of only the boxes it draws, for a partial diagram. Where the boxes stand is kept, and it is one undo step. So every diagram is a file, edited the same way; From project brings the project into the diagram, Apply the diagram into the project. Later rows add a state diagram per enum, a sequence diagram per handler, an activity diagram per handler or function, and the architecture (modules, engines, data sources, the stack), written the same way.
- **Written diagrams** are `.mermaid` files in the project's `diagrams` folder, beside `resources`, `engines` and `translations`, written by the person or by the agent. Unlike resources they are not carried into the application. A written diagram shows how it differs from the project and proposes edits through **Apply**; it changes nothing by itself.

#### The project part

- **Project tree.** A Diagrams section lists the views first, then the written files, with the same Open, Rename and Delete menu as other parts.
- **New diagram.** A dialog built like New resource asks for a name, a kind (ERD or Class in DG1) and optionally a `.mermaid` file to copy in, and writes a starter of that kind.
- **Kinds in one place.** `clarity_spec::diagrams::kinds()` lists the kinds with their Mermaid header (`erDiagram`, `classDiagram`, `stateDiagram-v2`, `sequenceDiagram`, `flowchart`, `architecture-beta`), so the tree, the dialog and the agent read the same list. Other Mermaid kinds (gantt, mindmap, pie) may be written and are drawn, but map to nothing.

#### What a diagram means

Types are written in Clarity's own words (`text(80)`, `money`, `email`, an enum's name); common foreign words are read as Clarity's:

| Written | Read as |
|---|---|
| `string`, `varchar`, `varchar(n)` | `text`, `text(n)` |
| `int`, `long`, `integer` | `integer` |
| `float`, `double`, `number` | `decimal` |
| `bool`, `boolean` | `bool` |
| `timestamp` | `datetime` |

ERD (`erDiagram`):

| Mermaid | Clarity |
|---|---|
| an entity block | an entity |
| an attribute `type name` | a field; Clarity adds `id` itself, so a `PK` attribute named `id` is left out and another `PK` is a warning |
| `UK` | `unique` |
| the attribute's comment, `"required, computed"` | modifiers; also the full type where Mermaid's type word cannot hold it (`"decimal(10, 2)"`) |
| `Order }o--\|\| Customer : places` | `customer: ref Customer` on the many side |
| an identifying line (`--`) from the one side to the many side | also `lines: list<OrderLine>, owned` on the one side |
| a non-identifying line (`..`) | also a plain `list<...>` on the one side, when the one side lists them |

Class diagram (`classDiagram`):

| Mermaid | Clarity |
|---|---|
| a class with attributes `+name: text` | an entity |
| `<<enumeration>>` with its values | an enum |
| `<<engine>>` with methods `play(square: integer) text` | an engine written in Clarity, with stubs of its functions |
| `<<module>>` with methods | the module's functions and actions |
| `A "*" --> "1" B` | `b: ref B` on A |
| composition `*--` | an owned list |
| aggregation `o--` | a plain list |
| uses `music ..> Images`, from a `<<module>>` or `<<engine>>` | nothing: what its code uses (engines called, entities taken, found or made), drawn only; From project draws it from the code |

In both kinds the count at the end referred to says whether the ref is required: exactly one (`||`, `"1"`) makes it required, zero or one (`o|`, `"0..1"`) leaves it optional.

Methods on an entity's class are a problem with a fix: functions belong to a module, so they go on its `<<module>>` class.

#### The diagram designer

A diagram opens like a window does, with **Design** and **Text** tabs:

- **Toolbox** on the left: Entity, Enum, Engine and Relationship in DG1; State, Step, Lifeline and Message come with their kinds.
- **Canvas** in the middle, Clarity's own for the kinds it designs (decided 2026-10-22, the ERD first): boxes stand where the person puts them, and each place is kept in the `.mermaid` file as a `%% clarity: Customer at 120,80` line, a comment Mermaid ignores, so the file stays plain Mermaid and opens anywhere Mermaid does. Relations show how many at each end (a bar for one, a ring for zero, a crow's foot for many), solid when owned. Ctrl+wheel zooms, dragging the background pans. Kinds Clarity does not design yet are drawn by mermaid.js, bundled into the app so it works offline and loaded only when such a diagram opens, in an isolated frame with Ctrl+wheel zoom and drag-to-pan in the IDE theme's colors.
- **Properties** on the right: for an entity, its fields as rows (name, type, required, unique, label key); for a relationship, its two ends, how many on each side, owned or not.
- **On the canvas:** drop an entity from the toolbox; drag from one box to another to draw a relationship; double-click a name to rename it; right-click for Add field and Delete; a click opens what a box stands for (the entity, the field, the enum, the engine, the function), as the Problems list jumps to a line.
- **Rename and Delete.** A written diagram is renamed in its header (the name, or the Rename button) or from the tree's menu, and deleted from the tree's menu; each is one undo step.
- **Places.** A box without a place gets the next cell of a grid beside the placed ones; the first edit on the canvas pins every box where it is shown, so nothing moves by itself.
- **Class diagrams** (built 2026-10-22) use the same canvas: Entity, Enum, Engine and Module boxes (the stereotype above the name, the title tinted by kind), and Association, Composition and Aggregation lines (an open arrow at what is referred to, a filled or hollow diamond at the whole, the multiplicity beside each end). Properties edit a box's name, its module (written as its namespace) and its members: fields, values, or functions as signatures (`play(square: integer) -> text`). Each kind turns its model into boxes and lines for the one canvas, so later kinds add an adapter, not a canvas.
- **Modules as frames** (2026-10-22). On a class diagram each module is a frame around its boxes, its name above the top left; the `<<module>>` box of that name stands inside it. Dragging the name moves the frame with everything in it. Boxes without a place are laid out a module at a time, each starting a row of its own, so frames do not overlap.
- **Edits.** On a view, every action is an edit of the dictionary. On a written diagram, it is an edit of the `.mermaid` text through `clarity_spec::diagrams::edit`, which, like `edit.rs`, rewrites only the lines it is about. Either way it is one undo step, and the Text tab follows.
- **Text.** The Text tab is the SourceEditor with `language="mermaid"`: colors and positioned problems from the same parser, as Clarity text has them.

#### Differences and Apply

- **In sight before anything changes.** A written diagram is drawn with what it would add (green), change (amber), and what the project has but the diagram leaves out (grey), through `classDef` and `style` lines Clarity adds to a copy of the text before drawing.
- **Apply** lists the proposed edits, each ticked: new entities, fields, refs and lists, enums, engines and changed types or modifiers. What the diagram leaves out is offered as a removal that starts unticked; Clarity never removes anything unasked. Only what the diagram draws is compared: a field the project has on an entity the diagram draws with fields, or a value of an enum it draws with values; an entity drawn without fields, or one the diagram does not name, is left alone. Each new entity names the module it goes to, by default the module of an entity it relates to. The ticked edits apply as one undo step; every label they need gets a key in every language file (as `apply_in` does today), and the migration step is offered as after any change of entities.

#### Checking

A parser in `clarity_spec::diagrams` reads the subset Clarity maps, `erDiagram` and `classDiagram` in DG1 and the other kinds with their rows, and gives positioned problems in the checker's words: an unknown type, a relationship to an entity that is not there, a name that is not snake_case or PascalCase. A diagram's problems show in the Problems list with the project's.

#### Names and texts

Names in a diagram are names, not texts people read. What Apply or the designer creates gets keys for its labels, proposed as the entity editor proposes them (`customerNameLabel`), with a default value from the name, in every language file.

#### The agent

- **Tools:** `diagram_view` (a generated view: the data model, the classes, and later an enum's states or a handler's sequence), `write_diagram` (a `.mermaid` file in `diagrams`), `diagram_proposals` (what a written diagram would change) and `apply_diagram` (apply the proposals it names).
- **Prompt:** diagrams propose and the dictionary decides. Asked for "an order system with customers and invoices", the agent sketches the ERD as a written diagram first; the person adjusts it in the designer; Apply makes the entities, and the migration step and windows follow.

#### Steps between enum values (DG2, decided 2026-10-22)

A state diagram on an enum says which values may follow which, and that becomes part of the language:

```clarity
enum OrderStatus
    Pending label @orderStatusPending
    Approved label @orderStatusApproved
    Shipped label @orderStatusShipped
    Cancelled label @orderStatusCancelled
    steps
        Pending -> Approved, Cancelled
        Approved -> Shipped, Cancelled
```

- **Meaning.** A stored field of the enum may only change along a step; a new record may start at any value unless `starts` says which. Saving a change that is not a step is refused with a keyed message, on every stack and both renderers, with conformance cases.
- **The checker** knows the steps too: a step names values of the enum, and a value no step reaches is a warning.
- **Open:** the exact syntax (`steps` block, `starts`), and whether a step can carry a condition or the name of the handler that takes it.

#### In order

1. **DG1**: the `diagrams` part, the parser and colors for ERD and class diagrams, the Data model and Classes views, the designer with toolbox, overlay and properties, written diagrams with differences and Apply, and the agent's tools.
2. **DG2**: state diagrams with `steps` on enums, and sequence diagrams per handler (a view first; handler stubs from a written one later).
3. **DG3**: activity diagrams per handler or function and the architecture, as views.

### 5.14 Asking the person: dialogs (2026-10-22, built)

Decided with the user on 2026-10-22. A handler can ask the person something: a yes or no, a text, a number, a value of an enum, a record, or the result of a dialog window of the application. Every stack shows it in Clarity's own dialog, never one of the system or the browser (the IDE's are `app/src/lib/dialogs.ts`, `DialogHost.svelte` and `Modal.svelte`).

**When.** A dialog shows after the handler that asks has finished; the handler never waits for the answer. The answer runs another handler, as an engine's tell does (5.12). So handlers keep running and replying as they do (5.9): the dialog is one more thing in the reply, as a message or a window to open is, and nothing on the backend stays blocked while the person thinks. Asking in the middle of a handler and going on with the answer was weighed and left out: every stack would have to hold a running handler and resume it, which a closed window or an answer that never comes makes fragile. Should the split into two handlers become a burden, the compiler can later write such code as two handlers underneath; the language stays as below.

**The statement.** `ask` is a statement of handlers, ending in `then` and the handler that takes the answer:

```clarity
handler deleteClicked
    ask @deleteAlbum(title = album.title) yes @delete no @keep then deleteConfirmed

handler deleteConfirmed
    delete album
    close

handler lendClicked
    ask @lendTo for Friend then lend(album)

handler lend(album: Album, friend: Friend)
    let loan = new Loan
    loan.album = album
    loan.friend = friend
    save loan
```

- **What is asked decides the dialog**, as the dictionary decides a form:
  - `ask @question yes @yes no @no then h`: a question with two buttons; `h` runs on yes and takes no answer. `yes` and `no` may be left out (Clarity's OK and Cancel, translated).
  - `ask @question for text then h`, and for `integer`, `decimal`, `money`, `date`, `datetime` or `duration`: a typed input; `default expr` fills it first. The checks of the type hold, as in a form.
  - `ask @question for Enum then h`: the enum's values, each with its label.
  - `ask @question for Entity then h`: a searchable list of the entity's records; `from expr` offers the records of a list instead, as `from find f in Friend where f.active`.
  - `ask Window with name = expr, ... then h`: a window of the application declared as a dialog (U1d) gives what it closes with: `close with expr`. Its type is the type of the answer. Built 2026-10-22, with the type declared on the page: `page PickFriend: dialog gives Friend`.
- **The answer** is the last parameter of the handler after `then`; values written in the call come first (`then lend(album)`), which is how what the asking handler knew reaches the answering one. An answering handler may take any type an answer has, records and dates included. The checker holds the types: the answer's type must be the last parameter's.
- **Cancel** (the no button, Escape, a click beside the dialog, or the window closing) runs nothing.
- **Texts are keys**, with slots filled by name, as everywhere (`@deleteAlbum(title = album.title)`).
- **Order.** The statements after `ask` still run; the dialog shows when the handler has finished, with whatever else it changed. A handler that asks twice shows the questions one after the other, each with its own answering handler. A handler that asks and closes its window is an error: the answer would have nowhere to go.
- **Where.** Only handlers ask. Actions and functions cannot: a function has no effects, and an action may run where nobody is at the screen (a data event, another action).
- **`message`** stays a statement as it is (decided 2026-10-22): an info message stays a toast that fades, so "Saved" needs no click; `message warning` and `message error` show in Clarity's dialog with OK.

**Every stack.** The reply of a handler gets the questions to ask, each with its answering handler and the values written for it; the renderer (and Avalonia's views) show them with Clarity's dialog, and an answer comes back as an ordinary event that runs that handler. The web renderer's dialog lives in `lib/ui` (copied into each application by `clarity-gen`); Avalonia's in Clarity.Avalonia, so a new package version and its NuGet upload. Conformance tests ask on every stack and answer, cancel, and check that the right handler ran with the right values.

**Progress is apart.** A dialog with a progress bar and Stop (5.12, row P) is not a question: the handler sends progress one way while it runs, and Stop is a flag it reads between steps. It needs the way to the screen of 5.12, not this design, and does not wait for anything either.

### 5.15 Wizards: the steps part (2026-10-22, built)

Decided with the user on 2026-10-22 (U1d). A wizard is a container part, `steps`, as `tabs` is one: it can stand in any window or dialog, so a wizard dialog gives its result as any dialog does (5.14).

```clarity
page NewOrder: dialog gives Order
    data order: Order
    steps name wizard at 1,1 span 12 rows 10 on next: checkStep on finish: done
        step @customerStep
            dropdown at 1,1 span 6 bind order.customer
        step @confirmStep
            text at 1,1 span 12 bind summary

code NewOrder
    handler checkStep
        if wizard.step == 1 and order.customer == none
            refuse @pickACustomer

    handler done
        save order
        close with order
```

- **One step at a time.** The part shows the titles of its steps above, the current one marked, and the parts of the current step below. Its own buttons, in Clarity's words (`back`, `next`, `finish`, translated as `ok` and `cancel` are), go below: Back from the second step on, Next before the last step, Finish on the last.
- **Next** runs the handler of `on next` with the step the person is on (`wizard.step`), then moves one step on. A `refuse` in it keeps the person on the step and says why, as a refusal anywhere does; a handler that sets `wizard.step` itself decides where the wizard goes. Without `on next`, Next just moves on.
- **Back** goes one step back and runs nothing: what was entered stays.
- **Finish** runs the handler of `on finish`, which saves, closes the dialog with its value, or does what the wizard is for.
- **`wizard.step`**, from 1, is the part's property: code reads it, and sets it to skip or go back to a step.
- **Rules.** `steps` needs at least two `step` blocks. A step's parts are placed in the step's own grid, as a tab's are.
- **Every stack.** The web renderer draws it in `lib/ui` (copied into each application) and Avalonia in Clarity.Avalonia, each in its own idiom; the step lives in the part's state, as a ribbon's open tab does. Conformance cases run the same steps on both.

### 5.16 Context menus and popovers (2026-10-22, built)

Decided with the user on 2026-10-22 (U1d), each the first of two shapes offered.

```clarity
page AlbumBrowse: window
    data albums: list<Album>
    data onlyVinyl: bool
    table name grid at 1,1 span 12 rows 6 bind albums columns title
        menu
            item @openItem on click: openClicked
            separator
            item @deleteItem name deleteItem on click: deleteClicked
    button @filterButton name filterButton at 1,7 span 2 on click -> filters
    popover name filters for filterButton span 4 rows 2
        checkbox @onlyVinylCaption at 1,1 span 4 bind onlyVinyl on change: refilter
```

- **A context menu** is a `menu` block indented under a part, its items written as the frame's menu items are (`item`, `separator`, nested items, `on click: handler` or `on click -> Target`, `name`, `key`). A right click (or the menu key) on the part opens it at the pointer. On a table or list, the right click chooses the row under the pointer first, as a click does, so the handler finds it as `grid.selected`. Named items hide and disable as any part does. Any part can have one, a container too: its `menu` line stands among its parts.
- **A popover** is a container part, `popover`, with its own grid of parts, as a panel has. It is not placed in the window's grid: it has no `at`, only a size (`span`, `rows`), and `for part` names the part it opens next to. It shares the window's data and handlers. It is closed until opened: a click with `on click -> filters` opens it, and code opens and closes it with `filters.visible`. A click outside it or Escape closes it, and the renderer keeps that in the part's state.
- **Every stack.** The web renderer draws both in `lib` (copied into each application); Avalonia uses its `ContextMenu` and `Popup` controls, bound to the part's state. The backends of the web stacks need nothing new: the menu's handlers run as any handler does, and `visible` is already part state.

### 5.17 Settings: what each person sets for themselves (2026-10-23, built)

Decided with the user on 2026-10-23. An application needs a settings window, not only a menu item that opens a quick choice: the language, the theme, and the application's own preferences, such as whether the toolbar shows or how many rows a page holds. Until now nothing held such values: a window binds records of the database, which is the wrong place for one person's preferences, and only the pickers of the click targets `language` and `theme` could change those two.

```clarity
settings
    language
    theme
    showToolbar: bool, default true, label @showToolbarSetting
    exportFolder: text
    pageSize: integer, min 10, max 200, default 50

page Preferences: dialog
    form name settingsForm at 1,1 span 8 rows 4 bind settings labels 3
        dropdown name languageBox at 4,1 span 4 bind settings.language
        dropdown name themeBox at 4,2 span 4 bind settings.theme
        switch name toolbarSwitch at 4,3 span 4 bind settings.showToolbar
        spinbox name pageSizeBox at 4,4 span 2 bind settings.pageSize
```

As built: the block has no name and windows have `settings` without a `data` line (the sketch shown to the user had `settings Shop` and `data settings: Shop`). The renderer of the web stacks passes the settings to every window it opens, so a window's `on open` sees what was kept.

- **One `settings` block** per project, in any module, without a name. Its lines are fields written as an entity's are (`name: type`, with `default`, `min`, `max`, `label`, `help`, `format`), of the plain types: text, integer, decimal, money, bool, date, datetime, duration, email, url, color and enums. A reference, a list, a picture or an id is no setting. Two lines are Clarity's own and take no type: `language`, the language shown, of the project's languages, and `theme`, the theme shown, of the themes it offers; a project lists them to show them in a settings window and to read and set them in code.
- **Kept per person, on their machine,** never in the project's database: in the folder for application settings of the person's system (`%APPDATA%`, `~/Library/Application Support`, `$XDG_CONFIG_HOME` or `~/.config`), in a folder named after the application, as `settings.json`. A setting that is not kept yet has its `default`, or the empty value of its type; a kept value that no longer fits (a setting removed, an enum value gone) is dropped.
- **Bound like data, everywhere.** Every window has `settings` as data of its own, without a `data` line: a part binds `settings.pageSize`, a form binds `settings`, and code anywhere reads and sets `settings.pageSize`. A change takes effect at once: the person changing an input bound to a setting keeps it, as does a handler setting it, and every open window sees it. Setting `settings.language` or `settings.theme` switches the language or the theme, as the pickers do; a dropdown bound to them offers the project's languages by their own names and the offered themes by their titles.
- **The pickers stay** as a shortcut: a menu item `on click -> language` changes the same `settings.language`.
- **Every stack.** The spec gives the settings as a record type of their own, `Settings`, beside the entities: it has no table, no migrations and no place in diagrams, and no entity can be called `Settings`. The generators write it as they write a record, without its database side, and give every window's data its `settings`. On the web stacks the renderer holds the settings: it reads them from the shell when the application starts, sends them with every window's data to the backend, takes them back from every reply, and has the shell keep them when they changed (the shell commands `read_settings` and `keep_settings`; Rust's comes from the runtime crate, Go's, C's and C++'s from their shells). On Avalonia the runtime's `Settings` keeps them, and the generated view models share one instance.
- **The designer** offers a "Settings window" template that writes a dialog with a form of the declared settings, labeled, as dropping fields writes one.

## 6. Generation

### 6.1 Shape and behavior

The central rule of spec-driven projects:

- **The spec owns the shape:** entities, enums, validation, actions with their signatures and contracts, screens, navigation, modules.
- **Hand-written code owns the behavior:** action bodies, computed fields, and frontend extension widgets, in the backend's or frontend's own language.

Shape can only come into existence through generation. To add a field, an endpoint or a screen, someone has to change the spec. This applies to humans and AI equally, and it is what forces both to work through the spec and its directives.

### 6.2 Neutral model and generators

```text
spec files -> parser -> neutral model -> backend generator (one per language)
                                      -> shell generator   (one per desktop shell)
                                      -> frontend generator (one per framework)
```

Backend and frontend never generate against each other. Both meet at a transport contract derived from the neutral model: a schema of all types and actions. This keeps the number of generators at backends plus frontends, not backends times frontends.

Generators are deterministic: the same spec and generator version always produce byte-identical output.

### 6.3 Output layout

Example for Rust + Tauri + Svelte:

```text
crates/
  model/src/generated/        entities, enums, validation, repository traits
  app/src/generated/          action traits, Tauri commands, wiring
  app/src/actions/            hand-written action implementations
  app/src/computed/           hand-written computed fields
frontend/src/
  generated/                  types, typed API client, screens, routes, i18n keys
  custom/                     hand-written extension widgets
```

Everything under `generated/` is frozen (section 11.5). Everything else is hand-written and subject to the normal rules.

### 6.4 Action implementations

For each action, the backend generator emits a declaration the hand-written code must implement: a trait in Rust, an interface in Go and C#, a pure virtual class or function declaration in a header in C++, a function prototype in C.

```rust
// generated
pub trait ApproveOrder {
    fn approve_order(&self, ctx: &Ctx, order: Order) -> Result<Order, ActionError>;
}
```

```rust
// app/src/actions/approve_order.rs, hand-written
impl ApproveOrder for Actions {
    fn approve_order(&self, ctx: &Ctx, order: Order) -> Result<Order, ActionError> { ... }
}
```

- A missing or mismatched implementation is a compile error. The compiler enforces the spec.
- The generated wiring checks `require` before and `ensure` after every call, and contract property tests are generated from the same contracts (section 15.3).
- `effects` are checked through the effects rule (section 11.1). The context object passed in only exposes the capabilities the action declared (database, file system, network), so in typed languages an undeclared effect usually does not even compile.
- On first generation, Clarity creates an implementation stub in the hand-written directory, with a failing test. After that the file belongs to the developer and is never overwritten. This puts the action straight into the TDD `red` state.

### 6.5 Frontend extension points

The frontend contains no business logic. Validation is generated from the dictionary and repeated on the backend. When a screen needs something the standard widgets cannot do, the spec declares a custom widget:

```clarity
screen ProductForm: form Product
    color: widget ColorPicker
```

The generator emits a typed props definition, and the implementation lives in `frontend/src/custom/ColorPicker.svelte`. A custom widget receives data and emits events; it cannot call the backend directly.

### 6.6 Regeneration

- `clarity gen` regenerates everything from the spec. It never writes outside `generated/` directories, except for creating missing stubs.
- When a spec change alters a signature, the compiler errors in hand-written code show what has to change.
- `clarity gen --verify` (used in CI) regenerates into a temporary directory and fails if the result differs from the tree.

### 6.7 Generated i18n

All user-facing text in the spec (labels, help texts, group titles, messages) is extracted into generated translation files (section 1.5). Keys are always emitted as explicit literal keys in a generated key map, never built by string interpolation, so tools and searches can find every key.

## 7. Stacks

A stack is a backend language, a desktop shell (determined by the backend), a frontend framework and a styling layer.

### 7.1 Backends and shells

| Backend | Desktop shell |
|---|---|
| Rust | Tauri |
| Go | Wails |
| C# | Avalonia, its usual UI (section 1.2), or WebView2 / Photino with the web renderer |
| C++ | webview (webview/webview) or Saucer |
| C | webview (webview/webview) |

### 7.2 Frontends and styling

Frontend generators: Svelte, Vue, React. The designer and the spec use abstract widgets (field, grid, button, tab, group); each frontend generator maps them to its framework's components.

Styling comes from the project's theme (section 1.5): named values and styles that each frontend maps onto its own means, CSS variables for the web renderer and resources for Avalonia.

### 7.3 Transport and the web target

The generated frontend talks to the backend through a generated typed API client with an interchangeable transport:

- **Desktop:** the shell's IPC (Tauri commands, Wails bindings, WebView2 host objects, webview bindings).
- **Web (later):** HTTP. The backend generator adds an HTTP server adapter (axum for Rust, net/http for Go, ASP.NET Core minimal APIs for C#), and the same frontend uses a fetch transport.

Because the action logic lives in hand-written modules behind generated interfaces, adding the web target does not change any hand-written backend code.

### 7.4 Persistence

The backend generator emits repository interfaces for entities and a storage adapter per data source. Data sources, the databases and web APIs they can be, and migrations are described in section 1.3.

### 7.5 Runtime libraries and UI binding

Every backend language gets its own Clarity runtime library, but the widgets are implemented once:

| Layer | Written | Per |
|---|---|---|
| UI renderer: the widget catalog, pages, screens, layout | once, in the frontend runtime | shell (all backends share it) |
| `clarity-runtime` library: binding protocol, page data and change tracking, validation from the dictionary, event and action dispatch, navigation | by hand, versioned with the transport contract | backend language (Rust, Go, C#, C++, C) |
| Generated code: typed records for entities, typed page data, action interfaces, transport wiring | by the backend generator | project and language |
| Behavior: action bodies, computed fields, renderers | by humans and AI | project |

The renderer and the runtime libraries talk through one binding protocol, derived from the neutral model: the renderer asks for a page's data, reports edits and clicks, and the backend answers with updated data, validation results or a navigation. Because every backend speaks the same protocol, a page designed once runs unchanged on any supported backend, and adding a widget to the catalog means changing the renderer and the catalog, not five libraries.

## 8. The designer

The designer is part of the desktop app and follows the Turbo C++ for Windows model: place widgets, set properties, see the result, connect a control to an action and jump to the code behind it.

| Turbo C++ for Windows | Clarity |
|---|---|
| Resource Workshop edits `.rc` text files | the designer edits `.clarity` spec files |
| AppExpert generates the application skeleton | `clarity init --stack ...` |
| ClassExpert generates the handler stub and response table entry | the generator emits the action declaration and wiring, plus an implementation stub |
| the developer writes the handler body | hand-written action implementation, TDD-gated |

- **One-way.** The designer reads and writes `.clarity` spec files. It never edits generated code. ClassExpert found its generated parts through marker comments inside hand-written source, and editing those by hand easily broke it. Clarity keeps generated code in separate `generated/` directories, so the spec is the only source and nothing has to be synchronized back.
- **Same footing for AI and humans.** The designer, an AI agent and a human in a text editor all edit the same text files and go through the same checks. Changes made in the designer show up as normal, readable diffs.
- **Business-application patterns only.** The designer edits the screen patterns from section 5.5 with groups, tabs and rows. It does not offer free pixel layout.
- **Live preview.** The frontend generator runs on every change and renders the screen with sample data generated from the dictionary (respecting types, lengths and patterns).
- **Code behind.** Double-clicking an action button opens the hand-written implementation in the user's editor (through the LSP or an editor command), or creates the stub if it does not exist yet.

### 8.1 The design canvas

The design canvas is the working surface of the designer, the equivalent of Resource Workshop's dialog editor. For pages (section 5.6) it shows the snap grid: widgets are dragged from the palette onto grid cells, moved by dragging, resized by their corner handle, and edited in the properties panel (caption, binding, click target, position and the widget's own properties from the catalog). For screens it arranges entity fields in groups, rows and tabs as described below.

- **Real widgets in edit mode.** The canvas renders the same components as the preview and the generated app, with a selection and drag layer on top. What is designed is exactly what runs.
- **Insertion points, not coordinates.** Dragging shows insertion markers between items, inside groups, rows and tabs. Hovering a tab during a drag switches to it. Dropping rewrites the spec.
- **Toolbox of unplaced fields.** The entity's fields that the screen does not show yet, dragged onto the canvas. Only fields from the dictionary can be placed.
- **Properties panel.** For the selected element: group and tab titles, widget override, screen title. Dictionary properties (type, length, required, help) are shown read-only with a reference to the entity, so a screen cannot redefine a field.
- **Structure commands.** Wrap a selection in a group or row, unwrap a group or row, remove an item. Containers left empty by a move or removal are removed with it.
- **Window size presets.** Narrow, medium and wide, to check how rows and groups wrap.
- **Safe writes.** Every change is a structural edit applied in Rust: it touches only the affected lines, keeps comments and blank lines, is rejected if it would introduce a parse error, and is rejected if the file changed on disk since the designer last read it. The file is written atomically, and the normal file watcher updates every view.

## 9. Directives

Directives bring contracts and rules to hand-written code: action implementations in spec-driven projects and all code in adopted projects. They are structured comments attached to the next symbol, so they move with the code through refactors and history.

### 9.1 Syntax

A directive block starts with a line `@clarity` inside the language's normal line comment, followed by `key: value` lines.

Rust, C, C++, C#, Go, TypeScript:

```rust
// @clarity
// intent: Move money between two accounts atomically.
// require: amount > 0
// require: !std::ptr::eq(src, dst)
// require: src.balance >= amount
// ensure: src.balance + dst.balance == old(src.balance + dst.balance)
// effects: none
// stability: frozen-signature
pub fn transfer(src: &mut Account, dst: &mut Account, amount: u64) { ... }
```

Python, Ruby:

```python
# @clarity
# require: len(items) > 0
# effects: db.read
def summarize(items): ...
```

Doc-comment forms (`///`, `/** */`, docstrings) are accepted where a pack declares them.

Directive expressions are written in the host language, unlike spec contracts (section 5.4), because they only ever apply to one language. Packs provide templates that turn `old()` and `result` into valid code.

### 9.2 Keys

| Key | Meaning | Checked |
|---|---|---|
| `intent` | Free text: why this exists. For humans and AI context only. | No (only that it exists, if a rule requires it) |
| `require` | Precondition. Repeatable. | Yes, through generated tests or runtime checks |
| `ensure` | Postcondition. `old(expr)` is the value before the call, `result` the return value. Repeatable. | Yes, same as `require` |
| `effects` | Declared side effects. | Yes, against the call graph |
| `stability` | `free`, `frozen-signature` or `frozen`. | Yes, against the diff |
| `waive` | An explicit exception to a rule (section 13). | Yes, format and expiry |

### 9.3 Effects vocabulary

`none`, `fs.read`, `fs.write`, `net`, `db.read`, `db.write`, `process`, `env`, `time`, `random`, `ipc`, `ui`.

An engine (section 5.11) has none of these: what it may call is the fixed list of its `uses` (clock, random, log, progress), and data and files reach it only as values from the application.

Each pack maps known APIs to effects (for example in Rust, `std::fs::write` is `fs.write` and `reqwest::*` is `net`). Projects extend the mapping in `clarity.toml` for their own wrappers. A function's effects must cover the effects of everything it calls. In statically typed languages this check is close to exact. In Python and Ruby it is best-effort, and reports say so.

## 10. Project configuration

`clarity.toml` at the project root holds the stack, the module map and the rules. Example for a spec-driven Tauri desktop app:

```toml
[project]
name = "ledger"
mode = "spec"
profile = "strict"

[stack]
backend  = "rust"
shell    = "tauri"
frontend = "svelte"
styling  = "tailwind"
storage  = "sqlite"

[modules]
customers = { spec = "spec/customers.clarity", path = "crates/customers", may_use = [] }
sales     = { spec = "spec/sales.clarity",     path = "crates/sales",     may_use = ["customers"] }
frontend  = { path = "frontend/src", lang = "typescript", may_use = ["generated"] }

[rules.modularity]
severity = "block"

[rules.tdd]
severity = "block"
changed_line_coverage_min = 0.85
mutation_score_min = 0.70

[rules.dry]
severity = "block"
min_clone_tokens = 80
warn_clone_tokens = 40
ignore = ["**/tests/**"]

[rules.shape-in-spec]
severity = "block"

[effects.map.rust]
"crate::storage::*" = "db.write"
```

Every rule has a `severity`: `off`, `report`, `warn` or `block`. Any setting can be overridden per module:

```toml
[modules.frontend.rules.dry]
min_clone_tokens = 120
```

Adopted projects use `mode = "adopted"`, leave out `[stack]` and `spec`, and may declare explicit codegen boundaries:

```toml
[boundaries.desktop-ipc]
owner     = "desktop"
generator = "tauri-specta"
output    = "frontend/src/api/generated/desktop.ts"
```

## 11. Rules

### 11.1 Modularity

- **Dependency allowlist.** Packs extract dependency edges (`use`, `#include`, `using`, `import`, `require`). An edge from module A to module B is a violation unless B is in A's `may_use`. In spec-driven projects the same check runs on `use` in spec files.
- **No cycles** between modules.
- **Public surface.** Other modules may only use what a module exports: `pub` items reachable from `lib.rs` in Rust, public headers or C++20 module interfaces in C/C++, `public` types in C#, exported identifiers in Go, `__all__` or a declared `api` file in Python, a declared entry file in Ruby, `export` from the module's index in TypeScript.
- **Effects per module.** A module may restrict which effects its code may have.
- **Unmapped files.** Source files that belong to no module are a violation, so new code cannot appear outside the map.

### 11.2 TDD

TDD is a process, and finished code does not show whether the test came first. Clarity enforces it differently depending on who is working.

**For AI agents: a strict state machine per task.**

```text
idle -> test-written -> red -> green -> (refactor) -> done
```

- `idle`: the agent may only create or edit test files (or the spec, which then generates stubs with failing tests).
- `test-written`: the agent must run the tests through Clarity.
- `red`: a new or changed test fails for the expected reason. Production edits are unlocked, but only for symbols the failing test reaches.
- `green`: all tests pass. The agent may refactor; every refactor edit is followed by a test run that must stay green.
- Production edits outside these states are rejected by the edit hook.

State lives in `.clarity/state/tdd.json`, one entry per task.

**For humans: proxies at commit and CI time.**

- A diff that changes production code must also add or change a test covering the changed code.
- Changed-line coverage must meet `changed_line_coverage_min`.

**For both: are the tests real?**

- Mutation testing runs on changed code. A test suite that executes code without asserting anything shows up as a low mutation score.
- Contracts from the spec and from directives generate property tests, so specified behavior is tested even when nobody wrote a test by hand.

### 11.3 DRY

- **Structural clones.** Syntax trees are normalized (identifiers and literals replaced by placeholders) and hashed over sliding windows. Clones above `min_clone_tokens` violate the rule; those between `warn_clone_tokens` and `min_clone_tokens` produce warnings. Generated code is excluded.
- **Knowledge duplication.** This matters more than code duplication. Clarity flags the same enum value sets, validation patterns (regex literals, numeric limits) and constants defined in more than one module, or in hand-written code when the spec already defines them.
- **Across languages.** Handled by generation (spec-driven) or the boundary rule (adopted): one owner, generated consumers.

Overly eager DRY makes AI tools invent bad abstractions to remove two similar lines. The thresholds are intentionally tunable, and the metrics loop shows where they are wrong.

### 11.4 Backend drives frontend

In spec-driven projects this follows from generation: the frontend only gets types, validation and API calls from generated code. In adopted projects it is a boundary rule: for every boundary between modules in different languages or processes, one side owns the types and the other side gets generated code (tauri-specta, utoipa with openapi-typescript, protobuf/buf).

In both modes:

- The frontend may not define its own copies of backend types, enums or validation rules.
- No raw transport calls. In a Tauri frontend, direct `invoke("...")` calls are forbidden; only the generated typed client may be used. The same applies to raw `fetch` against the backend.

### 11.5 Shape lives in the spec

Spec-driven projects only.

- **Generated code is frozen.** Hand edits to files under `generated/` are rejected by the edit hook, and `clarity gen --verify` fails in CI if generated output differs from the spec.
- **No hand-written shape.** Packs declare what counts as shape in their language and stack, and hand-written code may not contain it: `#[tauri::command]` functions, HTTP route registrations, Wails-bound methods, frontend route files, database table definitions or migrations outside `generated/`.
- **Every declared action and computed field has an implementation**, which the compiler enforces through the generated declarations.

### 11.6 Directive and spec integrity

- Every directive and spec file must parse and be in canonical form.
- `require` and `ensure` expressions must compile in the generated tests.
- A directive that no longer attaches to a symbol is a violation (it drifted).
- `stability: frozen` spans may not change; `frozen-signature` symbols may not change their signature.

## 12. Gates

Each gate alone can be bypassed, so Clarity uses three.

| Gate | Who it catches | When | Bypass |
|---|---|---|---|
| AI edit hooks | AI agents | before and after every edit | none for the agent |
| Pre-commit | humans | locally, before a commit | `--no-verify` |
| CI | everyone | on every push or pull request | only a repository admin |

### AI edit hooks

For Claude Code, `clarity init` installs hooks in `.claude/settings.json`:

- **PreToolUse** on Edit and Write: rejects edits to `clarity.toml`, to generated files, to frozen spans, hand-written shape, and production edits the TDD state does not allow. Exit code 2 blocks the edit and returns the reason to the agent.
- **PostToolUse** on Edit and Write: runs `clarity check --changed` (and `clarity gen` after spec edits) and returns violations to the agent so it fixes them immediately.

Other agents use the MCP server (section 17.3) and the same CLI.

### Pre-commit and CI

`clarity init` writes configuration for a hook manager (lefthook or pre-commit) and a CI workflow (GitHub Actions first). The user installs and enables them; Clarity does not touch `.git`.

## 13. Protecting the rules

- **The rules file is protected.** AI agents may never edit `clarity.toml`; the edit hook enforces this. Humans change it through a reviewed change. On GitHub, `clarity init` adds a CODEOWNERS entry for it.
- **Waivers are explicit and expire.**

  ```rust
  // @clarity
  // waive: dry, reason: "Two parsers for two wire versions, merged after v2 sunset", until: 2026-12-01
  ```

  A waiver without a reason or expiry date is a violation. Expired waivers become violations. AI agents may propose waivers, but a waiver added by an agent is reported as such and needs a human to accept it in review.

Without a legitimate way to make exceptions, people disable tools like this. Waivers also show where the rules themselves are wrong.

## 14. Tuning loop

Clarity treats the rules as something to optimize, not something fixed.

- **Metrics per rule and per module:** how often it fires, how often it is waived, how often a violation is marked as a false positive (`clarity mark-fp <violation-id>`), and how long violations take to fix.
- **Tuning proposals:** `clarity tune` analyzes the metrics and proposes changes, for example: "dry fired 40 times in `frontend/custom`, 31 were waived. Raise `min_clone_tokens` there to 120?" A proposal is a patch to `clarity.toml`. Only a human applies it.
- **Report mode:** a new or changed rule can run at `severity = "report"` first, so its effect is visible before it blocks anything.
- **Ratchet mode:** for adopted projects. `clarity adopt` records a baseline of current violations in `.clarity/baseline.json`. New and changed code must be clean; the baseline may only shrink.
- **Profiles:** `strict`, `standard` and `legacy` bundle defaults; projects override from there.

## 15. Language packs

Packs serve both modes: they power the rule engine for hand-written code, and backend and frontend generators build on them.

### 15.1 Structure

A pack is mostly data:

```text
packs/go/
  pack.toml             toolchain commands, file patterns, effect map, shape patterns
  queries/
    symbols.scm         functions, types, methods
    imports.scm         dependency edges
    exports.scm         public surface
    directives.scm      where @clarity comments attach
    tests.scm           what counts as a test
    calls.scm           call sites for the effects check
    shape.scm           hand-written shape (routes, bindings) for rule 11.5
  templates/
    property_test.tmpl  contract to property test
    runtime_check.tmpl  contract to inline check (optional)
```

```toml
[pack]
name = "go"
extensions = ["go"]
grammar = "tree-sitter-go"
test_files = ["**/*_test.go"]

[toolchain]
build    = "go build ./..."
test     = "go test ./... -json"
coverage = "go test -coverprofile={out} ./..."
mutation = "gremlins unleash --output={out}"
format   = "gofmt -l ."
lsp      = "gopls"

[effects]
"os.WriteFile" = "fs.write"
"net/http.*"   = "net"
"time.Now"     = "time"
```

Packs that need real logic ship a WebAssembly plugin, run through Extism or wasmtime, so plugin authors can use any language that compiles to WASM. In v1 the packs and their tree-sitter grammars are built into the Clarity binary; external packs come later.

### 15.2 v1 language set

v1 covers the languages where every rule can be enforced. Rust and TypeScript come first because the first stack (Rust + Tauri + Svelte) and Clarity itself use them.

| Language | Build / test | Coverage | Mutation | Property tests | LSP |
|---|---|---|---|---|---|
| Rust | cargo | cargo-llvm-cov | cargo-mutants | proptest | rust-analyzer |
| TypeScript | npm + vitest | vitest (v8) | StrykerJS | fast-check | typescript-language-server |
| C | CMake + CTest | llvm-cov / gcovr | Mull | RapidCheck (via C++ harness) | clangd |
| C++ | CMake + CTest (GoogleTest, Catch2, doctest) | llvm-cov / gcovr | Mull | RapidCheck | clangd |
| Go | go | go cover | gremlins | rapid | gopls |
| Python | pytest | coverage.py | mutmut | Hypothesis | pyright |
| C# | dotnet test (xUnit, NUnit, MSTest) | coverlet | Stryker.NET | FsCheck | csharp-ls |
| Ruby | RSpec or Minitest | SimpleCov | mutant | prop_check | ruby-lsp |

Notes:

- Python and Ruby are supported for adopted projects only; they are not backend targets for generation in v1.
- Effects checking in Python and Ruby is best-effort (dynamic dispatch, monkey patching). Reports mark it as such.
- Mull requires building with Clang.
- mutant (Ruby) requires a license for commercial closed-source use. The pack must allow switching mutation testing off or substituting another tool.

Later packs: Haxe, Java, Kotlin, Swift, Zig and others. A pack that cannot enforce a rule declares that, and Clarity shows per module which rules are actually enforced. A module is never reported as passing a rule that could not be checked.

### 15.3 Contracts per language

1. **Generated property tests (default).** For each action or function with `require`/`ensure`, a property test is generated: inputs are generated and filtered by `require`, the function is called, and every `ensure` is asserted. `old(expr)` is evaluated and stored before the call. Generated tests follow the frozen-generated-code rule.
2. **Runtime checks.** In spec-driven projects, the generated action wiring always checks contracts at runtime (configurable to debug builds only). In hand-written code, inline checks are opt-in where the language supports them without rewriting source: the `contracts` crate in Rust, C++26 `pre`/`post` once compilers support them (plain `assert` until then), icontract in Python.

## 16. Project setup

- **`clarity init --stack rust+tauri+svelte`** calls the native scaffolding tools (`cargo new --vcs none`, the frontend tool's scaffolder and so on), then adds Clarity's layer: `clarity.toml`, a starter `spec/` directory, generated code, agent hooks, hook-manager config, CI workflow, and a `CLAUDE.md`/`AGENTS.md` section generated from the rules.
- **`clarity adopt`** sets up an existing project: detects languages and structure, proposes a module map, records a baseline and starts in ratchet mode.
- **Templates** are a directory of files plus a recipe (`template.toml`). Users can add their own without changing Clarity.

## 17. Surfaces

### 17.1 CLI

```text
clarity init --stack S            create a spec-driven project
clarity adopt                     set up an existing project, ratchet mode
clarity fmt                       format spec files canonically
clarity gen [--verify]            generate code from the spec; --verify for CI
clarity index                     build or update the index
clarity check [--changed|--all]   run rules
clarity test                      run tests through the TDD state machine
clarity graph                     print or export the module graph
clarity explain <violation-id>    explain a violation and how to fix it
clarity mark-fp <violation-id>    mark a false positive
clarity tune                      propose rule tuning from metrics
clarity report                    metrics and enforcement coverage per module
```

### 17.2 LSP server

`clarity lsp` serves two purposes:

- Full language support for `.clarity` spec files: diagnostics, completion, go to definition, rename across spec files.
- Rule violations as diagnostics in hand-written code in any editor (VS Code, Zed, Neovim, Rider, CLion), next to the language's own LSP. Code actions: generate a test stub, add a waiver template, jump to the spec declaration of an action.

### 17.3 MCP server

`clarity mcp` exposes tools to any MCP-capable agent:

- `spec_for(name)`: the spec declaration of an entity, action or screen
- `directives_for(symbol)`: directives, module, allowed dependencies and effects for a symbol
- `module_map()`: modules, their rules and boundaries
- `check(diff)`: violations a proposed change would cause
- `tdd_state(task)` and `tdd_advance(task)`
- `explain_violation(id)`

### 17.4 Desktop app

A Tauri app that combines:

- **The designer** (section 8)
- **The control room:** module graph with violations, TDD state per task including AI sessions and what the hooks blocked, rule metrics, waivers with expiry, tuning proposals to review, enforcement coverage per module
- **`init` and `adopt` wizards**, including stack selection

## 18. Implementation

Clarity is written in Rust. The workspace:

```text
Cargo.toml                  workspace
crates/
  clarity-core/             config model, directive parser, violation types
  clarity-spec/             spec parser (lossless syntax tree), formatter, neutral model
  clarity-gen/              generator framework, transport contract
  clarity-gen-rust/         Rust backend generator
  clarity-gen-tauri/        Tauri shell generator
  clarity-gen-svelte/       Svelte frontend generator
  clarity-index/            index storage (SQLite), incremental updates
  clarity-lang/             pack loader, tree-sitter adapters, built-in packs
  clarity-rules/            all rules from section 11
  clarity-tools/            running native toolchains, parsing coverage and mutation output
  clarity-metrics/          metrics store, tuning proposals
  clarity-cli/              the `clarity` binary
  clarity-lsp/              LSP server
  clarity-mcp/              MCP server
app/                        Tauri desktop app with designer (Rust + TypeScript)
packs/                      pack data: pack.toml, queries, templates
templates/                  project templates
```

Main dependencies: `tree-sitter` and the grammar crates, `rowan` or a similar lossless syntax tree library for the spec parser, `serde` and `toml`, `clap`, `rusqlite`, `petgraph`, `notify`, `tower-lsp` or `lsp-server`, `rmcp`, `tauri` with `tauri-specta`, and later `extism` or `wasmtime` for plugins.

Clarity is developed under its own rules from the first milestone that can enforce them.

## 19. Roadmap

The first goal was a visible slice: the spec language, a live preview and the visual designer. Next come projects with a stack (section 1.2), the behavior language, and one stack end to end (Rust with the web renderer) before C# with Avalonia proves a native toolkit and the other backends add breadth.

| Milestone | Content | Status |
|---|---|---|
| S1 | Screen spec: parser and checker for modules, enums, entities, `browse` and `form` screens with groups, rows and tabs. Neutral model as JSON, TypeScript bindings generated with ts-rs, `clarity spec check` and `clarity spec model` | done |
| S2 | Live preview: Tauri + Svelte app that watches a spec directory and renders screens with deterministic sample data, including browse-to-form navigation and a problems list | done |
| S3 | Design canvas for form screens, see section 8.1. Backed by a structural edit engine in Rust that rewrites spec files line-block by line-block, keeping comments, blank lines and untouched text exactly as they were | done for forms; open: undo/redo, adding and removing tabs, designing browse columns, per-screen label overrides |
| S3b | Page designer: `page` construct with snap grid, widget catalog, palette, placing, moving and resizing widgets, binding and properties (sections 5.6 and 8.1) | done; open: checking that container contents fit the container height, undo/redo, responsive reflow on narrow windows |
| S3c | Frame regions: `app` construct, menu, toolbar, ribbon, sidebar and footer docked around the grid (section 5.6) | done |
| S3d | One designer: every window is designed in the page designer with the same Parts catalog (frame, basic input, containers, data, graphics), free placement and resizing, and a Data panel of what it binds to; form and browse become starter templates (section 1.1) | done |
| S3e | Part names and events: each part gets a name and an Events list; wiring an event writes the handler name into the window, ready for code behind | done |
| P1 | Projects: `clarity.toml` with backend, UI, targets and data sources (sections 1.2 and 1.3); New project asks for them and starts with one SQLite source; the Project tab shows and edits them. Opening and creating projects in the app is done | done |
| P2 | The project as one file: `<name>.clarity` with a `project` block (stack, output, data sources) replacing `clarity.toml`, and the modules in a folder of the same name next to it (section 1.2). Open and New work on the project file; the shop example moves to `examples/shop.clarity` with `examples/shop/`, its app module renamed so it does not clash with the project file | done |
| P3 | Build tools per stack: Clarity is installed as a finished program, so a machine may lack a stack's tools (`cargo`, the .NET SDK, Go, a C or C++ compiler). Clarity looks for them, marks a stack whose tools are missing in New project and Settings with what to install, and keeps designing, previewing and generating working; only Build and Run say why they cannot run, so the generated source can be built elsewhere | done: `clarity_gen::tools` checks each stack through a `Probe` (tested with a faked machine): cargo, and on Linux WebKitGTK through pkg-config, for Rust; a .NET SDK of the version generated projects target for C#; Go; for C and C++ a compiler, CMake, and make or Ninja. One list of what to install, with where to get it, serves `clarity tools` (a line per stack), the Build tools row under the stack in New project and on the Project tab (with Check again; `stack_tools` in the Tauri app), Build and Run (they still generate, then say what to install), and the README.md every generated project now has: what the stack needs, the commands to build and run it without Clarity, and its data sources (a server database by its environment variable, with an example connection). Like JetBrains' toolchains; still open: pointing Clarity at a tool that is not on PATH |
| L1 | The Clarity behavior language: handlers written in Clarity in `code` blocks, with typed access to a window's parts and data (sections 1.1 and 5.9); wiring an event writes an empty handler; the code panel in the designer checks while you type | core done; open: modules with shared functions and actions (L2) |
| L2 | Backend modules in Clarity: functions and actions in modules, callable from handlers and each other, with `require`, `ensure` and `effects` (section 5.10); design as text with the designer following the text (section 5.9); modules show and edit their functions | done |
| G1a | Rust backend: `clarity-gen` writes a Cargo project into `out/` (deterministic, kept in sync by `write_out`), with records and enums from the dictionary, the SQLite schema as the first migration, functions and actions as Rust with contract checks, and the `clarity-runtime` crate copied in; `clarity gen`; the generated shop builds and runs, and generated logic is tested | done |
| G1b | Handlers and the event protocol, the web renderer as a standalone package, the Tauri shell, Run and Build on the Preview tab | done |
| E1 | Events beyond parts: window `on open`, `on close` and `on return` (back from a window it opened, to refresh), app `on start`; data events (before and after save) follow with D1 | done (section 5.9): window and app events, `reload`, Events in the designer, the browse template refreshes on return; data events stay with D1 |
| U1a | Forms and text parts (section 5.6): `form` bound to one record with `on submit`, `label for`, `help`, `heading`, `text`, `memo`, `password`, `switch`, `radio`; `placeholder`, `tooltip`, `readonly`, `taborder`, `key`, `default` and `cancel`; `error` and `readonly` as part properties in handlers; the form template and field drops place label, input and help together | done; open: a `time` part waits for a time field type, menu item keys are shown but not yet pressed at runtime |
| U1b | Actions and containers: icon button, split button, toggle button, button group; expander, accordion, scroll box, separator, link, badge, icon, progress, busy indicator; containers that share the column lines of the grid around them (as CSS subgrid does), so a label and an input in a panel line up with those outside it, and a label column for forms | icons done (2026-10-23): Lucide 1.50.0 vendored in crates/clarity-spec/icons (converted to one plain path per icon that the web and Avalonia read alike), `icon "name"` on buttons and commands, the `icon` part, the checker's suggestions, only the used icons in each application with Lucide's license in THIRD_PARTY_NOTICES.md. Small parts done (2026-10-23): `separator [vertical]`, `link` (`to`, `bind` or `on click`), `badge`, `progress ... max`, `busy`, with conformance/small-parts.json. Toggle, button group (`buttons`) and split button done (2026-10-23; Avalonia ToggleButton, SplitButton with MenuFlyout; the designer's Add menu). Accordion (`section` blocks, `part.open`, new part state in every runtime: Rust 0.14.0, Go v0.16.0, NuGet 0.20.0) and scroll box done (2026-10-23). Shared grid lines (always: frames sit 3px out into the column gap, ClarityFramePadding on Avalonia, GridLineTests) and the form label column (`labels 3`, labels made by the spec after parsing, named `<input>Label`, showing the field's label key) done (2026-10-23): U1b complete. Was: toggle, button group and split button (`split` on a button, its `menu` block the dropdown), then accordion and scroll box, then shared grid lines and a label column |
| U1c | Data parts: tree, tree table, pager, search bar, lookup (a reference picked from a searchable list), chart; combo box, slider, spin box, color, file and tags inputs | decided (2026-10-23): inputs first, in slices: (1a) `slider` and `spinbox` for integer, decimal and money, with `min`, `max` and `step` (the field's own `min` and `max` without them; a slider 0 to 100 without either) done 2026-10-23: conformance/ranges.json (exact decimal steps), web SliderInput and NumberInput with a range, Avalonia SliderInput and NumberInput with Min, Max and Step in NuGet 0.21.0; (1b) the field type `color` (kept as text `#rrggbb`) and its `color` input, done 2026-10-23: VARCHAR(7), conformance/colors.json (reading typed colors, the shared 60-color palette), Clarity's own palette popup on both renderers (no system color dialog), color table columns, the checker on defaults and on text code puts into a color field, NuGet 0.22.0; (1c) the field type `tags` (a list of texts, kept in one text column) and its `tags` input; (1d) the field type `file` (kept like an `image`: in the database, or with `store` as a file) and its `file` input. Then (2) `combo` (a text box with suggestions, `from` an enum or a list; any text may be typed) and `lookup` (an existing record for a `ref` field, searched as typed, the popup showing the fields of `show`); (3) search bar and pager; (4) `tree` and `treetable`, bound to the top records with `children` naming a field that lists the entity's own kind (`children: list<Folder>`); (5) chart |
| U1d | Window kinds and overlays: dialogs with OK and Cancel results, wizards with steps, context menus, popovers | dialogs that give a value done (2026-10-22): `page X: dialog gives Type`, `close with value`, `ask X [with ...] then handler(values)` on every stack (the reply of the closing handler carries `result`; the web renderer remembers which open window asked; Avalonia hands the value to the opener). wizards with steps done (2026-10-22, 5.15); context menus and popovers done (2026-10-22, 5.16), on the web renderer and Avalonia's ContextMenu and Popup, with no runtime release needed |
| U1e | Table columns (decided 2026-10-22): a width in pixels after a field (`columns title 240, artist, year 60`; a column without one shares what is left), `resizable` and `sortable` as options, and `order by field [desc]` for the order rows show in first, written as in queries. On the design canvas the header edges are dragged to set widths. Running: with `resizable` the person drags header edges and the widths are remembered per window and table for that person (double-click: the designed widths again); with `sortable` a click on a header sorts by it, again the other way. Sorting is the renderer's: rows keep their places in the data, so `selected` and the handlers see the same record. Web renderer: `lib/ui/Table.svelte`, copied by `clarity-gen`; Avalonia: ClarityTable in Clarity.Avalonia (widths remembered through `Remembered`), so a new package version and its NuGet upload | done (2026-10-22): spec, checker and renamer; IDE properties and header edges on the canvas; web table (sorting in the view, grips, widths in local storage, conformance/table-order.json); Avalonia ClarityTable (own sorting, Remembered widths), C# generator; Clarity.Avalonia 0.17.0 |
| U1f | Clarity's own dialogs in the applications people design, as the IDE has them: `ask` in handlers with the answer running another handler (section 5.14, decided 2026-10-22), dialog windows closing `with` a value (with U1d), and `message` in the same dialog. Every stack in its idiom: the web renderer (`lib/ui`, copied by `clarity-gen`) and Avalonia (Clarity.Avalonia, so a new package version and its NuGet upload). With U1e, after the class diagram designer | done (2026-10-22), dialog windows closing `with` a value included (U1d): `ask` in the language and checker; replies carry the questions on every web stack (Rust crate 0.11.0, the Go module, the C and C++ runtimes) with `answer_event` running the answering handler; the web renderer's dialog (lib/ui/AskDialog.svelte, Dialog.svelte) with warnings and errors in it and a small x on notes; Avalonia asks through WindowViewModel.Ask and the Shell's dialog (Clarity.Avalonia 0.17.0); build tests for Go, C, C++ and C# |
| U1g | Settings (section 5.17, decided 2026-10-23): one `settings` block of typed values kept per person on their machine, with Clarity's own `language` and `theme`; every window binds `settings` and code reads and sets it; the pickers change the same values; a "Settings window" template | done 2026-10-23: the spec reads the block as the record type `Settings` (`Entity.settings`, `FieldType::Language` and `Theme`, `DataDecl.made`, `Project::stored_entities`), conformance/settings.json (which kept values fit, where the file is), the web renderer holds them (lib/settings.ts, Runtime.svelte) with the shell commands `read_settings` and `keep_settings` (Rust `clarity_runtime::settings`, Go `clarity.ReadSettings`, C `clarity/settings.c`, C++ `clarity/settings.cpp`), Avalonia `KeptSettings` and a generated `Settings.Current`; the Settings window template. Releases: clarity-runtime 0.15.0, Go v0.17.0, NuGet 0.22.0 |
| I1a | Formats (section 1.5): dates, times, numbers and money shown and entered in the locale of the person using the application; `language` and `currency` in the project file; named formats for `format`; money with its currency's decimals | done for the web renderer (LANGUAGE.md section 4, Formats): `language "tag"` and `currency CODE` checked and on the Project tab, `format` a name checked per type (catalog in `clarity_spec::formats`, a Format column in the entity editor), the renderer formats and reads values with `Intl` (`app/src/lib/format.ts`), the shared cases in `conformance/formats.json` (`npm test`); Avalonia meets the same cases with G2; open: date inputs follow the system's locale through the browser's own date picker |
| I1b | Text with slots (section 1.5): `"Saved {customer.name}"` in messages, refusals and captions, slots checked as expressions and shown in the locale's format | done for the web renderer (LANGUAGE.md section 10, Text with slots): slots are paths checked like expressions, `{{` and `}}` braces, text with slots allowed in `message`, `refuse`, `caption` and `error` and a warning for shown text joined with `+`; the runtime's `Text` and `Slot` carry the text as written and typed values with the field's format and scale (plain text stays a string on the wire), refusals reach the screen with their slots, and the renderer fills them (`fillText`, cases in `conformance/formats.json` under `fills`); Avalonia meets the same cases with G2 |
| I1c | Translations (section 1.5): text collected from the whole project, translation files per language and module, slots kept, missing translations listed, a Translations table on the Project tab, a language choice for the canvas and Preview, switching language in the running application | done for the web renderer (LANGUAGE.md section 2, Translations): one file per language for the project (`translations/nl.clarity`), texts collected by `clarity_spec::translations::texts` (labels, help, titles, captions, placeholders, tooltips, tabs, enum labels, frame items, status, messages, refusals, captions and errors set in code, Yes and No), checked for slots, duplicates, stale lines and the file's language; the Translations table and the designer's language choice on the Project tab; the application picks the translation for the person's locale (cases in `conformance/translations.json`), and `on click -> language` switches it while the application runs (remembered for the next start); open: `context` for one text with two meanings, the generated literal key maps of section 6.7 for the other stacks (G2) |
| T1 | Themes (section 1.5): a `theme` with named colors per light and dark mode, font, size, radius and named styles; `style` on parts; a theme editor with a live sample; CSS variables for the web renderer, resources for Avalonia | done for the web renderer (LANGUAGE.md section 7, Themes): `theme` declarations checked (`clarity_spec::themes`), `theme Name` in the project file and on the Project tab, `style` on parts checked against that theme and chosen in part properties; a theme resolves for light or dark mode in `clarity_spec::themes::resolve` (cases in `conformance/themes.json`), the generator writes a CSS file per theme into `ui/themes/` and links it, the application holds the theme's class and each part its style's class, the designer gets the same CSS from the backend, and palettes are chosen per element (`data-scheme`) so a theme can be light on a dark system; themes open from Themes in the project tree in their own view, written as text beside a live sample window; several themes per application (`theme Shop Night`), switched with `on click -> theme` and kept for the next start, each with a `title` that is translated, every offered theme holding every style parts use; Clarity's own look as the built-in `theme Clarity` with every value (colors, fonts, sizes, radii, padding, border, shadow, part looks in their states), the application's CSS split from the designer's and reading only those values; Avalonia maps the same resolution onto resources with G2 |
| C1 | Command-line applications: `targets = ["cli"]`; `command` declarations with typed arguments and options, written in Clarity like handlers; `print`, `ask` and `exit`; generated as a program without a window, over the same records and actions; needs queries (`find ... where`) from D1 | |
| D1a | Queries: `find c in Entity where ... order by ...` as an expression, `contains` on text, the `text` of unbound text inputs, handlers that run other handlers; the runtime builds the SQL from a condition tree; the browse template filters and refreshes through its filter | done (LANGUAGE.md section 10); open: conditions through references (`o.customer.name`), which need joins |
| D1b | Migrations (section 1.3): steps written in Clarity in `migrations/`, the schema from replaying them, pending changes as warnings and in the Project tab, proposed renames, confirmation for changes that lose data, `clarity migrate`, SQL per step with table rebuilds for SQLite, applied at startup with references checked | done (LANGUAGE.md section 4); open: data migrations (Clarity code that transforms rows), down steps, counting the rows a narrowing change would not fit, renaming enum values, a command-line switch to apply steps separately |
| D1c | Entities placed per data source: `source` in a module, checked against the project file; migration steps per SQL source; one database per source at run time, each record type knowing its source | done (LANGUAGE.md section 3) |
| D1f | Data events: before and after save, before and after delete, written in Clarity next to the entity | done (LANGUAGE.md section 8, Data events): `code Entity` with `before save`, `after save`, `before delete` and `after delete`, each checked and generated as an action on its record; `refuse "text"` as a statement; the runtime runs them through `Record` hooks in one transaction per database with the saves and deletes they make, and a failed save leaves a new record new; renaming an entity renames its events; open: editing events in the designer (the Code panel shows windows only), one transaction across databases |
| D1d | PostgreSQL, MySQL and SQL Server for owned sources: a runtime driver per database as a cargo feature, queries and record statements per dialect, migration steps written per dialect (types, keys, foreign keys added at the end of a step, unique indexes that skip empty text, type changes through a new column), connections from `env:NAME` | done (LANGUAGE.md section 4, Databases), tested against live servers (`crates/clarity-gen/tests/servers.rs`); open: TLS for PostgreSQL and MySQL, baselines and Import for these databases (D1e), money shown with the four decimals the servers store |
| D1e | External databases: `external` and `readonly` on a data source; `table`, `key` and `column` mapping; the runtime and generator using the mapped names; Import (Project tab and `clarity import`); comparing the dictionary with the live tables on every load; new crate `clarity-db` | done for SQLite (LANGUAGE.md section 4), Take ownership included: a baseline step that creates only what is missing, mapped names kept in the migration history with `map table`, `map key` and `map column` steps; open: keys that are not one integer column, Import, checking and baselines for PostgreSQL, MySQL and SQL Server |
| D2 | Web API data sources: REST endpoints mapped to entities, auth and paging, OpenAPI import | |
| D3 | Copying data between databases when a source changes kind | |
| G2a | C# data and logic: the C# runtime (`runtimes/csharp/Clarity.Runtime`, net10.0, ADO.NET with Microsoft.Data.Sqlite, copied into every generated project like the Rust runtime), records and enums from the dictionary, migration steps from the shared `migration_sql`, functions, actions and data events in C#; the SQL of queries checked against the Rust runtime's through `conformance/queries.json`; the generated project builds with `dotnet build` | done: `stack csharp avalonia` generates `<Name>.csproj` (net10.0, the runtime's NuGet packages copied from its own project file), one C# file per module with enums (and their labels), records as classes implementing `IRecord<T>` (`CreditLimit` properties, defaults, data events as `BeforeSave` and the others calling `Logic`), functions, actions and data events as methods of a `static partial class Logic` (contracts through `Contracts.Require` and `Ensure`, refusals as `RefusedException` with the text and its slots, queries as `Query` and `Cond`), `Schema.cs` with the steps of the shared `migration_sql` (the SQL writer moved out of the Rust backend into `clarity-gen/src/schema.rs` and `ddl.rs`, and the choice of databases into `clarity-gen/src/lib.rs`), and `Program.cs` that opens and migrates every database; the runtime has 20 tests of its own (`dotnet test runtimes/csharp/Clarity.Runtime.Tests`, ported from the Rust runtime's) and passes `conformance/queries.json` with the Rust runtime; the shop example builds with warnings as errors and runs (`cargo test -p clarity-gen --test csharp_build -- --ignored`); open: handlers and windows (G2b), the server drivers (G2d) |
| G2b | Avalonia windows: each window as AXAML with a ViewModel (CommunityToolkit.Mvvm), the grid as a `Grid` whose growing rows are `*` rows (section 2), the parts of U1a, tables as `DataGrid`, the frame (menu, toolbar, ribbon, sidebars, footer), handlers as ViewModel methods, messages, opening, closing and returning to windows; Run and Build on the Preview tab for C# projects | done: the runtime `runtimes/csharp/Clarity.Avalonia` (PartState, WindowViewModel whose handlers open and close windows after they end, closing first; Shell, one desktop window with a stack of views; messages; bindings and converters) with 14 headless tests, among them `conformance/windows.json` shared with the web renderer; per window `Windows/XView.axaml`, its code behind and `XViewModel.cs`; `App.axaml`, `Views.cs`, `Shown.cs`; layout values shared through `clarity_spec::layout`; `in new window` on both stacks; Run and Build use `clarity_gen::toolchain` (dotnet for C#). Verified live: the shop runs from Run on Avalonia and on Tauri, `New customer` opens a second desktop window on both. Open: images are placeholders on every stack; the language and theme click targets wait for G2c |
| G2c | Formats, translations and themes for Avalonia (section 1.5): `CultureInfo` formats and converters, `.resx` with a typed accessor and the language switch, theme `ResourceDictionary`s with Light and Dark and style classes and the theme switch; `conformance/formats.json`, `translations.json` and `themes.json` passing in C# | done: the runtime's `Formats` (Clarity.Runtime) shows, reads and fills text by the culture's data, pinned to the shared answer where .NET and `Intl` differ (two-digit days and months, `09:30` on a 24-hour clock, no weekday in a long date, no-break spaces beside a currency or percent sign), and passes every case of `formats.json`, which gained cases for early dates and negative money; `Languages` picks the language as `translations.json` says; `Locale` (Clarity.Avalonia) holds the language shown, its formats and the kept choice, and translates and fills texts; `PartState` keeps captions and errors as `Text` and shows them translated; the `Shell` makes every view again when the language changes; `Pickers` gives the `language` and `theme` targets a submenu on a menu item or a flyout under a button, so no window opens; `Themes` swaps the theme's dictionary and its light or dark mode; `Remembered` keeps both choices in the person's application data. The generator writes `Texts.resx` and `Texts.<language>.resx` with a typed `Texts` class (a property per text, and an explicit map from each text as written to its resource name, so a message made while the application runs translates too), every text in AXAML through `{x:Static local:Texts.X}`, enum labels and choices through `Texts`, a `FormatConverter` per field with a format and an `EditConverter` per number field (`Shown.cs`), and per offered theme `Themes/<Name>.axaml`: Clarity's own theme merged with the project's, Light and Dark dictionaries with every color as a `Clarity*` brush, part looks as Fluent per-state keys, the values of styles, and fonts, sizes, radii and spacing (`clarity_spec::layout::BUTTON_SIDES` and `INPUT_SIDES`, guarded against render.css); styles become style classes in `App.axaml` that read those values, in every button state; `AppThemes.cs` lists the offered themes. Tests: 29 runtime and 22 headless tests, `tests/avalonia_themes.rs` runs `themes.json` against the generated dictionaries, and the shop builds with warnings as errors, its Dutch texts as a satellite assembly. Verified live: the shop started in Dutch with the Night theme kept from a choice, money as `0,00`. Not mapped yet: a button's focus look (Fluent has no key for it), and `_` in a translated caption is an access key |
| G2d | The server databases and external sources for C#: Npgsql, MySqlConnector and Microsoft.Data.SqlClient behind the same runtime calls, `table`, `key` and `column` mapping, read-only sources | done: `PostgresDriver` (Npgsql), `MysqlDriver` (MySqlConnector) and `MssqlDriver` (Microsoft.Data.SqlClient, with `System.Security.Cryptography.Pkcs` pinned past its vulnerable pick) on a shared `ServerDriver` over ADO.NET, behind the same `IDriver` as SQLite: values read back as the runtime's types, a broken reference told apart by each database's code, a migration step in one transaction (MySQL, which cannot undo changes to tables, says which statements ran); `Locations` reads a location as the project file writes it for every stack (PostgreSQL as a URL or libpq's key=value words, MySQL as a URL, SQL Server as an ADO.NET string). A generated project holds only the drivers of its data sources (`<DefineConstants>` `CLARITY_POSTGRES`, `CLARITY_MYSQL`, `CLARITY_MSSQL`) and their packages. The migration steps of the server scenario are shared in `conformance/migrations.json` (kept current by `tests/servers.rs`), and the C# runtime runs the same scenario as the Rust runtime (`ServerTests.cs`) on SQLite always and on each server with the same `CLARITY_TEST_*` variables. Mapping and external sources were there from G2a; `tests/csharp.rs` now checks them, read-only stays the checker's rule. Verified live in docker: both runtimes' scenarios on PostgreSQL 18, MySQL 8.4 and SQL Server 2022, and the generated shop built per database and migrated each with `--check` (`cargo test -p clarity-gen --test csharp_build -- --ignored`). Not tried: the shop's windows against a server database |
| X1 | A second example, so the generators and tests meet other shapes than the shop: `examples/records.clarity`, a record collection in the style of the 2000s, on C# with Avalonia | done: albums with their tracks (master and detail through a filtered track table and a `TrackDialog`), artists (a `list` part), genres as a tree (`parent: ref Genre`, a `before delete` that refuses while genres sit below), loans to friends (dialogs, a function and an action, a `before save` that refuses lending an album twice), a splitter, radio buttons, two themes (Classic, Midnight), a German translation of every text (tested), its first migration written by `clarity migrate`. Build tests run both examples on both stacks. It found and fixed: a button could not `close` or `quit` without a handler; a record's `id` could not be read; `grid.selected` inside a path or a call was not generated (both stacks); a record given to a reference field was not stored as its id (both stacks); two texts whose .resx names differed only in case. Open, worked around in the example: X2 |
| X2 | What the second example lacked: a `duration` type, an empty date, `today`, the value of an unbound checkbox; a tree part, images (placeholders on every stack), modules that use each other, queries through a reference (`a.artist.name`) | partly done: `duration` (whole seconds; `3:45` and `1:02:03` shown and read by both renderers through new formats.json cases; adds, scales and compares in code; an integer field that becomes a duration is proposed as a rename), `none` (the empty value of a date, a date and time or a reference, in code and in queries through a new `isNone` condition in queries.json, `IS NULL` in both runtimes), `today()` and `now()` (the Rust runtime's `today`/`now`, C#'s `Clock`), and `checked` for unbound checkboxes and switches (Rust `Ui`, the web renderer's live parts, C# `PartState.IsChecked`). Records uses all four (step 2 of its migrations). Queries through a reference (2026-10-18): a condition or an order key may go through reference fields to the fields of the records they lead to (`a.artist.name contains x`, `order by a.artist.name`), as a subquery per step (`"artist" IN (SELECT "id" FROM "artist" WHERE ...)`, and for an order a correlated subquery on aliases t1, t2), in all three runtimes with four new queries.json cases and in the server scenario (run on SQLite and PostgreSQL); the paths are resolved once for every generator (`clarity-gen/src/query.rs`); a query that would go into another data source is an error; Records filters and sorts albums by artist. Modules that use each other turned out to work already (code, windows, and tables that refer to each other, whose references servers add after creating the tables); `tests/modules.rs` keeps it so. Fixed pictures (2026-10-18): the project's `images` folder (png, jpg, gif, bmp, webp), checked against `image source`, embedded by every stack (`ui/images` on the web stacks, `AvaloniaResource` on C#), shown by the designer through the `project_picture` command; Records shows a logo, verified live on Avalonia and on Wails. Pictures in records (2026-10-18), as agreed with the user: the field type `image`, kept in the database (BLOB, BYTEA, LONGBLOB, VARBINARY(MAX)) or with `store name` as a file in a file store of the project settings (`filestore photos "data/photos"` or `"env:NAME"`, relative to the project's folder like a SQLite file; the Project tab edits them, and the entity editor picks a field's store). A picture is none, stored or new: reading records selects only whether there is one (`CASE WHEN col IS NULL THEN 0 ELSE 1 END`), the screen fetches each on its own (the `picture` command on the web stacks, `PictureOf` through the view model on Avalonia), a save leaves a stored picture's column alone and never binds NULL to a binary column, and a file store names a file by the SHA-256 of its bytes and never deletes one. Alike in the three runtimes through `conformance/pictures.json` (kinds, file names, wire form, presence), with the server scenario's `portrait` run on SQLite, PostgreSQL, MySQL and SQL Server for each runtime. The image part loads and removes a picture (web renderer, Avalonia `PictureBox`), with its words ("No picture", "Load a picture", "Remove the picture") translated like the renderer's own. Records keeps album covers in the database and artist photos in a file store, verified live on Wails and Avalonia. File access in code (2026-10-18), agreed with the user: only the file stores of the settings, written like `find` (`files f in photos where f.extension == "jpg" order by f.path`), `read text of f`, `write t to exports "collection.txt"`, a file as a picture (linked by its path in the field's own store, copied elsewhere), `new Entity` for a new record, and the effects `files.read` and `files.write`; alike in the three runtimes through `conformance/files.json` (listing, refused paths). Records finds artist photos named after the artist and exports the collection, built on all three stacks, the Go actions run against a scratch store. On the way, a Rust loop that changed and saved its records did not compile (it borrowed them read-only); it now works on the records in the list, as Go and C# do. Still open: choosing a store's folder while the app runs (`-> folder photos`, remembered like the language), copying a picture from one record to another, the tree part. Agreed order (2026-10-18): G4 C and C++ next, before the folder picker and the tree part, so the C/C++ runtime catches up while the backlog is small; SQLite first there (it is C itself), the server databases after. |
| R1 | Resources and the markdown part: one `resources` folder per module folder (in place of `images`) holding the files an application carries, each with a name of its own; a key's value in a language can point at one (`gameRules resource "rules-nl.md"`), so a file per language goes through i18n; `markdown @key`, `markdown source "file"` or `bind`, the same three for an image; one Markdown subset on every stack; links open in the browser | done (2026-10-22): `clarity_spec::resources` (files, Markdown text by name; sources and keys checked by kind), `translations::Value` (text or resource) and `project.resource_keys`, the languages table with `takes` and a file per language; the generators make each resource key a text per language (the Markdown or the picture's name); `conformance/markdown.json` read by `markdownBlocks.ts` and `MarkdownBlocks` (Clarity.Avalonia `MarkdownView`); `open_link` (http, https, mailto) in every shell and the IDE; Resources under i18n in the project tree with a view per file, each of a type (`clarity_spec::resources::kinds`: Markdown, picture, PDF, drawing for SVG and DXF, other file), New resource (name, type, a file copied in) and Replace file; SVG as a picture is for later; the noughts rules as `resources/rules-en.md` and `rules-nl.md` |
| G3a | Go data and logic (2026-10-18: Wails v3, pinned at alpha.84, and modernc.org/sqlite, chosen by the user): the Go runtime (`runtimes/go/clarity`, copied into every generated module like the others) with `database/sql`, decimals from shopspring/decimal, dates as the runtime's own `Date` and `DateTime`, optional values as pointers; records, enums, functions, actions and data events in Go, where a call that can fail is lifted into its own statement with `if err != nil` (keeping `and` and `or` short-circuiting) rather than panicking; `conformance/queries.json` passing; the generated module builds with `go build` | done: the runtime (`Dialect`, `Query` and `Cond` with `IsNone`, `Db` with generic `Save`, `Delete`, `Find`, `All`, `Get`, `Load` and `Ref`, data events as optional interfaces on the record, `Reader` that keeps its first error, `Text` and `Slot` with the Rust runtime's wire format, errors as types, the SQLite driver on one connection) with Go tests: queries.json, the server scenario of migrations.json on SQLite, data events, text on the wire. The generator (`clarity-gen/src/go`): `go.mod` with the runtime's requirements and `go.sum`, the runtime as package `clarity`, a file per module (enums as string types with `Label`, records as structs with JSON names in snake case, defaults in `NewX`, methods on the pointer; functions and actions returning `error`, data events as unexported functions run by the record's methods), `schema.go`, `main.go` with `--check`, and a README. A call that can fail is lifted (`v1, err := ...`) and an `and` or `or` whose right side needs it becomes an `if`; unused locals get `_ = x`, imports only what a file uses. `tests/go_build.rs` (`--ignored`) generates both examples, finds nothing for gofmt and go vet to change, builds them and runs `--check`. Not yet: handlers and windows (G3b), so Build and Run still say Go is not built yet |
| G3b | The Wails v3 shell: the web renderer over Wails bindings with the same commands as the Tauri shell, handlers and windows, `in new window` as a second Wails window; Run and Build use `go build` | done: the runtime's package `shell` (a Wails v3 application whose one service answers the renderer's commands through `Invoke(ctx, command, args)`, with the Tauri shell's commands and JSON, the calling window from the context for `set_title`, `apart_start` and `close_apart`, a second Wails window for `in new window` that tells its opener when it closes, answers as `{"value"}` or `{"error"}`, a refusal keeping its slots) with Go tests that run the commands without a window; `clarity/ui.go` (window state, reply and `Ui` in the Rust runtime's JSON, `Opened` checking `with` against the window). The renderer reaches either shell through `app/src/lib/backend.ts` (Tauri's invoke, or Wails' `/wails/runtime.js` calling `main.Clarity.Invoke`, which `main.go` binds as its own type). The generator writes a struct per window and the app with its data, `loadX`, handlers as methods (part properties through `ui`, the selected row, open, close, reload, sibling handlers), and `main.go` with the dispatch tables, `//go:embed` of the renderer (shared with the Rust output through `web::renderer_files` and `web::model_json`) and model.json. Run and Build: `go build -o bin/<name> .`, a release with `-tags production -trimpath -ldflags=-s -w`; `clarity tools` counts Go as generated and asks for WebKitGTK on Linux. Verified live: the shop ran on Wails with its title from `set_title` ("Customer card"), its theme, frame and data; the Rust shop still runs through the shim. Not tried live: clicks (handlers and `in new window`), since this desktop does not tell which window is active; the shell tests cover the commands |
| G3c | Formats, translations and themes on Go | done: they live in the web renderer, which the Go output ships with the same model.json (one function for both web stacks) and theme CSS; the remembered choices persist in WebKitGTK's storage (`~/.local/share/<name>`) as on Tauri |
| G3d | The server databases and external sources for Go: pgx, go-sql-driver/mysql and go-mssqldb behind the same runtime calls | done (2026-10-18): a file per database in the runtime (`postgres.go`, `mysql.go`, `mssql.go`) that adds itself to `servers`, over `server.go`, which holds one `*sql.Conn` for the driver's life (the pool resets a connection between calls, which lost the transaction of a save with data events on PostgreSQL and SQL Server). pgx and go-mssqldb read the project's locations as they are; a MySQL URL becomes the driver's settings. The migrations.json scenario and the data event tests pass on SQLite, PostgreSQL 18, MySQL 8.4 and SQL Server 2022 in docker (`CLARITY_TEST_*`, as for the other runtimes). A generated module gets only the driver files of its data sources, and its go.mod and go.sum leave out the modules of the others; `go_build` checks with `go mod tidy -diff` that go.mod is what tidy would write, and builds the shop for each server database and migrates it twice. External sources needed nothing new: `Table`, `Key` and `Source` come from the entity, and only owned sources are in `schema.go` |
| G4 | C and C++: emitters, runtimes, webview shell; Qt as a native toolkit for C++ if wanted. The project is a CMake project (`CMakeLists.txt`, built with make or Ninja, the tools P3 checks for), with the README.md every stack gets | G4a (C++) done: the runtime `runtimes/cpp` (C++20; system SQLite 3 and nlohmann-json, webview 0.12.0 copied into `third_party`; errors as exceptions, its own `Decimal`, SHA-256 and base64; drivers in `unique_ptr`, conditions in `shared_ptr<const Node>`, SQLite handles closed by RAII; records as plain structs that meet a `Record` concept) with its own tests on the conformance files; the generator `clarity-gen/src/cpp` writes `CMakeLists.txt`, `CMakePresets.json` (`cmake --workflow --preset debug` or `release`), `src/records.hpp` and `.cpp` with every module's enums and records, `src/modules/<module>.cpp` with each module's functions, actions, data events and windows, `src/app.hpp`, `src/schema.cpp`, `src/page.cpp` with the web renderer inlined, and `src/main.cpp` with `--check`; the program is a native window (webview on WebKitGTK) that talks to the renderer through `clarityInvoke`. Both examples build and run (`cargo test -p clarity-gen --test cpp_build -- --ignored`). SQLite only; the server databases come later. Every stack now writes module files where they cannot overwrite its own files (`src/modules/` in Rust and C++, `Modules/` in C#, `<module>_module.go` in Go, which also keeps a module named `windows` from building on Windows only). G4b (C) done: the runtime `runtimes/c` (C17; system SQLite 3, cJSON 1.7.18 and webview 0.12.0 copied into `third_party`, the webview built from one C++ file). Every call that can fail returns whether it worked and leaves its error, as errno does; values live in a scratch arena the shell empties after each command, so generated code never frees anything (cJSON allocates there too). Records are plain structs described to the runtime by a `ClarityEntity` (table, columns, values, data events); `customer_save`, `customer_find` and the others are typed calls around it. 20 tests of its own on the conformance files, clean under AddressSanitizer and UBSan. The generator `clarity-gen/src/c` lifts each call that can fail into `if (!call(..., &v1)) return false;` (as the Go generator lifts into `if err != nil`), passes records by pointer, gives a function's result through its last parameter, and writes the same files as C++ in C (`records.h`/`.c`, `app.h`, `src/modules/<module>.c`, `schema.c`, `page.c`, `main.c`); the program builds without a warning under `-Wall -Wextra`, debug and release. Both examples build and run (`cargo test -p clarity-gen --test c_build -- --ignored`). `clarity tools` checks the C and C++ libraries too (SQLite, WebKitGTK, nlohmann-json for C++) and, for C, a C++ compiler for the webview. SQLite only; the server databases come later |
| H1 | Hardware, ideas raised by the user (2026-10-18), not decided. (a) A desktop application that talks to a device such as an Arduino over a serial port: the settings name the device as they name a file store (`serialport arduino "env:ARDUINO_PORT"`), code sends and receives with an effect like `device.write`; every stack can (Rust `serialport`, Go `go.bug.st/serial`, C# `System.IO.Ports`, C `libserialport`). (b) Code that runs on the device itself: Arduino sketches are C++, so a firmware target generated from a small part of the language, without a database or windows | idea |
| N1 | Engines, first slice (section 5.11): `engine Name from "file.wasm"` with stateless functions on plain values, enums, records and lists; the `engines` folder; the engine interface (JSON over plain memory); the host checks the file at start; hosted on the backend of all five stacks (wasmi, wazero, wasm3, Wasmtime for .NET); a shared test engine and `conformance/engines.json` | done (2026-10-20): see "Built" in section 5.11. The designer reloads when the `engines` folder changes, lists the engines in the project tree (marking one that does not fit its file) and shows their functions in the module view |
| N2 | Engines written in Clarity (section 5.12): N2a `engine Name` with what it keeps and its functions, transpiled per stack; then `record` declarations; N2b with N5 beside the screen | N2a done (2026-10-21) |
| N3 | Imports and limits: `uses clock, random, log, progress`, time and memory limits per call, failures shown as refusals | done (2026-10-20) but `progress`: see "Built (N3)" in section 5.11 |
| N4 | State and hooks: `state per window` and `state per app`, hooks such as `on open` and `on close`; `bytes` for document processing | done (2026-10-20): state, `on open` and `bytes`, see "Built (N4)" in section 5.11; no `on close` (the engine has nothing it could do then) |
| N5 | Engines beside the screen (section 5.12): N5a brought along, in a Web Worker and in Avalonia, the frame loop, input, tells and hears; N5b written in Clarity (`on tick`, `on draw`, `on pointer`, `on key`, `tell`) | |
| N6 | Engines in more languages and outside the IDE, so hand-written logic never collides with generation (RT1). Already there for Rust (`clarity_project::engines`): the source as a crate in `engines/<name>/` on the `clarity-engine` library, made by Clarity with a stub per declared function and given stubs for functions added later, edited, built and rebuilt before Build and Run when it changed, compiler messages as problems at their lines, Rust's WebAssembly target checked like a stack's tools; any other language is a `.wasm` file only, which Clarity checks and leaves be. What N6 adds: building engines from the command line (`clarity engines build`, and `clarity watch` rebuilding on save), so someone working in VS Code needs no cargo command of their own; libraries like `clarity-engine` and templates for more languages (C and C++, Go through TinyGo or `go:wasmexport`, Zig, AssemblyScript; C# only once .NET makes plain WebAssembly modules well), each with its build tools checked. Open: the WebAssembly Component Model with WIT interfaces instead of the JSON interface, which brings glue for many languages but is heavier and unevenly supported by the five runtimes. Open, not planned: widening the sandbox with more `uses` services (`uses http`, `uses serial` for H1), each one implemented by every stack's host; it lets engines reach more of the platform, but every service costs five hosts and makes an engine less of a pure computation that is the same everywhere and tested with plain values. Raised by the user (2026-10-23) | |
| P | Progress of a long engine call on the backend (section 5.12): `uses progress`, a bar and Stop on the window while the handler runs, through each shell's way to the screen | |
| DG1 | Diagrams, first slice (section 5.13): a Diagrams part with a `diagrams` folder (not carried into applications), the Data model and Classes views generated from the project, written `.mermaid` ERD and class diagrams checked by `clarity_spec::diagrams`, the diagram designer (mermaid.js with Clarity's layer over it: toolbox, properties, click through, relationships drawn by dragging, Design and Text tabs), differences marked before Apply, Apply as one undo step with label keys and the migration step, and the agent's diagram tools | |
| DG2 | State and sequence diagrams (section 5.13): `steps` between enum values as a language construct, checked and enforced on every stack, designed as a state diagram; a sequence diagram per handler as a view | |
| DG3 | Activity diagrams per handler or function and the architecture of the project, as views (section 5.13) | |
| W1 | The web target: each backend serves the web renderer over HTTP; Avalonia in the browser through WebAssembly | |
| A1 | AI through the MCP server: the whole project and its output readable, changes only as designer edits, proposals inside a project and for Clarity itself (section 1.2) | |
| A2 | Local models for Clarity's agent, so a person can work without a cloud account or sending the project anywhere: connection kinds beside `anthropic`, first `ollama`, then `openai` for any OpenAI-compatible endpoint (llama.cpp server, LM Studio, vLLM), each a `Model` in `clarity-agent` beside `anthropic.rs`. A local connection needs a base URL and a model but no API key; fields a kind does not use (`cacheTtl`) are not asked for. Testing a connection checks that the model answers and can call tools, and says so when it cannot. The tools, the permission step for Build and Run and the turn budget stay the same for every kind. Tested against a faked server per kind. Raised by the user (2026-10-23) after comparing with Embarcadero's Kai, which offers local models | |
| RT1 | Round tripping, so a developer can open the generated project in VS Code, change it with any AI, and keep designing in Clarity afterwards. Logic that only computes goes into an engine first (N6), which generation never touches; RT1 is for code that needs the platform: libraries such as a NuGet package, OS features, native UI. In this order: (a) seams per file: generated files and files the developer owns side by side in each stack's idiom (partial classes in C#, separate files in Go, Rust, C and C++, beside `src/modules/`), an owned file written once and never again, as the stub rule of section 11 did before behavior moved into the Clarity language; (b) hand-written items: a window, function or action marked as hand-written in the spec keeps its signature generated but not its body, the designer shows it as owned by code, and the stack's compiler reports a body that no longer fits the spec; (c) a three-way merge as the safety net for a generated file edited anyway: every generation writes a stamp, `clarity-generated.json` at the root of the generated project (the Clarity version, a hash of the spec, and a hash of every generated file), so Clarity sees which generated files were edited by hand and which output they started from; it keeps that last generated output, merges old to new generated into the edited file, and shows conflicts in its own merge view. Every generated project gets an `AGENTS.md` and `CLAUDE.md` saying which files are generated and which are owned, and that new logic belongs in an engine, so other AIs edit the right ones. Reading hand edits back into the spec is not planned, apart perhaps from layout later. Raised by the user (2026-10-23) | |
| S4 | Graphics canvas (section 5.7): `bitmap` and `vector` field types, the standard canvas widget with view and edit modes, the `canvas` screen pattern, backend and frontend renderers | |
| M0 | Workspace, `clarity.toml`, Rust pack, index, modularity rule, `clarity check`, severities | |
| M1 | Rest of the spec language: actions, contracts, expression language, formatter | |
| M2 | First stack end to end: Rust backend, Tauri shell, Svelte frontend, SQLite storage, `clarity gen --verify` | replaced by G1 and D1, apart from `clarity gen --verify` |
| M3 | TypeScript pack, TDD state machine, Claude Code hooks, protected rules file, shape-in-spec rule, directive integrity | |
| M4 | Contracts to property tests and runtime checks, DRY, mutation testing for Rust and TypeScript, remaining screen patterns | |
| M5 | Second stack to prove the generator split: Go + Wails + Vue | replaced by G2 and G3 |
| M6 | Metrics, waivers, ratchet mode, `clarity adopt`, `clarity tune` | |
| M7 | Packs for C, C++, Python, C#, Ruby. Backend generators for C# and C++ | generators replaced by G2 and G4; packs remain for adopted projects |
| M8 | LSP server (spec and rules), MCP server | |
| M9 | Desktop app: control room, with the designer from S3 | |
| M10 | Web target (HTTP adapters, fetch transport, PostgreSQL), C backend generator, React frontend, external packs, Haxe | web target and generators replaced by W1 and G4, PostgreSQL by D1; external packs remain |

## 20. Open questions

1. **Name.** "Clarity" is already the name of the smart-contract language of the Stacks blockchain. Keep it, or pick another name (and file extension) before publishing anything.
2. **Spec syntax.** Indentation-based as in the examples (the S1 parser uses this), or braces. Indentation reads better; braces are easier to edit safely for tools and AI. Hyphenated names such as `master-detail` do not lex as one word yet; either allow hyphens in keywords or spell it `master_detail`.
3. **Expression language scope.** Is the set in section 5.4 enough for real-world contracts, or are quantifiers over related entities needed?
4. **Migrations.** How far automatic migration generation goes (renames and type changes need human input), and how migration files are reviewed.
5. **Reports.** Which report engine or format (PDF, HTML, CSV) the `report` screen pattern targets.
6. **TDD task boundaries.** How a task starts and ends for an AI agent: explicitly through `clarity test --task`, or inferred from the agent session.
7. **Test-to-symbol mapping** for the `red` state: coverage data from the failing test (exact but slow) or static call analysis (fast but approximate).
8. **Index storage** in the working tree (`.clarity/`) or a user cache directory, and what gets committed (baseline and metrics: yes; index: no).
9. **Monorepos** with several `clarity.toml` files: nested projects or one root config.
10. **Mutation testing cost.** Running it on every CI build may be too slow for large changes; possibly sample or limit to changed functions.
