<script lang="ts">
  // The buttons beside a password or another secret: an eye that shows what
  // is typed or hides it again, and one that copies it. The same on Avalonia.

  let {
    revealed = $bindable(false),
    text,
    words = { show: "Show", hide: "Hide", copy: "Copy" },
    disabled = false,
  }: {
    /** Whether the secret shows as text. */
    revealed?: boolean;
    /** What is in the input now, typed or not yet taken. */
    text: () => string;
    /** The buttons' names, in the person's language. */
    words?: { show: string; hide: string; copy: string };
    disabled?: boolean;
  } = $props();

  async function copy() {
    const value = text();
    try {
      await navigator.clipboard.writeText(value);
    } catch {
      // A webview without the clipboard API copies a selection instead.
      const scratch = document.createElement("textarea");
      scratch.value = value;
      document.body.append(scratch);
      scratch.select();
      document.execCommand("copy");
      scratch.remove();
    }
  }
</script>

<button type="button" class="secret-button" title={revealed ? words.hide : words.show} aria-label={revealed ? words.hide : words.show} aria-pressed={revealed} {disabled} onclick={() => (revealed = !revealed)}>
  <svg viewBox="0 0 24 24" aria-hidden="true">
    <path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z" />
    <circle cx="12" cy="12" r="3" />
    {#if revealed}<path d="M3 3l18 18" />{/if}
  </svg>
</button>
<button type="button" class="secret-button" title={words.copy} aria-label={words.copy} {disabled} onclick={copy}>
  <svg viewBox="0 0 24 24" aria-hidden="true">
    <rect x="9" y="9" width="12" height="12" rx="2" />
    <path d="M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1" />
  </svg>
</button>

<style>
  .secret-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 6px;
  }
  svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
