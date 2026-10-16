<script lang="ts">
  // A name shown as a heading that can be edited in place. Rename applies
  // it; Escape puts the old name back.

  let {
    name,
    title,
    onrename,
  }: {
    name: string;
    title: string;
    onrename: (newName: string) => void;
  } = $props();

  let draft = $state("");
  $effect.pre(() => {
    draft = name;
  });

  const changed = $derived(draft.trim() !== "" && draft.trim() !== name);

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (changed) onrename(draft.trim());
  }
</script>

<form class="rename" onsubmit={submit}>
  <input
    bind:value={draft}
    {title}
    spellcheck="false"
    onkeydown={(e) => {
      if (e.key === "Escape") draft = name;
    }}
  />
  {#if changed}
    <button type="submit" class="primary">Rename</button>
    <button type="button" onclick={() => (draft = name)}>Cancel</button>
  {/if}
</form>

<style>
  .rename {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  input {
    font-size: 18px;
    font-weight: 600;
    border-color: transparent;
    background: transparent;
    padding: 2px 6px;
    margin-left: -7px;
    width: 18ch;
  }

  input:hover,
  input:focus {
    border-color: var(--border-strong);
    background: var(--panel);
  }
</style>
