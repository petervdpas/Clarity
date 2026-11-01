<script lang="ts">
  import type { PartContext } from "./context.ts";
  import Icon from "../parts/Icon.svelte";
  import { tr } from "../screen/model.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const preview = $derived(ctx.preview);
</script>

<span class="icon-part" title={p.tooltip ? tr(preview, p.tooltip) : undefined}><Icon paths={preview.icons} name={p.icon ?? ""} size={20} /></span>

<style>
  .icon-part {
    flex: 1;
    display: grid;
    place-items: center;
  }
</style>
