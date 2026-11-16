<script lang="ts">
  // The DevTools section of Clarity settings, only in a Clarity run from
  // its repository (src-tauri/src/devtools): a tab per piece of Clarity an
  // app uses: shown by itself while its own build is behind its source
  // (marked !), else added by the cDdev when they know that piece has
  // changes (+ Add a piece) or kept while swapped to their own build; every
  // piece tab closes (its piece is the release again) until added again. Disk and Release
  // planning come first and stay,
  // then the pieces marked with a signal, then the others; when the tabs do
  // not fit, arrows cycle through them, keeping the one shown in view.
  // A piece's tab says what is in use (the release or the cDdev's own
  // build), signals when that build is behind its source, swaps, and has
  // Build, which makes the cDdev's own build and shows it while it builds.

  import { invoke } from "@tauri-apps/api/core";
  import type { DevToolsView } from "@clarity/ui/generated/DevToolsView.ts";
  import BuildLines from "./BuildLines.svelte";
  import DiskSettings from "./DiskSettings.svelte";
  import ReleasePlanning from "./ReleasePlanning.svelte";
  import { browserFiles } from "./browserFilesStore.svelte";
  import { busy, devtools } from "./devtoolsStore.svelte";
  import { DISK_TAB as DISK, RELEASE_TAB as RELEASE, cycle, tabOrder } from "./devtools";

  let view = $state<DevToolsView | null>(null);
  let error = $state<string | null>(null);
  let working = $state(false);
  /** The tab shown, by its piece's key, DISK or RELEASE. */
  let shown = $state<string>(DISK);

  const order = $derived(tabOrder(view?.tabs ?? []));
  const pieces = $derived(order.map((key) => view?.tabs.find((t) => t.key === key)).filter((t) => t !== undefined));
  const tab = $derived(view?.tabs.find((t) => t.key === shown) ?? null);
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
    if (shown === key) shown = DISK;
    void call("devtools_close_tab", { key });
  }

  // The piece tabs scroll sideways when they do not fit; the arrows show then.
  let strip: HTMLDivElement | undefined = $state();
  let canLeft = $state(false);
  let canRight = $state(false);
  function measure() {
    if (!strip) return;
    canLeft = strip.scrollLeft > 0;
    canRight = strip.scrollLeft + strip.clientWidth < strip.scrollWidth - 1;
  }
  function cycleTabs(step: number) {
    shown = cycle(order, shown, step);
  }
  $effect(() => {
    if (!strip) return;
    const observer = new ResizeObserver(measure);
    observer.observe(strip);
    for (const child of strip.children) observer.observe(child);
    measure();
    return () => observer.disconnect();
  });
  // The tab shown is scrolled into view.
  $effect(() => {
    void view?.tabs.length;
    void shown;
    const active = strip?.querySelector<HTMLElement>(".tab.active");
    active?.scrollIntoView({ block: "nearest", inline: "nearest" });
    measure();
  });

  function builtText(built: number | null): string {
    return built === null ? "" : new Date(built).toLocaleString(undefined, { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });
  }
</script>

<div class="devtools">
  <p class="muted">This Clarity runs from its repository. Each piece's tab chooses the release or your own build of it, to test apps with, and builds your own with Build. Release planning shows the release checks with one button, what a release would start and what changed since each release.</p>
  {#if error}<p class="refused" role="alert">{error}</p>{/if}
  {#if view}
    <div class="tabs" role="tablist" aria-label="DevTools">
      <div class="strip" bind:this={strip} onscroll={measure}>
        <span class="tab fixed" class:active={shown === DISK}>
          <button role="tab" aria-selected={shown === DISK} onclick={() => (shown = DISK)}>Disk</button>
        </span>
        <span class="tab fixed" class:active={shown === RELEASE}>
          <button role="tab" aria-selected={shown === RELEASE} onclick={() => (shown = RELEASE)}>Release planning</button>
        </span>
        {#each pieces as t (t.key)}
          <span class="tab" class:active={shown === t.key} title={t.auto ? `Shown while your own build of ${t.name} is behind its source` : undefined}>
            <button role="tab" aria-selected={shown === t.key} onclick={() => (shown = t.key)}>
              {t.name}{#if busy(devtools.jobs[t.job])}<span class="spinner" title="Building"></span>{/if}{#if t.mine}<span class="mark" title="Your own build is in use">&#9679;</span>{/if}{#if t.signal}<span class="warn" title={t.signal}>!</span>{/if}
            </button>
            <button class="close" title="Close: {t.name} is the release again" aria-label="Close {t.name}" onclick={() => close(t.key)}>&times;</button>
          </span>
        {/each}
      </div>
      {#if canLeft || canRight}
        <span class="cycle" role="group" aria-label="Go through the tabs">
          <button onclick={() => cycleTabs(-1)} aria-label="The tab before" title="The tab before"><span class="triangle left"></span></button>
          <button onclick={() => cycleTabs(1)} aria-label="The next tab" title="The next tab"><span class="triangle right"></span></button>
        </span>
      {/if}
      {#if view.addable.length}
        <select class="add" aria-label="Add the tab of a piece" onchange={add} disabled={working}>
          <option value="">+ Add a piece</option>
          {#each view.addable as a (a.key)}<option value={a.key}>{a.name}</option>{/each}
        </select>
      {/if}
    </div>
    {#if shown === RELEASE}
      <div role="tabpanel" aria-label="Release planning">
        <ReleasePlanning />
      </div>
    {:else if shown === DISK}
      <div role="tabpanel" aria-label="Disk">
        <DiskSettings />
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
    align-items: flex-end;
    gap: 2px;
    border-bottom: 1px solid var(--border);
    min-width: 0;
  }
  .strip {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    flex: 1 1 auto;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .strip::-webkit-scrollbar {
    display: none;
  }
  .cycle {
    flex: 0 0 auto;
    display: inline-flex;
    align-self: center;
    margin: 0 0 4px 2px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    overflow: hidden;
  }
  .cycle button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 0;
    background: var(--panel);
    color: var(--text);
    cursor: pointer;
  }
  .cycle button + button {
    border-left: 1px solid var(--border-strong);
  }
  .cycle button:hover {
    background: var(--panel-alt);
  }
  .triangle {
    width: 0;
    height: 0;
    border-top: 5px solid transparent;
    border-bottom: 5px solid transparent;
  }
  .triangle.left {
    border-right: 7px solid currentColor;
  }
  .triangle.right {
    border-left: 7px solid currentColor;
  }
  .tab {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--border);
    border-bottom: none;
    border-radius: 6px 6px 0 0;
    background: var(--surface-2, transparent);
  }
  .tab.fixed button {
    font-weight: 600;
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
    flex: 0 0 auto;
    align-self: center;
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
