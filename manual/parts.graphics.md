# Graphics parts

The parts of the Graphics group: an image, a seven-segment display, a canvas drawn by code or changed by the person, and a custom part an outside library draws.

## Placing a picture or a drawing

Drag a part from the **Graphics** group of the Parts palette onto the window. An image shows a picture of the resources folder or of a record; a display shows a number as a calculator does; a canvas shows what its draw handler draws, what an engine draws frame by frame, or a diagram, a picture or a drawing the person changes. **Custom part**, dropped, asks which of the project's custom parts it is, or makes a new one.

A canvas's properties have a **Canvas** group: **Surface**, **Units**, **Zoom and pan**, **Grid**, **Snap to the grid**, **Dot size** and **Dot color**, **Background**, and **Mode** (objects, with the list and the fields that place its records, paint or vector), with the mode's **Tools**, **Brush** and **Ink**, and **Toolbox**.

## Drawing on a canvas

A canvas is drawn by its draw handler, wired with `on draw`. The handler draws shapes in the canvas's own units (`units 200 x 100`, 100 x 100 without it), x to the right and y down; the canvas scales them to fit the part, in proportion and centered.

```xml
<!-- dial/design/MainView.cxaml -->
<View class="dial.MainView">
  <Data name="speed" type="integer" default="80" />
  <Canvas name="dial" at="1,1" span="12" rows="7" none="surface" ondraw="drawDial" />
  <Button name="faster" at="7,8" span="6" caption="@fasterCaption" key="+" onclick="change(10)" />
</View>
```

```clarity
code MainView
    handler drawDial
        let angle = 210 - speed * 240 / 200
        dial.clear("panel")
        dial.ring(50, 50, 42, "border", 3)
        dial.line(50, 50, 50 + 34 * cos(angle), 50 - 34 * sin(angle), "accent", 3)
        dial.text(50, 72, "{speed} km/h")

    handler change(by: integer)
        speed = speed + by
```

| Shape | Values left out | What it draws |
|---|---|---|
| `canvas.clear(color)` | the canvas's `background` | Fills the whole canvas; on a canvas with `zoom`, the whole part, which has no edges. |
| `canvas.line(x1, y1, x2, y2, color, width)` | width 1 | A line with round ends. |
| `canvas.rect(x, y, width, height, color)` | | A filled rectangle. |
| `canvas.circle(x, y, r, color)` | | A filled circle around x, y. |
| `canvas.ring(x, y, r, color, width)` | width 1 | A circle's outline. |
| `canvas.ellipse(x, y, rx, ry, color)` | | A filled ellipse around x, y, rx across and ry high from its center. |
| `canvas.path(points, color, width)` | width 1 | An open line through its points, a `list<integer>` x, y, x, y, .... |
| `canvas.text(x, y, text, color, size, align, style)` | color `"text"`, size 10, align `"center"`, no style | Text centered on x, y, or with `"left"` starting at x and with `"right"` ending there; slots are filled in the person's format, like a message. `style` is any of `"bold"`, `"italic"` and `"mono"` (the theme's code font), together as `"bold mono"`. |
| `canvas.image(x, y, width, height, picture)` | | A picture of the resources folder by its name (`"pill.svg"`, or a picture constant), stretched to fill the box whose top left corner is x, y. |
| `canvas.link(name, x1, y1, x2, y2, via, color, width, start, end, label, style)` | width 1, plain ends, no label, solid | A line people bend; see below. |
| `canvas.object(record, x, y, width, height)` | | On a canvas in `mode objects`: the box of an object, a record of its list. It draws nothing itself; the shapes drawn after it, until the next `object`, are the object and move with it while it is dragged. |

