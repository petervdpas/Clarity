<script lang="ts">
  // The DevTools section of Clarity settings, only in a Clarity run from
  // its repository (src-tauri/src/devtools): a tab per piece of Clarity an
  // app uses, the core ones always there and the others added by the cDdev
  // when they know that piece has changes, and the Release planning tab.
  // A piece's tab says what is in use (the release or the cDdev's own
  // build), signals when that build is behind its source, swaps, and has
  // Build, which makes the cDdev's own build and shows it while it builds.

  import { invoke } from "@tauri-apps/api/core";
  import type { DevToolsView } from "@clarity/ui/generated/DevToolsView.ts";
  import BuildLines from "./BuildLines.svelte";
  import ReleasePlanning from "./ReleasePlanning.svelte";
  import { browserFiles } from "./browserFilesStore.svelte";
  import { busy, devtools } from "./devtoolsStore.svelte";

  /** The release planning tab's key: no piece has it. */
  const RELEASE = "releasePlanning";

  let view = $state<DevToolsView | null>(null);
  let error = $state<string | null>(null);
  let working = $state(false);
  /** The tab shown, by its piece's key or RELEASE. */
  let shown = $state<string | null>(null);

  const tab = $derived(shown === RELEASE ? null : (view?.tabs.find((t) => t.key === shown) ?? view?.tabs[0] ?? null));
  const job = $derived(tab ? devtools.jobs[tab.job] : undefined);

  async function call(command: string, args: Record<string, unknown> = {}) {
    error = null;
    working = true;
    try {
      view = await invoke<DevToolsView | null>(command, args);
      if (view) devtools.know(view.jobs);
      // The design and preview draw with the browser files: swapped, every canvas loads them again.
      if (args.key === "clarityUiBrowser") browserFiles.reload();
    } catch (err) {
      error = String(err);
    } finally {
      working = false;
    }
  }

  $effect(() => {
    devtools.init();
    void call("devtools_view");
  });

  // A build of a piece ended: its tab reads how the piece stands again.
  let ended = new Set<string>();
  $effect(() => {
    for (const t of view?.tabs ?? []) {
      const j = devtools.jobs[t.job];
      const done = j && (j.state === "passed" || j.state === "failed") ? `${t.job}:${j.ended}` : null;
      if (done && !ended.has(done)) {
        ended.add(done);
        void call("devtools_view");
        if (t.key === "clarityUiBrowser" && t.mine && j?.state === "passed") browserFiles.reload();
      }
    }
  });

  async function build(key: string) {
    error = null;
    try {
      await invoke("devtools_build", { key });
    } catch (err) {
      error = String(err);
    }
  }

  function add(e: Event & { currentTarget: HTMLSelectElement }) {
    const key = e.currentTarget.value;
    e.currentTarget.value = "";
    if (!key) return;
    shown = key;
    void call("devtools_add_tab", { key });
  }

  function close(key: string) {
    if (shown === key) shown = null;
    void call("devtools_close_tab", { key });
  }

  function builtText(built: number | null): string {
    return built === null ? "" : new Date(built).toLocaleString(undefined, { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });
  }
</script>

