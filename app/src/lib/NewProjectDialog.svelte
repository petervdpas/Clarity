<script lang="ts">
  // Asks what a new project is transpiled into before it is created. The
  // project starts with one SQLite data source; more are added in the
  // project settings. With a projects folder in Clarity settings it asks
  // for the name too, and the project goes there; else the file was chosen
  // before and shows on top.

  import Modal from "./Modal.svelte";
  import StackFields from "./StackFields.svelte";
  import type { Catalog } from "./generated/Catalog";
  import type { Stack } from "./generated/Stack";

  let {
    catalog,
    folder,
    error,
    projects = null,
    onsubmit,
    oncancel,
  }: {
    catalog: Catalog;
    folder: string;
    error: string | null;
    /** The projects folder of Clarity settings, when the name is asked here. */
    projects?: string | null;
    onsubmit: (stack: Stack, name: string) => void;
    oncancel: () => void;
  } = $props();

  let name = $state("");
  let stack = $state<Stack>({ backend: "rust", ui: "web", targets: [] });
  $effect.pre(() => {
    stack = { ...catalog.starter, targets: [...catalog.starter.targets] };
  });

  function submit(e: SubmitEvent) {
    e.preventDefault();
    onsubmit(stack, name.trim());
  }
</script>

<Modal title={"New project"} width={400} onclose={oncancel}>
  <form class="dialog" onsubmit={submit}>
    {#if projects}
      <p class="folder" title={projects}>In {projects}</p>
    {:else}
      <p class="folder" title={folder}>{folder}</p>
    {/if}
    <div class="form-grid compact">
      {#if projects}
        <label class="form-label" for="new-project-name">Name</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input id="new-project-name" bind:value={name} placeholder="shop" spellcheck="false" autofocus />
      {/if}
      <StackFields {catalog} {stack} onchange={(next) => (stack = next)} />
    </div>
    <p class="note">The project starts with one SQLite database. Data sources and the stack can be changed later on the Project tab.</p>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="buttons">
      <button type="button" onclick={oncancel}>Cancel</button>
      <button type="submit" class="primary" disabled={stack.targets.length === 0 || (projects !== null && !name.trim())}>Create project</button>
    </div>
  </form>
</Modal>

<style>

  .compact {
    grid-template-columns: 80px minmax(0, 1fr);
  }

  .dialog {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }


  p {
    margin: 0;
    font-size: 12px;
  }

  .folder {
    font-family: var(--mono);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .note {
    color: var(--text-muted);
  }

  .error {
    color: var(--error);
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