`canvas.link` draws a line from x1, y1 through its bend points to x2, y2. `via` is a `list<integer>` of the points, x, y, x, y, ... (`[]` for straight); `name` is plain text the code knows it by, as `"a{association.id}"`. An end that lies in a box (as `on connect` finds boxes) is drawn at the edge of that box, so a line from the middle of one box to the middle of another runs between their edges; while an object is dragged (`mode objects`), an end that lies in its box moves with it, and its bend points stay. `start` and `end` say what its ends show: `"arrow"`, `"triangle"` (hollow, a base type), `"diamond"` and `"filledDiamond"` (a whole), or a data model's `"one"`, `"many"` (a crow's foot), `"zeroOrOne"`, `"zeroOrMany"` and `"oneOrMany"`. `label` is a text halfway along it, in quiet italics, which moves with it. `style` is `"dashed"` or `"dotted"`.

- A color is a color of the theme by name (`"accent"`, `"text"`, a color the theme declares) or written out (`"#101810"`); a name the theme does not have is its text color.
- The draw handler runs when the window opens and after every other handler of the window, each time on an empty drawing, so the canvas always shows the data as it is. Only a canvas's own draw handler draws on it, and the handlers it calls that nothing else runs: `drawMark(5, 100, 100)` can draw one square of a board. A handler that an event or another handler also runs cannot draw, since its shapes would land outside a drawing.
- `sin(degrees)` and `cos(degrees)` give a decimal, rounded to 10 decimals. `snapped(value, step)` gives the multiple of step nearest to value (a half rounds up; a step of 0 gives the value).

The checker refuses a shape a canvas does not draw ("a canvas draws clear, line, ..."), too few or too many values, a value of the wrong type, a word for `align`, `start`, `end` or `style` it does not know, `object` on a canvas without objects, a drawing on a canvas without a draw handler ("wire 'on draw: handler'"), and a drawing from a handler that is not the canvas's draw handler or one only it runs ("is drawn only by its draw handler, ..., and the handlers only it runs").

## The pointer on a canvas

What the person does on a canvas runs its handlers. Each reads where, in the canvas's units, whatever is in view.

### Click

`on click: handler` runs the handler when the canvas is clicked, with `canvas.x` and `canvas.y` saying where, rounded down. They are read, not set, and 0 before the first click. A click beside the drawing, where the part shows none of it, runs nothing. The window draws the canvas anew after the handler, as after any other.

```xml
<!-- noughts/design/GameView.cxaml -->
<View class="noughts.GameView">
  <Data name="status" type="text" />
  <Canvas name="board" at="1,1" span="12" rows="8" units="{&quot;height&quot;:300,&quot;width&quot;:300}" none="surface" ondraw="drawBoard" onclick="boardClicked" />
</View>
```

```clarity
code GameView
    handler drawBoard
        board.clear("panel")

    handler boardClicked
        let square = 1
        if board.x >= 200
            square = 3
        else if board.x >= 100
            square = 2
        if board.y >= 200
            square = square + 6
        else if board.y >= 100
            square = square + 3
        status = "Square {square}"
```

### Zoom

`zoom` lets the person zoom and pan the drawing, as Clarity's own diagram designer does, for a drawing larger than the part shows well. It starts whole, fitted in proportion. The wheel pans (with Shift sideways), Ctrl (Cmd on macOS) with the wheel or a touchpad's pinch zooms around the pointer, and three buttons in its top right corner zoom out, show the zoom it is at (a click shows it whole again) and zoom in, named in Clarity's words `zoomOut`, `zoomWhole` and `zoomIn`. It zooms from a quarter of its size up to three times (further when the whole drawing is smaller or larger than that), and pans past its edges. The view stays while the canvas is drawn anew.

### Drag and drop

