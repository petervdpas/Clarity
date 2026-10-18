<script lang="ts">
  // The sidebar of the Vault tab: the categories of the secrets, or the
  // connections, to pick from.
  import { vault } from "./vaultStore.svelte";

  let filter = $state("");
  const count = (category: string) => vault.rows.filter((r) => r.category.toLowerCase() === category.toLowerCase()).length;
  const connections = $derived(vault.connections.filter((c) => c.name.toLowerCase().includes(filter.trim().toLowerCase())));
</script>

{#if vault.view === "secrets"}
  <section>
    <h2>Categories</h2>
    {#if vault.unlocked}
      <ul>
        <li><button class:active={vault.category === null} onclick={() => (vault.category = null)}>(All) <span>{vault.rows.length}</span></button></li>
        {#each vault.categories as category (category)}
          <li>
            <button class:active={vault.category?.toLowerCase() === category.toLowerCase()} onclick={() => (vault.category = category)}>
              {category} <span>{count(category)}</span>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="none">Unlock the vault to see its secrets.</p>
    {/if}
  </section>
{:else}
  <section>
    <h2>Connections</h2>
    <input class="filter" placeholder="Filter..." bind:value={filter} aria-label="Filter connections" />
    <ul>
      {#each connections as c (c.name)}
        <li>
          <button class:active={vault.connection === c.name} onclick={() => vault.pickConnection(c.name)}>
            {c.name} <span>{c.fields.length === 1 ? "1 field" : `${c.fields.length} fields`}</span>
          </button>
        </li>
      {:else}
        <li class="none">{vault.connections.length === 0 ? "No connections yet." : "No connection matches."}</li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  /* As the app's other sidebar sections. */
  section {
    flex: 1 1 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
    overflow: auto;
    padding: 10px 14px;
  }

  h2 {
    margin: 0 0 6px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li button {
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    padding: 5px 8px;
    border-radius: 4px;
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }

  li button:hover:not(.active) {
    background: var(--panel-alt);
  }

  li button.active {
    background: var(--accent-soft, color-mix(in srgb, var(--accent) 25%, transparent));
  }

  li button span {
    color: var(--text-muted);
    font-size: 12px;
  }

  .filter {
    width: 100%;
    box-sizing: border-box;
  }

  .none {
    color: var(--text-muted);
    font-size: 13px;
    padding: 4px 8px;
    margin: 0;
  }
</style>
