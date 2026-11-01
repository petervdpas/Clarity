<script lang="ts">
  import EngineCanvas from "../parts/EngineCanvas.svelte";
  import { DEFAULT_UNITS } from "../screen/drawing.ts";
  import { languageOf, shownText } from "../screen/model.ts";
  import type { PartContext } from "./context.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<!-- An engine beside the screen draws it, frame by frame. -->
<EngineCanvas
  id={ctx.id}
  engine={ctx.preview.screenEngines.find((e) => e.name === p.runs)!}
  part={p.name}
  units={p.units ?? { width: DEFAULT_UNITS, height: DEFAULT_UNITS }}
  shown={(text) => shownText(ctx.preview, text)}
  language={() => languageOf(ctx.preview) ?? ""}
/>
