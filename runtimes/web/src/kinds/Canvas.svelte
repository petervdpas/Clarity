<script lang="ts">
  import CanvasView from "../parts/CanvasView.svelte";
  import { DEFAULT_UNITS } from "../screen/drawing.ts";
  import { ownWord, shownText } from "../screen/model.ts";
  import { wiredCall } from "../screen/calls.ts";
  import { isStored, linkedFile, pictureUrl as recordPictureUrl } from "../screen/recordPictures.ts";
  import type { WidgetProps } from "../generated/WidgetProps.ts";
  import type { PartContext } from "./context.ts";
  import { boundEntity as boundEntityOf } from "./shared.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const boundEntity = $derived(boundEntityOf(ctx));

  /** The tools of a canvas's mode the person may use (DESIGN 5.31), as crates/clarity-spec's CanvasMode::tools names them. */
  const MODE_TOOLS: Record<string, string[]> = { objects: [], paint: ["pen", "eraser"], vector: ["select", "line", "rect", "ellipse", "path", "text"] };
  function canvasTools(props: WidgetProps): string[] {
    return props.tools.length ? props.tools : (props.mode ? MODE_TOOLS[props.mode] : []);
  }

  /** What the person did on a canvas with a mode, for the runtime to apply and keep as one edit (DESIGN 5.31). */
  function canvasActed(action: Record<string, unknown>): Promise<boolean> {
    const live = ctx.live;
    const partState = ctx.partState;
    if (!live?.canvas || !p.name || !p.mode) return Promise.resolve(false);
    const objects = p.objects;
    const units: [number, number] | null = p.units ? [p.units.width, p.units.height] : null;
    const canvas =
      p.mode === "objects" && objects
        ? { mode: p.mode, list: objects.list, at: [objects.x, objects.y] as [string, string], grid: partState?.grid ?? p.grid ?? 0, snap: partState?.snap ?? p.snap }
        : { mode: p.mode, path: p.bind ?? "", units, grid: partState?.grid ?? p.grid ?? 0, snap: partState?.snap ?? p.snap };
    return live.canvas(p.name, canvas, action, wiredCall(p.handlers, "change"));
  }

  const linkedPicture = $derived(p.mode === "paint" ? linkedFile(ctx.value) : null);
  const fetchedPicture = $derived.by(() => {
    const { live, bound, value } = ctx;
    return p.mode !== "paint" || !live || !bound
      ? undefined
      : linkedPicture && bound.field.modifiers.store
        ? live.storeFile(bound.field.modifiers.store, linkedPicture)
        : boundEntity && isStored(value)
          ? live.picture(boundEntity, Number(bound.record.id ?? 0), bound.field.name.name)
          : undefined;
  });
</script>

<CanvasView
  mode={p.mode}
  picture={p.mode === "paint" ? recordPictureUrl(ctx.value, fetchedPicture) : null}
  vector={p.mode === "vector" && typeof ctx.value === "string" ? ctx.value : null}
  tools={canvasTools(p)}
  tool={ctx.partState?.tool ?? null}
  brush={ctx.partState?.brush ?? p.brush}
  ink={ctx.partState?.ink ?? p.ink}
  toolWords={{
    select: ownWord(ctx.preview, "Select"),
    line: ownWord(ctx.preview, "Line"),
    rect: ownWord(ctx.preview, "Rectangle"),
    ellipse: ownWord(ctx.preview, "Ellipse"),
    path: ownWord(ctx.preview, "Path"),
    text: ownWord(ctx.preview, "Text"),
    pen: ownWord(ctx.preview, "Pen"),
    eraser: ownWord(ctx.preview, "Eraser"),
    canvasText: ownWord(ctx.preview, "The text"),
  }}
  oncanvas={ctx.live && p.name && !ctx.design && p.mode ? (action) => canvasActed(action) : null}
  id={ctx.id}
  drawing={ctx.partState?.drawing ?? []}
  units={p.units ?? { width: DEFAULT_UNITS, height: DEFAULT_UNITS }}
  shown={(text) => shownText(ctx.preview, text)}
  zoom={p.zoom && !ctx.design}
  grid={ctx.partState?.grid ?? p.grid}
  background={ctx.partState?.background ?? p.background}
  dotSize={ctx.partState?.dotSize ?? p.dotSize}
  dotColor={ctx.partState?.dotColor ?? p.dotColor}
  words={{ zoomOut: ownWord(ctx.preview, "Zoom out"), zoomWhole: ownWord(ctx.preview, "Show it whole"), zoomIn: ownWord(ctx.preview, "Zoom in") }}
  onpoint={ctx.live && p.name && !ctx.design && p.handlers.some((h) => h.event === "click")
    ? (point) => {
        // Where it was clicked goes with the part's state, for the handler to read.
        if (ctx.live && p.name) ctx.live.setPointer(p.name, point);
        ctx.fire("click");
      }
    : null}
  ondrag={ctx.live && p.name && !ctx.design && p.handlers.some((h) => h.event === "drag")
    ? (point) => {
        if (ctx.live && p.name) ctx.live.setPointer(p.name, point);
        return ctx.fire("drag");
      }
    : null}
  onbend={ctx.live && p.name && !ctx.design && p.handlers.some((h) => h.event === "bend")
    ? (bend) => {
        // Which link was bent and its points now go with the part's state, for the handler to read.
        if (ctx.live && p.name) ctx.live.setPointer(p.name, { x: 0, y: 0, bent: bend.name, via: bend.via });
        ctx.fire("bend");
      }
    : null}
  ondrop={ctx.live && p.name && !ctx.design && p.handlers.some((h) => h.event === "drop")
    ? (point) => {
        if (ctx.live && p.name) ctx.live.setPointer(p.name, point);
        ctx.fire("drop");
      }
    : null}
/>
