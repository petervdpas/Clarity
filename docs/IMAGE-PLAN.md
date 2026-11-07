# Image tools plan

Step 4 of [VIDEO-PLAN.md](VIDEO-PLAN.md): picture functions in code on every stack, and a GIMP-like toolbox on the painting canvas. Decided by the user on 2026-11-07.

## Decisions

- **Clarity.Image** is a library of its own, like Clarity.Audio and Clarity.Video: a Rust crate (`runtimes/rust-image`), a C table (`runtimes/rust-image-ffi`, `clarity_image_api()`), the NuGet `Clarity.Image`. An app that uses picture functions carries it; one that does not, does not.
- **The `image` crate** (pure Rust) decodes and encodes PNG, JPEG and WebP, resizes, rotates and adjusts colour. No system libraries, so it works offline and alike on Linux, Windows and macOS.
- **One picture type.** In code a picture is what an `image` field holds (`Ty::Image`): its encoded bytes. Every function takes pictures and gives a picture as PNG, so a result can go straight into an `image` field, to a file, or into the next function. The same pictures are what video playing gives and export takes ("Pictures" in VIDEO-PLAN.md).
- **The toolbox** belongs to a canvas in `mode paint`: a grid of tool icons with the chosen tool's options under it and the foreground and background colours, as GIMP's. How it shows is an option of the canvas (user's decision): `toolbox popup`, the default, floats over the app, dragged anywhere and folded, its place remembered per window; `toolbox slideout` is a pinnable slide-out against the canvas's edge (`beside` the canvas), with its handle, resize and pin.

## The language

Built-in functions, like the text functions; each gives a new picture and leaves its arguments as they were:

| Function | Gives |
|---|---|
| `resize(p, width, height)` | the picture at that size, smoothly scaled |
| `crop(p, x, y, width, height)` | the part of the picture at that place, clamped to the picture |
| `rotate(p, degrees)` | turned by 90, 180 or 270 degrees; other angles turn on a transparent ground the size of the turned picture |
| `flip(p, horizontal)` | mirrored left to right (`true`) or top to bottom (`false`) |
| `blend(base, over, x, y, opacity)` | `over` laid onto `base` at that place, with its own alpha times `opacity` (0 to 100) |
| `adjust(p, hue, saturation, brightness, contrast)` | hue turned by degrees, the others from -100 to 100, 0 leaving it as it is |
| `pictureWidth(p)`, `pictureHeight(p)` | its size in pixels |

Loading and saving go through what is there: `read bytes of f` reads a file as a picture (`photo.picture = f` already does), and `write p to exports "out.png"` writes one. A picture that cannot be read (no PNG, JPEG or WebP) is refused with a message, as a wrong picture file is now.

All stacks give the same pixels for the same call: `conformance/picture-functions.json` holds small pictures and each result's pixels.

## Clarity.Image

- `runtimes/rust-image`: the functions on decoded RGBA pictures, and PNG out. Clarity.UI's painting canvas uses the crate directly.
- `runtimes/rust-image-ffi`: the C table, bytes in and bytes out, an allocation freed by the table's `free`; no panic crosses it.
- The Rust runtime calls the crate; Go, C and C++ call the C table, linked as Clarity.UI's library is; C# through the NuGet. The IDE's build carries the library, as it carries Clarity.Audio and Clarity.Video.

## The toolbox

On a canvas in `mode paint`, the floating popup has, in this first round:

1. **Paint tools:** brush (size, opacity, soft edge), pencil (hard pixels), eraser, fill bucket (with a tolerance), colour picker; the foreground and background colours, swapped with a click.
2. **Selection and move:** rectangle and ellipse select; move the selection, or a picture placed on the canvas.
3. **Transform:** crop to the selection, rotate, flip, scale, on the whole picture or the selection.
4. **Colour and layers:** hue and saturation, brightness and contrast, in small dialogs with a live preview; a layers list (add, remove, hide, opacity, order), composed into the bound picture.

Each change is one edit Ctrl+Z takes back. `tools` on the canvas line names the tools the toolbox offers, as it does for pen and eraser now. Code reads the canvas back as a picture (`board.picture()`), the composed layers.

## Tool groups

As GIMP's, each button of the toolbox is a group showing the tool last used in it; holding it (or a right click) opens the group's other tools beside it: select (rectangle, ellipse, lasso), drawing (brush, pencil, spray can), shapes (line, rectangle, ellipse), and eraser, fill and picker on their own. A selection is a mask of the picture's pixels: dragged from inside, its pixels move; the transforms (turn a quarter, mirror across or down, crop) work on it, or on the whole picture fitted to the paper. Import lays a picture file on the paper, scaled to fit; export saves the picture as PNG.

## Designable toolboxes (user's decision, 2026-11-07: build now)

A toolbox is a thing of its own, designed like a view and used by the parts that need it ("independent utilization"): one project can have a painting toolbox, another for a vector drawing, later one for a diagram's association lines or a markdown editor's formatting. In a module:

```clarity
toolbox Painting
    title @paintingTools
    group select: select, ellipseSelect, lasso
    group draw: pen, pencil, spray
    tool eraser
    tool fill
    group shapes: line, rect, ellipse
    transforms
    size
    opacity
    colors
    files
    button @starCaption name starButton icon "star" on click: stampStar
```

- `group name: tools` is one button showing the tool last used, holding it shows the others; `tool name` is a button of its own. The tools are those of the mode of the part that uses the toolbox; one it does not have is an error there.
- `transforms`, `size`, `opacity`, `colors` and `files` turn on its rows: turn and mirror and crop, brush size, opacity, foreground and background colours, import and export. What is not written is not there.
- `button` is the developer's own: a caption, a name, an icon and the handler it runs in the code of the window the part is in, as a toolbar's button does. So a toolbox serves any use without new built-in tools.
- A part uses one by name: `canvas ... mode paint toolbox Painting` (`toolbox Painting slideout` against its edge). Plain `toolbox` is the built-in one of the part's mode, with everything.
- The IDE designs a toolbox as it designs a view: its tools and rows turned on and off, their order, its own buttons, with the toolbox shown as it will look.

## Later steps (user's wishes, 2026-11-07)

- **Stamps:** ready-made shapes (stars, arrows, speech balloons, a project's own pictures) laid on the picture at a place, size and angle.
- **Scripts on the toolbox's subject (generic, user's direction):** no image-only hook. A toolbox is connected to a subject (the canvas's picture now; later a diagram, a markdown text, a record), and a tool can open a script editor whose script works on that subject: it gets the subject, changes it or hands it to a program (ImageMagick, a command the project declares), and the result goes back as one edit. Coded tools (`on click: handler`) are the plain form of it; running anything outside asks the person first, as everything that executes does.

## Order

1. Clarity.Image: the crate, the conformance file, the C table.
2. The language: the functions in the checker, each stack's runtime and generator, an example.
3. The toolbox popup with the paint tools.
4. Selection, move and transform.
5. Colour dialogs and layers; `board.picture()`.
6. The IDE carries Clarity.Image; release notes.
