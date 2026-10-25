<script lang="ts">
  // Clarity's keys window in a running application (DESIGN 5.28), where the
  // project file says `keys editable`: every command per place, Everywhere
  // (the app's frame) first, then each window's own, with its keys. Change,
  // then press the new key; a key another command has in the same place
  // offers a swap; Reset puts the design's key back. In Clarity's words.

  import Dialog from "./ui/Dialog.svelte";
  import { alsoHas, assign, eventKey, keysOf, refused, reset, type Command, type KeyChoices } from "./keys";
  import { appCommands } from "./appKeys";
  import { ownWord, shownText, tr } from "./model";
  import type { Preview } from "./generated/Preview";
  import type { Slot } from "./format";

  let {
    preview,
    choices,
    onchange,
    onclose,
  }: {
    preview: Preview;
    choices: KeyChoices;
    onchange: (next: KeyChoices) => void;
    onclose: () => void;
  } = $props();

  /** Why a key cannot be a command's, as Clarity's words say it. */
  const REFUSED = { typing: "keysTyping", tab: "keysTab", enter: "keysEnter", escape: "keysEscape" } as const;

  const text = (value: string): Slot => ({ kind: "text", value }) as Slot;
  const word = (key: string, values: Record<string, Slot> = {}) => shownText(preview, { key, values });

  const commands = $derived(appCommands(preview.keys, (caption, name) => (caption ? tr(preview, caption) : name)));
  const nameOf = (id: string) => commands.find((c) => c.id === id)?.name ?? id;
  const placeName = (title: string | null) => (title ? tr(preview, title) : word("keysEverywhere"));

  let capturing = $state<string | null>(null);
  let note = $state<string | null>(null);
  let swap = $state<{ id: string; key: string; other: string } | null>(null);

  function take(id: string, key: string) {
    onchange(assign(commands, choices, id, key).choices);
    const also = alsoHas(commands, choices, id, key);
    note = also.length ? word("keysAlso", { key: text(key), command: text(also.map(nameOf).join(", ")), mine: text(nameOf(id)) }) : null;
  }

  // Before anything else hears it: the window's keys and the dialog's Escape.
  function onkeydown(e: KeyboardEvent) {
    if (!capturing) return;
    e.preventDefault();
    e.stopImmediatePropagation();
    if (e.key === "Escape" && !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey) {
      capturing = null;
      return;
    }
    const key = eventKey(e, false);
    if (!key) return;
    const why = refused(key);
    if (why) {
      note = word(REFUSED[why], { key: text(key) });
      return;
    }
    const id = capturing;
    capturing = null;
    const other = assign(commands, choices, id, key).swapped;
    if (other) swap = { id, key, other };
    else take(id, key);
  }

  const shown = (c: Command) => keysOf(c, choices);
</script>

<svelte:window onkeydowncapture={onkeydown} />

<Dialog label={word("keysTitle")} {onclose}>
  <div class="keys">
    <h2>{word("keysTitle")}</h2>
    {#if swap}
      {@const s = swap}
      <div class="swap" role="alert">
        <span>{word("keysTaken", { key: text(s.key), command: text(nameOf(s.other)) })}</span>
        <button onclick={() => (take(s.id, s.key), (swap = null))}>{word("keysSwap")}</button>
        <button onclick={() => (swap = null)}>{ownWord(preview, "Cancel")}</button>
      </div>
    {/if}
    {#if note}<p class="note" role="status">{note}</p>{/if}
    {#each preview.keys as layer (layer.id)}
      <fieldset>
        <legend>{placeName(layer.title)}</legend>
        {#each commands.filter((c) => c.layers[0] === layer.id) as c (c.id)}
          <div class="row">
            <span class="name">{c.name}</span>
            <span class="key">
              {#if capturing === c.id}
                <em>{word("keysPress")}</em>
              {:else if shown(c).length}
                {#each shown(c) as k, i (k)}{#if i > 0}, {/if}<kbd>{k}</kbd>{/each}
              {:else}
                <span class="muted">{word("keysNone")}</span>
              {/if}
            </span>
            <button onclick={() => ((capturing = c.id), (swap = null), (note = null))}>{word("keysChange")}</button>
            <button disabled={!(c.id in choices)} onclick={() => (onchange(reset(commands, choices, c.id)), (note = null))}>{word("keysReset")}</button>
          </div>
        {/each}
      </fieldset>
    {/each}
    <div class="actions">
      <button disabled={Object.keys(choices).length === 0} onclick={() => (onchange({}), (note = null), (swap = null))}>{word("keysResetAll")}</button>
      <button class="primary" onclick={onclose}>{word("keysClose")}</button>
    </div>
  </div>
</Dialog>

<style>
  .keys {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  h2 {
    margin: 0;
    font-size: 1.1em;
  }
  fieldset {
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius-panel);
    display: grid;
    gap: 4px;
  }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto auto;
    gap: 6px;
    align-items: center;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .swap,
  .actions {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
  }
  .actions {
    justify-content: flex-end;
  }
  .note {
    margin: 0;
  }
</style>
