<script lang="ts">
  // Clarity's own settings, kept for the person rather than in a project: a
  // view in the main window with its sections in a sidebar, Display (the
  // theme), Keys (Clarity's shortcut keys), Folders (where projects and apps go), Environment (where each
  // stack's build tools are), Agent (what it may run) and System (the
  // computer's load, loaded only when it is opened). A choice shows at once.

  import { RUNS_CHOICES } from "./agent";
  import EnvironmentSettings from "./EnvironmentSettings.svelte";
  import FoldersSettings from "./FoldersSettings.svelte";
  import KeysSettings from "./KeysSettings.svelte";
  import { appSettings, type AppSettingsSection } from "./appSettings.svelte";
  import { agent } from "./agentStore.svelte";
  import { IDE_THEMES, type IdeTheme } from "./ideTheme";
  import { ideTheme, setIdeTheme } from "./ideTheme.svelte";

  let { onclose }: { onclose: () => void } = $props();

  type Section = AppSettingsSection;

  /** The sections, in the sidebar's order. */
  const SECTIONS: { key: Section; name: string; note: string }[] = [
    { key: "display", name: "Display", note: "Clarity's theme and its colors" },
    { key: "keys", name: "Keys", note: "Clarity's shortcut keys, as you want them" },
    { key: "folders", name: "Folders", note: "Where projects and their apps go" },
    { key: "environment", name: "Environment", note: "Where each stack's build tools are" },
    { key: "agent", name: "Agent", note: "What the agent may run" },
    { key: "system", name: "System", note: "How busy the computer is" },
  ];

  // The section shown, kept while Clarity runs, so the settings open where they were left.
  const section = $derived(appSettings.section);

  /** A strip of each theme's colors: its binding, its pages and its accent. */
  const SWATCHES: Record<IdeTheme, string[]> = {
    system: ["#20242c", "#ffffff", "#1d2129", "#1f5fbf"],
    light: ["#20242c", "#eef0f3", "#ffffff", "#1f5fbf"],
    dark: ["#0d0f13", "#15181e", "#1d2129", "#6fa3f2"],
    "cream-green": ["#23402b", "#efe9d4", "#fbf8ee", "#2e6b3a"],
  };
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<section class="settings" aria-labelledby="app-settings-title">
  <nav aria-label="Sections of Clarity's settings">
    <h2 id="app-settings-title">Clarity settings</h2>
    <p class="muted">For you on this computer, in every project.</p>
    <ul>
      {#each SECTIONS as s (s.key)}
        <li>
          <button class:active={section === s.key} aria-current={section === s.key ? "page" : undefined} onclick={() => (appSettings.section = s.key)}>
            <strong>{s.name}</strong>
            <span class="muted">{s.note}</span>
          </button>
        </li>
      {/each}
    </ul>
  </nav>

  <div class="page">
    <header>
      <h3>{SECTIONS.find((s) => s.key === section)?.name}</h3>
      <button class="primary" onclick={onclose} title="Back to the project (Escape)">Close</button>
    </header>

    {#if section === "display"}
      <fieldset>
        <legend>Theme</legend>
        {#each IDE_THEMES as t (t.value)}
          <label class="choice" class:chosen={ideTheme.value === t.value}>
            <input type="radio" name="ide-theme" checked={ideTheme.value === t.value} onchange={() => setIdeTheme(t.value)} />
            <span class="swatch" aria-hidden="true">
              {#each SWATCHES[t.value] as color (color)}<span style:background={color}></span>{/each}
            </span>
            <span class="words">
              <strong>{t.label}</strong>
              <span class="muted">{t.note}</span>
            </span>
          </label>
        {/each}
      </fieldset>
    {:else if section === "keys"}
      <KeysSettings />
    {:else if section === "folders"}
      <FoldersSettings />
    {:else if section === "system"}
      <!-- Loaded when first opened: the graphs cost nothing until someone looks. -->
      {#await import("./SystemSettings.svelte") then page}
        <page.default />
      {/await}
    {:else if section === "environment"}
      <EnvironmentSettings />
    {:else}
      <fieldset>
        <legend>Running and building</legend>
        {#each RUNS_CHOICES as c (c.value)}
          <label class="choice" class:chosen={agent.runs === c.value}>
            <input type="radio" name="agent-runs" checked={agent.runs === c.value} onchange={() => agent.setRuns(c.value)} />
            <span class="words">
              <strong>{c.label}</strong>
              <span class="muted">{c.note}</span>
            </span>
          </label>
        {/each}
      </fieldset>
      <p class="muted">The agent runs nothing else: it reads and writes the project, and one Undo takes a whole answer back. A choice holds from the next message.</p>
    {/if}
  </div>
</section>

<style>
  .settings {
    display: grid;
    grid-template-columns: 260px minmax(0, 1fr);
    min-height: 0;
    background: var(--bg);
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 10px;
    background: var(--panel);
    border-right: 1px solid var(--border);
    overflow: auto;
  }

  h2 {
    margin: 0 4px;
    font-size: 15px;
  }

  nav > .muted {
    margin: 0 4px 8px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  nav button {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 7px 10px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  nav button.active {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .page {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px 18px;
    overflow: auto;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h3 {
    margin: 0;
    font-size: 14px;
  }

  .muted {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  fieldset {
    border: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  legend {
    font-size: 12px;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .choice {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
  }

  .choice.chosen {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .swatch {
    display: flex;
    flex-shrink: 0;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    overflow: hidden;
  }

  .swatch span {
    width: 14px;
    height: 26px;
  }

  .words {
    display: flex;
    flex-direction: column;
  }
</style>
