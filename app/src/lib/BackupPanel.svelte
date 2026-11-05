<script lang="ts">
  // The backups of a data source's database (docs/BACKUP-PLAN.md): take one,
  // and restore or delete one, each confirmed in Clarity's own dialog. The
  // backend decides which databases are backed up and does the work; this
  // only shows it.

  import { invoke } from "@tauri-apps/api/core";
  import type { BackupList } from "@clarity/ui/generated/BackupList.ts";
  import type { BackupEntry } from "@clarity/ui/generated/BackupEntry.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import { dialogs } from "./dialogStore.svelte";

  let {
    source,
    named,
    preview,
  }: {
    /** The data source whose database this is. */
    source: string;
    /** Whether to name the source: the project has more than one database. */
    named: boolean;
    /** The loaded project; the list is read again when it changes (a migration can add a backup). */
    preview: Preview;
  } = $props();

  let list = $state<BackupList | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function run(command: string, args: Record<string, unknown> = {}) {
    busy = true;
    try {
      list = await invoke<BackupList>(command, { source, ...args });
      error = null;
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    void preview;
    run("list_backups");
  });

  /** When a backup was taken, in the person's own time: `20261105T213012Z` read as UTC. */
  function when(b: BackupEntry): string {
    const w = b.when;
    const at = new Date(Date.UTC(+w.slice(0, 4), +w.slice(4, 6) - 1, +w.slice(6, 8), +w.slice(9, 11), +w.slice(11, 13), +w.slice(13, 15)));
    return at.toLocaleString();
  }

  /** Why it was taken, as people read it. */
  function why(b: BackupEntry): string {
    if (b.why === "manual") return "taken by hand";
    if (b.why === "before-restore") return "before a restore";
    const step = b.why.match(/^before-migration-(\d+)$/);
    if (step) return `before migration step ${Number(step[1])}`;
    return b.why;
  }

  function size(bytes: number): string {
    if (bytes < 1000) return `${bytes} B`;
    if (bytes < 1_000_000) return `${(bytes / 1000).toFixed(1)} kB`;
    return `${(bytes / 1_000_000).toFixed(1)} MB`;
  }

  async function restore(b: BackupEntry) {
    const text = `Put the backup of ${when(b)} (${why(b)}) in place of ${source}'s database? What it holds now is backed up first, and an application Run started is stopped.`;
    if (!(await dialogs.confirm({ text, action: "Restore", danger: true }))) return;
    await run("restore_backup", { file: b.file });
  }

  async function remove(b: BackupEntry) {
    if (!(await dialogs.confirm({ text: `Delete the backup of ${when(b)} (${why(b)})? It cannot be restored after.`, action: "Delete", danger: true }))) return;
    await run("delete_backup", { file: b.file });
  }
</script>

<section>
  <h2>Backups{named ? ` of ${source}` : ""}</h2>
  {#if list?.unavailable}
    <p class="done">{list.unavailable}.</p>
  {:else}
    <p class="intro">
      Copies of the database in <code>backups/</code> beside it. Clarity takes one before it applies migration steps to a database that holds data,
      and keeps the newest ten of those; one taken by hand stays until you delete it.
    </p>
    {#if error}<p class="problem">{error}</p>{/if}
    {#if list?.done}<p class="done">{list.done}.</p>{/if}
    <button disabled={busy || !list} onclick={() => run("take_backup")}>Back up now</button>
    {#if list && list.backups.length > 0}
      <table>
        <tbody>
          {#each list.backups as b (b.file)}
            <tr>
              <td>{when(b)}</td>
              <td class="why">{why(b)}</td>
              <td class="size">{size(b.bytes)}</td>
              <td class="actions">
                <button disabled={busy} onclick={() => restore(b)}>Restore</button>
                <button class="danger" disabled={busy} title="Delete the backup" onclick={() => remove(b)}>&times;</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else if list}
      <p class="done">No backups yet.</p>
    {/if}
  {/if}
</section>

<style>
  table {
    border-collapse: collapse;
    margin-top: 8px;
  }

  td {
    padding: 2px 10px 2px 0;
    font-size: 12px;
  }

  .why {
    color: var(--text-muted);
  }

  .size {
    text-align: right;
    font-family: var(--mono);
  }

  .actions {
    display: flex;
    gap: 6px;
  }

  .problem {
    color: var(--error);
  }

  .done {
    color: var(--text-muted);
  }
</style>
