<script lang="ts">
  import type { PartContext } from "./context.ts";
  import CustomPartView from "../parts/CustomPartView.svelte";
  import type { JsonValue } from "../generated/serde_json/JsonValue.ts";
  import { tr } from "../screen/model.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const preview = $derived(ctx.preview);
  const hidden = $derived(ctx.partState?.visible === false);
</script>

<CustomPartView
  {p}
  live={ctx.design ? undefined : ctx.live}
  value={ctx.bound ? (ctx.value as JsonValue) : undefined}
  {hidden}
  disabled={ctx.disabled}
  preview={preview.project.files.flatMap((f) => f.parts).find((d) => d.name.name === p.part)?.preview ?? null}
  words={(text) => tr(preview, text)}
/>
