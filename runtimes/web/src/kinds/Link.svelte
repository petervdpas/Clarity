<script lang="ts">
  import type { PartContext } from "./context.ts";
  import { displayValue } from "../screen/model.ts";
  import { linkAddress } from "../screen/smallParts.ts";
  import { openLink } from "../host/backend.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const bound = $derived(ctx.bound);

  /** A link: its address, the address it binds, or its click. */
  function followLink() {
    if (ctx.design) return;
    const address = p.to ?? (p.bind ? linkAddress(ctx.value) : null);
    if (address) openLink(address);
    else if (!p.to && !p.bind) ctx.click();
  }
</script>

<button type="button" class="link" id={ctx.id} disabled={ctx.disabled} tabindex={p.tabOrder ?? undefined} onclick={followLink}
  >{bound ? displayValue(ctx.preview, bound.field, ctx.value) : (ctx.caption ?? "")}</button
>

<style>
  .link {
    align-self: center;
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
    text-align: left;
    font: inherit;
  }

  .link:disabled {
    color: var(--text-muted);
    cursor: default;
  }
</style>
