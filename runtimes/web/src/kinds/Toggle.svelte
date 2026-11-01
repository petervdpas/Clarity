<script lang="ts">
  import Icon from "../parts/Icon.svelte";
  import type { PartContext } from "./context.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const pressed = $derived(ctx.bound ? ctx.value === true : ctx.partState?.checked === true);
</script>

<button
  type="button"
  id={ctx.id}
  class="push toggle {ctx.partClass ?? ''}"
  class:pressed
  aria-pressed={pressed}
  disabled={ctx.disabled || ctx.readonly}
  tabindex={p.tabOrder ?? undefined}
  title={!ctx.caption && p.name ? p.name : undefined}
  onclick={() => {
    if (ctx.design) return;
    if (ctx.bound) ctx.setValue(!pressed);
    else if (ctx.live && p.name) ctx.live.setChecked(p.name, !pressed);
    ctx.fire("change");
  }}>{#if p.icon}<Icon paths={ctx.preview.icons} name={p.icon} />{/if}{ctx.caption ?? ""}</button
>

<style>
  /* An icon stands beside the caption. */
  .push {
    width: 100%;
    height: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }

  .toggle.pressed {
    background: var(--accent-soft, var(--panel-alt));
    border-color: var(--accent);
    box-shadow: inset 0 1px 2px rgb(0 0 0 / 0.2);
  }
</style>
