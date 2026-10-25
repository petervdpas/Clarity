<script lang="ts">
  // The System section of Clarity settings: the computer's load over the
  // last minute, with Clarity's own share. It is loaded only when opened,
  // and the backend reads the cores, Clarity's processes and the network
  // only while it is open (watchSystem); the status bar's meters need less.

  import { onDestroy, onMount } from "svelte";
  import { invoke } from "./backend";
  import LoadChart from "./LoadChart.svelte";
  import type { MachineFacts } from "./generated/MachineFacts";
  import type { DiskSpace } from "./generated/DiskSpace";
  import { bytes, percent } from "./systemLoad";
  import { systemLoad, watchSystem } from "./systemLoad.svelte";

  /** A color per core, again from the first for a ninth. */
  const CORE_COLORS = ["#e5262f", "#f57c12", "#f5d22a", "#3ddc84", "#2a9d63", "#6aa8f0", "#1f6fe0", "#8e44ad"];
  const CLARITY_COLOR = "#ffffff";
  const MEMORY_COLOR = "#e5262f";
  const SWAP_COLOR = "#3ddc84";
  const IN_COLOR = "#3b82f6";
  const OUT_COLOR = "#f57c12";

  let facts = $state<MachineFacts | null>(null);
  let disks = $state<DiskSpace[] | null>(null);

  /** How often the disks' space is asked again while the page is open; it changes slowly. */
  const SPACE_EVERY_MS = 10_000;
  let spaceTimer: ReturnType<typeof setInterval> | undefined;

  function askSpace() {
    invoke<DiskSpace[]>("disk_space").then((d) => (disks = d)).catch(() => {});
  }

  onMount(() => {
    watchSystem(true);
    invoke<MachineFacts>("machine_facts").then((f) => (facts = f)).catch(() => {});
    askSpace();
    spaceTimer = setInterval(askSpace, SPACE_EVERY_MS);
  });
  onDestroy(() => {
    watchSystem(false);
    clearInterval(spaceTimer);
  });

  const history = $derived(systemLoad.history);
  const last = $derived(history.at(-1) ?? null);
  const share = (part: number, whole: number) => (whole > 0 ? (part * 100) / whole : 0);

  const cores = $derived(
    (last?.cores ?? []).map((_, i) => ({
      color: CORE_COLORS[i % CORE_COLORS.length],
      values: history.map((r) => r.cores[i] ?? 0),
    })),
  );
  const perSecond = (n: number) => `${bytes(n)}/s`;
</script>

