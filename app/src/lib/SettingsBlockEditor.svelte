<script lang="ts">
  // The app's settings (the `settings` block of a module, DESIGN 5.17), by
  // hand: Clarity's own settings ticked on or off, and the app's own
  // settings as fields, with the types and modifiers a setting takes.

  import EntityEditor from "./EntityEditor.svelte";
  import { applyEdit } from "./applyEdit.ts";
  import type { Edit } from "@clarity/ui/generated/Edit.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";

  let {
    preview,
    onapplied,
  }: {
    preview: Preview;
    onapplied: (preview: Preview) => void;
  } = $props();

  /** The file with the settings block; the first module while there is none, where a first setting starts it. */
  const holder = $derived(preview.project.files.find((f) => f.entities.some((e) => e.settings)) ?? preview.project.files[0]);
  const block = $derived(holder?.entities.find((e) => e.settings) ?? null);

  const OWN = [
    { word: "language", label: "Language", help: "The language shown; the person picks one of the project's languages." },
    { word: "theme", label: "Theme", help: "The theme shown; the person picks one of the themes the project offers." },
    { word: "agentReadsWhileClosed", label: "Agents may read while the app is closed", help: "For an app with commands for agents: the person allows reads through its MCP service while it is closed." },
    { word: "agentChangesWhileClosed", label: "Agents may change while the app is closed", help: "For an app with commands for agents: the person allows changes through its MCP service while it is closed." },
  ] as const;

  let error = $state<string | null>(null);

  async function edit(change: Edit) {
    if (!holder) return;
    try {
      error = null;
      onapplied((await applyEdit(holder, change)).preview);
    } catch (err) {
      error = String(err);
    }
  }

  /** Ticks one of Clarity's own settings on, or takes it away. */
  function setOwn(word: string, on: boolean) {
    const there = block?.fields.find((f) => f.name.name === word);
    if (on && !there) edit({ op: "addOwnSetting", word });
    if (!on && there) edit({ op: "remove", item: there.span.start });
  }
</script>

<div class="view">
  <header>
    <h1>Settings</h1>
    <span class="muted">what each person using the app sets for themselves{holder?.module ? `, in module ${holder.module.name}` : ""}</span>
  </header>
  {#if error}<p class="problem" role="alert">{error}</p>{/if}

  <section>
    <h2>Clarity's own</h2>
    {#each OWN as own (own.word)}
      <label class="own" title={own.help}>
        <input type="checkbox" checked={block?.fields.some((f) => f.name.name === own.word) ?? false} onchange={(e) => setOwn(own.word, e.currentTarget.checked)} />
        {own.label} <code>{own.word}</code>
      </label>
    {/each}
  </section>

  <section>
    <h2>The app's own</h2>
    <EntityEditor {preview} entity={block} module={holder?.module?.name ?? ""} onedit={edit} {onapplied} stores={[]} settings />
  </section>
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  header {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }

  h1 {
    font-size: 18px;
    margin: 0;
  }

  h2 {
    font-size: 14px;
    margin: 0 0 8px;
  }

  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }

  .problem {
    color: var(--error);
    margin: 0;
  }

  .own {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
  }
</style>
