<script lang="ts">
  // A window of the project drawn by Clarity.UI (DESIGN 8.1), as the
  // generated program draws it: in the preview it runs with sample data;
  // on the design canvas (`design`) it takes no input, and says where its
  // parts and grids are (`onlayout`) for the designer's layer over it. A
  // change of the project (`project`, a new value for each) or of what is
  // shown draws again on the same canvas, without loading the library again.
  import { onMount, untrack } from "svelte";
  import { browserFilesNotice, type BrowserFilesNotice } from "./browserFiles";
  import { browserFiles } from "./browserFilesStore.svelte";
  import { filesAddress, pageScript, previewAddress, startOptions, type Chosen, type ClarityUi, type Layout, type Shown, type Showing } from "./clarityWindow";

  let {
    shown,
    design = false,
    project,
    showing = {},
    chosen = {},
    onlayout,
  }: {
    shown: Shown;
    design?: boolean;
    /** The project as the IDE has it now: each new one draws the window again. */
    project: unknown;
    /** The page each container shows, the unsaved draft, the popovers open (the designer's). */
    showing?: Showing;
    /** The language the IDE shows the window in. */
    chosen?: Chosen;
    onlayout?: (layout: Layout) => void;
  } = $props();

  let canvas: HTMLCanvasElement | undefined = $state();
  let ui: ClarityUi | null = null;
  let base = "";
  let revision = 0;
  let problem = $state<string | null>(null);
  /** What Clarity says instead of drawing when the browser files are missing or behind. */
  let notice = $state<BrowserFilesNotice | null>(null);
  let ready = $state(false);

  /** Reads where the parts are, for the designer. */
  async function laid() {
    if (!ui || !onlayout) return;
    try {
      onlayout(await ui.layout());
    } catch (err) {
      problem = String(err);
    }
  }

  let stopped = false;
  let watching: ResizeObserver | null = null;

  /** Loads the library's page script and starts the drawing on the canvas. */
  async function begin() {
    try {
      base = await previewAddress();
      const page = await pageScript(base);
      if (stopped || !canvas) return;
      ui = page.start(canvas, untrack(() => startOptions(base, shown, design, revision, showing, chosen)));
      // A test of the IDE drives the drawing as a driven program (DESIGN 1.6) through its canvas.
      (canvas as HTMLCanvasElement & { clarityUi?: ClarityUi }).clarityUi = ui;
      await ui.started;
      if (stopped) return;
      ready = true;
      canvas.dataset.drawn = "";
      await laid();
      // A new size moves the parts: the designer reads them again.
      watching = new ResizeObserver(() => void laid());
      watching.observe(canvas);
    } catch (err) {
      problem = String(err instanceof Error ? err.message : err);
      await askFiles();
    }
  }

  /** Stops the drawing, if one runs. */
  function end() {
    watching?.disconnect();
    watching = null;
    ui?.stop();
    ui = null;
    ready = false;
  }

  /** The drawing could not start or draw: when the browser files are why, Clarity says so in its place. */
  async function askFiles() {
    const said = browserFilesNotice(await browserFiles.check());
    if (!stopped) notice = said;
  }

  onMount(() => {
    void begin();
    return () => {
      stopped = true;
      end();
    };
  });

  // The browser files were built again: the canvas loads them again, by itself.
  let generation = browserFiles.generation;
  $effect(() => {
    const now = browserFiles.generation;
    if (now === generation) return;
    generation = now;
    untrack(() => {
      end();
      problem = null;
      notice = null;
      if (canvas) delete canvas.dataset.drawn;
      void begin();
    });
  });

  // The project or what is shown changed: the same canvas draws it again.
  const key = $derived(JSON.stringify({ shown, showing, chosen }));
  let first = true;
  $effect(() => {
    void project;
    const now = JSON.parse(key) as { shown: Shown; showing: Showing; chosen: Chosen };
    if (first) {
      first = false;
      return;
    }
    untrack(() => void redraw(now));
  });

  async function redraw(now: { shown: Shown; showing: Showing; chosen: Chosen }) {
    if (!ui || !ready) return;
    revision += 1;
    const why = await ui.reload(filesAddress(base, now.shown, design, revision, now.showing), now.chosen);
    problem = why;
    if (why) await askFiles();
    else notice = null;
    await laid();
  }
</script>

<div class="drawn">
  <canvas bind:this={canvas} class:design></canvas>
  {#if notice}
    <div class="notice" role="alert">
      <p>{notice.text}</p>
      {#if browserFiles.failure}
        <p class="failure">{browserFiles.failure}</p>
      {/if}
      {#if notice.rebuild}
        <button type="button" disabled={browserFiles.rebuilding} onclick={() => void browserFiles.offer(notice!)}>{browserFiles.rebuilding ? "Rebuilding..." : "Rebuild"}</button>
      {/if}
    </div>
  {:else if problem}
    <p class="problem" role="alert">{problem}</p>
  {/if}
</div>

<style>
  .drawn {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }

  canvas {
    display: block;
    width: 100%;
    height: 100%;
  }

  .notice {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
    background: var(--bg);
    text-align: center;
  }

  .notice p {
    margin: 0;
    max-width: 480px;
    font-size: 13px;
  }

  .notice .failure {
    color: var(--error);
    font-size: 12px;
  }

  .problem {
    position: absolute;
    left: 8px;
    right: 8px;
    bottom: 8px;
    margin: 0;
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--error-soft);
    color: var(--error);
    font-size: 12px;
  }
</style>
