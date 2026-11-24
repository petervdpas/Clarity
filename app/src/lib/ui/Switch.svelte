<script lang="ts">
  // A toggle switch: a track with a knob that slides to on or off, one
  // click (or Space) flips it. A checkbox with the role switch underneath,
  // so keyboards and screen readers treat it as one.
  let {
    checked,
    label,
    title = "",
    onchange,
  }: {
    checked: boolean;
    label: string;
    title?: string;
    onchange: (on: boolean) => void;
  } = $props();
</script>

<label class="switch" {title}>
  <input type="checkbox" role="switch" {checked} onchange={(e) => onchange(e.currentTarget.checked)} />
  <span class="track" aria-hidden="true"><span class="knob"></span></span>
  <span class="label">{label}</span>
</label>

<style>
  .switch {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    user-select: none;
  }
  input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
    pointer-events: none;
  }
  .track {
    position: relative;
    flex-shrink: 0;
    width: 26px;
    height: 14px;
    border-radius: 7px;
    background: var(--error);
    transition: background 0.15s;
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--panel);
    transition: left 0.15s;
  }
  input:checked + .track {
    background: var(--success);
  }
  input:checked + .track .knob {
    left: 14px;
  }
  input:focus-visible + .track {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
</style>
