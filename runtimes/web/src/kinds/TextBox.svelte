<script lang="ts">
  import type { PartContext } from "./context.ts";
  import TextInput from "../parts/TextInput.svelte";
  import { ownWord, tr } from "../screen/model.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<TextInput
  id={ctx.id}
  kind={p.kind === "password" ? "password" : p.kind === "memo" ? "memo" : ctx.bound?.field.type.kind === "email" ? "email" : "text"}
  words={p.kind === "password" ? { show: ownWord(ctx.preview, "Show"), hide: ownWord(ctx.preview, "Hide"), copy: ownWord(ctx.preview, "Copy") } : undefined}
  value={ctx.shown}
  placeholder={p.placeholder ? tr(ctx.preview, p.placeholder) : undefined}
  tabindex={p.tabOrder ?? undefined}
  describedBy={ctx.describedBy}
  invalid={ctx.error !== null}
  disabled={ctx.disabled}
  readonly={ctx.readonly}
  onchange={(next) => {
    ctx.setText(next);
    ctx.fire("change");
  }}
  onsubmit={ctx.submit}
  onfocus={() => ctx.fire("enter")}
  onblur={() => ctx.fire("leave")}
/>
