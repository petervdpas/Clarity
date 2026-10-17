<script lang="ts">
  // The data dictionary for one entity: a row per field with its name, type
  // and modifiers. Every change is one edit, checked by the Rust backend.

  import RenameField from "./RenameField.svelte";
  import type { Edit } from "./generated/Edit";
  import type { Entity } from "./generated/Entity";
  import type { Field } from "./generated/Field";
  import type { FieldSpec } from "./generated/FieldSpec";
  import type { Literal } from "./generated/Literal";
  import type { Modifiers } from "./generated/Modifiers";
  import type { Preview } from "./generated/Preview";

  let {
    preview,
    entity,
    module,
    onedit,
    ondelete,
    onrename,
  }: {
    preview: Preview;
    entity: Entity;
    module: string;
    onedit: (edit: Edit) => void;
    ondelete: () => void;
    onrename: (newName: string) => void;
  } = $props();

  const BASE_TYPES = ["text", "text(80)", "integer", "decimal(10, 2)", "money", "bool", "date", "datetime", "email", "url", "id"];
  const types = $derived([
    ...BASE_TYPES,
    ...preview.project.files.flatMap((f) => f.enums.map((e) => e.name.name)),
    ...preview.project.files.flatMap((f) => f.entities.flatMap((e) => [`ref ${e.name.name}`, `list<${e.name.name}>`])),
  ]);

  function specOf(field: Field): FieldSpec {
    return { name: field.name.name, typeText: field.typeText, modifiers: field.modifiers };
  }

  function update(field: Field, changes: { name?: string; typeText?: string; modifiers?: Partial<Modifiers> }) {
    const spec = specOf(field);
    onedit({
      op: "updateField",
      item: field.span.start,
      spec: {
        name: changes.name ?? spec.name,
        typeText: changes.typeText ?? spec.typeText,
        modifiers: { ...spec.modifiers, ...changes.modifiers },
      },
    });
  }

  /** A typed value as a literal: a number, true or false, a variant name, or text. */
  function literalOf(text: string): Literal | null {
    const value = text.trim();
    if (value === "") return null;
    if (/^-?\d+(\.\d+)?$/.test(value)) return { kind: "number", value };
    if (value === "true" || value === "false") return { kind: "bool", value: value === "true" };
    if (/^[A-Za-z_][A-Za-z0-9_]*$/.test(value)) return { kind: "ident", value };
    return { kind: "str", value: value.replace(/^"(.*)"$/, "$1") };
  }

  function literalText(literal: Literal | null): string {
    if (literal === null) return "";
    return literal.kind === "str" ? `"${literal.value}"` : String(literal.value);
  }

  function textOf(e: Event): string | null {
    const value = (e.currentTarget as HTMLInputElement).value.trim();
    return value === "" ? null : value;
  }

  let newName = $state("");
  let newType = $state("text(80)");

  function addField(e: SubmitEvent) {
    e.preventDefault();
    const name = newName.trim();
    if (!name) return;
    const modifiers: Modifiers = {
      required: false,
      unique: false,
      owned: false,
      computed: false,
      min: null,
      max: null,
      pattern: null,
      default: null,
      label: null,
      help: null,
      format: null,
      column: null,
    };
    onedit({ op: "addField", entity: entity.span.start, spec: { name, typeText: newType.trim() || "text", modifiers } });
    newName = "";
  }
</script>

<datalist id="field-types">
  {#each types as t (t)}<option value={t}></option>{/each}
</datalist>

<div class="editor">
  <header>
    <div>
      <RenameField name={entity.name.name} title="Rename the entity everywhere it is used" {onrename} />
      <span class="muted">entity in module {module}, {entity.fields.length} fields</span>
    </div>
    <button class="danger" onclick={ondelete}>Delete entity</button>
  </header>

  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th>Field</th>
          <th>Type</th>
          <th title="A value is required">Req.</th>
          <th title="No two records have the same value">Unique</th>
          <th>Label</th>
          <th>Default</th>
          <th title="How values are shown, in every locale">Format</th>
          <th>Help</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each entity.fields as field, i (field.span.start)}
          {@const m = field.modifiers}
          {@const formats = preview.formats[field.type.kind] ?? []}
          <tr>
            <td><input class="mono" value={field.name.name} onchange={(e) => update(field, { name: e.currentTarget.value.trim() })} /></td>
            <td>
              <input
                class="mono"
                list="field-types"
                value={field.typeText}
                onchange={(e) => update(field, { typeText: e.currentTarget.value.trim() })}
              />
            </td>
            <td class="center">
              <input type="checkbox" checked={m.required} onchange={(e) => update(field, { modifiers: { required: e.currentTarget.checked } })} />
            </td>
            <td class="center">
              <input type="checkbox" checked={m.unique} onchange={(e) => update(field, { modifiers: { unique: e.currentTarget.checked } })} />
            </td>
            <td><input value={m.label ?? ""} placeholder={field.label} onchange={(e) => update(field, { modifiers: { label: textOf(e) } })} /></td>
            <td>
              <input
                class="mono"
                value={literalText(m.default)}
                onchange={(e) => update(field, { modifiers: { default: literalOf(e.currentTarget.value) } })}
              />
            </td>
            <td>
              {#if formats.length > 0}
                <select value={m.format ?? ""} onchange={(e) => update(field, { modifiers: { format: e.currentTarget.value || null } })}>
                  <option value="">default</option>
                  {#each formats as f (f.name)}<option value={f.name} title={f.shows}>{f.name}</option>{/each}
                </select>
              {/if}
            </td>
            <td><input value={m.help ?? ""} onchange={(e) => update(field, { modifiers: { help: textOf(e) } })} /></td>
            <td class="actions">
              <button title="Move up" disabled={i === 0} onclick={() => onedit({ op: "moveSibling", item: field.span.start, delta: -1 })}>&uarr;</button>
              <button
                title="Move down"
                disabled={i === entity.fields.length - 1}
                onclick={() => onedit({ op: "moveSibling", item: field.span.start, delta: 1 })}>&darr;</button
              >
              <button class="danger" title="Remove the field" onclick={() => onedit({ op: "remove", item: field.span.start })}>&times;</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <form class="add" onsubmit={addField}>
    <input class="mono" bind:value={newName} placeholder="new_field" spellcheck="false" />
    <input class="mono" list="field-types" bind:value={newType} />
    <button type="submit" disabled={!newName.trim()}>Add field</button>
  </form>
  <p class="muted">
    Windows bind to these fields; labels, input types and validation come from here. A change that would break a
    window is refused, with the reason.
  </p>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 1100px;
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

  p.muted {
    margin: 0;
  }

  .table-wrap {
    overflow: auto;
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
    font-weight: 600;
    color: var(--text-muted);
    padding: 6px 8px;
    background: var(--panel-alt);
    border-bottom: 1px solid var(--border-strong);
  }

  td {
    padding: 3px 4px;
    border-bottom: 1px solid var(--border);
  }

  td input:not([type="checkbox"]) {
    width: 100%;
    min-width: 0;
  }

  .mono {
    font-family: var(--mono);
    font-size: 12px;
  }

  .center {
    text-align: center;
  }

  .actions {
    white-space: nowrap;
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
