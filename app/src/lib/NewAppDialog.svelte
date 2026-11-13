<script lang="ts">
  // Asks for the name of a new app and the module it goes into, as New window does.

  import Modal from "./Modal.svelte";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";

  let {
    preview,
    file,
    error,
    onsubmit,
    oncancel,
  }: {
    preview: Preview;
    /** The file the app goes into, unless the user picks another. */
    file: SpecFile;
    error: string | null;
    onsubmit: (file: SpecFile, name: string) => void;
    oncancel: () => void;
  } = $props();

  /** Starts as the module's name with a capital, which the user may change. */
  let name = $state("");
  let path = $state("");
  $effect.pre(() => {
    path = file.path;
    const module = file.module?.name ?? "App";
    name = module.charAt(0).toUpperCase() + module.slice(1);
  });

  /** The modules: a page's or the app's own `.cxaml` file is not one. */
  const modules = $derived(preview.project.files.filter((f) => !f.path.endsWith(".cxaml")));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const target = preview.project.files.find((f) => f.path === path);
    if (!target || !name.trim()) return;
    onsubmit(target, name.trim());
  }

  function fileName(p: string): string {
    return p.split(/[\\/]/).at(-1) ?? p;
  }
</script>

<Modal title="New app" width={420} onclose={oncancel}>
  <form class="dialog" onsubmit={submit}>
    <label>
      <span>Name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={name} placeholder="Shop" spellcheck="false" autofocus />
    </label>
    <label>
      <span>In</span>
      <select bind:value={path}>
        {#each modules as f (f.path)}
          <option value={f.path}>{f.module?.name ?? "(no module)"} ({fileName(f.path)})</option>
        {/each}
      </select>
    </label>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="buttons">
      <button type="button" onclick={oncancel}>Cancel</button>
      <button type="submit" class="primary" disabled={!name.trim()}>Add app</button>
    </div>
  </form>
</Modal>

<style>
  .dialog {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .error {
    margin: 0;
    color: var(--error);
    font-size: 12px;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
