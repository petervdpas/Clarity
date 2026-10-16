<script lang="ts">
  // Asks for the name of a new window, where it goes and what it starts from.

  import type { PageTemplate } from "./generated/PageTemplate";
  import type { Preview } from "./generated/Preview";
  import type { SpecFile } from "./generated/SpecFile";

  let {
    preview,
    file,
    error,
    onsubmit,
    oncancel,
  }: {
    preview: Preview;
    /** The file the window goes into, unless the user picks another. */
    file: SpecFile;
    error: string | null;
    onsubmit: (file: SpecFile, name: string, template: PageTemplate) => void;
    oncancel: () => void;
  } = $props();

  let name = $state("");
  /** `blank`, or a template and an entity: `form:Customer`, `browse:Customer`. */
  let start = $state("blank");
  let path = $state("");
  $effect.pre(() => {
    path = file.path;
  });

  const entities = $derived(preview.project.files.flatMap((f) => f.entities.map((e) => e.name.name)));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const target = preview.project.files.find((f) => f.path === path);
    if (!target || !name.trim()) return;
    const [kind, entity] = start.split(":");
    const template: PageTemplate =
      kind === "form" ? { kind: "form", entity } : kind === "browse" ? { kind: "browse", entity } : { kind: "blank" };
    onsubmit(target, name.trim(), template);
  }

  function fileName(p: string): string {
    return p.split(/[\\/]/).at(-1) ?? p;
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && oncancel()} />

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && oncancel()}>
  <form class="dialog" onsubmit={submit}>
    <h2>New window</h2>
    <label>
      <span>Name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={name} placeholder="CustomerList" spellcheck="false" autofocus />
    </label>
    <label>
      <span>Start from</span>
      <select bind:value={start}>
        <option value="blank">Blank window</option>
        {#each entities as entity (entity)}
          <option value="form:{entity}">Form for {entity}</option>
          <option value="browse:{entity}">Browse for {entity}</option>
        {/each}
      </select>
    </label>
    <label>
      <span>In</span>
      <select bind:value={path}>
        {#each preview.project.files as f (f.path)}
          <option value={f.path}>{f.module?.name ?? "(no module)"} ({fileName(f.path)})</option>
        {/each}
      </select>
    </label>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="buttons">
      <button type="button" onclick={oncancel}>Cancel</button>
      <button type="submit" class="primary" disabled={!name.trim()}>Add window</button>
    </div>
  </form>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    display: grid;
    place-items: center;
    z-index: 50;
  }

  .dialog {
    width: min(420px, calc(100vw - 32px));
    background: var(--panel);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.35);
  }

  h2 {
    margin: 0;
    font-size: 15px;
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
