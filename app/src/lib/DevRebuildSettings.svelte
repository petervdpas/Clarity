<script lang="ts">
  // The Rebuild section of Clarity settings, only in an IDE run from its
  // repository: a tick for each of Clarity's own pieces to build again from
  // the source, and Restart, which builds what is ticked (the splash shows
  // each line), clears every tick and shows the IDE drawing with it.

  import { devRebuild } from "./devRebuildStore.svelte";

  $effect(() => {
    void devRebuild.load();
  });

  const anyTicked = $derived(devRebuild.view?.pieces.some((p) => p.ticked) ?? false);
</script>

<div class="rebuild">
  <p class="muted">This Clarity runs from its repository. Tick what changed in Clarity's source; Restart, or the next start of the IDE, builds it, with the splash showing the build, clears every tick and shows the IDE again with what was built. An agent that changed a piece ticks it for you.</p>
  {#if devRebuild.error}<p class="refused" role="alert">{devRebuild.error}</p>{/if}
  {#if devRebuild.failed}
    <div class="failed" role="alert">
      <p><strong>The rebuild failed.</strong> {devRebuild.failed.message}</p>
      <pre>{devRebuild.failed.lines.join("\n")}</pre>
      <p class="muted">The ticks stay; fix the source and restart again.</p>
    </div>
  {/if}

  {#if devRebuild.view}
    <fieldset>
      <legend>Build again from the source</legend>
      {#each devRebuild.view.pieces as piece (piece.key)}
        <label class="piece">
          <input type="checkbox" checked={piece.ticked} onchange={(e) => devRebuild.tick(piece.key, e.currentTarget.checked)} />
          <strong>{piece.name}</strong>
          <span class="muted">for {piece.uses}</span>
        </label>
      {/each}
    </fieldset>
    <div class="bar">
      <p class="muted">Kept in <code>{devRebuild.view.file}</code>.</p>
      <button class="primary" onclick={() => devRebuild.restart()} disabled={!anyTicked || devRebuild.restarting} title="Build what is ticked and show the IDE again">
        {devRebuild.restarting ? "Restarting..." : "Restart"}
      </button>
    </div>
  {/if}
</div>

<style>
  .rebuild {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .rebuild > p {
    margin: 0;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  .refused {
    color: var(--error);
    margin: 0;
  }
  .failed {
    border: 1px solid var(--error);
    border-radius: 6px;
    padding: 8px 12px;
  }
  .failed p {
    margin: 0 0 6px;
  }
  .failed pre {
    margin: 0 0 6px;
    max-height: 240px;
    overflow: auto;
    font-size: 11px;
    white-space: pre-wrap;
  }
  fieldset {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 12px;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  legend {
    font-size: 12px;
    color: var(--text-muted);
  }
  .piece {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .bar p {
    margin: 0;
  }
</style>
