<script lang="ts">
  // A custom part (DESIGN 5.19): an outside library draws it in a native
  // window the shell keeps over this place. In a running application the
  // part follows where this box is and whether it shows, and gets its values;
  // on the design canvas and in the Preview it is a box with its name, or its
  // picture.

  import { onDestroy } from "svelte";
  import type { JsonValue } from "./generated/serde_json/JsonValue";
  import type { WidgetProps } from "./generated/WidgetProps";
  import type { Live } from "./live";
  import { OVERLAY_CLASS, changedValues, covered, placeOf, shownValues, type Place } from "./customParts";
  import { pictureUrl } from "./pictures.svelte";

  let {
    p,
    live,
    value,
    hidden,
    disabled,
    preview: picture,
    words,
  }: {
    p: WidgetProps;
    /** The running application; none on the design canvas and in the Preview. */
    live: Live | undefined;
    /** What it binds, or undefined when it binds nothing. */
    value: JsonValue | undefined;
    hidden: boolean;
    disabled: boolean;
    /** Its picture of the resources, from its declaration's `preview`. */
    preview: string | null;
    words: (text: string) => string;
  } = $props();

  let box = $state<HTMLDivElement | null>(null);
  const kind = $derived(p.part ?? "");
  const pictured = $derived(picture ? pictureUrl(picture) : null);

  // Where the part is and whether it shows, given again when either changes.
  let placed: { place: Place; shown: boolean } | null = null;
  let frame = 0;

  function follow() {
    frame = requestAnimationFrame(follow);
    if (!box || !live || !p.name) return;
    const rect = box.getBoundingClientRect();
    const overlays = [...document.getElementsByClassName(OVERLAY_CLASS)].map((e) => e.getBoundingClientRect());
    const shown = !hidden && box.offsetParent !== null && rect.width > 0 && rect.height > 0 && !covered(rect, overlays);
    const place = placeOf(rect, window.devicePixelRatio || 1);
    if (placed && placed.shown === shown && JSON.stringify(placed.place) === JSON.stringify(place)) return;
    placed = { place, shown };
    live.placePart(p.name, kind, place, shown);
  }

  $effect(() => {
    if (!live || !box) return;
    frame = requestAnimationFrame(follow);
    return () => cancelAnimationFrame(frame);
  });

  onDestroy(() => {
    if (live && p.name && placed) live.placePart(p.name, kind, placed.place, false);
  });

  // Its values: written on its line, set by code, what it binds and whether it can be used.
  let given: Record<string, JsonValue> | null = null;
  $effect(() => {
    if (!live || !p.name) return;
    const now = shownValues(p.values, live.parts[p.name]?.values, words);
    if (value !== undefined) now.value = value;
    now.enabled = !disabled;
    for (const [name, next] of changedValues(given, now)) live.setPartValue(p.name, name, next);
    given = now;
  });
</script>

<div class="custom-part" bind:this={box}>
  {#if !live}
    {#if pictured}
      <img src={pictured} alt={kind} />
    {:else}
      <span>{kind}</span>
    {/if}
  {/if}
</div>

<style>
  .custom-part {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--border, #999);
    box-sizing: border-box;
    overflow: hidden;
  }
  img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  span {
    opacity: 0.7;
  }
</style>
