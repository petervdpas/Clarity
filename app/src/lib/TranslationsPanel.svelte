<script lang="ts">
  // The project's translations: every text a person sees, with a column per
  // language. The backend collects the texts and writes the files; this
  // shows them and sends what is typed.

  import { invoke } from "@tauri-apps/api/core";
  import type { Preview } from "./generated/Preview";
  import type { TranslationTable } from "./generated/TranslationTable";
  import { languageChoice } from "./i18n.svelte";

  let {
    preview,
    onapplied,
  }: {
    /** The loaded project; the table is read again when it changes. */
    preview: Preview;
    onapplied: (preview: Preview) => void;
  } = $props();

  let table = $state<TranslationTable | null>(null);
  let error = $state<string | null>(null);
  let newLanguage = $state("");
  let onlyMissing = $state(false);

  $effect(() => {
    void preview;
    invoke<TranslationTable>("translation_table")
      .then((t) => {
        table = t;
        error = null;
      })
      .catch((err) => (error = String(err)));
  });

  const rows = $derived((table?.rows ?? []).filter((r) => !onlyMissing || r.translations.some((t) => t === null)));
  const base = $derived(preview.project.locale.language ?? "en");

  async function apply(command: string, args: Record<string, unknown>) {
    try {
      onapplied(await invoke<Preview>(command, args));
      error = null;
    } catch (err) {
      error = String(err);
    }
  }

  function translate(language: string, text: string, translation: string) {
    apply("set_translation", { language, text, translation });
  }

  function addLanguage(e: SubmitEvent) {
    e.preventDefault();
    const language = newLanguage.trim();
    if (!language) return;
    apply("add_language", { language });
    newLanguage = "";
  }

  /** The designer's language: the texts as written, or a translation. */
  function choose(value: string) {
    languageChoice.language = value === "" ? undefined : value === base ? null : value;
  }
</script>

<section>
  <h2>Translations</h2>
  <p class="intro">
    Every text people see, collected from the whole project. The texts are written in {base}; a translation file per language,
    <code>translations/nl.clarity</code>, gives them in another. A text without a translation shows as written, and text with slots keeps
    its slots: <code>{"{customer.name} opgeslagen"}</code>.
  </p>

  {#if error}<p class="problem">{error}</p>{/if}

  <div class="bar">
    <form onsubmit={addLanguage}>
      <input class="mono" bind:value={newLanguage} placeholder="nl" spellcheck="false" aria-label="Language to add" />
      <button type="submit" disabled={!newLanguage.trim()}>Add language</button>
    </form>
    <label class="check"><input type="checkbox" bind:checked={onlyMissing} /> Only texts that miss a translation</label>
    <label>
      Show the designer in
      <select value={languageChoice.language === undefined ? "" : (languageChoice.language ?? base)} onchange={(e) => choose(e.currentTarget.value)}>
        <option value="">the system's language</option>
        <option value={base}>{base} (as written)</option>
        {#each table?.languages ?? [] as language (language)}<option value={language}>{language}</option>{/each}
      </select>
    </label>
  </div>

  {#if table && table.languages.length > 0}
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>Text</th>
            <th>Used in</th>
            {#each table.languages as language, i (language)}
              <th>{language} <span class="muted">{table.missing[i] === 0 ? "complete" : `${table.missing[i]} missing`}</span></th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each rows as row (row.text)}
            <tr>
              <td class="text">{row.text}</td>
              <td class="muted">{row.places.join(", ")}</td>
              {#each table.languages as language, i (language)}
                <td>
                  <input
                    value={row.translations[i] ?? ""}
                    class:missing={row.translations[i] === null}
                    aria-label={`${row.text} in ${language}`}
                    onchange={(e) => translate(language, row.text, e.currentTarget.value)}
                  />
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else if table}
    <p class="muted">No translations yet: add a language to start one.</p>
  {/if}
</section>

<style>
  .bar {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    align-items: center;
  }

  .bar form,
  .bar label {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .bar input.mono {
    width: 80px;
  }

  .table-wrap {
    overflow: auto;
    max-height: 480px;
  }

  table {
    border-collapse: collapse;
    width: 100%;
  }

  th,
  td {
    text-align: left;
    padding: 3px 6px;
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
  }

  td input {
    width: 100%;
    min-width: 160px;
  }

  td input.missing {
    border-color: var(--warning, var(--border));
  }

  .text {
    font-family: var(--mono);
    font-size: 12px;
  }

  .mono {
    font-family: var(--mono);
  }

  .muted {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: normal;
  }

  .problem {
    color: var(--error);
  }
</style>
