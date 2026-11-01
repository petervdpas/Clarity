<script lang="ts">
  import GridBox from "@clarity/ui/grid/GridBox.svelte";
  import type { Page } from "@clarity/ui/generated/Page.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import { pageData, type PageData } from "@clarity/ui/screen/pages.ts";

  let {
    preview,
    page,
    onnavigate,
  }: {
    preview: Preview;
    page: Page;
    onnavigate: (target: string, newWindow?: boolean) => void;
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
    /* Passes the height on, so a fluid window fills it. */
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>
