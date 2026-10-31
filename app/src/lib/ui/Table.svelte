<script lang="ts" module>
  import type { JsonValue } from "../generated/serde_json/JsonValue";
  import type { Choice } from "./ChoiceInput.svelte";

  /** What a cell holds: the kind of value of its column. */
  export type CellKind = "text" | "number" | "bool" | "choice" | "date" | "moment" | "color";

  /** A column of a table: the field it shows, and whether and how it is edited. */
  export type TableColumn = {
    field: string;
    label: string;
    kind: CellKind;
    /** Whether its cells are inputs (`edit`). */
    edit: boolean;
    /** A bool column shown as a switch (`as switch`). */
    switch?: boolean;
    /** The choices of a choice column: an enum's values, or the records a reference can point at. */
    choices?: Choice[];
    /** A value as the column shows it, when it is not edited. */
    shown?: (value: JsonValue | undefined) => string;
    /** How a number cell shows its value and reads what is typed (null: no number). */
    number?: { text: (value: JsonValue | undefined) => string; read: (text: string) => JsonValue | null };
    /** The designed width in pixels (`columns title 240`); none shares what is left. */
    width?: number;
  };

  export type Row = Record<string, JsonValue>;
</script>

<script lang="ts">
  // Clarity's table part on the web. It owns how a table behaves, the same as
  // on Avalonia (conformance/tables.json): which input a cell holds, when a
  // row counts as chosen and when it counts as edited, and how text that is
  // no value is marked. The designer and running windows only describe the
  // columns. Rows show in the table's order (`order by`, a click on a
  // `sortable` header) but keep their place in the data, which every event
  // names (conformance/table-order.json). With `resizable` the header edges
  // are dragged and the widths remembered under `remember`; a double-click
  // on an edge gives the designed widths back. In the designer the edges set
  // the designed widths.
  import CheckInput from "./CheckInput.svelte";
  import ChoiceInput from "./ChoiceInput.svelte";
  import ColorInput from "./ColorInput.svelte";
  import DateInput from "./DateInput.svelte";
  import DateTimeInput from "./DateTimeInput.svelte";
  import NumberInput from "./NumberInput.svelte";
  import TextInput from "./TextInput.svelte";
  import { readWidths, resized } from "../columnWidths";
  import { nextOrder, pageOf, pageWith, shownOrder, type Order } from "./tableView";
  import { above, shownTree } from "./trees";
  import type { Move } from "./rowDrag";
  import { keyMove, rowDrag } from "./rowDrag";

  let {
    columns,
    rows,
    selected = null,
    disabled = false,
    readonly = false,
    design = false,
    resizable = false,
    sortable = false,
    order = null,
    remember = null,
    page = null,
    tree = null,
    header = true,
    pageWords = { first: "First page", previous: "Previous page", next: "Next page", last: "Last page" },
    ondesignwidth,
    onselect,
    onopen,
    onedit,
    onchanged = () => {},
    reorder = false,
    onreorder = () => {},
    view = null,
    onview = undefined,
  }: {
    columns: TableColumn[];
    rows: Row[];
    /** The chosen row, by its place. */
    selected?: number | null;
    disabled?: boolean;
    readonly?: boolean;
    /** In the designer: shown, but nothing can be done. */
    design?: boolean;
    /** The person drags the header edges (`resizable`). */
    resizable?: boolean;
    /** A click on a header sorts by its column (`sortable`). */
    sortable?: boolean;
    /** The order the rows show in first (`order by`). */
    order?: Order;
    /** Where the person's widths are kept, per window and table; none keeps them for the run. */
    remember?: string | null;
    /** The rows shown at a time (`page 50`); none shows them all. */
    page?: number | null;
    /**
     * A tree (U1c): the field of each row that holds its parent, the words of
     * the toggle, and the field a parent names a row by: its id, or in the
     * designer the text sample references hold.
     */
    tree?: { parent: string; open: string; close: string; key?: string } | null;
    /** Whether the column headers show; a tree shows none. */
    header?: boolean;
    /** Clarity's own words on the page bar's buttons. */
    pageWords?: { first: string; previous: string; next: string; last: string };
    /** In the designer: a header edge was dragged; 0 takes the designed width away. */
    ondesignwidth?: (field: string, width: number) => void;
    /** Another row was chosen: Clarity's `select`. */
    onselect: (row: number) => void;
    /** A row was opened (double-click): Clarity's `open`. */
    onopen: (row: number) => void;
    /** The person edited the row, which is chosen first: Clarity's `change`. */
    onedit: (row: number) => void;
    /** A cell the person changed, with its value before and after: an edit for undo (DESIGN 5.29). */
    onchanged?: (record: Record<string, JsonValue>, field: string, before: JsonValue | undefined, after: JsonValue) => void;
    /** The person moves rows by dragging and with Alt and the arrows (`reorder by`, DESIGN 5.30). */
    reorder?: boolean;
    /** A row was moved: by its place in the list, onto another row or by a key. */
    onreorder?: (move: Move) => void;
    /**
     * What the runtime filled in of this part's view (DESIGN 1.7): the places
     * of the rows shown (a tree's as [place, depth, children]), the page, the
     * order, the open rows. Without one (the designer, or before the first
     * answer) the table arranges its rows itself.
     */
    view?: { shown?: unknown[]; pages?: { page: number; pages: number; from: number; to: number; count: number }; order?: Order; open?: unknown[] } | null;
    /** The person sorted, turned a page or opened a tree row: for the runtime to decide. */
    onview?: (action: Record<string, unknown>) => void;
  } = $props();

  /** Whether the runtime has filled in what shows. */
  const filled = $derived(Array.isArray(view?.shown));

  const inert = $derived(design || disabled);

  // The order the rows show in: the table's first order, then the person's clicks.
  let sorting = $state<Order>(null);
  let sortingFrom = $state("");
  $effect.pre(() => {
    const written = JSON.stringify(order);
    if (written !== sortingFrom) {
      sortingFrom = written;
      sorting = order;
    }
  });
  const orderedRows = $derived(filled && !tree ? (view!.shown as number[]) : shownOrder(rows, columns, sorting));
  /** The order shown: the runtime's, else the table's own. */
  const shownSorting = $derived(filled && view?.order !== undefined ? view.order : sorting);

  // A page at a time (conformance/table-pages.json): the page stays when the
  // rows come again, the first one shows when the order changes, and a row
  // selected by code shows its page.
  let at = $state(1);
  const paged = $derived(
    filled && view?.pages
      ? { ...view.pages, text: view.pages.count === 0 ? "0 / 0" : `${view.pages.from}-${view.pages.to} / ${view.pages.count}` }
      : page
        ? pageOf(orderedRows.length, page, at)
        : null,
  );
  // A tree (conformance/trees.json): the rows under their parents, the open ones showing theirs.
  let opened = $state(new Set<unknown>());
  const idOf = (row: Row | undefined) => (row ? row[tree?.key ?? "id"] : undefined);
  const treeRows = $derived(tree ? rows.map((r) => ({ id: idOf(r), parent: r[tree.parent] ?? null })) : []);
  // The designer shows a tree all open, so its levels show.
  const openIds = $derived(design ? new Set(treeRows.map((r) => r.id)) : filled ? new Set(view?.open ?? []) : opened);
  const treeShown = $derived(
    !tree ? [] : filled ? (view!.shown as [number, number, boolean][]).map(([place, depth, children]) => ({ place, depth, children })) : shownTree(treeRows, openIds),
  );
  const depths = $derived(new Map(treeShown.map((t) => [t.place, t])));
  function toggle(place: number) {
    const id = idOf(rows[place]);
    if (onview && filled) {
      onview(openIds.has(id) ? { close: id } : { open: id });
      return;
    }
    const next = new Set(opened);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    opened = next;
  }
  $effect.pre(() => {
    // The runtime opens the rows above a chosen row itself.
    if (!tree || selected === null || filled) return;
    const missing = above(treeRows, selected).filter((id) => !opened.has(id));
    if (missing.length) opened = new Set([...opened, ...missing]);
  });
  const shownRows = $derived(tree ? treeShown.map((t) => t.place) : filled ? orderedRows : paged ? orderedRows.slice(Math.max(paged.from - 1, 0), paged.to) : orderedRows);
  $effect.pre(() => {
    // The runtime shows the page of a chosen row itself.
    if (!page || selected === null || filled) return;
    const place = orderedRows.indexOf(selected);
    if (place >= 0) at = pageWith(page, place);
  });

  function sortBy(field: string) {
    if (!sortable || design) return;
    if (onview && filled) {
      onview({ sort: field });
      return;
    }
    sorting = nextOrder(sorting, field);
    at = 1;
  }

  /** A page button: the runtime turns the page when it shows the rows, else the table. */
  function turn(page: number) {
    if (onview && filled) onview({ page });
    else at = page;
  }

  // Widths: the person's (remembered), else the designed ones; while an edge is dragged, those being dragged.
  const MAX_WIDTH = 2000;
  const DEFAULT_WIDTH = 120;
  function load(key: string | null): Record<string, number> | null {
    if (!key) return null;
    try {
      const widths = readWidths(localStorage.getItem(key));
      return Object.keys(widths).length > 0 ? widths : null;
    } catch {
      return null;
    }
  }
  function keep(key: string | null, widths: Record<string, number> | null) {
    if (!key) return;
    try {
      if (widths) localStorage.setItem(key, JSON.stringify(widths));
      else localStorage.removeItem(key);
    } catch {
      // Without storage the widths last for the run.
    }
  }
  let remembered = $state<Record<string, number> | null>(null);
  let rememberedFrom = $state<string | null | undefined>(undefined);
  $effect.pre(() => {
    if (remember !== rememberedFrom) {
      rememberedFrom = remember;
      remembered = design ? null : load(remember);
    }
  });
  let dragging = $state<Record<string, number> | null>(null);
  const sized = $derived(dragging ?? remembered);
  const widthOf = (column: TableColumn): number | undefined => (sized ? (sized[column.field] ?? column.width ?? DEFAULT_WIDTH) : column.width);
  const fixed = $derived(sized !== null || columns.some((c) => c.width));
  const tableWidth = $derived(sized ? `${columns.reduce((sum, c) => sum + (widthOf(c) ?? 0), 0)}px` : undefined);
  const grips = $derived(design ? ondesignwidth !== undefined : resizable && !disabled);

  let head: HTMLTableRowElement | undefined = $state();

  /** Drags the right edge of `field`'s header; let go, the widths are kept (or, in the designer, written). */
  function pressEdge(e: PointerEvent, field: string) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const cells = [...(head?.cells ?? [])];
    const measured = Object.fromEntries(columns.map((c, i) => [c.field, Math.round(cells[i]?.offsetWidth ?? widthOf(c) ?? DEFAULT_WIDTH)]));
    const start = { x: e.clientX, width: measured[field] };
    dragging = measured;
    const move = (ev: PointerEvent) => {
      if (dragging) dragging = { ...dragging, [field]: Math.min(MAX_WIDTH, resized(start.width, ev.clientX - start.x)) };
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      const done = dragging;
      dragging = null;
      if (!done) return;
      if (design) {
        if (done[field] !== start.width) ondesignwidth?.(field, done[field]);
      } else {
        remembered = done;
        keep(remember, done);
      }
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  /** A double-click on an edge: the designed widths again (in the designer, this column's width taken away). */
  function resetEdge(e: MouseEvent, field: string) {
    e.stopPropagation();
    if (design) {
      ondesignwidth?.(field, 0);
      return;
    }
    remembered = null;
    keep(remember, null);
  }

  function choose(row: number) {
    if (design || selected === row) return;
    onselect(row);
  }

  /** The person changed a cell: only a value that differs is an edit. */
  function edit(row: number, field: string, next: JsonValue) {
    const record = rows[row];
    if (!record || inert || (record[field] ?? null) === next) return;
    onchanged(record, field, structuredClone($state.snapshot(record[field] as unknown)) as JsonValue | undefined, next);
    record[field] = next;
    choose(row);
    onedit(row);
  }

  const shown = (column: TableColumn, value: JsonValue | undefined) => (column.shown ? column.shown(value) : value === null || value === undefined ? "" : String(value));
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_static_element_interactions -->
<div
  class="table-wrap"
  class:paged={paged !== null}
  tabindex={reorder && !inert ? 0 : undefined}
  onkeydown={(e) => {
    const move = reorder && !inert ? keyMove(e, selected) : null;
    if (move) {
      e.preventDefault();
      onreorder(move);
    }
  }}
>
  <table class:fixed class:no-header={!header} class:tree={tree !== null} style:width={tableWidth}>
    <thead>
      <tr bind:this={head}>
        {#each columns as column (column.field)}
          {@const sortedHere = shownSorting?.field === column.field}
          <th
            data-field={column.field}
            class:sortable={sortable && !design}
            style:width={widthOf(column) ? `${widthOf(column)}px` : undefined}
            aria-sort={sortedHere ? (shownSorting?.desc ? "descending" : "ascending") : undefined}
            tabindex={sortable && !design ? 0 : undefined}
            onclick={() => sortBy(column.field)}
            onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), sortBy(column.field))}
          >
            <span class="label">{column.label}</span>
            {#if sortedHere}<span class="arrow" aria-hidden="true">{shownSorting?.desc ? "▼" : "▲"}</span>{/if}
            {#if grips}
              <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
              <span
                class="grip"
                title={design ? "Drag to set the width; double-click to take it away" : "Drag to resize; double-click for the designed widths"}
                onpointerdown={(e) => pressEdge(e, column.field)}
                onclick={(e) => e.stopPropagation()}
                ondblclick={(e) => resetEdge(e, column.field)}
              ></span>
            {/if}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody use:rowDrag={{ enabled: reorder && !inert, tree: tree !== null, onmove: onreorder }}>
      {#each shownRows as i (i)}
        {@const row = rows[i]}
        <!-- A right click chooses the row first, so a context menu's handler finds it. -->
        <tr data-row={i} class:chosen={selected === i} onclick={() => choose(i)} oncontextmenu={() => choose(i)} ondblclick={() => !design && onopen(i)}>
          {#each columns as column, c (column.field)}
            {@const cell = row[column.field]}
            {#if !column.edit}
              <td data-field={column.field} class:numeric={column.kind === "number"}>
                {#if tree && c === 0}
                  {@const at = depths.get(i)}
                  <span class="indent" style:width="{(at?.depth ?? 0) * 16}px"></span>
                  {#if at?.children}
                    {@const isOpen = openIds.has(idOf(row))}
                    <button
                      type="button"
                      class="toggle"
                      aria-expanded={isOpen}
                      aria-label={isOpen ? tree.close : tree.open}
                      title={isOpen ? tree.close : tree.open}
                      onclick={(e) => {
                        e.stopPropagation();
                        toggle(i);
                      }}
                      ondblclick={(e) => e.stopPropagation()}
                      ><svg viewBox="0 0 24 24" aria-hidden="true"><path d={isOpen ? "M 6 9 l 6 6 l 6 -6" : "M 9 6 l 6 6 l -6 6"} /></svg></button
                    >
                  {:else}
                    <span class="toggle-space"></span>
                  {/if}
                {/if}
                {#if column.kind === "color" && typeof cell === "string" && cell !== ""}<span class="cell-swatch" style:background-color={cell}></span>{/if}{shown(column, cell)}
              </td>
            {:else}
              <td data-field={column.field} class="edit" class:numeric={column.kind === "number"}>
                {#if column.kind === "bool"}
                  <CheckInput switch={column.switch} label={column.label} checked={cell === true} disabled={inert || readonly} onchange={(next) => edit(i, column.field, next)} />
                {:else if column.kind === "choice"}
                  <ChoiceInput label={column.label} value={cell} choices={column.choices ?? []} disabled={inert || readonly} onchange={(next) => edit(i, column.field, next)} />
                {:else if column.kind === "color"}
                  <ColorInput label={column.label} value={typeof cell === "string" ? cell : null} disabled={inert} {readonly} onchange={(next) => edit(i, column.field, next)} />
                {:else if column.kind === "moment"}
                  <DateTimeInput value={cell} label={column.label} disabled={inert} {readonly} onchange={(next) => edit(i, column.field, next)} />
                {:else if column.kind === "date"}
                  <DateInput label={column.label} value={cell} disabled={inert} {readonly} onchange={(next) => edit(i, column.field, next)} />
                {:else if column.kind === "number"}
                  <!-- Text that is no number stays as typed and is marked; the value stays. Enter takes it and does nothing else. -->
                  <NumberInput
                    label={column.label}
                    value={cell}
                    text={column.number?.text ?? ((v) => String(v ?? ""))}
                    read={column.number?.read ?? ((typed) => typed)}
                    disabled={inert}
                    {readonly}
                    onchange={(next) => edit(i, column.field, next)}
                    onsubmit={() => true}
                  />
                {:else}
                  <TextInput label={column.label} value={cell} disabled={inert} {readonly} onchange={(next) => edit(i, column.field, next)} onsubmit={() => true} />
                {/if}
              </td>
            {/if}
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
  {#if columns.length === 0}<div class="empty">Choose columns in the properties</div>{/if}
</div>
{#if paged}
  <!-- The page bar: to the first, previous, next and last page, and which rows show. -->
  <div class="pager">
    <button type="button" title={pageWords.first} aria-label={pageWords.first} disabled={design || paged.page === 1} onclick={() => turn(1)}
      ><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M 17 18 l -6 -6 l 6 -6 M 7 6 v 12" /></svg></button
    >
    <button type="button" title={pageWords.previous} aria-label={pageWords.previous} disabled={design || paged.page === 1} onclick={() => turn(paged.page - 1)}
      ><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M 15 18 l -6 -6 l 6 -6" /></svg></button
    >
    <span class="shown">{paged.text}</span>
    <button type="button" title={pageWords.next} aria-label={pageWords.next} disabled={design || paged.page === paged.pages} onclick={() => turn(paged.page + 1)}
      ><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M 9 18 l 6 -6 l -6 -6" /></svg></button
    >
    <button type="button" title={pageWords.last} aria-label={pageWords.last} disabled={design || paged.page === paged.pages} onclick={() => turn(paged.pages)}
      ><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M 7 18 l 6 -6 l -6 -6 M 17 6 v 12" /></svg></button
    >
  </div>
{/if}

<style>
  table.no-header thead {
    display: none;
  }

  table.tree td:first-child {
    white-space: nowrap;
  }

  .indent,
  .toggle-space {
    display: inline-block;
  }

  .toggle-space {
    width: 20px;
  }

  .toggle {
    display: inline-flex;
    width: 20px;
    padding: 2px;
    border: 0;
    background: transparent;
    cursor: pointer;
    vertical-align: middle;
  }

  .toggle svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }

  .table-wrap.paged {
    border-bottom-left-radius: 0;
    border-bottom-right-radius: 0;
  }

  .pager {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 2px;
    padding: 2px 4px;
    border: var(--border-width) solid var(--border-strong);
    border-top: 0;
    border-radius: 0 0 var(--radius-panel) var(--radius-panel);
    font-size: var(--font-size);
  }

  .pager button {
    display: inline-flex;
    padding: 2px;
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .pager button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .pager svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }

  .pager .shown {
    padding: 0 8px;
    font-variant-numeric: tabular-nums;
  }

  .cell-swatch {
    display: inline-block;
    width: 12px;
    height: 12px;
    margin-right: 6px;
    vertical-align: -1px;
    border: 1px solid rgb(0 0 0 / 0.2);
    border-radius: 2px;
  }

  .table-wrap {
    flex: 1;
    overflow: auto;
    border: var(--border-width) solid var(--border-strong);
    border-radius: var(--radius-panel);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size);
  }

  th {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--header-bg);
    color: var(--header-text);
    font-weight: var(--header-weight, 600);
    text-align: left;
    padding: var(--padding) calc(var(--padding) * 2);
    border-bottom: var(--border-width) solid var(--border-strong);
  }

  td {
    padding: calc(var(--padding) - 1px) calc(var(--padding) * 2);
    border-bottom: var(--border-width) solid var(--border);
    white-space: nowrap;
  }

  /* With widths, columns keep them and what does not fit is cut off. */
  table.fixed {
    table-layout: fixed;
  }

  table.fixed th,
  table.fixed td {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  th.sortable {
    cursor: pointer;
  }

  th.sortable:hover .label {
    text-decoration: underline;
  }

  .arrow {
    margin-left: 4px;
    font-size: 0.75em;
  }

  /* The edge of a header, dragged to resize; it answers even where the rest is inert, as in the designer. */
  .grip {
    position: absolute;
    top: 0;
    right: 0;
    width: 6px;
    height: 100%;
    cursor: col-resize;
    pointer-events: auto;
  }

  .grip:hover {
    background: var(--accent);
    opacity: 0.5;
  }

  tr.chosen td {
    background: var(--selection);
  }

  /* Where a dragged row lands (DESIGN 5.30): a line before or after a row, or the row it goes under. */
  tbody:global(.dragging-row) {
    cursor: grabbing;
  }

  tr:global([data-drop="before"]) td {
    box-shadow: inset 0 2px 0 var(--accent);
  }

  tr:global([data-drop="after"]) td {
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  tr:global([data-drop="into"]) td {
    background: var(--accent-soft);
  }

  .numeric {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  /* An edited cell holds its input flush in the cell, as a spreadsheet does. */
  td.edit {
    padding: 0 calc(var(--padding) * 2 - 4px);
  }

  td.edit :global(input:not([type="checkbox"])),
  td.edit :global(select) {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
    font: inherit;
    padding: 1px 4px;
    border: 1px solid transparent;
    background-color: transparent;
    color: inherit;
  }

  td.edit :global(input:not([type="checkbox"]):focus),
  td.edit :global(input:not([type="checkbox"]):hover),
  td.edit :global(select:focus),
  td.edit :global(select:hover) {
    border-color: var(--border-strong);
    background-color: var(--input-bg);
  }

  td.edit.numeric :global(input) {
    text-align: right;
  }

  td.edit :global(.moment input.time) {
    width: 64px;
  }

  td.edit :global(input.bad) {
    border-color: var(--error);
  }

  td.edit :global(.check),
  td.edit :global(.toggle) {
    margin: 0 4px;
    vertical-align: middle;
  }

  .empty {
    padding: 8px;
    color: var(--text-muted);
    font-size: var(--font-size-small);
  }
</style>
