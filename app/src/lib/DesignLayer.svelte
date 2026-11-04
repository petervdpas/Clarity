<script lang="ts">
  // The designer's layer over the window Clarity.UI draws (DESIGN 8.1): a
  // box over each part, where the library drew it, to pick, move, size and
  // open the code of; each grid, to drop new parts and fields into, with
  // its cells when the snap grid shows; and, while a frame region is
  // dragged, the window's edges it docks to. A container's headers switch
  // the page it shows, a table's header edges size its columns, and a
  // frame's buttons are picked. The drawing below takes no input.
  import type { Frame } from "@clarity/ui/generated/Frame.ts";
  import type { Page } from "@clarity/ui/generated/Page.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import { GAP, rowAt } from "@clarity/ui/screen/grid.ts";
  import { locateWidget, widgetInfo, withProps } from "@clarity/ui/screen/pages.ts";
  import type { Layout } from "./clarityWindow";
  import { cellsOf, layerOf, placeIn, type LayerPart } from "./designLayer";
  import { pageDesigner } from "./pageDesignerStore.svelte";

  let { layout, page, preview, appFrame = null }: { layout: Layout | null; page: Page; preview: Preview; appFrame?: Frame | null } = $props();

  const layer = $derived(layout ? layerOf(layout, page, appFrame) : { parts: [], grids: [] });
  const DOCKS = ["top", "bottom", "left", "right"] as const;

  function where(box: { x: number; y: number; width: number; height: number }): string {
    return `left: ${box.x}px; top: ${box.y}px; width: ${box.width}px; height: ${box.height}px;`;
  }

  /** The grid a part lies in, by the design. */
  function containerOf(part: LayerPart): number | null {
    return locateWidget(preview, page, part.item)?.container.offset ?? null;
  }

  /** A header of a container: it shows that page, and the page is picked. */
  function pressPage(e: PointerEvent, part: LayerPart, index: number) {
    if (e.button !== 0) return;
    e.stopPropagation();
    pageDesigner.activeTabs[part.item] = index;
    pageDesigner.selected = part.pages[index].item;
  }

  /** A frame's button: the window's own is picked; its app's is designed in the app. */
  function pressCommand(e: PointerEvent, part: LayerPart) {
    if (e.button !== 0) return;
    e.stopPropagation();
    if (part.own) pageDesigner.selected = part.item;
  }

  /** The edge of a table's column header, dragged: the column's width, written into the part. */
  function pressColumn(e: PointerEvent, part: LayerPart, field: string, width: number) {
    const found = locateWidget(preview, page, part.item);
    if (!found || e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const handle = e.currentTarget as HTMLElement;
    const layerBox = handle.closest<HTMLElement>("[data-design-layer]");
    // The window may be shown smaller than it is.
    const scale = layerBox && layerBox.offsetWidth > 0 ? layerBox.getBoundingClientRect().width / layerBox.offsetWidth : 1;
    const from = e.clientX;
    handle.setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => (sizing = { item: part.item, field, width: Math.min(2000, Math.max(20, Math.round(width + (m.clientX - from) / scale))) });
    const up = () => {
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", up);
      const done = sizing;
      sizing = null;
      if (!done || done.width === Math.round(width)) return;
      const p = found.widget.props;
      pageDesigner.edit({ op: "updateWidget", item: part.item, container: null, props: withProps(p, { widths: { ...p.widths, [field]: done.width } }) });
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", up);
  }

  /** A column's width while its edge is dragged. */
  let sizing = $state<{ item: number; field: string; width: number } | null>(null);

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
    {#if part.kind === "command"}
      <div
        class="part command"
        class:selected
        class:app={!part.own}
        data-command={part.item}
        data-part={part.name}
        style={where(part.box)}
        role="presentation"
        title={part.own ? undefined : "The app's: double-click to design it in the app"}
        onpointerdown={(e) => pressCommand(e, part)}
        ondblclick={(e) => {
          e.stopPropagation();
          if (part.own) pageDesigner.openPartCode(part.item);
          else pageDesigner.openInApp(part.item);
        }}
      ></div>
    {:else}
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
    {#each part.pages as shownPage, index (shownPage.item)}
      {#if shownPage.box}
        <div
          class="page-header"
          class:selected={pageDesigner.selected === shownPage.item}
          data-tab-switch="{part.item}:{index}"
          data-page={shownPage.item}
          style={where(shownPage.box)}
          role="presentation"
          title="Show this page"
          onpointerdown={(e) => pressPage(e, part, index)}
        ></div>
      {/if}
    {/each}
    {#if part.kind === "steps" && part.pages.length > 1}
      <!-- A wizard's steps have no header to press: the designer gives each a button of its own. -->
      <div class="page-chips" style="left: {part.box.x + part.box.width}px; top: {part.box.y}px;">
        {#each part.pages as shownPage, index (shownPage.item)}
          <button
            class:active={(pageDesigner.activeTabs[part.item] ?? 0) === index}
            data-tab-switch="{part.item}:{index}"
            data-page={shownPage.item}
            title={shownPage.title ?? `Step ${index + 1}`}
            onpointerdown={(e) => pressPage(e, part, index)}>{index + 1}</button
          >
        {/each}
      </div>
    {/if}
    {#each part.columns as column (column.field)}
      <div
        class="column-edge"
        data-column={column.field}
        data-table={part.item}
        style="left: {column.box.x + column.box.width - 3}px; top: {column.box.y}px; height: {column.box.height}px;"
        role="separator"
        aria-label="Width of the column {column.field}"
        title="Drag to set the width of {column.field}"
        onpointerdown={(e) => pressColumn(e, part, column.field, column.box.width)}
      ></div>
      {#if sizing && sizing.item === part.item && sizing.field === column.field}
        <span class="column-size" style="left: {column.box.x}px; top: {column.box.y}px; width: {sizing.width}px; height: {column.box.height}px;">{sizing.width}</span>
      {/if}
    {/each}
    {/if}
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

  .part.command {
    cursor: pointer;
  }

  .part.command.app:hover {
    outline-style: dashed;
  }

  .page-header {
    position: absolute;
    cursor: pointer;
    border-radius: 3px;
  }

  .page-header:hover,
  .page-header.selected {
    outline: 1px solid var(--code-keyword, var(--accent));
  }

  .page-chips {
    position: absolute;
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-left: 4px;
  }

  .page-chips button {
    font-size: 10px;
    padding: 0 5px;
    line-height: 16px;
    border-radius: 3px;
  }

  .page-chips button.active {
    background: var(--code-keyword, var(--accent));
    color: #fff;
  }

  .column-edge {
    position: absolute;
    width: 6px;
    cursor: ew-resize;
    z-index: 2;
  }

  .column-edge:hover {
    background: color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .column-size {
    position: absolute;
    pointer-events: none;
    outline: 2px solid var(--accent);
    font-size: 10px;
    color: var(--accent);
    text-align: right;
    padding-right: 4px;
    box-sizing: border-box;
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
