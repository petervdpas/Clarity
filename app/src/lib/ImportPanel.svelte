<script lang="ts">
  // Imports tables of an external database, or of a data source that is a
  // file, as a module of entities mapped onto them. The backend reads the
  // tables and writes the module; this lets the developer pick what to
  // import. A file source holds one table, so one is picked, and the backend
  // writes a mapping beside its entity; a format with schemas (an XSD for
  // XML) can take its tables, typed, from a schema.

  import { invoke } from "@tauri-apps/api/core";
  import { browse } from "./fileBrowserStore.svelte";
  import type { Imported } from "@clarity/ui/generated/Imported.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import type { SourceTable } from "@clarity/ui/generated/SourceTable.ts";

  let {
    source,
    single = false,
    schemaKinds = [],
    onapplied,
    onclose,
  }: {
    /** The external data source to import from. */
    source: string;
    /** A file source: one table is imported. */
    single?: boolean;
    /** The file kinds of the format's schemas (`xsd`), when it has them. */
    schemaKinds?: string[];
    onapplied: (preview: Preview) => void;
    onclose: () => void;
  } = $props();

  /** A schema the tables come from, relative to the project's folder; none for the file itself. */
  let schema = $state<string | null>(null);

  let tables = $state<SourceTable[] | null>(null);
  let chosen = $state<string[]>([]);
  let module = $state("");
  let error = $state<string | null>(null);
  let done = $state<Imported | null>(null);
  let busy = $state(false);

  $effect(() => {
    const name = source;
    const from = schema;
    module = name;
    done = null;
    invoke<SourceTable[]>("source_tables", { source: name, schema: from })
      .then((found) => {
        tables = found;
        const open = found.filter((t) => t.notImportable === null).map((t) => t.name);
        chosen = single ? open.slice(0, 1) : open;
        error = null;
      })
      .catch((err) => {
        tables = null;
        error = String(err);
      });
  });

  function toggle(name: string, on: boolean) {
    if (single) chosen = on ? [name] : [];
    else chosen = on ? [...chosen, name] : chosen.filter((n) => n !== name);
  }

  async function pickSchema() {
    const picked = await browse({ purpose: "import-schema", title: "Schema to import from", pick: "file", kinds: schemaKinds, start: await invoke<string>("project_folder") });
    if (typeof picked === "string") schema = await invoke<string>("project_path", { picked });
  }

  async function run(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    busy = true;
    try {
      done = await invoke<Imported>("import_tables", { source, module: module.trim(), tables: chosen, schema });
      onapplied(done.preview);
      error = null;
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="import">
  <div class="head">
    <strong>Import from {source}</strong>
    <button class="small" onclick={onclose} title="Close">&times;</button>
  </div>
  <p class="intro">
    {#if single}
      The chosen table becomes an entity in a new module, with a mapping beside it that says where its records and each field are in the
      file; the data source is laid over that mapping, whose paths can be changed on its page. Clarity reads and writes the records and
      keeps the rest of the file as it is.
    {:else}
      The chosen tables become entities in a new module, mapped onto their tables and columns. Clarity reads and writes their records but
      never changes these tables.
    {/if}
  </p>
  {#if schemaKinds.length > 0}
    <div class="row">
      <span class="muted">{schema ? `Typed from ${schema}` : "From the file itself; a schema gives the fields their types"}</span>
      <button class="small" onclick={pickSchema} title="Take the tables and their types from a schema">Schema...</button>
      {#if schema}<button class="small" onclick={() => (schema = null)} title="Take the tables from the file itself">From the file</button>{/if}
    </div>
  {/if}

  {#if error}<p class="problem">{error}</p>{/if}

  {#if done}
    <p>Wrote <code>{done.file}</code>.</p>
    {#each done.skipped as s (s.table)}
      <p class="muted">{s.table} is left out: {s.reason}</p>
    {/each}
  {:else if tables}
    <form onsubmit={run}>
      <ul class="tables">
        {#each tables as t (t.name)}
          <li>
            <label class:off={t.notImportable !== null}>
              <input
                type={single ? "radio" : "checkbox"}
                name="importTable"
                disabled={t.notImportable !== null}
                checked={chosen.includes(t.name)}
                onchange={(e) => toggle(t.name, e.currentTarget.checked)}
              />
              <span class="mono">{t.name}</span>
              <span class="muted">{t.notImportable ?? `${t.columns} columns`}</span>
            </label>
          </li>
        {/each}
      </ul>
      <div class="row">
        <label>Module <input class="mono" bind:value={module} spellcheck="false" /></label>
        <button type="submit" disabled={busy || chosen.length === 0 || !module.trim()}>
          Import {chosen.length} {chosen.length === 1 ? "table" : "tables"}
        </button>
      </div>
    </form>
  {/if}
</div>

<style>
  .import {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .tables {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tables label {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }

  .tables label.off {
    opacity: 0.6;
  }

  .row {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-top: 6px;
  }

  .row label {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .mono {
    font-family: var(--mono);
  }

  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }

  .problem {
    color: var(--error);
  }
</style>
