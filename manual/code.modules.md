# Modules

How a project's code is split into modules: the `module`, `use` and `source` lines, what a module declares, how lines, names and values are written, constants and records.

## Lines and blocks

Clarity is line based. Each declaration, setting, part or statement is one line, and a block is the lines indented below it.

- Indent with spaces. A tab is an error: "tabs are not allowed for indentation, use spaces". Four spaces per level is the convention, and the designer writes four.
- A comment starts with `//` and runs to the end of the line. It stands on a line of its own or after code. The designer keeps comments when it rewrites a file.
- Blank lines are free, and they are kept too.

## Names

A name starts with a letter, followed by letters and digits: `customer`, `vatNumber`, `OrderLine`. Names are case sensitive.

- Entities, enums and their values, records, windows and the app are `PascalCase`.
- Every other name is `camelCase`: letters and digits only, starting with a small letter. That holds for the parts of a window and their handlers (`newGame`, `square1`, `saveClicked`), and for fields, settings, data, functions, actions, parameters, locals, constants and what an engine keeps (`vatNumber`, `withinCredit`). The checker reports any other, with the camelCase form: "local 'board_clicked' is not camelCase: letters and digits, starting with a small letter, such as 'boardClicked'".
- Some names are left as written: the functions an engine answers its host by (`clarity_tick`, `clarity_draw`, `clarity_pointer`, `clarity_key`, `clarity_open`, `clarity_alloc`, `clarity_free`), the name `_`, and a camelCase name with one `_` after it, which steps around a word Clarity keeps (`key_`).
- A field is stored in the column of its name in snake case (`vatNumber` in `vat_number`) unless it names its `column`. Every stack turns names into its own style.

Every part has a name (`saveOrder`, `vatNumber`), which its events and code go by. The designer names a part after what it shows or is for when it is placed (`ordersTable`, `email`), or `button1` when it does not know yet, and renames a numbered part after its purpose once it knows it (a label given `for vatNumber` becomes `vatNumberLabel`), unless code goes by the number. A part without a name is an error, and **Name them** under Problems names every such part in the file the same way.

Words such as `page`, `data` and `bind` are keywords only where the grammar expects them. Elsewhere they are ordinary names, so a field may be called `data`. In expressions, `and`, `or`, `not`, `in` and `contains` are reserved; `div`, `mod` and `matches` are operators only after a value. A word that is a keyword of the stack's own language (`type`, `match`, `struct`, `switch`, `class`) is still a good Clarity name: the generated code renames it where the stack needs that, so you never see it.

## Values

