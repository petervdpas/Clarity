<script lang="ts">
  // A popover (DESIGN 5.16): parts on a grid of their own, shown next to the
  // part named in its `for`, below it, or above when there is no room. It is
  // closed until opened; a click outside it or Escape closes it. Live, the
  // part's state keeps whether it is open, so handlers open and close it
  // with `visible`. In the designer it shows while it or a part in it is picked.
  import { onMount } from "svelte";
  import GridBox from "./GridBox.svelte";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import type { Widget } from "./generated/Widget";
  import { useLive } from "./live";
  import { popoverShown } from "./overlays";
  import { pageDesigner } from "./pageDesignerStore.svelte";
  import { GAP, POPOVER_COLUMN, ROW_HEIGHT, type PageData } from "./pages";
  import { previewPopovers } from "./previewPopovers.svelte";

  let {
    preview,
    page,
    data,
    design,
    widget,
    onnavigate,
  }: {
    preview: Preview;
    page: Page;
    data: PageData;
    design: boolean;
    widget: Widget;
    onnavigate: (target: string, newWindow?: boolean) => void;
  } = $props();

  const p = $derived(widget.props);
  const live = useLive();
  const picked = $derived(design && pageDesigner.selected !== null && pageDesigner.selected >= widget.span.start && pageDesigner.selected < widget.span.end);
  const open = $derived(
    design ? picked : live && p.name ? popoverShown(live.parts[p.name]) : p.name ? previewPopovers[p.name] === true : false,
  );

  function close() {
    if (!p.name || design) return;
    if (live) live.setVisible(p.name, false);
    else previewPopovers[p.name] = false;
  }

  let box: HTMLElement | undefined = $state();
  let place = $state<{ left: number; top: number; width: number } | null>(null);

  /** Next to its part: below it, else above; its own columns of a popover's width. */
  function measure() {
    const anchor = p.for ? document.querySelector<HTMLElement>(`[data-part="${CSS.escape(p.for)}"]`) : null;
    if (!anchor) {
      place = null;
      return;
    }
    const at = anchor.getBoundingClientRect();
    const width = POPOVER_COLUMN * p.width + GAP * (p.width - 1);
    const height = box?.offsetHeight ?? ROW_HEIGHT * p.height;
    const below = at.bottom + 4;
    const top = below + height > window.innerHeight && at.top - height - 4 > 0 ? at.top - height - 4 : below;
    const left = Math.max(4, Math.min(at.left, window.innerWidth - width - 4));
    place = { left, top, width };
  }

  $effect(() => {
    if (!open) return;
    measure();
    const again = () => measure();
    window.addEventListener("resize", again);
    window.addEventListener("scroll", again, true);
    return () => {
      window.removeEventListener("resize", again);
      window.removeEventListener("scroll", again, true);
    };
  });

  onMount(() => {
    const away = (e: PointerEvent) => {
      if (!open || !box || box.contains(e.target as Node)) return;
      // A click on the part it opens next to is that part's own click.
      const anchor = p.for ? document.querySelector(`[data-part="${CSS.escape(p.for)}"]`) : null;
      if (anchor?.contains(e.target as Node)) return;
      close();
    };
    const key = (e: KeyboardEvent) => {
      if (open && e.key === "Escape") {
        e.stopPropagation();
        close();
      }
    };
    window.addEventListener("pointerdown", away, true);
    window.addEventListener("keydown", key, true);
    return () => {
      window.removeEventListener("pointerdown", away, true);
      window.removeEventListener("keydown", key, true);
    };
  });
</script>

{#if open}
  <div
    class="popover clarity-overlay"
    class:design
    class:selected={design && pageDesigner.selected === widget.span.start}
    role="dialog"
    tabindex="-1"
    aria-label={p.name ?? "popover"}
    data-part={p.name}
    data-widget={widget.span.start}
    bind:this={box}
    style:left="{place?.left ?? 0}px"
    style:top="{place?.top ?? 0}px"
    style:width={place ? `${place.width}px` : undefined}
    style:box-sizing="content-box"
    style:visibility={place ? "visible" : "hidden"}
    onpointerdown={(e) => {
      if (!design) return;
      e.stopPropagation();
      if (e.target === e.currentTarget) pageDesigner.selected = widget.span.start;
    }}
  >
    <GridBox {preview} {page} {data} {design} widgets={widget.children} columns={p.width} fixedRows={p.height} container={widget.span.start} {onnavigate} />
  </div>
{/if}

<style>
  .popover {
    position: fixed;
    z-index: 50;
    background: var(--panel);
    color: var(--text);
    border: var(--border-width) solid var(--border-strong);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow);
    padding: calc(var(--padding) * 2);
  }

  .popover.selected {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
</style>