<div class="devtools">
  <p class="muted">This Clarity runs from its repository. Each piece's tab chooses the release or your own build of it, to test apps with, and builds your own with Build. Release planning prepares the Release Helper's plan.</p>
  {#if error}<p class="refused" role="alert">{error}</p>{/if}
  {#if view}
    <div class="tabs" role="tablist" aria-label="DevTools">
      {#each view.tabs as t (t.key)}
        <span class="tab" class:active={tab?.key === t.key}>
          <button role="tab" aria-selected={tab?.key === t.key} onclick={() => (shown = t.key)}>
            {t.name}{#if busy(devtools.jobs[t.job])}<span class="spinner" title="Building"></span>{/if}{#if t.mine}<span class="mark" title="Your own build is in use">&#9679;</span>{/if}{#if t.signal}<span class="warn" title={t.signal}>!</span>{/if}
          </button>
          {#if !t.core}
            <button class="close" title="Close: {t.name} is the release again" aria-label="Close {t.name}" onclick={() => close(t.key)}>&times;</button>
          {/if}
        </span>
      {/each}
      {#if view.addable.length}
        <select class="add" aria-label="Add the tab of a piece that has changes" onchange={add} disabled={working}>
          <option value="">+ Add a piece</option>
          {#each view.addable as a (a.key)}<option value={a.key}>{a.name}</option>{/each}
        </select>
      {/if}
      <span class="tab release" class:active={shown === RELEASE}>
        <button role="tab" aria-selected={shown === RELEASE} onclick={() => (shown = RELEASE)}>Release planning</button>
      </span>
    </div>
    {#if shown === RELEASE}
      <div role="tabpanel" aria-label="Release planning">
        <ReleasePlanning />
      </div>
    {:else if tab}
      <div role="tabpanel" aria-label={tab.name}>
        <dl class="piece">
          <dt>Piece</dt>
          <dd><strong>{tab.name}</strong> <span class="muted">for {tab.uses}</span></dd>
          <dt>In use</dt>
          <dd>
            {#if tab.mine}
              your own build: <code>{tab.own}</code>{#if tab.built !== null} <span class="muted">(built {builtText(tab.built)})</span>{/if}
            {:else}
              the release {tab.release}
            {/if}
          </dd>
          <dt>State</dt>
          <dd>
            {#if busy(job)}<span class="muted">building</span>{:else if tab.signal}<span class="signal">{tab.signal}</span>{:else}<span class="muted">your own build is as new as its source</span>{/if}
          </dd>
          <dt>Swap</dt>
          <dd class="choices">
            <label><input type="radio" name="swap-{tab.key}" checked={!tab.mine} disabled={working} onchange={() => call("devtools_swap", { key: tab.key, mine: false })} /> the release {tab.release}</label>
            <label><input type="radio" name="swap-{tab.key}" checked={tab.mine} disabled={working} onchange={() => call("devtools_swap", { key: tab.key, mine: true })} /> your own build <code>{tab.own}</code></label>
          </dd>
          <dt>Build</dt>
          <dd class="choices">
            <span><button class="primary" onclick={() => build(tab.key)} disabled={busy(job)}>{busy(job) ? "Building..." : "Build"}</button> <code class="muted">{tab.build}</code></span>
            {#if job}<BuildLines {job} />{/if}
          </dd>
        </dl>
      </div>
    {/if}
    <p class="muted">Swaps kept in <code>{view.file}</code>.</p>
  {/if}
</div>

<style>
  .devtools {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .devtools > p {
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
  .tabs {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 2px;
    border-bottom: 1px solid var(--border);
  }
  .tab {
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--border);
    border-bottom: none;
    border-radius: 6px 6px 0 0;
    background: var(--surface-2, transparent);
  }
  .tab.release {
    margin-left: auto;
  }
  .tab.active {
    background: var(--surface, #fff);
    font-weight: 600;
  }
  .tab button {
    border: none;
    background: none;
    padding: 6px 10px;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }
  .tab .close {
    padding: 6px 8px 6px 0;
    color: var(--text-muted);
  }
  .mark {
    margin-left: 6px;
    color: var(--accent);
    font-size: 9px;
  }
  .warn {
    margin-left: 6px;
    color: var(--error);
    font-weight: 700;
  }
  .spinner {
    display: inline-block;
    width: 9px;
    height: 9px;
    margin-left: 6px;
    border: 2px solid var(--accent);
    border-right-color: transparent;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation: none;
    }
  }
  .add {
    margin: 0 0 4px 8px;
  }
  .piece {
    display: grid;
    grid-template-columns: 80px 1fr;
    gap: 8px 12px;
    margin: 0;
  }
  .piece dt {
    color: var(--text-muted);
  }
  .piece dd {
    margin: 0;
    min-width: 0;
  }
  .choices {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .signal {
    color: var(--error);
  }
</style>
