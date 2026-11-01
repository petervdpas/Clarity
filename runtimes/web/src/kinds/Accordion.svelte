<script lang="ts">
  import { pressedSection, shownSection } from "../screen/accordions.ts";
  import { tr } from "../screen/model.ts";
  import type { PartContext } from "./context.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);

  // An accordion's open section where no application runs; live, the part's state keeps it.
  let openHere = $state(1);
  const open = $derived(ctx.design ? ctx.activeTab + 1 : shownSection(ctx.live && p.name ? ctx.partState?.open : openHere, ctx.widget.pages.length));

  /** A press on the header of section `press` (from 1): it opens, or folds when it was open. */
  function pressSection(press: number) {
    if (ctx.design) return;
    const count = ctx.widget.pages.length;
    if (ctx.live && p.name) {
      ctx.live.setOpen(p.name, pressedSection(ctx.partState?.open, press, count));
      void ctx.fire("change");
    } else openHere = pressedSection(openHere, press, count);
  }
</script>

<div class="accordion">
  {#each ctx.widget.pages as section, index (section.span.start)}
    <button
      type="button"
      class="section-header"
      class:open={open === index + 1}
      class:selected={ctx.design && ctx.isSelected(section.span.start)}
      aria-expanded={open === index + 1}
      data-tab-switch="{ctx.offset}:{index}"
      disabled={ctx.disabled}
      onpointerdown={(e) => ctx.design && ctx.pressTab(e, index, section.span.start)}
      onclick={() => pressSection(index + 1)}
      ><span class="chevron" class:closed={open !== index + 1}>&#9662;</span>{section.title ? tr(ctx.preview, section.title) : ""}</button
    >
    {#if open === index + 1}
      <div class="section-page">
        {@render ctx.grid({ widgets: section.children, columns: p.width, container: section.span.start })}
      </div>
    {/if}
  {/each}
</div>

<style>
  .chevron {
    display: inline-block;
    transition: transform 0.1s;
  }

  .chevron.closed {
    transform: rotate(-90deg);
  }

  .accordion {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 2px;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
  }

  .section-header.selected {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .section-page {
    flex: 1;
    min-height: 0;
    padding: calc(var(--padding) * 2) 0;
    overflow: hidden;
  }
</style>
