<script lang="ts">
  // A long engine call on the screen (DESIGN 5.12): a bar of how far it is,
  // `done` of `total`, and a Stop button named in Clarity's own word. The
  // shell sends how far the call is while the handler runs; Stop asks it
  // once to stop the call, which then fails with "it was stopped". The same
  // as on Avalonia.

  let {
    done,
    total,
    words,
    onstop,
  }: {
    done: number;
    total: number;
    /** Clarity's own word, in the language shown. */
    words: { stop: string };
    onstop: () => void;
  } = $props();

  let stopping = $state(false);
  const share = $derived(total > 0 ? Math.min(100, Math.max(0, (done / total) * 100)) : 0);

  function stop() {
    if (stopping) return;
    stopping = true;
    onstop();
  }
</script>

<div class="long-call">
  <div class="bar" role="progressbar" aria-valuemin="0" aria-valuenow={done} aria-valuemax={total}>
    <div class="done" style:width="{share}%"></div>
  </div>
  <button type="button" disabled={stopping} onclick={stop}>{words.stop}</button>
</div>

<style>
  .long-call {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 10px 10px 14px;
    border-radius: var(--radius-panel);
    border: var(--border-width) solid var(--border-strong);
    background: var(--panel);
    color: var(--text);
    box-shadow: var(--shadow);
  }
  .bar {
    flex: 1;
    min-width: 160px;
    height: 8px;
    border-radius: 4px;
    background: var(--border-strong);
    overflow: hidden;
  }
  .done {
    height: 100%;
    background: var(--accent);
    transition: width 0.1s linear;
  }
</style>
