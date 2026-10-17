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

```
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
| C# | WebView2 or Photino | ASP.NET |
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
| `on click: save_click` | `Command="{Binding SaveClick}"`, a generated command that calls the handler |
| data and validation | a generated ViewModel with typed properties and validation from the dictionary |

The designer is the same for every UI: a window is designed once, and the stack decides how it is rendered. For a native toolkit the preview shows the layout faithfully but not pixel for pixel; the real window appears with Run.

**Output.** Clarity keeps an `out/` folder in sync with the project, the way the preview follows every change: a complete project for the chosen stack that its own toolchain compiles (`Cargo.toml`, `go.mod`, a `.csproj`, `CMakeLists.txt`). Nobody edits it; it is regenerated deterministically from the Clarity files. **Run** on the Preview tab compiles it and starts the real application; **Build** produces a release. Clarity calls the native toolchains and never replaces them.

**AI.** Clarity knows the whole project as a structured model: data, windows, handlers, diagnostics, the edit history, the generated code, compile errors and, later, runtime measurements. Through its MCP server (section 17.3) an AI can read all of it. An AI changes a project only with the edits the designer uses, so the checker judges its changes like anyone's and Undo takes them back. Optimization works in two loops:

- **Inside a project:** the AI proposes improvements (layout, data model, slow handlers, duplicated logic); the developer accepts or rejects them, and the rules decide what is valid.
- **For Clarity itself:** patterns that recur across projects (the same field added everywhere, generated code that is always slow) become proposals for better templates, emitters and rules, and improve Clarity as a whole once a human approves them.

Generation, checking and enforcement stay deterministic (section 2): the AI proposes, Clarity decides what is valid, humans decide what is accepted.

Superseded by this: the frontend framework choice (sections 7.2 and 11.4 where they assume Svelte, Vue or React output) and the stack milestones M2, M5, M7 and M10 in the roadmap.

**The project file (2026-10-16).** A project is one file, `<name>.clarity`, which is what you open, with a folder of the same name next to it that holds the modules, the output and the data. The settings are written in Clarity, so the whole project is one language:

```
examples/
  shop.clarity          the project: open this
  shop/
    customers.clarity   modules
    sales.clarity
    out/                generated application (setting `output`)
    data/
```

```
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

