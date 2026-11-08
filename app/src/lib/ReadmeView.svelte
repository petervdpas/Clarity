<script lang="ts">
  // The project's README.md, opened from the top of the project tree: its
  // design goal and a guide for whoever works on it next, written in
  // Markdown with how it reads beside it. The agent reads it first. A
  // project without one starts from headings to fill in.

  import { invoke } from "@tauri-apps/api/core";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import MarkdownView from "@clarity/ui/parts/MarkdownView.svelte";
  import { openLink } from "@clarity/ui/host/backend.ts";

  let { preview, onapplied }: { preview: Preview; onapplied: (preview: Preview) => void } = $props();

  const saved = $derived(preview.project.readme ?? null);
  /** What a project without a README starts from. */
  const starter = $derived(`# ${preview.project.files.find((f) => f.apps.length > 0)?.apps[0]?.name.name ?? "Project"}\n\n## Goal\n\n## Scope\n\n## Order of work\n`);

  /** The text being written; it starts as the file is, and again when the file changes on disk unedited. */
  let text = $state("");
  let shown = $state<string | null | undefined>(undefined);
  $effect.pre(() => {
    if (saved !== shown) {
      if (shown === undefined || text === (shown ?? starter)) text = saved ?? starter;
      shown = saved;
    }
  });
  const changed = $derived(text !== (saved ?? ""));
  let error = $state<string | null>(null);

  async function save() {
    try {
      error = null;
      onapplied(await invoke<Preview>("save_readme", { text }));
    } catch (err) {
      error = String(err);
    }
  }

  function keydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      if (changed) save();
    }
  }
</script>

<section class="readme">
  <header>
    <h2>README.md</h2>
    <span class="kind">The project's design goal and guide; the agent reads it first</span>
    <button class="primary" onclick={save} disabled={!changed} title="Save the file (Ctrl+S)">Save</button>
  </header>
  {#if saved === null}<p class="note">No README yet: fill in the headings and save to write README.md beside the modules.</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="sides">
    <textarea bind:value={text} onkeydown={keydown} spellcheck="true" aria-label="Markdown of README.md"></textarea>
    <div class="shown"><MarkdownView {text} onlink={openLink} /></div>
  </div>
</section>

<style>
  .readme {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 16px;
    height: 100%;
    min-height: 0;
    box-sizing: border-box;
  }

  header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  h2 {
    margin: 0;
    font-size: 15px;
  }

  .kind {
    flex: 1;
    color: var(--text-muted);
    font-size: 12px;
  }

  .note,
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .error {
    color: var(--error);
  }

  .sides {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  textarea {
    resize: none;
    font-family: var(--mono);
    font-size: 13px;
    padding: 8px;
    min-height: 0;
  }

  .shown {
    min-height: 0;
    display: flex;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--panel);
  }
</style>