`on drag: handler` runs while the pointer moves on the canvas with its button down, and `on drop: handler` once when it is let go. Both read where the pointer is (`canvas.x`, `canvas.y`, at the canvas's nearest edge when it is beside it) and where it was pressed (`canvas.fromX`, `canvas.fromY`). A press that does not move is a click. The handler runs with the latest place: moves that come while it runs are not queued one by one. The canvas is drawn anew after each, so code that moves what it draws by `canvas.x - canvas.fromX` shows it moving; the drop is the place to save.

On a canvas with `zoom`, a drag pans instead when there is no drag handler, or when it starts on empty paper; a press on something drawn (a box, a circle, an ellipse, a ring, a picture or an object; not lines or text) is the handler's drag. A panning drag has no drop.

```xml
<!-- sheets/design/SheetView.cxaml -->
<View class="sheets.SheetView">
  <Data name="boxX" type="integer" default="20" />
  <Data name="boxY" type="integer" default="20" />
  <Data name="startX" type="integer" />
  <Data name="startY" type="integer" />
  <Canvas name="sheet" at="1,1" span="12" rows="10" units="{&quot;height&quot;:2000,&quot;width&quot;:3000}" zoom="true" none="surface" ondraw="drawSheet" ondrag="dragged" ondrop="dropped" />
</View>
```

```clarity
code SheetView
    handler drawSheet
        sheet.clear("panel")
        sheet.rect(boxX, boxY, 200, 100, "raised")

    handler dragged
        if startX == 0
            startX = boxX
            startY = boxY
        boxX = startX + sheet.x - sheet.fromX
        boxY = startY + sheet.y - sheet.fromY

    handler dropped
        startX = 0
        startY = 0
```

`on drop` without `on drag`: the canvas carries a box itself. A press on a box (as `on connect` finds boxes, below) and a drag move the box and everything drawn inside it with the pointer, by whole grid steps when the canvas has `snap`, and the end of a link that lies in the box comes along; no handler runs while it moves, so a large drawing moves as smoothly as a small one. When the person lets go, `on drop` runs once with where it was pressed (`canvas.fromX`, `canvas.fromY`) and where that point was carried to (`canvas.x`, `canvas.y`), so the code moves the box it was pressed on by `canvas.x - canvas.fromX` and saves it. The canvas shows the box where it was put until the drawing comes back. Let go where it was, nothing runs. A drag on empty paper pans a canvas with `zoom`.

### Grid, background and dots

- `grid 20` shows a dot every 20 of the canvas's units over the whole view, behind the shapes: after the drawing's `clear`, first when it has none. Zoomed out, dots closer than 8 pixels are thinned to every second, fourth and so on. `snap` (with a grid) says that what is moved lands on the grid; the pointer itself stays free, and code puts the moved thing on the grid with `snapped`: `boxX = snapped(sheet.x - grabX, sheet.grid)`. Code reads and sets both while the app runs: `sheet.grid` (0 for none) and `sheet.snap`, as the line says until a handler sets them, so an app can offer Show grid and Snap to grid.
- `background panel` (a theme color by name, or written out: `background "#f5f0e1"`) is what `clear()` without a color fills the canvas with; without it, the theme's `panel`. Code reads and sets it as `canvas.background`; an empty one is the theme's panel again. A person's chosen color can so reach a canvas through a setting: `sheet.background = settings.diagramColor`.
- `dots 3 muted` says how many pixels across a grid's dots are (1 to 20; 2 without it) and their color (a theme color by name, or written out; the theme's `border` without it, and the color can be left out: `dots 3`). They are the same size whatever the zoom. Code reads and sets them as `canvas.dotSize` and `canvas.dotColor`; an empty color is the theme's border again.

### Bending lines

`on bend: handler` lets people bend the canvas's links: a press on a link adds a point there (or takes the point under it), dragging moves that point, and a double click on a point takes it away; the points of the link in hand show while it is. When the person lets go, the handler runs and reads which link (`canvas.bent`, its name) and its points now (`canvas.via`, a `list<integer>`), to keep them: `association.via = sheet.via` and `save association`. The canvas draws the link with those points until the drawing comes back with them. A press on a shape drawn over a link (a box above its line) is not a bend, and a press on no link is a click or a drag as before.

### Drawing a new line

`on connect: handler` lets people draw a new line from one box to another. A box is a shape the canvas can hit (a rect, circle, ellipse, ring, picture or object), together with the shapes under it that hold it whole, so a box drawn as an outline, a body and a header is one box; a shape over the whole paper is the paper. While the pointer is over a box, a small dot in the theme's accent shows in the middle of each side, with a crosshair over a dot. A press on a dot starts a new line (before the box's drag and before a bend): a dashed line follows the pointer and the box under it is outlined. No handler runs while it is drawn.

