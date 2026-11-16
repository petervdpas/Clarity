# Canvas and toolbox

Status: partial. The `canvas` part (draw handler, modes objects, paint and vector) and toolboxes: Clarity's own per mode, or a designed `<Toolbox>` page.

Progress: 65%

## Code
- Spec: `crates/clarity-spec/src/widgets.rs` (canvas props; toolbox kinds tool, toolGroup, toolSize, toolOpacity, toolColors, toolLayers at :23-34, :155, :463-467, :932); `crates/clarity-spec/src/views.rs:129, 240, 290` (a toolbox page placed in the canvas that names it); `crates/clarity-spec/src/code/check.rs:2239` (`canvas.shape(...)`: clear, line, rect, circle, ring, text, link, object, ellipse, path, image); page kind toolbox `model.rs:519`, root `<Toolbox>` in `cxaml.rs`
- Generators: `crates/clarity-gen/src/lib.rs:526` (`canvas_draws`), :565 (`canvas_tool`); each stack's logic.rs for draw handlers and `on change`
- Runtimes: R `runtimes/rust/src/canvas.rs`; Go `runtimes/go/clarity/canvas.go`; C `runtimes/c/clarity/canvas.c`; C++ through Clarity.UI and `runtimes/cpp/tests/canvas_test.cpp`; C# `runtimes/csharp/Clarity.Runtime/CanvasModes.cs`, `Shape.cs`
- Clarity.UI: `runtimes/rust-ui/src/parts/data/canvas.rs`; `parts/data/canvas/` (drawing.rs, modes.rs, raster.rs, layers.rs, adjust.rs, toolbox.rs, engine.rs); toolbox parts `parts/tools.rs`; memo's context toolbox `parts/text_editor.rs:582`
- IDE: palette and inspector (see [ide-designer](ide-designer.md)); New window offers a toolbox template (`app/src/lib/NewWindowDialog.svelte:92`)
- Docs: manual `parts.graphics.md` (Drawing on a canvas, The pointer on a canvas, Canvas modes, `canvas`), `parts.toolbox.md`, `code.handlers.md` (Drawing on a canvas)

## Tests
- `conformance/drawing.json`: run by R (`runtimes/rust/tests/values.rs`), Go (`drawing_test.go`), C (`values_test.c`), C++ (`values_test.cpp`, `ui_test.cpp`), C# (`FrameLoopTests.cs`, `RoundingTests.cs`), Clarity.UI (`tests/data/data_canvas.rs`)
- `conformance/canvas-modes.json`: run by R (`runtimes/rust/tests/canvas.rs`), Go (`canvas_test.go`), C (`canvas_test.c`), C++ (`canvas_test.cpp`), C# (`CanvasModeTests.cs`), Clarity.UI (`data_canvas.rs`)
- Spec: `crates/clarity-spec/tests/drawing.rs`, `canvas_modes.rs`, `toolbox.rs`; gen: `crates/clarity-gen/tests/canvas_drag.rs`, `csharp_canvas_modes.rs`
- Engine canvases: see [engines](engines.md)

## Open
- `surface svg|bitmap|gpu`: PARTIAL. Parsed and stored (`crates/clarity-spec/src/parser.rs:2538`), read by nothing; only `widgets.rs:2083` writes it back.
- `bitmap` and `vector` field types: NOT BUILT. Painting stores into an `image` field, vector into a `text` field (SVG).
- Layer opacity: NOT BUILT; see Design "Layer opacity".
- `board.picture()`: NOT BUILT; see Design "Reading the canvas back".
- A scale tool: NOT BUILT; see Design "Scale".
- Stamps: NOT BUILT; see Design "Stamps".
- Scripts on the toolbox's subject: NOT BUILT; see Design "Scripts on the subject".
- A toolbox page on parts other than canvas and memo: NOT BUILT (memory: designable-toolboxes.md).
- No example uses the paint toolbox with picture functions (see [picture-functions](picture-functions.md)).

## Design

### Layer opacity
- Each layer of a painting canvas gets an opacity, 0 to 100, set in the layers list (`toollayers`) beside hide and order.
- The layers are composed with it into the bound picture. Today `Layer` holds name, pixels and shown only (`runtimes/rust-ui/src/parts/data/canvas/layers.rs:14`).

### Reading the canvas back
- `board.picture()` in code gives the painting canvas's picture, the layers composed, as PNG: the same picture an `image` field holds.
- So it goes into a picture field, into a picture function, to a file, or into a video export or the frame buffer (see [video-camera](video-camera.md) Design).

### Scale
- A built-in tool that scales the whole picture, or the selection, as rotate, flipAcross, flipDown and crop work now. Its tool name is an open question.

### Stamps
- Ready-made shapes (stars, arrows, speech balloons, a project's own pictures) laid on the picture at a place, size and angle, as one edit.
- Open question: whether a stamp is a built-in tool, a coded tool on a toolbox page, or both.

### Scripts on the subject
- No image-only hook. A toolbox serves a subject: the canvas's picture now; later a diagram, a markdown text, a record.
- A tool can open a script editor; the script gets the subject, changes it or hands it to a program, and the result comes back as one edit Ctrl+Z takes back.
- Coded tools (`on click: handler`) are the plain form of it. Running anything outside asks the person first.
- Step 1, commands: a command line run on the subject, such as `magick {in} -blur 0x4 {out}`. Clarity.UI writes the subject to `{in}`, runs the program after asking, and takes `{out}` back. The same works for a video's frame or a text later.
- Step 2, Clarity code: a script in Clarity, `subject = resize(subject, 400, 300)`, run inside Clarity.UI by Clarity's interpreter with the picture functions, alike on every stack. It waits until the parser and interpreter can ship inside Clarity.UI.
- Open question: how a toolbox page declares a script tool, and where the command is kept.

## Conflicts
- none

## Decisions
- 2026-10-19: the backend renderer is a draw handler written in Clarity, shapes in the canvas's units, on every stack; the frontend renderer is an engine beside the screen.
- 2026-10-31: one option `mode` (draw, objects, paint, vector) sharing parameters, `on change`, `canvas.selected` and one `canvas_event` message. User: "you're not approaching things generic enough!!!!"
- 2026-10-30: a canvas "can be for painting a picture or draw objects you can drag like the diagrams"; "there is one more... could be a vector editor!!!" (user).
- 2026-11-07: a paint toolbox shows as `toolbox popup` (default, floating) or `toolbox slideout` (user's decision).
- 2026-11-07: toolboxes are designed like views and used by the parts that need them. User: "independent utilization"
- 2026-11-08: scripts on the toolbox's subject: commands first, Clarity code later; running anything asks first.
- 2026-11-08: on a memo the toolbox is called up at the caret (Ctrl+Space or right-click), runs its action on that context, then closes (memory: designable-toolboxes.md).

## Was
- DESIGN 5.7, DESIGN 5.31, roadmap S4
- IMAGE-PLAN.md "The toolbox" (scale, layer opacity, `board.picture()` now in Design), "Tool groups", "Designable toolboxes" (outdated module syntax, dropped), "Later steps" (now Design "Stamps", "Scripts on the subject"), Order steps 3-5
- VIDEO-PLAN.md step 4 (painting canvas half), "Pictures" (painting canvas: layer opacity and `board.picture()`, now Design)
- memory: designable-toolboxes.md
