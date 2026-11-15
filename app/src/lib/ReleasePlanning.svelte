<script lang="ts">
  // DevTools' Release planning tab (src-tauri/src/devtools/release.rs):
  // each of Clarity's releases as the repository has it (its newest tag,
  // the files changed since, the version in source) with Bump; the checks a
  // release needs first, run as builds one at a time; and Write plan, which
  // writes the Release Helper's plan into releases/. A check that has not
  // passed holds the plan until the person decides to write it anyway.

  import { invoke } from "@tauri-apps/api/core";
  import type { ReleaseView } from "@clarity/ui/generated/ReleaseView.ts";
  import BuildLines from "./BuildLines.svelte";
  import { dialogs } from "./dialogStore.svelte";
  import { includedAtFirst, openChecks, planStamp, stateText } from "./devtools";
  import { busy, devtools } from "./devtoolsStore.svelte";

  let view = $state<ReleaseView | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  /** Per release: in the plan, its version and its note. */
  let included = $state<Record<string, boolean>>({});
  let versions = $state<Record<string, string>>({});
  let notes = $state<Record<string, string>>({});
  /** The release whose changed files show. */
  let opened = $state<string | null>(null);
  /** The check whose lines show. */
  let watched = $state<string | null>(null);
  let title = $state("");
  let message = $state("");
  let written = $state<string | null>(null);

  async function load(command = "release_view", args: Record<string, unknown> = {}) {
    error = null;
    loading = true;
    try {
      const next = await invoke<ReleaseView>(command, args);
      for (const row of next.rows) {
        if (!(row.key in included) || command === "release_bump") included[row.key] = included[row.key] ?? includedAtFirst(row);
        if (!(row.key in versions) || (command === "release_bump" && args.key === row.key)) versions[row.key] = row.next;
        notes[row.key] = notes[row.key] ?? "";
      }
      view = next;
      if (!title) title = `Clarity ${new Date().toISOString().slice(0, 10)}`;
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

  const open = $derived(view ? openChecks(view.checks, devtools.jobs) : []);
  const anyRunning = $derived(view?.checks.some((c) => busy(devtools.jobs[c.job])) ?? false);
  const watchedJob = $derived(watched ? devtools.jobs[watched] : undefined);

  async function run(jobs: string[]) {
    error = null;
    if (jobs.length) watched = jobs[0];
    try {
      await invoke("release_run", { jobs });
    } catch (err) {
      error = String(err);
    }
  }

  async function write() {
    if (!view) return;
    const releases = view.rows.filter((r) => included[r.key]).map((r) => ({ key: r.key, version: versions[r.key], note: notes[r.key] ?? "" }));
    let anyway = false;
    if (open.length) {
      anyway = await dialogs.confirm({
        title: "Write the plan",
        text: `${open.length} checks have not passed: ${open.join(", ")}. Write the plan anyway?`,
        action: "Write anyway",
        danger: true,
      });
      if (!anyway) return;
    }
    error = null;
    try {
      written = await invoke<string>("release_write_plan", { title, message, stamp: planStamp(new Date()), releases, anyway });
    } catch (err) {
      error = String(err);
    }
  }
</script>

<div class="release">
  {#if error}<p class="refused" role="alert">{error}</p>{/if}
  {#if view}
    <section>
      <h3>Releases</h3>
      <table>
        <thead>
          <tr><th>In plan</th><th>Release</th><th>Released</th><th>In source</th><th>Changed</th><th>Version</th><th>Note</th></tr>
        </thead>
        <tbody>
          {#each view.rows as row (row.key)}
            <tr class:quiet={!included[row.key]}>
              <td><input type="checkbox" bind:checked={included[row.key]} aria-label="{row.name} in the plan" /></td>
              <td>{row.name}</td>
              <td><code>{row.tag ?? "never"}</code></td>
              <td>
                {#if row.source}{row.source}{#if row.bumped} <span class="ok">bumped</span>{/if}{:else}<span class="muted">the tag</span>{/if}
                {#if row.bumpable && !row.bumped}<button onclick={() => load("release_bump", { key: row.key })} disabled={loading} title="Raise the version in source to the next minor one">Bump</button>{/if}
              </td>
              <td>
                {#if row.changed.length}
                  <button class="link" onclick={() => (opened = opened === row.key ? null : row.key)}>{row.changed.length} files</button>
                {:else}<span class="muted">none</span>{/if}
              </td>
              <td><input class="version" bind:value={versions[row.key]} aria-label="{row.name}'s version" /></td>
              <td><input class="note" bind:value={notes[row.key]} aria-label="{row.name}'s note" /></td>
            </tr>
            {#if opened === row.key}
              <tr><td colspan="7"><pre class="files">{row.changed.join("\n")}</pre></td></tr>
            {/if}
          {/each}
        </tbody>
      </table>
      <p class="muted">A release whose version in source is not past its tag needs a Bump before its plan; the IDE's version is its tag alone.</p>
    </section>

    <section>
      <h3>Checks <span class="muted">(changes since {view.since ?? "the start"})</span></h3>
      <div class="bar">
        <button class="primary" onclick={() => run(open.length ? view!.checks.filter((c) => devtools.jobs[c.job]?.state !== "passed").map((c) => c.job) : [])} disabled={anyRunning || !open.length}>
          {anyRunning ? "Running..." : `Run the ${open.length} open`}
        </button>
        <button onclick={() => load()} disabled={loading}>Read again</button>
      </div>
      <ul class="checks">
        {#each view.checks as check (check.job)}
          {@const j = devtools.jobs[check.job]}
          <li class={j?.state ?? "none"}>
            <button class="link" onclick={() => (watched = check.job)} title={check.command}>{check.title}</button>
            <span class="state">{stateText(j)}</span>
            <button onclick={() => run([check.job])} disabled={busy(j)}>Run</button>
          </li>
        {/each}
      </ul>
      {#if watchedJob}<BuildLines job={watchedJob} />{/if}
    </section>

    <section>
      <h3>Plan</h3>
      <label>Title <input bind:value={title} /></label>
      <label>Commit message <textarea rows="6" bind:value={message}></textarea></label>
      <div class="bar">
        <button class="primary" onclick={write} disabled={!view.rows.some((r) => included[r.key]) || !message.trim()}>Write plan</button>
        {#if open.length}<span class="muted">{open.length} checks have not passed</span>{/if}
      </div>
      {#if written}<p>Written: <code>{written}</code>. Commit, then start the Release Helper with it.</p>{/if}
    </section>
  {:else if loading}
    <p class="muted">Reading the repository...</p>
  {/if}
</div>

<style>
  .release {
    display: flex;
    flex-direction: column;
    gap: 16px;
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
  .ok {
    color: var(--accent);
    font-size: 11px;
  }
  table {
    border-collapse: collapse;
    font-size: 12px;
  }
  th,
  td {
    text-align: left;
    padding: 3px 6px;
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
  }
  tr.quiet td {
    color: var(--text-muted);
  }
  .version {
    width: 70px;
  }
  .note {
    width: 100%;
    min-width: 120px;
  }
  .files {
    margin: 0;
    max-height: 160px;
    overflow: auto;
    font-size: 11px;
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
  .bar {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .checks {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 260px;
    overflow: auto;
    font-size: 12px;
  }
  .checks li {
    display: grid;
    grid-template-columns: 1fr 70px auto;
    gap: 8px;
    align-items: center;
    padding: 2px 0;
  }
  .checks li.passed .state {
    color: var(--accent);
  }
  .checks li.failed .state {
    color: var(--error);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
  }
  textarea {
    font: 12px ui-monospace, monospace;
  }
</style>
