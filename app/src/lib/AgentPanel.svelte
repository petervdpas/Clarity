<script lang="ts">
  // The agent: a conversation with a model that works on the open project.
  // It sits right of every tab; what it changes is one Undo step per turn.
  import { toolLine } from "./agent";
  import { agent } from "./agentStore.svelte";

  let text = $state("");
  let list: HTMLElement | undefined = $state();

  // The list follows the newest line.
  $effect(() => {
    if (agent.items.at(-1) && list) list.scrollTop = list.scrollHeight;
  });

  $effect(() => {
    agent.loadConnections();
  });

  async function send() {
    if (await agent.send(text)) text = "";
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      send();
    }
  }
</script>

<section class="agent" aria-label="Agent">
  <header>
    <strong>Agent</strong>
    <select aria-label="Connection" value={agent.connection} onchange={(e) => agent.pick(e.currentTarget.value)} disabled={agent.running}>
      {#each agent.connections as name (name)}
        <option value={name}>{name}</option>
      {:else}
        <option value={agent.connection}>{agent.connection}</option>
      {/each}
    </select>
    <button onclick={() => agent.forget()} disabled={agent.running || agent.items.length === 0} title="Start a new conversation">New</button>
    <button onclick={() => agent.toggle()} title="Hide the agent">&times;</button>
  </header>

  <div class="items" bind:this={list}>
    {#each agent.items as item, i (i)}
      {#if item.kind === "user"}
        <p class="user">{item.text}</p>
      {:else if item.kind === "assistant"}
        <p class="assistant">{item.text}</p>
      {:else if item.kind === "tool"}
        <details class="tool" class:failed={item.isError}>
          <summary>
            <span class="mark">{item.result === undefined ? "..." : item.isError ? "!" : "✓"}</span>
            {toolLine(item.name, item.input)}
          </summary>
          {#if item.result !== undefined}<pre>{item.result}</pre>{/if}
        </details>
      {:else if item.kind === "note"}
        <p class="note">{item.text}</p>
      {:else}
        <p class="error" role="alert">{item.text}</p>
      {/if}
    {:else}
      <p class="empty">
        Ask for an app, a window or a change. The agent reads and writes the open project, checks every change, and one Undo takes a whole answer back.
        Its key comes from the connection {agent.connection} in the vault.
      </p>
    {/each}
    {#if agent.running}<p class="note">Working...</p>{/if}
  </div>

  <footer>
    <textarea bind:value={text} onkeydown={keydown} rows="3" placeholder="Make a todo app with a list of tasks and a done flag" aria-label="Message"></textarea>
    {#if agent.running}
      <button onclick={() => agent.stop()}>Stop</button>
    {:else}
      <button class="primary" onclick={send} disabled={!text.trim()}>Send</button>
    {/if}
  </footer>
</section>

<style>
  .agent {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    background: var(--panel);
    border-left: 1px solid var(--border);
  }

  header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
  }

  header strong {
    flex: 1;
  }

  header select {
    max-width: 140px;
  }

  .items {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .items p {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-size: 13px;
  }

  .user {
    align-self: flex-end;
    max-width: 90%;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-radius: 8px;
    padding: 6px 9px;
  }

  .tool {
    font-size: 12px;
    color: var(--text-muted);
  }

  .tool summary {
    cursor: pointer;
  }

  .tool.failed {
    color: var(--warning);
  }

  .mark {
    display: inline-block;
    width: 1.2em;
  }

  .tool pre {
    margin: 4px 0 0;
    padding: 6px;
    max-height: 200px;
    overflow: auto;
    font-family: var(--mono);
    font-size: 11px;
    white-space: pre-wrap;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .note,
  .empty {
    color: var(--text-muted);
  }

  .error {
    color: var(--error);
  }

  footer {
    display: flex;
    gap: 6px;
    align-items: flex-end;
    padding: 8px;
    border-top: 1px solid var(--border);
  }

  footer textarea {
    flex: 1;
    resize: vertical;
    font: inherit;
    font-size: 13px;
  }
</style>
