<script lang="ts">
  // A small dialog to pick one of a few choices, such as the application's
  // language or theme. Escape or a click beside it closes it unchanged.

  type Choice = { value: string; label: string; lang?: string };

  let {
    title,
    choices,
    current,
    onchoose,
    onclose,
  }: {
    /** What is chosen, for screen readers. */
    title: string;
    choices: Choice[];
    current: string | undefined;
    onchoose: (value: string) => void;
    onclose: () => void;
  } = $props();
</script>

<div class="backdrop" role="presentation" onclick={onclose}>
  <div
    class="picker"
    role="dialog"
    aria-label={title}
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.key === "Escape" && onclose()}
  >
    {#each choices as choice (choice.value)}
      <button class:current={current === choice.value} lang={choice.lang} onclick={() => onchoose(choice.value)}>{choice.label}</button>
    {/each}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.25);
    display: grid;
    place-items: center;
    z-index: 20;
  }

  .picker {
    background: var(--panel);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 200px;
  }

  .picker button.current {
    border-color: var(--accent);
  }
</style>