<div class="system">
  {#if facts}
    <dl class="facts" aria-label="This computer">
      <dt>Machine</dt>
      <dd>{facts.machine || "Not named by its firmware"}</dd>
      <dt>User</dt>
      <dd>{facts.user || "-"}</dd>
      <dt>Host name</dt>
      <dd>{facts.host || "-"}</dd>
      <dt>Processor</dt>
      <dd>{facts.processor || "Not named"}, {facts.cores} {facts.cores === 1 ? "core" : "cores"}</dd>
      <dt>Memory</dt>
      <dd>{bytes(facts.memory)}</dd>
      <dt>System</dt>
      <dd>{facts.system || "-"}</dd>
    </dl>
  {/if}
  <p class="muted">How busy this computer is, over the last minute, and Clarity's share of it.</p>

  {#if !last}
    <p class="muted">Reading the load...</p>
  {:else}
    <section>
      <h4>Processors</h4>
      <LoadChart
        label="Each processor's load over the last minute"
        max={100}
        scale={(v) => `${Math.round(v)} %`}
        series={[...cores, { color: CLARITY_COLOR, values: history.map((r) => r.clarity_cpu) }]}
      />
      <ul class="legend">
        {#each last.cores as value, i (i)}
          <li><span class="swatch" style:background={CORE_COLORS[i % CORE_COLORS.length]}></span>CPU{i + 1} <span class="figure">{percent(value)}</span></li>
        {/each}
        <li class="whole"><span class="swatch" style:background={CLARITY_COLOR}></span>Clarity <span class="figure">{percent(last.clarity_cpu)} of all</span></li>
      </ul>
      <p class="muted">
        Clarity is {last.clarity_processes}
        {last.clarity_processes === 1 ? "process" : "processes"}: the IDE itself and the builds and applications it has started, with whatever they started in turn.
      </p>
    </section>

    <section>
      <h4>Memory and swap</h4>
      <LoadChart
        label="Memory and swap in use over the last minute"
        max={100}
        scale={(v) => `${Math.round(v)} %`}
        series={[
          { color: MEMORY_COLOR, values: history.map((r) => share(r.memory_used, r.memory_total)) },
          { color: SWAP_COLOR, values: history.map((r) => share(r.swap_used, r.swap_total)) },
          { color: CLARITY_COLOR, values: history.map((r) => share(r.clarity_memory, r.memory_total)) },
        ]}
      />
      <div class="figures">
        <dl>
          <dt><span class="swatch" style:background={MEMORY_COLOR}></span>Memory</dt>
          <dd>{bytes(last.memory_used)} ({percent(share(last.memory_used, last.memory_total))}) of {bytes(last.memory_total)}</dd>
          <dt><span class="swatch" style:background={CLARITY_COLOR}></span>Clarity</dt>
          <dd>{bytes(last.clarity_memory)} ({percent(share(last.clarity_memory, last.memory_total))})</dd>
        </dl>
        <dl>
          <dt><span class="swatch" style:background={SWAP_COLOR}></span>Swap</dt>
          <dd>{last.swap_total > 0 ? `${bytes(last.swap_used)} (${percent(share(last.swap_used, last.swap_total))}) of ${bytes(last.swap_total)}` : "None"}</dd>
        </dl>
      </div>
    </section>

    <section>
      <h4>Network</h4>
      <LoadChart
        label="Network traffic over the last minute"
        scale={perSecond}
        series={[
          { color: IN_COLOR, values: history.map((r) => r.received) },
          { color: OUT_COLOR, values: history.map((r) => r.sent) },
        ]}
      />
      <div class="figures">
        <dl>
          <dt><span class="swatch" style:background={IN_COLOR}></span>Receiving</dt>
          <dd>{perSecond(last.received)}</dd>
          <dt>Total received</dt>
          <dd>{bytes(last.received_total)}</dd>
        </dl>
        <dl>
          <dt><span class="swatch" style:background={OUT_COLOR}></span>Sending</dt>
          <dd>{perSecond(last.sent)}</dd>
          <dt>Total sent</dt>
          <dd>{bytes(last.sent_total)}</dd>
        </dl>
      </div>
    </section>

    <section>
      <h4>Disk</h4>
      <LoadChart
        label="Disk activity over the last minute"
        scale={perSecond}
        series={[
          { color: IN_COLOR, values: history.map((r) => r.read) },
          { color: OUT_COLOR, values: history.map((r) => r.written) },
        ]}
      />
      <div class="figures">
        <dl>
          <dt><span class="swatch" style:background={IN_COLOR}></span>Reading</dt>
          <dd>{perSecond(last.read)}</dd>
          <dt>Total read</dt>
          <dd>{bytes(last.read_total)}</dd>
        </dl>
        <dl>
          <dt><span class="swatch" style:background={OUT_COLOR}></span>Writing</dt>
          <dd>{perSecond(last.written)}</dd>
          <dt>Total written</dt>
          <dd>{bytes(last.written_total)}</dd>
        </dl>
      </div>
    </section>
  {/if}

  {#if disks && disks.length > 0}
    <section>
      <h4>Disk space</h4>
      <ul class="disks">
        {#each disks as d (d.name)}
          {@const used = share(d.total - d.available, d.total)}
          <li>
            <div class="disk-head">
              <span><strong>{d.mount}</strong> <span class="muted">{d.name}{d.removable ? ", removable" : ""}</span></span>
              <span class="space"><strong>{bytes(d.available)} available</strong> <span class="muted">of {bytes(d.total)}</span></span>
            </div>
            <div class="fill-bar" role="meter" aria-label="{d.mount} in use" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(used)} title="{percent(used)} in use">
              <span class:full={used > 90} style:width="{used}%"></span>
            </div>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</div>

<style>
  .system {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  p {
    margin: 0;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h4 {
    margin: 0;
    font-size: 13px;
  }
  .legend {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
    gap: 4px 12px;
    font-size: 12px;
  }
  .legend li {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .legend .whole {
    grid-column: 1 / -1;
  }
  .swatch {
    display: inline-block;
    width: 18px;
    height: 10px;
    border-radius: 2px;
    border: 1px solid var(--border);
    margin-right: 6px;
    vertical-align: middle;
  }
  .legend .swatch {
    margin-right: 0;
  }
  .figure {
    font-variant-numeric: tabular-nums;
  }
  .figures {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 2px 12px;
    margin: 0;
    font-size: 12px;
  }
  dt {
    color: var(--text-muted);
  }
  .facts {
    grid-template-columns: max-content 1fr;
    gap: 4px 16px;
    font-size: 13px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  .disks {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .disk-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    font-size: 12px;
    margin-bottom: 4px;
  }
  .space {
    text-align: right;
  }
  .fill-bar {
    height: 6px;
    border-radius: 3px;
    background: var(--border);
    overflow: hidden;
  }
  .fill-bar span {
    display: block;
    height: 100%;
    background: #3b82f6;
  }
  .fill-bar span.full {
    background: var(--error);
  }
  dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
</style>
