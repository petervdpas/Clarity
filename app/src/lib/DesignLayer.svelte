<script lang="ts">
  import { designLog } from "./designLog";
  // The designer's layer over the window Clarity.UI draws (DESIGN 8.1): a
  // box over each part, where the library drew it, to pick, move, size and
  // open the code of; each grid, to drop new parts and fields into, with
  // its cells when the snap grid shows; and, while a frame region is
  // dragged, the window's edges it docks to. A container's headers switch
  // the page it shows, a table's header edges size its columns, and a
  // frame's buttons are picked. A splitter's divider, dragged, sets the size
  // its first pane starts with; a slide-out's inner edge, dragged, its width
  // (its height at the top or bottom). The drawing below takes no input.
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import { gapOf, rowAt } from "@clarity/ui/screen/grid.ts";
  import { contentViews, locateKey, widgetInfo, withProps, type DesignKey, type ShownPage } from "@clarity/ui/screen/pages.ts";
  import type { Layout } from "./clarityWindow";
  import { cellsOf, dividerOf, layerOf, paneSizeAt, placeIn, slideEdgeOf, slideSizeAt, type LayerGrid, type LayerPart } from "./designLayer";
  import { pageDesigner } from "./pageDesignerStore.svelte";

  let {
    layout,
    shown,
    preview,
    appCommands = new Set(),
  }: {
    layout: Layout | null;
    /** The window as designed, with the key of each thing it shows. */
    shown: ShownPage;
    preview: Preview;
    /** The keys of the app's frame buttons the window shows, which are designed in the app. */
    appCommands?: Set<DesignKey>;
  } = $props();

  const page = $derived(shown.page);
  const layer = $derived(layout ? layerOf(layout, page, shown.keyOf, appCommands, contentViews(preview, shown)) : { parts: [], grids: [] });
  const DOCKS = ["top", "bottom", "left", "right"] as const;

  /** How high a row of a grid is when it holds nothing: on a page with its own cells (`grid 56 columns rows 10`) every grid's, else the project's. */
  function rowHeightOf(_grid: LayerGrid): number {
    return page.cell ?? preview.project.rowHeight;
  }

  function where(box: { x: number; y: number; width: number; height: number }): string {
    return `left: ${box.x}px; top: ${box.y}px; width: ${box.width}px; height: ${box.height}px;`;
  }

  /** The grid a part lies in, by the design. */
  function containerOf(part: LayerPart): DesignKey | null {
    return locateKey(preview, shown, part.item)?.container.key ?? null;
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
    const found = locateKey(preview, shown, part.item);
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
      pageDesigner.edit({ op: "updateWidget", item: found.widget.span.start, container: null, props: withProps(p, { widths: { ...p.widths, [field]: done.width } }) }, part.item);
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", up);
  }

  /** A column's width while its edge is dragged. */
  let sizing = $state<{ item: DesignKey; field: string; width: number } | null>(null);

  /** Whether a splitter lays its panes one above the other. */
  function stackedOf(part: LayerPart): boolean {
    return locateKey(preview, shown, part.item)?.widget.props.orientation === "vertical";
  }

  /** A splitter's divider while it is dragged: where it is now, and the size the first pane would start with. */
  let splitting = $state<{ item: DesignKey; at: number; size: number } | null>(null);

  /** A splitter's divider, dragged: the size its first pane starts with (`rows` stacked, `span` side by side), written into the pane. */
  function pressDivider(e: PointerEvent, part: LayerPart) {
    const found = locateKey(preview, shown, part.item);
    if (!found || e.button !== 0 || found.widget.pages.length !== 2) return;
    e.preventDefault();
    e.stopPropagation();
    const props = found.widget.props;
    const stacked = props.orientation === "vertical";
    const length = stacked ? props.height : props.width;
    const handle = e.currentTarget as HTMLElement;
    const layerBox = handle.closest<HTMLElement>("[data-design-layer]");
    if (!layerBox) return;
    // The window may be shown smaller than it is.
    const scale = layerBox.offsetWidth > 0 ? layerBox.getBoundingClientRect().width / layerBox.offsetWidth : 1;
    const origin = layerBox.getBoundingClientRect();
    const along = (m: PointerEvent) => (stacked ? (m.clientY - origin.top) / scale : (m.clientX - origin.left) / scale);
    handle.setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => {
      const at = along(m);
      splitting = { item: part.item, at, size: paneSizeAt(part, at, length, stacked) };
    };
    const up = () => {
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", up);
      const done = splitting;
      splitting = null;
      if (!done || done.size === found.widget.pages[0].columns) return;
      pageDesigner.edit({ op: "setPaneSpan", target: found.widget.pages[0].span.start, columns: done.size }, part.item);
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", up);
  }

  /** A slide-out's inner edge while it is dragged: where it is now, and the size it would take. */
  let sliding = $state<{ item: DesignKey; at: number; size: number } | null>(null);

  /** The side a slide-out comes in from, as designed; the right without one. */
  function slideSideOf(part: LayerPart) {
    return locateKey(preview, shown, part.item)?.widget.props.side ?? null;
  }

  /** A slide-out's inner edge, dragged: its width (`slideWidth`), or its height at the top or bottom (`slideHeight`), within its min and max. */
  function pressSlideEdge(e: PointerEvent, part: LayerPart) {
    const found = locateKey(preview, shown, part.item);
    if (!found || e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const props = found.widget.props;
    const side = props.side ?? null;
    const across = side === "top" || side === "bottom";
    const limit = (v: string | null) => (v === null || v.trim() === "" || Number.isNaN(Number(v)) ? null : Number(v));
    const handle = e.currentTarget as HTMLElement;
    const layerBox = handle.closest<HTMLElement>("[data-design-layer]");
    if (!layerBox) return;
    // The window may be shown smaller than it is.
    const scale = layerBox.offsetWidth > 0 ? layerBox.getBoundingClientRect().width / layerBox.offsetWidth : 1;
    const origin = layerBox.getBoundingClientRect();
    const along = (m: PointerEvent) => (across ? (m.clientY - origin.top) / scale : (m.clientX - origin.left) / scale);
    handle.setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => {
      const at = along(m);
      sliding = { item: part.item, at, size: slideSizeAt(part, side, at, limit(props.min), limit(props.max)) };
    };
    const up = () => {
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", up);
      const done = sliding;
      sliding = null;
      if (!done) return;
      const now = across ? props.slideHeight : props.slideWidth;
      if (done.size === now) return;
      const changed = across ? { slideHeight: done.size } : { slideWidth: done.size };
      pageDesigner.edit({ op: "updateWidget", item: found.widget.span.start, container: null, props: withProps(props, changed) }, part.item);
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", up);
  }

  function pressMove(e: PointerEvent, part: LayerPart) {
    const found = locateKey(preview, shown, part.item);
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
    // The row grabbed, counted in the grid's own rows: some are taller than others.
    const grid = found.container.key === null ? null : el.parentElement?.querySelector<HTMLElement>(`[data-grid="${CSS.escape(found.container.key)}"]`);
    const gap = grid ? gapOf(grid) : 0;
    const cellWidth = (rect.width - gap * (p.width - 1)) / p.width;
    const gridRect = grid?.getBoundingClientRect();
    const row = grid && gridRect ? rowAt(grid, e.clientY - gridRect.top, gridRect.height) : p.row;
    const grab = {
      dc: Math.min(p.width - 1, Math.max(0, Math.floor((e.clientX - rect.left) / (cellWidth + gap)))),
      dr: Math.min(p.height - 1, Math.max(0, row - p.row)),
    };
    pageDesigner.press(e, { kind: "move", item: part.item, from: found.container.key ?? "", props: p, grab, label: widgetInfo(preview, p.kind).name }, pick);
  }

  function pressResize(e: PointerEvent, part: LayerPart) {
    const found = locateKey(preview, shown, part.item);
    designLog("resize press on item", part.item, found ? `found ${found.widget.props.kind} ${found.widget.props.width}x${found.widget.props.height}` : "NOT FOUND in the page");
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
      data-row-height={rowHeightOf(grid)}
      data-gap={grid.gap}
      data-inside={JSON.stringify(grid.inside)}
      style="{where(grid.box)} grid-template-rows: {grid.rows.map((r) => `${r}px`).join(' ')}; row-gap: {grid.gap}px;"
    >
      {#if pageDesigner.showGrid}
        {#each cellsOf(grid, rowHeightOf(grid)) as cell, i (i)}
          <span class="cell" style={where(cell)}></span>
        {/each}
      {/if}
      {#if pageDesigner.hover && pageDesigner.hover.container === grid.item}
        <span class="ghost" style={where(placeIn(grid, pageDesigner.hover, rowHeightOf(grid)))}></span>
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
      class:content={part.kind === "content"}
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
      {#if part.kind === "content"}
        <!-- Content has no look of its own: the designer marks it, and names the view it shows. -->
        <span class="tag content-tag">content {part.name ?? ""}: {part.pages[pageDesigner.activeTabs[part.item] ?? 0]?.title ?? ""}</span>
      {:else}
        <span class="tag">{widgetInfo(preview, part.kind as never)?.name ?? part.kind}</span>
      {/if}
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
    {#if (part.kind === "steps" || part.kind === "content") && part.pages.length > 1}
      <!-- A wizard's steps and a content's views have no header to press: the designer gives each a button of its own. -->
      <div class="page-chips" style="left: {part.box.x + part.box.width}px; top: {part.box.y}px;">
        {#each part.pages as shownPage, index (shownPage.item)}
          <button
            class:active={(pageDesigner.activeTabs[part.item] ?? 0) === index}
            data-tab-switch="{part.item}:{index}"
            data-page={shownPage.item}
            title={shownPage.title ?? `Step ${index + 1}`}
            onpointerdown={(e) => pressPage(e, part, index)}>{part.kind === "content" ? (shownPage.title ?? index + 1) : index + 1}</button
          >
        {/each}
      </div>
    {/if}
    {#if part.kind === "splitter" && part.pages.length === 2}
      {@const stacked = stackedOf(part)}
      {@const divider = dividerOf(part, layer.grids, stacked)}
      {#if divider}
        <div
          class="pane-divider"
          class:stacked
          data-divider={part.item}
          style={where(divider)}
          role="separator"
          aria-label="Where the first pane of {part.name ?? 'the splitter'} ends"
          title={stacked ? "Drag to set the rows the first pane starts with" : "Drag to set the columns the first pane starts with"}
          onpointerdown={(e) => pressDivider(e, part)}
        ></div>
        {#if splitting && splitting.item === part.item}
          <span
            class="pane-size"
            style={stacked
              ? `left: ${part.box.x}px; top: ${splitting.at - 1}px; width: ${part.box.width}px; height: 2px;`
              : `left: ${splitting.at - 1}px; top: ${part.box.y}px; width: 2px; height: ${part.box.height}px;`}
          ><em>{splitting.size} {stacked ? "rows" : "columns"}</em></span>
        {/if}
      {/if}
    {/if}
    {#if part.kind === "slideout"}
      {@const side = slideSideOf(part)}
      {@const across = side === "top" || side === "bottom"}
      <div
        class="slide-edge"
        class:across
        data-slide-edge={part.item}
        style={where(slideEdgeOf(part, side))}
        role="separator"
        aria-label="The inner edge of {part.name ?? 'the slide-out'}"
        title={across ? "Drag to set the slide-out's height" : "Drag to set the slide-out's width"}
        onpointerdown={(e) => pressSlideEdge(e, part)}
      ></div>
      {#if sliding && sliding.item === part.item}
        <span
          class="pane-size"
          style={across
            ? `left: ${part.box.x}px; top: ${sliding.at - 1}px; width: ${part.box.width}px; height: 2px;`
            : `left: ${sliding.at - 1}px; top: ${part.box.y}px; width: 2px; height: ${part.box.height}px;`}
        ><em>{sliding.size} px</em></span>
      {/if}
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

  /* A window's content draws nothing itself: the designer always shows where it is. */
  .part.content {
    outline: 1px dashed color-mix(in srgb, var(--code-keyword, var(--accent)) 70%, transparent);
    outline-offset: -1px;
  }

  .part.content.selected {
    outline: 2px dashed var(--code-keyword, var(--accent));
  }

  .part.content .content-tag {
    display: block;
    top: 0;
    border-radius: 0 0 3px 0;
    opacity: 0.85;
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

  .pane-divider {
    position: absolute;
    cursor: ew-resize;
    z-index: 3;
  }

  .pane-divider.stacked {
    cursor: ns-resize;
  }

  .slide-edge {
    position: absolute;
    cursor: ew-resize;
    z-index: 3;
  }

  .slide-edge.across {
    cursor: ns-resize;
  }

  .slide-edge:hover,
  .pane-divider:hover {
    background: color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .pane-size {
    position: absolute;
    pointer-events: none;
    background: var(--accent);
    z-index: 3;
  }

  .pane-size em {
    position: absolute;
    left: 4px;
    top: 4px;
    font-style: normal;
    font-size: 10px;
    color: var(--accent);
    white-space: nowrap;
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
