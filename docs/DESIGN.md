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
    // sqlite, postgres, mysql, mssql or rest; secrets only as env:, keychain: or setting: references
    datasource main postgres "env:SHOP_DB_URL"
    datasource payments rest "https://api.example.com/v2"
        auth bearer "keychain:payments"   // bearer, api_key or basic
```

The text after the kind is a file path for SQLite (`data/shop.db`), a connection for the SQL servers and the base URL for a web API. Passwords, tokens and connection strings with secrets never appear in the project file; it only says where to find them (`env:`, `keychain:`, or `setting:` for a secret setting of the person, 5.27), and the checker refuses a connection that contains a password. A new project gets one SQLite source named `main`.

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
| Graphics | `image` (`source "file"`), `canvas` (`surface`, see 5.7) |

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
- **Frontend renderer:** for real-time graphics, where a round trip per frame is too slow. It runs in the renderer with typed input events and cannot call the backend directly. As built, this is an engine beside the screen (see below); graphics drawn by an outside library are a custom part (section 5.19).

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
| `on unconnected: handler` (app) | the databases did not open; the handler may take why as text (5.27) |

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

- **Compiled to WebAssembly (decided 2026-10-26; it was transpiled per stack from 2026-10-21).** An engine is a WebAssembly file, whoever wrote it. Clarity compiles an engine written in Clarity when it generates the application, as `<name>.wasm` among its engine files, speaking the engine interface as one built from Rust does: `clarity_alloc` and `clarity_free`, JSON values in and answers out, the services as imports, an export per function, `clarity_open` for `on open`. From there nothing knows how it was written: every backend host and both renderers run the file, so it behaves alike on every stack by construction, and the generators write no engine code of their own. No toolchain is needed: the compiler is Clarity's (`clarity-wasm`), adding the engine's functions to a support module Clarity carries (memory, JSON, texts, lists, exact decimals, random), which keeps the runtimes' meaning against the shared conformance files.
- **Keeps.** `keep name: type = value` is what an instance keeps between calls. State works as for any engine: none gives every call the values as written, `state per app` one set, `state per window` one set per open window, dropped when it closes (each runtime's window-closed listeners).
- **A call changes nothing when it fails.** A call works on a copy of what the engine keeps, and the support module keeps that copy only when the call succeeds.
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
- **Back to the window: tells.** What the engine has to say to the application (the game is over, with its score) it tells: the answer of any of its calls carries `"tells": [{"tell": "finished", "values": [1200]}]`, and the window's handler wired to that tell runs on the backend as any handler does, with the values as its parameters (`handler gameOver(score: integer)`).
- **From the window: hears.** A handler sends the engine an order with `sky.start(2)`; the reply carries it to the renderer, which calls the engine's export of that name at the start of the next frame.
- **Where it runs.** On the web stacks in a Web Worker of its own, with the browser engine's WebAssembly: the worker draws nothing itself, it answers shapes, and the page shows them. A worker can be stopped from outside, so the limits hold here too: a frame that takes longer than its limit ends the engine, and the canvas shows the failure. On Avalonia in Wasmtime in the same process, on a timer at the screen's rate, stopped by its epoch. (Decided 2026-10-20: a worker, not the page's own thread, where an engine that loops would freeze the window.)
- **Written in Clarity too (decided 2026-10-26).** Since a Clarity engine compiles to WebAssembly, one beside the screen needs no words of its own: what it can do is what any engine can do. The frame loop is functions of the interface's names (`function clarity_tick(ms: integer)`, `function clarity_draw() -> list<Shape>`, `clarity_pointer`, `clarity_key`), what it draws is data it returns (records of `conformance/drawing.json`), a hear is a function of its name, and `tell finished(time)` is a statement that puts a tell in the call's answer.
- **Services beside the screen** are the renderer's: the clock and random from the page or the process, the log into the application's log through the shell. Progress has no meaning there.

Built (N5a, the frame loop, 2026-10-26). The language: `beside the screen` on an engine with a file (`EngineDecl::beside`), `runs Engine` on a canvas (`WidgetProps::runs`, `WidgetProperty::Runs`); checked: what a canvas runs is an engine beside the screen, such a canvas has no draw handler, the engine has no functions and no state line and is not written in Clarity yet, its file exports `clarity_tick` and `clarity_draw` (`engines::SCREEN_INTERFACE`), and one no canvas runs is a warning. The frame calls are the engine interface's: `clarity_tick` takes `[ms]` (0 first), `clarity_draw` takes `[]` and its value is the shapes of conformance/drawing.json; a refusal ends the engine as a failure does. The model carries each one as `Preview::screen_engines` (the file in base64, uses, opens, limits); the backend generators leave them out (`crate::engines`). The canvas says why its engine stopped in Clarity's words, `SCREEN_ENGINE_KEYS` (`screenEngineFailed` with `{problem}`). The web: `engineHost.ts` (the engine interface and the services with the browser's WebAssembly, run against conformance/engines.json too; memory checked after each call, since the page cannot hold a module's own memory back), `screenWorker.ts` (inlined into runtime.js, so the C and C++ pages carry it), `screenEngine.ts` (`ScreenRun`: requestAnimationFrame, a frame skipped while busy, the time limit per call by a timer that terminates the worker, the worker's own errors; the engine is sent as a plain copy, since a reactive proxy cannot cross to a worker) and `ui/EngineCanvas.svelte`. Avalonia: `IFrames` and `EngineException` in Clarity.Runtime, `ScreenEngine` in Clarity.Runtime.Engines (one instance per canvas, `EngineDeclaration.Screen`), `FrameLoop` and `Shape.Of` in Clarity.Avalonia, `CanvasView.Runs` (a loop while it shows) and `Stopped`; the generated `ScreenEngines.cs` gives a `Func<IFrames>` per engine, embedded with the engines' files. Tests: `clarity-spec/tests/screen_engines.rs`, `clarity-gen/tests/screen_engines.rs`, `engineHost.test.ts`, `screenEngine.test.ts`, `ScreenEngineTests`, `FrameLoopTests`, and the ignored build tests `an_engine_beside_the_screen_builds_in_go` and `_in_csharp` on conformance/engines/play.clarity; tried live on Go and Avalonia. Not yet (the log was built after, below): the designer's field for `runs`, and engines beside the screen written in Clarity (N5b).

Built (N5a, pointer and keys, 2026-10-26). `clarity_pointer` and `clarity_key` are optional exports: an engine that has them hears that input (`Engine.hears` on the web, `IFrames.Hears` on Avalonia), one without is sent none, and the language needs no word for it. What happens between frames waits and goes at the start of the next, in order, before `clarity_tick`; each call keeps its own time limit (the worker says which call it starts, `calling`, in place of the old `ticked`). Pointer: `down` (primary button or touch on the drawing), `move` (over the drawing, or anywhere while down), `up`, in the canvas's units rounded down as a click is; while down the canvas holds the pointer (pointer capture on the web, `Capture` on Avalonia) and gives the nearest edge beside the drawing (`pointNear`, `CanvasView.PointNear`). Moves in a row keep the last (`queue`, `ScreenInput.Queue`). Keys: a fixed set of names alike on both stacks (`keyName` from `KeyboardEvent.key`, `ScreenKeys.NameOf` from Avalonia's `Key`), sent down once while it repeats (`HeldKeys`), all let go when the canvas loses focus; the canvas is focusable only when its engine takes keys and takes focus when pressed; a key with Control, Alt or Meta stays the window's. Shared cases: conformance/screen-engines.json `input`, `queued`, `near`, `keys`, with screen.wasm dragging its dot and naming the keys down. Tried live on Go (WebKitGTK) and Avalonia. NuGet 0.41.0 for the changed `IFrames`.

Built (N5a, tells and hears, 2026-10-26). The language: `tells name(params)` and `hears name(params)` lines on an engine beside the screen (`EngineDecl::tells`, `hears`, as a custom part's `PartMessage`s, plain values only); on any other engine they are an error. A canvas that runs it wires its tells as a custom part does (`on finished: gameOver`, `WidgetProps::tells`), checked against the engine's tells and the handler's parameters (`TellOf::Engine`); `sky.start(2)` on such a canvas is a hear (`make_hears`), checked against the engine's hears; the file exports a function per hear (`engines::attach`). Every stack's backend already answered a tell's handler with its values (`given_values_handlers`) and sent hears with the reply, so the generators changed only for Avalonia's canvas, which binds its `State`. The engine interface: any answer may carry `tells`, collected per frame and handed over once it is drawn, in order; a tell during `on open` is dropped. Hears queue with pointer and keys (`{hear, values}`), kept even before the engine is ready, and run the export of their name with its own time limit. The web: `Engine.frame` gives `{shapes, tells}`, `ScreenRun.hear` and `onTell`, `CanvasHears` (a hear told before the canvas shows waits for it; an `on open` handler's reply comes first), `Live.screen` in Runtime.svelte, tells run through the custom parts' `told`. Avalonia: `ScreenInput.Hear`, `ScreenTell`, `Drawn` (what `IFrames.Frame` gives now), `Engine.CallIn` with a tells sink, `FrameLoop.Hear` and its `told`, `CanvasView.State` (listened to while it shows) and `Told`, wired by the generated code as a custom part's. Shared cases: conformance/screen-engines.json `tells`, a refused hear in `fails`, and screen.wasm telling `lap` and `finished` and hearing `start`; conformance/engines/play.clarity is a small game with both. Tried live on Go and Avalonia. NuGet 0.42.0.

Built (N5a, the log, 2026-10-26). A line an engine beside the screen logs goes into the application's log, the standard error, where the backend's engines write (`engine Screen: <line>`). On Avalonia the engine runs in the process, so Wasmtime's `log` already wrote there (`ScreenEngine.Load` takes the services now, for tests). On the web stacks the worker hands the line to the page, and the page to the shell with the command `log_line` (`{"line": text}`, `logLine` in backend.ts; in the designer it stays in the console): the Go shell (`Service.logLine`), the generated Tauri command, the C shell (`clarity_service_on_log`) and the C++ shell (`Service::log_line`) write it to the standard error. Shared case: conformance/screen-engines.json `uses` and `logs`, with screen.wasm `uses log` writing `level N` when it hears `start(N)`. Tried live on Go. Go runtime v0.31.0 for the shell's command.

Built (N5a, the designer, 2026-10-26). A canvas's properties have an **Engine** field (`WidgetProperty::Runs`): none, or an engine beside the screen (`besideEngines` in engines.ts). Picking one sets `runs` and unwires the draw handler (the code stays) and any tell the new engine lacks (`runEngine`); no engine unwires every tell. While it runs one, the canvas offers no draw event (`canvasEvents`), its engine's tells are wired in the same **Tells** section a custom part has, and a hint names the hears code can call (`sky.start(level)`). The written line reads back as it was (`the_designer_writes_a_canvas_that_runs_an_engine`).

Built (N5b slice 1, 2026-10-26). `crates/clarity-wasm`: `compile(project, engine)` gives the engine's WebAssembly file. The support module (`crates/clarity-wasm/support`, Rust built to `support.wasm`, which the crate carries) has the engine interface and the `rt_*` functions: a call's arguments and answer, values in an arena emptied when the call ends (a whole number is an i64 and yes or no an i32 in the compiled code, any other value a handle), lists as values (setting an item gives a new list), what the engine keeps (made on its first call, a call working on a copy kept only when it succeeds), texts and refusals. The compiler (`emit.rs`) writes per function an inner wasm function and an export that reads the JSON arguments, calls it and hands over the answer, from the checker's type of each expression; it stops a call at the first failure, asking `rt_failed` after each statement. `merge.rs` re-encodes the support module with the engine's functions, exports and texts added (a data segment where its memory ended). Compiled now: whole numbers, yes or no, texts and text with slots, lists of them, `let`, assignment (an item of a list too), `if`, `for`, `return`, `refuse`, `in`, `contains`, `count` and the engine's own functions; anything else is refused by name ("not compiled yet"). Shared cases: `conformance/clarity-engines.json`, run through the Rust runtime's host.

Built (N5b slice 2, 2026-10-27). Decimals and money (the support module holds them as rust_decimal, so a division writes as many places as it gives: 7 / 2 is 3.50, as on the Rust stack), arguments read by their declared type (`rt_arg_dec`, `rt_arg_decs`), arithmetic mixing whole numbers and decimals, `round`, `sin` and `cos` as the runtimes have them, `require` (`condition failed: ` and the condition as written, through `code::print`, moved from clarity-gen to clarity-spec), a text by key (it gives its key, which the host translates as any engine's `says`; a key with values is not compiled yet) and `random(n)` from the imported service. The services an engine `uses` are imported from `clarity`; the support module's functions move up past them (`merge::Shift`). Not yet: `ensure`, texts with slots translated (today they give their text as written), the project's own functions called from an engine, records.

Built (N5b slice 3, 2026-10-27). Generation compiles every engine written in Clarity first (`compile_engines` in clarity-gen): its file becomes `<name>.wasm` among the project's engine files, its body goes and the keys its code gives join its `says`, so each generator takes it as any engine file (embedded on Rust, Go and C#, bytes in `src/engines.c` or `.cpp`). The five generators no longer write engine code (their `emit_engine`, `written_call`, kept structs and `random` went); the runtimes' helpers for it went too (`random` and splitmix64 in Rust, Go, C, C++ and C#, C's `clarity_*_keep` and `_drop`), with the `random` cases of conformance/lists.json, whose rule conformance/clarity-engines.json now holds. The expression printer moved to `clarity_spec::code::print`. Tests: `clarity-gen/tests/clarity_engines.rs` (every stack carries board.wasm and writes no engine code), `clarity-wasm/tests/noughts.rs`, and the ignored build tests `an_engine_written_in_clarity_runs_in_{go,c,cpp,csharp}`, which play the compiled Board through each stack's engine host (wazero, wasm3, wasm3, Wasmtime); all pass.

Built (N5b slice 4, 2026-10-27). Records: `record Name` with fields (`SpecFile::records`, an `Entity` with `record`; parsed with the entity's fields, a decimal without precision and scale), found by the checker wherever an entity is (`Env::entity`), never saved, deleted or found, fields only values, lists of them as values (`[a, b]`, an empty one typed); for now only engines use them (a record in other code is an error until the generators write records). The support module holds a record as its fields in order, a JSON object across the interface (`rt_record`, `rt_with`, `rt_field*`); the compiler writes `new` with each field's default, reads and sets fields. `tell finished(time)` (`StmtKind::Tell`, checked against the engine's tells, only beside the screen; `rt_tell`). An engine beside the screen written in Clarity: `clarity_tick(ms: integer)`, `clarity_draw() -> list<Record>`, a function per hear, no `on open` (`check_written_screen`); generation compiles it as any Clarity engine, so the model and Avalonia carry the `.wasm`. `ensure` compiles too (the statements a function of their own, the ensures checked on its result). Shared case: conformance/engines/sky.clarity compiled to sky.wasm (kept in step by `clarity-wasm/tests/sky.rs`, `CLARITY_RECORD_SKY=1` writes it), screen-engines.json `written`, run by the web host (`engineHost.test.ts`), the Avalonia host (`ScreenEngineTests`) and the Rust host; the ignored build test `a_clarity_engine_beside_the_screen_builds_in_go`, tried live on Go (Start sends the hear, the tell fills in the time).

**Compiled or interpreted, chosen per project (decided 2026-10-27, user's idea; built the same day).** Both give the same kind of file, so the hosts never know. The checked code is lowered once to an IR (`crates/clarity-wasm/ir`, `clarity-engine-ir`: every name resolved to a local, a kept value or a function, every operation saying what it works on), from which `emit.rs` writes wasm (compiled) and the interpreter in the support module runs it (interpreted). The support module's value operations are written once (`support/src/values.rs`): the `rt_*` functions of compiled code and the interpreter (`support/src/interp.rs`, the `interpreter` feature, built to `interpreter.wasm`) both call them. An interpreted file is `interpreter.wasm` with the engine's IR as JSON in a data segment and per function an export that calls `rt_run(ir, its number, the call's arguments, the table entry of random)`: the interpreter cannot import a service only some engines declare, so the host's `random` is added to the function table (in the oldest form of element segment, which wasm3 reads) and called through it. The project file says `engines interpreted` (`Settings::engines_interpreted`, the IDE's **Engines** choice in Project settings); compiled without it. Tests: every case of conformance/clarity-engines.json, the noughts Board and Sky run both ways through the Rust host; screen-engines.json `written` lists sky.wasm and sky-interpreted.wasm for the web and Avalonia hosts; the ignored build tests `an_interpreted_engine_runs_in_{go,c,cpp,csharp}` play the interpreted Board on wazero, wasm3 and Wasmtime; all pass. The `pills` example (Go) is a game like Arkanoid whose engine beside the screen is written in Clarity and interpreted: records for its shapes, lists of what it keeps, pointer and keys, tells and a hear; built by `the_pills_example_builds_and_runs_in_go` and tried live.

Built (2026-10-27, after the interpreter, the user's asks for the pills game). **Pictures on a canvas:** the shape `image(x, y, width, height, picture)` (conformance/drawing.json; `ShapeValue::Picture` in the shape table, a name of the resources folder checked where it is written out, `named_problem`), drawn by the web renderer as an SVG `<image>` through the pictures loader and by Avalonia through `ResourcePictures`. **SVG is a picture** (`PICTURE_EXTENSIONS`; a drawing is DXF only): the web shows it as it is, Avalonia through Svg.Controls.Skia.Avalonia (MIT), which Clarity.Avalonia now depends on; the image part on Avalonia is `c:ResourceImage` (by name) or `c:ResourcePictures.Named` (by key), both SVG-aware. Avalonia's shape reader takes a number written as text, as an engine gives a decimal, as the web's already did. **Constants:** `const name = value` or `const name: type = value` in a module or an engine (`SpecFile::consts`, `EngineBody::consts`, `ConstDecl`), a value written out, never set (`NameKind::Const`), an engine's own first; `picture` is text the resources check finds (`const pill is 'pill.svg', which ...`); generation writes each use out as the value with places of its own (`clarity_gen::consts`), a whole number in a decimal constant as a decimal, and the engine lowering does the same for engines compiled on their own; a constant's keys count as used, its plain text is no literal. Tests: `tests/consts.rs` in clarity-spec and clarity-gen, constant cases in clarity-engines.json (both modes), the image case in drawing.json on the web and Avalonia, and the ignored `pictures_build_in_csharp`. The pills example draws its pills and paddle from SVGs named by picture constants. NuGet 0.44.0 for Clarity.Avalonia.

#### In order

1. **P**: progress and Stop for engine calls on the backend, on all five stacks and both renderers, with `progress n of total` if that is chosen.
2. **N2a**: `record` declarations and engines written in Clarity, on the backend, built by Clarity into the engines folder.
3. **N5a**: engines beside the screen, brought along: the frame loop, input, tells and hears, in a Web Worker and in Avalonia.
4. **N5b**: Clarity engines compile to WebAssembly, on every stack (replacing the transpiled engines of N2a), in slices: (1) the compiler `clarity-wasm` and its support module, with functions of plain values, `let`, `if`, `for`, `return`, `refuse`, run through the engine conformance files; (2) keeps and state, lists, texts and keys, decimals and money, the services; (3) Clarity builds the file into the engines folder and the five generators treat it as any engine file, the noughts example on every stack; (4) records and `tell`, so an engine beside the screen is written in Clarity, the game of conformance/engines/play.clarity among them.

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
- **Kept per person, on their machine,** never in the project's database: in the folder for application settings of the person's system (`%APPDATA%`, `~/Library/Application Support`, `$XDG_CONFIG_HOME` or `~/.config`), in a folder named after the application, as `settings.json`. That name is the project's name in snake case (`ShopDesk` keeps `shop_desk/settings.json`), the same on every stack and for the command-line program (`clarity_gen::settings_program`, conformance/secret-store.json; one form since 2026-10-25, when Go said `shopdesk`, C# `ShopDesk` and some command-line programs `shop-desk`). A setting that is not kept yet has its `default`, or the empty value of its type; a kept value that no longer fits (a setting removed, an enum value gone) is dropped.
- **Bound like data, everywhere.** Every window has `settings` as data of its own, without a `data` line: a part binds `settings.pageSize`, a form binds `settings`, and code anywhere reads and sets `settings.pageSize`. A change takes effect at once: the person changing an input bound to a setting keeps it, as does a handler setting it, and every open window sees it. Setting `settings.language` or `settings.theme` switches the language or the theme, as the pickers do; a dropdown bound to them offers the project's languages by their own names and the offered themes by their titles.
- **The pickers stay** as a shortcut: a menu item `on click -> language` changes the same `settings.language`.
- **Every stack.** The spec gives the settings as a record type of their own, `Settings`, beside the entities: it has no table, no migrations and no place in diagrams, and no entity can be called `Settings`. The generators write it as they write a record, without its database side, and give every window's data its `settings`. On the web stacks the renderer holds the settings: it reads them from the shell when the application starts, sends them with every window's data to the backend, takes them back from every reply, and has the shell keep them when they changed (the shell commands `read_settings` and `keep_settings`; Rust's comes from the runtime crate, Go's, C's and C++'s from their shells). On Avalonia the runtime's `Settings` keeps them, and the generated view models share one instance.
- **The designer** offers a "Settings window" template that writes a dialog with a form of the declared settings, labeled, as dropping fields writes one.

### 5.18 Usability rules (2026-10-23, a design)

Raised by the user on 2026-10-23: the agent should design applications that are pleasant to use, led by a set of rules rather than by taste. The same rules serve the person in the designer. Nothing here is decided beyond that aim.

Usability guidance that is already written down and widely used is the source, so no rule is Clarity's invention:

- Nielsen's ten usability heuristics (status shown, undo and a way out, consistency, preventing errors, recognition over recall).
- The laws of interaction design: Fitts (large, near targets are quicker to hit), Hick (more choices take longer), Miller (few items held in mind at once).
- WCAG 2.2 for accessibility: contrast, every input labeled, everything reachable by keyboard, a minimum target size.
- The platform guides of the stacks' toolkits (Fluent for Avalonia on Windows, Apple's Human Interface Guidelines, GNOME's) for button order, spacing and the wording of dialogs.

**Rules are checks on the spec.** A window is a spec, not loose code, so most rules can be tested by the checker, as it tests types. A broken rule is a warning with the rule's name and a line on why, never an error: an application that breaks one still builds and runs. Each rule has a name, so a person can turn one off for a part, a window or the project with a reason written next to it. How that is written is open (question 11 in section 20).

**One set for the agent and the designer.** The agent already checks the project after each write and sees its problems, so it sees these warnings too and fixes them as it fixes errors. Its instructions carry the rules' names and what each wants, read from the same table the checker uses, so the two cannot drift apart. The designer shows the warnings on the canvas next to the part, as it shows problems now, with a fix where one is clear.

A first set, each a check on what the spec already holds:

| Rule | Warns when | Source |
|---|---|---|
| `confirm-delete` | a handler runs `delete` with no `ask` before it on the way there | Nielsen: error prevention |
| `say-what-happens` | a button's caption is a bare "OK", "Yes" or "Go" outside a dialog, or a dialog's buttons do not name the action ("Delete album", "Keep") | platform guides |
| `label-every-input` | an input has no `label for`, no form label column and no placeholder | WCAG 1.3.1, 3.3.2 |
| `keyboard-order` | `taborder` values skip back against reading order, or two inputs share one | WCAG 2.4.3 |
| `default-and-cancel` | a dialog has no `default` button or no `cancel` button | platform guides |
| `long-form` | a form or window holds more than about 12 inputs without groups, tabs or steps | Miller, Hick |
| `long-list` | a table without `page` binds a query with no limit, or a dropdown over a reference has no `search` | Hick |
| `empty-state` | a table or list shows nothing when empty (no text for an empty list) | Nielsen: system status |
| `slow-work-shown` | a handler calls an engine or reaches a server database with no `busy` or progress on the window | Nielsen: system status |
| `small-target` | a button or icon button is smaller than the minimum target in the running size | WCAG 2.5.8, Fitts |
| `contrast` | a theme's text and background colors fall below 4.5:1, in light or dark mode | WCAG 1.4.3 |
| `menu-has-key` | a common command (New, Open, Save, Delete, Find) in the menu has no `key` | platform guides |

The thresholds (12 inputs, the target size) are settings of the rule, with these as defaults.

**Later, from use.** The checks above see the design only. A running application could also note where people struggle: clicks on things that do nothing, repeated clicks on one part, errors shown again and again, forms given up halfway. Kept only on the person's machine and only when they agree, such notes could reach the designer as suggestions on the parts concerned. That is a second step (UX2) and needs its own decisions on privacy.

**Every stack.** The rules look at the spec, so they are the same on every stack and need no runtime release. Only `small-target` and `contrast` depend on how a renderer draws; they use the sizes in `clarity_spec::layout` and the theme's colors, which both renderers share.

### 5.19 Custom parts (2026-10-23, decided)

Raised by the user on 2026-10-23: a developer should be able to put a part in a window that an outside library draws, such as a game scene drawn with SFML, a map or a code editor. A custom part is a link to that library. Every part is Clarity's own today; the early sketch of custom widgets (`widget ColorPicker`, from before the designer) was never built and is replaced by this.

**In the project's own language.** A part is written in the language of the project's stack, with that language's own libraries and packages (decided with the user, 2026-10-23): a project that picks a stack picks its parts' language. The library line says which:

| Stack | Library line | The part is |
|---|---|---|
| C, C++ | `native "parts/Arena"` | C or C++ with its `CMakeLists.txt`, using SFML, SDL, raylib or anything CMake finds |
| Rust | `crate "parts/Arena"` | a Rust crate with its `Cargo.toml`, using any crate |
| Go | `go "parts/arena"` | a Go package with its `go.mod`, using any module |
| C# | `nuget "Package" version` or `dll "file"` | an Avalonia control from that package or assembly, through a C# adapter |

A native, Rust or Go part draws into a native child window the host gives it at the part's place; a C# part is an Avalonia control in the window itself. A part whose library line does not fit the project's stack is a problem at the declaration, so changing stack shows each part that must be written again.

**The declaration is Clarity.** What the rest of the project sees is declared in a module, so the checker, code, the Parts palette and the properties panel treat the part as they treat a built-in one, without knowing what is inside it. The words are the ones engines already use for talking with the screen (section 5.12):

```clarity
part Arena
    native "parts/Arena"
    binds integer
    speed: integer, min 1, max 10, default 3
    tells scored(points: integer)
    hears restart(level: integer)
    size 8 x 6

part Notes
    nuget "AvaloniaEdit" 11.1.0
    binds text
    tells caretMoved(line: integer)
    size 6 x 4

page Game: window
    data player: Player
    Arena name arena at 1,1 span 8 rows 6 bind player.best speed 5 on scored: arenaScored
```

- **The library line** says where the library is: `native "folder"` (built by Clarity, below) or `nuget "Package" version`, `dll "file"` for .NET.
- **`binds`** names the type of value the part shows and edits (any field type, or none). The part reports a change by the person as a built-in input does, so `on change` and saving work as they do for a text box.
- **Properties** are written as settings are (`name: type` with `default`, `min`, `max`), of the plain types. Each is set in the window (`speed 5`), shown in the properties panel, and read and set in code (`arena.speed = 7`). `visible` and `enabled` come for free, held by the host around the part; a native part is told when `enabled` changes.
- **`tells`** are the part's own events with typed values: a handler is wired to one (`on scored: arenaScored`) and receives them as its parameters, as with an engine's tells.
- **`hears`** are commands code gives the part: `arena.restart(2)` in a handler, carried to the part, which runs it.
- **`size`** is the span and rows the Parts palette drops it with.
- A part is placed by its name, written as declared (`Arena ...`), so it never clashes with a built-in keyword, which is always lower case.

**The part's interface, in its language.** Clarity writes one file of the part's folder from the declaration, and writes it again whenever the declaration changes; the part implements what it asks, and one that no longer fits the declaration fails its build:

- C or C++: `parts/Arena/clarity/Arena.h`, the functions below.
- Rust: `parts/Arena/src/clarity.rs`, the trait `ArenaPart` that the crate's `Arena` implements, the `Window` it draws in (with the raw handle wgpu and Bevy take; a Bevy app runs without its own window loop and updates once a frame), and `ArenaHost` with a method per tell. The file needs nothing of Clarity's: the crate depends on `raw-window-handle` only.
- Go: `parts/arena/clarity.go`, the interface `Part` that the package's `New(window, width, height, host)` makes, the `Window`, and `Host` with a method per tell; the package's module needs nothing of Clarity's either.
- C#: the generated half of the adapter, below.

In C and C++:

```c
ArenaPart *Arena_create(void *parent, int width, int height, ArenaHost *host);
void Arena_resize(ArenaPart *part, int width, int height);
void Arena_frame(ArenaPart *part);
void Arena_destroy(ArenaPart *part);
void Arena_set_value(ArenaPart *part, int64_t value);
void Arena_set_enabled(ArenaPart *part, bool enabled);
void Arena_set_speed(ArenaPart *part, int64_t speed);
void Arena_restart(ArenaPart *part, int64_t level);
/* Given by the header, called by the library: */
void Arena_changed(ArenaHost *host, int64_t value);
void Arena_scored(ArenaHost *host, int64_t points);
```

- `parent` is the native child window the part draws into: an X11 window on Linux, an `HWND` on Windows, an `NSView` on macOS. SFML takes it as `sf::RenderWindow(handle)`.
- An integer is `int64_t`, a bool `bool`; every other value (text, decimal, money, date, an enum) is a `const char *` in its plain form, as the command line prints it (section 5.20).
- The host calls everything on the UI thread. `Arena_frame` comes about 60 times a second while the part is shown, for a part that animates; a still part leaves it empty. The child window gets the mouse and keyboard itself, so the library reads them as it would in its own window.
- A tell or change the library reports is queued and goes to the backend as an event, after the call it came in.

**Built with the program.** A C or C++ part's `CMakeLists.txt` makes a library named after the part, which the program's own CMake build takes in; a Rust part's crate is a path dependency of the program's; a Go part's module is required by the program's and replaced by its folder, its requirements and sums joining the program's. Declaring a part writes a starter in its language that builds without other libraries (it keeps its values and draws nothing). Clarity never writes over a file of the part's folder after the first, except its interface file.

**A .NET part's adapter.** The control in the package is not Clarity's, so a small C# class sits between them: `parts/Notes/Notes.cs`, the developer's half of a partial class, holding the control and passing values to it and from it. The generated half (on Clarity.Avalonia's `DotNetPart`) calls a partial method the adapter must write for the value (`ShowValue`), each property (`Show<Property>`) and each hear (`On<Hear>`), so the C# compiler reports an adapter that no longer fits its declaration, and gives the adapter a method per tell and `Changed(value)` to call. Declaring the part writes a starter adapter.

**Above the page.** A native child window lies over the window's own drawing, so nothing of the window can show on top of it. While a Clarity dialog, popover, menu or open dropdown overlaps the part, the host hides the part, and shows it again when that closes. A .NET part is a real Avalonia control and has no such limit.

**On the design canvas** a part is a box with its name and size, or a picture of it (`preview "arena.png"` in the declaration, from the resources). Preview in the IDE shows the box too: the library is loaded only by the built program. Run shows the real part.

**On Linux** a C, C++, Rust or Go part needs X11 for its child window, as SFML does. A program that holds one starts under X11 (XWayland on a Wayland desktop).

**Checked.** A part's declaration is checked like an entity's; a window that sets an undeclared property, wires an undeclared tell or calls an undeclared hear gets an error; a missing library, one whose build fails or a .NET part on another stack is a problem at the declaration; the usability rules (section 5.18) apply to its properties as to a built-in part's.

**Not a sandbox.** An engine is a pure computation the host can stop; a custom part is the developer's library running in the program with its rights, as the owned files of RT1 are. Logic that only computes still belongs in an engine. A part receives data and tells; it does not call the backend, and the host gives it no way to.

**The canvas's `renderer Name`**, parsed and shown in the properties panel but generated by no stack, goes: a canvas is drawn by its draw handler or `runs` an engine (N5), and graphics drawn by an outside library are a custom part.

### 5.20 Command-line applications (2026-10-23, built)

Decided with the user on 2026-10-23 (roadmap C1). A project can also be a program without a window: commands typed in a terminal, over the same records, actions and settings as its windows.

```clarity
project Shop
    stack go web desktop cli        // the windowed application and a command-line program
```

```clarity
cli Shop
    help @shopHelp

command customer add(name: text, email: text)
    help @addCustomerHelp
    option vip: bool
    option limit: money, default 1000, help @limitHelp
    let c = new Customer
    c.name = name
    c.email = email
    c.credit_limit = limit
    save c
    print @customerAdded(name = c.name)

command customer list
    help @listCustomersHelp
    option active: bool
    let found = find c in Customer where c.active == active order by c.name
    print found columns name, email

command customer remove(name: text)
    help @removeCustomerHelp
    let found = find c in Customer where c.name == name
    if count(found) == 0
        refuse @noSuchCustomer(name = name)
    if ask @removeIt(name = name) yes @remove no @keep
        delete found[1]
```

    $ shop customer add Ada ada@example.org --vip --limit 500
    $ shop customer list --active
    $ shop help customer add

**The target.** `cli` is a target beside `desktop`: `stack csharp avalonia desktop cli` gives both programs from one project, and `stack go cli` gives only the command-line program; with `cli` as the only target the UI word is left out. The output has one folder per target, always: `out/desktop/` and `out/cli/`, each a complete project for its stack's toolchain, so a project with one target has one folder in `out/`. A program served to a browser alone (the web target, section 7.3) is in `out/web/`; with `desktop web` the one program does both, in `out/desktop/`. Until now no generator read the targets; from C1 on they decide what is generated.

**The `cli` block.** One per project with the `cli` target, named after the project, as `app` is: `help @key` is the program's description in its help. The program's name is the project's name in lower case with hyphens (`Shop` gives `shop`).

**Commands** are declared in any module, their body written inline as a function's is:

- **The name** is one word or a group and a word: `command customer add` is `shop customer add`, `command backup` is `shop backup`. A name written in camelCase is written with hyphens on the command line (`addAll` is `add-all`). A group is no declaration of its own; it is there when a command names it, and the help lists its commands.
- **Arguments** are the parameters in the signature, given in that order, all required. **Options** are indented `option name: type` lines, written as settings are (`default`, `min`, `max`, `required`, `help`), and given as `--name value` in any order; a `bool` option is a flag (`--vip`), false when it is left out. An option without a default that is left out has the empty value of its type, as a setting does. The types of both are the plain types of code: text, integer, decimal, money, date, datetime, duration and enums (an enum by its value's name); a `bool` is only an option. A record is no argument for now; a command looks it up with `find`.
- **`help @key`** on a command and on an option is its line in the help. Every command and option needs one (a warning, as an unlabeled input is).
- **What a command sees:** its arguments and options by name, `settings` (the same ones the windows keep, read and set), the functions and actions of its module and the modules it uses, engines, queries, `save`, `delete` and files. It has no window, so `open`, `close`, `reload` and part properties are errors in a command, as `print` and `exit` are outside one.

**The statements of a command:**

| Statement | Meaning |
|---|---|
| `print expr` | Writes one line to standard output: a text (a key, with slots), a number, a date. Values are written as the command line reads them (`2500.5`, `2026-10-23`, an enum value by its name), the same on every stack and in every language, so scripts can read them back. |
| `print list columns a, b` | Writes the records of a list as a table: a heading line of the fields' labels, then a line per record, the columns aligned. |
| `message expr` | Writes a line to standard error, whatever its level: a terminal shows no levels of its own. |
| `ask question ...` | An expression that waits for the answer and gives it: `if ask @removeIt yes @remove no @keep`, `let n = ask @howMany for integer default 3`, `let k = ask @which for CustomerKind`. The forms are those of a handler's `ask` (section 5.14) without `then`; a record is asked `for Entity from list` as a numbered choice. |
| `exit n` | Ends the program with exit code n (0 to 255). |
| `refuse expr` | As everywhere: the text goes to standard error, the program ends with exit code 1, and a save or delete it is part of does not happen. |

**Exit codes.** 0 when a command finishes; 1 when it refuses or fails (a database that cannot be opened, a failed save); 2 for a wrong command line (an unknown command or option, a missing argument, a value that does not fit its type or its `min` and `max`), with the reason and the command's usage on standard error; `exit n` for anything else.

**Asking without a person.** When standard input is no terminal (piped, in a script), an `ask` with a `default` gives the default and one without ends the program with exit code 2 and a line saying what it needed. Ctrl+D or an empty answer to a question without a default is a no, or ends with exit code 1 for a value.

**Texts.** Everything the program writes is a key, as in windows. The words of the help and of its own errors ("Usage", "Commands", "Options", "unknown option", "needs a value") are Clarity's own keys, as the renderer's `ok` and `cancel` are, listed explicitly in the spec and present in every language file of a project with the `cli` target. The language is `settings.language` when the project has one kept, else the system's locale (`LANG`, `LC_ALL`; the user's language on Windows), else the project's `language`.

**Every stack.** The command-line parser, the help writer, `ask` on a terminal and the table of `print` are written once in each runtime and checked against one shared file, `conformance/command-line.json` (command lines in, parsed values or exit code and message out; help texts; tables), so the five behave alike. No outside parser library is used.

- **Rust:** `out/cli/` is a crate with a binary on `clarity-runtime`, without Tauri; the runtime gains a `cli` module.
- **Go:** `out/cli/` is a module with `main.go` on the Clarity runtime, without Wails; the runtime gains `cli.go`. Its `go.mod` lists the runtime's requirements as the desktop module's does, Wails's among them, so `go mod tidy` would trim it (open).
- **C#:** `out/cli/` is a console project on Clarity.Runtime only, without Avalonia; Clarity.Runtime gains `Cli.cs`, and the settings file's code (`KeptSettings`) moves to it from Clarity.Avalonia, as the other stacks keep it in their runtime, so both packages need a version.
- **C and C++:** `out/cli/` is a CMake project without webview; the runtimes gain `cli.c`/`cli.h` and `cli.cpp`/`cli.hpp`, copied as the other runtime files are, and `clarity.cmake` builds the shell only where its files are copied.
- The records, queries, actions, migrations and settings code is the same generated code the desktop program has, written into each folder.

**In the IDE.** The project tree has a Commands section listing each command by its command line (`customer add`); choosing one opens its module at it. For a project with the `cli` target, Run asks which program to run when there are two, and the command line in Clarity's dialog, and shows what the program writes in the Output panel, where a line typed answers its `ask`: Clarity starts it with `CLARITY_TERMINAL=1`, which every runtime takes for a terminal. The Project tab offers `cli` among the targets, and shows no UI choice for a project whose only target it is.

**Examples.** Two new example projects test it, each built only on its own stack: `examples/ledger` (`stack csharp cli`, a command-line program alone: accounts and entries, groups, tables, a confirmed delete, exit codes) and `examples/stock` (`stack go web desktop cli`, a small stock list with a window and a command line over the same records, which tests `out/desktop` and `out/cli` side by side). "All examples" is eight from then on.

### 5.21 External libraries (2026-10-23, built)

Decided with the user on 2026-10-23, by the same rule as custom parts (section 5.19): a project calls an outside library written in its own stack's language, with that language's own package. The library is declared in Clarity with the functions code calls, and an adapter in the project's language writes them with the package.

```clarity
library Words
    nuget "Humanizer.Core" 2.14.1
    function spell(n: integer) -> text
    function rounded(amount: money, places: integer) -> money

action describe(order: Order)
    effects db.write
    order.words = Words.spell(42)
```

| Stack | Package line | The adapter in `libraries/Words/` | Clarity writes |
|---|---|---|---|
| C# | `nuget "Package" version`, `dll "file"` | `Words.cs`, the other half of a static partial class | `Words.g.cs`: a partial method per function |
| Rust | `crate "name" "version"` | a crate whose `Words` implements the trait | `src/clarity.rs`: the trait `WordsLibrary` |
| Go | `go "module" version` | a module whose package has the functions | `clarity.go`: each function's type, checked |
| C, C++ | `cmake "Package"` | a CMake target `Words` (its `find_package` lines) | `clarity/Words.h` or `.hpp`: the functions |

- **Calls** are written as an engine's: `Words.spell(42)`, checked against the declaration. Values in and out are text, a number, a date, a duration or bool, in the language's own types (`long` and `decimal` in C#, `i64` and `Decimal` in Rust, `int64` and `decimal.Decimal` in Go, `std::string` in C++, `const char*` in C).
- **Checked:** a library needs its package line, of the project's language (another language's is an error at the declaration, as for parts), and its adapter's file; an adapter that no longer fits the declaration fails the build.
- **Built with the program:** the package is the C# project's `PackageReference`, the crate a path dependency of the program's, the Go module required and replaced by its folder (its requirements and sums joining the program's), the CMake target a subdirectory linked into the program. Declaring a library writes a starter adapter (`clarity parts` from the command line, Build and Run in the IDE).

### 5.22 Sound (2026-10-24, built)

Decided with the user on 2026-10-24: a built-in part that plays the project's sound files and makes tones and melodies, on every stack. It has nothing to show: it is a window's part without a place in its grid, as a timer was on C++ Builder's forms, and the designer keeps it in a tray under the window.

```clarity
page Table: window
    sound name rollSound source "roll.wav" volume 80
    sound name tune

code Table
    handler rollClicked
        rollSound.play()
        tune.melody("C4 E4 G4 C5*2", 140)
```

- **Properties:** `source "file"`, a sound of the resources folder; `volume 0` to `100` (100 unless given); `repeat`, which plays the source over and over until `stop`.
- **What code tells it**, as it tells a custom part (5.19): `play()` plays the source from its start; `stop()` stops everything it plays; `tone(hertz: integer, seconds: decimal)` plays a tone; `melody(notes: text, tempo: integer)` plays notes at `tempo` beats a minute; `volume(level: integer)` sets the volume from 0 to 100.
- **Notes:** words apart by spaces. A note is a letter `A` to `G`, then `#` or `b`, then an octave `0` to `8` (4 unless given): `C4`, `F#3`, `Bb`. A rest is `-`. A note or rest lasts one beat, `*2` two beats, `/2` half a beat. A melody written as text in the code is checked; one made while the program runs plays its notes and skips words that are not notes. `conformance/melody.json` holds the notes every stack reads alike (each note's frequency and length).
- **Sound files** are a kind of resource every stack plays: `.wav`, `.mp3` and `.flac`. A `source` that is not one is an error that lists the sounds there are.
- **Web stacks:** the renderer plays with the Web Audio API: files from the resources next to it, tones and melodies as oscillators.
- **C# (Avalonia):** Clarity.Avalonia's `Sound` plays through SoundFlow (MIT, miniaudio for Windows, macOS and Linux, a NuGet package), files from the embedded resources; tones and melodies are made into WAV data in memory and played alike.

### 5.23 HTTP calls (2026-10-24, first slice built)

Asked by the user on 2026-10-24 (roadmap HC1): code calls a web API, so a Clarity application can be an API client, sync with a service, or send a webhook. The first slice is the call and its reply as text; reading a reply's JSON into records, and web APIs as data sources (D2), come after it.

```clarity
action send_order(order: Order, token: text)
    effects net
    let reply = http post "https://api.example.com/orders" with order.summary header "Authorization": "Bearer {token}"
    if not reply.ok
        refuse @orderNotSent(status = reply.status)

code Status
    handler refreshClicked
        let reply = http get "https://status.example.com/api/now"
        statusText = reply.text

    handler sendClicked
        send_order(order, settings.apiToken)
```

- **The call** is an expression: `http get url`, `http delete url`, `http post url with body` and `http put url with body`. The url and the body are text. `header "Name": value` adds a header, as many as needed, the name a text as written and the value any text.
- **The reply** has `status` (an integer: 200, 404, and 0 when no answer came), `ok` (true for a status from 200 to 299) and `text` (what the service answered, or why no answer came: `cannot reach status.example.com: ...`). Its type is `http reply`. A call never stops the handler: a refused connection, a wrong name or a timeout is a reply with status 0, so code decides what a failure means, with `refuse` or a message, as it does for any other answer.
- **Where:** in handlers and commands, and in actions that declare the effect `net`. A function, an engine and a data event cannot call out: a function is safe anywhere, an engine keeps to itself, and a data event runs inside the save's transaction, which must not wait on the network.
- **Keys stay out of the code.** A header whose name is `Authorization`, or holds `key`, `token` or `secret`, with a plain text as its value is a warning: a key belongs in a setting (5.17) or comes from the environment, and reaches the header through a slot.
- **Only http and https.** Any other url is a reply with status 0 that says so. Every call has a timeout of 30 seconds; redirects are followed.
- **The window waits** while a handler calls out, as it waits for any handler, until background work is planned (N5, P).
- **Each stack its own client:** Rust `ureq` (blocking, rustls; the runtime's feature `http`), Go `net/http`, C# `HttpClient` in Clarity.Runtime, C and C++ libcurl. A C or C++ project links libcurl only when its code calls out, so projects without calls need nothing new.
- **conformance/http.json** holds the calls every runtime makes against a small test server of its own (the routes are in the file) and the replies they must give: a page, a missing page, an echoed body and header, and a server that is not there.
- **JSON, the second slice (2026-10-25, built).** `json of value` gives the JSON of a record, a list of records or a plain value, as Clarity writes values everywhere (the engine interface's JSON: decimals and money as exact text, enums by name, dates as `YYYY-MM-DD`, dates and times as `YYYY-MM-DDTHH:MM:SS`), so a post sends a record: `http post url with json of order header "Content-Type": "application/json"`. `read Order from json text` and `read list<Order> from json text` read records, `at "data.orders"` starting inside the JSON. A service's JSON is not Clarity's own, so each runtime fits it to the entity before its decoder reads it: names match also ignoring case, underscores and hyphens (`creditLimit` is `credit_limit`), numbers and strings of numbers are taken for decimals and integers, a date and time keeps its date where a date is wanted, an enum value matches in any case; what the JSON leaves out or gives as null keeps its default, other names and `id` are ignored (the records are new; `save` adds them). A reading that does not fit stops the code with a message that says where, as a refused save does. `conformance/json.json` holds the fitting every runtime does, word for word. Reading is pure, so a function may read JSON too; an engine does not. Built on all five stacks: `ExprKind::JsonOf` and `ExprKind::ReadJson`, `clarity-gen/src/json_fields.rs` (each entity's fields and what they take, handed to the runtime), `clarity_runtime::json`, Go `clarity.ReadJSON` and `ReadJSONList`, C# `JsonValues` in Clarity.Runtime, C `clarity/json_read`, C++ `clarity/json_read`, each passing conformance/json.json; `tests/http_build.rs` reads a service's list and sends a record on every stack, with the same results.
- **An OpenAPI description in the project (asked 2026-10-25, D2, decided with the user: schemas become entities, a new revision updates them, and the project picks what it uses; built).** A web API data source names its description and what it uses: `openapi "forms.yaml"` (a file of the project's `apis` folder) and `pick "listTemplates", ...` (operations) with `pick schema "Entry", ...` (schemas on their own), indented under its `datasource` line. Clarity writes `apis/<source>.clarity` from them (`clarity_project::refresh_apis`, `clarity_project::openapi`): `module forms` with `source forms`, an entity per schema the picked operations take or give (fields from the properties: text, integer, decimal, bool, date, datetime, enums of identifier values, lists of plain values, and since section 5.25 records and lists of records held inside), and an action per picked operation in plain Clarity (`http get "/templates" from forms query "status": status`, `refuse @...` when it is not ok, `read list<Template> from json reply.text`), with keys for its texts in the language files. It writes it again when the project opens, when the description, the project file or the module changes, and before every build; only a changed text is written, so a new revision of the description updates the entities and actions, and a pick it no longer has is a warning. The IDE's **API...** beside a web API data source lists every operation and schema with a box to pick it (`api_listing`, `add_api_file`, ApiPanel.svelte). Entities in a web API are not stored (`Project::in_web_api`, `stored_entities`): no migration, no `find`, `save` or `delete`. `http ... from source` calls a path at the source's address with its auth, and `query "name": value` adds an encoded query parameter (conformance/http.json, on all five runtimes; `tests/http_build.rs` reads entities from a web API source on every stack). An optional query or header parameter is a parameter with a default, sent only when it differs (section 5.24): the description's own default, else `""`, `0` or `false`; a bool goes as `true` or `false`, not as a text slot would show it. Nested objects are records inside records (section 5.25). Not yet: patch, keys from the keychain in generated applications, paging, and the agent picking for itself.

### 5.24 Defaults, named values and `when` (2026-10-25, built)

Asked 2026-10-25 for the optional query parameters of an OpenAPI description, and decided with the user out of three (empty means left out; a record of options; defaults with named values).

- **A default** makes a parameter of a function or action optional: `limit: integer = 0`. It is a value written out (a number, a text, a bool, `none`, an enum's value), of the parameter's type, and the parameters with one come after those without. An engine's functions have none: an engine is called from other languages too.
- **A call** gives values in order, then any by name: `list_templates("published", limit: 10)`, `greeting(times: 2, name: "Bo")`. Each parameter once, every one without a default given; a value in order cannot follow a named one. Handlers and the built-in calls take their values in order only.
- **`when`** after a `header` or `query` pair of an http call leaves the pair out when its condition, a bool, does not hold: `query "limit": "{limit}" when limit != 0`.
- **Generation** fills every call in before a stack is written (`clarity_gen::calls`): named values put in their places, defaults copied in, so no target language needs default or named parameters. A copied default gets a place (span) of its own past the end of every file, as generation types expressions by their place. `when` is per stack: Rust a list of `Option`s flattened, Go and C a list filled in the lines before the call, C# a spread of an array or nothing, C++ a lambda that fills a vector; the runtimes are unchanged. `tests/http_build.rs` calls with a default left out, with named values, and with pairs left out, on all five stacks.

### 5.25 Records inside records (2026-10-25, built)

Asked 2026-10-25 for the nested objects of an OpenAPI description; decided with the user out of three (flatten to fields, keep JSON text, records inside records).

- **The type.** In an entity of a web API, a field named after another entity holds a record of it (`owner: User`, `FieldType::Record`, resolved like `list<Genre>` once every name is known), and `list<Line>` holds records inside rather than in a table. Code reads them as any record and list: `order.owner.name`, `for line in order.lines`. `FieldType::is_held` names both: neither is a column.
- **The rules** (`held_records`): only an entity of a web API holds a record (a stored one refers with `ref`); what it holds lives in the web API too; a record never holds itself, through any number of records, as C, C++ and Rust need a size for it. A list may hold its own kind, as it may be empty.
- **JSON** (conformance/json.json, kinds `record` and `records`, on all five runtimes): a nested object is fitted to its entity's own fields over a new record of it, a list each item alike; an error says where, `field 'lines' of Order, item 2: ...`. Generators describe the nested entity at the call (`json_fields::Nested`); a list that would repeat its entity inside itself is not read there. `json of` writes them nested, as each stack's records do.
- **Generation.** Each stack's record has the field (Rust and C# by value with a new record as default, Go with `NewUser()` and an empty slice, C and C++ by value, declared after what they hold: `holding_order`); C reaches a held record through a pointer to it, as all its records. Rust, Go and C# new runtime versions carry the nested kinds (crate 0.25.0, Go v0.27.0, NuGet 0.32.0); C and C++ copy theirs. `tests/http_build.rs` reads a basket with its buyer and lines on every stack.
- **OpenAPI.** A `$ref` property is a record inside, an array of `$ref` a list of them, and the schemas they name are written too, any depth down. One that would close a circle, an object without a schema of its own and a list of those are named in a comment, not read.
- **The IDE.** In a module with a `source`, the entity editor offers the module's other entities as types; ER and class diagrams draw a held record as one to one.
- **Built (2026-10-24):** the language (`ExprKind::Http`, the type `http reply`, the effect `net`, where a call may be, the key warning, `Project::calls_out`), the five clients (`clarity_runtime::http` behind the feature `http`, Go `clarity.HTTP`, C# `Http.Get` and the others in Clarity.Runtime, `runtimes/c/http` and `runtimes/cpp/http` on libcurl as the library `clarity_http`) each passing conformance/http.json, the five generators, and `tests/http_build.rs`, which builds the command-line program of `tests/fixtures/calls.clarity` on every stack and runs its calls against a test server. Next: a reply's JSON read into records, then web APIs as data sources (D2); background work (N5, P) so a slow call does not hold the window.

### 5.26 Live graphs: a chart of numbers, the trend and the timer (2026-10-25, built)

Asked 2026-10-25: the IDE's System graphs as a generic part. The user chose both forms: the chart's line of a list of numbers and a part of its own.

- **A chart of a list of numbers.** `chart line|bar ... bind readings` on a plain value `list<integer|decimal>` (or such a field) draws the numbers as they are, without groups; `points n` shows the last n, `max n` fixes the top of the axis; a pie or `by` is refused for it (`check_number_chart`). A line of more than 30 points has no dots.
- **The trend** (`WidgetKind::Trend`, palette Data) binds one number and takes its value every `every n` seconds (1), keeping the last `points n` (60): its own clock, so its axis is time, whatever code does. Web `ui/TrendView.svelte` on `ui/trend.ts` (sampled, numbersOf), Avalonia `TrendView`, both on the chart's drawing; the designer shows a still sample.
- **The timer** (`WidgetKind::Timer`, palette Time, `Event::Tick`) has no place, like the sound, and sits in the designer's tray. While its window is open and it is enabled (`clock.enabled`), it runs `on tick` every `every n` seconds (0.1 to 86400); a tick while a handler runs is left out. Web: Runtime.svelte keeps an interval per open window's timer and runs the handler on that window; Avalonia: `Clarity.Avalonia.Timer` on a DispatcherTimer with the tick command.
- **Lists of values in code and windows.** A window's data may be a list of text, integers, decimals or dates, starting empty; `list + [item]` is a new list (the checker wants lists of one kind on both sides). Each stack joins its way: Rust extends a clone, Go appends to `slices.Clone`, C# `Enumerable.Concat`, C `clarity_<kind>_join` (runtime, copied), C++ a lambda that inserts.
- Tests: `clarity-spec/tests/live_charts.rs`, `clarity-gen/tests/live_charts.rs` (a monitor window on every stack; ignored tests build it with cargo/Tauri, go vet, dotnet with warnings as errors, CMake for C and C++: all pass), `Clarity.Avalonia.Tests/LiveChartTests.cs`, `app/src/lib/trend.test.ts`. Clarity.Avalonia gets new controls: NuGet 0.33.0.

### 5.27 Secret settings (2026-10-25, built)

Asked 2026-10-25: a token for a web API, and later a database's connection, belongs in the application's settings, in a protected store, per application. A setting of type `secret` is the person's own, kept in the system's protected store, never in `settings.json`.

- **The type.** `apiToken: secret` is a setting (`FieldType::Secret`), only in `settings`: an entity's field cannot be one. It has no default, and only a password input shows it; a text box or a text part bound to it is an error. A settings window gives it a password input (`widget_for`). In code it is text: `settings.apiToken = value` keeps it, `""` forgets it.
- **The store.** Per program and per person: service = the program's name (the project's in snake case, as the settings folder: one form on every stack, all lower case, since the Credential Manager reads a target in any case but keeps a write apart per case), user = the setting's name (on Windows the generic credential `program:name`, its user the setting's name and its blob the value's UTF-8 bytes, alike on every stack so an application built on another stack finds the secret: conformance/secret-store.json; kept for this machine (local-machine persistence, not enterprise, which roams with the person's account so a secret forgotten could come back); Rust writes the Credential Manager through Win32 itself (windows-sys), as keyring's own is `name.program` in UTF-16 with enterprise persistence, and keeps keyring for macOS and the Secret Service, and C# writes UTF-8, fixed 2026-10-25 after the Windows test, crate 0.28.0 and NuGet 0.39.0). Not yet alike: on macOS go-keyring keeps the value as `go-keyring-base64:` and base64, which the other stacks read as it stands. Linux and the BSDs use the Secret Service over D-Bus, macOS the Keychain, Windows the Credential Manager. Rust `clarity_runtime::secrets` (crate `keyring`, feature `secrets`); Go `runtimes/go/secrets` (`go-keyring`, set as `clarity.Secrets = secrets.System{}`); C# `Secrets` (Tmds.DBus.Protocol, CredRead and CredWrite, the Security framework); C and C++ `secrets/` (gio, wincred, Security), turned on with `clarity_secrets_use()` / `clarity::secrets_use()`. An application without a secret setting gets none of it: no feature, no Go module (`SECRET_MODULES`), no copied files or `clarity_secrets` link.
- **Reading and keeping.** The settings runtimes take the secret names: Rust `read_with`/`keep_with`, Go `ReadSettingsWith`/`KeepSettingsWith`/`CliKeepSettingsWith`, C# `KeptSettings.Read(program, secrets)`/`Keep(program, values, secrets)`, C `clarity_read_settings_with`/`clarity_keep_settings_with`, C++ `read_settings_with`/`keep_settings_with`. The web screen sends the names with `read_settings` and `keep_settings` (`secretSettings` in `settings.ts`).
- **Where a secret signs or connects.** The project file names a secret setting as `setting:name`: a web API's `auth bearer "setting:apiToken"`, and a server database's connection, `datasource main postgres "setting:shopDb"`. The checker wants a secret setting of that name (`check_web_sources`); a SQLite file is no secret. Generation writes it as `setting:<program>/<name>` (`with_secret_settings`, `with_secret_locations`), which the runtimes read from the store: an HTTP call signs with it (empty when not kept), and a database connects with it or fails with "the secret setting shopDb is not kept yet". C and C++ store records in SQLite only for now, so their connection reading waits for server databases.
- **When the databases do not open** (asked 2026-10-25, with a way for the designer to take over, so not every application does the same). The application starts anyway, its windows on an empty database in memory, and shows first what keeps the connection. The app's `on unconnected: handler` (`Event::Unconnected`, an app event) runs with why (`handler notConnected(reason: text)`, the parameter optional) on empty app data, and what it opens shows; without it, or when it opens nothing, the first window that shows the secret setting (`Project::connection_window`, `shows_setting`) shows, saying why. A secret connection needs one of the two (checked); `on unconnected` also catches any other connection that fails, such as a server that is down. Closing that last window connects again when a setting changed while it was open, and the main window opens once it does (or the window shows again, saying the new why); closing it unchanged ends the application. `--check` still fails. Each stack in its own way: the web screen (`Runtime.svelte`: `start` answers `unconnected` with the handler, the window and why; a new `connect` command) over the Rust and Go shells (`open_databases`, `Connecting`; Go `shell.Options.Unconnected` and `Connect`) and the C and C++ shells (`ClarityShellOptions.unconnected` and `connect`, `shell::Options`); Avalonia shows the window in a desktop window of its own while the application stays up (`ShutdownMode.OnExplicitShutdown`), through `Clarity.Avalonia.Unconnected`, with the handler on a ViewModel never shown (`AppUnconnectedViewModel`, the app's frame and code, `RunUnconnected`). C and C++ keep records in SQLite, so for them it is a file that does not open.
- **Why, in the person's language; the title bar's close** (asked 2026-10-25, after trying the flow live). Why is in Clarity's own words with a translation key, as `cliUsage` and the other command-line words are (`translations::UNCONNECTED_KEYS`, needed by a project with a secret connection or an `on unconnected` handler): `unconnectedNotKept` "The database connection is not set yet" for a secret setting not kept, `unconnectedFailed` "The database did not open: {problem}" for anything else, its slot the runtime's own message without its kind (no `storage: `). Each runtime turns its error into that text (`clarity_runtime::unconnected_reason`, Go `clarity.UnconnectedReason`, C# `Unconnected.Reason`, C `clarity_unconnected_reason`, C++ `clarity::unconnected_reason`), with a not-kept error of its own (`Error::NotKept`, `NotKeptError`, `NotKeptException`, `CLARITY_NOT_KEPT`) whose message stays as it was for logs; conformance/unconnected.json holds the words and the cases. `start` and a failed `connect` send it as `{key, values}`; the screen shows it translated and hands the handler the translated text. Until the project gives a value, the English shows (the screen's `OWN_TEXTS`, and in C# the text's template). While unconnected, the title bar's close of the main window (Alt+F4 too) goes to the screen as the last window's close does: every window closes from the top (`Navigator.closeAll`), then the screen connects again or ends the application. Each shell holds back the system's own close only then (`closes_to_screen`, set by `start`, cleared by `connect` and `quit_app`): Tauri's `CloseRequested`, Wails' `WindowClosing` hook, and in C and C++ GTK's `delete-event`, the window procedure's `WM_CLOSE` on Windows and the window delegate's `windowShouldClose:` on macOS; each asks the screen through `window.clarityCloseRequested`. Avalonia's own close already ran this way. Whether the settings changed compares their values (`sameSettings`), not the order the backend sends them in.
- **Showing and copying a secret** (asked 2026-10-25: "passwords, secrets and alike should have an option (open eye/closed eye) to show it or copy it"). A password input has an eye that shows what is typed or hides it again (crossed out while shown, `aria-pressed` on the web) and a button that copies what is in the input, typed or not yet taken. Their names are Clarity's own words `showSecret`, `hideSecret` and `copySecret` (`translations::SECRET_KEYS`, needed by a project with a password input). Each renderer in its own way: the web `TextInput` with `SecretButtons` beside the input (the clipboard API, or a selection copied where a webview has none); Avalonia's `TextInput` in its `InnerRightContent`, with `RevealPassword` and the window's clipboard. Clarity's own vault fields have the same buttons (`SecretField`).
- Tests: `clarity-spec/tests/secrets.rs`, `clarity-project/tests/settings.rs`, `clarity-gen/tests/secrets.rs` (with ignored tests that build an application with `on unconnected` on every stack: all pass), the shells' tests (Go, C, C++), `Clarity.Avalonia.Tests/UnconnectedTests.cs`, `http_build.rs` (a web API signed with a secret on every stack, with `CLARITY_SECRET_STORE=1`), and each runtime's secrets tests against the real store with `CLARITY_SECRET_STORE=1` (Rust, Go, C#, C, C++: all pass on Linux with gnome-keyring).

### 5.28 Shortcut keys: what is active, and set by the person (2026-10-25, a design; the IDE first)

Asked 2026-10-25: keys belong to what is active in a view or a menu; menus change with the views, and a ribbon is a view too, so its buttons hold keys as well. And: a person using the IDE, or an application with the option on, sets the keys themselves. The IDE goes first, as the pilot of the model, before it becomes language.

- **Layers, the innermost first.** The part with the focus keeps its own keys (typing in an input, arrows in a table, the code editor's). Then the view that shows: its own buttons and the menu items or ribbon tab it brings. Then the window: its menu, toolbar and ribbon. Then the app's frame, in every window that does not replace it. A key is unique within one layer (an error); an inner layer may reuse an outer layer's key on purpose. A hidden or disabled command does not fire. A menu item and a ribbon button with the same target are one command and may share a key.
- **Ribbon and toolbar buttons get `key`**, as menu items and buttons have. A ribbon button's key works while the ribbon shows, whichever tab is open, as in Office.
- **A view brings its own commands.** A `view` page may declare a ribbon tab and menu items that join its window's frame while it shows and leave with it, their keys with them (Office's contextual tabs).
- **Set by the person, when the project says so.** Whether people may set the keys is a project setting (asked 2026-10-25: "shortcuts on/off is project setting"): the project file's line `keys editable`, a switch in Project, Settings. Off, the keys are the design's. On, the keys are maintained centrally: one keys window, Clarity's own, which the click target `-> keys` opens, lists every named command per layer with its key; pick one, press the new key, and it is taken; Reset puts one back to the design's, Reset all every one. Any named command can get a key, also one without one in the design. A key taken in the same layer offers a swap, naming the command that has it; a key an outer layer has is allowed, and the window says that this command goes first where both are active. The keys the system and the inputs need (typing keys, Tab, Enter and Escape) are refused, saying why. They are kept in the person's settings file beside the other settings, as command name to key and only what differs from the design, so a changed default in a new version still reaches whoever left it, and a command renamed or removed drops out. The window's texts are Clarity's own words with keys, as the pickers' are.
- **Every OS.** Keys are written with Ctrl; on macOS Ctrl is Cmd, in matching and in what is shown.
- **The IDE pilot.** One list of the IDE's commands, each with its name, its default key and its layer (global, or one of the views Project, Design, Preview and Vault, which are the ribbon's tabs); the ribbon buttons and the key handler both read it. A ribbon button shows its key in its tooltip and as `aria-keyshortcuts`. On Windows the main window turns off WebView2's own browser keys (`AreBrowserAcceleratorKeysEnabled`, `browser_keys.rs`), so F5, Ctrl+R, Ctrl+P, Ctrl+F and Alt+Left are Clarity's or nothing, never a reload, print or find; the manual's window keeps them. The person sets the keys in Clarity settings, Keys, kept with Clarity's other own choices; in the IDE it is always on, since its user is its designer. Defaults: global F1 (manual), Ctrl+1 to Ctrl+4 (the views), Ctrl+O (open project), Ctrl+Shift+N (new project), Ctrl+Shift+W (new window); Project F5 (reload), Ctrl+, (settings); Design Ctrl+Z, Ctrl+Y and Ctrl+Shift+Z (undo, redo), Ctrl+G (grid); Preview F5 (run again), Shift+F5 (stop), Alt+Left (back); Vault Ctrl+N (add), F2 (edit), Delete (delete), Ctrl+Shift+C (copy value), Ctrl+L (lock). Change password and Destroy vault get none: what cannot be undone is not one key away.
- **Built (2026-10-25), the first part of the language slice.** `key` on toolbar and ribbon buttons (`frame::validate_command`, the designer's Shortcut field). The checker: one key, one command, in a window's frame (with regions of its own; the app's alone with the app) and among a window's own buttons; a menu item and a button that open the same window or run the same handler are one command (`check.rs`, `check_page_keys`, `frame_keys`). The web: `frameKeys.ts` (`keyedCommands`: what shows and takes clicks, every ribbon tab) heard by `FrameView` on the window, after the window's buttons, which `WidgetView` now hears on the document; this also fixes a menu item's key, which only showed on the web. Avalonia: `Keys.Frame` on menu items and buttons and `Keys.FrameRoot` on the window's frame, which answers after the window's own HotKeys, finds a shut menu's items and a closed tab's buttons, and skips what is hidden or disabled. Tests: `clarity-spec/tests/frame.rs`, `app/src/lib/frameKeys.test.ts`, `FrameKeyTests`, `clarity-gen/tests/avalonia.rs`. Next: `keys editable` and the keys window, then views that bring their own commands.
- **Built (2026-10-25), the keys window.** The project file's `keys editable` (`clarity_project::Settings::keys_editable`, Project settings' switch), handed to the spec as `Project::keys_editable`; `-> keys` is a built-in target (`frame::BUILT_IN_COMMANDS`), an error without the line (`target_problems`, `frame::first_click_to`). The catalog of commands (`clarity_spec::shortcuts::catalog`: the app's frame under the place "", each window's own frame commands and named buttons that do something under its name, ids `name` and `Window.name`) travels in the model as `Preview::keys` where the project lets people set keys. The window's words are `translations::KEYS_WINDOW_KEYS`. The rules (assign with a swap within a place, reset that swaps back, the other places that have a key, refused keys) are shared as conformance/shortcut-keys.json: `app/src/lib/keys.ts` and C# `KeyRules`. The web: `appKeys.ts`, `KeysWindow.svelte`, the screen's `keysOf` in the live context (FrameView and WidgetView answer the person's keys), the person's keys kept with the settings as `keys` (also without a settings block); a toolbar or ribbon button's tooltip is its key now (it said "Opens Window" in English). Avalonia: `PersonKeys` (kept in the settings file, beside the generated `Settings.ToJson`), `Keys.Id` on commands (a button's HotKey and a frame command follow the person's keys), `KeysWindow`/`KeysView`, the generated `KeysCatalog.cs`, `PersonKeys.Start` at the start, `Keys_Command` (`ShowKeys`). The C# settings file is written without escaping (`Ctrl+K`, not `Ctrl\u002BK`). Tried live on Go and on Avalonia, and on Windows on every stack. The keys window keeps a message line of its own and a fixed width, and says "press the new key" there, so its buttons do not move when it says something (found on Windows, Avalonia). Not yet: on macOS, Ctrl in an application's keys is the Control key, not Cmd.

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

### 6.5 Custom parts in the output (2026-10-23, decided)

How each stack carries a custom part (section 5.19), in the stack's own language. The frontend still holds no business logic: a part receives data and tells, and validation stays generated from the dictionary and repeated on the backend.

- **A part host per runtime.** The runtime of a stack whose parts draw in native windows hosts them in its own language: `runtimes/parts` in C for the C and C++ stacks, `clarity_runtime::parts` for Rust (the `parts` feature), and the `parts` package of the Go runtime. Each makes a native child window for each part (an X11 window on Linux), places and hides it, calls the part's frame about 60 times a second, and passes values, hears and tells, on a thread of its own, so it needs nothing of the shell's main loop. They behave alike, each tested with a test part of its own.
- **Generated glue.** The program registers each kind of part with its host and turns the host's plain calls (a property by name, a hear with its values) into the part's typed ones, and its typed tells back: `parts/glue.c` built with the part into `libclarity_parts` beside a C or C++ program, which the shell loads; `src/parts.rs` in a Rust program; `parts.go` in a Go program.
- **Web stacks (Rust, Go, C, C++).** The renderer draws a placeholder where the part sits and tells the shell its rectangle (in the webview's pixels) whenever it moves, resizes, shows or hides, or a dialog, popover, menu or open dropdown covers it. It sets each property and the bound value on the part when the window opens and when a reply changes them, and gives it the hears a reply carries. A tell comes back from the shell as a renderer event and runs its handler as a question's answer does, with the tell's values. The shell gives the part host the webview's native window (on Linux the WebKitGTK view's X11 window) and passes the renderer's calls on, through its own commands.
- **Avalonia.** A C# part is an Avalonia control from its package, through its adapter: the package is a `PackageReference` in the generated `.csproj` (a DLL a `Reference` with its path), and the adapter and its generated half (on Clarity.Avalonia's `DotNetPart`) go into the generated project's `parts/<Name>/`. It is placed in the window's AXAML and bound to the view model like a built-in part.
- **Backends.** A custom part's properties are part state as `visible` is, held by name, and its hears go out with the part changes of the reply, so every runtime carries them with no code per custom part (`conformance/custom-parts.json`). A tell comes in as an event with values.
- **Owned, never written over.** The files of a part's folder are the developer's, except its interface file. The generated project lists them in its `AGENTS.md` and `CLAUDE.md` as owned (RT1), so another AI edits those and not the copies.

#### Windows (2026-10-24, done)

Custom parts drawn by a native library run on Linux with X11 and on Windows with Win32; on a system with neither the part host says so and the part's place stays empty. C# parts are Avalonia controls and need nothing of this. What follows is how it works on Windows 10 and 11 (roadmap CP2); macOS is a later piece of its own.

1. **The child window.** Each host makes a Win32 child window per part (`CreateWindowExW` with `WS_CHILD | WS_CLIPSIBLINGS | WS_CLIPCHILDREN`, a window class of Clarity's) in the parent the shell gives, above the WebView2 view (`SetWindowPos` with `HWND_TOP`), moved with `SetWindowPos`, shown and hidden with `ShowWindow`, ended with `DestroyWindow`. The renderer already reports places in device pixels; each program is per-monitor DPI aware so they match the screen.
2. **The host's thread pumps messages.** A Win32 window belongs to the thread that made it, and a child made on another thread than its parent joins the parent's input queue. The host already wakes for every frame, so it answers its windows' messages there: `flush`, which sends what was asked on its way on X11, drains the queue with `PeekMessage` on Windows. The part's window gets its mouse, keys and paints, and the shell is never kept waiting.
3. **One platform piece per host,** beside the X11 one, behind the same few calls (open, child, move, show, destroy, flush): `platform_win32.c` in `runtimes/parts` (CMake picks it on Windows; the C and C++ shells load `clarity_parts.dll` with `LoadLibrary` where they `dlopen` on Linux); a `cfg(windows)` platform in `clarity_runtime::parts` on `windows-sys`; a `windows` file in the Go runtime's `parts` package on `golang.org/x/sys/windows`, without cgo.
4. **The shells give the parent HWND:** Tauri's `WebviewWindow::hwnd()`, Wails v3's `NativeWindow()`, and the webview's own window (`WEBVIEW_NATIVE_HANDLE_KIND_UI_WINDOW`) in the C and C++ shells. It is the shell's top-level window, so the part's window is a child beside the webview's, not inside it. `GDK_BACKEND=x11` stays a Linux setting.
5. **What a part gets.** C and C++: `parent` is already a `void*`; on Windows it is the HWND, which SFML takes as `sf::RenderWindow(static_cast<sf::WindowHandle>(parent))`; the header says so per system. Rust: `Window` carries the Win32 handle (`hwnd`, `hinstance`) on Windows and gives a `Win32WindowHandle`, so Bevy and wgpu draw as on Linux. Go: `Window` gets one `Handle` (the HWND on Windows, the X11 window on Linux) in place of `XID`, which changes the Go interface.
6. **The webview must clip its siblings.** A window without `WS_CLIPSIBLINGS` paints over whatever shares its place however the two are ordered, and the webview of every shell is such a window: the part is drawn and then painted over, so its place looks empty though its window is there, visible and first in the z-order. Before it puts a part among them, each host walks the parent's own children and gives every one that lacks it `WS_CLIPSIBLINGS`. This is what makes a part show at all on Windows.
7. **Unchanged:** the renderer, the overlays that hide a part (a native child window always sits above the web view), tells, hears and values, and the glue.
8. **Tests.** The host tests keep their window-less platform. The round-trip tests (a test part in each language tells, hears and is set) still run in Xvfb on Linux; a GitHub Actions job on `windows-latest` for C, C++, Rust and Go is open. Verified by hand on Windows 11: the dice example (Rust, Bevy) and the orbs example (C++, SFML from MSYS2) draw in their part's window, and the waves example (Go) with them.
9. **Versions.** The Rust runtime went to 0.21.0 and the Go runtime to v0.23.0, which is a breaking release: Go's `Window` now carries one `Handle` (the HWND on Windows, the X11 window on Linux) in place of `XID`, as point 5 says. The C host travels with the generator, so it needs no version of its own. Go v0.23.1 makes the host's two Win32 callbacks once (Go keeps every callback for good and allows about 2000), where v0.23.0 made one per part window.
10. **The webview's own SDK.** The webview a C or C++ program shows builds against Microsoft's WebView2 SDK headers, which no MSYS2 package carries. `clarity.cmake` fetches the SDK into the build folder once and puts its include folder on `clarity_shell`; the loader is built into `webview.h` (`WEBVIEW_MSWEBVIEW2_BUILTIN_IMPL`), so no import library is needed, and the WebView2 runtime itself ships with Windows 11. Without this, C and C++ do not build on Windows at all.
11. **Open, where CMake looks.** A C or C++ project on Windows finds SQLite and nlohmann-json only when CMake is told the prefix its compiler belongs to (`CMAKE_PREFIX_PATH=C:/msys64/ucrt64` for MSYS2's UCRT64). `tool_env` already hands a build the chosen `CC` and `CXX`; it must hand it that prefix too, or a build started from Clarity fails where the same build from a terminal works. This is the Environment's to give, since it is the Environment that knows which copy of each tool was chosen (EN1).
12. **Open, Visual Studio's build environment (C and C++ on Windows, also without custom parts).** The Environment settings find Visual Studio's compiler (`cl`), its CMake and its Ninja (EN1), but `cl` compiles only inside Visual Studio's environment (`INCLUDE`, `LIB` and `PATH` as `vcvars64.bat` sets them), so `CC=cl` with the Ninja or Makefile generators fails on its own. Two ways: (a) on Windows the generated `CMakePresets.json` uses CMake's Visual Studio generator, which finds the compiler and sets that environment itself (multi-config: the program lands in `build/<preset>/Debug`, so Run and the README follow); (b) Clarity runs each C and C++ build inside that environment, read once from `vcvars64.bat` (`cmd /c "vcvars64.bat && set"`) and kept per Visual Studio installation. (a) keeps the generated project buildable without Clarity from a plain terminal; (b) keeps one generator on every system. MinGW and Clang from MSYS2 need neither, and are what Windows was tried with: one prefix for the C compiler, the C++ compiler, CMake and Ninja, never a mix of two.
13. **Paths for other tools.** A path Clarity writes for another tool to read (a SQLite location or file store in generated code, cargo's `--config`, `CC` and `CXX`, the files the agent is shown) goes through `clarity_project::slashed`: plain (without Windows' verbatim prefix, `clarity_project::plain`) and with `/` between its folders. Windows takes `/` everywhere a path goes, and a backslash is an escape in TOML and in C, Go and C# strings. Only the system's own separator changes, so on Linux a backslash in a name stays.

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

Built (W1, 2026-10-27), for the stacks with the web renderer (Rust, Go, C, C++). A project with `stack go web web` is a web server; with `stack go web desktop web` it opens its window, and `--serve` runs the server instead. The server listens on `--address host:port`, else `CLARITY_ADDRESS`, else `127.0.0.1` at the project's `port` (8080 without the line). It serves the renderer's files, adding to the page `window.clarityServer = "/clarity/invoke"`, and takes each command as a POST of `{"command", "args"}` to that path, answered as `{"value"}` or `{"error"}` by the same code the desktop shell binds into its window: Go's `Service.Invoke` (`shell.Serve`, net/http), C's `clarity_service_invoke` and C++'s `Service::invoke` (`clarity_serve` and `shell::serve`, plain sockets with Winsock on Windows, one request at a time), and for Rust a generated `served` that calls what each Tauri command calls, behind `clarity_runtime::serve` (std only), with the files from what Tauri embeds. The renderer's `server` transport (`backend.ts`) posts with fetch; what belongs to the browser it does itself: a window's title is the tab's, a link opens a tab, a file to save is a download, and the person's settings stay in the browser's storage without the secret ones. Custom parts and windows of their own are the desktop's. Every browser tab runs the application of its own against the server's database. A program served alone links nothing of the desktop toolkit, so a server needs no WebKitGTK: Go's shell package is the service and the server, and package desktop the Wails window (its go.mod takes the modules of the runtime's package groups it imports, as go mod tidy picks them, recorded in `src/go/modules.txt` by a go_build test); C and C++ build `clarity_service` (service and server) apart from `clarity_shell` (the webview window, parts), which links it; Rust's Tauri commands, windows of their own and parts are a module of their own, `desktop.rs`, which a web program does not have, with no Tauri in its Cargo.toml and the renderer's files in the program by `include_bytes!`. Its output folder is `out/web`. The build tests check with ldd that a web program links neither WebKitGTK nor GTK and its desktop build does. In the IDE, Run starts a web program at the project's port (refused, with the reason, when another program listens there), the Browser button and the Output panel's link open its address, and Run is off while the application runs; the backend tells the screen when it runs and ends (`app-running`, `app-ended`, `app_state`), also after the screen reloads. What remains is Avalonia in the browser. The invaders example (`stack go web web`) is a game served this way; a screen engine now hears whole milliseconds, their fractions carried, and a whole pointer position, which a browser's clock and pointer do not give.

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

### 7.6 Avalonia in the browser (W2)

The web target for C# with Avalonia: `stack csharp avalonia web` serves the application to a browser, and the browser shows the same Avalonia windows the desktop shows, compiled to WebAssembly (Avalonia.Browser, .NET's browser runtime). It answers where the data lives and where the handlers run.

**What the generated C# is.** A window is a ViewModel, and its handlers are methods of it: they read and set its properties (the window's data, each part's `PartState`: shown, enabled, caption, text, selected row, drawing), call `Db` directly and synchronously, open another window by making its ViewModel (whose constructor loads its data), and continue a question through a closure (`Ask(question, answer => ...)`, a dialog's `whenClosedWith`). None of that can be taken apart from the ViewModel, and a closure does not survive a request.

**Three ways, and the one taken.**

| Way | What runs in the browser | Why not, or why |
|---|---|---|
| Everything in the browser | windows, handlers, SQLite in WebAssembly, data in the browser's storage | Each visitor has a database of their own and nobody shares data. Fits an offline tool for one person. Left for later, as a choice. |
| The database at a distance | windows and handlers; every `Db` call over HTTP | Every handler and `Db` call becomes async, a handler makes many round trips, and the server answers any query a browser sends: the whole database open to whoever visits. |
| **Live windows on the server** (taken) | the views, bound to a mirror of each ViewModel | The server keeps each tab's ViewModels alive in a session and runs every handler on them, with the database beside it, as the desktop does. The browser shows the windows and sends what the person does; the server answers what changed. No handler, query or connection runs in the browser, closures and dialogs keep working, and the generated ViewModels and handlers stay as they are. |

**A session.** A tab starts one (`start`, which gives its id); the server makes the main window's ViewModel, as the desktop shell does, and keeps it with every window it opens until the window closes or the tab has been quiet for a while (30 minutes, then its windows close). The database is the server's, one handler at a time, as on every stack.

**The mirror.** In the browser each window is the same ViewModel class with the same views, made without a database (`WindowViewModel(Db? db)`: a mirror skips the constructor's loading) and filled from what the server sends. Its commands do not run their handler: `Handle` hands the command's name to the session instead, with the state the person changed (the texts typed, the rows selected, the checks set: the ViewModel's settable properties and its parts' `Text`, `SelectedItem`, `IsFolded`, `Pointer`). The server applies that state to its ViewModel, runs the handler, and answers the state back, with what the handler asked of the windows: open (a new ViewModel, its id and its state), close, a question (its id, which the answer goes back to), a message. The state crosses as JSON, both ways, by one generic snapshot of a ViewModel (its public properties and its `PartState`s, records by the runtime's JSON); a selected row crosses as its place in its list.

**The parts of `out/web/`.**

- **The application** (a class library, net10.0): the generated windows, views, modules, schema and texts, as the desktop program has them, without `Program.cs`.
- **The server** (a console program, net10.0, speaking HTTP over a plain socket, as the other stacks' servers do; not .NET's `HttpListener`, which on Windows goes through http.sys, where only an administrator may listen on 127.0.0.1 or 0.0.0.0): the sessions at `/clarity/session`, the published browser program as its files, at the project's `port` with `--address` and `CLARITY_ADDRESS` as on every stack. Its build publishes the browser program into it (`-p:SkipBrowser=true` leaves that out).
- **The browser program** (Avalonia.Browser, net10.0-browser): the application with the mirror shell, which shows the windows in the page.

**Clarity.Avalonia splits.** What only a desktop program has leaves the core for `Clarity.Avalonia.Desktop`: `Avalonia.Desktop` (the platform's windows), sound through SoundFlow (native audio), the desktop `Shell`, `Unconnected`'s start, quitting and the keys window's own window. `Clarity.Avalonia` keeps everything without native code: the parts, the ViewModel base, the dialogs, the keys view, and the server's and the browser's own (`Snapshot`, `Sessions`, `WebServer`, `MirrorShell`, the page's sound), so the server and the browser program take it alone. A desktop program references `Clarity.Avalonia.Desktop`, which brings it, and starts with `UseClarityDesktop()`, which gives the windows the platform, the speakers (`Sound.Output`) and the desktop (`DesktopProgram.Current`: quit and keys); without it they do nothing.

**What belongs to the browser.** As the web renderer does (section 7.3): the window's title is the tab's, a link opens a tab, a file to save is a download, and the person's settings stay in the browser's storage, without the secret ones, through .NET's JavaScript interop. Dialogs are Clarity's own (U1f), which Avalonia draws in the page as on the desktop; a dialog window opens in place, since a page has one window.

**Engines.** On the server, as on the desktop: Wasmtime. An engine beside the screen runs in the browser, where Wasmtime cannot: the browser's own WebAssembly in a Web Worker, the worker the web renderer already has (`screenWorker`); its tells go to the session as any command does.

**What a browser cannot have.** Custom parts drawn by native libraries (section 6.5): the checker says so for a project with the web target and such a part. Sound plays through the browser's audio. Pictures and SVG draw through Skia as on the desktop; fonts are embedded.

**Building.** SkiaSharp's native code has to be linked into the browser program, which needs .NET's `wasm-tools` workload (`dotnet workload install wasm-tools`), on every machine that builds it; the tools check says so. The first load: the todo example's browser program is 49 MB untrimmed, 28 MB trimmed; the server sends the Brotli copies the publish makes, so a browser fetches some 8 MB. A Release build is trimmed, keeping whole the application's assembly, Clarity.Avalonia and Clarity.Runtime (the snapshot reaches their types by reflection, and System.Text.Json's reflection stays on); a Debug build is not, so Run publishes faster.

**The IDE.** Run publishes the browser program into the server, builds the server, starts it at the project's port and offers its address (the Browser button), as for the other web stacks.

**Slices.**

| Slice | What | Done when |
|---|---|---|
| W2a | The snapshot: a ViewModel's state out and in, records and selections included, and the mirror constructor; tests in Clarity.Avalonia | a ViewModel's state survives the round trip, and a mirror applies it |
| W2b | The sessions and the server: start, a command with its state, open, close, questions and dialogs by id, the quiet timeout; the generator's `out/web` with the application and the server | a server test drives the todo example's windows through the protocol |
| W2c | Clarity.Avalonia's split and the browser program: the mirror shell, published into the server; Run and Browser in the IDE | the todo example works in a browser against the server's database |
| W2d | The browser's own: title, links, downloads, settings, sound, engines beside the screen, embedded fonts, the word on native parts | done (2026-10-28): the pills game, retargeted to C# on the web, plays in a browser |
| W2e | Windows and macOS; the first load's size (the split, trimming) | the split, compression and trimming done (2026-10-28); the WINDOWS-TESTS items open |

**Open choices.** Everything in the browser, as a project setting for offline tools, later and only on request. AOT compilation for a faster start, per project.

Built (W2d, 2026-10-28): what belongs to the browser.
- **Fonts.** The browser program carries Inter (`Avalonia.Fonts.Inter`) and names it the default font (`FontManagerOptions.DefaultFamilyName`), which the themes' lists end with.
- **The browser page.** `IBrowserPage` (`BrowserPage.Current`) is the browser's side: the tab's title (the window on top's), a file to save as a download, a file to open shown in a tab, the browser's storage, and sounds. The browser program implements it with .NET's JavaScript interop (`Page.cs`, `clarity.js`). Links needed nothing: Avalonia's launcher opens a tab.
- **Settings per person.** `KeptSettings.Store` keeps settings in the browser's storage, without the secret ones; `Remembered` keeps choices there. Each request brings the person's settings, which `Settings.Current.Apply` takes on the server before their handlers run, and the reply takes back what a handler changed (`Sessions.Settings`, `MirrorShell.Start(..., settings)`).
- **Files and record pictures.** A window's fetchers are async (`Func<Task<byte[]?>>`, through `Fetched`). A mirror asks the server with a `fetch` request, which runs the window's fetcher of that name. `PictureBox` and `FileInput` await them.
- **What code tells a part.** A sound to play, or a custom part's hear, crosses to the mirror once, in the part's `told`.
- **Sound.** It plays through the page's audio (`PageSoundOutput`).
- **Engines beside the screen.** They run in the browser's own WebAssembly through `engine.js`, the web renderer's engine host built on its own (`browserEngines.ts`, `loadEngineNow`; `npm run build:runtime` builds it too). They run on the page's thread, since .NET's frame loop asks for each frame at once. An engine that runs past its time holds the page up, as no Web Worker can stop it there. The generated `ScreenEngines.InBrowser` is the seam. What an engine tells reaches its handler on the server: `Told_` methods carry `[Command]`, a method the server runs for a mirror with more than one value.
- **Whole milliseconds.** The C# frame loop now gives an engine whole milliseconds, their fractions carried, as the web renderer does.
- **Custom parts.** One drawn in a native window (`native`, `crate`, `go`) is refused for a program served to a browser alone; a .NET part is a control of the page.
- **Builds.** The browser publish runs again only when a file of the application, the browser program or the runtime projects it builds on changed.
- **The proof.** The pills game, retargeted to `stack csharp avalonia web` (in `target/`, not an example), plays in a browser: its interpreted engine draws the SVG pills, and its tells reach the server.

Built (W2e, the split; 2026-10-28). `Clarity.Avalonia.Desktop` is a package of its own (0.49.0), as above; the core's project names nothing native, which a test checks (`SplitTests`), and its default sound output is `Silent`. The generator: a desktop program references `Clarity.Avalonia.Desktop` and starts with `UseClarityDesktop()`; the application of a program served to browsers alone references `Clarity.Avalonia`, and its README says so.

Built (W2e, the size; 2026-10-28). `WebServer` sends a file's `.br` or `.gz` copy to a browser whose Accept-Encoding takes it (`Content-Encoding`, `Vary: Accept-Encoding`); the browser project trims in Release with the roots above. Checked in headless Chromium, trimmed in Release: the todo example shows its data and opens its Users window, and the pills game plays (SVG pictures, its engine in the page, the score through the server). The todo example went from 48.7 MB (14.4 MB compressed) to 28.0 MB (8.3 MB compressed).

Built (W2e, Windows; 2026-10-28). Found before testing on Windows, and fixed: `WebServer` listened through `HttpListener`, which Windows refuses at the default address to a person who is not an administrator; it now speaks HTTP/1.1 over a `TcpListener` (`Start` gives the port, so a test serves on port 0), with connections kept open between requests, request paths decoded (a file name with a space) and a backslash or a drive refused. The server's publish of the browser program quotes its paths, which broke in a folder with a space in its name (a Windows user's folder often has one), on Linux too.

Built (W2a, W2b, the mirror shell of W2c; 2026-10-28). In Clarity.Avalonia: `Snapshot` (a window's state as JSON, out and in, by reflection over the window's own properties: those with a setter, and get-only ones through their backing field; texts with their slots, shapes by kind, values of any type with their type; a selected row matched back by id to the row of the window's lists; a command run by its name, only a `[RelayCommand]`); `WindowViewModel(Db? db)` with `IsMirror` and `Remote`, and `Handle(handler, args, [CallerMemberName] command)`, which in a mirror sends the command instead of running it; `Sessions` (start, run, answer, close; windows in place and dialogs as the desktop shell orders them; questions by number; messages said; quiet sessions end after 30 minutes; `SessionException` for what a browser asked that cannot be); `WebServer`; `MirrorShell` with `ISessionLink` and `HttpSessionLink`. Tests: the snapshot, the sessions, the server, and the mirror shell against real sessions in the same process. The generator: a generated window's constructor returns at once in a mirror, its lists and choices start empty, a canvas click passes its pointer to `Handle`; `stack csharp avalonia web` writes `out/web/` with `App/` (the library, `Databases.Open`, the browser's `App.axaml.cs`), `Server/` and `Browser/`; with `desktop web` both programs. A build test builds the server and drives a session over HTTP. With the `wasm-tools` workload the server's build publishes the browser program, and the todo example works in a browser against the server's database (the first look, 2026-10-28). Clarity.Avalonia takes SkiaSharp's and HarfBuzz's WebAssembly packages at the versions its SVG pictures bring (Avalonia.Browser alone links older native ones, and the link fails); the browser program is not trimmed, since the windows' state crosses by reflection and themes load by name (83 MB in Debug). The tools check knows the workload for the web target of C# with Avalonia (`web_found`, `web_missing`, `ready_for(web)`), and Clarity installs it from the stack's Build tools row (Install), with the system's permission where .NET is installed for every user: pkexec on Linux, the administrator prompt on macOS, UAC on Windows (`tools::wasm_tools_install`). Run builds the server project of `out/web` and starts it at the project's port. The walker example (`stack csharp avalonia web`, a stick figure of SVG frames that a timer's handler on the server walks across a canvas) plays in a browser, each tick a round trip; the resources folder is found through the assembly of the running App (`MarkdownView.AppAssembly`), which in a browser is the application's library, not the program. Open: the person's settings per session (the server's `Settings.Current` is one for all), Clarity.Avalonia's split for the browser (Avalonia.Desktop, SoundFlow), the first load's size, and W2d (fonts embedded: the browser shows its fallback font now).

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
| P3 | Build tools per stack: Clarity is installed as a finished program, so a machine may lack a stack's tools (`cargo`, the .NET SDK, Go, a C or C++ compiler). Clarity looks for them, marks a stack whose tools are missing in New project and Settings with what to install, and keeps designing, previewing and generating working; only Build and Run say why they cannot run, so the generated source can be built elsewhere | done: `clarity_gen::tools` checks each stack through a `Probe` (tested with a faked machine): cargo, and on Linux WebKitGTK through pkg-config, for Rust; a .NET SDK of the version generated projects target for C#; Go; for C and C++ a compiler, CMake, and make or Ninja. One list of what to install, with where to get it, serves `clarity tools` (a line per stack), the Build tools row under the stack in New project and on the Project tab (with Environment..., which opens Clarity settings at Environment; `stack_tools` in the Tauri app), Build and Run (they still generate, then say what to install), and the README.md every generated project now has: what the stack needs, the commands to build and run it without Clarity, and its data sources (a server database by its environment variable, with an example connection). Like JetBrains' toolchains; a tool that is not on PATH is chosen in Clarity settings, Environment (EN1) |
| L1 | The Clarity behavior language: handlers written in Clarity in `code` blocks, with typed access to a window's parts and data (sections 1.1 and 5.9); wiring an event writes an empty handler; the code panel in the designer checks while you type | core done; open: modules with shared functions and actions (L2) |
| L2 | Backend modules in Clarity: functions and actions in modules, callable from handlers and each other, with `require`, `ensure` and `effects` (section 5.10); design as text with the designer following the text (section 5.9); modules show and edit their functions | done |
| G1a | Rust backend: `clarity-gen` writes a Cargo project into `out/` (deterministic, kept in sync by `write_out`), with records and enums from the dictionary, the SQLite schema as the first migration, functions and actions as Rust with contract checks, and the `clarity-runtime` crate copied in; `clarity gen`; the generated shop builds and runs, and generated logic is tested | done |
| G1b | Handlers and the event protocol, the web renderer as a standalone package, the Tauri shell, Run and Build on the Preview tab | done |
| E1 | Events beyond parts: window `on open`, `on close` and `on return` (back from a window it opened, to refresh), app `on start`; data events (before and after save) follow with D1 | done (section 5.9): window and app events, `reload`, Events in the designer, the browse template refreshes on return; data events stay with D1 |
| U1a | Forms and text parts (section 5.6): `form` bound to one record with `on submit`, `label for`, `help`, `heading`, `text`, `memo`, `password`, `switch`, `radio`; `placeholder`, `tooltip`, `readonly`, `taborder`, `key`, `default` and `cancel`; `error` and `readonly` as part properties in handlers; the form template and field drops place label, input and help together | done; open: a `time` part waits for a time field type, menu item keys are shown but not yet pressed at runtime |
| U1b | Actions and containers: icon button, split button, toggle button, button group; expander, accordion, scroll box, separator, link, badge, icon, progress, busy indicator; containers that share the column lines of the grid around them (as CSS subgrid does), so a label and an input in a panel line up with those outside it, and a label column for forms | icons done (2026-10-23): Lucide 1.50.0 vendored in crates/clarity-spec/icons (converted to one plain path per icon that the web and Avalonia read alike), `icon "name"` on buttons and commands, the `icon` part, the checker's suggestions, only the used icons in each application with Lucide's license in THIRD_PARTY_NOTICES.md. Small parts done (2026-10-23): `separator [vertical]`, `link` (`to`, `bind` or `on click`), `badge`, `progress ... max`, `busy`, with conformance/small-parts.json. Toggle, button group (`buttons`) and split button done (2026-10-23; Avalonia ToggleButton, SplitButton with MenuFlyout; the designer's Add menu). Accordion (`section` blocks, `part.open`, new part state in every runtime: Rust 0.14.0, Go v0.16.0, NuGet 0.20.0) and scroll box done (2026-10-23). Shared grid lines (always: frames sit 3px out into the column gap, ClarityFramePadding on Avalonia, GridLineTests) and the form label column (`labels 3`, labels made by the spec after parsing, named `<input>Label`, showing the field's label key) done (2026-10-23): U1b complete. Was: toggle, button group and split button (`split` on a button, its `menu` block the dropdown), then accordion and scroll box, then shared grid lines and a label column |
| U1c | Data parts: tree, tree table, pager, search bar, lookup (a reference picked from a searchable list), chart; combo box, slider, spin box, color, file and tags inputs | decided (2026-10-23): inputs first, in slices: (1a) `slider` and `spinbox` for integer, decimal and money, with `min`, `max` and `step` (the field's own `min` and `max` without them; a slider 0 to 100 without either) done 2026-10-23: conformance/ranges.json (exact decimal steps), web SliderInput and NumberInput with a range, Avalonia SliderInput and NumberInput with Min, Max and Step in NuGet 0.21.0; (1b) the field type `color` (kept as text `#rrggbb`) and its `color` input, done 2026-10-23: VARCHAR(7), conformance/colors.json (reading typed colors, the shared 60-color palette), Clarity's own palette popup on both renderers (no system color dialog), color table columns, the checker on defaults and on text code puts into a color field, NuGet 0.22.0; Replanned with the user (2026-10-23): parts that are an option of another part become that option, and what other parts already make is dropped. Next, in order: (1c) the field type `file` and its `file` input (decided 2026-10-23): kept like an `image`, in the database or with `store` as a file, with its name kept too: in the database the bytes in the field's column and the name in a second column beside it (`manual`, `manual_name`), which migrations add, rename and remove together; in a store the file keeps its own name in a folder named by its contents, and the column holds that path; the person chooses a file, opens it in the system's program for its kind, saves a copy and removes it; done 2026-10-23 (as built the store mode has the name column too, so every runtime reads the name the same way; a copy goes to the Downloads folder without a dialog; conformance/record-files.json; the file part's own words noFile, chooseFile, openFile, saveFileCopy, removeFile, fileCopySaved); (1d) lists of values as fields, the generic ground for tags: `list<text>`, `list<integer>`, `list<decimal>`, `list<date>`, `list<Enum>` (the value lists code already has), kept as one JSON column, shown in tables, used in code with `for`, `count` and `contains`, edited with the `tags` input as chips; done 2026-10-23 (code's value lists also hold dates and enum values now; conformance/list-fields.json; Clarity's words addTag and removeTag; a query finds the records whose list holds an item, `where a.tags contains x`, with each database's own JSON functions: json_each, jsonb `@>`, JSON_CONTAINS, OPENJSON; conformance/queries.json); (2) options on the dropdown instead of new parts: `typed` (any text may be typed, the choices are suggestions: the combo box) and `search` with `show field, ...` (the choices filter as typed, each showing those fields: the lookup for a reference among many records); done 2026-10-23 (user chose: `typed` offers the values the field already has, from the shell's new `field_values`; `search` filters in the window over the records `ref_options` loads, which now carry the fields shown; conformance/dropdown-options.json; web TypeAheadInput, Avalonia TypedInput and LookupInput); (3) a pager as a table option (`page 50`), not a part, done 2026-10-23 (pages the rows the window holds, after ordering; conformance/table-pages.json; Clarity's words firstPage, previousPage, nextPage, lastPage; no runtime change); (4) `tree` and `treetable`, with `children` naming a field that lists the entity's own kind (`children: list<Folder>`), done 2026-10-23 (user chose: bound to all the rows in the window, arranged by the one `ref Self` field, not loaded as expanded; conformance/trees.json; both are Clarity's table in tree mode on each renderer; Clarity's words expandRow and collapseRow; no runtime change); (5) chart, done 2026-10-23 (user chose: `chart bar|line|pie ... by field count|sum f|average f`, grouped in the window over the rows it holds; conformance/charts.json; web ChartView in SVG, Avalonia ChartView on a Canvas; no runtime change). The U1c batch is complete. Dropped: the search bar, which a text box and a handler already make (the Browse template writes one) |
| U1d | Window kinds and overlays: dialogs with OK and Cancel results, wizards with steps, context menus, popovers | dialogs that give a value done (2026-10-22): `page X: dialog gives Type`, `close with value`, `ask X [with ...] then handler(values)` on every stack (the reply of the closing handler carries `result`; the web renderer remembers which open window asked; Avalonia hands the value to the opener). wizards with steps done (2026-10-22, 5.15); context menus and popovers done (2026-10-22, 5.16), on the web renderer and Avalonia's ContextMenu and Popup, with no runtime release needed |
| U1e | Table columns (decided 2026-10-22): a width in pixels after a field (`columns title 240, artist, year 60`; a column without one shares what is left), `resizable` and `sortable` as options, and `order by field [desc]` for the order rows show in first, written as in queries. On the design canvas the header edges are dragged to set widths. Running: with `resizable` the person drags header edges and the widths are remembered per window and table for that person (double-click: the designed widths again); with `sortable` a click on a header sorts by it, again the other way. Sorting is the renderer's: rows keep their places in the data, so `selected` and the handlers see the same record. Web renderer: `lib/ui/Table.svelte`, copied by `clarity-gen`; Avalonia: ClarityTable in Clarity.Avalonia (widths remembered through `Remembered`), so a new package version and its NuGet upload | done (2026-10-22): spec, checker and renamer; IDE properties and header edges on the canvas; web table (sorting in the view, grips, widths in local storage, conformance/table-order.json); Avalonia ClarityTable (own sorting, Remembered widths), C# generator; Clarity.Avalonia 0.17.0 |
| U1f | Clarity's own dialogs in the applications people design, as the IDE has them: `ask` in handlers with the answer running another handler (section 5.14, decided 2026-10-22), dialog windows closing `with` a value (with U1d), and `message` in the same dialog. Every stack in its idiom: the web renderer (`lib/ui`, copied by `clarity-gen`) and Avalonia (Clarity.Avalonia, so a new package version and its NuGet upload). With U1e, after the class diagram designer | done (2026-10-22), dialog windows closing `with` a value included (U1d): `ask` in the language and checker; replies carry the questions on every web stack (Rust crate 0.11.0, the Go module, the C and C++ runtimes) with `answer_event` running the answering handler; the web renderer's dialog (lib/ui/AskDialog.svelte, Dialog.svelte) with warnings and errors in it and a small x on notes; Avalonia asks through WindowViewModel.Ask and the Shell's dialog (Clarity.Avalonia 0.17.0); build tests for Go, C, C++ and C# |
| U1g | Settings (section 5.17, decided 2026-10-23): one `settings` block of typed values kept per person on their machine, with Clarity's own `language` and `theme`; every window binds `settings` and code reads and sets it; the pickers change the same values; a "Settings window" template | done 2026-10-23: the spec reads the block as the record type `Settings` (`Entity.settings`, `FieldType::Language` and `Theme`, `DataDecl.made`, `Project::stored_entities`), conformance/settings.json (which kept values fit, where the file is), the web renderer holds them (lib/settings.ts, Runtime.svelte) with the shell commands `read_settings` and `keep_settings` (Rust `clarity_runtime::settings`, Go `clarity.ReadSettings`, C `clarity/settings.c`, C++ `clarity/settings.cpp`), Avalonia `KeptSettings` and a generated `Settings.Current`; the Settings window template. Releases: clarity-runtime 0.15.0, Go v0.17.0, NuGet 0.22.0 |
| U1h | Full-text search (decided 2026-10-23, after the U1c batch): a `search` modifier on text fields and list fields, whose JSON column is text to a full-text index like any other, and `where a matches x` in queries; each database's own index (SQLite FTS5 kept by triggers a migration writes, PostgreSQL GIN on to_tsvector, MySQL FULLTEXT, SQL Server's full-text catalog where it is installed, else LIKE). Words match, not whole items; `contains` stays the exact test, and the index narrows it | done 2026-10-23 (user chose: every word, each as the start of a word; `matches` looks in the query's own record; SQLite FTS5 with triggers (`table__search`, unicode61, accents count), PostgreSQL GIN on to_tsvector('simple'), MySQL FULLTEXT without stopwords plus REGEXP for words under 3 characters, SQL Server LIKE at word starts without an index; a step that changes an entity builds its index again; migration steps `add search` and `remove search`; conformance/queries.json; tested on real SQLite, PostgreSQL and MySQL, SQL Server by its SQL only; the IDE's entity editor has a Search column) |
| UX1 | Usability rules (section 5.18, a design): a named rule set from published guidance (Nielsen, WCAG 2.2, the platform guides), checked on the spec as warnings; the agent's instructions read the same table; the designer shows them on the canvas with a fix where one is clear; a rule turned off for a part, window or project with a reason. First slice: `confirm-delete`, `label-every-input`, `default-and-cancel` and `long-list`, then the rest of the table | |
| CP1 | Custom parts (sections 5.19 and 6.5, decided 2026-10-23): `part Name` declared in Clarity with its library in the project's own language (`native` for C and C++, `crate` for Rust, `go` for Go, `nuget` or `dll` for C#), `binds`, typed properties, `tells`, `hears` and `size`, placed by its name, checked and set in the designer like a built-in part. A C, C++, Rust or Go part draws in a native child window its runtime's part host keeps at the part's place; a C# part is an Avalonia control through its adapter | done 2026-10-23: the language and checker (a part of another language than the stack's is an error), `clarity parts` writing each part's interface file and a starter, the part hosts (`runtimes/parts` in C for C and C++, `clarity_runtime::parts`, the Go runtime's `parts` package; Linux with X11), the shells' part commands, the renderer placing parts and hiding them under Clarity's overlays, Avalonia's `DotNetPart`, `conformance/custom-parts.json`; a test part in each language tells, hears and is set in Xvfb on C, C++, Rust and Go; the orbs example (C++, SFML) and the dice example (Rust, Bevy 0.19 without its window loop, drawing in the part's window through its raw handle); the designer's palette and properties panel, a view per part with its files in CodeMirror; the canvas's `renderer` removed. Open: macOS child windows (Windows is CP2) |
| CP2 | Custom parts on Windows (section 6.5, Windows, 2026-10-24): Win32 child windows in the C, Rust and Go part hosts, the hosts' threads pumping messages, the shells giving the parent HWND, Rust's `Window` with a Win32 handle and Go's with one `Handle`; the round-trip tests in a GitHub Actions job on Windows. Needs new Rust and Go runtime versions | done 2026-10-24: `platform_win32.c` in `runtimes/parts` (the C and C++ shells load `clarity_parts.dll` with `LoadLibrary`, and the generated CMake names it the same on every system), a `cfg(windows)` platform in `clarity_runtime::parts` on `windows-sys`, and `win32_windows.go` in the Go runtime's `parts` on `golang.org/x/sys/windows` without cgo; each host gives the parent's children `WS_CLIPSIBLINGS` before it puts a part among them, without which the webview paints over the part; Rust's `Window` carries `hwnd` and `hinstance` and gives a `Win32WindowHandle`, Go's carries one `Handle` in place of `XID` (runtimes/rust 0.21.0, runtimes/go v0.23.0, breaking). Verified by hand on Windows 11: dice (Bevy), orbs (SFML from MSYS2) and waves (Go) all draw. Open: the GitHub Actions job on `windows-latest`, and macOS |
| EL1 | External libraries (section 5.21, decided 2026-10-23): `library Name` with its package in the project's language (`nuget`, `dll`, `crate`, `go`, `cmake`) and its `function` lines, called as `Name.function(...)`; the adapter in `libraries/Name/` in the stack's language, typed by a file Clarity writes | done 2026-10-23 on all five stacks: the language and checker, the stack's language check, starters, generators; tests build a program with a library on every stack |
| SD1 | Sound (section 5.22, decided 2026-10-24): the built-in `sound` part, without a place in the grid; `source`, `volume`, `repeat`; code tells it `play`, `stop`, `tone`, `melody`, `volume`; sound files a kind of resource; Web Audio in the renderer, SoundFlow in Clarity.Avalonia; a tray of parts without a place under the designer's window. Needs a Clarity.Avalonia version | done 2026-10-24: the language and checker (calls, melodies written in code, volume, the source a sound of the resources), `conformance/melody.json` read alike by the checker, the renderer and Clarity.Avalonia, sound files a kind of resource (played in the resource view), the renderer's Web Audio player (files carried in the page on C and C++), Clarity.Avalonia's `Sound` on SoundFlow with tones made into WAV data (0.28.0), the designer's Media palette, tray and inspector; the dice example rattles and plays a fanfare. Not yet heard on every stack by a person |
| EN1 | Environment settings (asked 2026-10-24): Clarity settings, Environment, for every project on the computer: each stack ready or what it misses, and each build tool with every copy a scan finds and its version; the person picks one or types a path, kept in `~/.clarity/environment.json`, which the app and the `clarity` command both read. Builds put chosen tools' folders first and pass chosen compilers to CMake as `CC` and `CXX`. On Windows the scan also looks in Program Files (Go, .NET, CMake, LLVM), in Visual Studio's installations (by folder and by `vswhere`: its `cl`, CMake and Ninja), in MSYS2's MinGW and Clang, and in Scoop's, Chocolatey's and WinGet's links; `cl` and `mingw32-make` count as a compiler and as make. Visual Studio's build environment is open (section 6.5, Windows, point 12) | done 2026-10-24; tried on Windows 11: the scan finds MSYS2's UCRT64 tools and reports every stack ready. Open: it reports C and C++ ready without looking for their libraries at all, because the library checks are gated on Linux, so a stack that cannot configure is called ready; the install hints name only Debian's packages; and a build gets the chosen `CC` and `CXX` but not the prefix they belong to, so C and C++ build from a terminal and not yet from Clarity (section 6.5, Windows, point 11). Guidance, 2026-10-24: the manual's page Setting up the build tools (`setup.tools`) says what to install per system and stack, with the commands and the official links; every missing tool carries the heading that tells how (`StackTools.guides`, `clarity_gen::tools::SETUP_TOPIC`), and **How to install** on the Environment page and the Build tools row opens the manual's window there; its web links open in the browser on a click; `clarity tools` and Build's message name the page |
| EN2 | Folders settings (asked 2026-10-25): Clarity settings, Folders, for every project on the computer: the Projects folder, where New makes a project after asking only its name, where Open starts and where the agent makes projects; and the Apps folder, where Run and Build write each application in a folder named after its project file. A project whose `output` is not the default `out` keeps its own. Kept in `~/.clarity/folders.json` (`clarity_project::Folders`, `out_dir_in`), which the app and the `clarity` command both read; the Project settings show the folder builds write to (`ProjectFile.app_folder`) | done 2026-10-25 |
| EN3 | Status bar and System settings (asked 2026-10-25): a status bar at the foot of the IDE (what is open, the project file, the build, errors and warnings, the stack, the app folder, and meters for cpu, mem and io), and Clarity settings, System: a minute of graphs for each core, memory and swap, network and disk, with Clarity's share (the IDE and every process below it). Read by `system.rs` with the `sysinfo` crate once a second; only the meters' part unless the page is open (`watch_system`), and the page itself is loaded on demand | done 2026-10-25 |
| TI1 | Timer part (asked 2026-10-24): a built-in `timer` without a place in the grid, kept in the designer's tray as a sound is (5.22): `timer name refresh every 30 seconds` (a duration) with `on tick: refreshTicked`, and `once` for a single tick; code tells it `start()`, `stop()` and `every(duration)`. It ticks only while its window is open, and a tick waits while a handler runs (ticks do not pile up). The renderer ticks with the browser's timers and runs the handler as a click's; Clarity.Avalonia with a `DispatcherTimer`. Makes clocks, countdowns, reminders and dashboards that refresh themselves possible; `conformance/timers.json` | planned |
| HC1 | HTTP calls from code (asked 2026-10-24): `let reply = http get "https://..."` and `http post url with text` (headers with `with headers`), giving a reply with `status`, `ok` and `text`, and a reply's JSON read into an entity's fields. An action that calls out declares `effects net`; a key or token comes from the vault or a setting, never from the code. Each backend with its own client (reqwest, net/http, HttpClient, libcurl), a timeout, and an error the handler can catch; the window waits as for any handler until background work is planned. Makes API clients, syncing and webhooks out possible | first slice done 2026-10-24 (section 5.23): the call, its reply as text, headers and the effect `net` on all five stacks, tested on each against a test server. Releases: clarity-runtime 0.22.0, Go v0.24.0, NuGet 0.29.0. JSON done 2026-10-25: `json of`, `read Order from json text at "path"` with fitting (conformance/json.json) on all five stacks. Next: the OpenAPI file (D2) |
| UX2 | Usability from use (section 5.18): a running application notes, with the person's consent and only on their machine, where they struggle (clicks that do nothing, repeated clicks, errors shown again, forms given up), and the designer shows it as suggestions on the parts | idea, not decided |
| I1a | Formats (section 1.5): dates, times, numbers and money shown and entered in the locale of the person using the application; `language` and `currency` in the project file; named formats for `format`; money with its currency's decimals | done for the web renderer (LANGUAGE.md section 4, Formats): `language "tag"` and `currency CODE` checked and on the Project tab, `format` a name checked per type (catalog in `clarity_spec::formats`, a Format column in the entity editor), the renderer formats and reads values with `Intl` (`app/src/lib/format.ts`), the shared cases in `conformance/formats.json` (`npm test`); Avalonia meets the same cases with G2; open: date inputs follow the system's locale through the browser's own date picker |
| I1b | Text with slots (section 1.5): `"Saved {customer.name}"` in messages, refusals and captions, slots checked as expressions and shown in the locale's format | done for the web renderer (LANGUAGE.md section 10, Text with slots): slots are paths checked like expressions, `{{` and `}}` braces, text with slots allowed in `message`, `refuse`, `caption` and `error` and a warning for shown text joined with `+`; the runtime's `Text` and `Slot` carry the text as written and typed values with the field's format and scale (plain text stays a string on the wire), refusals reach the screen with their slots, and the renderer fills them (`fillText`, cases in `conformance/formats.json` under `fills`); Avalonia meets the same cases with G2 |
| I1c | Translations (section 1.5): text collected from the whole project, translation files per language and module, slots kept, missing translations listed, a Translations table on the Project tab, a language choice for the canvas and Preview, switching language in the running application | done for the web renderer (LANGUAGE.md section 2, Translations): one file per language for the project (`translations/nl.clarity`), texts collected by `clarity_spec::translations::texts` (labels, help, titles, captions, placeholders, tooltips, tabs, enum labels, frame items, status, messages, refusals, captions and errors set in code, Yes and No), checked for slots, duplicates, stale lines and the file's language; the Translations table and the designer's language choice on the Project tab; the application picks the translation for the person's locale (cases in `conformance/translations.json`), and `on click -> language` switches it while the application runs (remembered for the next start); open: `context` for one text with two meanings, the generated literal key maps of section 6.7 for the other stacks (G2) |
| T1 | Themes (section 1.5): a `theme` with named colors per light and dark mode, font, size, radius and named styles; `style` on parts; a theme editor with a live sample; CSS variables for the web renderer, resources for Avalonia | done for the web renderer (LANGUAGE.md section 7, Themes): `theme` declarations checked (`clarity_spec::themes`), `theme Name` in the project file and on the Project tab, `style` on parts checked against that theme and chosen in part properties; a theme resolves for light or dark mode in `clarity_spec::themes::resolve` (cases in `conformance/themes.json`), the generator writes a CSS file per theme into `ui/themes/` and links it, the application holds the theme's class and each part its style's class, the designer gets the same CSS from the backend, and palettes are chosen per element (`data-scheme`) so a theme can be light on a dark system; themes open from Themes in the project tree in their own view, written as text beside a live sample window; several themes per application (`theme Shop Night`), switched with `on click -> theme` and kept for the next start, each with a `title` that is translated, every offered theme holding every style parts use; Clarity's own look as the built-in `theme Clarity` with every value (colors, fonts, sizes, radii, padding, border, shadow, part looks in their states), the application's CSS split from the designer's and reading only those values; Avalonia maps the same resolution onto resources with G2 |
| C1 | Command-line applications (section 5.20, built): `cli` target beside `desktop` (`stack go cli` alone, or `desktop cli` for both), output `out/desktop/` and `out/cli/`; `cli Shop` block; `command [group] name(args)` with `option` lines and `help @key`; `print` (a table with `columns`), `ask` as an expression that waits, `exit n`, `refuse` exits 1, wrong command lines exit 2; one parser per runtime against `conformance/command-line.json`; Commands in the IDE tree, Run asks the command line; examples `ledger` (csharp cli) and `stock` (go desktop cli). Needs runtime versions on every stack | done; open: `go mod tidy` would trim the CLI module's requirements |
| D1a | Queries: `find c in Entity where ... order by ...` as an expression, `contains` on text, the `text` of unbound text inputs, handlers that run other handlers; the runtime builds the SQL from a condition tree; the browse template filters and refreshes through its filter | done (LANGUAGE.md section 10); open: conditions through references (`o.customer.name`), which need joins |
| D1b | Migrations (section 1.3): steps written in Clarity in `migrations/`, the schema from replaying them, pending changes as warnings and in the Project tab, proposed renames, confirmation for changes that lose data, `clarity migrate`, SQL per step with table rebuilds for SQLite, applied at startup with references checked | done (LANGUAGE.md section 4); open: data migrations (Clarity code that transforms rows), down steps, counting the rows a narrowing change would not fit, renaming enum values, a command-line switch to apply steps separately |
| D1c | Entities placed per data source: `source` in a module, checked against the project file; migration steps per SQL source; one database per source at run time, each record type knowing its source | done (LANGUAGE.md section 3) |
| D1f | Data events: before and after save, before and after delete, written in Clarity next to the entity | done (LANGUAGE.md section 8, Data events): `code Entity` with `before save`, `after save`, `before delete` and `after delete`, each checked and generated as an action on its record; `refuse "text"` as a statement; the runtime runs them through `Record` hooks in one transaction per database with the saves and deletes they make, and a failed save leaves a new record new; renaming an entity renames its events; open: editing events in the designer (the Code panel shows windows only), one transaction across databases |
| D1d | PostgreSQL, MySQL and SQL Server for owned sources: a runtime driver per database as a cargo feature, queries and record statements per dialect, migration steps written per dialect (types, keys, foreign keys added at the end of a step, unique indexes that skip empty text, type changes through a new column), connections from `env:NAME` | done (LANGUAGE.md section 4, Databases), tested against live servers (`crates/clarity-gen/tests/servers.rs`); open: TLS for PostgreSQL and MySQL, baselines and Import for these databases (D1e), money shown with the four decimals the servers store |
| D1e | External databases: `external` and `readonly` on a data source; `table`, `key` and `column` mapping; the runtime and generator using the mapped names; Import (Project tab and `clarity import`); comparing the dictionary with the live tables on every load; new crate `clarity-db` | done for SQLite (LANGUAGE.md section 4), Take ownership included: a baseline step that creates only what is missing, mapped names kept in the migration history with `map table`, `map key` and `map column` steps; open: keys that are not one integer column, Import, checking and baselines for PostgreSQL, MySQL and SQL Server |
| D2 | Web API data sources: REST endpoints mapped to entities, auth and paging, OpenAPI import. Asked 2026-10-25: an OpenAPI (Swagger) file in the project's `apis` folder that the developer and Clarity's agent build an application around (section 5.23) | first slice done 2026-10-25: entities in a web API source, `http ... from source` with `query`, the `openapi` and `pick` lines, `apis/<source>.clarity` written from the picks and again for each revision, the IDE's API... picking. Next: optional query parameters, nested objects, paging, the agent picking |
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
| G4 | C and C++: emitters, runtimes, webview shell; Qt as a native toolkit for C++ if wanted. The project is a CMake project (`CMakeLists.txt`, built with make or Ninja, the tools P3 checks for), with the README.md every stack gets | G4a (C++) done: the runtime `runtimes/cpp` (C++20; system SQLite 3 and nlohmann-json, webview 0.12.0 copied into `third_party`; errors as exceptions, its own `Decimal`, SHA-256 and base64; drivers in `unique_ptr`, conditions in `shared_ptr<const Node>`, SQLite handles closed by RAII; records as plain structs that meet a `Record` concept) with its own tests on the conformance files; the generator `clarity-gen/src/cpp` writes `CMakeLists.txt`, `CMakePresets.json` (`cmake --workflow --preset debug` or `release`), `src/records.hpp` and `.cpp` with every module's enums and records, `src/modules/<module>.cpp` with each module's functions, actions, data events and windows, `src/app.hpp`, `src/schema.cpp`, `src/page.cpp` with the web renderer inlined, and `src/main.cpp` with `--check`; the program is a native window (webview on WebKitGTK) that talks to the renderer through `clarityInvoke`. Both examples build and run (`cargo test -p clarity-gen --test cpp_build -- --ignored`). SQLite only; the server databases come later. Every stack now writes module files where they cannot overwrite its own files (`src/modules/` in Rust and C++, `Modules/` in C#, `<module>_module.go` in Go, which also keeps a module named `windows` from building on Windows only). G4b (C) done: the runtime `runtimes/c` (C17; system SQLite 3, cJSON 1.7.18 and webview 0.12.0 copied into `third_party`, the webview built from one C++ file). Every call that can fail returns whether it worked and leaves its error, as errno does; values live in a scratch arena the shell empties after each command, so generated code never frees anything (cJSON allocates there too). Records are plain structs described to the runtime by a `ClarityEntity` (table, columns, values, data events); `customer_save`, `customer_find` and the others are typed calls around it. 20 tests of its own on the conformance files, clean under AddressSanitizer and UBSan. The generator `clarity-gen/src/c` lifts each call that can fail into `if (!call(..., &v1)) return false;` (as the Go generator lifts into `if err != nil`), passes records by pointer, gives a function's result through its last parameter, and writes the same files as C++ in C (`records.h`/`.c`, `app.h`, `src/modules/<module>.c`, `schema.c`, `page.c`, `main.c`); the program builds without a warning under `-Wall -Wextra`, debug and release. Both examples build and run (`cargo test -p clarity-gen --test c_build -- --ignored`). `clarity tools` checks the C and C++ libraries too (SQLite, WebKitGTK, nlohmann-json for C++) and, for C, a C++ compiler for the webview. SQLite only; the server databases come later. Both build on Windows as well since 2026-10-24: `clarity.cmake` fetches the WebView2 SDK the webview builds against, and the libraries come from one MSYS2 prefix (section 6.5, Windows, points 10 and 11) |
| H1 | Hardware, ideas raised by the user (2026-10-18), not decided. (a) A desktop application that talks to a device such as an Arduino over a serial port: the settings name the device as they name a file store (`serialport arduino "env:ARDUINO_PORT"`), code sends and receives with an effect like `device.write`; every stack can (Rust `serialport`, Go `go.bug.st/serial`, C# `System.IO.Ports`, C `libserialport`). (b) Code that runs on the device itself: Arduino sketches are C++, so a firmware target generated from a small part of the language, without a database or windows | idea |
| N1 | Engines, first slice (section 5.11): `engine Name from "file.wasm"` with stateless functions on plain values, enums, records and lists; the `engines` folder; the engine interface (JSON over plain memory); the host checks the file at start; hosted on the backend of all five stacks (wasmi, wazero, wasm3, Wasmtime for .NET); a shared test engine and `conformance/engines.json` | done (2026-10-20): see "Built" in section 5.11. The designer reloads when the `engines` folder changes, lists the engines in the project tree (marking one that does not fit its file) and shows their functions in the module view |
| N2 | Engines written in Clarity (section 5.12): N2a `engine Name` with what it keeps and its functions (transpiled per stack until N5b compiled them to WebAssembly); then `record` declarations; N2b with N5 beside the screen | N2a done (2026-10-21); compiled to WebAssembly since N5b slice 3 (2026-10-27) |
| N3 | Imports and limits: `uses clock, random, log, progress`, time and memory limits per call, failures shown as refusals | done (2026-10-20) but `progress`: see "Built (N3)" in section 5.11 |
| N4 | State and hooks: `state per window` and `state per app`, hooks such as `on open` and `on close`; `bytes` for document processing | done (2026-10-20): state, `on open` and `bytes`, see "Built (N4)" in section 5.11; no `on close` (the engine has nothing it could do then) |
| N5 | Engines beside the screen (section 5.12): N5a brought along, in a Web Worker and in Avalonia, the frame loop, input, tells and hears; N5b: Clarity engines compile to WebAssembly on every stack, so one can run beside the screen too (`tell`, records) | N5b done (2026-10-27: compiler and interpreter, chosen per project, every stack, records and `tell`); N5a done (2026-10-26: frame loop, pointer and keys, tells and hears, the log into the app's log, the designer's Engine field); next N5b |
| N6 | Engines in more languages and outside the IDE, so hand-written logic never collides with generation (RT1). Already there for Rust (`clarity_project::engines`): the source as a crate in `engines/<name>/` on the `clarity-engine` library, made by Clarity with a stub per declared function and given stubs for functions added later, edited, built and rebuilt before Build and Run when it changed, compiler messages as problems at their lines, Rust's WebAssembly target checked like a stack's tools; any other language is a `.wasm` file only, which Clarity checks and leaves be. What N6 adds: building engines from the command line (`clarity engines build`, and `clarity watch` rebuilding on save), so someone working in VS Code needs no cargo command of their own; libraries like `clarity-engine` and templates for more languages (C and C++, Go through TinyGo or `go:wasmexport`, Zig, AssemblyScript; C# only once .NET makes plain WebAssembly modules well), each with its build tools checked. Open: the WebAssembly Component Model with WIT interfaces instead of the JSON interface, which brings glue for many languages but is heavier and unevenly supported by the five runtimes. Open, not planned: widening the sandbox with more `uses` services (`uses http`, `uses serial` for H1), each one implemented by every stack's host; it lets engines reach more of the platform, but every service costs five hosts and makes an engine less of a pure computation that is the same everywhere and tested with plain values. Raised by the user (2026-10-23) | |
| P | Progress of a long engine call on the backend (section 5.12): `uses progress`, a bar and Stop on the window while the handler runs, through each shell's way to the screen | |
| DG1 | Diagrams, first slice (section 5.13): a Diagrams part with a `diagrams` folder (not carried into applications), the Data model and Classes views generated from the project, written `.mermaid` ERD and class diagrams checked by `clarity_spec::diagrams`, the diagram designer (mermaid.js with Clarity's layer over it: toolbox, properties, click through, relationships drawn by dragging, Design and Text tabs), differences marked before Apply, Apply as one undo step with label keys and the migration step, and the agent's diagram tools | |
| DG2 | State and sequence diagrams (section 5.13): `steps` between enum values as a language construct, checked and enforced on every stack, designed as a state diagram; a sequence diagram per handler as a view | |
| DG3 | Activity diagrams per handler or function and the architecture of the project, as views (section 5.13) | |
| W1 | The web target: each backend serves the web renderer over HTTP; Avalonia in the browser through WebAssembly | web renderer stacks done (2026-10-28), without the desktop toolkit, with the project's `port`; see "Built (W1)" in section 7.3; Avalonia in the browser is W2 |
| W2 | Avalonia in the browser (section 7.6): live ViewModels in sessions on a C# server, mirrored windows in the browser through Avalonia.Browser; slices W2a to W2e | W2a to W2d done (2026-10-28): C# programs run in a browser against their server, the pills game included; W2e: size done (8 MB first load), Windows and macOS open |
| A1 | AI through the MCP server: the whole project and its output readable, changes only as designer edits, proposals inside a project and for Clarity itself (section 1.2) | |
| A2 | Local models for Clarity's agent, so a person can work without a cloud account or sending the project anywhere: connection kinds beside `anthropic`, first `ollama`, then `openai` for any OpenAI-compatible endpoint (llama.cpp server, LM Studio, vLLM), each a `Model` in `clarity-agent` beside `anthropic.rs`. A local connection needs a base URL and a model but no API key; fields a kind does not use (`cacheTtl`) are not asked for. Testing a connection checks that the model answers and can call tools, and says so when it cannot. The tools, the permission step for Build and Run and the turn budget stay the same for every kind. Tested against a faked server per kind. Raised by the user (2026-10-23) after comparing with Embarcadero's Kai, which offers local models | |
| RT1 | Round tripping, so a developer can open the generated project in VS Code, change it with any AI, and keep designing in Clarity afterwards. Logic that only computes goes into an engine first (N6), which generation never touches; RT1 is for code that needs the platform: libraries such as a NuGet package, OS features, native UI. In this order: (a) seams per file: generated files and files the developer owns side by side in each stack's idiom (partial classes in C#, separate files in Go, Rust, C and C++, beside `src/modules/`), an owned file written once and never again, as the stub rule of section 11 did before behavior moved into the Clarity language; (b) hand-written items: a window, function or action marked as hand-written in the spec keeps its signature generated but not its body, the designer shows it as owned by code, and the stack's compiler reports a body that no longer fits the spec; (c) a three-way merge as the safety net for a generated file edited anyway: every generation writes a stamp, `clarity-generated.json` at the root of the generated project (the Clarity version, a hash of the spec, and a hash of every generated file), so Clarity sees which generated files were edited by hand and which output they started from; it keeps that last generated output, merges old to new generated into the edited file, and shows conflicts in its own merge view. Every generated project gets an `AGENTS.md` and `CLAUDE.md` saying which files are generated and which are owned, and that new logic belongs in an engine, so other AIs edit the right ones. Reading hand edits back into the spec is not planned, apart perhaps from layout later. Raised by the user (2026-10-23) | |
| LR1 | Learning with Clarity, for someone who knows a little programming and wants to see how real desktop applications are built, or to pick up C#, Go, Rust, C or C++ by comparison. (a) A beginner project in New project: one stack, one SQLite source and one window, with the parts of the IDE a first project does not need (data sources, migrations, themes, engines, diagrams) out of sight until the person opens them. (b) The generated code beside the Clarity code: picking a handler, function or field shows what the chosen stack made of it, with the matching lines marked, and the same piece can be shown for another stack to compare; read only, as all generated code. (c) A tutor setting for the agent: it explains, asks questions and points at mistakes and at the checker's problems, but writes no project files unless the person asks, and answers in the language the person picked for Clarity; the setting's own label and description come from the translations. Raised by the user (2026-10-23) | |
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
11. **Turning a usability rule off** (section 5.18). A comment directive beside the part (`# ux off long-form: a data entry screen for experts`), a property on the part or window, or a list in the project file; and whether an agent may turn a rule off by itself or only propose it.
12. **Custom parts** (section 5.19). Whether Clarity's agent may write a native part's C or C++ and a .NET part's adapter, or only the declaration (today it writes only Clarity and the Markdown of resources); and how a part shows on the design canvas beyond a box or a picture.
