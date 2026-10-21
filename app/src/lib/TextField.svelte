<script lang="ts">
  // A text people read, in the properties: a caption, a title, a tooltip.
  // It is a key with its value in each language file; the field shows and
  // edits the default language's value, and the key below it can be renamed.
  // A text that is no key yet (a new part's) is written as it is: the edit
  // makes it a key, its text the default language's value.

  import { invoke } from "@tauri-apps/api/core";
  import type { Preview } from "./generated/Preview";

  let {
    preview,
    text,
    placeholder = "",
    multiline = false,
    onwrite,
    onapplied,
  }: {
    preview: Preview;
    /** The text as the design has it: `@key`, a text without letters, or none. */
    text: string | null;
    placeholder?: string;
    multiline?: boolean;
    /** Writes a text that is no key yet, or none to take the text away. */
    onwrite: (text: string | null) => void;
    onapplied: (preview: Preview) => void;
  } = $props();

  const fallback = $derived(preview.project.locale.language ?? "en");
  const key = $derived(text?.startsWith("@") ? text.slice(1) : null);
  const value = $derived(key ? (preview.project.translations[fallback]?.[key] ?? "") : (text ?? ""));
  /** The other languages that leave this key empty. */
  const missing = $derived(
    key
      ? Object.entries(preview.project.translations)
          .filter(([language, values]) => language !== fallback && !values[key])
          .map(([language]) => language)
      : [],
  );

  let renaming = $state<string | null>(null);
  let error = $state<string | null>(null);

  async function apply(command: string, args: Record<string, unknown>) {
    try {
      onapplied(await invoke<Preview>(command, args));
      error = null;
    } catch (err) {
      error = String(err);
    }
  }

  function changed(typed: string) {
    const written = typed.trim();
    // Emptied: the part has no text any more; the key's values stay in the files.
    if (written === "") onwrite(null);
    else if (key) void apply("set_translation", { language: fallback, key, value: written });
    else onwrite(written);
  }

  function rename(e: SubmitEvent) {
    e.preventDefault();
    const to = renaming?.trim();
    if (key && to && to !== key) void apply("rename_key", { from: key, to }).then(() => (renaming = error ? renaming : null));
    else renaming = null;
  }
</script>

{#if multiline}
  <textarea rows="3" {value} {placeholder} onchange={(e) => changed(e.currentTarget.value)}></textarea>
{:else}
  <input {value} {placeholder} onchange={(e) => changed(e.currentTarget.value)} />
{/if}
{#if key}
  <div class="key">
    {#if renaming !== null}
      <form onsubmit={rename}>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="mono" bind:value={renaming} autofocus spellcheck="false" aria-label="The key's new name" onkeydown={(e) => e.key === "Escape" && (renaming = null)} />
        <button type="submit">Rename</button>
      </form>
    {:else}
      <span>key <code>{key}</code></span>
      <button type="button" class="link" onclick={() => (renaming = key)}>Rename</button>
      {#if missing.length > 0}<span class="missing" title="Open the language under i18n to translate it">not in {missing.join(", ")} yet</span>{/if}
    {/if}
  </div>
{/if}
{#if error}<p class="problem">{error}</p>{/if}

<style>
  textarea {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
    resize: vertical;
  }

  .key {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    margin-top: 2px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .key form {
    display: flex;
    gap: 4px;
  }

  .key input {
    width: 160px;
  }

  .mono {
    font-family: var(--mono);
  }

  .link {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    font-size: 11px;
  }

  .missing {
    color: var(--warning, var(--text-muted));
  }

  .problem {
    margin: 2px 0 0;
    color: var(--error);
    font-size: 12px;
  }
</style>
