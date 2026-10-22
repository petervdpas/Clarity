<script lang="ts">
  // A diagram drawn by Mermaid, bundled with Clarity and loaded the first
  // time a diagram opens. The drawing goes into a frame of its own, so the
  // IDE's styles cannot reach it: Ctrl+wheel zooms, dragging pans, and a
  // click on a box opens what it draws. Proposals mark their boxes: green
  // for what is added, amber for what changes, grey for what would go.

  import { onDestroy } from "svelte";
  import { boxName, themeVariables, type Mark } from "./diagramCanvas";

  let {
    text,
    names,
    marks = {},
    onopen = null,
  }: {
    /** The Mermaid text to draw. */
    text: string;
    /** The names of the boxes a click can open. */
    names: string[];
    marks?: Record<string, Mark>;
    /** A click on the box of `name`; null where nothing opens. */
    onopen?: ((name: string) => void) | null;
  } = $props();

  type Mermaid = (typeof import("mermaid"))["default"];
  let loading: Promise<Mermaid> | null = null;
  function mermaid(): Promise<Mermaid> {
    loading ??= import("mermaid").then((m) => m.default);
    return loading;
  }

  const ZOOM_MIN = 0.25;
  const ZOOM_MAX = 4;
  const ZOOM_STEP = 1.2;
  /** How far the pointer moves before a press is a drag, not a click. */
  const DRAG_SLOP = 4;
  const RENDER_DELAY = 250;

  let frame: HTMLIFrameElement | undefined = $state();
  let zoom = $state(1);
  let error = $state<string | null>(null);
  let drawn = $state(false);
  /** Changes when the IDE's theme does, so the drawing takes its colors. */
  let themeTick = $state(0);
  let renders = 0;

  const watcher = new MutationObserver(() => themeTick++);
  watcher.observe(document.documentElement, { attributes: true, attributeFilter: ["data-ide-theme", "data-scheme"] });
  onDestroy(() => watcher.disconnect());

  function frameDoc(): Document | null {
    return frame?.contentDocument ?? null;
  }

  /** Builds the frame's page once: its styles, the holder of the drawing, and its own pointer handling. */
  function prepare(doc: Document) {
    if (doc.getElementById("drawing")) return;
    const style = doc.createElement("style");
    style.textContent = `
      html, body { margin: 0; height: 100%; }
      body { overflow: auto; cursor: grab; background: transparent; }
      body.panning { cursor: grabbing; user-select: none; }
      #pad { box-sizing: border-box; min-width: 100%; min-height: 100%; padding: 24px; display: flex; align-items: center; justify-content: center; }
      #drawing svg { display: block; max-width: none !important; }
      .box { cursor: pointer; }
      .box:hover { filter: drop-shadow(0 0 2px #1f5fbf); }
      .mark-add { filter: drop-shadow(0 0 2px #2f8f4e) drop-shadow(0 0 2px #2f8f4e); }
      .mark-change { filter: drop-shadow(0 0 2px #c08a12) drop-shadow(0 0 2px #c08a12); }
      .mark-remove { opacity: 0.5; }
    `;
    doc.head.appendChild(style);
    const pad = doc.createElement("div");
    pad.id = "pad";
    const holder = doc.createElement("div");
    holder.id = "drawing";
    pad.appendChild(holder);
    doc.body.appendChild(pad);

    const scroller = (doc.scrollingElement ?? doc.documentElement) as HTMLElement;
    doc.addEventListener(
      "wheel",
      (e: WheelEvent) => {
        if (!(e.ctrlKey || e.metaKey)) return;
        e.preventDefault();
        setZoom(zoom * (e.deltaY < 0 ? ZOOM_STEP : 1 / ZOOM_STEP));
      },
      { passive: false },
    );
    let press: { x: number; y: number; left: number; top: number; moved: boolean } | null = null;
    doc.addEventListener("pointerdown", (e: PointerEvent) => {
      if (e.button !== 0) return;
      press = { x: e.clientX, y: e.clientY, left: scroller.scrollLeft, top: scroller.scrollTop, moved: false };
    });
    doc.addEventListener("pointermove", (e: PointerEvent) => {
      if (!press) return;
      const dx = e.clientX - press.x;
      const dy = e.clientY - press.y;
      if (!press.moved && Math.hypot(dx, dy) < DRAG_SLOP) return;
      press.moved = true;
      doc.body.classList.add("panning");
      scroller.scrollLeft = press.left - dx;
      scroller.scrollTop = press.top - dy;
    });
    const release = (e: PointerEvent) => {
      const wasClick = press !== null && !press.moved && e.type === "pointerup";
      press = null;
      doc.body.classList.remove("panning");
      if (!wasClick || !onopen) return;
      const box = (e.target as Element | null)?.closest?.(".box");
      const name = box?.getAttribute("data-box");
      if (name) onopen(name);
    };
    doc.addEventListener("pointerup", release);
    doc.addEventListener("pointerleave", release);
  }

  function setZoom(next: number) {
    zoom = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, next));
    const holder = frameDoc()?.getElementById("drawing");
    if (holder) holder.style.zoom = String(zoom);
  }

  /** Knows each drawn box by the name it draws, and marks it. */
  function label(doc: Document) {
    for (const node of doc.querySelectorAll("#drawing g.node, #drawing g[id*='entity-'], #drawing g[id*='classId-']")) {
      const name = boxName({ id: node.id, dataId: node.getAttribute("data-id"), text: node.textContent ?? "" }, names);
      if (!name || node.closest(".box") !== null) continue;
      node.classList.add("box");
      node.setAttribute("data-box", name);
      const mark = marks[name];
      if (mark) node.classList.add(`mark-${mark}`);
    }
  }

  async function draw(source: string) {
    const turn = ++renders;
    const doc = frameDoc();
    if (!doc?.body) return;
    prepare(doc);
    if (!source.trim()) {
      error = null;
      drawn = false;
      doc.getElementById("drawing")!.innerHTML = "";
      return;
    }
    const id = `clarity-diagram-${turn}`;
    try {
      const m = await mermaid();
      const css = getComputedStyle(document.documentElement);
      m.initialize({
        startOnLoad: false,
        securityLevel: "strict",
        theme: "base",
        themeVariables: { ...themeVariables((n) => css.getPropertyValue(n)), fontFamily: css.getPropertyValue("font-family") },
      });
      const { svg } = await m.render(id, source);
      if (turn !== renders) return;
      const holder = doc.getElementById("drawing")!;
      holder.innerHTML = svg;
      holder.style.zoom = String(zoom);
      label(doc);
      error = null;
      drawn = true;
    } catch (err) {
      if (turn !== renders) return;
      error = err instanceof Error ? err.message : String(err);
    } finally {
      document.getElementById(id)?.remove();
      document.getElementById(`d${id}`)?.remove();
    }
  }

  // Draws again when the text, the marks or the theme change, after typing rests.
  $effect(() => {
    const source = text;
    void marks;
    void names;
    void themeTick;
    if (!frame) return;
    const timer = setTimeout(() => draw(source), drawn ? RENDER_DELAY : 0);
    return () => clearTimeout(timer);
  });
