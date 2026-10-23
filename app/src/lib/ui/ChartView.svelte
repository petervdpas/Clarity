<script lang="ts">
  // Clarity's chart on the web, the same as on Avalonia
  // (conformance/charts.json): bars or a line on an axis in steps of 1, 2,
  // 2.5 or 5 times a power of ten, or the slices of a pie in the palette's
  // colors. Each bar, point and slice tells its group and number when
  // pointed at.
  import { scaleOf, sliceColors } from "./charts";

  type Shown = { label: string; value: number; text: string };

  let {
    kind,
    groups,
    tick = String,
    label = undefined,
  }: {
    kind: "bar" | "line" | "pie";
    /** The groups in order: what each is called, its number, and that number as people read it. */
    groups: Shown[];
    /** A number of the axis as people read it. */
    tick?: (value: number) => string;
    label?: string;
  } = $props();

  let width = $state(0);
  let height = $state(0);

  const LEFT = 52;
  const BOTTOM = 22;
  const TOP = 8;
  const RIGHT = 8;

  const values = $derived(groups.map((g) => g.value));
  const scale = $derived(scaleOf(Math.min(0, ...values), Math.max(0, ...values)));
  const ticks = $derived.by(() => {
    const out: number[] = [];
    for (let v = scale.bottom; v <= scale.top + scale.step / 1e6; v += scale.step) out.push(Math.round(v * 1e12) / 1e12);
    return out;
  });
  const plotW = $derived(Math.max(0, width - LEFT - RIGHT));
  const plotH = $derived(Math.max(0, height - TOP - BOTTOM));
  const y = (v: number) => TOP + plotH * (1 - (v - scale.bottom) / (scale.top - scale.bottom));
  const band = $derived(groups.length ? plotW / groups.length : 0);
  const x = (i: number) => LEFT + band * (i + 0.5);

  // A pie: its slices from the top, clockwise.
  const colors = $derived(sliceColors(values));
  const radius = $derived(Math.max(0, Math.min(width * 0.6, height) / 2 - 8));
  const slices = $derived.by(() => {
    const total = values.reduce((sum, v) => sum + (v > 0 ? v : 0), 0);
    let start = -Math.PI / 2;
    return groups.map((g, i) => {
      const color = colors[i];
      if (!color || total <= 0) return null;
      const angle = (g.value / total) * Math.PI * 2;
      const end = start + angle;
      const cx = radius + 8;
      const cy = height / 2;
      const point = (a: number) => `${cx + radius * Math.cos(a)} ${cy + radius * Math.sin(a)}`;
      const d =
        angle >= Math.PI * 2 - 1e-9
          ? `M ${cx - radius} ${cy} a ${radius} ${radius} 0 1 0 ${radius * 2} 0 a ${radius} ${radius} 0 1 0 ${-radius * 2} 0`
          : `M ${cx} ${cy} L ${point(start)} A ${radius} ${radius} 0 ${angle > Math.PI ? 1 : 0} 1 ${point(end)} Z`;
      start = end;
      return { d, color, g };
    });
  });
</script>

<div class="chart" bind:clientWidth={width} bind:clientHeight={height}>
  <svg {width} {height} role="img" aria-label={label}>
    {#if kind === "pie"}
      {#each slices as slice, i (i)}
        {#if slice}
          <path d={slice.d} fill={slice.color}><title>{slice.g.label}: {slice.g.text}</title></path>
        {/if}
      {/each}
      {#each groups as g, i (i)}
        {#if colors[i]}
          <g transform="translate({radius * 2 + 28} {TOP + 16 * i})">
            <rect width="10" height="10" y="2" fill={colors[i]} />
            <text x="16" y="11" class="legend">{g.label}</text>
          </g>
        {/if}
      {/each}
    {:else}
      {#each ticks as t (t)}
        <line x1={LEFT} x2={LEFT + plotW} y1={y(t)} y2={y(t)} class:zero={t === 0} class="grid" />
        <text x={LEFT - 6} y={y(t) + 4} class="tick">{tick(t)}</text>
      {/each}
      {#each groups as g, i (i)}
        <text x={x(i)} y={TOP + plotH + 16} class="group">{g.label}</text>
      {/each}
      {#if kind === "bar"}
        {#each groups as g, i (i)}
          <rect x={x(i) - band * 0.3} width={band * 0.6} y={Math.min(y(g.value), y(0))} height={Math.abs(y(g.value) - y(0))} class="bar"
            ><title>{g.label}: {g.text}</title></rect
          >
        {/each}
      {:else}
        <polyline points={groups.map((g, i) => `${x(i)},${y(g.value)}`).join(" ")} class="line" />
        {#each groups as g, i (i)}
          <circle cx={x(i)} cy={y(g.value)} r="3.5" class="point"><title>{g.label}: {g.text}</title></circle>
        {/each}
      {/if}
    {/if}
  </svg>
</div>

<style>
  .chart {
    width: 100%;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  svg {
    display: block;
    font-size: 11px;
  }

  .grid {
    stroke: var(--border);
  }

  .grid.zero {
    stroke: var(--border-strong);
  }

  .tick {
    text-anchor: end;
    fill: var(--text-muted);
  }

  .group {
    text-anchor: middle;
    fill: var(--text-muted);
  }

  .legend {
    fill: var(--text);
  }

  .bar {
    fill: var(--accent);
  }

  .line {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
  }

  .point {
    fill: var(--accent);
  }
</style>
