<script lang="ts">
  // A small dialog to pick one of a few choices, such as the application's
  // language or theme, in Clarity's dialog frame. Escape or a click beside
  // it closes it unchanged.
  import Dialog from "./ui/Dialog.svelte";

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

<Dialog label={title} onclose={onclose}>
  <div class="picker">
    {#each choices as choice (choice.value)}
      <button class:current={current === choice.value} lang={choice.lang} onclick={() => onchoose(choice.value)}>{choice.label}</button>
    {/each}
  </div>
</Dialog>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .picker button.current {
    border-color: var(--accent);
  }
</style>
