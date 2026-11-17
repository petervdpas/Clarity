<script lang="ts">
  // DevTools' Disk tab (src-tauri/src/devtools/disk.rs): the folders
  // Clarity's builds and checks fill in its repository, biggest first, each
  // with what removing it costs and Remove, which asks first; and the free
  // space of the disk, with a warning below 40 GB. At next start marks a
  // folder the running IDE holds (on Windows its own files) to go when
  // dev.sh or dev.bat starts it again, before anything builds.

  import { invoke } from "@tauri-apps/api/core";
  import type { DiskView } from "@clarity/ui/generated/DiskView.ts";
  import { dialogs } from "./dialogStore.svelte";
  import { bytesText } from "./devtools";

  let view = $state<DiskView | null>(null);
  let error = $state<string | null>(null);
  let working = $state(false);

  async function call(command: string, args: Record<string, unknown> = {}) {
    error = null;
    working = true;
    try {
      view = await invoke<DiskView>(command, args);
    } catch (err) {
      error = String(err);
    } finally {
      working = false;
    }
  }

  $effect(() => {
    void call("disk_view");
  });

  const places = $derived(view ? [...view.places].sort((a, b) => b.bytes - a.bytes) : []);
  const total = $derived(places.reduce((sum, p) => sum + p.bytes, 0));

  async function remove(place: DiskView["places"][number]) {
    const sure = await dialogs.confirm({
      title: "Remove",
      text: `Remove ${place.name}, ${bytesText(place.bytes)}? ${place.cost}`,
      action: "Remove",
      danger: true,
    });
    if (sure) await call("disk_remove", { key: place.key });
  }

  const anyMarked = $derived(places.some((p) => p.marked));

  /** Marks the place to go at the next start, or takes the mark away. */
  async function mark(place: DiskView["places"][number], marked: boolean) {
    await call("disk_mark", { key: place.key, marked });
  }
</script>

<div class="disk">
  {#if error}<p class="refused" role="alert">{error}</p>{/if}
  {#if view}
    <div class="free" class:low={view.low} role="status">
      {#if view.free === null}
        <strong>The free space of this disk is not known.</strong>
      {:else if view.low}
        <strong>{bytesText(view.free)} free: below the 40 GB builds and checks need.</strong> Remove a folder below.
      {:else}
        <strong>{bytesText(view.free)} free.</strong>
      {/if}
      <button onclick={() => call("disk_view")} disabled={working}>{working ? "Measuring..." : "Measure again"}</button>
    </div>
    <table>
      <thead>
        <tr><th>Folder</th><th class="size">Size</th><th>Removing it</th><th></th></tr>
      </thead>
      <tbody>
        {#each places as p (p.key)}
          <tr class:empty={p.bytes === 0}>
            <td>{p.name}</td>
            <td class="size">{bytesText(p.bytes)}</td>
            <td class="muted">{p.cost}{#if p.marked}{" "}<strong class="marked">Goes at the next start.</strong>{/if}</td>
            <td class="actions">
              <button onclick={() => remove(p)} disabled={working || p.bytes === 0}>Remove</button>
              {#if p.marked}
                <button onclick={() => mark(p, false)} disabled={working} title="It stays at the next start">Unmark</button>
              {:else}
                <button onclick={() => mark(p, true)} disabled={working || p.bytes === 0} title="Removed when dev.sh or dev.bat starts Clarity again, before anything builds: for a folder the running Clarity holds">At next start</button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
      <tfoot>
        <tr><td>All of them</td><td class="size">{bytesText(total)}</td><td></td><td></td></tr>
      </tfoot>
    </table>
    <p class="muted">Every folder here is one git ignores; a build makes it again, except what examples no longer in Clarity left behind. Remove waits until no build runs.</p>
    {#if anyMarked}
      <p class="muted">The marked folders go when dev.sh or dev.bat starts Clarity again, before anything builds: close Clarity and start it again.</p>
    {/if}
  {:else if working}
    <p class="muted">Measuring the folders...</p>
  {/if}
</div>

<style>
  .disk {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .refused {
    color: var(--error);
    margin: 0;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  .free {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-left: 4px solid var(--accent);
    border-radius: 6px;
  }
  .free.low {
    border-left-color: var(--error);
  }
  table {
    border-collapse: collapse;
    font-size: 12px;
  }
  th,
  td {
    text-align: left;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
  }
  .size {
    text-align: right;
    white-space: nowrap;
  }
  tr.empty td {
    color: var(--text-muted);
  }
  .actions {
    white-space: nowrap;
  }
  .marked {
    color: var(--warning);
  }
  tfoot td {
    font-weight: 600;
    border-bottom: none;
  }
  p {
    margin: 0;
  }
</style>
