<script lang="ts">
  // Clarity's keys, set by the person (DESIGN 5.28): every command per layer
  // with its keys. Change, then press the new key; a key another command has
  // in the same layer offers a swap; Reset puts the design's key back.

  import { IDE_COMMANDS, LAYERS, alsoHas, assign, eventKey, keysOf, refused, shownKey, type Command } from "./keys";
  import { ideKeys, mac, resetIdeKeys, setIdeKey } from "./keys.svelte";

  /** Why a key cannot be a command's, by what `refused` says. */
  const REFUSED: Record<NonNullable<ReturnType<typeof refused>>, string> = {
    tab: "Tab moves between fields.",
    enter: "Enter confirms in dialogs and forms.",
    escape: "Escape cancels and stops.",
    typing: "A key without Ctrl or Alt is typing.",
  };

  const nameOf = (id: string) => IDE_COMMANDS.find((c) => c.id === id)?.name ?? id;
  const layerName = (command: Command) => LAYERS.find((l) => l.layer === command.layers[0])?.name ?? "";

  /** The command waiting for its new key, and what was said about the last try. */
  let capturing = $state<string | null>(null);
  let note = $state<string | null>(null);
  /** A key another command has in the same layer, waiting for the swap. */
  let swap = $state<{ id: string; key: string; other: string } | null>(null);

  function start(id: string) {
    capturing = id;
    swap = null;
    note = null;
  }

  function take(id: string, key: string) {
    setIdeKey(id, key);
    const also = alsoHas(IDE_COMMANDS, ideKeys.choices, id, key);
    const command = IDE_COMMANDS.find((c) => c.id === id)!;
    note = also.length
      ? `${shownKey(key, mac)} is ${also.map(nameOf).join(" and ")}'s too, elsewhere; in ${layerName(command)}, ${command.name} goes first.`
      : null;
  }

  // Before anything else hears the key: Clarity's own keys and the settings' Escape.
  function onkeydown(e: KeyboardEvent) {
    if (!capturing) return;
    e.preventDefault();
    e.stopImmediatePropagation();
    if (e.key === "Escape" && !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey) {
      capturing = null;
      return;
    }
    const key = eventKey(e, mac);
    if (!key) return;
    const why = refused(key);
    if (why) {
      note = `${shownKey(key, mac)} cannot be used: ${REFUSED[why]}`;
      return;
    }
    const id = capturing;
    capturing = null;
    const other = assign(IDE_COMMANDS, ideKeys.choices, id, key).swapped;
    if (other) swap = { id, key, other };
    else take(id, key);
  }

  const keysShown = (command: Command) => keysOf(command, ideKeys.choices).map((k) => shownKey(k, mac));
</script>

<svelte:window onkeydowncapture={onkeydown} />

<div class="keys">
  <p class="muted">Pick Change, then press the new key. A key another command has in the same place offers to swap them. On macOS, Cmd works where a key says Ctrl.</p>
  <p><button onclick={() => (resetIdeKeys(), (note = null), (swap = null))} disabled={Object.keys(ideKeys.choices).length === 0}>Reset all</button></p>

  {#if swap}
    {@const s = swap}
    <div class="swap" role="alert">
      <span>{shownKey(s.key, mac)} is {nameOf(s.other)}'s key. Swap, so {nameOf(s.other)} gets {nameOf(s.id)}'s key?</span>
      <button class="primary" onclick={() => (take(s.id, s.key), (swap = null))}>Swap</button>
      <button onclick={() => (swap = null)}>Cancel</button>
    </div>
  {/if}
  {#if note}<p class="note" role="status">{note}</p>{/if}

  {#each LAYERS as l (l.layer)}
    {@const commands = IDE_COMMANDS.filter((c) => c.layers[0] === l.layer)}
    {#if commands.length}
      <fieldset>
        <legend>{l.name}</legend>
        <table>
          <tbody>
            {#each commands as c (c.id)}
              <tr>
                <th scope="row">{c.name}</th>
                <td>
                  {#if capturing === c.id}
                    <span class="waiting">Press the new key; Escape stops</span>
                  {:else if keysShown(c).length}
                    {#each keysShown(c) as k, i (k)}{#if i > 0}<span class="muted"> or </span>{/if}<kbd>{k}</kbd>{/each}
                  {:else}
                    <span class="muted">None</span>
                  {/if}
                </td>
                <td class="actions">
                  <button onclick={() => start(c.id)} aria-label="Change the key of {c.name}">Change</button>
                  <button onclick={() => (resetIdeKeys(c.id), (note = null))} disabled={!(c.id in ideKeys.choices)} aria-label="Put back the key of {c.name}">Reset</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </fieldset>
    {/if}
  {/each}
</div>

<style>
  .keys {
    display: grid;
    gap: 12px;
    max-width: 720px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th {
    text-align: left;
    font-weight: normal;
    width: 40%;
  }
  th,
  td {
    padding: 4px 6px;
  }
  .actions {
    text-align: right;
    white-space: nowrap;
  }
  .waiting {
    font-style: italic;
  }
  .swap {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  p {
    margin: 0;
  }
</style>
