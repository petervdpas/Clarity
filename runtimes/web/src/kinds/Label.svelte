<script lang="ts">
  import type { PartContext } from "./context.ts";
  import { partId } from "../screen/pages.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const unset = $derived(ctx.unset);
  const caption = $derived(ctx.caption);
  const targetField = $derived(ctx.targetField);
</script>

<label class="label-text" for={p.for ? partId(p.for) : undefined}
  >{#if unset && !caption}<span class="unset">{unset}</span>{/if}{caption ?? ""}{#if targetField?.modifiers.required}<span class="required" title="Required">*</span>{/if}</label
>

<style>
  .label-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-muted);
  }

  .required {
    color: var(--error);
    margin-left: 2px;
  }

  .unset {
    color: var(--text-muted);
    font-style: italic;
    font-weight: 400;
  }
</style>
