<script lang="ts">
  import { tr } from "../screen/model.ts";
  import type { PartContext } from "./context.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<div class="tabs">
  <div class="tab-strip">
    {#each ctx.widget.pages as tab, index (tab.span.start)}
      <button
        class:active={index === ctx.activeTab}
        class:selected={ctx.design && ctx.isSelected(tab.span.start)}
        data-tab-switch="{ctx.offset}:{index}"
        onpointerdown={(e) => ctx.pressTab(e, index, tab.span.start)}>{tab.title ? tr(ctx.preview, tab.title) : tab.title}</button
      >
    {/each}
  </div>
  {#if ctx.widget.pages[ctx.activeTab]}
    {@const tab = ctx.widget.pages[ctx.activeTab]}
    <div class="tab-page">
      {@render ctx.grid({ widgets: tab.children, columns: p.width, container: tab.span.start })}
    </div>
  {/if}
</div>

<style>
  .tabs {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    /* The tab page's frame sits out between the columns, as a panel's does. */
    margin: 0 -3px;
  }

  .tab-strip {
    display: flex;
    gap: 2px;
    border-bottom: var(--border-width) solid var(--border-strong);
  }

  .tab-strip button {
    border-radius: var(--tab-radius, var(--radius-button)) var(--tab-radius, var(--radius-button)) 0 0;
    border-bottom: none;
    margin-bottom: -1px;
    background: var(--tab-bg);
    color: var(--tab-text);
    border-color: var(--tab-border);
    font-weight: var(--tab-weight, inherit);
  }

  .tab-strip button:hover:not(.active) {
    background: var(--tab-hover-bg, var(--tab-bg));
    color: var(--tab-hover-text, var(--tab-text));
  }

  .tab-strip button.active {
    background: var(--panel);
    color: var(--text);
    border-bottom: var(--border-width) solid var(--panel);
  }

  .tab-strip button.selected {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .tab-page {
    flex: 1;
    border: var(--border-width) solid var(--border-strong);
    border-top: none;
    border-radius: 0 0 var(--radius-panel) var(--radius-panel);
    padding: calc(var(--padding) * 2) calc(3px - var(--border-width));
    overflow: hidden;
  }
</style>
