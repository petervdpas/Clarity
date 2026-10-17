<script lang="ts">
  // The project's migration steps against its data dictionary: the changes
  // no step covers yet, the renames Clarity proposes, and writing them as
  // the next step. The backend decides what is pending; this only shows it.

  import { invoke } from "@tauri-apps/api/core";
  import type { PendingMigration } from "./generated/PendingMigration";
  import type { Preview } from "./generated/Preview";
  import type { Rename } from "./generated/Rename";

  let {
    source,
    named,
    preview,
    onapplied,
  }: {
    /** The data source whose steps these are. */
    source: string;
    /** Whether to name the source: the project has more than one database. */
    named: boolean;
    /** The loaded project; the changes are read again when it changes. */
    preview: Preview;
    onapplied: (preview: Preview) => void;
  } = $props();

  let pending = $state<PendingMigration | null>(null);
  let error = $state<string | null>(null);
  /** The proposed renames the developer turned down. */
  let declined = $state<Rename[]>([]);
  let title = $state("");
  let checked = $state(false);
  let busy = $state(false);

  function same(a: Rename, b: Rename): boolean {
    return a.entity === b.entity && a.from === b.from && a.to === b.to;
  }

  function isDeclined(r: Rename): boolean {
    return declined.some((d) => same(d, r));
  }

  function describe(r: Rename): string {
    return r.entity ? `${r.entity}.${r.from} is now ${r.to}` : `${r.from} is now ${r.to}`;
  }

  /** The proposed renames that are still taken as renames. */
  function accepted(): Rename[] {
    return (pending?.proposed ?? []).filter((r) => !isDeclined(r));
  }

  async function refresh(renames: Rename[] | null) {
    try {
      pending = await invoke<PendingMigration>("pending_migration", { source, renames });
      error = null;
    } catch (err) {
      error = String(err);
    }
  }

  // Every reload of the project (an edit, a file changed elsewhere) can
  // change what is pending.
  $effect(() => {
    void preview;
    declined = [];
    checked = false;
    refresh(null);
  });

  function toggle(r: Rename, on: boolean) {
    declined = on ? declined.filter((d) => !same(d, r)) : [...declined, r];
    refresh(accepted());
  }

  const losesData = $derived(pending?.changes.some((c) => c.losesData) ?? false);
  const next = $derived((pending?.steps ?? 0) + 1);

  async function create(e: SubmitEvent) {
    e.preventDefault();
    if (!pending || busy) return;
    busy = true;
    try {
      const name = title.trim() || `Step ${next}`;
      onapplied(await invoke<Preview>("create_migration", { source, title: name, renames: accepted() }));
      title = "";
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<section>
  <h2>Migrations{named ? ` of ${source}` : ""}</h2>
  <p class="intro">
    Each step in <code>migrations/{source}</code> brings the database to the next version of the data dictionary. Run and the generated
    application apply the steps a database does not have yet.
  </p>

  {#if error}<p class="problem">{error}</p>{/if}

  {#if pending && pending.changes.length === 0}
    <p class="done">
      {pending.steps === 1 ? "The step covers" : `The ${pending.steps} steps cover`} the data dictionary; there is nothing to migrate.
    </p>
  {:else if pending}
    <p>Changes in the data dictionary that no step covers yet:</p>
    <ul class="changes">
      {#each pending.changes as change, i (i)}
        <li>
          <pre>{change.text}</pre>
          {#if change.losesData}<span class="risk">can lose data</span>{/if}
        </li>
      {/each}
    </ul>

    {#if pending.proposed.length > 0}
      <fieldset class="renames">
        <legend>Renames</legend>
        <p class="hint">A name went and a name of the same type came. Kept as a rename, the data stays; otherwise the old field is removed.</p>
        {#each pending.proposed as r, i (i)}
          <label>
            <input type="checkbox" checked={!isDeclined(r)} onchange={(e) => toggle(r, e.currentTarget.checked)} />
            {describe(r)}
          </label>
        {/each}
      </fieldset>
    {/if}

    {#if pending.blocked}
      <p class="problem">{pending.blocked}</p>
    {:else}
      <form class="create" onsubmit={create}>
        <input bind:value={title} placeholder={`Step ${next}`} aria-label="Title of the step" />
        {#if losesData}
          <label class="confirm">
            <input type="checkbox" bind:checked />
            I checked the changes that can lose data
          </label>
        {/if}
        <button type="submit" disabled={busy || (losesData && !checked)}>Create step {next}</button>
      </form>
    {/if}
  {/if}
</section>

<style>
  .changes {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .changes li {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }

  pre {
    margin: 0;
    font-family: var(--mono);
    font-size: 12px;
    white-space: pre-wrap;
  }

  .risk {
    flex: none;
    font-size: 11px;
    color: var(--error);
    border: 1px solid var(--error);
    border-radius: 3px;
    padding: 0 4px;
  }

  .renames {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .renames label,
  .confirm {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .create {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .problem {
    color: var(--error);
  }

  .done,
  .hint {
    color: var(--text-muted);
  }
</style>
