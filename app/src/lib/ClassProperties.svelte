<script lang="ts">
  // The properties of what is picked on a class diagram: a box's name, the
  // module it goes to and its members (an entity's fields, an enum's
  // values, an engine's or module's functions), or a relation's kind, how
  // many stand at each end, and its label. Every change is written at
  // once; text is taken when a box is left or Enter is pressed.

  import type { Cardinality } from "./generated/Cardinality";
  import type { ClassBox } from "./generated/ClassBox";
  import type { ClassDesign } from "./generated/ClassDesign";
  import type { ClassLink } from "./generated/ClassLink";
  import type { ClassRelation } from "./generated/ClassRelation";
  import type { CanvasPick } from "./designCanvas";
  import { MULTIPLICITY, removeBox, renameBox } from "./classDesign";
  import FieldList from "./FieldList.svelte";
  import TextList from "./TextList.svelte";

  let {
    design,
    picked = $bindable(null),
    types,
    onchange,
  }: {
    design: ClassDesign;
    picked?: CanvasPick;
    /** The types a field may have: Clarity's, the project's enums, refs and lists of the entities. */
    types: string[];
    onchange: (design: ClassDesign) => void;
  } = $props();

  const KIND_NAMES: Record<ClassBox["kind"], string> = { entity: "Entity", enum: "Enum", engine: "Engine", module: "Module" };

  const LINKS: { value: ClassLink; name: string }[] = [
    { value: "association", name: "Association: refers to one" },
    { value: "composition", name: "Composition: owns a list" },
    { value: "aggregation", name: "Aggregation: lists" },
    { value: "uses", name: "Uses: its code uses it (drawn only)" },
  ];

  const COUNTS: Cardinality[] = ["exactlyOne", "zeroOrOne", "zeroOrMore", "oneOrMore"];

  const pickedName = $derived(picked?.kind === "box" ? picked.name : null);
  const box = $derived(pickedName === null ? undefined : design.boxes.find((b) => b.name === pickedName));
  const relationIndex = $derived(picked?.kind === "link" ? picked.index : -1);
  const relation = $derived(relationIndex >= 0 ? design.relations[relationIndex] : undefined);

  function setBox(change: Partial<ClassBox>) {
    if (!box) return;
    const name = box.name;
    onchange({ ...design, boxes: design.boxes.map((b) => (b.name === name ? { ...b, ...change } : b)) });
  }

  function rename(to: string) {
    if (!box) return;
    const next = renameBox(design, box.name, to);
    if (next === design) return;
    onchange(next);
    picked = { kind: "box", name: to.trim() };
  }

  function setRelation(change: Partial<ClassRelation>) {
    if (!relation) return;
    onchange({ ...design, relations: design.relations.map((r, i) => (i === relationIndex ? { ...r, ...change } : r)) });
  }

  function swap() {
    if (!relation) return;
    setRelation({ left: relation.right, right: relation.left, leftCount: relation.rightCount, rightCount: relation.leftCount });
  }

  function remove() {
    if (box) onchange(removeBox(design, box.name));
    else if (relation) onchange({ ...design, relations: design.relations.filter((_, i) => i !== relationIndex) });
    picked = null;
  }

  function enter(e: KeyboardEvent) {
    if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur();
  }
</script>

<aside class="properties">
  {#if box}
    <h3>{KIND_NAMES[box.kind]}</h3>
    <label class="line">
      <span>Name</span>
      <input value={box.name} spellcheck="false" onchange={(e) => rename(e.currentTarget.value)} onkeydown={enter} />
    </label>
    {#if box.kind !== "module"}
      <label class="line">
        <span>Module</span>
        <input
          value={box.module ?? ""}
          spellcheck="false"
          placeholder="the module of a related box"
          title="The module it goes to, drawn as its namespace"
          onchange={(e) => setBox({ module: e.currentTarget.value.trim() || null })}
          onkeydown={enter}
        />
      </label>
    {/if}
    {#if box.kind === "entity"}
      <FieldList fields={box.fields} {types} onchange={(fields) => setBox({ fields })} />
    {:else if box.kind === "enum"}
      <TextList title="Values" items={box.values} placeholder={(n) => `Value${n}`} hint="A value, in PascalCase" adds="+ Value" onchange={(values) => setBox({ values })} />
    {:else}
      <TextList
        title="Functions"
        items={box.functions}
        placeholder={(n) => `function${n}()`}
        hint="A signature, as play(square: integer) -> text"
        adds="+ Function"
        onchange={(functions) => setBox({ functions })}
      />
    {/if}
    <button class="danger" onclick={remove}>Delete {KIND_NAMES[box.kind].toLowerCase()}</button>
  {:else if relation}
    <h3>Relation</h3>
    <p class="ends"><strong>{relation.left}</strong> to <strong>{relation.right}</strong></p>
    <label class="line">
      <span>Kind</span>
      <select value={relation.link} onchange={(e) => setRelation({ link: e.currentTarget.value as ClassLink })}>
        {#each LINKS as l (l.value)}<option value={l.value}>{l.name}</option>{/each}
      </select>
    </label>
    {#if relation.link !== "uses"}
    <label class="line">
      <span>{relation.left} end</span>
      <select value={relation.leftCount} onchange={(e) => setRelation({ leftCount: e.currentTarget.value as Cardinality })}>
        {#each COUNTS as c (c)}<option value={c}>{MULTIPLICITY[c]}</option>{/each}
      </select>
    </label>
    <label class="line">
      <span>{relation.right} end</span>
      <select value={relation.rightCount} onchange={(e) => setRelation({ rightCount: e.currentTarget.value as Cardinality })}>
        {#each COUNTS as c (c)}<option value={c}>{MULTIPLICITY[c]}</option>{/each}
      </select>
    </label>
    {/if}
    <label class="line">
      <span>Label</span>
      <input value={relation.label ?? ""} spellcheck="false" placeholder="the field's name" onchange={(e) => setRelation({ label: e.currentTarget.value.trim() || null })} onkeydown={enter} />
    </label>
    <div class="buttons">
      <button onclick={swap} title="Turn the relation around">Swap ends</button>
      <button class="danger" onclick={remove}>Delete relation</button>
    </div>
  {:else}
    <h3>Class diagram</h3>
    <p class="help">
      Drag an <strong>Entity</strong>, <strong>Enum</strong>, <strong>Engine</strong> or <strong>Module</strong> from the palette onto the canvas. Pick a
      line and drag from one box to another: an <strong>Association</strong> from the box that refers, a <strong>Composition</strong> or
      <strong>Aggregation</strong> from the whole to its parts. Click a box or a line to edit it here.
    </p>
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
