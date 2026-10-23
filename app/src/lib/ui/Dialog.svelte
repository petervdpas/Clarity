<script lang="ts">
  // The frame of Clarity's dialogs in a running application, in the
  // application's theme: a panel over a dimmed window, with what the dialog
  // puts in it. Escape and a click beside the panel close it. Never a dialog
  // of the system or the browser (DESIGN 5.14).
  import type { Snippet } from "svelte";

  let { label, onclose, children }: { label: string; onclose: () => void; children: Snippet } = $props();
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="backdrop clarity-overlay" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="panel" role="dialog" aria-modal="true" aria-label={label}>
    {@render children()}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.3);
    display: grid;
    place-items: center;
    z-index: 110;
  }

  .panel {
    width: min(420px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    overflow: auto;
    background: var(--panel);
    color: var(--text);
    border: var(--border-width) solid var(--border-strong);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow);
    padding: calc(var(--padding) * 3);
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
  }
</style>
