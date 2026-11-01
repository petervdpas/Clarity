<script lang="ts">
  // Clarity's folder input on the web (X2): the path of a folder setting,
  // where a file store keeps its files. It is typed like any text, or chosen
  // with the system's folder picker through its button, named in Clarity's
  // own word; a chosen folder is a change at once. Where nothing opens a
  // picker, as in a browser, there is no button and the path is typed. The
  // same as on Avalonia.

  let {
    value,
    words,
    choose = undefined,
    id = undefined,
    tabindex = undefined,
    describedBy = undefined,
    invalid = false,
    disabled = false,
    readonly = false,
    onchange,
    onfocus = undefined,
    onblur = undefined,
  }: {
    value: unknown;
    /** Clarity's own word, in the language shown. */
    words: { choose: string };
    /** Opens the system's folder picker with a title: the folder chosen, or null. None where there is no picker. */
    choose?: (title: string) => Promise<string | null>;
    id?: string;
    tabindex?: number;
    describedBy?: string;
    invalid?: boolean;
    disabled?: boolean;
    readonly?: boolean;
    /** The person changed the folder: Clarity's `change`. */
    onchange: (next: string) => void;
    onfocus?: () => void;
    onblur?: () => void;
  } = $props();

  const shown = $derived(value === null || value === undefined ? "" : String(value));

  function take(typed: string) {
    if (typed !== shown) onchange(typed);
  }

  async function pick() {
    const chosen = await choose?.(words.choose);
    if (chosen) take(chosen);
  }
</script>

<span class="folder-input">
  <input
    {id}
    type="text"
    value={shown}
    {tabindex}
    aria-describedby={describedBy}
    aria-invalid={invalid}
    {disabled}
    {readonly}
    spellcheck="false"
    onchange={(e) => take(e.currentTarget.value)}
    onkeydown={(e) => e.key === "Enter" && take(e.currentTarget.value)}
    {onfocus}
    {onblur}
  />
  {#if choose && !readonly}
    <button type="button" aria-label={words.choose} title={words.choose} {disabled} onclick={pick}>...</button>
  {/if}
</span>

<style>
  .folder-input {
    display: flex;
    align-items: stretch;
    width: 100%;
    height: 100%;
    gap: 4px;
  }
  .folder-input input {
    flex: 1;
    min-width: 0;
  }
</style>
