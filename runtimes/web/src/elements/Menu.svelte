<script lang="ts">
  // A part's menu in a generated window (DESIGN 5.16, 7.8): the part opens it
  // on a right click, or from its arrow on a split button.
  import { untrack, type Snippet } from "svelte";
  import type { Menu } from "../generated/Menu.ts";
  import { provideHolder, useHolder } from "./holder.ts";

  let { name = null, children }: { name?: string | null; children?: Snippet } = $props();

  const holder = useHolder();
  const place = holder.next();
  const menu: Menu = untrack(() => ({ name, items: [], span: { start: place, end: place, line: 0, col: 0 } }));
  holder.setMenu?.(menu);
  provideHolder({ widgets: [], next: holder.next, commands: menu.items });
</script>

{@render children?.()}
