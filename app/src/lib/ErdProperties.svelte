<script lang="ts">
  // The properties of what is picked on the ERD canvas: an entity's name
  // and fields, or a relation's ends, how many stand at each, whether the
  // one end owns the many, and its label. Every change is written at once;
  // text is taken when the field is left or Enter is pressed.

  import type { Cardinality } from "./generated/Cardinality";
  import type { Erd } from "./generated/Erd";
  import type { ErdField } from "./generated/ErdField";
  import type { CanvasPick } from "./designCanvas";
  import { removeEntity, renameEntity } from "./erdDesign";
  import FieldList from "./FieldList.svelte";

  let {
    erd,
    picked = $bindable(null),
    types,
    onchange,
  }: {
    erd: Erd;
    picked?: CanvasPick;
    /** The types a field may have: Clarity's, the project's enums, refs and lists of the entities. */
    types: string[];
    onchange: (erd: Erd) => void;
  } = $props();

  const COUNTS: { value: Cardinality; name: string }[] = [
    { value: "exactlyOne", name: "exactly one" },
    { value: "zeroOrOne", name: "zero or one" },
    { value: "zeroOrMore", name: "zero or more" },
    { value: "oneOrMore", name: "one or more" },
  ];

  const pickedName = $derived(picked?.kind === "box" ? picked.name : null);
  const entity = $derived(pickedName === null ? undefined : erd.entities.find((e) => e.name === pickedName));
  const relationIndex = $derived(picked?.kind === "link" ? picked.index : -1);
  const relation = $derived(relationIndex >= 0 ? erd.relations[relationIndex] : undefined);

  function rename(to: string) {
    if (!entity) return;
    const next = renameEntity(erd, entity.name, to);
    if (next === erd) return;
    onchange(next);
    picked = { kind: "box", name: to.trim() };
  }

  function setFields(fields: ErdField[]) {
    if (!entity) return;
    const name = entity.name;
    onchange({ ...erd, entities: erd.entities.map((e) => (e.name === name ? { ...e, fields } : e)) });
  }

  function setRelation(change: Partial<(typeof erd.relations)[number]>) {
    if (!relation) return;
    onchange({ ...erd, relations: erd.relations.map((r, i) => (i === relationIndex ? { ...r, ...change } : r)) });
  }

  function swap() {
    if (!relation) return;
    setRelation({ left: relation.right, right: relation.left, leftCount: relation.rightCount, rightCount: relation.leftCount });
  }

  function remove() {
    if (entity) onchange(removeEntity(erd, entity.name));
    else if (relation) onchange({ ...erd, relations: erd.relations.filter((_, i) => i !== relationIndex) });
    picked = null;
  }

  /** Takes a typed text when Enter is pressed, as when the field is left. */
  function enter(e: KeyboardEvent) {
    if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur();
  }
</script>

<aside class="properties">
  {#if entity}
    <h3>Entity</h3>
    <label class="line">
      <span>Name</span>
      <input value={entity.name} spellcheck="false" onchange={(e) => rename(e.currentTarget.value)} onkeydown={enter} />
    </label>
    <FieldList fields={entity.fields} {types} onchange={setFields} />
    <button class="danger" onclick={remove}>Delete entity</button>
  {:else if relation}
    <h3>Relation</h3>
    <p class="ends"><strong>{relation.left}</strong> to <strong>{relation.right}</strong></p>
    <label class="line">
      <span>{relation.left} side</span>
      <select value={relation.leftCount} onchange={(e) => setRelation({ leftCount: e.currentTarget.value as Cardinality })}>
        {#each COUNTS as c (c.value)}<option value={c.value}>{c.name}</option>{/each}
      </select>
    </label>
    <label class="line">
      <span>{relation.right} side</span>
      <select value={relation.rightCount} onchange={(e) => setRelation({ rightCount: e.currentTarget.value as Cardinality })}>
        {#each COUNTS as c (c.value)}<option value={c.value}>{c.name}</option>{/each}
      </select>
    </label>
    <label class="flag wide">
      <input type="checkbox" checked={relation.identifying} onchange={(e) => setRelation({ identifying: e.currentTarget.checked })} />
      Owned: the one side lists the many and they go with it (solid line)
    </label>
    <label class="line">
      <span>Label</span>
      <input value={relation.label ?? ""} spellcheck="false" placeholder="places" onchange={(e) => setRelation({ label: e.currentTarget.value.trim() || null })} onkeydown={enter} />
    </label>
    <div class="buttons">
      <button onclick={swap} title="Turn the relation around">Swap ends</button>
      <button class="danger" onclick={remove}>Delete relation</button>
    </div>
  {:else}
    <h3>ERD</h3>
    <p class="help">Drag an <strong>Entity</strong> from the palette onto the canvas. Pick <strong>Relation</strong> and drag from one entity to another: many of the first refer to one of the second. Click a box or a line to edit it here.</p>
    <p class="help">The diagram proposes; Apply makes the changes in the project.</p>
  {/if}
</aside>

<style>
  .properties {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    font-size: 12px;
  }

  h3 {
    margin: 0;
    font-size: 13px;
  }

  .line {
    display: grid;
    grid-template-columns: 90px 1fr;
    align-items: center;
    gap: 6px;
  }

  .line span {
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .flag {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-muted);
  }

  .flag.wide {
    align-items: flex-start;
  }

  .ends {
    margin: 0;
  }

  .buttons {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .danger {
    color: var(--error);
  }

  .help {
    margin: 0;
    color: var(--text-muted);
    line-height: 1.5;
  }
</style>
