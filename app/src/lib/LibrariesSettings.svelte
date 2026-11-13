<script lang="ts">
  // The Libraries section of Clarity settings: what of Clarity each build
  // uses (Clarity.UI's library and browser files, each stack's packages),
  // where it comes from now, and a folder chosen in its place with Browse.
  // Factory reset forgets every choice, so builds take the libraries that
  // belong to this Clarity's version. Nothing here downloads: fetching stays
  // with the build that needs it.

  import { browse as browseFiles } from "./fileBrowserStore.svelte";
  import { dialogs } from "./dialogStore.svelte";
  import { invoke } from "@clarity/ui/host/backend.ts";
  import type { LibrariesView } from "@clarity/ui/generated/LibrariesView.ts";
  import type { LibraryRow } from "@clarity/ui/generated/LibraryRow.ts";
  import { anyChosen, usedText } from "./libraries";

  let view = $state<LibrariesView | null>(null);
  let error = $state<string | null>(null);

  /** The choice that leaves it to Clarity: what belongs to this Clarity. */
  const OWN = "";

  async function load() {
    error = null;
    try {
      view = await invoke<LibrariesView>("libraries_view");
    } catch (err) {
      error = String(err);
    }
  }

  async function choose(row: LibraryRow, folder: string | null) {
    error = null;
    try {
      view = await invoke<LibrariesView>("choose_library", { key: row.key, folder });
    } catch (err) {
      error = String(err);
    }
  }

  /** Chooses a folder with Clarity's file browser, starting in the one builds use now. */
  async function browse(row: LibraryRow) {
    const picked = await browseFiles({ purpose: "library-folder", title: `The folder of the ${row.name} builds use`, pick: "folder", start: row.chosen ?? row.folder });
    if (typeof picked === "string") choose(row, picked);
  }

  async function factoryReset() {
    const text = "Forget every folder chosen here? Builds then use the libraries and packages that belong to this Clarity's version.";
    if (!(await dialogs.confirm({ text, title: "Factory reset", action: "Reset" }))) return;
    error = null;
    try {
      view = await invoke<LibrariesView>("reset_libraries");
    } catch (err) {
      error = String(err);
    }
  }

  $effect(() => {
    void load();
  });
</script>

<div class="libraries">
  <p class="muted">What of Clarity each build uses, in every project: Clarity's own, which belongs to this Clarity's version, or a folder you choose, such as your own build of Clarity's repository. A choice counts from the next build; the design and preview draw with new browser files from their next load.</p>
  {#if error}<p class="refused" role="alert">{error}</p>{/if}

  {#if view}
    <fieldset>
      <legend>Libraries and packages</legend>
      {#each view.rows as row (row.key)}
        <div class="library">
          <div class="head">
            <strong>{row.name}</strong>
            <span class="muted">for {row.uses}</span>
            {#if row.version}<span class="version">{row.version}</span>{/if}
          </div>
          {#if row.choosable}
            <select
              aria-label="The {row.name} builds use"
              value={row.chosen ?? OWN}
              onchange={(e) => choose(row, e.currentTarget.value === OWN ? null : e.currentTarget.value)}
            >
              <option value={OWN}>Clarity's own (this IDE's version)</option>
              {#if row.chosen}<option value={row.chosen}>{row.chosen}</option>{/if}
            </select>
            <div class="other">
              <input type="text" readonly value={row.chosen ?? ""} placeholder="Or choose a folder with Browse" aria-label="The folder chosen for the {row.name}" title={row.chosen ?? ""} />
              <button onclick={() => browse(row)}>Browse...</button>
            </div>
          {:else if row.key === "goModule"}
            <p class="muted used">No folder can be chosen: Clarity does not point a Go program at one. A go.work beside the program does; the program's README says how.</p>
          {/if}
          <p class="muted used">
            Builds use: {usedText(row)}
            {#if row.folder}<code>{row.folder}</code>{/if}
          </p>
        </div>
      {/each}
    </fieldset>
    <div class="bar">
      <p class="muted">Kept in <code>{view.file}</code>.</p>
      <button onclick={factoryReset} disabled={!anyChosen(view.rows)} title="Forget every folder chosen here">Factory reset</button>
    </div>
  {:else if !error}
    <p class="muted">Looking at the libraries...</p>
  {/if}
</div>

<style>
  .libraries {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .libraries > p {
    margin: 0;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  .refused {
    color: var(--error);
    margin: 0;
  }
  fieldset {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 12px;
    margin: 0;
  }
  legend {
    font-size: 12px;
    color: var(--text-muted);
  }
  .library {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }
  .library:last-child {
    border-bottom: none;
  }
  .head {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  .version {
    font-family: var(--mono);
    font-size: 11px;
  }
  .library select {
    max-width: 100%;
  }
  .other {
    display: flex;
    gap: 6px;
  }
  .other input {
    flex: 1;
    font-family: var(--mono);
    cursor: default;
  }
  .used {
    margin: 0;
  }
  .used code {
    word-break: break-all;
  }
  .bar {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .bar p {
    flex: 1;
    margin: 0;
  }
</style>
