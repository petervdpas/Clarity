<script lang="ts">
  // A list of one-line texts on a diagram box's properties: an enum's
  // values, or the functions of an engine or module as signatures. Text is
  // taken when a box is left or Enter is pressed.

  let {
    title,
    items,
    placeholder,
    hint,
    adds,
    onchange,
  }: {
    title: string;
    items: string[];
    /** What a new item starts as: Value1, or a signature to change. */
    placeholder: (n: number) => string;
    /** What each box takes, shown when hovered. */
    hint: string;
    /** The button that adds one: "+ Value". */
    adds: string;
    onchange: (items: string[]) => void;
  } = $props();

  function add() {
    let n = 1;
    while (items.includes(placeholder(n))) n++;
    onchange([...items, placeholder(n)]);
  }

  function enter(e: KeyboardEvent) {
    if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur();
  }
</script>

<h4>{title} <span class="count">{items.length}</span></h4>
{#each items as item, i (i)}
  <div class="item">
    <input
      value={item}
      spellcheck="false"
      title={hint}
      onchange={(e) => {
        const text = e.currentTarget.value.trim();
        onchange(text ? items.map((x, j) => (j === i ? text : x)) : items.filter((_, j) => j !== i));
      }}
      onkeydown={enter}
    />
    <button class="remove" onclick={() => onchange(items.filter((_, j) => j !== i))} title="Take this away" aria-label="Take {item} away">x</button>
  </div>
{/each}
<button onclick={add}>{adds}</button>

<style>
  h4 {
    margin: 4px 0 0;
    font-size: 13px;
  }

  .count {
    color: var(--text-muted);
    font-weight: normal;
  }

  .item {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 4px;
  }

  .item input {
    min-width: 0;
    font-family: var(--mono);
    font-size: 12px;
  }

  .remove {
    padding: 0 7px;
  }
</style>
