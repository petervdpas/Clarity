<script lang="ts">
  // An icon of the application (Lucide's, by name): the path the model
  // carries for it, drawn with a round line in the color of the text around
  // it. An icon the model lacks draws nothing, at the same size.
  let { paths, name, size = 16 }: { paths: Record<string, string>; name: string; size?: number } = $props();
  const d = $derived(paths[name]);
</script>

<svg
  class="icon"
  viewBox="0 0 24 24"
  width={size}
  height={size}
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if d}<path {d} />{/if}
</svg>

<style>
  .icon {
    flex: none;
    vertical-align: middle;
  }
</style>
