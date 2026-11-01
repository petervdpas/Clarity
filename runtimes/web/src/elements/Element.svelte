<script lang="ts">
  // A part in a generated window: it adds itself, as its attributes write
  // it, to what holds it, and holds the parts written inside it (DESIGN 7.8).
  import { untrack, type Snippet } from "svelte";
  import type { WidgetKind } from "../generated/WidgetKind.ts";
  import { provideHolder, useHolder } from "./holder.ts";
  import { partOf } from "./partOf.ts";

  let { kind, children, ...given }: { kind: WidgetKind; children?: Snippet } & Record<string, unknown> = $props();

  const holder = useHolder();
  // A window's markup is fixed: the part is made once, as written.
  const widget = untrack(() => partOf(kind, given, holder.next()));
  holder.widgets.push(widget);
  provideHolder({ widgets: widget.children, pages: widget.pages, next: holder.next, setMenu: (menu) => (widget.menu = menu) });
</script>

{@render children?.()}
