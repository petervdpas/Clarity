<script lang="ts">
  // Clarity's slider on the web, the same as on Avalonia
  // (conformance/ranges.json): dragged, it takes the nearest step when let
  // go; its keys move a step, ten, or to an end. Only a value that differs
  // is a change.
  import { dragged, keyed, rangeKey, rangeValue, type Range } from "../ranges";

  let {
    value,
    range,
    whole,
    id = undefined,
    label = undefined,
    tabindex = undefined,
    describedBy = undefined,
    disabled = false,
    readonly = false,
    onchange,
    onfocus = undefined,
    onblur = undefined,
  }: {
    value: string | number | null | undefined;
    range: Range;
    /** The field holds whole numbers: the value is a number, else text. */
    whole: boolean;
    id?: string;
    label?: string;
    tabindex?: number;
    describedBy?: string;
    disabled?: boolean;
    readonly?: boolean;
    /** The person moved the slider to another value: Clarity's `change`. */
    onchange: (next: string | number) => void;
    onfocus?: () => void;
    onblur?: () => void;
  } = $props();

  const shown = $derived(value === null || value === undefined || value === "" ? (range.min ?? "0") : String(value));

  function take(next: string) {
    if (readonly || next === shown) return;
    onchange(rangeValue(next, whole));
  }

  function key(e: KeyboardEvent) {
    const k = rangeKey(e.key);
    if (k === null) return;
    e.preventDefault();
    take(keyed(value ?? null, range, k));
  }
</script>

<input
  {id}
  type="range"
  class="slider"
  class:readonly
  min={range.min}
  max={range.max}
  step={range.step}
  value={shown}
  aria-label={label}
  {tabindex}
  aria-describedby={describedBy}
  aria-readonly={readonly}
  {disabled}
  onchange={(e) => take(dragged(e.currentTarget.value, range))}
  onkeydown={key}
  {onfocus}
  {onblur}
/>

<style>
  .slider {
    width: 100%;
    accent-color: var(--accent);
  }
  .readonly {
    pointer-events: none;
  }
</style>
