<script lang="ts">
  // Clarity's date input on the web, the same as on Avalonia
  // (conformance/forms.json, tables.json): a day picked from the calendar or
  // typed, and only another day picked by the person is a change. Its value
  // is the day as the backends send it (2026-10-20), or none.

  let {
    value,
    id = undefined,
    label = undefined,
    tabindex = undefined,
    describedBy = undefined,
    invalid = false,
    disabled = false,
    readonly = false,
    onchange,
    onsubmit = undefined,
  }: {
    value: unknown;
    id?: string;
    label?: string;
    tabindex?: number;
    describedBy?: string;
    invalid?: boolean;
    disabled?: boolean;
    readonly?: boolean;
    /** The person picked another day: Clarity's `change`. */
    onchange: (next: string | null) => void;
    /** Enter was pressed: the form's `submit`; false when no form took it. */
    onsubmit?: () => boolean | void;
  } = $props();

  const shown = $derived(typeof value === "string" ? value : "");

  function pick(day: string) {
    const next = day === "" ? null : day;
    if (next !== (value ?? null)) onchange(next);
  }
</script>

<input
  {id}
  type="date"
  aria-label={label}
  value={shown}
  {tabindex}
  aria-describedby={describedBy}
  aria-invalid={invalid}
  {disabled}
  {readonly}
  onchange={(e) => pick(e.currentTarget.value)}
  onkeydown={(e) => {
    if (e.key === "Enter" && onsubmit && onsubmit() !== false) e.preventDefault();
  }}
/>
