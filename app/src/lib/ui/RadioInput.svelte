<script lang="ts">
  // Clarity's radio group on the web, the same as on Avalonia
  // (conformance/forms.json): one button per choice of an enum, and only the
  // person picking another one is a change.
  import type { JsonValue } from "../generated/serde_json/JsonValue";
  import type { Choice } from "./ChoiceInput.svelte";

  let {
    value,
    choices,
    name,
    vertical = false,
    id = undefined,
    tabindex = undefined,
    describedBy = undefined,
    invalid = false,
    disabled = false,
    onchange,
  }: {
    value: JsonValue | undefined;
    choices: Choice[];
    /** The group's name in the page: its buttons are one group. */
    name: string;
    vertical?: boolean;
    /** The id of the first button, which a label names. */
    id?: string;
    tabindex?: number;
    describedBy?: string;
    invalid?: boolean;
    /** Disabled or read-only. */
    disabled?: boolean;
    /** The person picked another choice: Clarity's `change`. */
    onchange: (next: JsonValue) => void;
  } = $props();
</script>

<div class="choices" class:vertical role="radiogroup" aria-describedby={describedBy} aria-invalid={invalid}>
  {#each choices as choice, i (i)}
    <label>
      <input
        id={i === 0 ? id : undefined}
        type="radio"
        {name}
        value={String(i)}
        checked={choice.value === (value ?? null)}
        {tabindex}
        {disabled}
        onchange={() => {
          if (choice.value !== (value ?? null)) onchange(choice.value);
        }}
      />
      {choice.label}
    </label>
  {/each}
</div>

<style>
  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
  }

  .choices.vertical {
    flex-direction: column;
    flex-wrap: nowrap;
  }

  label {
    display: flex;
    gap: 6px;
    align-items: center;
  }
</style>
