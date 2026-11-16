<script lang="ts">
  // DevTools' Release planning tab, for reviewing (src-tauri/src/devtools/release.rs):
  // the release checks as one outcome, with one button that runs them all;
  // the chain of steps a release would start (the checks on main, each tag,
  // its workflow and where it shows); and each release's changes since its
  // tag, code first. How each check ends is kept, and the agent reads it
  // with the rest through Clarity's MCP (release_state) to write the plan.

  import { invoke } from "@tauri-apps/api/core";
  import type { ReleaseView } from "@clarity/ui/generated/ReleaseView.ts";
  import BuildLines from "./BuildLines.svelte";
  import { changeText, checkState, stateText, verdict } from "./devtools";
  import { devtools } from "./devtoolsStore.svelte";

  let view = $state<ReleaseView | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  /** The release whose files show. */
  let opened = $state<string | null>(null);
  /** The check whose lines show. */
  let watched = $state<string | null>(null);

  async function load() {
    error = null;
    loading = true;
    try {
      view = await invoke<ReleaseView>("release_view");
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    devtools.init();
    void load();
  });

  const outcome = $derived(view ? verdict(view.checks, devtools.jobs) : null);
  const busy = $derived(!!outcome && (outcome.running !== null || view!.checks.some((c) => devtools.jobs[c.job]?.state === "waiting")));
  const shownJob = $derived(watched ? devtools.jobs[watched] : outcome?.running ? devtools.jobs[outcome.running.job] : undefined);
  const withCode = $derived(view?.releases.filter((r) => r.code.length) ?? []);
  const withoutCode = $derived(view?.releases.filter((r) => !r.code.length) ?? []);

  // A run ended: read again, so the kept outcomes show.
  let wasBusy = false;
  $effect(() => {
    if (wasBusy && !busy) void load();
    wasBusy = busy;
  });

  async function runAll() {
    if (!view) return;
    error = null;
    watched = null;
    try {
      await invoke("release_run", { jobs: view.checks.map((c) => c.job) });
    } catch (err) {
      error = String(err);
    }
  }

  function lastText(c: ReleaseView["checks"][number]): string {
    const job = devtools.jobs[c.job];
    if (job) return stateText(job);
    if (!c.last) return "not run";
    return `${c.last.passed ? "passed" : "failed"} ${new Date(c.last.ended).toLocaleString(undefined, { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" })}`;
  }
</script>

