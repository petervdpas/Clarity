<script lang="ts">
  // The project's Agents part (docs/ledger/app-mcp.md), opened from the
  // project tree: what the app offers AI agents through its MCP service. The
  // guidance in its four sections (agent/guidance.md, given to an agent when
  // it connects), the commands marked `agent`, and what agents may do while
  // the app is closed until the person sets it in the app's own settings.

  import { invoke } from "@tauri-apps/api/core";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import type { ProjectFile } from "@clarity/ui/generated/ProjectFile.ts";
  import type { Settings } from "@clarity/ui/generated/Settings.ts";
  import type { SectionShown } from "@clarity/ui/generated/SectionShown.ts";
  import { agentCommands, agentSettings, commandRows } from "./agentsPart.ts";
  import { applyEdit } from "./applyEdit.ts";

  let {
    project,
    preview,
    onsave,
    onapplied,
  }: {
    project: ProjectFile;
    preview: Preview;
    onsave: (settings: Settings) => void;
    onapplied: (preview: Preview) => void;
  } = $props();

  const commands = $derived(agentCommands(preview.project.files));
  /** Every command, ticked when it is offered to agents. */
  const rows = $derived(commandRows(preview.project.files));

  /** Offers a command to agents, or stops offering it: its `agent` line. */
  async function setAgent(row: { path: string; item: number }, agent: boolean) {
    const file = preview.project.files.find((f) => f.path === row.path);
    if (!file) return;
    try {
      error = null;
      onapplied((await applyEdit(file, { op: "setCommandAgent", item: row.item, agent })).preview);
    } catch (err) {
      error = String(err);
    }
  }
  /** Which switches the app's own settings offer the person, and where its settings block is. */
  const offered = $derived(agentSettings(preview.project.files));

  /** Adds one of the switches to the app's settings block, as Clarity's own setting. */
  async function addSetting(word: "agentReadsWhileClosed" | "agentChangesWhileClosed") {
    const file = preview.project.files.find((f) => f.path === offered.file);
    if (!file) return;
    try {
      error = null;
      onapplied((await applyEdit(file, { op: "addOwnSetting", word })).preview);
    } catch (err) {
      error = String(err);
    }
  }

  /** The sections as the file has them, read again when the file changes. */
  let sections = $state<SectionShown[]>([]);
  /** What is written in each section, by its key. */
  let texts = $state<Record<string, string>>({});
  let error = $state<string | null>(null);
  $effect(() => {
    void preview.project.guidance;
    invoke<SectionShown[]>("guidance_sections")
      .then((read) => {
        // A section being written keeps what is written; the others follow the file.
        const next: Record<string, string> = {};
        for (const s of read) {
          const before = sections.find((o) => o.key === s.key);
          next[s.key] = before && texts[s.key] !== before.body ? texts[s.key] : s.body;
        }
        sections = read;
        texts = next;
      })
      .catch((err) => (error = String(err)));
  });
  const changed = $derived(sections.filter((s) => (texts[s.key] ?? "") !== s.body));

  async function save() {
    try {
      error = null;
      let last: Preview | null = null;
      for (const s of changed) last = await invoke<Preview>("save_guidance_section", { key: s.key, text: texts[s.key] ?? "" });
      if (last) onapplied(last);
    } catch (err) {
      error = String(err);
    }
  }

  function keydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      if (changed.length > 0) save();
    }
  }

  function setClosed(change: Partial<Pick<Settings, "agents_read_closed" | "agents_change_closed">>) {
    if (project.settings) onsave({ ...project.settings, ...change });
  }

  /** How many agents the running app serves at once without the line, as the backend says. */
  const AT_MOST = $derived(project.catalog.agents_at_most);

  /** The cap, kept only when it differs from the default; 1 to 100. */
  function setAtMost(value: number) {
    if (!project.settings || !Number.isInteger(value) || value < 1 || value > 100) return;
    onsave({ ...project.settings, agents_at_most: value === AT_MOST ? null : value });
  }
</script>

