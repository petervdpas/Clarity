<script lang="ts">
  // A mapping (docs/FORMATS-PLAN.md, step 4) on a page of its own: how one
  // kind of file is laid over an entity, for this case only. The format, the
  // entity, where the records are, a row per field with its path, the records
  // inside with their own rows, the schema it is checked against and the
  // format's options; beside them a sample file laid over the mapping, its
  // values as the fields hold them. Every
  // change is saved at once: the backend writes the mapping back into its
  // module and checks the project; the preview is read by the backend too.

  import { invoke } from "@tauri-apps/api/core";
  import { browse } from "./fileBrowserStore.svelte";
  import { dialogs } from "./dialogStore.svelte";
  import FormatOptions from "./FormatOptions.svelte";
  import type { EditApplied } from "@clarity/ui/generated/EditApplied.ts";
  import type { Entity } from "@clarity/ui/generated/Entity.ts";
  import type { FormatChoice } from "@clarity/ui/generated/FormatChoice.ts";
  import type { MappedFieldForm } from "@clarity/ui/generated/MappedFieldForm.ts";
  import type { MappingEntry } from "@clarity/ui/generated/MappingEntry.ts";
  import type { MappingForm } from "@clarity/ui/generated/MappingForm.ts";
  import type { MappingPreview } from "@clarity/ui/generated/MappingPreview.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";

  let {
    preview,
    name,
    formats,
    onapplied,
    ondelete,
  }: {
    preview: Preview;
    /** The mapping, by name. */
    name: string;
    /** The formats of the catalog, with their options and schema kinds. */
    formats: FormatChoice[];
    onapplied: (preview: Preview) => void;
    /** Deletes the mapping, after asking, unless something uses it. */
    ondelete: (name: string) => void;
  } = $props();

  /** The module that declares the mapping, as the project was loaded. */
  const file = $derived(preview.project.files.find((f) => f.mappings.some((m) => m.name.name === name)));
  const entities = $derived(preview.project.files.flatMap((f) => f.entities).filter((e) => !e.settings));

  let entry = $state<MappingEntry | null>(null);
  /** The form as edited; each change is saved at once. */
  let draft = $state<MappingForm | null>(null);
  let error = $state<string | null>(null);
  let shown = $state<MappingPreview | null>(null);
  let previewError = $state<string | null>(null);
  /** The sample file laid over the mapping, relative to the project's folder. */
  let sample = $state<string | null>(null);

  // The mapping as written, read again whenever its module changes.
  $effect(() => {
    const wanted = name;
    void file?.version;
    invoke<MappingEntry>("read_mapping", { name: wanted })
      .then((found) => {
        entry = found;
        draft = structuredClone(found.form);
        error = null;
      })
      .catch((err) => {
        entry = null;
        draft = null;
        error = String(err);
      });
  });

  // The sample read again when the mapping or the sample changes.
  $effect(() => {
    const from = sample;
    void entry;
    if (!from) return;
    invoke<MappingPreview>("preview_mapping", { name, sample: from })
      .then((read) => {
        shown = read;
        previewError = null;
      })
      .catch((err) => {
        shown = null;
        previewError = String(err);
      });
  });

  const format = $derived(formats.find((f) => f.name === draft?.format));
  const hasAt = $derived(format?.options.some((o) => o.name === "at") ?? false);

  function entityOf(entityName: string): Entity | undefined {
    return entities.find((e) => e.name.name === entityName);
  }

  /** The entity a field holds records of, when it is a list or record of one. */
  function heldBy(field: Entity["fields"][number]): string | null {
    if (field.type.kind === "list") return field.type.item.name;
    if (field.type.kind === "record") return field.type.entity.name;
    return null;
  }

  /** The fields of `inner` that can name the record around, of type `ref outer`. */
  function refsTo(inner: Entity | undefined, outer: string): string[] {
    return inner?.fields.filter((f) => f.type.kind === "ref" && f.type.entity.name === outer).map((f) => f.name.name) ?? [];
  }

  /** The lines in the entity's field order, those with nothing to say left out. */
  function tidy(lines: MappedFieldForm[], entity: Entity | undefined): MappedFieldForm[] {
    const order = (field: string) => {
      const i = entity?.fields.findIndex((f) => f.name.name === field) ?? -1;
      return i < 0 ? Number.MAX_SAFE_INTEGER : i;
    };
    return lines
      .filter((l) => l.at !== "" || l.inside !== null)
      .map((l) => (l.inside ? { ...l, inside: { ...l.inside, fields: tidy(l.inside.fields, entityOf(l.inside.entity)) } } : l))
      .toSorted((a, b) => order(a.field) - order(b.field));
  }

  async function save() {
    if (!draft || !entry || !file) return;
    const form = $state.snapshot(draft) as MappingForm;
    form.fields = tidy(form.fields, entityOf(form.entity));
    try {
      const applied = await invoke<EditApplied>("save_mapping", { path: file.path, version: file.version, item: entry.item, form });
      error = null;
      onapplied(applied.preview);
    } catch (err) {
      error = String(err);
    }
  }

  /** The line of `field` among `lines`, made when it has none. */
  function lineOf(lines: MappedFieldForm[], field: string): MappedFieldForm {
    let line = lines.find((l) => l.field === field);
    if (!line) {
      lines.push({ field, at: "", inside: null });
      line = lines[lines.length - 1];
    }
    return line;
  }

  function setPath(lines: MappedFieldForm[], field: string, at: string) {
    lineOf(lines, field).at = at.trim();
    save();
  }

  function setInside(lines: MappedFieldForm[], field: string, entity: string | null) {
    lineOf(lines, field).inside = entity ? { entity, parent: "", fields: [] } : null;
    save();
  }

  function setFormat(next: string) {
    if (!draft) return;
    const chosen = formats.find((f) => f.name === next);
    draft.format = next;
    draft.options = draft.options.filter((o) => chosen?.options.some((c) => c.name === o.name));
    if (!chosen?.options.some((o) => o.name === "at")) draft.at = "";
    if (!chosen?.schemaKinds.length) draft.check = "";
    save();
  }

  async function setEntity(next: string) {
    if (!draft || next === draft.entity) return;
    if (draft.fields.length > 0) {
      const sure = await dialogs.confirm({ title: "Change the entity", text: `The paths of ${draft.entity}'s fields do not belong to ${next}: they are taken out of the mapping.`, action: "Change" });
      if (!sure) {
        draft.entity = entry?.form.entity ?? draft.entity;
        return;
      }
    }
    draft.entity = next;
    draft.fields = [];
    save();
  }

  function optionValue(option: string): string {
    return draft?.options.find((o) => o.name === option)?.value ?? "";
  }

  function setOption(option: string, value: string) {
    if (!draft) return;
    draft.options = draft.options.filter((o) => o.name !== option);
    if (value !== "") draft.options.push({ name: option, value });
    save();
  }

  async function pickSchema() {
    if (!draft || !format) return;
    const picked = await browse({ purpose: "mapping-schema", title: "Schema the file is checked against", pick: "file", kinds: format.schemaKinds, start: await invoke<string>("project_folder") });
    if (typeof picked !== "string") return;
    draft.check = await invoke<string>("project_path", { picked });
    save();
  }

  async function pickSample() {
    if (!format) return;
    const picked = await browse({ purpose: "mapping-sample", title: "Sample file to lay over the mapping", pick: "file", kinds: [format.name], start: await invoke<string>("project_folder") });
    if (typeof picked === "string") sample = await invoke<string>("project_path", { picked });
  }
