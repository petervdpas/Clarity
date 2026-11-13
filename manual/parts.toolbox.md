# Toolbox parts

The parts of the Toolbox group: tools, tool groups, brush size, opacity, colours and layers, and how a toolbox page made of them serves a canvas or a memo.

## A canvas's toolbox

A canvas with a mode (objects, paint or vector) shows its tools as a strip on the canvas. With **Toolbox** in its properties it shows them in a toolbox instead, as a paint program does: a panel of the tools with the brush's size and opacity and the foreground and background colours (a click on a colour opens the palette; the arrow swaps them).

- **Clarity's own** (`toolbox`) is the built-in toolbox of the canvas's mode: a diagram's has select, connect, zoom in and zoom out.
- A toolbox page (`toolbox CoverTools`, in a `.cxaml` file `toolboxPage="CoverTools"`) is one you design, below.
- **Shows as**: **Floating** (`toolbox popup`, the default) floats over the window, dragged by its title and folded with its arrow; **Slide-out** (`toolbox slideout`) lies against the canvas's right edge, as high as the canvas.

The toolbox's tool, size and foreground colour are the canvas's `tool`, `brush` and `ink`, which code reads and sets too; opacity and the background colour are the toolbox's own.

```xml
<Canvas name="coverCanvas" at="1,2" span="12" rows="15" bind="book.cover" mode="paint" toolbox="popup" toolboxPage="CoverTools" brush="8" ink="#8a5a2b" units="{&quot;height&quot;:900,&quot;width&quot;:600}" zoom="true" none="surface" onchange="coverPainted" />
```

The checker refuses a toolbox on a canvas without a mode ("a toolbox holds the tools of a canvas in mode objects, paint or vector").

## A memo's toolbox

A memo takes a toolbox page too, of coded tools that work on its text where the person is. In the memo's properties, **Toolbox** names the page and **Shows as** offers **At the caret** (Ctrl+Space, or a right-click in the text; its tool is done where the person is, then it goes), **Floating** and **Slide-out**. In Clarity's words a memo's toolbox is `toolbox ManuscriptTools`, at the caret unless `popup` or `slideout` follows.

```xml
<Memo name="manuscriptBox" at="1,2" span="9" rows="13" bind="manuscript.text" markdown="true" toolbox="caret" toolboxPage="ManuscriptTools" />
```

The checker refuses a memo's toolbox that is not a page ("a memo's toolbox is a page of its own"), and a page for a memo that holds tools with a built-in action, size, opacity, colours or layers ("a memo's toolbox holds tools with 'on click', buttons and labels").

## Designing a toolbox page

A toolbox is designed as a view is, in its own file in `design/`, with a `<Toolbox>` root (in a module, `page CoverTools: toolbox`), its parts on its own grid (`columns="4"`). **New window** offers **Toolbox** to start from: a painting canvas's tools on a grid of 4, with a select group, a draw group, the eraser, fill, size, opacity and colours. Placed in the canvas or memo that names it, its parts work on that part, and its code joins the code of the window it shows in.

```xml
<!-- novel/design/CoverTools.cxaml -->
<Toolbox class="writing.CoverTools" columns="4">
  <ToolGroup name="drawGroup" at="1,1" span="1">
    <Tool name="penTool" at="1,1" span="1" does="pen" />
    <Tool name="pencilTool" at="1,1" span="1" does="pencil" />
    <Tool name="sprayTool" at="1,1" span="1" does="spray" />
  </ToolGroup>
  <Tool name="eraserTool" at="2,1" span="1" does="eraser" />
  <Tool name="fillTool" at="3,1" span="1" does="fill" />
  <Tool name="pickerTool" at="4,1" span="1" does="picker" />
  <ToolSize name="sizeOption" at="1,2" span="4" />
  <ToolOpacity name="opacityOption" at="1,3" span="4" />
  <ToolColors name="colorOption" at="1,4" span="2" />
  <Tool name="exportTool" at="4,4" span="1" does="export" />
</Toolbox>
```

A toolbox holds tools, tool groups, size, opacity, colours and layers, and buttons, labels, headings and lines. A tool is built in or coded. A built-in tool does one of these:

| Action | What it does |
|---|---|
| `pen`, `pencil`, `spray`, `eraser`, `fill`, `picker`, `select`, `ellipseSelect`, `lasso`, `line`, `rect`, `ellipse` | Takes that tool of a painting canvas in hand. |
| `select`, `line`, `rect`, `ellipse`, `path`, `text` | Takes that tool of a vector canvas in hand. |
| `select`, `connect` | Takes that tool of a diagram (mode objects) in hand. |
| `rotate`, `flipAcross`, `flipDown`, `crop`, `import`, `export` | Done at once on a painting canvas's picture. |
| `adjust` | Opens the colour dialog of a painting canvas (hue, saturation, brightness, contrast), shown on the picture as they change and kept with OK as one edit, on the selection or the whole picture. |
| `zoomIn`, `zoomOut` | Zooms the canvas. |

