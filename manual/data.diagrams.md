# Diagrams

Designing data and code in diagrams: the ERD and class designer, Apply and From project, and the Mermaid Clarity reads.

## Where diagrams live

A diagram is a Mermaid file in the project's `diagrams` folder, such as `diagrams/shop.mermaid`. The sidebar lists them under **Diagrams**; **+ New diagram** asks for a name, a kind, and whether it starts from the project or with a small example of the kind.

| Kind | What it does |
| --- | --- |
| ERD | Entities, their fields and relations. Designed on the canvas and mapped onto the project. |
| Class diagram | Entities, enums, engines and modules with their functions. Designed on the canvas and mapped onto the project. |
| State, sequence, activity, architecture | Drawn from their text; you change them under **Text**. |

A project keeps one ERD: data changes go into it. Clarity refuses a second one and names the ERD there is. Other kinds have no such limit.

## The header

- **From project** writes the diagram again from the project, of everything or of only the boxes it draws, and keeps where the boxes stand. One Undo takes it back.
- **Rename** renames the file.
- **Delete** deletes the diagram, after asking. Undo brings it back.
- **Design** shows the canvas, **Text** the Mermaid text.

## The canvas

The zoom bar at the top left zooms out, shows the zoom (a click goes back to 100%), zooms in, and centers the drawing in view with the target button. On an ERD or class diagram, **Arrange** lays every box out again: what is referred to above what refers to it.

The wheel pans; Ctrl (Cmd on macOS) with the wheel zooms. Drag a box to move it; on a class diagram, drag a module's name to move its frame with everything in it. Drag a box from the palette under the zoom bar onto the canvas to add it, or pick a line kind and drag from one box to another. Press a line to add a bend point there and drag it; a double click on a point takes it away. A click on a box or a line edits it in the properties.

## The properties

The properties lie over the right side of the canvas: what you picked, and the diagram's proposals. Drag their left edge to make them wider or narrower. **Hide** folds them away to a **Properties** tab at the right edge, which brings them back. Clarity remembers the width and whether they are folded.

## Proposals and Apply

An ERD or class diagram proposes; the project decides. Under **Proposals** Clarity lists what the diagram would change in the project: a field to add, a type to change, a modifier to take away. Tick the ones you want and press **Apply**: Clarity makes them, checks the whole project, and one Undo takes them all back. A removal is never ticked for you. When the diagram and the project agree, it says so and proposes nothing.

Write every field with its modifiers (`text(80) name "required"` in an ERD, `+name: text(80), required` in a class diagram): a field written without them proposes taking them away. A new field's label becomes a key with its text in every language file. A change to the data still needs its migration step for a database; see [Migrations](data.migrations.md).

## Diagrams and agents

An agent designs data in the project's ERD first: it writes the change into the ERD, says what it proposes, and waits. Apply it yourself, or ask the agent to. Once the data is in, the agent places the new fields on the forms that show them. See [Clarity's agent](ai.agent.md).

## Writing a diagram by hand

Under **Text** a diagram is Mermaid, which Clarity reads as you type and checks line by line. Blank lines, `%%` comments, a leading frontmatter block and `direction` lines mean nothing to the project. The `%% clarity:` lines say where each box stands and how a line bends (`%% clarity: Customer at 24,109`, `%% clarity: Order -> Customer : customer via 300,120`): keep them when you write the diagram again, and **From project** keeps them too.

Field names are camelCase; a name written in snake case (`issued_on`) becomes `issuedOn`. Clarity gives every entity its own key, `id`, and adds it itself.

### ERD

An ERD starts with `erDiagram`. It holds entity blocks of attributes, and relationships between entities.

```text
erDiagram
    Entity {
        type name [PK | FK | UK] ["modifiers"]
    }
    Left ||--o{ Right : label
```

```
erDiagram
    Customer {
        text(80) name "required"
        email email UK
        CustomerKind kind
        text[] tags
    }
    Order {
        text(12) number UK "required"
        Customer customer FK "required"
        money total "computed"
        decimal discount "decimal(5, 2), min 0"
    }
    OrderLine {
        Order order FK "required"
        integer quantity "default 1"
    }
    Order }o..|| Customer : customer
    OrderLine }o..|| Order : order
    %% clarity: Customer at 24,109
    %% clarity: Order at 408,93
```

