<script lang="ts">
  // Outside agents in Clarity settings, Agent (docs/ACP-PLAN.md): whether an
  // agent program the person added Clarity to (Claude Code, Codex, Copilot
  // through `clarity mcp`) may work on the open project, and what those
  // agents did, call by call, as it happens. The list is read only while
  // this section is open.

  import { invoke } from "@clarity/ui/host/backend.ts";
  import type { AgentActivity } from "@clarity/ui/generated/AgentActivity.ts";

  /** How often the list looks for new calls while it is shown. */
  const FOLLOW_MS = 1500;
  /** The calls the list keeps, newest first. */
  const SHOWN = 200;

  let allowed = $state<boolean | null>(null);
  let calls = $state<AgentActivity[]>([]);
  let error = $state<string | null>(null);
  let last = 0;

  async function load() {
    try {
      allowed = await invoke<boolean>("outside_agents");
    } catch (err) {
      error = String(err);
    }
  }

  async function set(value: boolean) {
    error = null;
    try {
      allowed = await invoke<boolean>("set_outside_agents", { allowed: value });
    } catch (err) {
      error = String(err);
    }
  }

  async function follow() {
    try {
      const fresh = await invoke<AgentActivity[]>("agent_activity", { after: last });
      if (fresh.length > 0) {
        last = fresh[fresh.length - 1].seq;
        calls = [...fresh.reverse(), ...calls].slice(0, SHOWN);
      }
    } catch (err) {
      error = String(err);
    }
  }

  function when(seconds: number): string {
    return new Date(seconds * 1000).toLocaleTimeString();
  }

  $effect(() => {
    void load();
    void follow();
    const timer = setInterval(() => void follow(), FOLLOW_MS);
    return () => clearInterval(timer);
  });
</script>

<div class="outside">
  {#if error}<p class="refused" role="alert">{error}</p>{/if}
  <fieldset>
    <legend>Outside agents</legend>
    <label class="check">
      <input type="checkbox" checked={allowed ?? true} disabled={allowed === null} onchange={(e) => set(e.currentTarget.checked)} />
      <span>Let outside agents work on my project</span>
    </label>
    <p class="muted">
      An agent program you added Clarity to (Claude Code, Codex or Copilot, with <code>clarity mcp</code>) works on the project open here. Turned off, it can
      still read Clarity's manual and references, and nothing else. Holds from its next call.
    </p>
  </fieldset>

  <fieldset>
    <legend>Agent activity</legend>
    {#if calls.length === 0}
      <p class="muted">No calls yet. Each call an outside agent makes shows here as it happens.</p>
    {:else}
      <ul aria-label="Calls of outside agents, newest first">
        {#each calls as call (call.seq)}
          <li class:failed={!call.ok}>
            <span class="time">{when(call.time)}</span>
            <span class="client">{call.client || "an agent"}</span>
            <strong>{call.tool}</strong>
            {#if call.path}<code>{call.path}</code>{/if}
            <span class="said" title={call.said}>{call.ok ? "" : "failed: "}{call.said}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </fieldset>
</div>

<style>
  .outside {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  p {
    margin: 0;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  .refused {
    color: var(--error);
  }
  fieldset {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 12px;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  legend {
    font-size: 12px;
    color: var(--text-muted);
  }
  .check {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 320px;
    overflow: auto;
    font-size: 12px;
  }
  li {
    display: flex;
    gap: 8px;
    align-items: baseline;
    padding: 2px 0;
    border-bottom: 1px solid var(--border);
    min-width: 0;
  }
  li.failed .said {
    color: var(--error);
  }
  .time,
  .client {
    color: var(--text-muted);
    flex: none;
  }
  .said {
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  code {
    font-family: var(--mono);
  }
</style>