```
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

`clarity gen` (and later Run and Build) writes the application into `out/`. Generation is deterministic: the same project gives the same files byte for byte, unchanged files are not rewritten, and files that are no longer generated are removed. `target/`, `data/` and `Cargo.lock` belong to the build and stay.

- **Layout:** a Cargo project with one Rust module per Clarity module (`src/sales.rs`), `src/schema.rs` with the migration steps, `src/main.rs`, and the `clarity-runtime` crate copied in, so `out/` builds on its own.
- **Records:** every entity is a struct with an `id` and a `Record` implementation; enums are Rust enums stored by name. Text is `String` (empty means no value), integer `i64`, decimal and money `Decimal`, date and datetime `Option<...>`, and a `ref` is the id of the record it points at. Reading `order.customer.credit_limit` loads the customer through the database. List fields are not generated yet; the generator says so.
- **Storage:** SQLite, in the file the data source names; a relative location is in the project's folder, for Clarity and the application alike (so the development database is `shop/data/shop.db`, not inside `out/`). The project's migration steps become SQL (a project loaded without its `migrations` folder gets the whole dictionary as one step). Changing or removing a column builds the table again and copies the rows, converting values where the stored type changes. Steps run with references unchecked and are kept only when every reference still points at a record. Decimals and dates are stored as text, so nothing loses precision. A record that is still referred to cannot be deleted.
- **Logic:** functions and actions become Rust functions that take the database and return a `Result`. `require` and `ensure` become runtime checks whose failure names the condition as written in Clarity. The emitter takes every type from the checker, so the checker is the only place types are decided.
- **Window data:** list data loads all records when the window opens, single data starts as a new record, and `open Window with ...` passes records in. Handlers narrow a list with a query (`customers = find c in Customer where ...`).
- **Queries:** `find` becomes a `Query` of the runtime: a tree of conditions on columns with the values as parameters, which the runtime writes as SQL. Generated code never contains SQL text, so another database is another way of writing the same tree in the runtime. Decimals and money are stored as text and compared and ordered as numbers. Unique text fields get a unique index that leaves empty text out, since empty text means no value.
- **The binding.** The renderer and the backend are bound three ways, all declared in Clarity: data (`bind customer.name`: the window's data goes to the parts, and what the user types goes back with the next event), events (`on click: save_clicked`: the screen reports the event, the backend runs the handler) and properties (`save.enabled = ...`: the reply carries part properties to the screen). Each window becomes a Rust struct of its data; each handler a function on it with a `Ui` for messages, windows to open and `close`. The frontend is the designer's renderer, built once (`npm run build:runtime`) and copied into `out/ui`; the shell is Tauri. For a native toolkit (G2) the same Clarity becomes that toolkit's own bindings.
- **Run and Build** on the Preview tab generate `out/`, run cargo there with its output in an Output panel, and start the program (Run) or build a release (Build); Stop ends it. Cargo's `target`, Tauri's `gen`, the lock file and the application's `data` are left alone by generation.
- **The output folder** is set in the project file (`output "out"`, the default): a folder in the project's folder or a full path elsewhere, never the project folder itself. Because generation removes what it no longer produces, Clarity writes only into an empty folder or one that carries its `.clarity-out` marker.
- **Fluid display.** Windows always resize with the screen, on the web and the desktop alike. The columns share the width; rows that hold a part that grows (a table, a list, an image or a canvas, or a panel, tabs or a splitter that holds one) share the extra height, and the other rows keep their height, so inputs never stretch. A window smaller than its design scrolls. The designer keeps every row at its fixed height, so placing parts stays exact. A native toolkit gets the same rule from its own layout: in Avalonia, rows of growing parts become `*` rows. Sidebars in a running application have a splitter on their inner edge; the designed width is where they start.

### 1.5 Languages, formats and themes (2026-10-17)

An application made with Clarity is shipped to people who read another language, write dates another way and expect their own house style. All three are part of the project, written once and carried to every stack. Nothing here may depend on a browser: the C# stack draws with Avalonia, so a theme is not CSS and a translation is not a JavaScript library's file. Each stack's runtime maps the same project onto its own means.

**Formats (I1a).** How values look depends on the locale, not on the developer.

- The project names its base language and, for money, its currency: `language en` and `currency EUR` in the project file.
- Dates, times, numbers and money are shown and entered in the locale of the person using the application: `17-10-2026` or `10/17/2026`, `1.250,50` or `1,250.50`, `€ 1.250,50`. The locale comes from the system, and the application can offer a choice.
- A field's `format` (accepted today, not used yet) picks a format by name rather than by pattern: `format short`, `format long`, `format percent`. Named formats stay the same across stacks and locales; a pattern does not.
- Money shows its currency's decimals, so `0.0000` from a server database reads `€ 0,00`.
- Values are stored the same way in every locale; only showing and entering them changes.

**Text with slots (I1b).** A translator must be able to move the parts of a sentence, so text in code names its values instead of adding them together:

```
message "Saved {customer.name}"
refuse "Order {order.number} is over its customer's credit limit"
```

- A slot is a name or a member path in braces, checked like any other expression; `{{` writes a brace. Slots are paths only, so a translator sees `{customer.name}` and knows what goes there.
- A slot shows its value in the locale's format (I1a).
- `+` on text stays valid, but the checker suggests slots for text that is shown: a message, a refusal, a caption.

**Translations (I1c).** Every text a person sees is in the project already: labels, help texts, titles, captions, enum labels, menu items, messages and refusals. Clarity collects them; nobody marks them.

- The text as written is the base language. Translations are files per language in the module folder, `translations/nl.clarity`, one per module:

  ```
  translation nl
      "Customers" "Klanten"
      "Saved {customer.name}" "{customer.name} opgeslagen"
      "Order" context "the verb" "Bestellen"
  ```

- A text is found by what it says, within its module. A text whose base changes loses its translation, which is what should happen: the old translation no longer says the same thing. `context` separates one base text that needs two translations.
- A translation must keep the slots of its base text; the checker reports a missing or unknown slot. Missing translations are listed per language, as information rather than warnings, so a project can grow a language step by step.
- The generated application carries the texts as explicit literal key maps (section 6.7), and changes language while it runs, without restarting.
- The designer gets a Translations table on the Project tab (base text, where it is used, one column per language) and a language choice for the canvas and Preview, so a window can be checked in German before it ships.

**Themes (T1).** A theme is a set of named values, not a style sheet:

```
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
- Parts pick a named style, `button "Delete" style danger`, never raw colors, so a design stays consistent and the theme can change without touching windows.
- The project file names the theme its application uses; a project without one gets Clarity's own, the look the renderer has today, which already follows the system's light or dark mode.
- The web renderer turns a theme into CSS variables, and Avalonia into resources and styles. The designer draws with the same renderer as the application, so the canvas shows the theme as it will ship.
- The designer gets a theme editor with a live preview of a sample window.

**Order.** Formats first, since they fix visible gaps (dates as raw text, money with four decimals). Then text with slots, since it changes the language and is cheapest to settle while little code is written the old way. Then translations, which build on both. Themes stand apart and can come at any point; they mostly touch the renderer.

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

```
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

```
module sales

use customers
```

`use` follows the module map: a spec module can only reference modules it may use.

### 5.2 Entities and enums

Entities are the data dictionary. Everything about a field is defined here once: type, validation, label, help text, display format. Screens, API types, database schema and frontend validation are all generated from it.

```
enum OrderStatus
    Pending
    Approved
    Shipped
    Cancelled

entity Customer
    name: text(80), required, label "Name"
    email: email, unique, label "E-mail"
    credit_limit: money, min 0, default 0, label "Credit limit"
    notes: text, help "Internal notes, not shown to the customer"

entity Order
    number: text(12), unique, pattern "^ORD-[0-9]{8}$"
    customer: ref Customer, required
    status: OrderStatus, default Pending
    lines: list<OrderLine>, owned
    total: money, computed
```

Types: `text`, `text(n)`, `integer`, `decimal(p, s)`, `money`, `bool`, `date`, `datetime`, `email`, `url`, `id`, enums, `ref Entity`, `list<T>`.

Field modifiers: `required`, `unique`, `min`, `max`, `pattern`, `default`, `computed`, `owned` (for lists: lifetime bound to the parent), `label`, `help`, `format`.

`computed` fields are declared in the spec and implemented in hand-written code (section 6.4), like actions.

### 5.3 Actions

Actions declare an operation: its signature, contracts and effects. The body is never in the spec.

```
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

```
page CustomerEdit: window
    title "Customer"
    data customer: Customer
    data orders: list<Order>
    grid 12 columns

    label "Name" at 1,1 span 3
    textbox at 4,1 span 6 bind customer.name
    dropdown at 4,2 span 4 bind customer.kind
    checkbox "Active" at 4,3 span 3 bind customer.active
    panel "Orders" at 1,5 span 12 rows 6
        table at 1,1 span 12 rows 5 bind orders columns number, ordered_on, total
    button "Close" at 11,12 span 2 on click -> CustomerBrowse
```

- **Page kinds:** `window` (a top-level window), `dialog` (modal) and `view` (a part embedded in another page).
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

```
button "Save" name save at 9,10 span 2 on click: save_clicked
textbox name email at 4,2 span 6 bind customer.email on change: email_changed
```

`on click: save_clicked` wires the event to a handler, written in Clarity in the window's code behind (section 1.1). Parts placed in the designer get a free name at once (`button1`, `textbox2`), and templates name their parts after the field or caption, so events can be wired straight away. `on click -> Window` remains as the one built-in action, opening a window, so the preview can navigate before code behind exists; a click is wired to a handler or opens a window, not both.

#### Forms, labels and help texts

A label, its input and the input's help text are three separate parts, each placed on the grid like any other. `for` ties a label or help text to an input by its part name:

```
form name details at 1,1 span 12 rows 4 bind customer
    label "Name" at 1,1 span 3 for name
    textbox name name at 4,1 span 6 bind customer.name placeholder "Full name"
    help at 10,1 span 3 for name
    label "Kind" at 1,2 span 3 for kind
    radio name kind at 4,2 span 6 bind customer.kind
button "Save" name save at 9,6 span 2 key "Ctrl+S" default on click: save_clicked
button "Close" name close at 11,6 span 2 cancel on click: close_clicked
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

```
app Shop
    main CustomerCard
    menu
        item "File"
            item "New customer" key "Ctrl+N" on click -> CustomerCard
            separator
            item "Quit" key "Ctrl+Q" on click -> quit
        item "Help"
            item "About" on click -> About
    ribbon
        tab "Home"
            group "Customers"
                button "New" icon "add" large on click -> CustomerCard
                button "Browse" icon "list" on click -> CustomerBrowse
    sidebar left span 3
        list at 1,1 span 3 rows 10 bind recent show name
    footer
        status "Ready"
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

```
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

```
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

### 5.8 Canonical form

`clarity fmt` defines the only valid formatting. The parser keeps comments and blank lines (a lossless syntax tree), so the designer, the formatter and humans can all edit the same file without destroying each other's work.

### 5.9 Code behind (2026-10-16)

A window's handlers live in a `code` block with the window's name, in the same file as the window. The designer rewrites only the `page` block, so layout edits never touch the code.

```
page CustomerForm: window
    title "Customer"
    data customer: Customer
    ...
    dropdown name kind at 4,2 span 4 bind customer.kind on change: kind_changed
    textbox name vat_number at 4,3 span 4 bind customer.vat_number
    button "Save" name save at 9,8 span 2 on click: save_clicked

code CustomerForm
    handler kind_changed
        vat_number.visible = customer.kind == Business

    handler save_clicked
        if customer.name == ""
            message error "Name is required"
            return
        save customer
        message "Saved " + customer.name
        close
```

**Where handlers run.** In the backend. When an event fires, the UI sends it with the window's state; the handler runs and the backend returns what changed: data, part properties, messages, windows to open or close. With a native toolkit (Avalonia) the same handler becomes a ViewModel command. The UI never holds behavior of its own.

**Names in a handler.** A handler sees the window's data (`customer`), its parts by name (`vat_number`, `save`) and its own local values (`let`). Enum values are written bare (`Business`) when the expected type says which enum they belong to, or qualified (`CustomerKind.Business`). A name that is both data and a part is an error.

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

```
page CustomerBrowse: window
    data customers: list<Customer>
    on return: window_returned
    table name grid at 1,1 span 12 rows 8 bind customers columns name, credit_limit

code CustomerBrowse
    handler window_returned
        reload customers
```

This is how a list refreshes after a form that edits one of its records: in code, not in a setting. The browse template writes exactly this, so a new browse window refreshes from the start; delete the line or the handler to change it. In the designer, the window section of the properties lists the events with Wire, Code and a remove button, the same as for parts; the app's events are in the same place when the app is selected. A window or app handler counts as wired.

**Expressions:** literals (`12`, `9.95`, `"text"`, `true`, `false`), paths (`customer.name`, `orders.selected.total`), enum values, `not`, unary `-`, `* /`, `+ -` (`+` also joins text), comparisons `== != < <= > >=`, `and`, `or`, `x in [a, b]`, parentheses and `count(list)`.

**Types** come from the dictionary: `text` (also email and url), `integer` (also id), `decimal` (also money), `bool`, `date`, `datetime`, an enum, a record of an entity and a list of records. Integers widen to decimals; nothing else converts implicitly. The checker reports unknown names, wrong types, assignments to read-only properties, and handlers that are wired but missing (an error) or written but not wired (a warning).

Later, with D1 and beyond: queries (`find`, `where`).

**Design as text.** The designer and the text are two views of the same declaration. The Text button shows a window's `page` block (or the `app` block) in the slide-out panel; while it is typed the backend parses and checks it, and the designer draws the window from the text without saving it. Save writes it as one edit; until then the canvas does not take edits, because its positions belong to the saved file. Every canvas edit shows up in the text at once.

### 5.10 Functions and actions (2026-10-16)

Modules hold logic that is not tied to one window: functions and actions, written in Clarity and callable from any handler and from each other through `use`.

```
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

## 6. Generation

### 6.1 Shape and behavior

The central rule of spec-driven projects:

- **The spec owns the shape:** entities, enums, validation, actions with their signatures and contracts, screens, navigation, modules.
- **Hand-written code owns the behavior:** action bodies, computed fields, and frontend extension widgets, in the backend's or frontend's own language.

Shape can only come into existence through generation. To add a field, an endpoint or a screen, someone has to change the spec. This applies to humans and AI equally, and it is what forces both to work through the spec and its directives.

### 6.2 Neutral model and generators

```
spec files -> parser -> neutral model -> backend generator (one per language)
                                      -> shell generator   (one per desktop shell)
                                      -> frontend generator (one per framework)
```

Backend and frontend never generate against each other. Both meet at a transport contract derived from the neutral model: a schema of all types and actions. This keeps the number of generators at backends plus frontends, not backends times frontends.

Generators are deterministic: the same spec and generator version always produce byte-identical output.

### 6.3 Output layout

Example for Rust + Tauri + Svelte:

```
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

```
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
| C# | WebView2 / Photino |
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

```
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

```
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

```
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

```
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
| L1 | The Clarity behavior language: handlers written in Clarity in `code` blocks, with typed access to a window's parts and data (sections 1.1 and 5.9); wiring an event writes an empty handler; the code panel in the designer checks while you type | core done; open: modules with shared functions and actions (L2) |
| L2 | Backend modules in Clarity: functions and actions in modules, callable from handlers and each other, with `require`, `ensure` and `effects` (section 5.10); design as text with the designer following the text (section 5.9); modules show and edit their functions | done |
| G1a | Rust backend: `clarity-gen` writes a Cargo project into `out/` (deterministic, kept in sync by `write_out`), with records and enums from the dictionary, the SQLite schema as the first migration, functions and actions as Rust with contract checks, and the `clarity-runtime` crate copied in; `clarity gen`; the generated shop builds and runs, and generated logic is tested | done |
| G1b | Handlers and the event protocol, the web renderer as a standalone package, the Tauri shell, Run and Build on the Preview tab | done |
| E1 | Events beyond parts: window `on open`, `on close` and `on return` (back from a window it opened, to refresh), app `on start`; data events (before and after save) follow with D1 | done (section 5.9): window and app events, `reload`, Events in the designer, the browse template refreshes on return; data events stay with D1 |
| U1a | Forms and text parts (section 5.6): `form` bound to one record with `on submit`, `label for`, `help`, `heading`, `text`, `memo`, `password`, `switch`, `radio`; `placeholder`, `tooltip`, `readonly`, `taborder`, `key`, `default` and `cancel`; `error` and `readonly` as part properties in handlers; the form template and field drops place label, input and help together | done; open: a `time` part waits for a time field type, menu item keys are shown but not yet pressed at runtime |
| U1b | Actions and containers: icon button, split button, toggle button, button group; expander, accordion, scroll box, separator, link, badge, icon, progress, busy indicator; containers that share the column lines of the grid around them (as CSS subgrid does), so a label and an input in a panel line up with those outside it, and a label column for forms | |
| U1c | Data parts: tree, tree table, pager, search bar, lookup (a reference picked from a searchable list), chart; combo box, slider, spin box, color, file and tags inputs | |
| U1d | Window kinds and overlays: dialogs with OK and Cancel results, wizards with steps, context menus, popovers | |
| I1a | Formats (section 1.5): dates, times, numbers and money shown and entered in the locale of the person using the application; `language` and `currency` in the project file; named formats for `format`; money with its currency's decimals | |
| I1b | Text with slots (section 1.5): `"Saved {customer.name}"` in messages, refusals and captions, slots checked as expressions and shown in the locale's format | |
| I1c | Translations (section 1.5): text collected from the whole project, translation files per language and module, slots kept, missing translations listed, a Translations table on the Project tab, a language choice for the canvas and Preview, switching language in the running application | |
| T1 | Themes (section 1.5): a `theme` with named colors per light and dark mode, font, size, radius and named styles; `style` on parts; a theme editor with a live sample; CSS variables for the web renderer, resources for Avalonia | |
| C1 | Command-line applications: `targets = ["cli"]`; `command` declarations with typed arguments and options, written in Clarity like handlers; `print`, `ask` and `exit`; generated as a program without a window, over the same records and actions; needs queries (`find ... where`) from D1 | |
| D1a | Queries: `find c in Entity where ... order by ...` as an expression, `contains` on text, the `text` of unbound text inputs, handlers that run other handlers; the runtime builds the SQL from a condition tree; the browse template filters and refreshes through its filter | done (LANGUAGE.md section 10); open: conditions through references (`o.customer.name`), which need joins |
| D1b | Migrations (section 1.3): steps written in Clarity in `migrations/`, the schema from replaying them, pending changes as warnings and in the Project tab, proposed renames, confirmation for changes that lose data, `clarity migrate`, SQL per step with table rebuilds for SQLite, applied at startup with references checked | done (LANGUAGE.md section 4); open: data migrations (Clarity code that transforms rows), down steps, counting the rows a narrowing change would not fit, renaming enum values, a command-line switch to apply steps separately |
| D1c | Entities placed per data source: `source` in a module, checked against the project file; migration steps per SQL source; one database per source at run time, each record type knowing its source | done (LANGUAGE.md section 3) |
| D1f | Data events: before and after save, before and after delete, written in Clarity next to the entity | done (LANGUAGE.md section 8, Data events): `code Entity` with `before save`, `after save`, `before delete` and `after delete`, each checked and generated as an action on its record; `refuse "text"` as a statement; the runtime runs them through `Record` hooks in one transaction per database with the saves and deletes they make, and a failed save leaves a new record new; renaming an entity renames its events; open: editing events in the designer (the Code panel shows windows only), one transaction across databases |
| D1d | PostgreSQL, MySQL and SQL Server for owned sources: a runtime driver per database as a cargo feature, queries and record statements per dialect, migration steps written per dialect (types, keys, foreign keys added at the end of a step, unique indexes that skip empty text, type changes through a new column), connections from `env:NAME` | done (LANGUAGE.md section 4, Databases), tested against live servers (`crates/clarity-gen/tests/servers.rs`); open: TLS for PostgreSQL and MySQL, baselines and Import for these databases (D1e), money shown with the four decimals the servers store |
| D1e | External databases: `external` and `readonly` on a data source; `table`, `key` and `column` mapping; the runtime and generator using the mapped names; Import (Project tab and `clarity import`); comparing the dictionary with the live tables on every load; new crate `clarity-db` | done for SQLite (LANGUAGE.md section 4), Take ownership included: a baseline step that creates only what is missing, mapped names kept in the migration history with `map table`, `map key` and `map column` steps; open: keys that are not one integer column, Import, checking and baselines for PostgreSQL, MySQL and SQL Server |
| D2 | Web API data sources: REST endpoints mapped to entities, auth and paging, OpenAPI import | |
| D3 | Copying data between databases when a source changes kind | |
| G2 | C# with Avalonia: C# emitter and runtime, the Avalonia emitter (windows as AXAML with ViewModels), desktop first | |
| G3 | Go: emitter, runtime, Wails shell | |
| G4 | C and C++: emitters, runtimes, webview shell; Qt as a native toolkit for C++ if wanted | |
| W1 | The web target: each backend serves the web renderer over HTTP; Avalonia in the browser through WebAssembly | |
| A1 | AI through the MCP server: the whole project and its output readable, changes only as designer edits, proposals inside a project and for Clarity itself (section 1.2) | |
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
