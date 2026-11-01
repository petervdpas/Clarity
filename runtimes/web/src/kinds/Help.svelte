<script lang="ts">
  import type { PartContext } from "./context.ts";
  import { helpId } from "../screen/pages.ts";
  import { tr } from "../screen/model.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const targetError = $derived(ctx.targetError);
  const targetField = $derived(ctx.targetField);
</script>

<span class="help-text" class:error={targetError !== null} id={p.for ? helpId(p.for) : undefined} aria-live="polite"
  >{targetError ?? ctx.caption ?? (targetField?.modifiers.help ? tr(ctx.preview, targetField.modifiers.help) : "")}</span
>

<style>
  .help-text {
    font-size: var(--font-size-small);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .help-text.error {
    color: var(--error);
  }
</style>
