<script lang="ts">
  // A graph of the last minute for Clarity settings, System: a line per
  // series, the newest reading at the right, the scale at the right edge.
  // A fixed scale (shares, up to 100) or one that follows the largest value.

  import { HISTORY } from "./systemLoad";

  let {
    series,
    max = null,
    scale,
    label,
  }: {
    series: { color: string; values: number[] }[];
    /** The top of the scale; none to follow the largest value. */
    max?: number | null;
    /** How a value on the scale is written. */
    scale: (value: number) => string;
    label: string;
  } = $props();

  const WIDTH = 600;
  const HEIGHT = 110;
  /** Lines across the graph, from the top. */
  const ROWS = 5;
  /** The time marks below it, from the left: a minute ago to ten seconds ago. */
  const MARKS = ["1 min", "50 secs", "40 secs", "30 secs", "20 secs", "10 secs"];

  const top = $derived.by(() => {
    if (max !== null) return max;
    const largest = Math.max(0, ...series.flatMap((s) => s.values));
    return largest > 0 ? largest * 1.25 : 1;
  });

  function points(values: number[]): string {
    const start = HISTORY - values.length;
    return values
      .map((v, i) => {
        const x = ((start + i) / (HISTORY - 1)) * WIDTH;
        const y = HEIGHT - (Math.min(v, top) / top) * HEIGHT;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  }
</script>

<figure class="chart" aria-label={label}>
  <div class="plot">
    <svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none" role="img" aria-label={label}>
      {#each { length: ROWS - 1 } as _, i (i)}
        <line class="grid" x1="0" x2={WIDTH} y1={((i + 1) * HEIGHT) / ROWS} y2={((i + 1) * HEIGHT) / ROWS} />
      {/each}
      {#each MARKS as _, i (i)}
        <line class="grid" y1="0" y2={HEIGHT} x1={(i * WIDTH) / MARKS.length} x2={(i * WIDTH) / MARKS.length} />
      {/each}
      {#each series as s, i (i)}
        {#if s.values.length > 1}
          <polyline points={points(s.values)} stroke={s.color} />
        {/if}
      {/each}
    </svg>
    <div class="scale">
      {#each { length: ROWS - 1 } as _, i (i)}
        <span style:top="{((i + 1) * 100) / ROWS}%">{scale((top * (ROWS - 1 - i)) / ROWS)}</span>
      {/each}
    </div>
  </div>
  <div class="marks">
    {#each MARKS as m (m)}<span>{m}</span>{/each}
  </div>
</figure>

<style>
  .chart {
    margin: 0;
  }
  .plot {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 70px;
  }
  svg {
    width: 100%;
    height: 110px;
    background: #1e1f22;
    display: block;
  }
  polyline {
    fill: none;
    stroke-width: 1.5;
    vector-effect: non-scaling-stroke;
  }
  .grid {
    stroke: #34363b;
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }
  .scale {
    position: relative;
    height: 110px;
    font-size: 10px;
    color: var(--text-muted);
  }
  .scale span {
    position: absolute;
    left: 6px;
    transform: translateY(-50%);
    white-space: nowrap;
  }
  .marks {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    margin-right: 70px;
    font-size: 10px;
    color: var(--text-muted);
  }
</style>
