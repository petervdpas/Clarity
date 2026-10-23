<script lang="ts">
  // The icon a part or command shows: its name, typed or picked from
  // Clarity's icons (Lucide's), with the icon itself beside it. Empty takes
  // the icon away.
  import type { Preview } from "./generated/Preview";
  import Icon from "./ui/Icon.svelte";

  let { preview, icon, onwrite }: { preview: Preview; icon: string | null; onwrite: (icon: string | null) => void } = $props();
  const list = `icons-${Math.random().toString(36).slice(2)}`;
  const known = $derived(icon === null || preview.iconNames.includes(icon));
</script>

<span class="icon-field">
  <input
    value={icon ?? ""}
    list={list}
    placeholder="an icon's name, as save"
    class:unknown={!known}
    onchange={(e) => onwrite((e.currentTarget as HTMLInputElement).value.trim() || null)}
  />
  {#if icon}<Icon paths={preview.icons} name={icon} />{/if}
  <datalist id={list}>
    {#each preview.iconNames as name (name)}<option value={name}></option>{/each}
  </datalist>
</span>

<style>
  .icon-field {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  input {
    flex: 1;
    min-width: 0;
  }

  input.unknown {
    border-color: var(--error);
  }
</style>
