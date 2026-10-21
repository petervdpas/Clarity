<script lang="ts">
  // A small menu where the person right-clicked, with what can be done to
  // the thing under the pointer. A click elsewhere or Escape closes it.

  let {
    x,
    y,
    items,
    onclose,
  }: {
    x: number;
    y: number;
    items: { label: string; danger?: boolean; run: () => void }[];
    onclose: () => void;
  } = $props();

  let menu: HTMLElement | undefined = $state();
  // Kept inside the window when it opens near an edge.
  const left = $derived(Math.min(x, window.innerWidth - (menu?.offsetWidth ?? 160) - 4));
  const top = $derived(Math.min(y, window.innerHeight - (menu?.offsetHeight ?? 80) - 4));
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} onpointerdown={(e) => menu && !menu.contains(e.target as Node) && onclose()} onblur={onclose} />

<div class="menu" role="menu" bind:this={menu} style:left="{left}px" style:top="{top}px">
  {#each items as item (item.label)}
    <button
      role="menuitem"
      class:danger={item.danger}
      onclick={() => {
        onclose();
        item.run();
      }}>{item.label}</button
    >
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 60;
    min-width: 150px;
    display: flex;
    flex-direction: column;
    padding: 4px;
    background: var(--panel);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.3);
  }

  button {
    text-align: left;
    border: none;
    background: none;
    padding: 5px 10px;
    border-radius: 4px;
    color: var(--text);
  }

  button:hover {
    background: var(--accent-soft);
  }

  button.danger {
    color: var(--error);
  }
</style>
