<script lang="ts">
  // Asks for a new diagram: its name, its kind, and whether it starts from
  // the project or with a small example of the kind to change. ERDs and class
  // diagrams are offered now; the other kinds come with their rows of the
  // roadmap (DESIGN 5.13).

  import Modal from "./Modal.svelte";
  import type { DiagramKind } from "./generated/DiagramKind";
  import type { DiagramKindInfo } from "./generated/DiagramKindInfo";

  let {
    kinds,
    taken,
    error,
    onsubmit,
    oncancel,
  }: {
    /** The kinds of diagram, as the backend lists them. */
    kinds: DiagramKindInfo[];
    /** The file names the diagrams have already. */
    taken: string[];
    error: string | null;
    onsubmit: (name: string, kind: DiagramKind, fromProject: boolean) => void;
    oncancel: () => void;
  } = $props();

  /** The kinds Clarity designs with so far. */
  const OFFERED: DiagramKind[] = ["erd", "class"];

  let name = $state("");
  let kind = $state<DiagramKind>("erd");
  let starts = $state<"project" | "example">("project");

  const stem = $derived(name.trim().replace(/\.mermaid$/i, ""));
  const file = $derived(stem ? `${stem}.mermaid` : "");
  const already = $derived(taken.includes(file));
  const ready = $derived(file !== "" && !already && !/[\\/]/.test(stem) && !stem.startsWith("."));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (ready) onsubmit(stem, kind, starts === "project");
  }
</script>

<Modal title={"New diagram"} width={380} onclose={oncancel}>
  <form class="dialog" onsubmit={submit}>
    <label>
      <span>Name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={name} placeholder="orders" spellcheck="false" autofocus />
    </label>
    <label>
      <span>Kind</span>
      <select bind:value={kind}>
        {#each kinds.filter((k) => OFFERED.includes(k.kind)) as k (k.kind)}
          <option value={k.kind}>{k.name}</option>
        {/each}
      </select>
    </label>
    <label>
      <span>Starts</span>
      <select bind:value={starts}>
        <option value="project">From the project</option>
        <option value="example">With a small example</option>
      </select>
    </label>
    <p class="hint">
      {#if already}
        The diagrams have {file} already.
      {:else if file}
        Clarity writes <code>diagrams/{file}</code>
        {starts === "project" ? (kind === "class" ? "with everything in the project" : "with all entities") : "with a small example to change"}. A
        diagram proposes; Apply makes the changes in the project, and From project writes it again.
      {:else}
        A name of its own, such as <code>orders</code>.
      {/if}
    </p>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="buttons">
      <button type="button" onclick={oncancel}>Cancel</button>
      <button type="submit" class="primary" disabled={!ready}>Add diagram</button>
    </div>
  </form>
</Modal>

<style>

  .dialog {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }


  label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .error {
    margin: 0;
    color: var(--error);
    font-size: 12px;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
