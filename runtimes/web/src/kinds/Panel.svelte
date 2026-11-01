<script lang="ts">
  import { isFolded } from "../screen/folding.ts";
  import { tr } from "../screen/model.ts";
  import type { PartContext } from "./context.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);

  // A foldable panel or form: folded as the person or a handler left it, or as designed.
  let foldedHere = $state<boolean | null>(null);
  const folded = $derived(p.foldable && (ctx.live ? isFolded(p.folded, ctx.partState) : (foldedHere ?? p.folded)));

  function toggleFold() {
    const next = !folded;
    // Live, the part's state carries it to the handlers; in the preview it is the window's own.
    if (ctx.live && p.name) ctx.live.setFolded(p.name, next);
    else foldedHere = next;
  }
</script>

<fieldset class={p.kind} class:bare={p.kind === "form" && !p.caption && !p.foldable} class:folded={folded && !ctx.design}>
  {#if p.foldable}
    <!-- The person folds and unfolds it by its header; designing, it stays open to edit. -->
    <legend>
      <button type="button" class="fold" aria-expanded={!folded} onclick={toggleFold} disabled={ctx.design}>
        <span class="chevron" class:closed={folded}>&#9662;</span>{p.caption ? tr(ctx.preview, p.caption) : ""}
      </button>
    </legend>
  {:else if p.caption}<legend>{tr(ctx.preview, p.caption)}</legend>{/if}
  {#if !folded || ctx.design}
    {@render ctx.grid({ widgets: ctx.widget.children, columns: p.width, container: ctx.offset })}
  {/if}
</fieldset>

<style>
  /* A form groups its inputs; it shows a frame only when it has a caption,
     and without one its grid lines up with the grid around it. */
  fieldset.form.bare {
    border: none;
    margin: 0;
    padding: 0;
  }

  :global(.widget.design > .content) > fieldset.form.bare {
    outline: 1px dashed var(--border-strong);
  }

  /* A framed container's border and padding sit out in the space between
     columns (GAP 8px: 3px each side, leaving 2px between frames side by
     side), so its own grid is exactly as wide as its span and its column
     lines are the window's. */
  fieldset {
    flex: 1;
    min-width: 0;
    margin: 0 -3px;
    border: var(--border-width) solid var(--border-strong);
    border-radius: var(--radius-panel);
    padding: calc(var(--padding) * 2) calc(3px - var(--border-width));
  }

  legend {
    padding: 0 6px;
    font-size: var(--font-size-small);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  /* The header of a foldable panel or form, which folds and unfolds it. */
  legend .fold {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0;
    border: none;
    background: none;
    font: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    color: inherit;
    cursor: pointer;
  }

  legend .fold:disabled {
    cursor: inherit;
    opacity: 1;
  }

  .chevron {
    display: inline-block;
    transition: transform 0.1s;
  }

  .chevron.closed {
    transform: rotate(-90deg);
  }

  fieldset.folded {
    flex: none;
    align-self: flex-start;
    padding-bottom: 0;
  }
</style>
