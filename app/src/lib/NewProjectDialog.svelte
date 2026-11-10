<script lang="ts">
  // Asks for a new project before it is created: its name, the folder it
  // goes in (the projects folder of Clarity settings to start with, chosen
  // with Browse), and what it is transpiled into (backend and targets; every
  // backend offers only Clarity.UI for a new project, so there is no UI to choose). The line under the folder
  // shows the file it becomes as the name is typed, or why it cannot be (a
  // name taken there). The project starts with one SQLite data source; more
  // are added in the project settings.

  import { browse as browseFiles } from "./fileBrowserStore.svelte";
  import { invoke } from "@clarity/ui/host/backend.ts";
  import Modal from "./Modal.svelte";
  import StackFields from "./StackFields.svelte";
  import type { Catalog } from "@clarity/ui/generated/Catalog.ts";
  import type { Stack } from "@clarity/ui/generated/Stack.ts";

  let {
    catalog,
    folder: startFolder,
    error,
    onsubmit,
    oncancel,
  }: {
    catalog: Catalog;
    /** The folder to start in: the projects folder, else the open project's; empty when there is neither. */
    folder: string;
    error: string | null;
    /** The project file chosen, and its stack. */
    onsubmit: (stack: Stack, file: string) => void;
    oncancel: () => void;
  } = $props();

  let name = $state("");
  let folder = $state("");
  let stack = $state<Stack>({ backend: "rust", ui: "clarity", targets: [] });
  /** The file the project becomes, or why it cannot be made there. */
  let target = $state<{ file: string } | { problem: string } | null>(null);
  $effect.pre(() => {
    stack = { ...catalog.starter, targets: [...catalog.starter.targets] };
    folder = startFolder;
  });

  // Asked again whenever the name or the folder changes; an older answer does not overwrite a newer one.
  let asked = 0;
  $effect(() => {
    const [n, f] = [name, folder];
    const ask = ++asked;
    if (!n.trim()) {
      target = null;
      return;
    }
    invoke<string>("new_project_path", { folder: f, name: n })
      .then((file) => ask === asked && (target = { file }))
      .catch((why) => ask === asked && (target = { problem: String(why) }));
  });

  async function browse() {
    const picked = await browseFiles({ purpose: "new-project-folder", title: "The folder the new project goes in", pick: "folder", start: folder || null });
    if (typeof picked === "string") folder = picked;
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (target && "file" in target) onsubmit(stack, target.file);
  }
</script>

<Modal title={"New project"} width={560} onclose={oncancel}>
  <form class="dialog" onsubmit={submit}>
    <div class="form-grid compact">
      <label class="form-label" for="new-project-name">Name</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="new-project-name" bind:value={name} placeholder="shop" spellcheck="false" autofocus />
      <label class="form-label" for="new-project-folder">Folder</label>
      <div class="folder-row">
        <!-- Chosen with Browse, never typed, so no path is mistyped. -->
        <input id="new-project-folder" value={folder} readonly placeholder="Choose a folder" title={folder} />
        <button type="button" onclick={browse}>Browse...</button>
      </div>
      <span></span>
      {#if target && "file" in target}
        <p class="becomes" title={target.file}>Creates {target.file}</p>
      {:else if target}
        <p class="error">{target.problem}</p>
      {:else}
        <p class="muted">The project's file goes in this folder, beside a folder of the same name for its modules.</p>
      {/if}
      <StackFields {catalog} {stack} showUi={false} onchange={(next) => (stack = next)} />
    </div>
    <p class="note">The project starts with one SQLite database. Data sources and the stack can be changed later on the Project tab.</p>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="buttons">
      <button type="button" onclick={oncancel}>Cancel</button>
      <button type="submit" class="primary" disabled={stack.targets.length === 0 || !(target && "file" in target)}>Create project</button>
    </div>
  </form>
</Modal>

<style>

  .compact {
    grid-template-columns: 80px minmax(0, 1fr);
  }

  .dialog {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }


  p {
    margin: 0;
    font-size: 12px;
  }

  .folder-row {
    display: flex;
    gap: 6px;
    min-width: 0;
  }

  .folder-row input {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
    cursor: default;
  }

  .becomes {
    font-family: var(--mono);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .muted {
    color: var(--text-muted);
  }

  .note {
    color: var(--text-muted);
  }

  .error {
    color: var(--error);
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
