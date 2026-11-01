<script lang="ts">
  import type { PartContext } from "./context.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const unset = $derived(ctx.unset);
  const caption = $derived(ctx.caption);
</script>

<h2 class="heading-text">{#if unset && !caption}<span class="unset">{unset}</span>{/if}{caption ?? ""}</h2>

<style>
  .heading-text {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--font-size-heading);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .unset {
    color: var(--text-muted);
    font-style: italic;
    font-weight: 400;
  }
</style>
