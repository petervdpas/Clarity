<script lang="ts">
  // One theme: its text in Clarity, written in place, and a sample drawn with
  // it. Clarity's own theme is shown read only; it is the base every other
  // theme changes.

  import { invoke } from "@tauri-apps/api/core";
  import SourceEditor from "./SourceEditor.svelte";
  import ThemeSample from "./ThemeSample.svelte";
  import type { CodeView } from "./generated/CodeView";
  import type { DeclTrial } from "./generated/DeclTrial";
  import type { Edit } from "./generated/Edit";
  import type { Preview } from "./generated/Preview";
  import type { SpecFile } from "./generated/SpecFile";
  import type { ThemeDecl } from "./generated/ThemeDecl";

  let {
    preview,
    theme,
    file,
    onapply,
    onopensettings,
  }: {
    preview: Preview;
    theme: ThemeDecl;
    /** The module that declares the theme; null for Clarity's own. */
    file: SpecFile | null;
    /** Applies an edit and throws with the reason when it is refused. */
    onapply: (edit: Edit) => Promise<void>;
    /** Opens the project settings, where the offered themes are chosen. */
    onopensettings: () => void;
  } = $props();

  const name = $derived(theme.name.name);
  const offered = $derived(preview.project.themes.includes(name));
  const first = $derived(preview.project.themes[0] === name);

  /** Clarity's own theme as written. */
  let baseText = $state<string | null>(null);
  $effect(() => {
    if (!file && baseText === null) invoke<string>("theme_starter", { name }).then((text) => (baseText = text));
  });
</script>

<div class="view">
  <header>
    <h2>{name}</h2>
    {#if file}
      <span class="muted">theme in module {file.module?.name ?? ""}, file <code>{file.path}</code></span>
    {:else}
      <span class="muted">Clarity's own theme, built in: the base every theme changes</span>
    {/if}
  </header>

  <p class="muted">
    {#if !file}
      Every value a theme can set: colors for light and dark mode, fonts, sizes, corner radii, spacing, borders, a shadow, and the look
      of each kind of part in its states. A theme only writes what it changes; New theme starts from a full copy of this one.
    {:else if offered}
      The application offers it{first ? " and starts with it" : ""}.
      <button class="link" onclick={onopensettings}>Change in the project settings</button>
    {:else}
      The application does not offer it yet.
      <button class="link" onclick={onopensettings}>Offer it in the project settings</button>
    {/if}
  </p>

  {#if file}
    {@const item = theme.span.start}
    <div class="editor">
      <SourceEditor
        load={() => invoke<CodeView>("decl_text", { path: file.path, item })}
        check={async (text) => (await invoke<DeclTrial>("try_decl", { path: file.path, item, text })).problems}
        save={(text) => onapply({ op: "replaceDecl", item, text })}
        reloadKey={file.path + "\n" + item + "\n" + file.version}
      />
    </div>
  {:else if baseText !== null}
    <pre class="base">{baseText}</pre>
  {/if}

  <ThemeSample {theme} />
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  h2 {
    margin: 0 0 2px;
    font-size: 18px;
  }

  .muted {
    color: var(--text-muted);
    font-size: 12px;
    margin: 0;
  }

  code {
    font-family: var(--mono);
  }

  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    font-size: 12px;
    cursor: pointer;
  }

  .editor {
    display: flex;
    height: 460px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .base {
    font-family: var(--mono);
    font-size: 12px;
    max-height: 460px;
    overflow: auto;
    margin: 0;
    background: var(--panel-alt);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 8px 10px;
  }
</style>
