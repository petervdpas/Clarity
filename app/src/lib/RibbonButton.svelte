<script lang="ts">
  // A large ribbon button: an icon above its label, its keys in its tooltip.
  import { shownKey } from "./keys";
  import { mac } from "./keys.svelte";

  let {
    icon,
    label,
    title = undefined,
    disabled = false,
    active = false,
    keys = [],
    onclick,
  }: {
    icon: string;
    label: string;
    title?: string;
    disabled?: boolean;
    active?: boolean;
    /** The keys that run it too (DESIGN 5.28). */
    keys?: string[];
    onclick: () => void;
  } = $props();

  const shown = $derived(keys.map((k) => shownKey(k, mac)));
  const tip = $derived(shown.length === 0 ? title : `${title ?? label} (${shown.join(" or ")})`);
</script>

<button class="ribbon-button" class:active title={tip} aria-keyshortcuts={keys.length ? keys.map((k) => k.replace(/^Ctrl(?=\+)/, mac ? "Meta" : "Control")).join(" ") : undefined} {disabled} {onclick}>
  <svg viewBox="0 0 24 24" aria-hidden="true">
    {#if icon === "new"}
      <path d="M6 3h8l4 4v14H6z" /><path d="M14 3v4h4" /><path d="M12 11v6M9 14h6" />
    {:else if icon === "open"}
      <path d="M3 6h6l2 2h10v11H3z" /><path d="M3 10h18" />
    {:else if icon === "reload"}
      <path d="M19 12a7 7 0 1 1-2-5" /><path d="M19 4v4h-4" />
    {:else if icon === "window"}
      <rect x="3" y="4" width="18" height="16" rx="2" /><path d="M3 8h18" /><path d="M12 11v6M9 14h6" />
    {:else if icon === "app"}
      <rect x="3" y="4" width="18" height="16" rx="2" /><path d="M3 8h18M3 16h18M8 8v8" />
    {:else if icon === "undo"}
      <path d="M9 14L4 9l5-5" /><path d="M4 9h11a5 5 0 0 1 0 10h-3" />
    {:else if icon === "redo"}
      <path d="M15 14l5-5-5-5" /><path d="M20 9H9a5 5 0 0 0 0 10h3" />
    {:else if icon === "delete"}
      <path d="M5 7h14" /><path d="M9 7V4h6v3" /><path d="M7 7l1 13h8l1-13" />
    {:else if icon === "save"}
      <path d="M5 3h11l3 3v15H5z" /><path d="M8 3v6h8V3" /><rect x="8" y="13" width="8" height="6" />
    {:else if icon === "grid"}
      <path d="M4 4h16v16H4zM4 9.3h16M4 14.6h16M9.3 4v16M14.6 4v16" />
    {:else if icon === "play"}
      <path d="M8 5l11 7-11 7z" />
    {:else if icon === "back"}
      <path d="M11 6l-6 6 6 6" /><path d="M5 12h14" />
    {:else if icon === "text"}
      <path d="M5 6h14M5 10h10M5 14h14M5 18h8" />
    {:else if icon === "code"}
      <path d="M9 7l-5 5 5 5M15 7l5 5-5 5" />
    {:else if icon === "build"}
      <path d="M4 7l8-4 8 4v10l-8 4-8-4z" /><path d="M4 7l8 4 8-4M12 11v10" />
    {:else if icon === "globe"}
      <circle cx="12" cy="12" r="9" />
      <path d="M3 12h18M12 3c2.5 2.7 3.8 5.7 3.8 9s-1.3 6.3-3.8 9c-2.5-2.7-3.8-5.7-3.8-9s1.3-6.3 3.8-9z" />
    {:else if icon === "stop"}
      <rect x="6" y="6" width="12" height="12" rx="1" />
    {:else if icon === "settings"}
      <circle cx="12" cy="12" r="3" /><path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M5.6 18.4l2.1-2.1M16.3 7.7l2.1-2.1" />
    {:else if icon === "check"}
      <path d="M5 12l5 5 9-10" />
    {:else if icon === "key"}
      <circle cx="8" cy="15" r="4" /><path d="M11 12l9-9M16 7l3 3M14 9l2 2" />
    {:else if icon === "link"}
      <path d="M10 14a4 4 0 0 0 6 0l3-3a4 4 0 0 0-6-6l-1 1" /><path d="M14 10a4 4 0 0 0-6 0l-3 3a4 4 0 0 0 6 6l1-1" />
    {:else if icon === "lock"}
      <rect x="5" y="11" width="14" height="10" rx="2" /><path d="M8 11V7a4 4 0 0 1 8 0v4" />
    {:else if icon === "edit"}
      <path d="M4 20h4L19 9l-4-4L4 16z" /><path d="M13 7l4 4" />
    {:else if icon === "copy"}
      <rect x="8" y="8" width="12" height="12" rx="2" /><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3" />
    {:else if icon === "tag"}
      <path d="M3 12V4h8l10 10-8 8z" /><circle cx="7.5" cy="8.5" r="1.5" />
    {/if}
  </svg>
  <span>{label}</span>
</button>

<style>
  .ribbon-button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    min-width: 58px;
    padding: 5px 6px 4px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    font-size: 11px;
  }

  .ribbon-button:hover:not(:disabled) {
    background: var(--panel-alt);
    border-color: var(--border);
  }

  .ribbon-button.active {
    background: var(--accent-soft);
    border-color: var(--accent);
  }

  .ribbon-button:disabled {
    opacity: 0.45;
  }

  svg {
    width: 22px;
    height: 22px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
