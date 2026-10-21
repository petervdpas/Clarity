<script lang="ts">
  // Asks for a new language of the project: its tag, with the name it goes
  // by. Adding it writes its translation file, translations/<tag>.clarity.

  import { languageName } from "./translate";

  let {
    taken,
    error,
    onsubmit,
    oncancel,
  }: {
    /** The languages the project has already, the one written in too. */
    taken: string[];
    error: string | null;
    onsubmit: (tag: string) => void;
    oncancel: () => void;
  } = $props();

  /** Languages offered to pick from; any other tag can be typed. */
  const COMMON = ["nl", "de", "fr", "es", "it", "pt", "pl", "sv", "da", "nb", "fi", "en", "ja", "zh", "ko", "tr", "uk", "cs"];

  let tag = $state("");
  const trimmed = $derived(tag.trim());
  const name = $derived(trimmed ? languageName(trimmed) : "");
  const already = $derived(taken.includes(trimmed));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (trimmed && !already) onsubmit(trimmed);
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && oncancel()} />

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && oncancel()}>
  <form class="dialog" onsubmit={submit}>
    <h2>New language</h2>
    <label>
      <span>Language</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={tag} list="new-language-tags" placeholder="nl" spellcheck="false" autofocus />
      <datalist id="new-language-tags">
        {#each COMMON.filter((t) => !taken.includes(t)) as t (t)}<option value={t}>{languageName(t)}</option>{/each}
      </datalist>
    </label>
    <p class="hint">
      {#if already}
        The project has {name} already.
      {:else if trimmed && name !== trimmed}
        {name}: Clarity writes <code>translations/{trimmed}.clarity</code> with every text to translate.
      {:else}
        Its tag, such as <code>nl</code> or <code>de</code>, or with its region, <code>fr-BE</code>.
      {/if}
    </p>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="buttons">
      <button type="button" onclick={oncancel}>Cancel</button>
      <button type="submit" class="primary" disabled={!trimmed || already}>Add language</button>
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
