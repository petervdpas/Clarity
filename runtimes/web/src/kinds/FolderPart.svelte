<script lang="ts">
  import type { PartContext } from "./context.ts";
  import FolderInput from "../parts/FolderInput.svelte";
  import { ownWord } from "../screen/model.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<FolderInput
  id={ctx.id}
  value={ctx.bound ? ctx.value : ctx.shown}
  words={{ choose: ownWord(ctx.preview, "Choose a folder") }}
  choose={ctx.design ? undefined : ctx.live?.chooseFolder}
  tabindex={p.tabOrder ?? undefined}
  describedBy={ctx.describedBy}
  invalid={ctx.error !== null}
  disabled={ctx.disabled}
  readonly={ctx.readonly}
  onchange={(next) => {
    if (ctx.bound) ctx.setValue(next);
    else ctx.setText(next);
    ctx.fire("change");
  }}
  onfocus={() => ctx.fire("enter")}
  onblur={() => ctx.fire("leave")}
/>
