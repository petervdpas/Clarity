<script lang="ts">
  // Clarity's number input on the web, the same as on Avalonia
  // (conformance/forms.json, tables.json): the value shows in the person's
  // format and typed text is read the same way, when the field is left or
  // Enter is pressed. Text that is no number stays as typed and is marked,
  // and the value stays; only a number that differs is a change.
  // With a range it is a spin box (conformance/ranges.json): its arrows and
  // keys move it a step, or ten, and a typed number is kept within the range.
  import type { JsonValue } from "../generated/serde_json/JsonValue";
  import { keyed, rangeKey, rangeValue, typedIn, type Range } from "../ranges";

  let {
    value,
    text,
    read,
    id = undefined,
    label = undefined,
    placeholder = undefined,
    tabindex = undefined,
    describedBy = undefined,
    invalid = false,
    disabled = false,
    readonly = false,
    range = undefined,
    whole = true,
    onchange,
    onsubmit = undefined,
    onfocus = undefined,
    onblur = undefined,
  }: {
    value: JsonValue | undefined;
    /** The value as it is shown for editing. */
    text: (value: JsonValue | undefined) => string;
    /** What is typed as a value; null when it is no number. */
    read: (typed: string) => JsonValue | null;
    id?: string;
    label?: string;
    placeholder?: string;
    tabindex?: number;
    describedBy?: string;
    invalid?: boolean;
    disabled?: boolean;
    readonly?: boolean;
    /** A spin box's range. */
    range?: Range;
    /** The number is whole: kept as a number, else as text. */
    whole?: boolean;
    /** The person typed another number: Clarity's `change`. */
    onchange: (next: JsonValue) => void;
    /** Enter was pressed and the number taken: the form's `submit`; false when no form took it. */
    onsubmit?: () => boolean | void;
    onfocus?: () => void;
    onblur?: () => void;
  } = $props();

  /** What was typed while it is no number. */
  let bad = $state<string | null>(null);
  // A value set from elsewhere (a handler, a record read again) shows as it is.
  $effect(() => {
    void value;
    bad = null;
  });

  function take(typed: string) {
    const number = read(typed);
    if (number === null) {
      bad = typed;
      return;
    }
    bad = null;
    const next = range && (typeof number === "number" || typeof number === "string") ? rangeValue(typedIn(number, range), whole) : number;
    if (next !== (value ?? null)) onchange(next);
  }

  /** A spin box's arrow or key: a step, or ten, from the value. */
  function spin(k: "up" | "down" | "pageUp" | "pageDown") {
    if (!range || readonly || disabled) return;
    bad = null;
    const now = typeof value === "number" || typeof value === "string" ? value : null;
    const next = rangeValue(keyed(now, range, k), whole);
    if (next !== (value ?? null)) onchange(next);
  }

  function key(e: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    const k = range ? rangeKey(e.key) : null;
    if (k === "up" || k === "down" || k === "pageUp" || k === "pageDown") {
      // Left and right move the caret in the text.
      if (e.key === "ArrowLeft" || e.key === "ArrowRight") return;
      e.preventDefault();
      spin(k);
      return;
    }
    if (e.key !== "Enter") return;
    take(e.currentTarget.value);
    if (onsubmit && onsubmit() !== false) e.preventDefault();
  }
</script>

{#if range}
  <span class="spin">
    {@render field()}
    <span class="arrows">
      <button type="button" tabindex="-1" aria-hidden="true" {disabled} onclick={() => spin("up")}>
        <svg viewBox="0 0 10 6" width="10" height="6"><path d="M1 5 5 1 9 5" /></svg>
      </button>
      <button type="button" tabindex="-1" aria-hidden="true" {disabled} onclick={() => spin("down")}>
        <svg viewBox="0 0 10 6" width="10" height="6"><path d="M1 1 5 5 9 1" /></svg>
      </button>
    </span>
  </span>
{:else}
  {@render field()}
{/if}

{#snippet field()}
<input
  {id}
  class="numeric"
  class:bad={bad !== null}
  inputmode="decimal"
  aria-label={label}
  value={bad ?? text(value)}
  {placeholder}
  {tabindex}
  aria-describedby={describedBy}
  aria-invalid={invalid || bad !== null}
  title={bad !== null ? "Not a number" : undefined}
  {disabled}
  {readonly}
  onchange={(e) => take(e.currentTarget.value)}
  onkeydown={key}
  {onfocus}
  {onblur}
/>
{/snippet}

<style>
  .bad {
    border-color: var(--error);
  }
  .spin {
    display: flex;
    width: 100%;
    height: 100%;
  }
  .spin input {
    flex: 1;
    min-width: 0;
  }
  .arrows {
    display: flex;
    flex-direction: column;
    margin-left: 2px;
  }
  .arrows button {
    flex: 1;
    padding: 0 6px;
    min-height: 0;
    line-height: 0;
  }
  .arrows path {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
  }
</style>