A coded tool runs its `on click` handler, a handler of the toolbox's code or of the window it shows in. A tool the canvas's mode does not have is an error at the canvas, so one toolbox serves a painting canvas, another a vector drawing, and coded tools any other use.

The checker refuses a toolbox part outside a toolbox page ("belongs in a toolbox, a page written 'page Tools: toolbox'"), another part in a toolbox, a part in a tool group that is no tool, and at the canvas: a toolbox page that does not exist or is no toolbox, a tool of another mode ("CoverTools's tool 'path' is no tool of a canvas in mode paint"), an action done on a picture on a canvas that is no painting, and layers on a canvas that is no painting.

## The parts

### `tool`

A toolbox's button with an icon, in the accent while it is in hand. It does a built-in action on its canvas or runs a handler.

```xml
<Tool name="name" at="col,row" span="1" does="action" icon="name" tooltip="@key" onclick="handler" />
```

| Property | Type | Default | What it does |
|---|---|---|---|
| `does` | one of the actions above | none | The built-in action (**Action**). |
| `icon` | an icon's name | the action's own | The icon it shows; a coded tool without one shows the first two letters of its tooltip or name. |

Event: `click`, for a coded tool.

```xml
<Tool name="newObjectTool" at="1,3" span="1" tooltip="@newObjectToolTip" icon="plus" onclick="newObjectToolClicked" />
```

The checker refuses a tool that does neither ("a tool does a built-in action (does pencil) or runs a handler (on click: handler)"), both ("not both"), and an action that is not one of the list.

### `toolgroup`

Tools inside one button, which shows the tool last used; the others show beside it while it is held (a little over a third of a second) or right-clicked, and a click takes one.

```xml
<ToolGroup name="name" at="col,row" span="1">
  <Tool name="name" at="1,1" span="1" does="action" />
</ToolGroup>
```

It has no properties of its own and no events. The checker refuses a part in it that is no tool ("a tool group holds tools").

### `toolsize`

The brush's size, made smaller and larger with a button at each side of the value: a step of 1 up to 10 pixels, of 5 above. It is the canvas's `brush`.

```xml
<ToolSize name="sizeOption" at="1,4" span="4" />
```

It has no properties of its own and no events.

### `toolopacity`

The paint's opacity, less and more with a button at each side of the value, in steps of 10%.

```xml
<ToolOpacity name="opacityOption" at="1,5" span="4" />
```

It has no properties of its own and no events.

### `toolcolors`

The foreground colour in front and the background colour behind it: a click on one opens the palette, the arrow swaps them. The foreground colour is the canvas's `ink`.

```xml
<ToolColors name="colorOption" at="1,6" span="2" />
```

It has no properties of its own and no events.

### `toollayers`

A painting canvas's layers while the canvas is open, top first: each shown or hidden with its eye and taken in hand with a click to paint on, with a row of buttons to add a layer, remove the one in hand, and move it up or down. The picture kept is the shown layers laid over each other; a change of the picture otherwise (Ctrl+Z, or another record) folds them back into one.

```xml
<ToolLayers name="layers" at="1,7" span="4" rows="4" />
```

It has no properties of its own and no events. The checker refuses a toolbox with layers on a canvas that is not in mode paint ("layers are a picture's").

## See also

- [Graphics parts](parts.graphics.md): the canvas and its modes.
- [Parts](design.parts.md): the Parts palette and the properties panel.
- [Windows](design.windows.md): views and the pages a window shows.
- [Handlers](code.handlers.md): the handlers coded tools run.

## The parts at a glance

<!-- catalog: written from Clarity's part catalog (crates/clarity-spec/src/widgets.rs) by CLARITY_WRITE_MANUAL=1 cargo test -p clarity-spec --test manual_catalog; not edited by hand -->

| Part | Line | Binds | Size | Grows | Properties | Events |
|---|---|---|---|---|---|---|
| Tool | `tool` | nothing | 1 x 1 | no | `icon`, `does` | `click` |
| Tool group | `toolgroup` | nothing | 1 x 1 | no | none | none |
| Brush size | `toolsize` | nothing | 4 x 1 | no | none | none |
| Opacity | `toolopacity` | nothing | 4 x 1 | no | none | none |
| Colours | `toolcolors` | nothing | 2 x 1 | no | none | none |
| Layers | `toollayers` | nothing | 4 x 4 | no | none | none |

<!-- /catalog -->
