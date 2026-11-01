<script lang="ts">
  // Shows a generated window (DESIGN 7.8): gives it the project, the window
  // as declared and its data, which its parts read.
  import type { Component } from "svelte";
  import type { Page } from "../generated/Page.ts";
  import type { Preview } from "../generated/Preview.ts";
  import type { Widget } from "../generated/Widget.ts";
  import type { PageData } from "../screen/pages.ts";
  import { provideWindowPlace } from "./holder.ts";

  let {
    window,
    preview,
    page,
    data,
    onnavigate,
    made,
  }: {
    window: Component;
    preview: Preview;
    page: Page;
    data: PageData;
    onnavigate: (target: string, newWindow?: boolean) => void;
    made?: (widgets: Widget[]) => void;
  } = $props();

  provideWindowPlace({
    get preview() {
      return preview;
    },
    get page() {
      return page;
    },
    get data() {
      return data;
    },
    onnavigate: (target, newWindow) => onnavigate(target, newWindow),
    made: (widgets) => made?.(widgets),
  });

  const Drawn = $derived(window);
</script>

<Drawn />
