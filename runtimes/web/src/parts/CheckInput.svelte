<script lang="ts">
  // Clarity's checkbox and switch on the web, the same as on Avalonia
  // (conformance/forms.json, tables.json): only the person ticking it is a
  // change; what the code sets is not. A read-only one cannot be ticked.

  let {
    checked,
    switch: isSwitch = false,
    caption = undefined,
    id = undefined,
    label = undefined,
    tabindex = undefined,
    describedBy = undefined,
    invalid = false,
    disabled = false,
    onchange,
  }: {
    checked: boolean;
    /** Shown as a switch (`switch` part, `as switch` column). */
    switch?: boolean;
    /** The text beside it. */
    caption?: string;
    id?: string;
    label?: string;
    tabindex?: number;
    describedBy?: string;
    invalid?: boolean;
    /** Disabled or read-only. */
    disabled?: boolean;
    /** The person ticked it: Clarity's `change`. */
    onchange: (next: boolean) => void;
  } = $props();
</script>

<label class={isSwitch ? "toggle" : "check"}>
  <input
    {id}
    type="checkbox"
    role={isSwitch ? "switch" : undefined}
    aria-label={label}
    {checked}
    {tabindex}
    aria-describedby={describedBy}
    aria-invalid={invalid}
    {disabled}
    onchange={(e) => {
      if (e.currentTarget.checked !== checked) onchange(e.currentTarget.checked);
    }}
  />
  {#if isSwitch}<span class="track" aria-hidden="true"></span>{/if}
  {#if caption}{caption}{/if}
</label>

<style>
  .check,
  .toggle {
    display: inline-flex;
    gap: 8px;
    align-items: center;
  }

  .toggle {
    position: relative;
  }

  .toggle input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }

  .track {
    flex: none;
    position: relative;
    width: 32px;
    height: 18px;
    border-radius: 9px;
    background: var(--border-strong);
    transition: background 0.15s;
  }

  .track::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--panel);
    transition: transform 0.15s;
  }

  .toggle input:checked + .track {
    background: var(--accent);
  }

  .toggle input:checked + .track::after {
    transform: translateX(14px);
  }

  .toggle input:focus-visible + .track {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
