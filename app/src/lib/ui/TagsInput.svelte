<script lang="ts">
  // Clarity's tags input on the web, the same as on Avalonia
  // (conformance/list-fields.json): a list field's items as chips, each with
  // a button that removes it, and a place to add one: a box to type text or
  // a number in (taken on Enter or when the box is left), a date to pick,
  // or a dropdown of the values of an enum the list has not. Text that is
  // no value of the list's kind stays as typed and is marked.
  import { added, addTyped, sameItem, type Item, type ItemKind } from "../listFields";

  let {
    items,
    kind,
    choices = [],
    show = String,
    words,
    id = undefined,
    describedBy = undefined,
    disabled = false,
    readonly = false,
    onchange,
  }: {
    items: Item[] | null | undefined;
    kind: ItemKind;
    /** An enum's values with their labels. */
    choices?: { value: Item; label: string }[];
    /** An item as people read it. */
    show?: (item: Item) => string;
    /** Clarity's own words: the place a new item goes, the button that removes one. */
    words: { add: string; remove: string };
    id?: string;
    describedBy?: string;
    disabled?: boolean;
    readonly?: boolean;
    /** The person added or removed an item: Clarity's `change`. */
    onchange: (next: Item[]) => void;
  } = $props();

  const list = $derived(items ?? []);
  const offered = $derived(choices.filter((c) => !list.some((i) => sameItem(kind, i, c.value))));
  let typed = $state("");
  let bad = $state(false);

  function offer(next: Item[]) {
    if (next !== list) onchange(next);
  }

  function take() {
    const got = addTyped(kind, list, typed);
    if ("refused" in got) {
      bad = true;
      return;
    }
    bad = false;
    typed = "";
    offer(got.items);
  }

  function remove(at: number) {
    offer(list.filter((_, i) => i !== at));
  }

  function chipLabel(item: Item): string {
    return choices.find((c) => sameItem(kind, c.value, item))?.label ?? show(item);
  }
</script>

<span class="tags-input" {id} aria-describedby={describedBy}>
  {#each list as item, at (at)}
    <span class="chip">
      <span>{chipLabel(item)}</span>
      {#if !readonly}
        <button type="button" class="remove" title={words.remove} aria-label="{words.remove} {chipLabel(item)}" {disabled} onclick={() => remove(at)}>
          <svg viewBox="0 0 8 8" width="8" height="8" aria-hidden="true"><path d="M0 0 L8 8 M8 0 L0 8" /></svg>
        </button>
      {/if}
    </span>
  {/each}
  {#if !readonly}
    {#if kind === "enum"}
      <select
        class="add"
        aria-label={words.add}
        {disabled}
        value=""
        onchange={(e) => {
          const picked = offered[Number(e.currentTarget.value)];
          e.currentTarget.value = "";
          if (picked) offer(added(kind, list, picked.value));
        }}
      >
        <option value="" disabled selected>{words.add}</option>
        {#each offered as choice, i (choice.value)}
          <option value={String(i)}>{choice.label}</option>
        {/each}
      </select>
    {:else if kind === "date"}
      <input
        class="add"
        type="date"
        aria-label={words.add}
        {disabled}
        value=""
        onchange={(e) => {
          const day = e.currentTarget.value;
          e.currentTarget.value = "";
          if (day) offer(added(kind, list, day));
        }}
      />
    {:else}
      <input
        class="add"
        class:invalid={bad}
        placeholder={words.add}
        aria-label={words.add}
        inputmode={kind === "text" ? "text" : "decimal"}
        {disabled}
        bind:value={typed}
        oninput={() => (bad = false)}
        onblur={take}
        onkeydown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            take();
          }
        }}
      />
    {/if}
  {/if}
</span>

<style>
  .tags-input {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    width: 100%;
    min-height: 100%;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 1px 2px 1px 8px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel);
    white-space: nowrap;
  }

  .remove {
    display: inline-flex;
    padding: 3px;
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .remove path {
    stroke: var(--text-muted);
    stroke-width: 1.5;
  }

  .add {
    flex: 1;
    min-width: 80px;
  }

  .add.invalid {
    border-color: var(--error);
  }
</style>
