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
  };

  export type Row = Record<string, JsonValue>;
</script>

<script lang="ts">
  // Clarity's table part on the web. It owns how a table behaves, the same as
  // on Avalonia (conformance/tables.json): which input a cell holds, when a
  // row counts as chosen and when it counts as edited, and how text that is
  // no value is marked. The designer and running windows only describe the
  // columns.
  import CheckInput from "./CheckInput.svelte";
  import ChoiceInput from "./ChoiceInput.svelte";
  import DateInput from "./DateInput.svelte";
  import DateTimeInput from "./DateTimeInput.svelte";
  import NumberInput from "./NumberInput.svelte";
  import TextInput from "./TextInput.svelte";

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
