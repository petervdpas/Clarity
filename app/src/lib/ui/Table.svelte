<script lang="ts" module>
  import type { JsonValue } from "../generated/serde_json/JsonValue";

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
    choices?: { value: JsonValue; label: string }[];
    /** A value as the column shows it, when it is not edited. */
    shown?: (value: JsonValue | undefined) => string;
    /** How a number cell shows its value and reads what is typed (null: no number). */
    number?: { text: (value: JsonValue | undefined) => string; read: (text: string) => JsonValue | null };
  };

  export type Row = Record<string, JsonValue>;
</script>

<script lang="ts">
  // Clarity's table part on the web. It owns how a table behaves, the same as
  // on Avalonia (conformance/tables.json): which input a cell holds, when a
  // row counts as chosen and when it counts as edited, and how text that is
  // no value is marked. The designer and running windows only describe the
  // columns.
  import { untrack } from "svelte";
  import DateTimeInput from "./DateTimeInput.svelte";

  let {
    columns,
    rows,
    selected = null,
    disabled = false,
    readonly = false,
    design = false,
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
    /** Another row was chosen: Clarity's `select`. */
    onselect: (row: number) => void;
    /** A row was opened (double-click): Clarity's `open`. */
    onopen: (row: number) => void;
    /** The person edited the row, which is chosen first: Clarity's `change`. */
    onedit: (row: number) => void;
  } = $props();

  const inert = $derived(design || disabled);

  /** Cells holding text that is no value, by row and field, with what was typed. */
  let bad = $state<Record<string, string>>({});
  // Rows read again are other records: their cells start over.
  $effect(() => {
    void rows;
    untrack(() => (bad = {}));
  });
  const key = (row: number, field: string) => `${row} ${field}`;

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

  function typedNumber(row: number, column: TableColumn, typed: string) {
    const read = column.number ? column.number.read(typed) : typed;
    if (read === null) {
      bad[key(row, column.field)] = typed;
      return;
    }
    delete bad[key(row, column.field)];
    edit(row, column.field, read);
  }

  /** Enter takes a typed cell, as leaving it does, and does nothing else. */
  function enter(e: KeyboardEvent, take: () => void) {
    if (e.key !== "Enter") return;
    take();
    e.preventDefault();
  }

  const shown = (column: TableColumn, value: JsonValue | undefined) => (column.shown ? column.shown(value) : value === null || value === undefined ? "" : String(value));
  const choiceIndex = (column: TableColumn, value: JsonValue | undefined) => (column.choices ?? []).findIndex((c) => c.value === (value ?? null));
</script>

<div class="table-wrap">
  <table>
    <thead>
      <tr>
        {#each columns as column (column.field)}<th>{column.label}</th>{/each}
      </tr>
    </thead>
    <tbody>
      {#each rows as row, i (i)}
        <tr class:chosen={selected === i} onclick={() => choose(i)} ondblclick={() => !design && onopen(i)}>
          {#each columns as column (column.field)}
            {@const cell = row[column.field]}
            {#if !column.edit}
              <td data-field={column.field} class:numeric={column.kind === "number"}>{shown(column, cell)}</td>
            {:else}
              <td data-field={column.field} class="edit" class:numeric={column.kind === "number"}>
                {#if column.kind === "bool" && column.switch}
                  <label class="toggle">
                    <input type="checkbox" role="switch" aria-label={column.label} checked={cell === true} disabled={inert || readonly}
                      onchange={(e) => edit(i, column.field, e.currentTarget.checked)} />
                    <span class="track" aria-hidden="true"></span>
                  </label>
                {:else if column.kind === "bool"}
                  <input type="checkbox" aria-label={column.label} checked={cell === true} disabled={inert || readonly}
                    onchange={(e) => edit(i, column.field, e.currentTarget.checked)} />
                {:else if column.kind === "choice"}
                  <select aria-label={column.label} value={String(choiceIndex(column, cell))} disabled={inert || readonly}
                    onchange={(e) => edit(i, column.field, column.choices?.[Number(e.currentTarget.value)]?.value ?? null)}>
                    {#each column.choices ?? [] as choice, c (c)}
                      <option value={String(c)}>{choice.label}</option>
                    {/each}
                  </select>
                {:else if column.kind === "moment"}
                  <DateTimeInput value={cell} label={column.label} disabled={inert} {readonly} onchange={(next) => edit(i, column.field, next)} />
                {:else if column.kind === "date"}
                  <input type="date" aria-label={column.label} value={cell ?? ""} disabled={inert} {readonly}
                    onchange={(e) => edit(i, column.field, e.currentTarget.value === "" ? null : e.currentTarget.value)} />
                {:else if column.kind === "number"}
                  {@const at = key(i, column.field)}
                  <!-- Text that is no number stays as typed and is marked; the value stays. -->
                  <input aria-label={column.label} inputmode="decimal" disabled={inert} {readonly} class:bad={at in bad} aria-invalid={at in bad}
                    value={bad[at] ?? (column.number ? column.number.text(cell) : String(cell ?? ""))}
                    onchange={(e) => typedNumber(i, column, e.currentTarget.value)}
                    onkeydown={(e) => enter(e, () => typedNumber(i, column, e.currentTarget.value))} />
                {:else}
                  <input aria-label={column.label} value={cell ?? ""} disabled={inert} {readonly}
                    onchange={(e) => edit(i, column.field, e.currentTarget.value)}
                    onkeydown={(e) => enter(e, () => edit(i, column.field, e.currentTarget.value))} />
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

  td.edit input:not([type="checkbox"]),
  td.edit :global(.moment input),
  td.edit select {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
    font: inherit;
    padding: 1px 4px;
    border: 1px solid transparent;
    background-color: transparent;
    color: inherit;
  }

  td.edit input:not([type="checkbox"]):focus,
  td.edit input:not([type="checkbox"]):hover,
  td.edit select:focus,
  td.edit select:hover,
  td.edit :global(.moment input:focus),
  td.edit :global(.moment input:hover) {
    border-color: var(--border-strong);
    background-color: var(--input-bg);
  }

  td.edit.numeric input {
    text-align: right;
  }

  td.edit :global(.moment input.time) {
    width: 64px;
  }

  td.edit input.bad,
  td.edit :global(.moment input.bad) {
    border-color: var(--error);
  }

  td.edit > input[type="checkbox"] {
    margin: 0 4px;
    vertical-align: middle;
  }

  /* `as switch`: the switch of a form, without its caption. */
  .toggle {
    position: relative;
    display: inline-flex;
    align-items: center;
    vertical-align: middle;
  }

  .toggle input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }

  .track {
    flex: none;
    position: relative;
    width: 32px;
    height: 18px;
    border-radius: 9px;
    background: var(--border-strong);
    transition: background 0.15s;
  }

  .track::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--panel);
    transition: transform 0.15s;
  }

  .toggle input:checked + .track {
    background: var(--accent);
  }

  .toggle input:checked + .track::after {
    transform: translateX(14px);
  }

  .toggle input:focus-visible + .track {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .empty {
    padding: 8px;
    color: var(--text-muted);
    font-size: var(--font-size-small);
  }
</style>