When the person lets go, the handler runs once and reads where the line started (`canvas.fromX`, `canvas.fromY`: the middle of the box it started from) and where it was let go (`canvas.x`, `canvas.y`), so the code finds the two boxes the way its drag finds the one it moves. Let go on the same box or on empty paper, it still runs and the code decides; let go where it was pressed, nothing runs.

`canvas.connecting` (a bool, false until code sets it) makes any press on a box start a new line instead of a drag, from where it was pressed, so an app can offer a line tool. The right button draws a new line too, from anywhere on a box: a right press and drag from one box to another is the same new line, and a right click that does not move opens the canvas's `menu` instead. The dots and the line are drawn over the drawing, never in it, and look the same on every surface. A new line is one edit for undo, like a drop. Without `on connect` a canvas shows no dots.

```xml
<!-- modelling/design/DiagramView.cxaml -->
<View class="modelling.DiagramView">
  <Data name="types" type="list&lt;ClassType&gt;" />
  <Canvas name="sheet" at="1,1" span="12" rows="10" units="{&quot;height&quot;:3000,&quot;width&quot;:4000}" zoom="true" none="surface" ondraw="drawTypes" onconnect="typesConnected" />
  <Button name="lineTool" at="1,11" span="2" caption="@lineToolCaption" onclick="lineToolClicked" />
</View>
```

```clarity
code DiagramView
    handler typesConnected
        let sourceId = 0
        let targetId = 0
        for t in types
            if sheet.fromX >= t.x and sheet.fromX <= t.x + 200 and sheet.fromY >= t.y and sheet.fromY <= t.y + 100
                sourceId = t.id
            if sheet.x >= t.x and sheet.x <= t.x + 200 and sheet.y >= t.y and sheet.y <= t.y + 100
                targetId = t.id
        if sourceId != 0 and targetId != 0 and sourceId != targetId
            message "From {sourceId} to {targetId}"

    handler lineToolClicked
        sheet.connecting = not sheet.connecting
```

### Resizing boxes

`on resize: handler` lets people resize a box by dragging its corner. While the pointer is over a box, a small square grip in the theme's accent shows at its bottom-right corner, with the resize cursor over it. A press on the grip (it comes before the box's dot, carry, bend and pan) starts a resize: a dashed outline of the box at its new size follows the pointer. The box's top-left corner stays; its right and bottom edges follow the pointer in whole units, on the grid when the canvas has `snap`, and it is never smaller than 40 by 30 units (on the grid, the next grid step past that). No handler runs while it is dragged.

When the person lets go, the handler runs once and reads where the box's bottom-right corner was (`canvas.fromX`, `canvas.fromY`) and where it is now (`canvas.x`, `canvas.y`). The code finds the box whose corner was at `fromX`, `fromY` (a point just inside it, `fromX - 1`, `fromY - 1`, is in the box) and makes it `canvas.x - box.x` wide and `canvas.y - box.y` high. Let go at the size it had, nothing runs; a pointer taken away mid-drag drops it. A resize is one edit for undo, like a drop. `canvas.resizable` (a bool, true until code sets it) turns the grip off and on. The grip and the outline are drawn over the drawing, never in it. Without `on resize` a canvas shows no grip.

```clarity
code Diagram
    handler typesResized
        for t in types
            if sheet.fromX - 1 >= t.x and sheet.fromX - 1 <= t.x + t.width and sheet.fromY - 1 >= t.y and sheet.fromY - 1 <= t.y + t.height
                t.width = sheet.x - t.x
                t.height = sheet.y - t.y
                save t
```

## Canvas modes

A canvas the person changes has a `mode`; without one, its draw handler draws it and code handles its pointer. Every mode shares the same parameters, the same event and the same property, and each change the person makes is one edit, which Ctrl+Z takes back. The canvas does what the gesture says: it snaps to the grid when `snap` is on, keeps the change in the data, and chooses what was touched.

