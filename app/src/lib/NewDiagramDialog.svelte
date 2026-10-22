<script lang="ts">
  // Asks for a new diagram: its name and its kind. Clarity writes it into
  // diagrams/ with a small example of the kind to change. ERDs and class
  // diagrams are offered now; the other kinds come with their rows of the
  // roadmap (DESIGN 5.13).

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
    onsubmit: (name: string, kind: DiagramKind) => void;
    oncancel: () => void;
  } = $props();

  /** The kinds Clarity designs with so far. */
  const OFFERED: DiagramKind[] = ["erd", "class"];

  let name = $state("");
  let kind = $state<DiagramKind>("erd");

  const stem = $derived(name.trim().replace(/\.mmd$/i, ""));
  const file = $derived(stem ? `${stem}.mmd` : "");
  const already = $derived(taken.includes(file));
  const ready = $derived(file !== "" && !already && !/[\\/]/.test(stem) && !stem.startsWith("."));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (ready) onsubmit(stem, kind);
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && oncancel()} />

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && oncancel()}>
  <form class="dialog" onsubmit={submit}>
    <h2>New diagram</h2>
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
    <p class="hint">
      {#if already}
        The diagrams have {file} already.
      {:else if file}
        Clarity writes <code>diagrams/{file}</code> with a small example to change. A diagram proposes; Apply makes the changes in the project.
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
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    display: grid;
    place-items: center;
    z-index: 50;
  }

  .dialog {
    width: min(380px, calc(100vw - 32px));
    background: var(--panel);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.35);
  }

  h2 {
    margin: 0;
    font-size: 15px;
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
