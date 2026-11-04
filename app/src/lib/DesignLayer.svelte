<script lang="ts">
  // The designer's layer over the window Clarity.UI draws (DESIGN 8.1): a
  // box over each part, where the library drew it, to pick, move, size and
  // open the code of; each grid, to drop new parts and fields into, with
  // its cells when the snap grid shows; and, while a frame region is
  // dragged, the window's edges it docks to. The drawing below takes no input.
  import type { Page } from "@clarity/ui/generated/Page.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import { GAP, rowAt } from "@clarity/ui/screen/grid.ts";
  import { locateWidget, widgetInfo } from "@clarity/ui/screen/pages.ts";
  import type { Layout } from "./clarityWindow";
  import { cellsOf, layerOf, placeIn, type LayerPart } from "./designLayer";
  import { pageDesigner } from "./pageDesignerStore.svelte";

  let { layout, page, preview }: { layout: Layout | null; page: Page; preview: Preview } = $props();

  const layer = $derived(layout ? layerOf(layout, page) : { parts: [], grids: [] });
  const DOCKS = ["top", "bottom", "left", "right"] as const;

  function where(box: { x: number; y: number; width: number; height: number }): string {
    return `left: ${box.x}px; top: ${box.y}px; width: ${box.width}px; height: ${box.height}px;`;
  }

  /** The grid a part lies in, by the design. */
  function containerOf(part: LayerPart): number | null {
    return locateWidget(preview, page, part.item)?.container.offset ?? null;
  }

  function pressMove(e: PointerEvent, part: LayerPart) {
    const found = locateWidget(preview, page, part.item);
    const pick = () => (pageDesigner.selected = part.item);
    // A label of a form's label column is the form's, and a part of a sidebar or footer is picked only.
    if (!found || found.widget.props.made) {
      e.stopPropagation();
      if (e.button === 0) pick();
      return;
    }
    const p = found.widget.props;
    const el = e.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    const cellWidth = (rect.width - GAP * (p.width - 1)) / p.width;
    // The row grabbed, counted in the grid's own rows: some are taller than others.
    const grid = el.parentElement?.querySelector<HTMLElement>(`[data-grid="${found.container.offset}"]`);
    const gridRect = grid?.getBoundingClientRect();
    const row = grid && gridRect ? rowAt(grid, e.clientY - gridRect.top, gridRect.height) : p.row;
    const grab = {
      dc: Math.min(p.width - 1, Math.max(0, Math.floor((e.clientX - rect.left) / (cellWidth + GAP)))),
      dr: Math.min(p.height - 1, Math.max(0, row - p.row)),
    };
    pageDesigner.press(e, { kind: "move", item: part.item, from: found.container.offset, props: p, grab, label: widgetInfo(preview, p.kind).name }, pick);
  }

  function pressResize(e: PointerEvent, part: LayerPart) {
    const found = locateWidget(preview, page, part.item);
    if (!found) return;
    const p = found.widget.props;
    pageDesigner.press(e, { kind: "resize", item: part.item, props: p, label: `${p.width} x ${p.height}` }, () => (pageDesigner.selected = part.item));
  }
</script>

<div class="layer" data-design-layer>
  {#each layer.grids as grid (grid.item)}
    <div
      class="grid"
      data-grid={grid.item}
      data-columns={grid.columns}
      data-inside={grid.inside.join(" ")}
      style="{where(grid.box)} grid-template-rows: {grid.rows.map((r) => `${r}px`).join(' ')}; row-gap: {grid.gap}px;"
    >
      {#if pageDesigner.showGrid}
        {#each cellsOf(grid) as cell, i (i)}
          <span class="cell" style={where(cell)}></span>
        {/each}
      {/if}
      {#if pageDesigner.hover && pageDesigner.hover.container === grid.item}
        <span class="ghost" style={where(placeIn(grid, pageDesigner.hover))}></span>
      {/if}
    </div>
  {/each}
  {#each layer.parts as part (part.item)}
    {@const selected = pageDesigner.selected === part.item}
    <div
      class="part"
      class:selected
      class:lifted={pageDesigner.drag?.kind === "move" && pageDesigner.drag.item === part.item}
      data-widget={part.item}
      data-part={part.name}
      data-container={containerOf(part)}
      style={where(part.box)}
      role="presentation"
      onpointerdown={(e) => pressMove(e, part)}
      ondblclick={(e) => {
        e.stopPropagation();
        pageDesigner.openPartCode(part.item);
      }}
    >
      <span class="tag">{widgetInfo(preview, part.kind as never)?.name ?? part.kind}</span>
      {#if selected}
        <div class="resize" role="presentation" title="Drag to resize" onpointerdown={(e) => pressResize(e, part)}></div>
      {/if}
    </div>
  {/each}
  {#if pageDesigner.drag?.kind === "region"}
    {#each DOCKS as dock (dock)}
      <div class="dock {dock}" class:active={pageDesigner.dock === dock} data-dock={dock}></div>
    {/each}
  {/if}
</div>

<style>
  .layer {
    position: absolute;
    inset: 0;
    user-select: none;
  }

  .grid {
    position: absolute;
    display: grid;
  }

  .cell {
    position: absolute;
    pointer-events: none;
    outline: 1px dashed color-mix(in srgb, var(--accent) 22%, transparent);
    outline-offset: -1px;
  }

  .ghost {
    position: absolute;
    pointer-events: none;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    outline: 2px solid var(--accent);
    border-radius: 4px;
  }

  .part {
    position: absolute;
    cursor: move;
    border-radius: 3px;
  }

  .part:hover {
    outline: 1px solid color-mix(in srgb, var(--accent) 60%, transparent);
  }

  .part.selected {
    outline: 2px solid var(--code-keyword, var(--accent));
  }

  .part.lifted {
    opacity: 0.4;
  }

  .tag {
    position: absolute;
    left: 0;
    top: -15px;
    font-size: 10px;
    line-height: 14px;
    padding: 0 4px;
    border-radius: 3px 3px 0 0;
    background: var(--code-keyword, var(--accent));
    color: #fff;
    display: none;
    pointer-events: none;
    white-space: nowrap;
  }

  .part:hover .tag,
  .part.selected .tag {
    display: block;
  }

  .resize {
    position: absolute;
    right: -5px;
    bottom: -5px;
    width: 10px;
    height: 10px;
    background: var(--code-keyword, var(--accent));
    border: 1px solid #fff;
    border-radius: 2px;
    cursor: nwse-resize;
  }

  .dock {
    position: absolute;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    border: 1px dashed var(--accent);
  }

  .dock.active {
    background: color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .dock.top {
    left: 0;
    right: 0;
    top: 0;
    height: 28px;
  }

  .dock.bottom {
    left: 0;
    right: 0;
    bottom: 0;
    height: 28px;
  }

  .dock.left {
    left: 0;
    top: 28px;
    bottom: 28px;
    width: 40px;
  }

  .dock.right {
    right: 0;
    top: 28px;
    bottom: 28px;
    width: 40px;
  }
</style>
