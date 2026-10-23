<script lang="ts">
  // Clarity's color input on the web, the same as on Avalonia
  // (conformance/colors.json): a swatch that opens the palette every
  // renderer shares, and the color as "#rrggbb" to type. Typed text is read
  // when the input is left or Enter is pressed; text that is no color stays
  // as typed and is marked, and the value stays. Only a color that differs
  // is a change.
  import { PALETTE, readColor } from "../colors";

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
  }: {
    value: string | null | undefined;
    id?: string;
    label?: string;
    tabindex?: number;
    describedBy?: string;
    invalid?: boolean;
    disabled?: boolean;
    readonly?: boolean;
    /** The person typed or picked another color: Clarity's `change`. */
    onchange: (next: string) => void;
  } = $props();

  const shown = $derived(value ?? "");
  /** What was typed while it is no color. */
  let bad = $state<string | null>(null);
  $effect(() => {
    void value;
    bad = null;
  });

  let swatch = $state<HTMLButtonElement | null>(null);
  let open = $state<{ left: number; top: number } | null>(null);

  function take(next: string | null, typed: string) {
    if (next === null) {
      bad = typed;
      return;
    }
    bad = null;
    if (next !== shown) onchange(next);
  }

  function toggle() {
    if (open || !swatch) {
      open = null;
      return;
    }
    const at = swatch.getBoundingClientRect();
    open = { left: Math.max(4, Math.min(at.left, window.innerWidth - 260)), top: at.bottom + 4 };
  }

  function pick(color: string) {
    open = null;
    take(color, color);
    swatch?.focus();
  }

  function outside(e: PointerEvent) {
    if (open && !(e.target as Element).closest?.(".palette, .swatch")) open = null;
  }

  function key(e: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    if (e.key === "Enter") take(readColor(e.currentTarget.value), e.currentTarget.value);
  }
</script>

<svelte:window onpointerdown={outside} onkeydown={(e) => e.key === "Escape" && open && (open = null)} />

<span class="color-input">
  <button
    bind:this={swatch}
    type="button"
    class="swatch"
    class:none={shown === ""}
    style:background-color={shown || "transparent"}
    aria-label={label}
    aria-haspopup="true"
    aria-expanded={open !== null}
    disabled={disabled || readonly}
    onclick={toggle}
  ></button>
  <input
    {id}
    class:bad={bad !== null}
    value={bad ?? shown}
    maxlength="7"
    spellcheck="false"
    {tabindex}
    aria-describedby={describedBy}
    aria-invalid={invalid || bad !== null}
    {disabled}
    {readonly}
    onchange={(e) => take(readColor(e.currentTarget.value), e.currentTarget.value)}
    onkeydown={key}
  />
</span>

{#if open}
  <div class="palette" role="listbox" aria-label={label} style:left="{open.left}px" style:top="{open.top}px">
    {#each PALETTE as row, r (r)}
      {#each row as color (color)}
        <button
          type="button"
          role="option"
          aria-selected={color === shown}
          class:chosen={color === shown}
          title={color}
          style:background-color={color}
          onclick={() => pick(color)}
        ></button>
      {/each}
    {/each}
  </div>
{/if}

<style>
  .color-input {
    display: flex;
    gap: 4px;
    width: 100%;
    height: 100%;
  }
  .swatch {
    flex: none;
    aspect-ratio: 1;
    height: 100%;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  /* No color: a diagonal line through the swatch. */
  .swatch.none {
    background-image: linear-gradient(to top right, transparent calc(50% - 1px), var(--text-muted) 50%, transparent calc(50% + 1px));
  }
  input {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
  }
  .bad {
    border-color: var(--error);
  }
  .palette {
    position: fixed;
    z-index: 50;
    display: grid;
    grid-template-columns: repeat(10, 22px);
    gap: 3px;
    padding: 6px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.2);
  }
  .palette button {
    width: 22px;
    height: 22px;
    padding: 0;
    border: 1px solid rgb(0 0 0 / 0.15);
    border-radius: 3px;
  }
  .palette .chosen {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
</style>