- An entity block is an entity: `Customer {`, with an alias if you like, `Customer["Our customers"] {`. An entity named only in a relationship is an entity too.
- An attribute `type name` is a field in Clarity's types: `text(80)`, `money`, `email`, an enum's name. A list is `text[] tags` (text, integers, decimals or dates, or an enum's values), or `Order[]` for an entity's records. Some words of other languages read as Clarity's: `string`, `varchar` and `char` as `text`, `int` and `long` as `integer`, `boolean` as `bool`, `timestamp` as `datetime`, `float` as `decimal`.
- `UK` is `unique`. The comment holds every other modifier as the module writes it (`"required"`, `"computed"`, `"min 0"`, `"default 1"`, `"owned"`), and, first, a type that Mermaid's word cannot hold (`"decimal(5, 2), min 0"`). An attribute left without its modifiers proposes removing them.
- A ref is an attribute with `FK` and the entity's name as its type: `Customer customer FK "required"`.
- `PK` on `id` is left out; `PK` on another attribute makes it a unique field, with a warning that Clarity keys every entity by `id`.

A relationship is `Left ends Right : label`, the ends six characters: how many stand at the left (`|o` zero or one, `||` exactly one, `}o` zero or more, `}|` one or more), the line (`--` identifying, `..` not), and how many stand at the right (`o|`, `||`, `o{`, `|{`). It gives the side with many a ref to the other, named after that entity in camelCase, unless the entity already draws a field of that name or type. Exactly one (`||`) at the side referred to makes the ref required; zero or one leaves it optional. An identifying line (`--`) also gives the one side an owned list of the many, named in the plural (`orders`). Many on both sides proposes nothing: it needs an entity between them. A line between an entity and an enum gives the entity a field of the enum's type.

Clarity marks as an error a line it cannot read ("write an entity `Name {`, an attribute `type name`, or a relationship `A ||--o{ B : label`"), a type it does not know, a decimal without its precision and scale ("needs its precision and scale, as \"decimal(10, 2)\""), and modifiers that a field line would refuse.

### Class diagram

A class diagram starts with `classDiagram`.

```
classDiagram
    namespace customers {
        class Customer {
            +name: text(80), required
            +email: email, unique
            +kind: CustomerKind
        }
        class CustomerKind {
            <<enumeration>>
            Private
            Business
        }
    }
    namespace sales {
        class Order {
            +number: text(12)
            +customer: Customer
        }
    }
    class sales {
        <<module>>
        +totalOf(order: Order) money
    }
    Order "*" --> "1" Customer : customer
    sales ..> Customer
```

- Each class stands in the `namespace` of its module and is named by its own name everywhere, in relations too: `Order`, never `sales.Order`. A class name is written once.
- An attribute carries its type and modifiers as the module writes them (`+name: text(80), required`, `+total: money, computed`), or Mermaid's `text name`; one left without its modifiers proposes removing them. An attribute typed with an entity's name is a ref to it. The marks `+ - # ~` before a member and `$ *` after it are read past.
- A class with attributes is an entity; `<<enumeration>>` (or `<<enum>>`) with its values, one a line, is an enum; `<<engine>>` with methods is an engine; a `<<module>>` class, named as its module and outside the namespaces, holds the module's functions and actions. A method is `name(param: type) result`; `String` reads as `text` and `List~Order~` as `list<Order>`.
- `A "*" --> "1" B` is a ref to B on A; without counts an association is many to one. A composition (`*--`) gives the whole an owned list, an aggregation (`o--`) a plain list. The counts are `"1"`, `"0..1"`, `"*"` and `"1..*"` (and `"0..*"`, `"n"`, `"many"`).
- A uses arrow from a `<<module>>` or `<<engine>>` (`sales ..> Customer`) is drawn only: the code makes these, not the diagram.
- Lines Mermaid uses for looks (`note`, `click`, `link`, `callback`, `style`, `classDef`, `cssClass`) are drawn only.

Clarity marks as an error a line it cannot read and an entity's member without a type ("'name' needs a type, as name: text"). It warns, and draws the line only, for inheritance ("Clarity has no inheritance"), another dependency than a uses arrow, a uses arrow from a class without code, counts it does not read, and a stereotype it does not map. It warns too of attributes on an enum (values only) or on an engine (what an engine keeps is written in the engine), and of methods on an entity (functions belong to a `<<module>>` class).

## Related pages

- [Entities and enums](data.entities.md)
- [Migrations](data.migrations.md)
- [Engines](code.engines.md)
- [Functions and actions](code.functions.md)
- [Clarity's agent](ai.agent.md)
