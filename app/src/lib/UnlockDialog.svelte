<script lang="ts">
  // Asks for the master password when the vault locked itself, over
  // whatever is on screen; what was being done goes on after.
  import Modal from "./Modal.svelte";
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
  <Modal title="Unlock the vault" width={440} onclose={() => unlock.cancel()}>
    <form onsubmit={submit}>
      <p>The vault locked itself after a while without use. Enter the master password to go on.</p>
      {#if unlock.error}<p class="refusal" role="alert">{unlock.error}</p>{/if}
      <div class="form-grid">
        <label class="form-label" for="unlock-password">Master password</label>
        <input id="unlock-password" type="password" bind:value={password} autocomplete="current-password" use:focused />
      </div>
      <div class="actions">
        <button type="button" onclick={() => unlock.cancel()}>Cancel</button>
        <button type="submit" class="primary" disabled={!password || unlock.busy}>{unlock.busy ? "Unlocking..." : "Unlock"}</button>
      </div>
    </form>
  </Modal>
{/if}

<style>


  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

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
