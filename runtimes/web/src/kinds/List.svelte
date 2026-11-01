<script lang="ts">
  import { keyMove, rowDrag } from "../parts/rowDrag.ts";
  import type { PartContext } from "./context.ts";
  import { reordered, reorders } from "./shared.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const moves = $derived(reorders(ctx));
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<ul
  class="list"
  class:cards={p.cards}
  tabindex={moves ? 0 : undefined}
  use:rowDrag={{ enabled: moves, tree: false, onmove: (move) => reordered(ctx, move) }}
  onkeydown={(e) => {
    const move = moves ? keyMove(e, ctx.selectedRow ?? null) : null;
    if (move) {
      e.preventDefault();
      reordered(ctx, move);
    }
  }}
>
  {#each ctx.collection?.rows ?? [] as row, i (i)}
    <li data-row={i} class:chosen={ctx.selectedRow === i} role="presentation" onclick={() => ctx.selectRow(i)} oncontextmenu={() => ctx.selectRow(i)} ondblclick={() => ctx.fire("open")}
      >{#if p.cards}<!-- A card: the first field its title, the others lines below it. -->{#each p.show as f, at (f)}<span class={at === 0 ? "card-title" : "card-line"}>{String(row[f] ?? "")}</span>{/each}{:else}{p.show.map((f) => String(row[f] ?? "")).join(" | ")}{/if}</li
    >
  {/each}
</ul>

<style>
  /* A list and its rows, as the theme's part list says (hover, selected). */
  .list {
    flex: 1;
    margin: 0;
    padding: 4px;
    list-style: none;
    overflow: auto;
    background: var(--list-bg);
    color: var(--list-text);
    border: var(--border-width) solid var(--list-border, var(--border-strong));
    border-radius: var(--list-radius, var(--radius-panel));
    font-size: var(--font-size);
  }

  .list li {
    padding: calc(var(--padding) - 1px) calc(var(--padding) * 1.5);
    border-bottom: var(--border-width) solid var(--border);
  }

  .list li:hover {
    background: var(--list-hover-bg);
    color: var(--list-hover-text, inherit);
  }

  .list li.chosen {
    background: var(--list-selected-bg, var(--selection));
    color: var(--list-selected-text, inherit);
  }

  /* A list of cards: no frame of its own; each row a card, as the theme's
     part card and part card_line say, the one chosen in its selected look. */
  .list.cards {
    padding: 0;
    border: none;
    background: transparent;
  }

  .list.cards li {
    margin-bottom: calc(var(--padding) * 1.5);
    padding: calc(var(--padding) * 2) calc(var(--padding) * 2.5);
    border: var(--border-width) solid var(--card-border, transparent);
    border-radius: var(--card-radius, var(--radius-panel));
    background: var(--card-bg);
    color: var(--card-text);
    cursor: default;
  }

  .list.cards li:hover {
    background: var(--card-hover-bg, var(--card-bg));
    border-color: var(--card-hover-border, var(--card-border));
    color: var(--card-hover-text, var(--card-text));
  }

  .list.cards li.chosen {
    background: var(--card-selected-bg, var(--card-bg));
    border-color: var(--card-selected-border, var(--card-border));
    color: var(--card-selected-text, var(--card-text));
  }

  .list .card-title,
  .list .card-line {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .list .card-title {
    font-weight: var(--card-weight, 400);
  }

  .list .card-line {
    font-size: var(--font-size-small);
    font-weight: var(--card-line-weight, 400);
    color: var(--card-line-text, inherit);
  }

  .list.cards li:hover .card-line {
    color: var(--card-line-hover-text, var(--card-line-text, inherit));
  }

  .list.cards li.chosen .card-line {
    color: var(--card-line-selected-text, var(--card-line-text, inherit));
  }

  /* Where a dragged row lands (DESIGN 5.30). */
  .list:global(.dragging-row) {
    cursor: grabbing;
  }

  .list li:global([data-drop="before"]) {
    box-shadow: inset 0 2px 0 var(--accent);
  }

  .list li:global([data-drop="after"]) {
    box-shadow: inset 0 -2px 0 var(--accent);
  }
</style>
