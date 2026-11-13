<script lang="ts">
  // The data dictionary for one entity: a row per field with its name, type
  // and modifiers. Every change is one edit, checked by the Rust backend.

  import { resizableColumns } from "@clarity/ui/screen/columnWidths.ts";
  import { tr } from "@clarity/ui/screen/model.ts";
  import TextField from "./TextField.svelte";
  import RenameField from "./RenameField.svelte";
  import type { Edit } from "@clarity/ui/generated/Edit.ts";
  import type { Entity } from "@clarity/ui/generated/Entity.ts";
  import type { Field } from "@clarity/ui/generated/Field.ts";
  import type { FieldSpec } from "@clarity/ui/generated/FieldSpec.ts";
  import type { Literal } from "@clarity/ui/generated/Literal.ts";
  import type { Modifiers } from "@clarity/ui/generated/Modifiers.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import { invoke } from "@tauri-apps/api/core";
  import { fieldTypes } from "./fieldTypes.svelte.ts";
  import { ofChoices, typeParts, typeText, withKind, type FieldTypeParts } from "./fieldType.ts";

  let {
    preview,
    entity,
    module,
    onedit,
    ondelete = () => undefined,
    onrename = () => undefined,
    onapplied,
    stores,
    settings = false,
  }: {
    preview: Preview;
    /** Takes the project after a text's value or key changed in the language files, or a field was renamed everywhere. */
    onapplied: (preview: Preview) => void;
    /** The entity; for the app's settings, its settings block, none while there is none. */
    entity: Entity | null;
    module: string;
    onedit: (edit: Edit) => void;
    ondelete?: () => void;
    onrename?: (newName: string) => void;
    /** The file stores of the project settings, by name. */
    stores: string[];
    /** The app's settings block: the types and modifiers a setting takes, Clarity's own words left to the settings page. */
    settings?: boolean;
  } = $props();

  /** The fields shown: a settings block's own bare words (`language`) are ticked on the settings page. */
  const fields = $derived((entity?.fields ?? []).filter((f) => !settings || !OWN_WORDS.includes(f.name.name)));
  const OWN_WORDS = ["language", "theme", "agentReadsWhileClosed", "agentChangesWhileClosed"];

  /** The module file the entity is declared in. */
  const declaredIn = $derived(entity ? preview.project.files.find((f) => f.entities.some((e) => e.name.name === entity.name.name)) : undefined);
  /** In a module of a data source (`source shop`), the other entities can be held inside as records (DESIGN 5.25). */
  const held = $derived(
    declaredIn?.source && entity ? declaredIn.entities.filter((e) => e.name.name !== entity.name.name && !e.settings).map((e) => e.name.name) : [],
  );
  const names = $derived({
    enums: preview.project.files.flatMap((f) => f.enums.map((e) => e.name.name)),
    records: [...preview.project.files.flatMap((f) => f.records.map((r) => r.name.name)), ...held],
    entities: preview.project.files.flatMap((f) => f.entities.filter((e) => !e.settings).map((e) => e.name.name)),
    simple: fieldTypes.simple,
  });
  /** The kinds a field can be, as the tools offer them; a named kind only once there is something to name. */
  const kinds = $derived(
    settings
      ? fieldTypes.settings.filter((k) => !fieldTypes.named.includes(k) || ofChoices(k, names).length > 0)
      : [...fieldTypes.simple, ...fieldTypes.named.filter((k) => ofChoices(k, names).length > 0)],
  );
  /** Where a field's columns are named (`column "CustName"`): an entity of a data source's module, or with a table of its own. */
  const mapped = $derived(Boolean(declaredIn?.source) || entity?.table != null);

  /** Which fields show their details. */
  let open = $state<Record<number, boolean>>({});
  let error = $state<string | null>(null);

  /** Format and Store are columns only when a field of this entity can have one. */
  const hasFormat = $derived(fields.some((f) => (preview.formats[f.type.kind] ?? []).length > 0 || f.modifiers.format !== null));
  const hasStore = $derived(!settings && fields.some((f) => f.type.kind === "image"));

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

  /** The field's type with new parts; nothing is sent while the type misses something. */
  function setType(field: Field, change: Partial<FieldTypeParts>) {
    const text = typeText({ ...typeParts(field.typeText, names), ...change });
    if (text !== null && text !== field.typeText) update(field, { typeText: text });
  }

  function setKind(field: Field, kind: string) {
    const text = typeText(withKind(kind, ofChoices(kind, names)));
    if (text !== null) update(field, { typeText: text });
  }

  /** A whole number typed in a box; null when empty or not one. */
  function whole(value: string): number | null {
    const n = Number.parseInt(value, 10);
    return Number.isFinite(n) && n >= 0 ? n : null;
  }

  /** Renames a field everywhere it is named, as asked in its name cell; a setting is renamed in its line, since each person keeps it under its name. */
  async function rename(field: Field, input: HTMLInputElement) {
    const newName = input.value.trim();
    if (!newName || newName === field.name.name) return;
    if (settings || !entity) return update(field, { name: newName });
    try {
      error = null;
      onapplied(await invoke<Preview>("rename_field", { entity: entity.name.name, name: field.name.name, newName }));
    } catch (err) {
      error = String(err);
      input.value = field.name.name;
    }
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

  /** Whether a full-text index can hold the field's words: text and lists of text. */
  function searchable(type: Field["type"]): boolean {
    return type.kind === "text" || type.kind === "email" || type.kind === "url" || (type.kind === "values" && type.item.kind === "text");
  }

  let newName = $state("");
  let newType = $state("text(80)");
  /** The kinds the new field's list offers, with a text's usual length. */
  const newTypes = $derived(kinds.flatMap((k) => (fieldTypes.named.includes(k) ? ofChoices(k, names).map((n) => typeText({ kind: k, of: n, length: null, precision: null, scale: null }) ?? k) : [k === "text" ? "text(80)" : k === "decimal" ? "decimal(10, 2)" : k])));

  function addField(e: SubmitEvent) {
    e.preventDefault();
    const name = newName.trim();
    if (!name) return;
    const modifiers: Modifiers = {
      required: false,
      unique: false,
      owned: false,
      cascade: false,
      computed: false,
      search: false,
      hashed: false,
      min: null,
      max: null,
      pattern: null,
      default: null,
      label: null,
      help: null,
      format: null,
      column: null,
      store: null,
    };
    const typeText = newType.trim() || "text";
    onedit(settings || !entity ? { op: "addSetting", spec: { name, typeText, modifiers } } : { op: "addField", entity: entity.span.start, spec: { name, typeText, modifiers } });
    newName = "";
  }
</script>

<datalist id="field-types">
  {#each newTypes as t (t)}<option value={t}></option>{/each}
</datalist>

<div class="editor">
  {#if !settings && entity}
    <header>
      <div>
        <RenameField name={entity.name.name} title="Rename the entity everywhere it is used" {onrename} />
        <span class="muted">entity in module {module}, {entity.fields.length} fields</span>
      </div>
      <button class="danger" onclick={ondelete}>Delete entity</button>
    </header>
  {/if}
  {#if error}<p class="problem" role="alert">{error}</p>{/if}

  <div class="table-wrap">
    <table use:resizableColumns={settings ? "clarity.columns.appSettings" : "clarity.columns.entityFields"}>
      <thead>
        <tr>
          <th></th>
          <th title={settings ? "Renamed in its line: each person keeps a setting under its name" : "Renaming a field renames it everywhere it is named"}>{settings ? "Setting" : "Field"}</th>
          <th>Type</th>
          {#if !settings}
            <th title="A value is required">Req.</th>
            <th title="No two records have the same value">Unique</th>
            <th title="In the full-text index that 'where a matches x' looks in: text and lists of text">Search</th>
            <th title="Kept as a hash: code assigns text to it and never reads it back, as a sign-in strategy's secret is">Hashed</th>
          {/if}
          <th>Label</th>
          <th>Default</th>
          {#if hasFormat}<th title="How values are shown, in every locale">Format</th>{/if}
          <th>Help</th>
          {#if hasStore}<th title="Pictures kept as files in a file store of the project settings, not in the database">Store</th>{/if}
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each fields as field, i (field.span.start)}
          {@const m = field.modifiers}
          {@const formats = preview.formats[field.type.kind] ?? []}
          {@const parts = typeParts(field.typeText, names)}
          {@const choices = ofChoices(parts.kind, names)}
          <tr>
            <td class="toggle">
              <button
                class="plain"
                title={open[field.span.start] ? "Hide the field's details" : "Show the field's details: min, max and more"}
                aria-expanded={open[field.span.start] ?? false}
                onclick={() => (open[field.span.start] = !open[field.span.start])}>{open[field.span.start] ? "\u25BE" : "\u25B8"}</button
              >
            </td>
            <td><input class="mono name" value={field.name.name} spellcheck="false" onchange={(e) => rename(field, e.currentTarget)} /></td>
            <td>
              <div class="type">
                <select class="mono" value={parts.kind} aria-label={`Type of ${field.name.name}`} onchange={(e) => setKind(field, e.currentTarget.value)}>
                  {#each kinds as k (k)}<option value={k}>{k}</option>{/each}
                  {#if !kinds.includes(parts.kind)}<option value={parts.kind}>{parts.kind}</option>{/if}
                </select>
                {#if fieldTypes.named.includes(parts.kind)}
                  <select class="mono" value={parts.of} aria-label={`What ${field.name.name} holds`} onchange={(e) => setType(field, { of: e.currentTarget.value })}>
                    {#each choices as c (c)}<option value={c}>{c}</option>{/each}
                    {#if !choices.includes(parts.of)}<option value={parts.of}>{parts.of}</option>{/if}
                  </select>
                {:else if parts.kind === "text"}
                  <input
                    class="size"
                    type="number"
                    min="1"
                    value={parts.length ?? ""}
                    placeholder="any"
                    title="The most characters; empty for any length"
                    onchange={(e) => setType(field, { length: whole(e.currentTarget.value) || null })}
                  />
                {:else if parts.kind === "decimal"}
                  <input class="size" type="number" min="1" value={parts.precision ?? ""} title="Precision: the digits in all" onchange={(e) => setType(field, { precision: whole(e.currentTarget.value) })} />
                  <input class="size" type="number" min="0" value={parts.scale ?? ""} title="Scale: the digits after the point" onchange={(e) => setType(field, { scale: whole(e.currentTarget.value) })} />
                {/if}
              </div>
            </td>
            {#if !settings}
              <td class="center">
                <input type="checkbox" checked={m.required} onchange={(e) => update(field, { modifiers: { required: e.currentTarget.checked } })} />
              </td>
              <td class="center">
                <input type="checkbox" checked={m.unique} onchange={(e) => update(field, { modifiers: { unique: e.currentTarget.checked } })} />
              </td>
              <td class="center">
                {#if searchable(field.type) || m.search}
                  <input type="checkbox" checked={m.search} onchange={(e) => update(field, { modifiers: { search: e.currentTarget.checked } })} />
                {/if}
              </td>
              <td class="center">
                {#if field.typeText.startsWith("text") || m.hashed}
                  <input type="checkbox" checked={m.hashed} onchange={(e) => update(field, { modifiers: { hashed: e.currentTarget.checked } })} />
                {/if}
              </td>
            {/if}
            <td><TextField {preview} {onapplied} text={m.label} placeholder={tr(preview, field.label)} onwrite={(text) => update(field, { modifiers: { label: text } })} /></td>
            <td>
              <input
                class="mono"
                value={literalText(m.default)}
                onchange={(e) => update(field, { modifiers: { default: literalOf(e.currentTarget.value) } })}
              />
            </td>
            {#if hasFormat}<td>
              {#if formats.length > 0}
                <select value={m.format ?? ""} onchange={(e) => update(field, { modifiers: { format: e.currentTarget.value || null } })}>
                  <option value="">default</option>
                  {#each formats as f (f.name)}<option value={f.name} title={f.shows}>{f.name}</option>{/each}
                </select>
              {/if}
            </td>{/if}
            <td><TextField {preview} {onapplied} text={m.help} onwrite={(text) => update(field, { modifiers: { help: text } })} /></td>
            {#if hasStore}<td>
              {#if field.type.kind === "image"}
                <select value={m.store ?? ""} onchange={(e) => update(field, { modifiers: { store: e.currentTarget.value || null } })}>
                  <option value="">database</option>
                  {#each stores as store (store)}<option value={store}>{store}</option>{/each}
                  {#if m.store && !stores.includes(m.store)}<option value={m.store}>{m.store} (not in the settings)</option>{/if}
                </select>
              {/if}
            </td>{/if}
            <td class="actions">
              <button title="Move up" disabled={i === 0} onclick={() => onedit({ op: "moveSibling", item: field.span.start, delta: -1 })}>&uarr;</button>
              <button
                title="Move down"
                disabled={i === fields.length - 1}
                onclick={() => onedit({ op: "moveSibling", item: field.span.start, delta: 1 })}>&darr;</button
              >
              <button class="danger" title={settings ? "Remove the setting" : "Remove the field"} onclick={() => onedit({ op: "remove", item: field.span.start })}>&times;</button>
            </td>
          </tr>
          {#if open[field.span.start]}
            <tr class="details">
              <td></td>
              <td colspan="20">
                <div class="details-grid">
                  <label>Min <input class="mono" value={literalText(m.min)} placeholder="none" onchange={(e) => update(field, { modifiers: { min: literalOf(e.currentTarget.value) } })} /></label>
                  <label>Max <input class="mono" value={literalText(m.max)} placeholder="none" onchange={(e) => update(field, { modifiers: { max: literalOf(e.currentTarget.value) } })} /></label>
                  {#if !settings}
                    <label title="A regular expression a text value matches"
                      >Pattern <input class="mono" value={m.pattern ?? ""} placeholder="none" spellcheck="false" onchange={(e) => update(field, { modifiers: { pattern: textOf(e) } })} /></label
                    >
                    {#if mapped || m.column}
                      <label title="The existing column of the table the field is stored in"
                        >Column <input class="mono" value={m.column ?? ""} placeholder={field.name.name} spellcheck="false" onchange={(e) => update(field, { modifiers: { column: textOf(e) } })} /></label
                      >
                    {/if}
                    <label class="check" title="Worked out, not entered"
                      ><input type="checkbox" checked={m.computed} onchange={(e) => update(field, { modifiers: { computed: e.currentTarget.checked } })} /> Computed</label
                    >
                    {#if parts.kind === "list" || parts.kind === "ref" || m.owned}
                      <label class="check" title="The records it holds belong to this one"
                        ><input type="checkbox" checked={m.owned} onchange={(e) => update(field, { modifiers: { owned: e.currentTarget.checked } })} /> Owned</label
                      >
                    {/if}
                    {#if parts.kind === "ref" || m.cascade}
                      <label class="check" title="Deleting the record it refers to deletes this one first"
                        ><input type="checkbox" checked={m.cascade} onchange={(e) => update(field, { modifiers: { cascade: e.currentTarget.checked } })} /> Cascade</label
                      >
                    {/if}
                  {/if}
                </div>
              </td>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
  </div>
  {#if entity?.keptVersion}
    <!-- The version is Clarity's, never a field (docs/APP-CAPABILITIES-PLAN.md step 5). -->
    <p class="muted">Clarity keeps a version of each record in <code>{entity.keptVersion}</code>: a save of a record someone else saved since it was read is refused, and <code>save ... anyway</code> writes regardless.</p>
  {/if}

  <form class="add" onsubmit={addField}>
    <input class="mono" bind:value={newName} placeholder={settings ? "newSetting" : "newField"} spellcheck="false" />
    <input class="mono" list="field-types" bind:value={newType} aria-label="Type" />
    <button type="submit" disabled={!newName.trim()}>{settings ? "Add setting" : "Add field"}</button>
  </form>
  <p class="muted">
    {#if settings}
      Each person sets these for themselves; every window has them as <code>settings</code>. A setting is a plain value or a reference.
    {:else}
      Windows bind to these fields; labels, input types and validation come from here. A change that would break a
      window is refused, with the reason. A renamed field is renamed everywhere it is named.
    {/if}
  </p>
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

  p.muted {
    margin: 0;
  }

  /* Scrolls sideways only when the pane is narrower than the fields can be. */
  .table-wrap {
    overflow-x: auto;
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

  /* Readable while the widths are automatic; a resized column decides for itself. */
  table:not(:global(.sized)) td input.name,
  table:not(:global(.sized)) td .type {
    min-width: 9rem;
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

  .problem {
    color: var(--error);
    margin: 0;
  }

  .type {
    display: flex;
    gap: 4px;
  }

  .type select {
    min-width: 0;
  }

  td input.size {
    width: 4.5rem;
    min-width: 0;
  }

  .toggle button.plain {
    border: none;
    background: none;
    padding: 0 4px;
    cursor: pointer;
    color: var(--text-muted);
  }

  .details-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    align-items: center;
    padding: 4px 0;
    font-size: 12px;
  }

  .details-grid label {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .details-grid input:not([type="checkbox"]) {
    width: 9rem;
  }
</style>
