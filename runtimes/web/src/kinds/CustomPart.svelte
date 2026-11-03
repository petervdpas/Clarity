<script lang="ts">
  import type { PartContext } from "./context.ts";
  import CustomPartView from "../parts/CustomPartView.svelte";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const preview = $derived(ctx.preview);
</script>

<CustomPartView kind={p.part ?? ""} preview={preview.project.files.flatMap((f) => f.parts).find((d) => d.name.name === p.part)?.preview ?? null} />
