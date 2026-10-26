<script lang="ts">
  // A canvas an engine beside the screen draws (DESIGN 5.12): the engine
  // runs in a Web Worker of its own from when the canvas shows until it
  // goes, and every frame's shapes show through CanvasView. When the engine
  // stops (a frame failed or ran past its limit), the canvas says why. The
  // pointer and keys on it go to the engine, when it takes them: the canvas
  // keeps the pointer while it is down, and takes the keyboard when clicked
  // or tabbed to. In a running application, what handlers tell the canvas
  // (hears) goes to the engine, and what the engine tells runs the handler
  // wired to it.
  import { onDestroy, onMount } from "svelte";
  import CanvasView from "./CanvasView.svelte";
  import ScreenWorker from "../screenWorker.ts?worker&inline";
  import { HeldKeys, keyName, ScreenRun, type WorkerLike } from "../screenEngine";
  import { pointNear, pointOf } from "../drawing";
  import type { Hears, Shape } from "../engineHost";
  import type { ScreenEngine } from "../generated/ScreenEngine";
  import type { ShownText } from "../format";
  import { useLive } from "../live";
  import type { JsonValue } from "../generated/serde_json/JsonValue";

  let {
    engine,
    units,
    shown,
    language,
    id,
    part = null,
  }: {
    engine: ScreenEngine;
    units: { width: number; height: number };
    shown: (text: ShownText) => string;
    /** The language the screen shows now, as its tag; "" for the texts as written. */
    language: () => string;
    id?: string;
    /** The canvas's name, by which handlers tell it and its tells are wired. */
    part?: string | null;
  } = $props();

  const live = useLive();
  let connected: ReturnType<NonNullable<typeof live>["screen"]> | null = null;

  let drawing = $state<Shape[]>([]);
  let stopped = $state<string | null>(null);
  let hears = $state<Hears>({ pointer: false, keys: false });
  let run: ScreenRun | null = null;
  /** The pointer that is down on the drawing, which the canvas keeps. */
  let pressed: number | null = null;
  const held = new HeldKeys();

  function at(e: PointerEvent, near: boolean) {
    const box = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const where = { x: e.clientX - box.left, y: e.clientY - box.top };
    const part = { width: box.width, height: box.height };
    return near ? pointNear(units, part, where) : pointOf(units, part, where);
  }

  function down(e: PointerEvent) {
    const canvas = e.currentTarget as HTMLElement;
    // Not every page focuses what holds a drawing when it is clicked.
    if (hears.keys) canvas.focus();
    if (!hears.pointer || !e.isPrimary || e.button !== 0 || pressed !== null) return;
    const point = at(e, false);
    if (!point) return;
    // A drag moves what the engine drew; it selects no text.
    e.preventDefault();
    pressed = e.pointerId;
    canvas.setPointerCapture(e.pointerId);
    run?.pointer("down", point.x, point.y);
  }

  function move(e: PointerEvent) {
    if (!hears.pointer || !e.isPrimary || (pressed !== null && pressed !== e.pointerId)) return;
    const point = at(e, pressed !== null);
    if (point) run?.pointer("move", point.x, point.y);
  }

  function up(e: PointerEvent) {
    if (pressed !== e.pointerId) return;
    pressed = null;
    const point = at(e, true);
    if (point) run?.pointer("up", point.x, point.y);
  }

  function keyDown(e: KeyboardEvent) {
    const name = keyName(e.key);
    // A shortcut with Control, Alt or the command key is the window's.
    if (!hears.keys || name === null || ((e.ctrlKey || e.altKey || e.metaKey) && !["Control", "Alt", "Shift"].includes(name))) return;
    e.preventDefault();
    if (held.down(name)) run?.key("down", name);
  }

  function keyUp(e: KeyboardEvent) {
    const name = keyName(e.key);
    if (name !== null && held.up(name)) run?.key("up", name);
  }

  function letGo() {
    for (const name of held.all()) run?.key("up", name);
  }

  onMount(() => {
    run = new ScreenRun(engine, {
      worker: () => new ScreenWorker() as unknown as WorkerLike,
      frames: { request: (f) => requestAnimationFrame(f), cancel: (id) => cancelAnimationFrame(id) },
      timers: { set: (f, ms) => window.setTimeout(f, ms), clear: (id) => window.clearTimeout(id) },
      language,
      onDraw: (shapes) => (drawing = shapes),
      onFail: (problem) => (stopped = problem),
      onLog: (line) => console.info(line),
      onReady: (taken) => (hears = taken),
      onTell: (tell, values) => connected?.tell(tell, values as JsonValue[]),
    });
    run.start();
    if (live && part) connected = live.screen(part, (hear, values) => run?.hear(hear, values));
  });

  onDestroy(() => {
    connected?.stop();
    run?.stop();
  });
</script>

{#if stopped !== null}
  <div {id} class="engine-stopped" role="alert">
    {shown({ key: "screenEngineFailed", values: { problem: { kind: "text", value: stopped } } } as ShownText)}
  </div>
{:else}
  <!-- An application: the engine, not the page, decides what its pointer and keys do. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    {id}
    class="engine-canvas"
    class:pointing={hears.pointer}
    role="application"
    tabindex={hears.keys ? 0 : undefined}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onkeydown={keyDown}
    onkeyup={keyUp}
    onblur={letGo}
  >
    <CanvasView {drawing} {units} {shown} />
  </div>
{/if}

<style>
  .engine-canvas {
    width: 100%;
    height: 100%;
    outline: none;
    user-select: none;
    -webkit-user-select: none;
  }

  .engine-canvas:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .engine-canvas.pointing {
    touch-action: none;
  }

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