</script>

<div class="canvas">
  <div class="zoom">
    <button onclick={() => setZoom(zoom / ZOOM_STEP)} title="Zoom out (Ctrl+wheel)" aria-label="Zoom out">-</button>
    <button class="level" onclick={() => setZoom(1)} title="Back to 100%">{Math.round(zoom * 100)}%</button>
    <button onclick={() => setZoom(zoom * ZOOM_STEP)} title="Zoom in (Ctrl+wheel)" aria-label="Zoom in">+</button>
  </div>
  <iframe bind:this={frame} title="Diagram" class:faded={error !== null && drawn}></iframe>
  {#if error}
    <p class="error" role="alert">Mermaid cannot draw this: {error}</p>
  {/if}
</div>

<style>
  .canvas {
    position: relative;
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }

  iframe {
    flex: 1;
    width: 100%;
    border: none;
    background: transparent;
  }

  iframe.faded {
    opacity: 0.4;
  }

  .zoom {
    position: absolute;
    top: 8px;
    right: 12px;
    display: flex;
    gap: 2px;
    z-index: 1;
  }

  .zoom button {
    min-width: 28px;
    padding: 2px 6px;
    font-size: 12px;
  }

  .zoom .level {
    min-width: 52px;
    font-variant-numeric: tabular-nums;
  }

  .error {
    margin: 0;
    padding: 8px 12px;
    border-top: 1px solid var(--border);
    background: var(--error-soft);
    color: var(--error);
    font-size: 12px;
    white-space: pre-wrap;
  }
</style>
