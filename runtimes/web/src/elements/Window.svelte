<script lang="ts">
  // A generated window (DESIGN 7.8): its markup adds its parts, and it draws
  // them with the screen's grid, as the host would draw the window from the model.
  import { onMount, type Snippet } from "svelte";
  import GridBox from "../grid/GridBox.svelte";
  import type { Widget } from "../generated/Widget.ts";
  import { provideHolder, useWindowPlace } from "./holder.ts";

  let { name, children }: { name: string; children?: Snippet } = $props();

  const place = useWindowPlace();
  const widgets: Widget[] = [];
  // Each part a place of its own after the window's, as in its file.
  let count = 0;
  provideHolder({ widgets, next: () => (place?.page.span.start ?? 0) + ++count });

  // The markup has added every part by the time the window is in the page.
  let ready = $state(false);
  onMount(() => (ready = true));
  const page = $derived(place ? { ...place.page, widgets } : null);
</script>

{@render children?.()}
{#if ready && place && page}
  <GridBox preview={place.preview} {page} data={place.data} design={false} {widgets} columns={place.page.columns} container={place.page.span.start} onnavigate={place.onnavigate} />
{:else if !place}
  <p>The window {name} is shown by Clarity's screen.</p>
{/if}
