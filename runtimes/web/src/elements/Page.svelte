<script lang="ts">
  // A tab, step, section or pane in a generated window, with the parts on it (DESIGN 7.8).
  import { untrack, type Snippet } from "svelte";
  import type { WidgetPage } from "../generated/WidgetPage.ts";
  import { provideHolder, useHolder } from "./holder.ts";

  let { title = null, span = null, children }: { title?: string | null; span?: number | null; children?: Snippet } = $props();

  const holder = useHolder();
  const place = holder.next();
  const page: WidgetPage = untrack(() => ({ title, columns: span, children: [], span: { start: place, end: place, line: 0, col: 0 } }));
  holder.pages?.push(page);
  provideHolder({ widgets: page.children, next: holder.next });
</script>

{@render children?.()}
