<script lang="ts">
  // Picks what an application uses of a web API's OpenAPI description
  // (DESIGN 5.23): the description file in the project's apis folder, the
  // operations it calls and the schemas it keeps on their own. Clarity
  // writes the API's module from the picks, an entity per schema they need
  // and an action per operation, and writes it again for a new revision.

  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import type { ApiListing } from "./generated/ApiListing";
  import type { DataSource } from "./generated/DataSource";

  let {
    source,
    onsave,
    onclose,
  }: {
    /** The web API data source whose picks these are. */
    source: DataSource;
    onsave: (change: Partial<DataSource>) => void;
    onclose: () => void;
  } = $props();

  let listing = $state<ApiListing | null>(null);
  let error = $state<string | null>(null);
  let filter = $state("");

  // What is picked, and with it what the operations need, follow the project file.
  $effect(() => {
    void source.openapi;
    void source.picks.join();
    void source.pick_schemas.join();
    invoke<ApiListing>("api_listing", { source: source.name })
      .then((found) => {
        listing = found;
        error = null;
      })
      .catch((err) => {
        listing = null;
        error = String(err);
      });
  });

  const shown = $derived(
    (listing?.operations ?? []).filter((o) => {
      const words = filter.trim().toLowerCase();
      return words === "" || `${o.id} ${o.method} ${o.path} ${o.summary ?? ""}`.toLowerCase().includes(words);
    }),
  );

  async function chooseFile() {
    const picked = await openDialog({ multiple: false, filters: [{ name: "OpenAPI description", extensions: ["yaml", "yml", "json"] }] });
    if (typeof picked !== "string") return;
    try {
      const name = await invoke<string>("add_api_file", { from: picked });
      onsave({ openapi: name });
      error = null;
    } catch (err) {
      error = String(err);
    }
  }

  function pickOperation(id: string, on: boolean) {
    const picks = on ? [...source.picks, id] : source.picks.filter((p) => p !== id);
    onsave({ picks });
  }

  function pickSchema(name: string, on: boolean) {
    const pick_schemas = on ? [...source.pick_schemas, name] : source.pick_schemas.filter((p) => p !== name);
    onsave({ pick_schemas });
  }
</script>

<div class="api">
  <div class="head">
    <strong>Web API {source.name}</strong>
    <button class="small" onclick={onclose} title="Close">&times;</button>
  </div>
  <p class="intro">
    Pick what the application uses. Clarity writes <code>apis/{source.name}.clarity</code> from the description: an entity for each schema the picked
    operations take or give, and an action for each picked operation. A new revision of the description writes it again.
  </p>

  <div class="row">
    <span>Description</span>
    {#if source.openapi}
      <code>apis/{source.openapi}</code>
      {#if listing?.title}<span class="muted">{listing.title} {listing.version}</span>{/if}
    {:else}
      <span class="muted">None yet</span>
    {/if}
    <button onclick={chooseFile} title="Copy an OpenAPI description (YAML or JSON) into the project's apis folder">Choose file...</button>
  </div>

  {#if error}<p class="problem">{error}</p>{/if}

  {#if listing?.file}
    <div class="row">
      <strong>Operations</strong>
      <span class="muted">{source.picks.length} of {listing.operations.length} picked</span>
      <input class="filter" bind:value={filter} placeholder="Filter" spellcheck="false" />
    </div>
    <ul class="list">
      {#each shown as o (o.id)}
        <li>
          <label class:off={!o.callable} title={o.callable ? "" : "Clarity does not call a patch yet"}>
            <input type="checkbox" disabled={!o.callable} checked={o.picked} onchange={(e) => pickOperation(o.id, e.currentTarget.checked)} />
            <span class="method">{o.method.toUpperCase()}</span>
            <span class="mono">{o.path}</span>
            <span class="mono muted">{o.id}</span>
            {#if o.summary}<span class="muted">{o.summary}</span>{/if}
          </label>
        </li>
      {/each}
    </ul>

    <div class="row"><strong>Schemas</strong><span class="muted">those the picked operations need are entities already</span></div>
    <ul class="list">
      {#each listing.schemas as s (s.name)}
        <li>
          <label title={s.needed ? "A picked operation takes or gives it" : ""}>
            <input type="checkbox" disabled={s.needed} checked={s.needed || s.picked} onchange={(e) => pickSchema(s.name, e.currentTarget.checked)} />
            <span class="mono">{s.name}</span>
            {#if s.needed}<span class="muted">needed</span>{/if}
          </label>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .api {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .head,
  .row {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .head {
    justify-content: space-between;
  }

  .filter {
    margin-left: auto;
    width: 200px;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 320px;
    overflow: auto;
  }

  .list label {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }

  .list label.off {
    opacity: 0.6;
  }

  .method {
    font-family: var(--mono);
    font-size: 11px;
    min-width: 52px;
    color: var(--text-muted);
  }

  .mono {
    font-family: var(--mono);
  }

  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }

  .problem {
    color: var(--error);
  }
</style>
