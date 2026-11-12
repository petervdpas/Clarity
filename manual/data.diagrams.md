# Diagrams

Designing data and code in diagrams: the ERD and class designer, Apply and From project.

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

The Mermaid that Clarity reads, with an example of each kind it maps, is in the language reference, section 13 "Diagrams".
