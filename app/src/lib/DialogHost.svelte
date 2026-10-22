<script lang="ts">
  // Shows the dialog the app asked for (dialogStore.svelte.ts): a
  // question with Cancel and a button that says what yes does, a message
  // with OK, a text to type, or a choice of buttons. Enter takes the
  // highlighted button, Escape cancels. Clarity never uses the system's or
  // the browser's dialogs.
  import Modal from "./Modal.svelte";
  import { dialogs } from "./dialogStore.svelte";

  let typed = $state("");
  $effect.pre(() => {
    if (dialogs.current?.kind === "prompt") typed = dialogs.current.value ?? "";
  });

  function focused(node: HTMLElement) {
    queueMicrotask(() => {
      node.focus();
      if (node instanceof HTMLInputElement) node.select();
    });
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (typed.trim()) dialogs.answer(typed.trim());
  }
</script>

{#if dialogs.current}
  {@const d = dialogs.current}
  <Modal title={d.title} label={d.text} onclose={() => dialogs.answer(d.kind === "message" ? true : null)}>
    {#if d.kind === "prompt"}
      <form onsubmit={submit}>
        <label>
          <span>{d.text}</span>
          <input bind:value={typed} placeholder={d.placeholder} spellcheck="false" use:focused />
        </label>
        <div class="buttons">
          <button type="button" onclick={() => dialogs.answer(null)}>Cancel</button>
          <button type="submit" class="primary" disabled={!typed.trim()}>{d.action}</button>
        </div>
      </form>
    {:else}
      <p>{d.text}</p>
      <div class="buttons">
        {#if d.kind === "confirm"}
          <button onclick={() => dialogs.answer(false)}>Cancel</button>
          <button class:primary={!d.danger} class:danger={d.danger} use:focused onclick={() => dialogs.answer(true)}>{d.action}</button>
        {:else if d.kind === "message"}
          <button class="primary" use:focused onclick={() => dialogs.answer(true)}>OK</button>
        {:else}
          <button onclick={() => dialogs.answer(null)}>Cancel</button>
          {#each d.choices as c, i (c.value)}
            {#if i === 0}
              <button class:danger={c.danger} use:focused onclick={() => dialogs.answer(c.value)}>{c.label}</button>
            {:else}
              <button class:danger={c.danger} onclick={() => dialogs.answer(c.value)}>{c.label}</button>
            {/if}
          {/each}
        {/if}
      </div>
    {/if}
  </Modal>
{/if}

<style>
  p {
    margin: 0;
    line-height: 1.5;
  }

  form,
  label {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  label {
    gap: 6px;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 8px;
  }

  .danger {
    background: var(--error);
    border-color: var(--error);
    color: #fff;
  }
</style>
