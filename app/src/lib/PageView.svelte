<script lang="ts">
  import GridBox from "./GridBox.svelte";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import { pageData, type PageData } from "./pages";

  let {
    preview,
    page,
    onnavigate,
  }: {
    preview: Preview;
    page: Page;
    onnavigate: (target: string) => void;
  } = $props();

  // The preview edits its own copy of the sample data; nothing is saved.
  let data = $state<PageData>({});
  $effect.pre(() => {
    // Typed as unknown: the snapshot type of JSON values is too deep for TypeScript.
    data = $state.snapshot(pageData(preview, page) as unknown) as PageData;
  });
</script>

<div class="page">
  <GridBox {preview} {page} {data} design={false} widgets={page.widgets} columns={page.columns} container={page.span.start} {onnavigate} />
</div>

<style>
  .page {
    padding: 4px;
  }
</style>
