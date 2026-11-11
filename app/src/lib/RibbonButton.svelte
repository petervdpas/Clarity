<script lang="ts">
  // A large ribbon button: an icon above its label, its keys in its tooltip.
  import { shownKey } from "@clarity/ui/host/keys.ts";
  import { mac } from "./keys.svelte";
  import IdeIcon from "./IdeIcon.svelte";
  import type { IdeIconName } from "./ideIcons";

  let {
    icon,
    label,
    title = undefined,
    disabled = false,
    active = false,
    keys = [],
    onclick,
  }: {
    icon: IdeIconName;
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
  <IdeIcon name={icon} size={22} />
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
</style>