<div class="view">
  <header>
    <h1>Agents</h1>
    <span class="muted">what the app offers AI agents through its MCP service</span>
    <button class="primary" onclick={save} disabled={changed.length === 0} title="Save the guidance (Ctrl+S)">Save</button>
  </header>
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <section>
    <h2>Commands for agents</h2>
    {#if rows.length > 0}
      <ul class="commands">
        {#each rows as row (row.path + row.item)}
          <li>
            <label title={row.agent ? "Offered to agents: untick to stop" : "Tick to offer it to agents"}>
              <input type="checkbox" checked={row.agent} onchange={(e) => setAgent(row, e.currentTarget.checked)} />
              <code>{row.line}</code>
            </label>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="muted">The app has no commands yet.</p>
    {/if}
    <p class="muted">
      A ticked command gets the line <code>agent</code> and is a tool of the app's MCP service; the app serves it itself, so it needs no command-line program. A command for agents says what it does in its <code>help</code>.
      {commands.offered.length === 0 ? "No command is offered to agents yet." : ""}
    </p>
  </section>

  <section>
    <h2>While the app is closed</h2>
    <p class="muted">What agents may do when nobody is there to allow it: where the app's settings <code>agentReadsWhileClosed</code> and <code>agentChangesWhileClosed</code> start, until the person using the app sets them.</p>
    {#if project.settings}
      <label><input type="checkbox" checked={project.settings.agents_read_closed} onchange={(e) => setClosed({ agents_read_closed: e.currentTarget.checked })} /> Agents may read</label>
      <label><input type="checkbox" checked={project.settings.agents_change_closed} onchange={(e) => setClosed({ agents_change_closed: e.currentTarget.checked })} /> Agents may change</label>
    {/if}
    <p class="muted">The person changes them in the app's own settings, which offer them as Clarity's own settings; put those in the app's settings window.</p>
    <ul class="offered">
      <li>
        <code>agentReadsWhileClosed</code>
        {#if offered.reads}<span class="muted">in the app's settings</span>{:else}<button onclick={() => addSetting("agentReadsWhileClosed")}>Add to the app's settings</button>{/if}
      </li>
      <li>
        <code>agentChangesWhileClosed</code>
        {#if offered.changes}<span class="muted">in the app's settings</span>{:else}<button onclick={() => addSetting("agentChangesWhileClosed")}>Add to the app's settings</button>{/if}
      </li>
    </ul>
  </section>

  <section>
    <h2>At once</h2>
    <p class="muted">How many AI agents the running app serves at the same time. One more is refused, and the app's window says so.</p>
    {#if project.settings}
      <label class="most">At most <input type="number" min="1" max="100" value={project.settings.agents_at_most ?? AT_MOST} onchange={(e) => setAtMost(e.currentTarget.valueAsNumber)} /> agents</label>
    {/if}
  </section>

  <section class="guidance">
    <h2>Guidance <span class="muted">agent/guidance.md, given to an agent when it connects</span></h2>
    {#if preview.project.guidance === null}<p class="muted">No guidance yet: write a section and save to start the file.</p>{/if}
    {#each sections as s (s.key)}
      <label class="section">
        <span class="heading">{s.heading}</span>
        <textarea bind:value={texts[s.key]} onkeydown={keydown} placeholder={s.asks} spellcheck="true" rows="4"></textarea>
      </label>
    {/each}
  </section>
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px 24px;
    overflow: auto;
    height: 100%;
    box-sizing: border-box;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  header .muted {
    flex: 1;
  }
  h1 {
    margin: 0;
    font-size: 18px;
  }
  h2 {
    margin: 0 0 6px;
    font-size: 14px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
    margin: 0;
  }
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--error);
  }
  .commands {
    margin: 0;
    padding-left: 18px;
    font-size: 13px;
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .heading {
    font-size: 13px;
    font-weight: 600;
  }
  .offered {
    margin: 0;
    padding-left: 18px;
    font-size: 13px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .offered li {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .most input {
    width: 5em;
  }
  textarea {
    resize: vertical;
    font-family: var(--mono);
    font-size: 13px;
    padding: 8px;
  }
</style>
