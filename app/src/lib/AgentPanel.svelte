<script lang="ts">
  // The agent: a conversation with a model that works on the open project.
  // It sits right of every tab; what it changes is one Undo step per turn.
  import { toolLine, usageLine } from "./agent";
  import { agent } from "./agentStore.svelte";
  import { markdown } from "./markdown";

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

  /** The answer whose text was copied last, to say so for a moment. */
  let copied = $state<number | null>(null);

  /** Copies an answer as its model wrote it (Markdown), to paste elsewhere. */
  async function copy(i: number, answer: string) {
    try {
      await navigator.clipboard.writeText(answer);
      copied = i;
      setTimeout(() => {
        if (copied === i) copied = null;
      }, 1500);
    } catch {
      copied = null;
    }
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
        <!-- Escaped first: the answer brings no HTML of its own. -->
        <div class="assistant">
          <button class="copy" onclick={() => copy(i, item.text)} title="Copy this answer as Markdown">{copied === i ? "Copied" : "Copy"}</button>
          {@html markdown(item.text)}
        </div>
      {:else if item.kind === "tool"}
        <details class="tool" class:failed={item.isError}>
          <summary>
            <span class="mark">{item.result === undefined ? "..." : item.isError ? "!" : "✓"}</span>
            {toolLine(item.name, item.input)}
          </summary>
          {#if item.result !== undefined}<pre>{item.result}</pre>{/if}
        </details>
      {:else if item.kind === "ask"}
        <div class="ask" role="group" aria-label="The agent asks">
          <p>The agent asks to: {item.what}.</p>
          {#if item.allowed === undefined}
            <div class="answers">
              <button class="primary" onclick={() => agent.answer(item.id, true)}>Allow</button>
              <button onclick={() => agent.answer(item.id, false)}>Deny</button>
            </div>
          {:else}
            <p class="answered">{item.allowed ? "Allowed." : "Denied."}</p>
          {/if}
        </div>
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

  {#if usageLine(agent.turnUsage, agent.sessionUsage)}
    <p class="usage" title="Tokens sent to the model (in) and written by it (out); what the cache served costs a tenth or less">{usageLine(agent.turnUsage, agent.sessionUsage)}</p>
  {/if}

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
  .usage {
    margin: 0;
    padding: 4px 12px;
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-muted);
  }

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

  .assistant {
    position: relative;
    font-size: 13px;
    overflow-wrap: anywhere;
    user-select: text;
  }

  /* Shown on the answer under the pointer, top right. */
  .copy {
    position: absolute;
    top: 0;
    right: 0;
    padding: 1px 7px;
    font-size: 11px;
    opacity: 0;
  }

  .assistant:hover .copy,
  .copy:focus-visible {
    opacity: 1;
  }

  .assistant :global(p),
  .assistant :global(ul),
  .assistant :global(ol),
  .assistant :global(h4),
  .assistant :global(pre) {
    margin: 0 0 6px;
  }

  .assistant :global(ul),
  .assistant :global(ol) {
    padding-left: 18px;
  }

  .assistant :global(h4) {
    font-size: 13px;
  }

  .assistant :global(code) {
    font-family: var(--mono);
    font-size: 12px;
    background: color-mix(in srgb, var(--text) 8%, transparent);
    border-radius: 3px;
    padding: 0 3px;
  }

  .assistant :global(pre) {
    padding: 6px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .assistant :global(pre code) {
    background: none;
    padding: 0;
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

  .ask {
    padding: 8px;
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .answers {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
  }

  .answered,
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