</script>

{#snippet rows(lines: MappedFieldForm[], entity: Entity | undefined, parent: string, depth: number)}
  {#each entity?.fields.filter((f) => f.name.name !== parent) ?? [] as field (field.name.name)}
    {@const line = lines.find((l) => l.field === field.name.name)}
    {@const held = heldBy(field)}
    <tr>
      <td style:padding-left="{8 + depth * 18}px"><code>{field.name.name}</code></td>
      <td class="muted">{field.typeText}</td>
      <td>
        <input
          class="mono"
          value={line?.at ?? ""}
          placeholder={line?.inside ? held ?? "" : field.name.name}
          title={line?.inside ? "Where the records inside are; empty: under the field's own name" : "Where the field is in the file: names with slashes between, @name for an attribute; empty: found by its name"}
          onchange={(e) => setPath(lines, field.name.name, e.currentTarget.value)}
        />
      </td>
      <td>
        {#if held && format?.nests}
          <label title="The field holds records of {held} from the same file, with their own paths">
            <input type="checkbox" checked={!!line?.inside} onchange={(e) => setInside(lines, field.name.name, e.currentTarget.checked ? held : null)} />
            Records inside
          </label>
        {/if}
      </td>
    </tr>
    {#if line?.inside}
      {@const inside = line.inside}
      {@const inner = entityOf(inside.entity)}
      {@const refs = refsTo(inner, entity?.name.name ?? "")}
      <tr class="inside">
        <td style:padding-left="{26 + depth * 18}px" colspan="4">
          <span class="muted">{inside.entity} records inside {field.name.name}; the record around is named by</span>
          <select
            value={inside.parent}
            onchange={(e) => {
              inside.parent = e.currentTarget.value;
              save();
            }}
          >
            <option value="">no field</option>
            {#each refs as r (r)}<option value={r}>{r} is ".."</option>{/each}
          </select>
        </td>
      </tr>
      {@render rows(inside.fields, inner, inside.parent, depth + 1)}
    {/if}
  {/each}
{/snippet}

<div class="mapping">
  <header>
    <strong>{name}</strong>
    <span class="muted">mapping in module {file?.module?.name ?? ""}: how a file is laid over an entity, for this case</span>
    <button class="delete" title="Delete the mapping; refused while a data source or code uses it" onclick={() => ondelete(name)}>Delete</button>
  </header>
  {#if error}<p class="problem">{error}</p>{/if}
  {#if draft}
    <div class="sides">
      <div class="form">
        <div class="grid">
          <label for="mapping-format">Format</label>
          <select id="mapping-format" value={draft.format} onchange={(e) => setFormat(e.currentTarget.value)}>
            {#if !format}<option value={draft.format}>{draft.format}</option>{/if}
            {#each formats as f (f.name)}<option value={f.name}>{f.label}</option>{/each}
          </select>

          <label for="mapping-entity">Entity</label>
          <select id="mapping-entity" value={draft.entity} onchange={(e) => setEntity(e.currentTarget.value)}>
            {#if !entityOf(draft.entity)}<option value={draft.entity}>{draft.entity}</option>{/if}
            {#each entities as e (e.name.name)}<option value={e.name.name}>{e.name.name}</option>{/each}
          </select>

          {#if hasAt}
            <label for="mapping-at">Records at</label>
            <input
              id="mapping-at"
              class="mono"
              value={draft.at}
              placeholder="the format's own place"
              title="Where the records are: names with slashes between, like shop/orders/order"
              onchange={(e) => {
                if (draft) draft.at = e.currentTarget.value.trim();
                save();
              }}
            />
          {/if}

          {#if format && format.schemaKinds.length > 0}
            <span>Checked against</span>
            <div class="with-browse">
              <code class:muted={!draft.check}>{draft.check || "no schema"}</code>
              <button onclick={pickSchema} title="A schema ({format.schemaKinds.join(', ')}) the file is checked against while it is read">Schema...</button>
              {#if draft.check}
                <button
                  onclick={() => {
                    if (draft) draft.check = "";
                    save();
                  }}
                  title="Read the file without checking it against a schema">&times;</button
                >
              {/if}
            </div>
          {/if}

          {#if (format?.options ?? []).some((o) => o.name !== "at")}
            <span>Options</span>
            <FormatOptions {format} value={optionValue} onset={setOption} skip={["at"]} />
          {/if}
        </div>

        <table class="fields">
          <thead>
            <tr><th>Field</th><th>Type</th><th>Path in the file</th><th></th></tr>
          </thead>
          <tbody>
            {@render rows(draft.fields, entityOf(draft.entity), "", 0)}
          </tbody>
        </table>
      </div>

      <aside class="preview">
        <div class="with-browse">
          <strong>Preview</strong>
          <code class:muted={!sample}>{sample ?? "no sample file"}</code>
          <button onclick={pickSample} title="Lay a sample file over the mapping, as the program reads it">Sample...</button>
        </div>
        {#if previewError}
          <p class="problem">{previewError}</p>
        {:else if shown?.problem}
          <p class="problem">{shown.problem}</p>
        {:else if shown}
          {#each shown.tables as table, i (i)}
            <h3>{table.name ? `${table.entity} inside ${table.name}` : table.entity} <span class="muted">{table.count} {table.count === 1 ? "record" : "records"}</span></h3>
            <div class="scroll">
              <table class="records">
                <thead><tr>{#each table.columns as c (c)}<th>{c}</th>{/each}</tr></thead>
                <tbody>
                  {#each table.rows as row, r (r)}
                    <tr>{#each row as cell, c (c)}<td>{cell}</td>{/each}</tr>
                  {/each}
                </tbody>
              </table>
            </div>
            {#if table.rows.length < table.count}<p class="muted">The first {table.rows.length} of {table.count}.</p>{/if}
          {/each}
        {:else}
          <p class="muted">Choose a sample file to see its records as the entity's fields hold them.</p>
        {/if}
      </aside>
    </div>
  {/if}
</div>

<style>
  .mapping {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    padding: 12px 16px;
    gap: 10px;
    box-sizing: border-box;
    overflow: auto;
  }

  header {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }

  header .delete {
    margin-left: auto;
  }

  .muted {
    color: var(--text-muted);
  }

  .problem {
    color: var(--error);
    margin: 0;
  }

  .sides {
    display: flex;
    gap: 16px;
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .form {
    flex: 1 1 420px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }

  .grid {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 12px;
    align-items: center;
  }

  .with-browse {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .mono {
    font-family: var(--mono);
  }

  table {
    border-collapse: collapse;
    width: 100%;
  }

  th,
  td {
    text-align: left;
    padding: 3px 8px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
  }

  .fields input.mono {
    width: 100%;
    box-sizing: border-box;
  }

  .fields label {
    display: flex;
    gap: 6px;
    align-items: center;
    white-space: nowrap;
  }

  tr.inside td {
    background: var(--panel-alt);
  }

  .preview {
    flex: 1 1 360px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .preview h3 {
    font-size: 12px;
    margin: 8px 0 0;
  }

  .scroll {
    overflow: auto;
    max-height: 320px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .records td {
    font-family: var(--mono);
    white-space: nowrap;
  }
</style>
