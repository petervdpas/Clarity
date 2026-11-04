<script lang="ts">
  // A window of the project drawn by Clarity.UI (DESIGN 8.1), as the
  // generated program draws it: in the preview it runs with sample data;
  // on the design canvas (`design`) it takes no input, and says where its
  // parts and grids are (`onlayout`) for the designer's layer over it. A
  // change of the project (`project`, a new value for each) or of what is
  // shown draws again on the same canvas, without loading the library again.
  import { onMount, untrack } from "svelte";
  import { filesAddress, pageScript, previewAddress, startOptions, type ClarityUi, type Layout, type Shown } from "./clarityWindow";

  let {
    shown,
    design = false,
    project,
    onlayout,
  }: {
    shown: Shown;
    design?: boolean;
    /** The project as the IDE has it now: each new one draws the window again. */
    project: unknown;
    onlayout?: (layout: Layout) => void;
  } = $props();

  let canvas: HTMLCanvasElement | undefined = $state();
  let ui: ClarityUi | null = null;
  let base = "";
  let revision = 0;
  let problem = $state<string | null>(null);
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

  onMount(() => {
    let stopped = false;
    let watching: ResizeObserver | null = null;
    (async () => {
      try {
        base = await previewAddress();
        const page = await pageScript(base);
        if (stopped || !canvas) return;
        ui = page.start(canvas, startOptions(base, untrack(() => shown), design, revision));
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
      }
    })();
    return () => {
      stopped = true;
      watching?.disconnect();
      ui?.stop();
      ui = null;
    };
  });

  // The project or what is shown changed: the same canvas draws it again.
  const key = $derived(JSON.stringify(shown));
  let first = true;
  $effect(() => {
    void project;
    const now = JSON.parse(key) as Shown;
    if (first) {
      first = false;
      return;
    }
    untrack(() => void redraw(now));
  });

  async function redraw(now: Shown) {
    if (!ui || !ready) return;
    revision += 1;
    const why = await ui.reload(filesAddress(base, now, design, revision));
    problem = why;
    await laid();
  }
</script>

<div class="drawn">
  <canvas bind:this={canvas} class:design></canvas>
  {#if problem}
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
