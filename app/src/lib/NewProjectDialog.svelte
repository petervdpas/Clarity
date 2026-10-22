<script lang="ts">
  // Asks what a new project is transpiled into before it is created. The
  // project starts with one SQLite data source; more are added in the
  // project settings.

  import Modal from "./Modal.svelte";
  import StackFields from "./StackFields.svelte";
  import type { Catalog } from "./generated/Catalog";
  import type { Stack } from "./generated/Stack";

  let {
    catalog,
    folder,
    error,
    onsubmit,
    oncancel,
  }: {
    catalog: Catalog;
    folder: string;
    error: string | null;
    onsubmit: (stack: Stack) => void;
    oncancel: () => void;
  } = $props();

  let stack = $state<Stack>({ backend: "rust", ui: "web", targets: [] });
  $effect.pre(() => {
    stack = { ...catalog.starter, targets: [...catalog.starter.targets] };
  });

  function submit(e: SubmitEvent) {
    e.preventDefault();
    onsubmit(stack);
  }
</script>

<Modal title={"New project"} width={400} onclose={oncancel}>
  <form class="dialog" onsubmit={submit}>
    <p class="folder" title={folder}>{folder}</p>
    <div class="form-grid compact">
      <StackFields {catalog} {stack} onchange={(next) => (stack = next)} />
    </div>
    <p class="note">The project starts with one SQLite database. Data sources and the stack can be changed later on the Project tab.</p>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="buttons">
      <button type="button" onclick={oncancel}>Cancel</button>
      <button type="submit" class="primary" disabled={stack.targets.length === 0}>Create project</button>
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
