<script lang="ts">
  import type { PartContext } from "./context.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);

  // A splitter's panes: their columns as the backend's `pane_columns` gives
  // them (`pane span 4` side by side: 4 and the rest), and the share of the
  // first once the person drags the divider.
  function paneColumns(): number[] {
    const first = ctx.widget.pages[0]?.columns;
    const sideBySide = p.orientation !== "vertical";
    if (first != null && sideBySide && ctx.widget.pages.length === 2 && first >= 1 && first < p.width) return [first, p.width - first];
    return ctx.widget.pages.map(() => p.width);
  }

  let splitShare = $state<number | null>(null);
  let splitterBox: HTMLDivElement | undefined = $state();

  function paneGrow(index: number, columns: number[]): number {
    const total = columns.reduce((a, b) => a + b, 0);
    const first = splitShare ?? columns[0] / total;
    return index === 0 ? first : 1 - first;
  }

  function pressDivider(event: PointerEvent) {
    if (ctx.design || !splitterBox) return;
    const box = splitterBox;
    const divider = event.currentTarget as HTMLElement;
    divider.setPointerCapture(event.pointerId);
    const vertical = p.orientation === "vertical";
    const move = (e: PointerEvent) => {
      const r = box.getBoundingClientRect();
      const at = vertical ? (e.clientY - r.top) / r.height : (e.clientX - r.left) / r.width;
      splitShare = Math.min(0.9, Math.max(0.1, at));
    };
    const up = () => {
      divider.removeEventListener("pointermove", move);
      divider.removeEventListener("pointerup", up);
    };
    divider.addEventListener("pointermove", move);
    divider.addEventListener("pointerup", up);
  }

  const columns = $derived(paneColumns());
</script>

<div class="splitter {p.orientation ?? 'horizontal'}" bind:this={splitterBox}>
  {#each ctx.widget.pages as pane, index (pane.span.start)}
    {#if index > 0}<div class="divider" role="separator" onpointerdown={pressDivider}></div>{/if}
    <div class="pane" style:flex-grow={paneGrow(index, columns)}>
      {@render ctx.grid({ widgets: pane.children, columns: columns[index], container: pane.span.start })}
    </div>
  {/each}
</div>

<style>
  .splitter {
    flex: 1;
    display: flex;
    min-width: 0;
    border: var(--border-width) solid var(--border-strong);
    border-radius: var(--radius-panel);
    overflow: hidden;
  }

  .splitter.vertical {
    flex-direction: column;
  }

  .pane {
    flex: 1 1 0;
    min-width: 0;
    min-height: 0;
    padding: calc(var(--padding) * 2);
    overflow: hidden;
  }

  .divider {
    flex: none;
    background: var(--border-strong);
    width: 4px;
    cursor: col-resize;
    touch-action: none;
  }

  .splitter.vertical .divider {
    width: auto;
    height: 4px;
    cursor: row-resize;
  }
</style>
