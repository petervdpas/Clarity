<script lang="ts" module>
  /** An offered item: what picking it gives, and the texts of its row. */
  export type Offered = { key: string; cells: string[] };
</script>

<script lang="ts">
  // A box to type in with a list of what fits below it, the same on every
  // renderer (conformance/dropdown-options.json): the dropdown's `typed`
  // (any text, the field's values offered) and `search` (a record picked
  // among many, filtered as it is typed). Arrow keys move through the list,
  // Enter or a click picks, Escape closes it and puts the text back. Typed
  // text that is not picked counts only for `typed`; for `search`, empty
  // text is no record.

  let {
    text,
    offer,
    free = false,
    id = undefined,
    label = undefined,
    tabindex = undefined,
    describedBy = undefined,
    invalid = false,
    disabled = false,
    readonly = false,
    onpick,
    oncommit = undefined,
    onclear = undefined,
  }: {
    /** What the box shows while it is not being typed in. */
    text: string;
    /** What fits the typed text. */
    offer: (typed: string) => Offered[];
    /** Any typed text is a value (`typed`); else only a picked item is. */
    free?: boolean;
    id?: string;
    label?: string;
    tabindex?: number;
    describedBy?: string;
    invalid?: boolean;
    disabled?: boolean;
    readonly?: boolean;
    /** An item was picked. */
    onpick: (key: string) => void;
    /** Typed text was taken (`free` only): Enter, or leaving the box. */
    oncommit?: (typed: string) => void;
    /** The text was emptied and the box left (not `free`): no value. */
    onclear?: () => void;
  } = $props();

  let box = $state<HTMLInputElement | null>(null);
  let draft = $state<string | null>(null);
  let at = $state(-1);
  let place = $state<{ left: number; top: number; width: number } | null>(null);
  const items = $derived(place ? offer(draft ?? "") : []);
  const listId = `offered-${Math.random().toString(36).slice(2)}`;

  function open() {
    if (!box || readonly || disabled) return;
    const r = box.getBoundingClientRect();
    place = { left: r.left, top: r.bottom + 2, width: r.width };
  }

  function close() {
    place = null;
    at = -1;
  }

  function pick(item: Offered) {
    close();
    draft = null;
    onpick(item.key);
  }

  function leave() {
    if (draft === null) return close();
    const typed = draft;
    draft = null;
    close();
    if (free) {
      if (typed !== text) oncommit?.(typed);
    } else if (typed.trim() === "" && text !== "") {
      onclear?.();
    }
  }

  function key(e: KeyboardEvent) {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!place) open();
      const n = items.length;
      if (n) at = e.key === "ArrowDown" ? (at + 1) % n : (at - 1 + n) % n;
    } else if (e.key === "Enter") {
      e.preventDefault();
      const chosen = items[at] ?? (!free && items.length === 1 ? items[0] : undefined);
      if (chosen) pick(chosen);
      else leave();
    } else if (e.key === "Escape" && place) {
      e.stopPropagation();
      draft = null;
      close();
    }
  }
</script>

<input
  bind:this={box}
  {id}
  class="type-ahead"
  class:invalid
  aria-label={label}
  aria-describedby={describedBy}
  aria-autocomplete="list"
  aria-controls={listId}
  aria-expanded={place !== null}
  role="combobox"
  autocomplete="off"
  {tabindex}
  {disabled}
  {readonly}
  value={draft ?? text}
  onfocus={open}
  oninput={(e) => {
    draft = e.currentTarget.value;
    at = -1;
    if (!place) open();
  }}
  onkeydown={key}
  onblur={leave}
/>
{#if place && items.length}
  <ul id={listId} class="offered clarity-overlay" role="listbox" aria-label={label} style:left="{place.left}px" style:top="{place.top}px" style:min-width="{place.width}px">
    {#each items as item, i (item.key)}
      <!-- The keys move through the list from the box, which keeps the focus. -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <li role="option" aria-selected={i === at} class:at={i === at} onpointerdown={(e) => e.preventDefault()} onclick={() => pick(item)}>
        {#each item.cells as cell, c (c)}<span>{cell}</span>{/each}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .type-ahead {
    width: 100%;
  }

  .type-ahead.invalid {
    border-color: var(--error);
  }

  .offered {
    position: fixed;
    z-index: 50;
    max-height: 240px;
    overflow-y: auto;
    margin: 0;
    padding: 2px 0;
    list-style: none;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 4px 16px rgb(0 0 0 / 25%);
  }

  li {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(0, 1fr);
    gap: 12px;
    padding: 3px 8px;
    cursor: pointer;
    white-space: nowrap;
  }

  li span + span {
    color: var(--text-muted);
  }

  li.at,
  li:hover {
    background: var(--accent);
    color: var(--accent-text, white);
  }

  li.at span + span,
  li:hover span + span {
    color: inherit;
  }
</style>
