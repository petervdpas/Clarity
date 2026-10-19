<script lang="ts">
  // Clarity's number input on the web, the same as on Avalonia
  // (conformance/forms.json, tables.json): the value shows in the person's
  // format and typed text is read the same way, when the field is left or
  // Enter is pressed. Text that is no number stays as typed and is marked,
  // and the value stays; only a number that differs is a change.
  import type { JsonValue } from "../generated/serde_json/JsonValue";

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
    const next = read(typed);
    if (next === null) {
      bad = typed;
      return;
    }
    bad = null;
    if (next !== (value ?? null)) onchange(next);
  }

  function key(e: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    if (e.key !== "Enter") return;
    take(e.currentTarget.value);
    if (onsubmit && onsubmit() !== false) e.preventDefault();
  }
</script>

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

<style>
  .bad {
    border-color: var(--error);
  }
</style>
