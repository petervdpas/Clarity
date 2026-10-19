<script lang="ts">
  // A number in seven-segment digits, as on a calculator: the display part.
  // Lit segments take the text color; dark ones show faintly, as on an LCD.
  // The part's style gives its colors; Avalonia's SegmentDisplay draws the same.
  import { CELL_HEIGHT, CELL_WIDTH, POINT, SEGMENTS, SHAPES, cells, displayText } from "../display";

  let { value, digits, id }: { value: unknown; digits: number; id?: string } = $props();

  const text = $derived(displayText(value === null || value === undefined || value === "" ? "0" : String(value), digits));
  const shown = $derived(cells(text, digits));
  const points = (shape: [number, number][], at: number) => shape.map(([x, y]) => `${x + at * CELL_WIDTH},${y}`).join(" ");
</script>

<svg
  {id}
  class="segment-display"
  role="img"
  aria-label={text}
  viewBox="0 0 {shown.length * CELL_WIDTH + 1} {CELL_HEIGHT}"
  preserveAspectRatio="xMaxYMid meet"
>
  {#each shown as cell, at (at)}
    {#each Object.entries(SHAPES) as [name, shape] (name)}
      <polygon points={points(shape, at)} class:lit={SEGMENTS[cell.char]?.includes(name)} />
    {/each}
    <circle cx={POINT.x + at * CELL_WIDTH} cy={POINT.y} r={POINT.r} class:lit={cell.point} />
  {/each}
</svg>

<style>
  .segment-display {
    width: 100%;
    height: 100%;
    padding: 6px 10px;
    box-sizing: border-box;
    fill: currentColor;
  }

  polygon,
  circle {
    opacity: 0.1;
  }

  .lit {
    opacity: 1;
  }
</style>