- **Keys** name the texts people read: `@saveCaption`, `@customerCardTitle`. A key's text is in the language files, one per language; see [Languages and formats](languages.md). A title, a caption, a label and a message are keys. Only a text without letters, such as a calculator's `"7"`, is written as it is. **Make texts keys** under Problems writes a key for each text a project wrote as it was, and gives it a text derived from the name where there was none: `vatNumber` becomes "Vat number", `CustomerCard` becomes "Customer card".
- **Strings** are written in double quotes: `"logo.png"`. Inside them `\"` is a quote, `\\` a backslash and `\n` a new line. Other backslashes are kept as written, so `"^[0-9]\d+$"` is a valid pattern. In code a string is also read for slots: `{customer.name}` is a value, `{{` and `}}` are braces; see [Expressions](code.expressions.md#text-with-slots). Strings in settings, such as patterns and file names, have no slots.
- **Numbers** are written without quotes: `12`, `9.95`, `-3`.
- **Booleans** are `true` and `false`.
- **Enum values** are written bare (`Business`) where the expected type says which enum is meant, and qualified (`CustomerKind.Business`) where it does not.
- **`none`** is the empty value of a date, a date and time, a reference, a picture or a file. Empty text is `""`.

## A module

The project file (`shop.clarity`) holds the project's settings; see [The project file](project.file.md). Every other `.clarity` file of the project is a module. It starts with `module`, then the modules it uses, then its declarations:

```clarity
module sales

use customers

entity Order
    customer: ref Customer
```

A module whose entities live in another data source names it:

```clarity
module erp

use customers
source erp

entity Invoice
    number: text(20), unique
    customer: ref Customer
```

A window, a view and the app are not declarations of a module: each is a `.cxaml` file of its own in the module folder's `design` folder, whose root names the module and the page (`<Window class="sales.OrderForm">`). Their code is a `code` block in the module; see [Handlers](code.handlers.md) and [Windows](design.windows.md).

In the IDE, open a module from Modules in the project tree. Its view shows the module's file, its **Uses** (a checkbox per other module, which writes the `use` lines), what it **Declares** (each opens in its own editor), its **Functions and actions**, **Engines**, **Themes** and **Data events**.

### `module`

The first line of a module file: its name. Comments and blank lines may come before it.

    module name

- `'module' must be the first declaration in a file`.
- Two files that declare the same module are an error: "module 'sales' is already declared in ...".

### `use`

Makes the declarations of another module visible in this file, one `use` line per module.

```text
use other
```

- Without it, a name from another module is an error that says which `use` is missing: "'Customer' is declared in module 'customers', add 'use customers'".
- `unknown module 'x'` for a module the project does not have; "a module cannot use itself".
- A name is declared once per module. Two modules may declare the same name, but a file that sees both (its own module and the modules it uses) cannot use that name: "'Status' is ambiguous, it is declared in modules sales, erp".

### `source`

Places the entities of this module in a data source of the project file.

```text
source name
```

- Without it, the entities live in the first data source.
- In a SQL database (sqlite, postgres, mysql or mssql) Clarity stores them. In a web API (`rest`) the service keeps them: no migration covers them, and `find`, `save` and `delete` do not reach them; the module's actions read and send them with `http ... from name`; see [Calling web APIs](code.web-apis.md).
- A reference to an entity in another database is stored as its id, without a foreign key, so the database cannot stop a referenced record from being deleted. A query reads one database, so it cannot go through such a reference.
- "a module has one 'source'"; `unknown data source 'x'; the project has ...`.

## What a module declares

| Declaration | Page |
|---|---|
| `enum Name`, `entity Name` | [Entities and enums](data.entities.md) |
| `settings` (one per project) | [The project file](project.file.md) |
| `record Name` | [`record`](#record) below |
| `const name = value` | [`const`](#const) below |
| `code Name` | [Handlers](code.handlers.md) |
| `function name(...)`, `action name(...)` | [Functions and actions](code.functions.md) |
| `engine Name`, `engine Name from "file.wasm"` | [Engines](code.engines.md) |
| `cli Name`, `command name(...)` | [Commands](code.commands.md) |
| `part Name` | [Custom parts](design.custom-parts.md) |
| `library Name` | [Libraries](code.libraries.md) |
| `mapping Name of Entity from format` | [Data formats](code.formats.md) |
| `toast name` | [Toasts](code.toasts.md) |
| `theme Name` | [Themes](themes.md) |

Any other first word is an error that lists these. A `page` or `app` written in a module is reported: a window, view or the app has a file of its own.

### `const`

A value written out once and read by name, never set.

```text
const name = value
const name: type = value
const name: picture = "file.svg"
```

```clarity
module game

const lives = 3
const speed: decimal = 1.5
const levels = [1, 2, 3]
const pillRed: picture = "pill-red.svg"
const welcome = @welcomeText
```

- **Where.** A `const` line in a module is read from its code, its windows' handlers and its engines. A `const` inside an engine written in Clarity is read by that engine, and comes before a module's constant of the same name.
- **What.** A value written out: a number, text, `true` or `false`, a key (`@welcome`), an enum's value written with its enum (`Level.Easy`), or a list of them. Its type is the value's own, or the one written after the name; a whole number in a `decimal` constant is a decimal.
- **`picture`** is text naming a picture of the resources folder; the checker finds the file there, once, where the constant is declared. A canvas's `image` draws it.
- Generation writes each constant's value where it is used, so every stack and every engine has it alike.

The checker says: "there is already a const lives"; "const lives is written out: a number, text, yes or no, a key, an enum value or a list of them" (for a value that is computed); "const speed is decimal, not text"; "const pill is a picture: the name of a file of the resources folder, as in "pill.svg""; and "lives is a constant and cannot be set" where code sets one. A constant's name is camelCase.

### `record`

A value with fields that belongs to no table: declared like an entity, never saved or found.

```text
record Name
    field: type [, default value]
```

```clarity
module drawing

record Dot
    x: integer
    y: integer
    size: decimal, default 1.5
    tags: list<text>

function moved(d: Dot) -> Dot
    let copy = d
    copy.x = copy.x + 1
    return copy
```

- **`new Dot`** has every field at its default: the one written with `default`, or 0, empty text, `false`, an empty list. A record's decimal needs no precision and scale, since it has no column.
- **`d.x`** reads a field and **`d.x = 3`** sets it.
- **Copied, never shared.** Putting a record into a list, giving it to a variable, a window's data or a function, or reading it from a list or from `grid.selected` makes a copy. After `let d = new Dot`, `dots = dots + [d]` and `d.x = 3`, the dot in the list keeps its own `x`. Every stack behaves alike: Rust, Go, C and C++ copy the struct, C# copies the object.
- **Lists of records** (`list<Dot>`, `[a, b]`) are read by item and by `for`, and grow by a list of the same records: `dots + [d]`. An empty one names its type: `let dots: list<Dot> = []`.
- **Fields** are values: text, numbers, `bool`, dates, enums and lists of them, or a reference to a stored record (`project: ref Project`), kept as its id. Not pictures or files, and no reference to another record. An engine uses records of values only: "record Node refers to stored records, which an engine has no way to; an engine uses records of values only".
- **A record is never none.** It always has its fields; compare a field instead: "Dot is a record, which is never none; compare one of its fields". Only a part's selected row is set to `none`.
- **Never stored.** `save`, `delete`, `reload` and `find` on a record are errors: "Dot is a record, which is never stored".
- **In windows.** A window holds one (`<Data name="picked" type="Node" />`, new when the window opens) or a list of them (`type="list&lt;Node&gt;"`, empty when it opens). A `list`, `table`, `tree` or `treetable` shows such a list as it shows records of an entity: `columns` and `show` name its fields, `grid.selected` is the record of the chosen row, and `on select` and `on open` run. A record has no id, so a row is its place in the list. Giving the window a new list (`nodes = outline(projects)`) selects no row; code selects one again with `nav.selected = ...`. People do not edit a table of records (`edit`) or move its rows (`reorder`): build the list in code.
- **Engines** written in Clarity give and take records as JSON objects of their fields; see [Engines](code.engines.md).

A tree of records, filled when the window opens:

```clarity
module modelling

record Node
    key: text
    parent: text
    label: text
    project: ref Project

function outline(projects: list<Project>) -> list<Node>
    let all: list<Node> = []
    let top = new Node
    top.key = "land"
    top.label = "Landscape"
    all = all + [top]
    for p in projects
        let n = new Node
        n.key = p.name
        n.parent = "land"
        n.label = p.name
        n.project = p
        all = all + [n]
    return all

code Workspace
    handler loadNodes
        projects = find p in Project order by p.name
        nodes = outline(projects)
        nav.selected = nodes[1]

code WorkspaceView
    handler chosen
        if nav.selected.project != none
            message nav.selected.project.name
```

```xml
<!-- modelling/design/Workspace.cxaml -->
<Window class="modelling.Workspace" onopen="loadNodes">
  <Content name="main" at="1,1" span="12" rows="20">
    <View name="WorkspaceView" />
  </Content>
</Window>
```

```xml
<!-- modelling/design/WorkspaceView.cxaml -->
<View class="modelling.WorkspaceView">
  <Data name="projects" type="list&lt;Project&gt;" />
  <Data name="nodes" type="list&lt;Node&gt;" />
  <Tree name="nav" at="1,1" span="4" rows="20" bind="nodes" rowKey="key" parentKey="parent" show="label" onselect="chosen" />
</View>
```

See also [Statements](code.statements.md), [Expressions](code.expressions.md) and [Functions and actions](code.functions.md).
