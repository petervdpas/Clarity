<script lang="ts">
  // Asks for a new resource: its name, its type, and the file it starts
  // from, which is copied into resources/ under the name with the file's
  // extension. Markdown may start without a file, as a heading to write
  // under in Clarity IDE; every other type starts from a file.

  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import type { ResourceKind } from "./generated/ResourceKind";
  import type { ResourceKindInfo } from "./generated/ResourceKindInfo";

  let {
    kinds,
    taken,
    error,
    onsubmit,
    oncancel,
  }: {
    /** The types a resource can have, as the backend lists them. */
    kinds: ResourceKindInfo[];
    /** The names the resources have already. */
    taken: string[];
    error: string | null;
    onsubmit: (name: string, kind: ResourceKind, from: string | null) => void;
    oncancel: () => void;
  } = $props();

  let name = $state("");
  let kind = $state<ResourceKind>("markdown");
  let from = $state<string | null>(null);

  const info = $derived(kinds.find((k) => k.kind === kind));
  const needsFile = $derived(kind !== "markdown");
  const fileName = $derived(from?.split(/[\\/]/).at(-1) ?? "");
  const extension = $derived(from ? (fileName.includes(".") ? (fileName.split(".").at(-1)?.toLowerCase() ?? "") : "") : (info?.extensions[0] ?? ""));
  const stem = $derived.by(() => {
    const trimmed = name.trim();
    const own = info?.extensions.find((e) => trimmed.toLowerCase().endsWith(`.${e}`));
    return own ? trimmed.slice(0, -(own.length + 1)) : trimmed;
  });
  const file = $derived(stem ? (extension ? `${stem}.${extension}` : stem) : "");
  const already = $derived(taken.includes(file));
  const ready = $derived(file !== "" && !already && (!needsFile || from !== null));

  /** Picks the file to copy in, of the type's extensions; its name names the resource until one is typed. */
  async function pick() {
    const picked = await openDialog({ multiple: false, filters: info && info.extensions.length > 0 ? [{ name: info.name, extensions: info.extensions }] : [] });
    if (typeof picked !== "string") return;
    from = picked;
    if (!name.trim()) name = (picked.split(/[\\/]/).at(-1) ?? "").replace(/\.[^.]*$/, "");
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (ready) onsubmit(stem, kind, from);
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && oncancel()} />

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && oncancel()}>
  <form class="dialog" onsubmit={submit}>
    <h2>New resource</h2>
    <label>
      <span>Name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={name} placeholder="rules-en" spellcheck="false" autofocus />
    </label>
    <label>
      <span>Type</span>
      <select bind:value={kind} onchange={() => (from = null)}>
        {#each kinds as k (k.kind)}
          <option value={k.kind}>{k.name}{k.extensions.length > 0 ? ` (.${k.extensions.join(", .")})` : ""}</option>
        {/each}
      </select>
    </label>
    <div class="file">
      <button type="button" onclick={pick}>{from ? "Pick another file..." : "Pick a file..."}</button>
      {#if from}
        <code title={from}>{fileName}</code>
        <button type="button" onclick={() => (from = null)} title="Start without a file">Clear</button>
      {:else}
        <span class="hint">{needsFile ? "A file to copy in" : "Optional: without one, you write it here"}</span>
      {/if}
    </div>
    <p class="hint">
      {#if already}
        The resources have {file} already.
      {:else if file}
        Clarity {from ? "copies the file to" : "writes"} <code>resources/{file}</code>.
      {:else}
        A name of its own, such as <code>rules-en</code>; one file per language is up to you.
      {/if}
    </p>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="buttons">
      <button type="button" onclick={oncancel}>Cancel</button>
      <button type="submit" class="primary" disabled={!ready}>Add resource</button>
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

  .file {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .file code {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
