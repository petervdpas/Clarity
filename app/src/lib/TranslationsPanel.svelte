<script lang="ts">
  // A language of the project, opened from i18n in the project tree. A
  // translation lists every text people see with its translation, to type
  // in place. The language the texts are written in gives an overview: each
  // text with how far each translation is. The backend collects the texts
  // and writes the translation files.

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
    /** The language opened: a translation, or the one the texts are written in. */
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

  /** The column of the opened translation; -1 for the language the texts are written in. */
  const column = $derived(table?.languages.indexOf(language) ?? -1);
  const total = $derived(table?.rows.length ?? 0);

  const rows = $derived(
    (table?.rows ?? []).filter((r) => {
      const words = filter.trim().toLocaleLowerCase();
      const translated = column >= 0 ? (r.translations[column] ?? "") : "";
      if (words && !r.text.toLocaleLowerCase().includes(words) && !translated.toLocaleLowerCase().includes(words)) return false;
      if (!onlyMissing) return true;
      return column >= 0 ? r.translations[column] === null : r.translations.some((t) => t === null);
    }),
  );

  /** A language as people know it: its own name and its tag. */
  function named(tag: string): string {
    const name = languageName(tag);
    return name === tag ? tag : `${name} (${tag})`;
  }

  async function translate(text: string, translation: string) {
    try {
      onapplied(await invoke<Preview>("set_translation", { language, text, translation }));
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
    <h2>{named(language)}</h2>
    {#if table}
      {#if column >= 0}
        {@const missing = table.missing[column]}
        <p class="lead">{total - missing} of {total} texts translated{missing === 0 ? ": complete" : ""}. Type a translation next to its text; one left empty shows as written.</p>
      {:else}
        <p class="lead">The texts as the project writes them, {total} in all. Change a text where it is used; open a translation to translate them.</p>
      {/if}
    {/if}
  </header>

  {#if error}<p class="problem">{error}</p>{/if}

  {#if table && column < 0}
    <div class="translations">
      {#each table.languages as tag, i (tag)}
        <button class="language" onclick={() => onopen(tag)}>
          {named(tag)}
          <span class:done={table.missing[i] === 0}>{table.missing[i] === 0 ? "complete" : `${table.missing[i]} missing`}</span>
        </button>
      {/each}
      <button class="language" onclick={onnewlanguage}>+ New language</button>
    </div>
  {/if}

  {#if table}
    <div class="bar">
      <input class="search" type="search" bind:value={filter} placeholder="Find a text" aria-label="Find a text" />
      {#if column >= 0 || table.languages.length > 0}
        <label class="check"><input type="checkbox" bind:checked={onlyMissing} /> Missing only</label>
      {/if}
    </div>

    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>Text</th>
            <th>Used in</th>
            {#if column >= 0}
              <th>{named(language)}</th>
            {:else}
              {#each table.languages as tag (tag)}<th class="status">{tag}</th>{/each}
            {/if}
          </tr>
        </thead>
        <tbody>
          {#each rows as row (row.text)}
            <tr>
              <td class="text">{row.text}</td>
              <td class="where">{row.places.join(", ")}</td>
              {#if column >= 0}
                <td class="translation">
                  <textarea
                    rows="1"
                    use:grow
                    value={row.translations[column] ?? ""}
                    class:missing={row.translations[column] === null}
                    placeholder={row.text}
                    aria-label={`${row.text} in ${named(language)}`}
                    onchange={(e) => translate(row.text, e.currentTarget.value)}
                  ></textarea>
                </td>
              {:else}
                {#each table.languages as tag, i (tag)}
                  <td class="status" class:missing={row.translations[i] === null} title={row.translations[i] ?? "Not translated yet"}>
                    {row.translations[i] === null ? "missing" : "✓"}
                  </td>
                {/each}
              {/if}
            </tr>
          {:else}
            <tr><td colspan="3" class="muted">{onlyMissing ? "Nothing is missing." : "No text fits."}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
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

  td.text {
    width: 40%;
    padding-top: 8px;
  }

  td.where {
    width: 15%;
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
