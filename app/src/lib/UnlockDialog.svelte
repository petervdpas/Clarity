<script lang="ts">
  // Asks for the master password when the vault locked itself, over
  // whatever is on screen; what was being done goes on after.
  import { unlock } from "./unlockStore.svelte";

  let password = $state("");

  $effect(() => {
    if (unlock.asking) password = "";
  });

  function focused(node: HTMLInputElement) {
    queueMicrotask(() => node.focus());
  }

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (password && !unlock.busy) unlock.submit(password);
  }
</script>

{#if unlock.asking}
  <div class="backdrop">
    <div class="dialog" role="dialog" aria-label="Unlock the vault">
    <form onsubmit={submit}>
      <h3>Unlock the vault</h3>
      <p>The vault locked itself after a while without use. Enter the master password to go on.</p>
      {#if unlock.error}<p class="refusal" role="alert">{unlock.error}</p>{/if}
      <div class="form-grid">
        <label class="form-label" for="unlock-password">Master password</label>
        <input id="unlock-password" type="password" bind:value={password} autocomplete="current-password" use:focused
          onkeydown={(e) => e.key === "Escape" && unlock.cancel()} />
      </div>
      <div class="actions">
        <button type="button" onclick={() => unlock.cancel()}>Cancel</button>
        <button type="submit" class="primary" disabled={!password || unlock.busy}>{unlock.busy ? "Unlocking..." : "Unlock"}</button>
      </div>
    </form>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 30;
  }

  .dialog {
    width: min(440px, calc(100vw - 32px));
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.4);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    padding: 16px;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  h3,
  p {
    margin: 0;
  }

  .refusal {
    color: var(--error);
    font-size: 13px;
  }

  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
</style>
