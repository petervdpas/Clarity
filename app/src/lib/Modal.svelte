<script lang="ts" module>
  /** The open dialogs, newest last: only the newest answers Escape. */
  const open: symbol[] = [];
</script>

<script lang="ts">
  // The frame of every dialog of Clarity's own: a panel over a dimmed
  // window, an optional title, and what the dialog puts in it. Escape and
  // a click beside the panel close it; with dialogs over each other, only
  // the top one. Clarity never uses the system's or the browser's dialogs.
  import { onMount, type Snippet } from "svelte";

  let {
    title = "",
    label,
    width = 420,
    tone = "plain",
    onclose,
    children,
  }: {
    title?: string;
    /** What a screen reader calls it, when there is no title. */
    label?: string;
    width?: number;
    /** A dialog about something that cannot be undone is framed in the error color. */
    tone?: "plain" | "danger";
    onclose: () => void;
    children: Snippet;
  } = $props();

  const me = Symbol("dialog");
  onMount(() => {
    open.push(me);
    return () => open.splice(open.indexOf(me), 1);
  });
  const onTop = () => open[open.length - 1] === me;
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape" && onTop()) {
      e.stopPropagation();
      onclose();
    }
  }}
/>

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="panel" class:danger={tone === "danger"} role="dialog" aria-modal="true" aria-label={title || label} style:width="min({width}px, calc(100vw - 32px))">
    {#if title}<h3>{title}</h3>{/if}
    {@render children()}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    display: grid;
    place-items: center;
    z-index: 70;
  }

  .panel {
    max-height: calc(100vh - 32px);
    overflow: auto;
    background: var(--panel);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.35);
  }

  .panel.danger {
    border-color: var(--error);
  }

  h3 {
    margin: 0;
    font-size: 15px;
  }
</style>
