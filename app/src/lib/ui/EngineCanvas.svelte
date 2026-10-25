<script lang="ts">
  // A canvas an engine beside the screen draws (DESIGN 5.12): the engine
  // runs in a Web Worker of its own from when the canvas shows until it
  // goes, and every frame's shapes show through CanvasView. When the engine
  // stops (a frame failed or ran past its limit), the canvas says why.
  import { onDestroy, onMount } from "svelte";
  import CanvasView from "./CanvasView.svelte";
  import ScreenWorker from "../screenWorker.ts?worker&inline";
  import { ScreenRun, type WorkerLike } from "../screenEngine";
  import type { Shape } from "../engineHost";
  import type { ScreenEngine } from "../generated/ScreenEngine";
  import type { ShownText } from "../format";

  let {
    engine,
    units,
    shown,
    language,
    id,
  }: {
    engine: ScreenEngine;
    units: { width: number; height: number };
    shown: (text: ShownText) => string;
    /** The language the screen shows now, as its tag; "" for the texts as written. */
    language: () => string;
    id?: string;
  } = $props();

  let drawing = $state<Shape[]>([]);
  let stopped = $state<string | null>(null);
  let run: ScreenRun | null = null;

  onMount(() => {
    run = new ScreenRun(engine, {
      worker: () => new ScreenWorker() as unknown as WorkerLike,
      frames: { request: (f) => requestAnimationFrame(f), cancel: (id) => cancelAnimationFrame(id) },
      timers: { set: (f, ms) => window.setTimeout(f, ms), clear: (id) => window.clearTimeout(id) },
      language,
      onDraw: (shapes) => (drawing = shapes),
      onFail: (problem) => (stopped = problem),
      onLog: (line) => console.info(line),
    });
    run.start();
  });

  onDestroy(() => run?.stop());
</script>

{#if stopped !== null}
  <div {id} class="engine-stopped" role="alert">
    {shown({ key: "screenEngineFailed", values: { problem: { kind: "text", value: stopped } } } as ShownText)}
  </div>
{:else}
  <CanvasView {id} {drawing} {units} {shown} />
{/if}

<style>
  .engine-stopped {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    padding: 8px;
    box-sizing: border-box;
    text-align: center;
    color: var(--text-muted);
    border: var(--border-width) dashed var(--border);
    border-radius: var(--radius-panel);
  }
</style>