<div class="release">
  {#if error}<p class="refused" role="alert">{error}</p>{/if}
  {#if view && outcome}
    <section>
      <h3>Release checks <span class="muted">(changes since {view.since ?? "the start"})</span></h3>
      <div class="outcome" class:passed={outcome.allPassed} class:failed={outcome.failed.length > 0 && !busy} role="status">
        <strong>{outcome.text}</strong>
        <span class="bar">
          <button class="primary" onclick={runAll} disabled={busy}>{busy ? "Running..." : "Run release checks"}</button>
          <button onclick={load} disabled={loading}>Read again</button>
        </span>
      </div>
      {#if outcome.failed.length}
        <ul class="failed-list">
          {#each outcome.failed as f (f.job)}
            {@const c = view.checks.find((x) => x.job === f.job)}
            <li>
              <button class="link" onclick={() => (watched = watched === f.job ? null : f.job)}>{f.title}</button>
              {#if !devtools.jobs[f.job] && c?.last?.why}<span class="muted">{c.last.why}</span>{/if}
            </li>
          {/each}
        </ul>
      {/if}
      {#if shownJob}<BuildLines job={shownJob} />{/if}
      <details>
        <summary>All {outcome.total} checks</summary>
        <ul class="checks">
          {#each view.checks as c (c.job)}
            <li class={checkState(c, devtools.jobs[c.job])}>
              <button class="link" onclick={() => (watched = c.job)} title={c.command}>{c.title}</button>
              <span class="state">{lastText(c)}</span>
            </li>
          {/each}
        </ul>
      </details>
    </section>

    <section>
      <h3>What a release would start</h3>
      {#if view.chain.length <= 1}
        <p class="muted">No release has code changes since its tag: nothing to release.</p>
      {:else}
        <ol class="chain">
          {#each view.chain as step (step.title)}
            <li>
              <strong>{step.title}</strong>
              {#if step.tag}<code>{step.tag}</code>{/if}
              <ul>
                {#if step.workflow}<li>runs <code>{step.workflow}</code>{#if step.nextAfterWorkflow}<span class="muted">; the next step goes once it passed</span>{/if}</li>{/if}
                {#if step.shows}<li>shows on {step.shows}</li>{/if}
                {#if step.waitsFor.length}<li class="muted">waits for {step.waitsFor.join(", ")}</li>{/if}
              </ul>
            </li>
          {/each}
        </ol>
      {/if}
      <p class="muted">The agent reads this, the changes and the checks' outcome through Clarity's MCP (<code>release_state</code>) and writes the Release Helper's plan.</p>
    </section>

    <section>
      <h3>Changes since each release</h3>
      {#if withCode.length}
        <ul class="releases">
          {#each withCode as r (r.key)}
            <li>
              <button class="link strong" onclick={() => (opened = opened === r.key ? null : r.key)}>{r.name}</button>
              <span>{changeText(r)}</span>
              <span class="muted">since <code>{r.tag ?? "never released"}</code>{#if r.source}, {r.source} in source{/if}</span>
              {#if opened === r.key}
                <pre class="files">{[...r.code, ...r.manifest, ...r.tests, ...r.docs].join("\n")}</pre>
              {/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted">No release has code changes since its tag.</p>
      {/if}
      <details>
        <summary>No code changed ({withoutCode.length})</summary>
        <ul class="releases quiet">
          {#each withoutCode as r (r.key)}
            <li>
              <button class="link" onclick={() => (opened = opened === r.key ? null : r.key)} disabled={changeText(r) === "no changes"}>{r.name}</button>
              <span>{changeText(r)}</span>
              {#if opened === r.key}<pre class="files">{[...r.manifest, ...r.tests, ...r.docs].join("\n")}</pre>{/if}
            </li>
          {/each}
        </ul>
      </details>
    </section>
  {:else if loading}
    <p class="muted">Reading the repository...</p>
  {/if}
</div>

<style>
  .release {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h3 {
    margin: 0;
    font-size: 13px;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: normal;
  }
  .refused {
    color: var(--error);
    margin: 0;
  }
  .outcome {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-left: 4px solid var(--border-strong);
    border-radius: 6px;
  }
  .outcome.passed {
    border-left-color: var(--accent);
  }
  .outcome.failed {
    border-left-color: var(--error);
  }
  .bar {
    display: flex;
    gap: 8px;
  }
  .failed-list {
    margin: 0;
    padding-left: 18px;
    font-size: 12px;
    color: var(--error);
  }
  .failed-list li {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    font: inherit;
    text-align: left;
  }
  .link:disabled {
    color: inherit;
    cursor: default;
  }
  .strong {
    font-weight: 600;
  }
  details summary {
    cursor: pointer;
    font-size: 12px;
    color: var(--text-muted);
  }
  .checks {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    max-height: 260px;
    overflow: auto;
    font-size: 12px;
  }
  .checks li {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
    padding: 2px 0;
  }
  .checks li.passed .state {
    color: var(--accent);
  }
  .checks li.failed .state {
    color: var(--error);
  }
  .chain {
    margin: 0;
    padding-left: 22px;
    font-size: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .chain > li > code {
    margin-left: 6px;
  }
  .chain ul {
    margin: 2px 0 0;
    padding-left: 16px;
    border-left: 1px solid var(--border);
    list-style: none;
  }
  .releases {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .releases li {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: baseline;
  }
  .releases.quiet {
    margin-top: 6px;
    color: var(--text-muted);
  }
  .files {
    flex-basis: 100%;
    margin: 0;
    max-height: 160px;
    overflow: auto;
    font-size: 11px;
  }
</style>
