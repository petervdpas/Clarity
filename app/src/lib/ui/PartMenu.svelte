<script lang="ts">
  // A part's context menu (DESIGN 5.16): opened by a right click at the
  // pointer, its items as the frame's menu has them, submenus flying out on
  // hover. A named item hidden by code is left out and a disabled one is
  // greyed. A click outside it or Escape closes it.
  import { onMount } from "svelte";
  import type { Command } from "../generated/Command";
  import Icon from "./Icon.svelte";

  let {
    items,
    icons = {},
    x,
    y,
    caption,
    shows,
    enabled,
    onrun,
    onclose,
  }: {
    items: Command[];
    /** The drawings of the icons the application shows, by name. */
    icons?: Record<string, string>;
    /** Where the pointer was, in the window. */
    x: number;
    y: number;
    /** An item's text, in the language shown. */
    caption: (text: string) => string;
    /** Whether a named item shows and takes clicks, as its part state says. */
    shows: (name: string | null) => boolean;
    enabled: (name: string | null) => boolean;
    onrun: (item: Command) => void;
    onclose: () => void;
  } = $props();

  let menu: HTMLElement | undefined = $state();
  // Kept inside the window: a menu near its right or bottom edge opens the other way.
  let left = $state(0);
  let top = $state(0);
  $effect(() => {
    const width = menu?.offsetWidth ?? 0;
    const height = menu?.offsetHeight ?? 0;
    left = Math.max(0, Math.min(x, window.innerWidth - width - 4));
    top = Math.max(0, Math.min(y, window.innerHeight - height - 4));
  });

  onMount(() => {
    const away = (e: PointerEvent) => {
      if (menu && !menu.contains(e.target as Node)) onclose();
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onclose();
      }
    };
    // Added after the right click that opened it has passed.
    const timer = setTimeout(() => {
      window.addEventListener("pointerdown", away, true);
      window.addEventListener("keydown", key, true);
    });
    return () => {
      clearTimeout(timer);
      window.removeEventListener("pointerdown", away, true);
      window.removeEventListener("keydown", key, true);
    };
  });

  function run(item: Command) {
    if (item.items.length > 0 || !enabled(item.props.name)) return;
    onclose();
    onrun(item);
  }
</script>

{#snippet entries(list: Command[])}
  {#each list.filter((i) => shows(i.props.name)) as item (item.span.start)}
    {#if item.props.kind === "separator"}
      <div class="separator-line" role="separator"></div>
    {:else}
      <div class="entry" class:sub={item.items.length > 0}>
        <button type="button" role="menuitem" disabled={!enabled(item.props.name)} onclick={() => run(item)}>
          {#if item.props.icon}<Icon paths={icons} name={item.props.icon} />{/if}
          <span>{item.props.caption ? caption(item.props.caption) : ""}</span>
          {#if item.props.shortcut}<kbd>{item.props.shortcut}</kbd>{/if}
          {#if item.items.length > 0}<span class="arrow">&rsaquo;</span>{/if}
        </button>
        {#if item.items.length > 0}
          <div class="flyout" role="menu">{@render entries(item.items)}</div>
        {/if}
      </div>
    {/if}
  {/each}
{/snippet}

<div class="part-menu clarity-overlay" role="menu" tabindex="-1" bind:this={menu} style:left="{left}px" style:top="{top}px" oncontextmenu={(e) => e.preventDefault()}>
  {@render entries(items)}
</div>

<style>
  .part-menu,
  .flyout {
    min-width: 180px;
    background: var(--menu-bg, var(--panel));
    color: var(--menu-text, var(--text));
    border: var(--border-width, 1px) solid var(--menu-border, var(--border-strong));
    border-radius: var(--menu-radius, var(--radius-panel, 4px));
    box-shadow: var(--shadow);
    padding: 4px;
  }

  .part-menu {
    position: fixed;
    z-index: 60;
  }

  .entry {
    position: relative;
  }

  .entry > button {
    display: flex;
    align-items: center;
    width: 100%;
    gap: 12px;
    text-align: left;
    border: none;
    background: transparent;
    color: inherit;
    border-radius: var(--radius-button, 3px);
    padding: calc(var(--padding, 4px) - 1px) calc(var(--padding, 4px) * 2);
    font-size: var(--font-size);
  }

  .entry > button:hover:not(:disabled) {
    background: var(--menu-hover-bg, var(--accent-soft));
    color: var(--menu-hover-text, inherit);
  }

  .entry > button:disabled {
    opacity: 0.5;
  }

  kbd {
    margin-left: auto;
    font-family: var(--mono);
    font-size: var(--font-size-small);
    color: var(--text-muted);
  }

  .arrow {
    margin-left: auto;
  }

  .flyout {
    display: none;
    position: absolute;
    top: -4px;
    left: 100%;
  }

  .entry.sub:hover > .flyout {
    display: block;
  }

  .separator-line {
    height: 1px;
    margin: 4px 2px;
    background: var(--menu-border, var(--border));
  }
</style>
