<!--
  What the IDE shows in its main area, guarded: an error while drawing it
  shows the error in its place, so the rest of Clarity keeps working.
  Another `shows` (another window or tab chosen) draws it afresh.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  let { shows, children }: { shows: string; children: Snippet } = $props();

  const reason = (error: unknown) => (error instanceof Error ? error.message : String(error));
</script>

{#key shows}
  <svelte:boundary onerror={(error) => console.error(`Clarity could not draw ${shows}:`, error)}>
    {@render children()}
    {#snippet failed(error, reset)}
      <div class="failed" role="alert">
        <p>This could not be drawn: {reason(error)}</p>
        <p class="hint">Clarity itself keeps working; please report this with the project it happened in.</p>
        <button type="button" onclick={reset}>Try again</button>
      </div>
    {/snippet}
  </svelte:boundary>
{/key}

<style>
  .failed {
    margin: auto;
    max-width: 36rem;
    text-align: center;
    color: var(--text);
  }

  .hint {
    color: var(--text-muted);
  }
</style>