| Mode | Binds | What the person does |
|---|---|---|
| `mode objects types at x, y` | nothing: its objects are the records of the list `types`, placed by their integer fields `x` and `y` | drags an object (the box its draw handler marks with `canvas.object`) and lets it go; the record's `x` and `y` move there |
| `mode paint` | an `image` field: `bind note.picture` | paints with the tools of its toolbar; each stroke is the picture after it |
| `mode vector` | a `text` field, which holds the drawing as SVG: `bind note.drawing` | draws lines, rectangles, ellipses, paths and texts with the tools of its toolbar; with `select` drags a shape, and Delete deletes it |

In a `.cxaml` file a canvas of objects is written `mode="objects" objects="{&quot;list&quot;:&quot;types&quot;,&quot;x&quot;:&quot;x&quot;,&quot;y&quot;:&quot;y&quot;}"`.

- `tools pen, eraser` names the tools of the mode the person may use, in the order its toolbar shows them; all of them without it. Paint has `pen` (soft edges), `pencil` (hard pixels), `spray`, `eraser`, `fill` (the area of near colours under the press takes the ink), `picker` (the pixel pressed becomes the ink), `select`, `ellipseSelect`, `lasso`, `line`, `rect` and `ellipse`. Vector has `select`, `line`, `rect`, `ellipse`, `path` and `text`. Objects has `select` (drag an object) and `connect` (draw a new line from one object to another, which runs `on connect`).
- `toolbox` puts the canvas's tools in a toolbox instead of a strip on the canvas: Clarity's own for its mode, or a toolbox page designed for it (`toolbox CoverTools`); see [Toolbox](parts.toolbox.md).
- `brush 4` (1 to 100 pixels, 2 without it) and `ink accent` (a theme color by name, or written out) are how wide and in which color paint and vector draw. A vector drawing keeps a theme color written out, so other programs show it alike.
- `on change: handler` runs when an edit of the canvas ended: an object dropped, a stroke painted, a shape added, changed, moved or deleted.
- `canvas.selected` is what the person touched last: the record of the object (mode objects), or the place of the shape in the drawing, from 1, and 0 for none (mode vector). It is read, not set. Handlers set `canvas.tool` (any mode), `canvas.brush` and `canvas.ink` (paint and vector), so buttons can be a palette.
- The SVG of a vector drawing has one element on a line. A drawing made in another program shows too; a line Clarity does not read as one of its shapes is kept as it is, and shows, but does not move.

```clarity
entity ClassType
    name: text
    x: integer
    y: integer

entity Note
    picture: image
    drawing: text
```

```xml
<!-- notes/design/DiagramView.cxaml -->
<View class="notes.DiagramView">
  <Data name="types" type="list&lt;ClassType&gt;" />
  <Canvas name="diagram" at="1,1" span="12" rows="10" mode="objects" objects="{&quot;list&quot;:&quot;types&quot;,&quot;x&quot;:&quot;x&quot;,&quot;y&quot;:&quot;y&quot;}" units="{&quot;height&quot;:3000,&quot;width&quot;:4000}" zoom="true" grid="20" snap="true" none="surface" ondraw="drawTypes" onchange="typeMoved" />
</View>
```

```xml
<!-- notes/design/SketchView.cxaml -->
<View class="notes.SketchView">
  <Data name="note" type="Note" />
  <Canvas name="sketch" at="1,1" span="6" rows="8" bind="note.picture" mode="paint" brush="4" ink="accent" none="surface" onchange="saveNote" />
  <Canvas name="drawing" at="7,1" span="6" rows="8" bind="note.drawing" mode="vector" grid="10" snap="true" none="surface" onchange="saveNote" />
  <Button name="redButton" at="1,9" span="2" caption="@redCaption" onclick="red" />
</View>
```

