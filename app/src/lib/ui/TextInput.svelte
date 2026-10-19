<script lang="ts">
  // Clarity's text input on the web: a line of text, an email, a password or
  // a memo, the same as on Avalonia (conformance/forms.json). Typed text is
  // taken when the field is left or Enter is pressed (a memo: only when it is
  // left, Enter is a new line); only text that differs from the value is a
  // change. What the code sets is never a change.

  let {
    value,
    kind = "text",
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
    value: unknown;
    kind?: "text" | "email" | "password" | "memo";
    id?: string;
    /** The name read out when no label part names the input. */
    label?: string;
    placeholder?: string;
    tabindex?: number;
    describedBy?: string;
    invalid?: boolean;
    disabled?: boolean;
    readonly?: boolean;
    /** The person changed the text: Clarity's `change`. */
    onchange: (next: string) => void;
    /** Enter was pressed and the text taken: the form's `submit`; false when no form took it. Never from a memo. */
    onsubmit?: () => boolean | void;
    onfocus?: () => void;
    onblur?: () => void;
  } = $props();

  const shown = $derived(value === null || value === undefined ? "" : String(value));

  function take(typed: string) {
    if (typed !== shown) onchange(typed);
  }

  function key(e: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    if (e.key !== "Enter") return;
    take(e.currentTarget.value);
    if (onsubmit && onsubmit() !== false) e.preventDefault();
  }
</script>

{#if kind === "memo"}
  <textarea
    {id}
    aria-label={label}
    value={shown}
    {placeholder}
    {tabindex}
    aria-describedby={describedBy}
    aria-invalid={invalid}
    {disabled}
    {readonly}
    onchange={(e) => take(e.currentTarget.value)}
    {onfocus}
    {onblur}
  ></textarea>
{:else}
  <input
    {id}
    type={kind}
    aria-label={label}
    value={shown}
    {placeholder}
    {tabindex}
    aria-describedby={describedBy}
    aria-invalid={invalid}
    {disabled}
    {readonly}
    onchange={(e) => take(e.currentTarget.value)}
    onkeydown={key}
    {onfocus}
    {onblur}
  />
{/if}

<style>
  textarea {
    width: 100%;
    height: 100%;
    resize: none;
    font: inherit;
  }
</style>
