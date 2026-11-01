<script lang="ts" module>
  import type { JsonValue } from "../generated/serde_json/JsonValue";

  /** A choice of a dropdown or radio group: the value stored and the label shown. */
  export type Choice = { value: JsonValue; label: string };
</script>

<script lang="ts">
  // Clarity's dropdown on the web, the same as on Avalonia
  // (conformance/forms.json, tables.json): it shows the choice of its value
  // (an enum has no empty choice, a reference has one first), and only the
  // person picking another choice is a change.

  let {
    value,
    choices,
    id = undefined,
    label = undefined,
    tabindex = undefined,
    describedBy = undefined,
    invalid = false,
    disabled = false,
    onchange,
  }: {
    value: JsonValue | undefined;
    choices: Choice[];
    id?: string;
    label?: string;
    tabindex?: number;
    describedBy?: string;
    invalid?: boolean;
    /** Disabled or read-only. */
    disabled?: boolean;
    /** The person picked another choice: Clarity's `change`. */
    onchange: (next: JsonValue) => void;
  } = $props();

  const index = $derived(choices.findIndex((c) => c.value === (value ?? null)));

  function pick(at: number) {
    const next = choices[at]?.value ?? null;
    if (next !== (value ?? null)) onchange(next);
  }
</script>

<select
  {id}
  aria-label={label}
  value={String(index)}
  {tabindex}
  aria-describedby={describedBy}
  aria-invalid={invalid}
  {disabled}
  onchange={(e) => pick(Number(e.currentTarget.value))}
>
  {#each choices as choice, c (c)}
    <option value={String(c)}>{choice.label}</option>
  {/each}
</select>
