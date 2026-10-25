<script lang="ts">
  // A trend (DESIGN 5.26): the last values of one number, taken every so
  // many seconds while the window is open, drawn as Clarity's line chart
  // with the newest at the right. The designer shows it waiting, still.
  import { onDestroy } from "svelte";
  import ChartView from "./ChartView.svelte";
  import { sampled, TREND_POINTS } from "./trend";

  let {
    value,
    points = null,
    every = null,
    top = null,
    running = true,
    label = undefined,
  }: {
    /** The number it follows, as the window holds it. */
    value: unknown;
    points?: number | null;
    /** Seconds between values, as written: `every 0.5`; one a second without it. */
    every?: string | null;
    top?: number | null;
    /** Whether it takes values: in a running window, not in the designer. */
    running?: boolean;
    label?: string;
  } = $props();

  let history = $state<number[]>([]);
  // The value read when the time comes, not the one it started with.
  let latest: unknown = null;
  $effect(() => {
    latest = value;
  });

  /** What the designer shows: a gentle line, still, so the part reads as a trend. */
  const SAMPLE = [42, 48, 45, 55, 60, 52, 58, 66, 61, 70, 64, 68];

  let timer: ReturnType<typeof setInterval> | undefined;
  $effect(() => {
    clearInterval(timer);
    if (!running) {
      history = SAMPLE;
      return;
    }
    const seconds = Number(every ?? "1");
    const kept = points ?? TREND_POINTS;
    history = sampled([], latest, kept);
    timer = setInterval(() => (history = sampled(history, latest, kept)), Math.max(100, seconds * 1000));
    return () => clearInterval(timer);
  });
  onDestroy(() => clearInterval(timer));
</script>

<ChartView kind="line" {label} {top} groups={history.map((v) => ({ label: "", value: v, text: String(v) }))} />
