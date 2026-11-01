<!--
  The IDE's notices as toasts, in the lower right corner: each says what
  happened, goes by itself after a while, and closes with its button.
-->
<script lang="ts" module>
  /** How long a notice stays before it goes by itself. */
  export const TOAST_MS = 8000;
</script>

<script lang="ts">
  let { toasts, ondismiss }: { toasts: { id: number; text: string }[]; ondismiss: (id: number) => void } = $props();
</script>

<div class="toasts" aria-live="polite">
  {#each toasts as toast (toast.id)}
    <div class="toast" role="alert">
      <span>{toast.text}</span>
      <button title="Dismiss" aria-label="Dismiss" onclick={() => ondismiss(toast.id)}>×</button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 40px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 420px;
    z-index: 1000;
  }
  .toast {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 10px 12px;
    border-radius: 6px;
    border: 1px solid var(--error);
    background: var(--panel, #1e2229);
    color: var(--error);
    font-size: 12px;
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.3);
    overflow-wrap: anywhere;
  }
  .toast span {
    flex: 1;
  }
  .toast button {
    border: none;
    background: none;
    color: inherit;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
  }
</style>
