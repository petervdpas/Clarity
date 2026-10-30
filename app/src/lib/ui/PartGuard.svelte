<!--
  One part or region of a window, guarded: an error while drawing it shows in
  its own place (in the grid, for a part), with its name and the reason, and
  everything else keeps working. The same in the designer, the preview and a
  running application. No words of its own: the name and the reason say it.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  type Place = { col: number; row: number; width: number; height: number };

  let { name, place = null, children }: { name: string; place?: Place | null; children: Snippet } = $props();

  const reason = (error: unknown) => (error instanceof Error ? error.message : String(error));
</script>

<svelte:boundary onerror={(error) => console.error(`${name} could not be drawn:`, error)}>
  {@render children()}
  {#snippet failed(error)}
    <div
      class="part-failed"
      role="alert"
      title={reason(error)}
      style:grid-column={place ? `${place.col} / span ${place.width}` : undefined}
      style:grid-row={place ? `${place.row} / span ${place.height}` : undefined}
    >
      <strong>{name}</strong>: {reason(error)}
    </div>
  {/snippet}
</svelte:boundary>

<style>
  .part-failed {
    min-width: 0;
    overflow: hidden;
    padding: 4px 8px;
    border: 1px dashed var(--error);
    border-radius: 4px;
    color: var(--error);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
