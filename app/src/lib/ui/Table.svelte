<script lang="ts" module>
  import type { JsonValue } from "../generated/serde_json/JsonValue";
  import type { Choice } from "./ChoiceInput.svelte";

  /** What a cell holds: the kind of value of its column. */
  export type CellKind = "text" | "number" | "bool" | "choice" | "date" | "moment";

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
  import DateInput from "./DateInput.svelte";
  import DateTimeInput from "./DateTimeInput.svelte";
  import NumberInput from "./NumberInput.svelte";
  import TextInput from "./TextInput.svelte";
  import { readWidths, resized } from "../columnWidths";
  import { nextOrder, shownOrder, type Order } from "./tableView";

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
    ondesignwidth,
    onselect,
    onopen,
    onedit,
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
    /** In the designer: a header edge was dragged; 0 takes the designed width away. */
    ondesignwidth?: (field: string, width: number) => void;
    /** Another row was chosen: Clarity's `select`. */
    onselect: (row: number) => void;
    /** A row was opened (double-click): Clarity's `open`. */
    onopen: (row: number) => void;
    /** The person edited the row, which is chosen first: Clarity's `change`. */
    onedit: (row: number) => void;
  } = $props();

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
  const shownRows = $derived(shownOrder(rows, columns, sorting));

  function sortBy(field: string) {
    if (!sortable || design) return;
    sorting = nextOrder(sorting, field);
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
    record[field] = next;
    choose(row);
    onedit(row);
  }

  const shown = (column: TableColumn, value: JsonValue | undefined) => (column.shown ? column.shown(value) : value === null || value === undefined ? "" : String(value));
</script>

<div class="table-wrap">
  <table class:fixed style:width={tableWidth}>
    <thead>
      <tr bind:this={head}>
        {#each columns as column (column.field)}
          {@const sortedHere = sorting?.field === column.field}
          <th
            data-field={column.field}
            class:sortable={sortable && !design}
            style:width={widthOf(column) ? `${widthOf(column)}px` : undefined}
            aria-sort={sortedHere ? (sorting?.desc ? "descending" : "ascending") : undefined}
            tabindex={sortable && !design ? 0 : undefined}
            onclick={() => sortBy(column.field)}
            onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), sortBy(column.field))}
          >
            <span class="label">{column.label}</span>
            {#if sortedHere}<span class="arrow" aria-hidden="true">{sorting?.desc ? "▼" : "▲"}</span>{/if}
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
    <tbody>
      {#each shownRows as i (i)}
        {@const row = rows[i]}
        <!-- A right click chooses the row first, so a context menu's handler finds it. -->
        <tr data-row={i} class:chosen={selected === i} onclick={() => choose(i)} oncontextmenu={() => choose(i)} ondblclick={() => !design && onopen(i)}>
          {#each columns as column (column.field)}
            {@const cell = row[column.field]}
            {#if !column.edit}
              <td data-field={column.field} class:numeric={column.kind === "number"}>{shown(column, cell)}</td>
            {:else}
              <td data-field={column.field} class="edit" class:numeric={column.kind === "number"}>
                {#if column.kind === "bool"}
                  <CheckInput switch={column.switch} label={column.label} checked={cell === true} disabled={inert || readonly} onchange={(next) => edit(i, column.field, next)} />
                {:else if column.kind === "choice"}
                  <ChoiceInput label={column.label} value={cell} choices={column.choices ?? []} disabled={inert || readonly} onchange={(next) => edit(i, column.field, next)} />
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

<style>
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