```clarity
code DiagramView
    handler drawTypes
        diagram.clear()
        for t in types
            diagram.object(t, t.x, t.y, 260, 120)
            diagram.rect(t.x, t.y, 260, 120, "raised")
            diagram.text(t.x + 130, t.y + 14, t.name)

    handler typeMoved
        save diagram.selected

code SketchView
    handler saveNote
        save note

    handler red
        sketch.ink = "#ff0000"
```

## The parts

### `image`

A picture: a picture of a record it binds, a fixed picture of the project's `resources` folder, or a picture per language through a key.

```xml
<Image name="name" at="col,row" span="n" rows="n" bind="record.field" source="file" caption="@key" noUndo="true" opens="target" onclick="handler" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `bind` | an `image` field | none | The picture of a record; while the pointer is on it, it offers a button to load a picture from a file and one to remove it. |
| `source` | a picture of the resources folder | none | A fixed picture (**Source**). |
| caption `@key` | a text key | none | A picture per language: each language file points the key at its picture. |
| `no undo` | yes or no | no | A picture loaded or removed is an edit Ctrl+Z takes back, unless it has this. |

Event: `click`; a click can also open a window, slide-out or popover (`opens="CustomerBrowse"`), as a button's does. A bound image takes a picture from a file, so a label or a help text can be for it.

```xml
<Image name="collectionLogo" at="12,1" span="1" source="logo.png" />
<Image name="cover" at="1,2" span="4" rows="6" bind="album.cover" />
```

The checker refuses an image with more than one of a key, a `source` and a `bind` ("one of them"), a `source` that is no picture or is not in the resources folder, and a key whose file in some language is no picture. An image with none of the three is a warning ("has no source yet"). See [Pictures and files](data.pictures-files.md).

### `display`

A number in seven-segment digits, as on a calculator, with its dark segments faint.

```xml
<Display name="name" at="col,row" span="n" rows="n" bind="number" digits="n" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `bind` | an integer, decimal or money, a field or a plain value | none | The number it shows. |
| `digits` | 1 to 24 | 12 | How many digits it has (**Digits**). |

It has no events. A value is rounded half away from zero to fit its digits, and shows `E` when its whole part does not fit. Its style gives its colors: `background` behind the digits, `text` for the lit segments.

```xml
<Display name="screen" at="1,1" span="4" rows="2" bind="shown" digits="12" />
```

The checker refuses `digits` outside 1 to 24 ("a display has 1 to 24 digits").

### `canvas`

Free graphics: drawn by its draw handler, by an engine frame by frame, or changed by the person in a mode. See Drawing on a canvas, The pointer on a canvas and Canvas modes above.

