<script lang="ts">
  // Asks for the name of a new Markdown file of the resources. Adding it
  // writes resources/<name>.md, starting with its heading; a key then points
  // a language at it, or a markdown part shows it by its source.

  let {
    taken,
    error,
    onsubmit,
    oncancel,
  }: {
    /** The names the resources have already. */
    taken: string[];
    error: string | null;
    onsubmit: (name: string) => void;
    oncancel: () => void;
  } = $props();

  let name = $state("");
  const file = $derived.by(() => {
    const trimmed = name.trim();
    return !trimmed ? "" : trimmed.toLowerCase().endsWith(".md") ? trimmed : `${trimmed}.md`;
  });
  const already = $derived(taken.includes(file));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (file && !already) onsubmit(file);
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && oncancel()} />

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && oncancel()}>
  <form class="dialog" onsubmit={submit}>
    <h2>New Markdown file</h2>
    <label>
      <span>Name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={name} placeholder="rules-en" spellcheck="false" autofocus />
    </label>
    <p class="hint">
      {#if already}
        The resources have {file} already.
      {:else if file}
        Clarity writes <code>resources/{file}</code>. Point a key at it per language, or show it with a markdown part's Source.
      {:else}
        A name of its own, such as <code>rules-en</code>; one file per language is up to you.
      {/if}
    </p>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="buttons">
      <button type="button" onclick={oncancel}>Cancel</button>
      <button type="submit" class="primary" disabled={!file || already}>Add Markdown file</button>
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

  .hint {
    margin: 0;
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
