<script lang="ts">
  // An enum of the data dictionary: its variants and their labels.

  import { resizableColumns } from "./columnWidths";
  import TextField from "./TextField.svelte";
  import type { Preview } from "./generated/Preview";
  import RenameField from "./RenameField.svelte";
  import type { Edit } from "./generated/Edit";
  import type { EnumDecl } from "./generated/EnumDecl";

  let {
    preview,
    decl,
    module,
    onedit,
    ondelete,
    onrename,
    onapplied,
  }: {
    preview: Preview;
    /** Takes the project after a text's value or key changed in the language files. */
    onapplied: (preview: Preview) => void;
    decl: EnumDecl;
    module: string;
    onedit: (edit: Edit) => void;
    ondelete: () => void;
    onrename: (newName: string) => void;
  } = $props();

  // A variant's line starts with its name, so the offset of the name
  // addresses the variant in edits.
  let newName = $state("");

  function add(e: SubmitEvent) {
    e.preventDefault();
    const name = newName.trim();
    if (!name) return;
    onedit({ op: "addVariant", owner: decl.span.start, name, label: null });
    newName = "";
  }
</script>

<div class="editor">
  <header>
    <div>
      <RenameField name={decl.name.name} title="Rename the enum everywhere it is used" {onrename} />
      <span class="muted">enum in module {module}, {decl.variants.length} variants</span>
    </div>
    <button class="danger" onclick={ondelete}>Delete enum</button>
  </header>

  <div class="table-wrap">
    <table use:resizableColumns={"clarity.columns.enumValues"}>
      <thead>
        <tr><th>Variant</th><th>Label</th><th></th></tr>
      </thead>
      <tbody>
        {#each decl.variants as v, i (v.name.span.start)}
          {@const item = v.name.span.start}
          <tr>
            <td>
              <input
                class="mono"
                value={v.name.name}
                onchange={(e) => onedit({ op: "updateVariant", item, name: e.currentTarget.value.trim(), label: v.label })}
              />
            </td>
            <td>
              <TextField {preview} {onapplied} text={v.label} onwrite={(text) => onedit({ op: "updateVariant", item, name: v.name.name, label: text })} />
            </td>
            <td class="actions">
              <button title="Move up" disabled={i === 0} onclick={() => onedit({ op: "moveSibling", item, delta: -1 })}>&uarr;</button>
              <button
                title="Move down"
                disabled={i === decl.variants.length - 1}
                onclick={() => onedit({ op: "moveSibling", item, delta: 1 })}>&darr;</button
              >
              <button class="danger" title="Remove the variant" onclick={() => onedit({ op: "remove", item })}>&times;</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <form class="add" onsubmit={add}>
    <input class="mono" bind:value={newName} placeholder="NewVariant" spellcheck="false" />
    <button type="submit" disabled={!newName.trim()}>Add variant</button>
  </form>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }


  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }

  .table-wrap {
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--panel);
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th {
    text-align: left;
    font-size: 12px;
    color: var(--text-muted);
    padding: 6px 8px;
    background: var(--panel-alt);
    border-bottom: 1px solid var(--border-strong);
  }

  td {
    padding: 3px 4px;
    border-bottom: 1px solid var(--border);
  }

  td input {
    width: 100%;
  }

  .mono {
    font-family: var(--mono);
    font-size: 12px;
  }

  .actions {
    white-space: nowrap;
    width: 1%;
  }

  .actions button {
    padding: 1px 7px;
  }

  .danger {
    color: var(--error);
  }

  .add {
    display: flex;
    gap: 8px;
  }
</style>
