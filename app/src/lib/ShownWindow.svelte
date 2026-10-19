<script lang="ts">
  // A window as the designer and the preview show it: at the size it opens
  // at on every stack (its own `size`, or WINDOW_WIDTH by WINDOW_HEIGHT), so
  // its columns share the same width and its growing rows the same height as
  // when it runs. It is scaled down to fit the room it has, never up. The
  // children are its title bar and its client, as tall as the window.
  import type { Snippet } from "svelte";
  import { WINDOW_WIDTH } from "./grid";

  let {
    area,
    clientWidth = WINDOW_WIDTH,
    children,
  }: {
    /** The theme's class and light or dark scheme. */
    area: { class: string; scheme: string };
    /** The window's width, without its border. */
    clientWidth?: number;
    children: Snippet;
  } = $props();

  let room = $state(0);
  let height = $state(0);
  // The window's border, one pixel on each side.
  const width = $derived(clientWidth + 2);
  const scale = $derived(room > 0 ? Math.min(1, room / width) : 1);
</script>

<div class="room" bind:clientWidth={room}>
  <div class="fit" style:width="{width * scale}px" style:height="{height * scale}px">
    <div class="window {area.class}" data-scheme={area.scheme} style:width="{width}px" style:transform="scale({scale})" bind:offsetHeight={height}>
      {@render children()}
    </div>
  </div>
</div>

<style>
  .room {
    min-width: 0;
  }

  .window {
    transform-origin: top left;
    background: var(--panel);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.08);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
