<script lang="ts">
  // The fields of an entity on a diagram, as the designers' properties
  // show them: name, type, required, unique and the other modifiers as
  // Clarity writes them. Text is taken when a box is left or Enter is
  // pressed.

  import type { ErdField } from "./generated/ErdField";

  let {
    fields,
    types,
    onchange,
  }: {
    fields: ErdField[];
    /** The types a field may have: Clarity's, the project's enums, refs and lists of the entities. */
    types: string[];
    onchange: (fields: ErdField[]) => void;
  } = $props();

  /** Each list of offered types has an id of its own, so two lists on one page do not mix. */
  const listId = $props.id();

  function setField(index: number, change: Partial<ErdField>) {
    onchange(fields.map((f, i) => (i === index ? { ...f, ...change } : f)));
  }

  function addField() {
    let n = 1;
    while (fields.some((f) => f.name === `field${n}`)) n++;
    onchange([...fields, { name: `field${n}`, typeText: "text", required: false, unique: false, extra: "" }]);
  }

  function enter(e: KeyboardEvent) {
    if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur();
  }
</script>

<h4>Fields <span class="count">{fields.length}</span></h4>
<datalist id={listId}>
  {#each types as t (t)}<option value={t}></option>{/each}
</datalist>
{#each fields as f, i (i)}
  <div class="field">
    <input class="fname" value={f.name} spellcheck="false" title="Name, in snake_case" onchange={(e) => setField(i, { name: e.currentTarget.value.trim() })} onkeydown={enter} />
    <input class="ftype" value={f.typeText} list={listId} spellcheck="false" title="Type: text(80), money, an enum, ref Customer, list<OrderLine>; in a web API, a record held inside: User" onchange={(e) => setField(i, { typeText: e.currentTarget.value.trim() })} onkeydown={enter} />
    <button class="remove" onclick={() => onchange(fields.filter((_, j) => j !== i))} title="Take this field away" aria-label="Take {f.name} away">x</button>
    <label class="flag"><input type="checkbox" checked={f.required} onchange={(e) => setField(i, { required: e.currentTarget.checked })} /> required</label>
    <label class="flag"><input type="checkbox" checked={f.unique} onchange={(e) => setField(i, { unique: e.currentTarget.checked })} /> unique</label>
    <input class="extra" value={f.extra} placeholder="min 1, default 0, owned..." spellcheck="false" title="Other modifiers, as Clarity writes them" onchange={(e) => setField(i, { extra: e.currentTarget.value.trim() })} onkeydown={enter} />
  </div>
{/each}
<button onclick={addField}>+ Field</button>

<style>
  h4 {
    margin: 4px 0 0;
    font-size: 13px;
  }

  .count {
    color: var(--text-muted);
    font-weight: normal;
  }

  .field {
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: 4px;
    padding: 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--panel-alt);
  }

  .field input {
    min-width: 0;
    font-family: var(--mono);
    font-size: 12px;
  }

  .field .extra {
    grid-column: 1 / -1;
  }

  .flag {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-muted);
  }

  .remove {
    padding: 0 7px;
  }
</style>
