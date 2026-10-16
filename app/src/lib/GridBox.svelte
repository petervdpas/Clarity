<script lang="ts">
  import WidgetView from "./WidgetView.svelte";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import type { Widget } from "./generated/Widget";
  import { pageDesigner } from "./pageDesigner.svelte";
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
  }: {
    preview: Preview;
    page: Page;
    data: PageData;
    design: boolean;
    widgets: Widget[];
    columns: number;
    /** Offset of the page, panel, tab or pane this grid belongs to. */
    container: number;
    onnavigate: (target: string) => void;
    /** A grid with a fixed number of rows, such as the footer. */
    fixedRows?: number | null;
  } = $props();

  const rows = $derived(fixedRows ?? gridRows(widgets, design ? 3 : 0, design ? 4 : 1));

  // Fluid outside the designer: columns always share the width, and rows
  // holding a part that grows (a table, a panel) share the extra height.
  // The designer keeps every row at its fixed height, so placing stays exact.
  const growing = $derived.by(() => {
    const found = new Set<number>();
    if (design || fixedRows !== null) return found;
    for (const w of widgets) {
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
  const ghost = $derived(pageDesigner.hover?.container === container ? pageDesigner.hover : null);
  // Shown in red while dragging when the drop would overlap a sibling; the
  // checker in Rust still decides when the edit is applied.
  const blocked = $derived.by(() => {
    if (!ghost) return false;
    const moving = pageDesigner.drag?.kind === "move" || pageDesigner.drag?.kind === "resize" ? pageDesigner.drag.item : null;
    return widgets.some((w) => w.span.start !== moving && overlaps(w.props, ghost));
  });
</script>

<div
  class="grid"
  class:design
  data-grid={container}
  data-columns={columns}
  style:grid-template-columns="repeat({columns}, minmax(0, 1fr))"
  class:fluid={growing.size > 0}
  style:grid-template-rows={rowTemplate}
  style:gap="{GAP}px"
>
  {#if design && pageDesigner.showGrid}
    {#each { length: rows * columns } as _, i (i)}
      <div class="cell" style:grid-column={(i % columns) + 1} style:grid-row={Math.floor(i / columns) + 1}></div>
    {/each}
  {/if}

  {#each widgets as widget (widget.span.start)}
    <WidgetView {preview} {page} {data} {design} {widget} {onnavigate} />
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

  .grid.fluid {
    flex: 1;
    height: 100%;
    min-height: 0;
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
