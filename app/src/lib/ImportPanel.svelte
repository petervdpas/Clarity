<script lang="ts">
  // Imports tables of an external database as a module of entities mapped
  // onto them. The backend reads the tables and writes the module; this
  // lets the developer pick what to import.

  import { invoke } from "@tauri-apps/api/core";
  import type { Imported } from "./generated/Imported";
  import type { Preview } from "./generated/Preview";
  import type { SourceTable } from "./generated/SourceTable";

  let {
    source,
    onapplied,
    onclose,
  }: {
    /** The external data source to import from. */
    source: string;
    onapplied: (preview: Preview) => void;
    onclose: () => void;
  } = $props();

  let tables = $state<SourceTable[] | null>(null);
  let chosen = $state<string[]>([]);
  let module = $state("");
  let error = $state<string | null>(null);
  let done = $state<Imported | null>(null);
  let busy = $state(false);

  $effect(() => {
    const name = source;
    module = name;
    done = null;
    invoke<SourceTable[]>("source_tables", { source: name })
      .then((found) => {
        tables = found;
        chosen = found.filter((t) => t.notImportable === null).map((t) => t.name);
        error = null;
      })
      .catch((err) => {
        tables = null;
        error = String(err);
      });
  });

  function toggle(name: string, on: boolean) {
    chosen = on ? [...chosen, name] : chosen.filter((n) => n !== name);
  }

  async function run(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    busy = true;
    try {
      done = await invoke<Imported>("import_tables", { source, module: module.trim(), tables: chosen });
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
    The chosen tables become entities in a new module, mapped onto their tables and columns. Clarity reads and writes their records but
    never changes these tables.
  </p>

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
                type="checkbox"
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
