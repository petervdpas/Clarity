<script lang="ts">
  // The Folders section of Clarity settings, for every project: where New
  // makes a project and Open starts, and where Run and Build write the apps,
  // a folder per project. A folder is chosen with Browse only; the field shows it.

  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { invoke } from "./backend";
  import type { FoldersView } from "./generated/FoldersView";

  type Which = "projects" | "output";

  /** The two folders, in the order the page shows them. */
  const FOLDERS: { key: Which; name: string; note: string; unset: string; browse: string }[] = [
    {
      key: "projects",
      name: "Projects",
      note: "New makes a project here and asks only for its name; Open starts here.",
      unset: "Not set: New asks where each project goes.",
      browse: "Projects folder: where New makes a project",
    },
    {
      key: "output",
      name: "Apps",
      note: "Run and Build write each app in a folder of its own here, named after its project file. A project that names its own app folder in its settings keeps it.",
      unset: "Not set: each app is written in the out folder of its project.",
      browse: "Apps folder: where Run and Build write the apps",
    },
  ];

  let view = $state<FoldersView | null>(null);
  let error = $state<string | null>(null);

  async function load() {
    try {
      view = await invoke<FoldersView>("folders_view");
    } catch (err) {
      error = String(err);
    }
  }

  async function set(which: Which, path: string | null) {
    error = null;
    try {
      view = await invoke<FoldersView>("set_folder", { which, path });
    } catch (err) {
      error = String(err);
    }
  }

  async function browse(which: Which, title: string) {
    const picked = await openDialog({ directory: true, title, defaultPath: view?.[which] ?? undefined });
    if (typeof picked === "string") set(which, picked);
  }

  $effect(() => {
    void load();
  });
</script>

<div class="folders">
  <p class="muted">Where your projects and their apps are on this computer, for every project.</p>
  {#if error}<p class="refused" role="alert">{error}</p>{/if}

  {#if view}
    {#each FOLDERS as f (f.key)}
      <fieldset>
        <legend>{f.name}</legend>
        <p class="muted">{f.note}</p>
        <div class="other">
          <input type="text" readonly value={view[f.key] ?? ""} placeholder="Not set" aria-label="{f.name} folder" title={view[f.key] ?? f.unset} />
          <button onclick={() => browse(f.key, f.browse)}>Browse...</button>
          <button disabled={!view[f.key]} onclick={() => set(f.key, null)} title="Forget this folder">Clear</button>
        </div>
        {#if !view[f.key]}<p class="muted">{f.unset}</p>{/if}
      </fieldset>
    {/each}
    <p class="muted">Kept in <code>{view.file}</code>, which the <code>clarity</code> command reads too.</p>
  {/if}
</div>

<style>
  .folders {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  p {
    margin: 0;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  .refused {
    color: var(--error);
  }
  fieldset {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 12px;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  legend {
    font-size: 12px;
    color: var(--text-muted);
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
</style>
