<script lang="ts">
  import type { PartContext } from "./context.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<div class="buttons" role="group">
  {@render ctx.grid({ widgets: ctx.widget.children, columns: p.width, container: ctx.offset, fixedRows: 1, joined: true })}
</div>

<style>
  .buttons {
    flex: 1;
    display: flex;
    min-width: 0;
  }

  /* Joined: only the outer corners of the group are round. */
  .buttons :global(.widget .push) {
    border-radius: 0;
  }

  .buttons :global(.widget:not(:first-child) .push) {
    margin-left: calc(-1 * var(--border-width, 1px));
  }

  .buttons :global(.widget:first-child .push) {
    border-top-left-radius: var(--radius-button);
    border-bottom-left-radius: var(--radius-button);
  }

  .buttons :global(.widget:last-child .push) {
    border-top-right-radius: var(--radius-button);
    border-bottom-right-radius: var(--radius-button);
  }
</style>
