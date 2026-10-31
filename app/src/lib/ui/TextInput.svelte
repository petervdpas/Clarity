<script lang="ts">
  import SecretButtons from "./SecretButtons.svelte";

  // Clarity's text input on the web: a line of text, an email, a password or
  // a memo, the same as on Avalonia (conformance/forms.json). Typed text is
  // taken when the field is left or Enter is pressed (a memo: only when it is
  // left, Enter is a new line); only text that differs from the value is a
  // change. What the code sets is never a change. A password has an eye that
  // shows what is typed or hides it again, and a button that copies it.

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
    words = { show: "Show", hide: "Hide", copy: "Copy" },
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
    /** A password's buttons, named in the person's language. */
    words?: { show: string; hide: string; copy: string };
  } = $props();

  let revealed = $state(false);
  let input = $state<HTMLInputElement | null>(null);

  const shown = $derived(value === null || value === undefined ? "" : String(value));

  function take(typed: string) {
    if (typed !== shown) onchange(typed);
  }

  /** The text Enter took last: leaving the input with it says nothing new. */
  let takenOnEnter: string | null = null;

  /**
   * The input was left (or the browser said its text changed): what was
   * typed is taken, unless Enter took that text already. A dialog that closes
   * on Enter removes the input, and the browser says its text once more.
   */
  function left(typed: string) {
    if (typed === takenOnEnter) {
      takenOnEnter = null;
      return;
    }
    takenOnEnter = null;
    take(typed);
  }

  function key(e: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    // Typing again after Enter: the next leave takes what it brings.
    if (e.key !== "Enter") takenOnEnter = null;
    if (e.key !== "Enter") return;
    take(e.currentTarget.value);
    takenOnEnter = e.currentTarget.value;
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
    onchange={(e) => left(e.currentTarget.value)}
    {onfocus}
    {onblur}
  ></textarea>
{:else if kind === "password"}
  <span class="secret">
    <input
      bind:this={input}
      {id}
      type={revealed ? "text" : "password"}
      aria-label={label}
      value={shown}
      {placeholder}
      {tabindex}
      aria-describedby={describedBy}
      aria-invalid={invalid}
      {disabled}
      {readonly}
      autocomplete="off"
      spellcheck="false"
      onchange={(e) => left(e.currentTarget.value)}
      onkeydown={key}
      {onfocus}
      {onblur}
    />
    <SecretButtons bind:revealed text={() => input?.value ?? ""} {words} {disabled} />
  </span>
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
    onchange={(e) => left(e.currentTarget.value)}
    onkeydown={key}
    {onfocus}
    {onblur}
  />
{/if}

<style>
  .secret {
    display: flex;
    align-items: stretch;
    width: 100%;
    gap: 2px;
  }
  .secret input {
    flex: 1;
    min-width: 0;
  }
  textarea {
    width: 100%;
    height: 100%;
    resize: none;
    font: inherit;
  }
</style>