```xml
<Canvas name="name" at="col,row" span="n" rows="n" bind="record.field" units="{...}" zoom="true" grid="n" snap="true" background="color" dotSize="n" dotColor="color" surface="svg|bitmap|gpu" runs="Engine" mode="objects|paint|vector" objects="{...}" tools="tool, tool" toolbox="popup|slideout" toolboxPage="Toolbox" brush="n" ink="color" noUndo="true" ondraw="handler" onclick="handler" ondrag="handler" ondrop="handler" onbend="handler" onconnect="handler" onresize="handler" onchange="handler" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `units W x H` | 1 to 10000 each | 100 x 100 | The space it draws in, scaled to fit the part in proportion. |
| `zoom` | yes or no | no | The person zooms and pans. |
| `grid` | 1 to 10000 units | none | A dot every so many units. |
| `snap` | yes or no | no | What is moved lands on the grid. |
| `background` | a color | the theme's `panel` | What `clear()` fills it with. |
| `dots N color` | 1 to 20 pixels, a color | 2, the theme's `border` | The grid's dots. |
| `surface` | `svg`, `bitmap` or `gpu` | `svg` | What it is drawn on. |
| `runs` | an engine declared `beside the screen` | none | An engine draws it frame by frame. |
| `mode` | `objects list at x, y`, `paint` or `vector` | none | What the person does on it. |
| `tools` | tools of its mode | all of them | The tools the person may use. |
| `toolbox` | `popup` or `slideout`, a toolbox page | none | Its tools in a toolbox. |
| `brush` | 1 to 100 pixels | 2 | How wide paint and vector draw. |
| `ink` | a color | | The color paint and vector draw with. |
| `no undo` | yes or no | no | Ctrl+Z takes back nothing on it. |

Events: `draw`, `click`, `drag`, `drop`, `bend`, `connect`, `resize`, `pointer`, `key`, `change`.

An engine draws a canvas with `runs Screen`, an engine declared `beside the screen` (see [Engines](code.engines.md)). Pick it under **Engine** in the properties: the draw event goes (its handler stays in the code), the engine's tells appear under **Tells** to wire like events, and a hint names what code can tell it (`sky.start(level)`). Without a draw handler, an engine or a mode it shows as a placeholder.

The checker refuses:

- `units` outside 1 to 10000, a `grid` outside 1 to 10000, dots outside 1 to 20 pixels, a brush outside 1 to 100 pixels, and `snap` without a grid;
- `on connect` on a canvas in mode paint or vector or one an engine draws, and `on resize` on a canvas with a mode or an engine;
- on a canvas without a mode: `bind`, `tools`, `brush`, `ink` and `on change` ("belongs to a canvas with a mode");
- a tool its mode does not have ("is no tool of mode paint, which has ..."), and a brush and ink in mode objects;
- in mode objects: a `bind`, a list that is no list of records, `x` or `y` fields that are missing or no integer, and a list of records that are never stored (a `record`, built in code) without an `id: integer` of their own, which the canvas follows a dragged object by;
- in mode paint a binding that is no `image` field, in mode vector one that is no `text` field;
- `runs` of something that is no engine, or of an engine that runs on the backend; a draw handler on a canvas an engine draws; tells on a canvas that runs no engine, and a tell its engine does not have;
- a `toolbox` on a canvas without a mode.

A canvas that draws nothing yet is a warning ("wire 'on draw: handler'"), and so is a canvas with a mode and no `on change`, since the person's change is lost.

### `custom part`

A part an outside library draws, such as a game scene, a map or a code editor, written in the project's own language. It is declared in a module and placed on a window by its declared name (`<Arena name="arena" ... />`), so it is never typed as `custom part`; the Parts palette lists the project's custom parts in its place. It binds what its declaration says, has `taborder` and `no undo` (what it binds the person changes through it, an edit Ctrl+Z takes back), the properties its declaration gives, and the event `change`. Declaring, placing and building one is in [Custom parts](design.custom-parts.md).

```xml
<Arena name="arena" at="1,1" span="8" rows="6" />
```

## See also

- [Parts](design.parts.md): the Parts palette and the properties panel.
- [Toolbox](parts.toolbox.md): the tools of a canvas in a toolbox page.
- [Handlers](code.handlers.md): the code behind, where the draw handler and the pointer handlers live.
- [Engines](code.engines.md): an engine that draws a canvas frame by frame.
- [Pictures and files](data.pictures-files.md): pictures in the resources folder and in records.
- [Custom parts](design.custom-parts.md): parts an outside library draws.

## The parts at a glance

<!-- catalog: written from Clarity's part catalog (crates/clarity-spec/src/widgets.rs) by CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test manual_catalog; not edited by hand -->

| Part | Line | Binds | Size | Grows | Properties | Events |
|---|---|---|---|---|---|---|
| Display | `display` | field (integer, decimal, money) | 4 x 2 | no | `digits` | none |
| Image | `image` | field (image) | 3 x 3 | yes | `source`, `noUndo` | `click` |
| Canvas | `canvas` | field (image, text) | 6 x 4 | yes | `units`, `noUndo`, `zoom`, `grid`, `snap`, `background`, `dots`, `runs`, `surface`, `mode`, `tools`, `toolbox`, `brush`, `ink` | `draw`, `click`, `drag`, `drop`, `bend`, `connect`, `resize`, `pointer`, `key`, `change` |
| Custom part | `custom part` | field | 4 x 3 | no | `tabOrder`, `noUndo` | `change` |

<!-- /catalog -->
