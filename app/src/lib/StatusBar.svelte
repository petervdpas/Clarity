<script lang="ts">
  // The status bar at the foot of Clarity: what is open, how the build is
  // doing, the project's errors and warnings, its stack and the folder
  // builds write to, and the computer's load as three meters (cpu, mem,
  // io) that open Clarity settings, System. While an outside agent (Claude
  // Code, Codex) works through `clarity mcp`, its last call shows, green when
  // it went through and red when refused, and opens the agent activity. Each
  // part that leads somewhere is a button.

  import type { Selected } from "./selection";
  import type { AgentActivity } from "@clarity/ui/generated/AgentActivity.ts";
  import { invoke } from "@clarity/ui/host/backend.ts";
  import { agentCall, buildStatus, counted, openLabel } from "./statusBar";
  import { bytes, ioLevel, percent } from "./systemLoad";
  import { systemLoad } from "./systemLoad.svelte";

  let {
    open,
    file,
    building,
    commandRunning,
    running,
    done,
    errors,
    warnings,
    stack,
    appFolder,
    onoutput,
    oncheck,
    onsettings,
    onsystem,
    onactivity,
  }: {
    open: Selected | null;
    /** The open project file; none before a project is open. */
    file: string | null;
    building: boolean;
    commandRunning: boolean;
    running: boolean;
    done: { ok: boolean; message: string } | null;
    errors: number;
    warnings: number;
    stack: string;
    appFolder: string;
    onoutput: () => void;
    oncheck: () => void;
    onsettings: () => void;
    onsystem: () => void;
    onactivity: () => void;
  } = $props();

  /** How often the bar asks for outside agents' new calls. */
  const FOLLOW_MS = 1500;
  let lastCall = $state<AgentActivity | null>(null);
  let now = $state(Date.now() / 1000);
  const agent = $derived(agentCall(lastCall, now));

  $effect(() => {
    let after = 0;
    async function follow() {
      now = Date.now() / 1000;
      try {
        const fresh = await invoke<AgentActivity[]>("agent_activity", { after });
        if (fresh.length > 0) {
          lastCall = fresh[fresh.length - 1];
          after = lastCall.seq;
        }
      } catch {
        // The activity is a convenience: a failed read leaves the bar as it was.
      }
    }
    void follow();
    const timer = setInterval(() => void follow(), FOLLOW_MS);
    return () => clearInterval(timer);
  });

  const build = $derived(buildStatus({ building, commandRunning, running, done }));

  /** The three meters, from the last reading: how full each is, and what it says on hover. */
  const meters = $derived.by(() => {
    const r = systemLoad.last;
    if (!r) return null;
    const memory = r.memory_total > 0 ? r.memory_used / r.memory_total : 0;
    return [
      { label: "cpu", level: r.cpu / 100, title: `Processors ${percent(r.cpu)} in use` },
      { label: "mem", level: memory, title: `Memory ${bytes(r.memory_used)} of ${bytes(r.memory_total)} in use` },
      { label: "io", level: ioLevel(Math.max(r.read, r.written)), title: `Disk: reading ${bytes(r.read)}/s, writing ${bytes(r.written)}/s` },
    ];
  });
</script>

<footer class="status-bar" aria-label="Status">
  {#if file}
    <span class="item open">{openLabel(open)}</span>
    <span class="item path" title={file}>{file}</span>
    <span class="spacer"></span>
    <button class="item build {build.tone}" title="Show the build output" onclick={onoutput}>{build.text}</button>
    <button class="item problems" title="Read every file again and check the whole project" onclick={oncheck}>
      <span class:error={errors > 0}>{counted(errors, "error", "errors")}</span>,
      <span class:warning={warnings > 0}>{counted(warnings, "warning", "warnings")}</span>
    </button>
    {#if agent}
      <button class="item agent" class:refused={!agent.ok} title={agent.title} onclick={onactivity}>
        <span class="dot" aria-hidden="true"></span>{agent.text}
      </button>
    {/if}
    <button class="item" title="The project's stack; opens the project settings" onclick={onsettings}>{stack}</button>
    {#if appFolder}
      <button class="item path" title="Builds write to {appFolder}; opens the project settings" onclick={onsettings}>{appFolder}</button>
    {/if}
  {:else}
    <span class="item">No project open</span>
    <span class="spacer"></span>
  {/if}
  {#if meters}
    <button class="item meters" title="The computer's load; opens Clarity settings, System" onclick={onsystem}>
      {#each meters as m (m.label)}
        <span class="meter" title={m.title}>
          {m.label}
          <span class="bar" aria-hidden="true"><span class="fill" class:high={m.level > 0.85} style:height="{Math.round(m.level * 100)}%"></span></span>
        </span>
      {/each}
    </button>
  {/if}
</footer>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    height: 32px;
    padding: 0 10px;
    background: var(--panel-alt);
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 13px;
    white-space: nowrap;
  }
  /* Only the two paths shorten when the bar is narrow; the rest stays whole. */
  .item {
    flex-shrink: 0;
    padding: 0 10px;
    line-height: 30px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .item + .item {
    border-left: 1px solid var(--border);
  }
  .spacer {
    flex: 1;
  }
  button.item {
    border: none;
    border-radius: 0;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  button.item:hover {
    background: var(--accent-soft);
    color: var(--text);
  }
  .open {
    color: var(--text);
    flex-shrink: 0;
  }
  .path {
    font-family: var(--mono);
    min-width: 0;
    flex-shrink: 1;
  }
  /* The project file takes the room there is; the app folder at the right keeps to a width. */
  button.path {
    max-width: 360px;
  }
  .build {
    max-width: 320px;
  }
  .busy {
    color: var(--accent);
  }
  .failed,
  .error {
    color: var(--error);
  }
  .warning {
    color: var(--warning);
  }
  .agent {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
  }
  .agent.refused .dot {
    background: var(--error);
  }
  .meters {
    display: flex;
    gap: 10px;
    flex-shrink: 0;
  }
  .meter {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
  }
  .bar {
    position: relative;
    width: 6px;
    height: 16px;
    background: var(--border);
    border-radius: 1px;
    overflow: hidden;
  }
  .fill {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--success, #2e8b57);
  }
  .fill.high {
    background: var(--error);
  }
</style>
