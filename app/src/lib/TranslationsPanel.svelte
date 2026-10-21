<script lang="ts">
  // A language of the project, opened from i18n in the project tree: every
  // key with its value to type in place. The default language gives every
  // key a value, and shows how far each translation is; a translation shows
  // the default's text next to its own. The backend collects the keys and
  // writes the language files.

  import { invoke } from "@tauri-apps/api/core";
  import type { Preview } from "./generated/Preview";
  import type { TranslationTable } from "./generated/TranslationTable";
  import { languageName } from "./translate";

  let {
    preview,
    language,
    onapplied,
    onopen,
    onnewlanguage,
  }: {
    /** The loaded project; the table is read again when it changes. */
    preview: Preview;
    /** The language opened: a translation, or the default language. */
    language: string;
    onapplied: (preview: Preview) => void;
    /** Opens another language. */
    onopen: (language: string) => void;
    /** Asks for a new language and writes its file. */
    onnewlanguage: () => void;
  } = $props();

  let table = $state<TranslationTable | null>(null);
  let error = $state<string | null>(null);
  let onlyMissing = $state(false);
  let filter = $state("");

  $effect(() => {
    void preview;
    invoke<TranslationTable>("translation_table")
      .then((t) => {
        table = t;
        error = null;
      })
      .catch((err) => (error = String(err)));
  });

  /** The column of the opened language; the default's is 0. */
  const column = $derived(table?.languages.indexOf(language) ?? -1);
  const isDefault = $derived(table !== null && language === table.default);
  /** The other languages, with their columns. */
  const others = $derived((table?.languages ?? []).map((l, i) => ({ l, i })).filter(({ l }) => l !== table?.default));
  const total = $derived(table?.rows.length ?? 0);

  const rows = $derived(
    (table?.rows ?? []).filter((r) => {
      const words = filter.trim().toLocaleLowerCase();
      const said = [r.key, r.values[0] ?? "", column >= 0 ? (r.values[column] ?? "") : ""].join(" ").toLocaleLowerCase();
      if (words && !said.includes(words)) return false;
      if (!onlyMissing) return true;
      return isDefault ? r.values.some((v) => v === null) : r.values[column] === null;
    }),
  );

  /** A language as people know it: its own name and its tag. */
  function named(tag: string): string {
    const name = languageName(tag);
    return name === tag ? tag : `${name} (${tag})`;
  }

  async function setValue(key: string, value: string) {
    try {
      onapplied(await invoke<Preview>("set_translation", { language, key, value }));
      error = null;
    } catch (err) {
      error = String(err);
    }
  }

  /** A box grows with its text, so a long translation reads whole. */
  function grow(box: HTMLTextAreaElement) {
    const fit = () => {
      box.style.height = "auto";
      box.style.height = `${box.scrollHeight}px`;
    };
    fit();
    box.addEventListener("input", fit);
    return { destroy: () => box.removeEventListener("input", fit) };
  }
</script>

<section class="languages">
  <header>
    <h2>{named(language)}{isDefault ? ", the default language" : ""}</h2>
    {#if table && column >= 0}
      {@const missing = table.missing[column]}
      {#if isDefault}
        <p class="lead">Every key with its text, {total} in all; each needs one. Translations show this text where they leave a key empty.</p>
      {:else}
        <p class="lead">{total - missing} of {total} keys translated{missing === 0 ? ": complete" : ""}. A key left empty shows the {named(table.default)} text.</p>
      {/if}
    {/if}
  </header>

  {#if error}<p class="problem">{error}</p>{/if}

  {#if table && isDefault}
    <div class="translations">
      {#each others as { l: tag, i } (tag)}
        <button class="language" onclick={() => onopen(tag)}>
          {named(tag)}
          <span class:done={table.missing[i] === 0}>{table.missing[i] === 0 ? "complete" : `${table.missing[i]} missing`}</span>
        </button>
      {/each}
      <button class="language" onclick={onnewlanguage}>+ New language</button>
    </div>
  {/if}

  {#if table && column >= 0}
    <div class="bar">
      <input class="search" type="search" bind:value={filter} placeholder="Find a key or a text" aria-label="Find a key or a text" />
      <label class="check"><input type="checkbox" bind:checked={onlyMissing} /> Missing only</label>
    </div>

    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>Key</th>
            <th>Used in</th>
            {#if !isDefault}<th>{named(table.default)}</th>{/if}
            <th>{named(language)}</th>
            {#if isDefault}
              {#each others as { l: tag } (tag)}<th class="status">{tag}</th>{/each}
            {/if}
          </tr>
        </thead>
        <tbody>
          {#each rows as row (row.key)}
            <tr>
              <td class="key">{row.key}</td>
              <td class="where">{row.places.join(", ")}</td>
              {#if !isDefault}<td class="text">{row.values[0] ?? ""}</td>{/if}
              <td class="value">
                <textarea
                  rows="1"
                  use:grow
                  value={row.values[column] ?? ""}
                  class:missing={row.values[column] === null}
                  placeholder={isDefault ? "Needs a text" : (row.values[0] ?? "")}
                  aria-label={`${row.key} in ${named(language)}`}
                  onchange={(e) => setValue(row.key, e.currentTarget.value)}
                ></textarea>
              </td>
              {#if isDefault}
                {#each others as { l: tag, i } (tag)}
                  <td class="status" class:missing={row.values[i] === null} title={row.values[i] ?? "Not translated yet"}>
                    {row.values[i] === null ? "missing" : "\u2713"}
                  </td>
                {/each}
              {/if}
            </tr>
          {:else}
            <tr><td colspan="4" class="muted">{onlyMissing ? "Nothing is missing." : "No key fits."}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else if table}
    <p class="muted">The project has no file for {named(language)}.</p>
  {/if}
</section>

<style>
  .languages {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
    min-height: 0;
  }

  h2 {
    margin: 0;
  }

  .lead {
    margin: 4px 0 0;
    color: var(--text-muted);
  }

  .translations {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .language {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }

  .language span {
    color: var(--warning, var(--text-muted));
    font-size: 12px;
  }

  .language span.done {
    color: var(--success, var(--text-muted));
  }

  .bar {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
  }

  .search {
    flex: 1;
    max-width: 360px;
  }

  .check {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .table-wrap {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  table {
    border-collapse: collapse;
    width: 100%;
  }

  thead th {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--bg);
  }

  th,
  td {
    text-align: left;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }

  td.key {
    width: 18%;
    padding-top: 8px;
    font-family: var(--mono);
    font-size: 12px;
  }

  td.text {
    width: 30%;
    padding-top: 8px;
    color: var(--text-muted);
  }

  td.where {
    width: 12%;
    padding-top: 9px;
    color: var(--text-muted);
    font-size: 12px;
  }

  th.status,
  td.status {
    width: 80px;
    text-align: center;
  }

  td.status {
    padding-top: 8px;
    color: var(--success, var(--text-muted));
  }

  td.status.missing {
    color: var(--warning, var(--text-muted));
    font-size: 12px;
  }

  textarea {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
    resize: none;
    overflow: hidden;
  }

  textarea.missing {
    border-color: var(--warning, var(--border));
  }

  .muted {
    color: var(--text-muted);
  }

  .problem {
    color: var(--error);
  }
</style>
