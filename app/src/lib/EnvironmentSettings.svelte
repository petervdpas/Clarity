<script lang="ts">
  // The Environment section of Clarity settings: the build tools of the five
  // stacks on this computer, for every project. A scan finds every copy of
  // each tool with its version; the person picks the one builds use, or
  // types a path, or leaves it to Clarity (the first one found).

  import { invoke } from "./backend";
  import type { EnvironmentView } from "./generated/EnvironmentView";
  import type { ToolView } from "./generated/ToolView";
  import { checkTools, openSetupGuide } from "./tools.svelte";

  let view = $state<EnvironmentView | null>(null);
  let scanning = $state(false);
  let error = $state<string | null>(null);
  /** A path being typed, by tool. */
  let typed = $state<Record<string, string>>({});

  /** The choice that leaves the tool to Clarity: the first copy found. */
  const AUTOMATIC = "";

  async function scan() {
    scanning = true;
    error = null;
    try {
      view = await invoke<EnvironmentView>("environment_view");
    } catch (err) {
      error = String(err);
    } finally {
      scanning = false;
    }
  }

  async function choose(tool: ToolView, path: string | null) {
    error = null;
    try {
      view = await invoke<EnvironmentView>("choose_tool", { program: tool.program, path });
      typed[tool.program] = "";
      // The project's Build tools row follows the choice.
      checkTools(true);
    } catch (err) {
      error = String(err);
    }
  }

  $effect(() => {
    void scan();
  });
</script>

<div class="environment">
  <div class="bar">
    <p class="muted">Where the build tools of the five stacks are on this computer. Build and Run use the one chosen for each tool, else the first one found, in every project.</p>
    <button onclick={scan} disabled={scanning}>{scanning ? "Scanning..." : "Scan again"}</button>
  </div>
  {#if error}<p class="refused" role="alert">{error}</p>{/if}

  {#if view}
    <fieldset>
      <legend>Stacks</legend>
      <ul class="stacks">
        {#each view.stacks as s (s.name)}
          <li class:ready={s.ready}>
            <strong>{s.name}</strong>
            {#if s.ready}
              <span class="state ok">Ready</span>
              <span class="muted">{s.found.join(", ")}</span>
            {:else}
              <span class="state missing">Missing</span>
              <ul class="needs">
                {#each s.missing as need (need.what)}
                  <li>
                    <span class="muted">{need.what}</span>
                    <button type="button" onclick={() => openSetupGuide(need.guide)} title="Opens the manual where it tells how to install this">How to install</button>
                  </li>
                {/each}
              </ul>
            {/if}
          </li>
        {/each}
      </ul>
    </fieldset>

    <fieldset>
      <legend>Tools</legend>
      {#each view.tools as tool (tool.program)}
        <div class="tool">
          <div class="head">
            <strong>{tool.name}</strong>
            <span class="muted">for {tool.stacks.join(", ")}</span>
          </div>
          <select
            aria-label="The {tool.name} builds use"
            value={tool.chosen ?? AUTOMATIC}
            onchange={(e) => choose(tool, e.currentTarget.value === AUTOMATIC ? null : e.currentTarget.value)}
          >
            <option value={AUTOMATIC}>{tool.copies.length > 0 ? "The first one found" : "Not found"}</option>
            {#each tool.copies as copy (copy.path)}
              <option value={copy.path}>{copy.path}{copy.version ? ` (${copy.version})` : " (does not answer)"}</option>
            {/each}
          </select>
          <div class="other">
            <input
              type="text"
              placeholder="Or type the program's path"
              aria-label="Path of {tool.name}"
              bind:value={typed[tool.program]}
              onkeydown={(e) => e.key === "Enter" && typed[tool.program]?.trim() && choose(tool, typed[tool.program])}
            />
            <button disabled={!typed[tool.program]?.trim()} onclick={() => choose(tool, typed[tool.program])}>Use</button>
          </div>
          <p class="muted used">
            {#if tool.used}Builds use <code>{tool.used}</code>{tool.chosen ? ", as chosen" : ""}.{:else}Not on this computer; the stacks that need it cannot build here.{/if}
          </p>
        </div>
      {/each}
    </fieldset>
    <p class="muted">Kept in <code>{view.file}</code>.</p>
  {:else if scanning}
    <p class="muted">Looking for the tools...</p>
  {/if}
</div>

<style>
  .environment {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .bar {
    display: flex;
    align-items: flex-start;
    gap: 12px;
  }
  .bar p {
    flex: 1;
    margin: 0;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  .refused {
    color: var(--error);
    margin: 0;
  }
  fieldset {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 12px;
    margin: 0;
  }
  legend {
    font-size: 12px;
    color: var(--text-muted);
  }
  .stacks {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .stacks li {
    display: grid;
    grid-template-columns: 200px auto 1fr;
    gap: 10px;
    align-items: baseline;
  }
  .needs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .needs li {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .needs .muted {
    flex: 1;
  }
  .state {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 8px;
  }
  .ok {
    background: color-mix(in srgb, var(--success, #2e8b57) 25%, transparent);
  }
  .missing {
    background: color-mix(in srgb, var(--error) 25%, transparent);
  }
  .tool {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }
  .tool:last-child {
    border-bottom: none;
  }
  .head {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  .tool select {
    max-width: 100%;
  }
  .other {
    display: flex;
    gap: 6px;
  }
  .other input {
    flex: 1;
    font-family: var(--mono);
  }
  .used {
    margin: 0;
  }
</style>
