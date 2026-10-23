<script lang="ts">
  import PopoverView from "./PopoverView.svelte";
  import WidgetView from "./WidgetView.svelte";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import type { Widget } from "./generated/Widget";
  import { pageDesigner } from "./pageDesignerStore.svelte";
  import { GAP, ROW_HEIGHT, gridRows, overlaps, widgetGrows, type PageData } from "./pages";

  let {
    preview,
    page,
    data,
    design,
    widgets,
    columns,
    container,
    onnavigate,
    fixedRows = null,
    joined = false,
  }: {
    preview: Preview;
    page: Page;
    data: PageData;
    design: boolean;
    widgets: Widget[];
    columns: number;
    /** Offset of the page, panel, tab or pane this grid belongs to. */
    container: number;
    onnavigate: (target: string, newWindow?: boolean) => void;
    /** A grid with a fixed number of rows, such as the footer. */
    fixedRows?: number | null;
    /** Parts that touch, without the space between columns: a button group. */
    joined?: boolean;
  } = $props();

  // While a part is dragged, the designer offers rows below the content to drop on.
  const dragging = $derived(design && pageDesigner.drag !== null);
  // A sound has no place in the grid (DESIGN 5.22): the designer shows it in a tray.
  const placed = $derived(widgets.filter((w) => w.props.kind !== "sound"));
  const rows = $derived(fixedRows ?? gridRows(placed, dragging ? 3 : 0, dragging ? 4 : 1));

  // Fluid, in the designer as when the window runs: columns always share the
  // width, and rows holding a part that grows (a table, a panel) share the
  // extra height of the window.
  const growing = $derived.by(() => {
    const found = new Set<number>();
    if (fixedRows !== null) return found;
    for (const w of placed) {
      if (!widgetGrows(preview, w)) continue;
      for (let r = w.props.row; r < w.props.row + w.props.height; r++) found.add(r);
    }
    return found;
  });
  const rowTemplate = $derived(
    growing.size === 0
      ? `repeat(${rows}, ${ROW_HEIGHT}px)`
      : Array.from({ length: rows }, (_, i) => (growing.has(i + 1) ? `minmax(${ROW_HEIGHT}px, 1fr)` : `${ROW_HEIGHT}px`)).join(" "),
  );
  // In the designer the grid reaches to the bottom of its window or
  // container, so the free space below the content (empty when the window
  // runs) shows cells to drop on.
  const fills = $derived(design && fixedRows === null && growing.size === 0);
  let height = $state(0);
  const freeRows = $derived(
    fills ? Math.max(0, Math.floor((height + GAP) / (ROW_HEIGHT + GAP)) - rows) : 0,
  );
  const ghost = $derived(pageDesigner.hover?.container === container ? pageDesigner.hover : null);
  // Shown in red while dragging when the drop would overlap a sibling; the
  // checker in Rust still decides when the edit is applied.
  const blocked = $derived.by(() => {
    if (!ghost) return false;
    const moving = pageDesigner.drag?.kind === "move" || pageDesigner.drag?.kind === "resize" ? pageDesigner.drag.item : null;
    return placed.some((w) => w.span.start !== moving && overlaps(w.props, ghost));
  });
</script>

<div
  class="grid"
  class:design
  data-grid={container}
  data-columns={columns}
  style:grid-template-columns="repeat({columns}, minmax(0, 1fr))"
  class:fluid={growing.size > 0}
  class:fills
  bind:clientHeight={height}
  style:grid-template-rows={rowTemplate}
  style:grid-auto-rows="{ROW_HEIGHT}px"
  style:gap={joined ? "0px" : `${GAP}px`}
>
  {#if design && pageDesigner.showGrid}
    {#each { length: (rows + freeRows) * columns } as _, i (i)}
      <div class="cell" style:grid-column={(i % columns) + 1} style:grid-row={Math.floor(i / columns) + 1}></div>
    {/each}
  {/if}

  {#each placed as widget (widget.span.start)}
    {#if widget.props.kind === "popover"}
      <!-- A popover has no place in the grid: it opens next to its part. -->
      <PopoverView {preview} {page} {data} {design} {widget} {onnavigate} />
    {:else}
      <WidgetView {preview} {page} {data} {design} {widget} {onnavigate} />
    {/if}
  {/each}

  {#if ghost}
    <div
      class="ghost"
      class:blocked
      style:grid-column="{ghost.col} / span {ghost.width}"
      style:grid-row="{ghost.row} / span {ghost.height}"
    ></div>
  {/if}
</div>

<style>
  .grid {
    display: grid;
    position: relative;
    min-width: 0;
  }

  .grid.fluid,
  .grid.fills {
    flex: 1;
    height: 100%;
    min-height: 0;
  }

  /* The content stays at the top, as when the window runs; the rows below
     are the free space. */
  .grid.fills {
    align-content: start;
  }

  .cell {
    border: 1px dashed var(--border);
    border-radius: 3px;
    opacity: 0.6;
    pointer-events: none;
  }

  .ghost {
    border: 2px solid var(--accent);
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-radius: 4px;
    pointer-events: none;
    z-index: 5;
  }

  .ghost.blocked {
    border-color: var(--error);
    background: color-mix(in srgb, var(--error) 18%, transparent);
  }
</style>
