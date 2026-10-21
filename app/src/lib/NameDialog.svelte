<script lang="ts">
  // Asks for a name and the module a new declaration goes into.

  import type { Preview } from "./generated/Preview";
  import type { SpecFile } from "./generated/SpecFile";

  let {
    preview,
    title,
    placeholder,
    action,
    file,
    error,
    choice = null,
    onsubmit,
    oncancel,
  }: {
    preview: Preview;
    title: string;
    placeholder: string;
    /** The label of the confirm button. */
    action: string;
    /** The module proposed first. */
    file: SpecFile;
    error: string | null;
    /** A choice to make with the name, its first option picked to start with. */
    choice?: { label: string; options: { value: string; label: string }[] } | null;
    onsubmit: (file: SpecFile, name: string, picked: string) => void;
    oncancel: () => void;
  } = $props();

  let name = $state("");
  let path = $state("");
  let picked = $state("");
  $effect.pre(() => {
    picked = choice?.options[0]?.value ?? "";
  });
  $effect.pre(() => {
    path = file.path;
  });

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const target = preview.project.files.find((f) => f.path === path);
    if (target && name.trim()) onsubmit(target, name.trim(), picked);
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && oncancel()} />

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && oncancel()}>
  <form class="dialog" onsubmit={submit}>
    <h2>{title}</h2>
    <label>
      <span>Name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={name} {placeholder} spellcheck="false" autofocus />
    </label>
    <label>
      <span>In module</span>
      <select bind:value={path}>
        {#each preview.project.files as f (f.path)}
          <option value={f.path}>{f.module?.name ?? "(no module)"}</option>
        {/each}
      </select>
    </label>
    {#if choice}
      <label>
        <span>{choice.label}</span>
        <select bind:value={picked}>
          {#each choice.options as o (o.value)}
            <option value={o.value}>{o.label}</option>
          {/each}
        </select>
      </label>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
    <div class="buttons">
      <button type="button" onclick={oncancel}>Cancel</button>
      <button type="submit" class="primary" disabled={!name.trim()}>{action}</button>
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
    width: min(380px, calc(100vw - 32px));
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
