<script lang="ts">
  // Asks for the name of a new window, where it goes and what it starts from.

  import Modal from "./Modal.svelte";
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

  /** The project's settings, which a settings window shows (DESIGN 5.17). */
  const hasSettings = $derived(preview.project.files.some((f) => f.entities.some((e) => e.settings)));
  const entities = $derived(preview.project.files.flatMap((f) => f.entities.filter((e) => !e.settings).map((e) => e.name.name)));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const target = preview.project.files.find((f) => f.path === path);
    if (!target || !name.trim()) return;
    const [kind, entity] = start.split(":");
    const template: PageTemplate =
      kind === "form"
        ? { kind: "form", entity }
        : kind === "browse"
          ? { kind: "browse", entity }
          : kind === "settings"
            ? { kind: "settings" }
            : { kind: "blank" };
    onsubmit(target, name.trim(), template);
  }

  function fileName(p: string): string {
    return p.split(/[\\/]/).at(-1) ?? p;
  }
</script>

<Modal title={"New window"} width={420} onclose={oncancel}>
  <form class="dialog" onsubmit={submit}>
    <label>
      <span>Name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={name} placeholder="CustomerList" spellcheck="false" autofocus />
    </label>
    <label>
      <span>Start from</span>
      <select bind:value={start}>
        <option value="blank">Blank window</option>
        {#if hasSettings}<option value="settings">Settings window</option>{/if}
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
